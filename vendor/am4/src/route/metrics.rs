use crate::route::config::{CargoConfig, PaxConfig};
use crate::route::ticket::Ticket;
use crate::route::{Ci, Distance, FlightTime};
use crate::user::{
    Co2Training, FuelTraining, GameMode, HeavyTraining, LargeTraining, RepairTraining,
};

#[inline]
pub fn fuel(ac_fuel: f32, total_distance: Distance, training: FuelTraining, ci: Ci) -> f32 {
    let training_factor = 1.0 - training.get() as f32 / 100.0;
    let dist_val = (total_distance.get() * 100.0).ceil() / 100.0;
    let ci_factor = ci.get() as f32 / 500.0 + 0.6;
    training_factor * dist_val * ac_fuel * ci_factor
}

#[inline]
pub fn co2_pax(
    ac_co2: f32,
    cfg: &PaxConfig,
    total_distance: Distance,
    training: Co2Training,
    load_factor: f32,
    ci: Ci,
) -> f32 {
    let training_factor = 1.0 - training.get() as f32 / 100.0;
    let dist_val = (total_distance.get() * 100.0).ceil() / 100.0;
    let ci_factor = ci.get() as f32 / 2000.0 + 0.9;

    let seats_total = (cfg.y + cfg.j + cfg.f) as f32;
    let pax_mass = (cfg.y as f32 + cfg.j as f32 * 2.0 + cfg.f as f32 * 3.0) * load_factor;

    training_factor * (dist_val * ac_co2 * pax_mass + seats_total) * ci_factor
}

#[inline]
pub fn co2_cargo(
    ac_co2: f32,
    ac_capacity: u32,
    cfg: &CargoConfig,
    total_distance: Distance,
    training: Co2Training,
    load_factor: f32,
    ci: Ci,
) -> f32 {
    let training_factor = 1.0 - training.get() as f32 / 100.0;
    let dist_val = (total_distance.get() * 100.0).ceil() / 100.0;
    let ci_factor = ci.get() as f32 / 2000.0 + 0.9;

    let cap = ac_capacity as f32;
    let l_pct = cfg.l as f32 / 100.0;
    let h_pct = cfg.h as f32 / 100.0;

    let mass_term = (l_pct * 0.7 / 1000.0 + h_pct / 500.0) * load_factor * cap;
    let capacity_term = (l_pct * 0.7 + h_pct) * cap;

    training_factor * (dist_val * ac_co2 * mass_term + capacity_term) * ci_factor
}

#[inline]
pub fn contribution(total_distance: Distance, game_mode: GameMode, ci: Ci) -> f32 {
    let d = total_distance.get();
    let k = if d > 10000.0 {
        0.0048
    } else if d > 6000.0 {
        0.0032
    } else {
        0.0064
    };

    let base = (k * d * (3.0 - ci.get() as f32 / 100.0)).min(152.0);
    base * game_mode.contribution_multiplier() * 0.875
}

#[inline]
pub fn acheck_cost(
    ac_check_cost: u32,
    ac_maint: u16,
    flight_time: FlightTime,
    game_mode: GameMode,
) -> f32 {
    let mode_mult = game_mode.acheck_cost_multiplier();
    let speed_mult = game_mode.speed_multiplier();

    (ac_check_cost as f32 * mode_mult) * (flight_time.get() * speed_mult).ceil() / ac_maint as f32
}

#[inline]
pub fn repair_cost(ac_cost: u32, training: RepairTraining) -> f32 {
    let wear_reduction = 1.0 - 2.0 * training.get() as f32 / 100.0;
    ac_cost as f32 / 1000.0 * 0.0075 * wear_reduction
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigVariant {
    Pax(PaxConfig),
    Cargo(CargoConfig),
}

/// Calculate revenue based on the configuration, ticket price, and user training/load settings.
#[inline]
pub fn revenue(
    cfg: &ConfigVariant,
    ticket: &Ticket,
    ac_capacity: u32,
    load_factor: f32,
    training_l: LargeTraining,
    training_h: HeavyTraining,
) -> f32 {
    match (cfg, ticket) {
        (ConfigVariant::Pax(c), Ticket::Pax(t)) | (ConfigVariant::Pax(c), Ticket::VIP(t)) => {
            let y_inc = c.y as f32 * t.y as f32;
            let j_inc = c.j as f32 * t.j as f32;
            let f_inc = c.f as f32 * t.f as f32;
            (y_inc + j_inc + f_inc) * load_factor
        }
        (ConfigVariant::Cargo(c), Ticket::Cargo(t)) => {
            let l_factor = 1.0 + training_l.get() as f32 / 100.0;
            let h_factor = 1.0 + training_h.get() as f32 / 100.0;

            let term_l = l_factor * (c.l as f32 / 100.0) * 0.7 * t.l;
            let term_h = h_factor * (c.h as f32 / 100.0) * t.h;

            (term_l + term_h) * (ac_capacity as f32) * load_factor
        }
        _ => 0.0,
    }
}
