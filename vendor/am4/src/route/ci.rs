use crate::route::{Distance, FlightTime, Speed};
use derive_more::{Add, Display, Into};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("ci must be less than or equal to 200")]
    Gt200,
}

/// Cost index, nondimensional
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Add, Into, Display)]
pub struct Ci(u8);

impl Ci {
    pub const MAX: Self = Self(200);

    pub fn new(val: u8) -> Result<Self, ValidationError> {
        if val > 200 {
            Err(ValidationError::Gt200)
        } else {
            Ok(Self(val))
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }

    /// Calculate the Cost Index required to achieve a target flight time.
    /// Note that the result is ceiled to avoid the resulting flight time being greater than the target time.
    pub fn calculate(dist: Distance, base_speed: Speed, target_time: FlightTime) -> Self {
        let d = dist.get();
        let v = base_speed.get();
        let t = target_time.get();

        let ci_val = (2000.0 * d) / (7.0 * v * t) - (600.0 / 7.0);
        let ci_clamped = ci_val.clamp(0.0, 200.0).ceil() as u8;
        Self(ci_clamped)
    }
}

impl Default for Ci {
    fn default() -> Self {
        Self::MAX
    }
}

impl TryFrom<u8> for Ci {
    type Error = ValidationError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
