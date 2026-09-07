#![allow(clippy::excessive_precision)]
use crate::user::GameMode;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaxTicket {
    pub y: u16,
    pub j: u16,
    pub f: u16,
}

impl PaxTicket {
    const fn base_easy(direct_distance: f32) -> (f32, f32, f32) {
        (
            0.4 * direct_distance + 170.0,
            0.8 * direct_distance + 560.0,
            1.2 * direct_distance + 1200.0,
        )
    }

    const fn base_realism(direct_distance: f32) -> (f32, f32, f32) {
        (
            0.3 * direct_distance + 150.0,
            0.6 * direct_distance + 500.0,
            0.9 * direct_distance + 1000.0,
        )
    }

    const fn make_optimal((y, j, f): (f32, f32, f32)) -> Self {
        Self {
            y: (1.10 * y) as u16 - 2,
            j: (1.08 * j) as u16 - 2,
            f: (1.06 * f) as u16 - 2,
        }
    }

    pub const fn from_optimal(direct_distance: f32, game_mode: GameMode) -> Self {
        PaxTicket::make_optimal(match game_mode {
            GameMode::Easy => PaxTicket::base_easy(direct_distance),
            GameMode::Realism => PaxTicket::base_realism(direct_distance),
        })
    }

    pub const fn from_optimal_vip(distance: f32, game_mode: GameMode) -> Self {
        let (y_base, j_base, f_base) = match game_mode {
            GameMode::Easy => PaxTicket::base_easy(distance),
            GameMode::Realism => PaxTicket::base_realism(distance),
        };
        let vip = 1.7489;
        Self {
            y: (1.22 * vip * y_base) as u16 - 2,
            j: (1.20 * vip * j_base) as u16 - 2,
            f: (1.17 * vip * f_base) as u16 - 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CargoTicket {
    pub l: f32,
    pub h: f32,
}

impl CargoTicket {
    pub const fn from_optimal(direct_distance: f32, game_mode: GameMode) -> Self {
        match game_mode {
            GameMode::Easy => Self {
                l: (1.10 * (0.0948283724581252 * direct_distance + 85.2045432642377)).floor()
                    / 100.0,
                h: (1.08 * (0.0689663577640275 * direct_distance + 28.2981124272893)).floor()
                    / 100.0,
            },
            GameMode::Realism => Self {
                l: (1.10 * (0.0776321822039374 * direct_distance + 85.0567600367807)).floor()
                    / 100.0,
                h: (1.08 * (0.0517742799409248 * direct_distance + 24.6369915396414)).floor()
                    / 100.0,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ticket {
    Pax(PaxTicket),
    Cargo(CargoTicket),
    VIP(PaxTicket),
}
