//! Create *regular* scheduled routes. Checks for stopover availability and destination demand
//! to determine the best configuration.
use crate::aircraft::{Aircraft, AircraftType};
use crate::airport::{db::Airports, Airport};
use crate::route::config::{CargoConfig, CargoConfigAlgorithm, PaxConfig, PaxConfigAlgorithm};
use crate::route::db::{DemandMatrix, DistanceMatrix};
use crate::route::demand::PaxDemand;
use crate::route::metrics::{self, ConfigVariant};
use crate::route::ticket::{CargoTicket, PaxTicket, Ticket};
use crate::route::{
    config::ConfigAlgorithm,
    search::{ConcreteRoutes, Routes},
    Ci, Distance, DistanceError, FlightTime, FlightTimeError, Speed,
};
use crate::user::{GameMode, HeavyTraining, LargeTraining, Settings};
use crate::utils::{Filter, FilterError};
use derive_more::Display;
use std::num::NonZeroU8;
use std::ops::Div;
use thiserror::Error;

use super::{stopover::Stopover, FailedRoute};

#[derive(Debug, Clone, Error)]
pub enum ScheduleError {
    #[error("distance failed the provided constraint")]
    DistanceConstraint,
    #[error("flight time failed the provided constraint")]
    FlightTimeConstraint,
    #[error("insufficient demand")]
    InsufficientDemand,
    #[error("trips per day too high for flight time")]
    TripsPerDayTooHigh,
}

/// Collection of [ScheduledRoute], checked against the provided aircraft.
pub type ScheduledRoutes<'a> = Routes<'a, ScheduledRoute<'a>, ScheduleConfig<'a>>;

#[derive(Debug, Clone)]
pub struct ScheduledRoute<'a> {
    pub destination: &'a Airport,
    pub direct_distance: Distance,
    pub stopover: Option<Stopover<'a>>,
    pub total_distance: Distance,
    pub flight_time: FlightTime,
    pub ci: Ci,
    pub contribution: f32,

    pub trips_per_day: NonZeroU8,
    pub num_aircraft: NonZeroU8,
    pub config: ConfigVariant,
    pub ticket: Ticket,

    pub revenue: f32,
    pub fuel: f32,
    pub co2: f32,
    pub acheck_cost: f32,
    pub repair_cost: f32,
    pub profit: f32,
}

#[derive(Debug, Clone)]
pub struct FerryFlightRoute<'a> {
    pub destination: &'a Airport,
    pub direct_distance: Distance,
    pub flight_time: FlightTime,
    pub fuel: f32,
    pub cost: f32,
}

#[derive(Debug, Clone)]
pub struct ScheduleConfig<'a> {
    pub airports: &'a Airports,
    pub origin: &'a Airport,
    pub aircraft: &'a Aircraft,
    pub game_mode: &'a GameMode,
    pub search_config: &'a SearchConfig<'a>,
}

struct SolverResult {
    config: ConfigVariant,
    revenue_per_trip: f32,
    tpd: TripsPerDay,
    num_ac: NumAircraft,
}

impl<'a> ConcreteRoutes<'a> {
    pub fn schedule(
        mut self,
        demand_matrix: &'a DemandMatrix,
        distances: &'a DistanceMatrix,
        search_config: &'a SearchConfig,
    ) -> ScheduledRoutes<'a> {
        let mut routes = Vec::new();
        let settings = search_config.user_settings;
        let ac = self.config.aircraft;
        let game_mode = self.config.game_mode;

        let speed_mult = game_mode.speed_multiplier();

