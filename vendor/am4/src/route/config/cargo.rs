use crate::route::demand::{CargoDemand, PaxDemand};
use crate::user::{HeavyTraining, LargeTraining};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CargoConfig {
    pub l: u8,
    pub h: u8,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum CargoConfigAlgorithm {
    #[default]
    Auto,
    L,
    H,
    LOnly,
    HOnly,
    Spread,
}

impl std::fmt::Display for CargoConfigAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::L => write!(f, "lh"),
            Self::H => write!(f, "hl"),
            Self::LOnly => write!(f, "l"),
            Self::HOnly => write!(f, "h"),
            Self::Spread => write!(f, "spread"),
        }
    }
}

impl std::str::FromStr for CargoConfigAlgorithm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "lh" => Ok(Self::L),
            "hl" => Ok(Self::H),
            "l" => Ok(Self::LOnly),
            "h" => Ok(Self::HOnly),
            "spread" => Ok(Self::Spread),
            _ => Err(()),
        }
    }
}

impl CargoConfig {
    fn calc_l_conf(
        d_pf: PaxDemand,
        capacity: u32,
        l_training: LargeTraining,
        h_training: HeavyTraining,
    ) -> Option<Self> {
        let d_pf_cargo = CargoDemand::from(&d_pf);

        let l_cap = capacity as f32 * 0.7 * (1.0 + l_training.get() as f32 / 100.0);

        if d_pf_cargo.l as f32 > l_cap {
            return Some(CargoConfig { l: 100, h: 0 });
        }

        let l = d_pf_cargo.l as f32 / l_cap;
        let h = 1. - l;
        if (d_pf_cargo.h as f32) < capacity as f32 * h * (1.0 + h_training.get() as f32 / 100.0) {
            None
        } else {
            let lu = (l * 100.0) as u8;

            Some(CargoConfig { l: lu, h: 100 - lu })
        }
    }

    fn calc_h_conf(
        d_pf: PaxDemand,
        capacity: u32,
        l_training: LargeTraining,
        h_training: HeavyTraining,
    ) -> Option<Self> {
        let d_pf_cargo = CargoDemand::from(&d_pf);

        let h_cap = capacity as f32 * (1.0 + h_training.get() as f32 / 100.0);

        if d_pf_cargo.h as f32 > h_cap {
            return Some(CargoConfig { l: 0, h: 100 });
        }

        let h = d_pf_cargo.h as f32 / h_cap;
        let l = 1. - h;
        if (d_pf_cargo.l as f32)
            < capacity as f32 * l * 0.7 * (1.0 + l_training.get() as f32 / 100.0)
        {
            None
        } else {
            let hu = (h * 100.0) as u8;

            Some(CargoConfig { l: 100 - hu, h: hu })
        }
    }

    fn calc_l_only(
        d_pf: PaxDemand,
        capacity: u32,
        l_training: LargeTraining,
        _h_training: HeavyTraining,
    ) -> Option<Self> {
        let d_pf_cargo = CargoDemand::from(&d_pf);
        let l_cap = capacity as f32 * 0.7 * (1.0 + l_training.get() as f32 / 100.0);

        if d_pf_cargo.l as f32 > l_cap {
            Some(CargoConfig { l: 100, h: 0 })
        } else {
            None
        }
    }

    fn calc_h_only(
        d_pf: PaxDemand,
        capacity: u32,
        _l_training: LargeTraining,
        h_training: HeavyTraining,
    ) -> Option<Self> {
        let d_pf_cargo = CargoDemand::from(&d_pf);
        let h_cap = capacity as f32 * (1.0 + h_training.get() as f32 / 100.0);

        if d_pf_cargo.h as f32 > h_cap {
            Some(CargoConfig { l: 0, h: 100 })
        } else {
            None
        }
    }

    fn calc_spread(
        d_pf: PaxDemand,
        capacity: u32,
        l_training: LargeTraining,
        h_training: HeavyTraining,
    ) -> Option<Self> {
        let d_pf_cargo = CargoDemand::from(&d_pf);
        if d_pf_cargo.l == 0 && d_pf_cargo.h == 0 {
            return None;
        }

        let l_cap_factor = 0.7 * (1.0 + l_training.get() as f32 / 100.0);
        let h_cap_factor = 1.0 + h_training.get() as f32 / 100.0;

        let (l_pct, h_pct) = if d_pf_cargo.h == 0 {
            (100u8, 0u8)
        } else if d_pf_cargo.l == 0 {
            (0u8, 100u8)
        } else {
            let ratio = d_pf_cargo.l as f32 / d_pf_cargo.h as f32;
            let target_pct_ratio = ratio * h_cap_factor / l_cap_factor;
            let l_p = (100.0 * target_pct_ratio / (1.0 + target_pct_ratio)).round() as u8;
            (l_p, 100 - l_p)
        };

        let alloc_l = capacity as f32 * (l_pct as f32 / 100.0) * l_cap_factor;
        let alloc_h = capacity as f32 * (h_pct as f32 / 100.0) * h_cap_factor;

        if (d_pf_cargo.l as f32 > alloc_l) || (d_pf_cargo.h as f32 > alloc_h) {
            Some(CargoConfig { l: l_pct, h: h_pct })
        } else {
            None
        }
    }

    // Implements a greedy configuration algorithm for cargo aircraft.
    pub fn calculate(
        d_pf: PaxDemand,
        capacity: u32,
        l_training: LargeTraining,
        h_training: HeavyTraining,
        algorithm: CargoConfigAlgorithm,
    ) -> Option<Self> {
        match algorithm {
            CargoConfigAlgorithm::Auto | CargoConfigAlgorithm::L => {
                Self::calc_l_conf(d_pf, capacity, l_training, h_training)
            }
            CargoConfigAlgorithm::H => Self::calc_h_conf(d_pf, capacity, l_training, h_training),
            CargoConfigAlgorithm::LOnly => {
                Self::calc_l_only(d_pf, capacity, l_training, h_training)
            }
            CargoConfigAlgorithm::HOnly => {
                Self::calc_h_only(d_pf, capacity, l_training, h_training)
            }
            CargoConfigAlgorithm::Spread => {
                Self::calc_spread(d_pf, capacity, l_training, h_training)
            }
        }
    }
}
