use crate::aircraft::Aircraft;
use crate::airport::Airport;
use crate::route::metrics;
use crate::route::search::{AbstractRoute, Routes};
use crate::route::{Ci, Distance, FlightTime, Speed};
use crate::user::{GameMode, Settings};

/// Represents a ferry flight (selling an aircraft at a destination).
///
/// - No passengers or cargo (so no CO2, demand, or configuration).
/// - Can fly to *any* airport within 2x range.
#[derive(Debug, Clone)]
pub struct FerryRoute<'a> {
    pub destination: &'a Airport,
    pub direct_distance: Distance,
    pub flight_time: FlightTime,
    pub fuel: f32,
    pub sale_price: f32,
    pub profit: f32,
}

#[derive(Debug, Clone)]
pub struct FerryConfig<'a> {
    pub aircraft: &'a Aircraft,
    pub settings: &'a Settings,
}

pub type FerryRoutes<'a> = Routes<'a, FerryRoute<'a>, FerryConfig<'a>>;

impl<'a> FerryRoutes<'a> {
    pub fn new(
        abstract_routes: Vec<AbstractRoute<'a>>,
        aircraft: &'a Aircraft,
        settings: &'a Settings,
        game_mode: GameMode,
    ) -> Self {
        let mut routes = Vec::with_capacity(abstract_routes.len());
        let speed_mult = game_mode.speed_multiplier();
        let range_limit = aircraft.range as f32 * 2.0;

        for ar in abstract_routes {
            if ar.direct_distance.get() > range_limit {
                continue;
            }

            let flight_time =
                ar.direct_distance / Speed::new_unchecked(aircraft.speed * speed_mult);
            let fuel = metrics::fuel(
                aircraft.fuel,
                ar.direct_distance,
                settings.training.fuel,
                Ci::MAX,
            );

            let market_pct = ar.destination.market as f32 / 100.0;
            let sale_price = aircraft.cost as f32 * market_pct;
            let profit = sale_price - fuel * settings.fuel_price.get() / 1000.0;

            routes.push(FerryRoute {
                destination: ar.destination,
                direct_distance: ar.direct_distance,
                flight_time,
                fuel,
                sale_price,
                profit,
            });
        }

        routes.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());

        Self {
            routes,
            errors: Vec::new(),
            config: FerryConfig { aircraft, settings },
        }
    }
}