        for route in self.routes.iter() {
            // quickly check direct distance first because stopover finding is expensive
            if !search_config
                .distance_filter
                .contains(&route.direct_distance)
            {
                self.errors.push(FailedRoute {
                    destination: route.destination,
                    error: ScheduleError::DistanceConstraint.into(),
                });
                continue;
            }
            let (stopover, total_distance) = if search_config.inflate_distance_with_stopover {
                let target_dist = match &search_config.distance_filter {
                    Filter::RangeTo(to) => to.end,
                    Filter::Range(r) => r.end,
                    _ => Distance::MAX,
                };

                let target_dist = if target_dist == Distance::MAX {
                    match &search_config.flight_time_filter {
                        Filter::RangeTo(to) => {
                            Distance::new_unchecked(to.end.get() * ac.speed * speed_mult)
                        }
                        _ => target_dist,
                    }
                } else {
                    target_dist
                };

                let direct_requires_stopover = route.direct_distance.get() > ac.range as f32;

                match Stopover::find_by_target_distance_lt(
                    self.config.airports.data(),
                    distances,
                    self.config.origin,
                    route.destination,
                    ac,
                    *game_mode,
                    target_dist,
                ) {
                    Some((s, d)) => (Some(s), d),
                    None if direct_requires_stopover => {
                        self.errors.push(FailedRoute {
                            destination: route.destination,
                            error: ScheduleError::DistanceConstraint.into(),
                        });
                        continue;
                    }
                    None => (None, route.direct_distance),
                }
            } else {
                let needs_stopover = route.direct_distance.get() > ac.range as f32;
                if needs_stopover {
                    match Stopover::find_by_efficiency(
                        self.config.airports.data(),
                        distances,
                        self.config.origin,
                        route.destination,
                        ac,
                        *game_mode,
                    ) {
                        Some((s, d)) => (Some(s), d),
                        None => continue,
                    }
                } else {
                    (None, route.direct_distance)
                }
            };

            // check full distance because stopover may incur a distance penalty
            if !search_config.distance_filter.contains(&total_distance) {
                self.errors.push(FailedRoute {
                    destination: route.destination,
                    error: ScheduleError::DistanceConstraint.into(),
                });
                continue;
            }

            let mut ci = Ci::MAX;
            let mut speed_val = ac.speed * speed_mult;
            let mut flight_time = total_distance / Speed::new_unchecked(speed_val);

            match search_config.ci {
                CiStrategy::AlignConstraint => {
                    // try to slow down to meet max flight time constraint
                    let max_ft = match &search_config.flight_time_filter {
                        Filter::RangeTo(to) => Some(to.end),
                        Filter::Range(r) => Some(r.end),
                        _ => None,
                    };

                    if let Some(target_t) = max_ft {
                        if flight_time < target_t {
                            ci = Ci::calculate(
                                total_distance,
                                Speed::new_unchecked(speed_val),
                                target_t,
                            );
                            let ci_mult = 0.0035 * ci.get() as f32 + 0.3;
                            speed_val = ac.speed * speed_mult * ci_mult;
                            flight_time = total_distance / Speed::new_unchecked(speed_val);
                        }
                    }
                }
                CiStrategy::Strict(c) => {
                    ci = c;
                    if ci != Ci::MAX {
                        let ci_mult = 0.0035 * ci.get() as f32 + 0.3;
                        speed_val = ac.speed * speed_mult * ci_mult;
                        flight_time = total_distance / Speed::new_unchecked(speed_val);
                    }
                }
            }

            if !search_config.flight_time_filter.contains(&flight_time) {
                self.errors.push(FailedRoute {
                    destination: route.destination,
                    error: ScheduleError::FlightTimeConstraint.into(),
                });
                continue;
            }

            let ticket = match ac.r#type {
                AircraftType::Pax => Ticket::Pax(PaxTicket::from_optimal(
                    route.direct_distance.get(),
                    *game_mode,
                )),
                AircraftType::Cargo => Ticket::Cargo(CargoTicket::from_optimal(
                    route.direct_distance.get(),
                    *game_mode,
                )),
                AircraftType::Vip => Ticket::VIP(PaxTicket::from_optimal_vip(
                    route.direct_distance.get(),
                    *game_mode,
                )),
            };

            let contribution = metrics::contribution(total_distance, *game_mode, ci);
            let acheck = metrics::acheck_cost(ac.check_cost, ac.maint, flight_time, *game_mode);
            let repair = metrics::repair_cost(ac.cost, settings.training.repair);
            let fuel = metrics::fuel(ac.fuel, total_distance, settings.training.fuel, ci);

            let demand = demand_matrix.get_unchecked(self.config.origin.idx, route.destination.idx);
            let res = solve_schedule(
                ac,
                demand,
                total_distance,
                &ticket,
                flight_time,
                &search_config.schedule,
                &search_config.config,
                settings.load.get(),
                settings.cargo_load.get(),
                settings.training.l,
                settings.training.h,
                settings.revenue_loss_tol.get(),
                *game_mode,
                settings.allow_invalid_tpd,
            );

