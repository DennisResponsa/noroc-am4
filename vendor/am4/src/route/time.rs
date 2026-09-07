use crate::utils::{PositiveReal, PositiveRealError};
use derive_more::{Add, Display, Into};
use std::num::ParseFloatError;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum FlightTimeError {
    #[error("not a valid float: {0}")]
    ParseError(#[source] ParseFloatError),
    #[error(transparent)]
    FloatError(#[from] PositiveRealError),
    #[error("invalid format: expected HH:MM or HH:MM:SS")]
    InvalidFormat,
}

/// Flight time, hrs
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Add, Into, Display)]
pub struct FlightTime(f32);

impl FlightTime {
    pub const MIN: Self = Self(0.1);
    pub const MAX: Self = Self(72.);

    pub fn new_unchecked(value: f32) -> Self {
        Self(value)
    }

    pub const fn get(&self) -> f32 {
        self.0
    }
}

impl PositiveReal for FlightTime {}

impl TryFrom<f32> for FlightTime {
    type Error = PositiveRealError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::validate_positive_real(value)?;
        Ok(Self(value))
    }
}

impl FromStr for FlightTime {
    type Err = FlightTimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains(':') {
            let parts: Vec<&str> = s.split(':').collect();
            let hours = match parts.as_slice() {
                [h, m] => {
                    let h = h.parse::<f32>().map_err(FlightTimeError::ParseError)?;
                    let m = m.parse::<f32>().map_err(FlightTimeError::ParseError)?;
                    h + m / 60.0
                }
                [h, m, s] => {
                    let h = h.parse::<f32>().map_err(FlightTimeError::ParseError)?;
                    let m = m.parse::<f32>().map_err(FlightTimeError::ParseError)?;
                    let s = s.parse::<f32>().map_err(FlightTimeError::ParseError)?;
                    h + m / 60.0 + s / 3600.0
                }
                _ => return Err(FlightTimeError::InvalidFormat),
            };
            return Self::try_from(hours).map_err(Into::into);
        }
        let value = s.parse::<f32>().map_err(FlightTimeError::ParseError)?;
        Self::try_from(value).map_err(Into::into)
    }
}
