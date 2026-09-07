use crate::route::demand::PaxDemand;
use crate::user::GameMode;
use derive_more::Display;
use std::cmp::min;

#[derive(Debug, Clone, Copy, PartialEq, Display)]
#[display("Y{y} J{j} F{f}")]
pub struct PaxConfig {
    pub y: u16,
    pub j: u16,
    pub f: u16,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum PaxConfigAlgorithm {
    #[default]
    Auto,
    Fjy,
    Fyj,
    Jfy,
    Jyf,
    Yfj,
    Yjf,
    YOnly,
    JOnly,
    FOnly,
    Spread,
}

impl std::fmt::Display for PaxConfigAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Fjy => write!(f, "fjy"),
            Self::Fyj => write!(f, "fyj"),
            Self::Jfy => write!(f, "jfy"),
            Self::Jyf => write!(f, "jyf"),
            Self::Yfj => write!(f, "yfj"),
            Self::Yjf => write!(f, "yjf"),
            Self::YOnly => write!(f, "y"),
            Self::JOnly => write!(f, "j"),
            Self::FOnly => write!(f, "f"),
            Self::Spread => write!(f, "spread"),
        }
    }
}

impl std::str::FromStr for PaxConfigAlgorithm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "fjy" => Ok(Self::Fjy),
            "fyj" => Ok(Self::Fyj),
            "jfy" => Ok(Self::Jfy),
            "jyf" => Ok(Self::Jyf),
            "yfj" => Ok(Self::Yfj),
            "yjf" => Ok(Self::Yjf),
            "y" => Ok(Self::YOnly),
            "j" => Ok(Self::JOnly),
            "f" => Ok(Self::FOnly),
            "spread" => Ok(Self::Spread),
            _ => Err(()),
        }
    }
}

impl PaxConfig {
    /// Implements a greedy configuration algorithm for pax aircraft.
    /// Returns None if demand is exhausted.
    pub fn calculate(
        d_pf: PaxDemand,
        capacity: u16,
        distance: f32,
        game_mode: GameMode,
        algorithm: PaxConfigAlgorithm,
    ) -> Option<PaxConfig> {
        match algorithm {
            PaxConfigAlgorithm::Auto => match game_mode {
                GameMode::Easy => {
                    if distance < 14425. {
                        Self::from_fjy(d_pf, capacity)
                    } else if distance < 14812.5 {
                        Self::from_fyj(d_pf, capacity)
                    } else if distance < 15200. {
                        Self::from_yfj(d_pf, capacity)
                    } else {
                        Self::from_yjf(d_pf, capacity)
                    }
                }
                GameMode::Realism => {
                    if distance < 13888.889 {
                        Self::from_fjy(d_pf, capacity)
                    } else if distance < 15694.444 {
                        Self::from_jfy(d_pf, capacity)
                    } else if distance < 17500. {
                        Self::from_jyf(d_pf, capacity)
                    } else {
                        Self::from_yjf(d_pf, capacity)
                    }
                }
            },
            PaxConfigAlgorithm::Fjy => Self::from_fjy(d_pf, capacity),
            PaxConfigAlgorithm::Fyj => Self::from_fyj(d_pf, capacity),
            PaxConfigAlgorithm::Jfy => Self::from_jfy(d_pf, capacity),
            PaxConfigAlgorithm::Jyf => Self::from_jyf(d_pf, capacity),
            PaxConfigAlgorithm::Yfj => Self::from_yfj(d_pf, capacity),
            PaxConfigAlgorithm::Yjf => Self::from_yjf(d_pf, capacity),
            PaxConfigAlgorithm::YOnly => Self::from_y_only(d_pf, capacity),
            PaxConfigAlgorithm::JOnly => Self::from_j_only(d_pf, capacity),
            PaxConfigAlgorithm::FOnly => Self::from_f_only(d_pf, capacity),
            PaxConfigAlgorithm::Spread => Self::from_spread(d_pf, capacity),
        }
    }

    fn from_fjy(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let mut remaining_capacity = capacity;

        let f = min(d_pf.f, remaining_capacity / 3);
        remaining_capacity -= f * 3;

        let j = min(d_pf.j, remaining_capacity / 2);
        remaining_capacity -= j * 2;

        let y = remaining_capacity;

        if y < d_pf.y {
            Some(PaxConfig { f, j, y })
        } else {
            None
        }
    }

    fn from_fyj(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let mut remaining_capacity = capacity;

        let f = min(d_pf.f, remaining_capacity / 3);
        remaining_capacity -= f * 3;

        let y = min(d_pf.y, remaining_capacity);
        remaining_capacity -= y;

        let j = remaining_capacity / 2;

        if j < d_pf.j {
            Some(PaxConfig { f, y, j })
        } else {
            None
        }
    }

    fn from_jfy(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let mut remaining_capacity = capacity;

        let j = min(d_pf.j, remaining_capacity / 2);
        remaining_capacity -= j * 2;

        let f = min(d_pf.f, remaining_capacity / 3);
        remaining_capacity -= f * 3;

        let y = remaining_capacity;

        if y < d_pf.y {
            Some(PaxConfig { j, f, y })
        } else {
            None
        }
    }

    fn from_jyf(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let mut remaining_capacity = capacity;

        let j = min(d_pf.j, remaining_capacity / 2);
        remaining_capacity -= j * 2;

        let y = min(d_pf.y, remaining_capacity);
        remaining_capacity -= y;

        let f = remaining_capacity / 3;

        if f < d_pf.f {
            Some(PaxConfig { j, y, f })
        } else {
            None
        }
    }

    fn from_yfj(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let mut remaining_capacity = capacity;

        let y = min(d_pf.y, remaining_capacity);
        remaining_capacity -= y;

        let f = min(d_pf.f, remaining_capacity / 3);
        remaining_capacity -= f * 3;

        let j = remaining_capacity / 2;

        if j < d_pf.j {
            Some(PaxConfig { y, f, j })
        } else {
            None
        }
    }

    fn from_yjf(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let mut remaining_capacity = capacity;

        let y = min(d_pf.y, remaining_capacity);
        remaining_capacity -= y;

        let j = min(d_pf.j, remaining_capacity / 2);
        remaining_capacity -= j * 2;

        let f = remaining_capacity / 3;

        if f < d_pf.f {
            Some(PaxConfig { y, j, f })
        } else {
            None
        }
    }

    fn from_y_only(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        if capacity < d_pf.y {
            Some(PaxConfig {
                y: capacity,
                j: 0,
                f: 0,
            })
        } else {
            None
        }
    }

    fn from_j_only(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let j = capacity / 2;
        if j < d_pf.j {
            Some(PaxConfig { y: 0, j, f: 0 })
        } else {
            None
        }
    }

    fn from_f_only(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let f = capacity / 3;
        if f < d_pf.f {
            Some(PaxConfig { y: 0, j: 0, f })
        } else {
            None
        }
    }

    fn from_spread(d_pf: PaxDemand, capacity: u16) -> Option<Self> {
        let eq_demand = d_pf.equivalent() as f32;
        if eq_demand <= capacity as f32 {
            return None;
        }

        let k = capacity as f32 / eq_demand;
        let y = (d_pf.y as f32 * k).floor() as u16;
        let j = (d_pf.j as f32 * k).floor() as u16;
        let f = (d_pf.f as f32 * k).floor() as u16;

        Some(PaxConfig { y, j, f })
    }
}