            match res {
                Some(solver_res) => {
                    let co2 = match &solver_res.config {
                        ConfigVariant::Pax(c) => metrics::co2_pax(
                            ac.co2,
                            c,
                            total_distance,
                            settings.training.co2,
                            settings.load.get(),
                            ci,
                        ),
                        ConfigVariant::Cargo(c) => metrics::co2_cargo(
                            ac.co2,
                            ac.capacity,
                            c,
                            total_distance,
                            settings.training.co2,
                            settings.cargo_load.get(),
                            ci,
                        ),
                    };

                    let expense = fuel * settings.fuel_price.get() / 1000.0
                        + co2 * settings.co2_price.get() / 1000.0
                        + acheck
                        + repair;

                    let profit = solver_res.revenue_per_trip - expense;

                    routes.push(ScheduledRoute {
                        destination: route.destination,
                        direct_distance: route.direct_distance,
                        stopover,
                        total_distance,
                        flight_time,
                        ci,
                        contribution,
                        trips_per_day: solver_res.tpd,
                        num_aircraft: solver_res.num_ac,
                        config: solver_res.config,
                        ticket,
                        revenue: solver_res.revenue_per_trip,
                        fuel,
                        co2,
                        acheck_cost: acheck,
                        repair_cost: repair,
                        profit,
                    });
                }
                None => {
                    self.errors.push(FailedRoute {
                        destination: route.destination,
                        error: ScheduleError::InsufficientDemand.into(),
                    });
                }
            }
        }

        match search_config.sort_by {
            SortBy::ProfitPerTrip => {
                routes.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());
            }
            SortBy::ProfitPerAcPerDay => {
                routes.sort_by(|a, b| {
                    let pa = a.profit * a.trips_per_day.get() as f32;
                    let pb = b.profit * b.trips_per_day.get() as f32;
                    pb.partial_cmp(&pa).unwrap()
                });
            }
        }

        ScheduledRoutes {
            routes,
            errors: self.errors,
            config: ScheduleConfig {
                airports: self.config.airports,
                origin: self.config.origin,
                aircraft: self.config.aircraft,
                game_mode: self.config.game_mode,
                search_config,
            },
        }
    }
}

// TODO: use generics to move aircraft type upwards
#[allow(clippy::too_many_arguments)]
fn solve_schedule(
    ac: &Aircraft,
    demand: PaxDemand,
    full_dist: Distance,
    ticket: &Ticket,
    flight_time: FlightTime,
    sched_strat: &ScheduleStrategy,
    conf_alg: &ConfigAlgorithm,
    load_factor: f32,
    cargo_load_factor: f32,
    training_l: LargeTraining,
    training_h: HeavyTraining,
    revenue_loss_tol: f32,
    game_mode: GameMode,
    allow_invalid_tpd: bool,
) -> Option<SolverResult> {
    let max_tpd_phys = (24.0 / flight_time.get()).floor() as u8;
    let tpd_per_ac = match sched_strat.trips_per_day {
        TripsPerDayStrategy::Strict(t) => {
            if !allow_invalid_tpd && (t.get() > max_tpd_phys) {
                return None;
            }
            t
        }
        TripsPerDayStrategy::Maximise => TripsPerDay::new(max_tpd_phys)?,
    };

    let try_solve = |total_tpd: u32| -> Option<(ConfigVariant, f32)> {
        match ac.r#type {
            AircraftType::Pax | AircraftType::Vip => {
                let eff_load = load_factor as f64;
                let pax_dem = demand.div(total_tpd as f64 * eff_load);

                let alg = if let ConfigAlgorithm::Pax(a) = conf_alg {
                    *a
                } else {
                    PaxConfigAlgorithm::Auto
                };

                let cfg = PaxConfig::calculate(
                    pax_dem,
                    ac.capacity as u16,
                    full_dist.get(),
                    game_mode,
                    alg,
                )?;
                let var = ConfigVariant::Pax(cfg);
                let inc = metrics::revenue(
                    &var,
                    ticket,
                    ac.capacity,
                    load_factor,
                    training_l,
                    training_h,
                );
                Some((var, inc))
            }
            AircraftType::Cargo => {
                let eff_load = cargo_load_factor as f64;
                let pax_dem = demand.div(total_tpd as f64 * eff_load);

                let alg = if let ConfigAlgorithm::Cargo(a) = conf_alg {
                    *a
                } else {
                    CargoConfigAlgorithm::Auto
                };

                let cfg =
                    CargoConfig::calculate(pax_dem, ac.capacity, training_l, training_h, alg)?;
                let var = ConfigVariant::Cargo(cfg);
                let inc = metrics::revenue(
                    &var,
                    ticket,
                    ac.capacity,
                    cargo_load_factor,
                    training_l,
                    training_h,
                );
                Some((var, inc))
            }
        }
    };

    match (&sched_strat.trips_per_day, &sched_strat.num_aircraft) {
        (TripsPerDayStrategy::Strict(_), NumAircraftStrategy::Strict(n)) => {
            let total_tpd = (tpd_per_ac.get() as u32) * (n.get() as u32);
            if let Some((c, i)) = try_solve(total_tpd) {
                return Some(SolverResult {
                    config: c,
                    revenue_per_trip: i,
                    tpd: tpd_per_ac,
                    num_ac: *n,
                });
            }
        }
        (TripsPerDayStrategy::Maximise, NumAircraftStrategy::Strict(n)) => {
            let mut t: u8 = tpd_per_ac.into();
            while t > 0 {
                let total_tpd = (t as u32) * (n.get() as u32);
                if let Some((c, i)) = try_solve(total_tpd) {
                    return Some(SolverResult {
                        config: c,
                        revenue_per_trip: i,
                        tpd: TripsPerDay::new(t).unwrap(), // never zero
                        num_ac: *n,
                    });
                }
                t -= 1;
            }
        }
        (TripsPerDayStrategy::Strict(_), NumAircraftStrategy::Maximise)
        | (TripsPerDayStrategy::Maximise, NumAircraftStrategy::Maximise) => {
            let mut best_res: Option<SolverResult> = None;
            let mut start_t: u8 = tpd_per_ac.into();
            let end_t: u8 = 1;

            let is_strict_tpd = matches!(sched_strat.trips_per_day, TripsPerDayStrategy::Strict(_));

            while start_t >= end_t {
                let current_tpd = TripsPerDay::new(start_t).unwrap();

                // if 1 AC works, we search upwards for N
                if let Some((mut best_c, mut best_i)) = try_solve(start_t as u32) {
                    let mut best_n: u8 = 1;
                    let min_revenue_threshold = best_i * (1.0 - revenue_loss_tol);

                    for n in 2..200u8 {
                        let total_tpd = (start_t as u32) * (n as u32);
                        if let Some((c, i)) = try_solve(total_tpd) {
                            if i < min_revenue_threshold {
                                break;
                            }
                            best_c = c;
                            best_i = i;
                            best_n = n;
                        } else {
                            break;
                        }
                    }
                    best_res = Some(SolverResult {
                        config: best_c,
                        revenue_per_trip: best_i,
                        tpd: current_tpd,
                        num_ac: NumAircraft::new(best_n).unwrap(),
                    });
                    break; // found the highest TPD that allows at least 1 AC.
                }

                if is_strict_tpd {
                    break;
                }
                start_t -= 1;
            }
            return best_res;
        }
    }

    None
}

