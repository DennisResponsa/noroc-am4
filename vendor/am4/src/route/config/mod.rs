//! Implements greedy configuration algorithms for pax and cargo aircraft

mod cargo;
mod pax;

pub use cargo::{CargoConfig, CargoConfigAlgorithm};
pub use pax::{PaxConfig, PaxConfigAlgorithm};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum ConfigAlgorithm {
    #[default]
    Auto,
    Pax(pax::PaxConfigAlgorithm),
    Cargo(cargo::CargoConfigAlgorithm),
}

impl std::fmt::Display for ConfigAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::Pax(p) => write!(f, "{}", p),
            Self::Cargo(c) => write!(f, "{}", c),
        }
    }
}

impl std::str::FromStr for ConfigAlgorithm {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s == "auto" {
            return Ok(Self::Auto);
        }
        if let Ok(p) = pax::PaxConfigAlgorithm::from_str(&s) {
            if p != pax::PaxConfigAlgorithm::Auto {
                return Ok(Self::Pax(p));
            }
        }
        if let Ok(c) = cargo::CargoConfigAlgorithm::from_str(&s) {
            if c != cargo::CargoConfigAlgorithm::Auto {
                return Ok(Self::Cargo(c));
            }
        }
        Err(())
    }
}

// TODO: redirect auto to pax and cargo respectively