#[derive(Debug, Clone, Error)]
pub enum SearchConfigError {
    #[error("filter error: {0}")]
    DistanceFilterError(FilterError<DistanceError>),
    #[error("filter error: {0}")]
    FlightTimeFilterError(FilterError<FlightTimeError>),
}

#[derive(Debug, Default)]
pub struct SearchConfig<'a> {
    pub user_settings: &'a Settings,
    pub distance_filter: Filter<Distance>,
    pub flight_time_filter: Filter<FlightTime>,
    pub schedule: ScheduleStrategy,
    pub config: ConfigAlgorithm,
    pub ci: CiStrategy,
    pub sort_by: SortBy,
    pub inflate_distance_with_stopover: bool,
}

pub type TripsPerDay = NonZeroU8;
pub type NumAircraft = NonZeroU8;

#[derive(Debug, Clone, Display, Default)]
pub enum TripsPerDayStrategy {
    #[default]
    Maximise,
    Strict(TripsPerDay),
}

#[derive(Debug, Clone, Display)]
pub enum NumAircraftStrategy {
    Maximise,
    Strict(NumAircraft),
}

impl Default for NumAircraftStrategy {
    fn default() -> Self {
        Self::Strict(NonZeroU8::new(1).unwrap())
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleStrategy {
    pub trips_per_day: TripsPerDayStrategy,
    pub num_aircraft: NumAircraftStrategy,
}

impl Default for ScheduleStrategy {
    fn default() -> Self {
        Self {
            trips_per_day: TripsPerDayStrategy::Maximise,
            num_aircraft: NumAircraftStrategy::Strict(NonZeroU8::new(1).unwrap()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CiStrategy {
    Strict(Ci),
    AlignConstraint,
}

impl Default for CiStrategy {
    fn default() -> Self {
        Self::Strict(Ci::default())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum SortBy {
    #[default]
    ProfitPerAcPerDay,
    ProfitPerTrip,
}
