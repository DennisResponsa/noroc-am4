use std::sync::LazyLock;

use derive_more::{Constructor, Display, From, Into};
use thiserror::Error;
use uuid::Uuid;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    #[error("invalid wear training, must be between 0 and 5 (inclusive)")]
    InvalidWearTraining,
    #[error("invalid repair training, must be between 0 and 5 (inclusive)")]
    InvalidRepairTraining,
    #[error("invalid large training, must be between 0 and 6 (inclusive)")]
    InvalidLargeTraining,
    #[error("invalid heavy training, must be between 0 and 6 (inclusive)")]
    InvalidHeavyTraining,
    #[error("invalid fuel training, must be between 0 and 3 (inclusive)")]
    InvalidFuelTraining,
    #[error("invalid co2 training, must be between 0 and 5 (inclusive)")]
    InvalidCo2Training,
    #[error("invalid aircraft load, must be between 0.1 and 1.5")]
    InvalidAircraftLoad,
    #[error("invalid revenue loss tolerance, must be between 0.0 and 1.0")]
    InvalidRevenueLossTol,
}

// TODO: escape strings to avoid injection attacks
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub game_id: u32,
    pub game_name: String,
    pub discord_id: u64,
    pub role: Role,
    // NOTE: intentionally not putting in settings to avoid duplicates in
    // AbstractRoutes vs. ScheduledRoutes
    pub game_mode: GameMode,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(default))]
pub struct Settings {
    pub training: Training,
    pub fuel_price: FuelPrice,
    pub co2_price: Co2Price,
    pub accumulated_count: u16,
    pub load: AircraftLoad,
    pub cargo_load: AircraftLoad,
    pub revenue_loss_tol: RevenueLossTol,
    pub default_4x: bool,
    pub default_speed_mod: bool,
    pub default_fuel_mod: bool,
    pub default_co2_mod: bool,
    pub airport_code_pref: AirportCodePref,
    pub allow_invalid_tpd: bool,
    pub csv_time_format: CsvTimeFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CsvTimeFormat {
    #[default]
    HhMmSs,
    Decimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AirportCodePref {
    #[default]
    Iata,
    Icao,
}

static DEFAULT_SETTINGS: LazyLock<Settings> = LazyLock::new(Settings::default);

impl Default for &Settings {
    fn default() -> Self {
        &DEFAULT_SETTINGS
    }
}

// TODO: impl FromStr for fuel and co2

/// The assumed fuel price, for use in profit calculations
#[derive(Debug, Clone, Copy, PartialEq, From, Into, Constructor)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FuelPrice(u16);

impl FuelPrice {
    pub const fn get(&self) -> f32 {
        self.0 as f32
    }
}

/// The assumed CO₂, for use in profit calculations
#[derive(Debug, Clone, Copy, PartialEq, From, Into, Constructor)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Co2Price(u16);

impl Co2Price {
    pub const fn get(&self) -> f32 {
        self.0 as f32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum GameMode {
    #[default]
    Easy,
    Realism,
}

impl GameMode {
    pub const fn speed_multiplier(&self) -> f32 {
        match self {
            Self::Easy => 1.5,
            Self::Realism => 1.0,
        }
    }

    pub const fn acheck_cost_multiplier(&self) -> f32 {
        match self {
            Self::Easy => 1.0,
            Self::Realism => 2.0,
        }
    }

    pub const fn contribution_multiplier(&self) -> f32 {
        match self {
            Self::Easy => 1.0,
            Self::Realism => 1.5,
        }
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Training {
    pub wear: WearTraining,
    pub repair: RepairTraining,
    pub l: LargeTraining,
    pub h: HeavyTraining,
    pub fuel: FuelTraining,
    pub co2: Co2Training,
}

macro_rules! create_newtype {
    ($name:ident, $inner_type:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Display, Into)]
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        pub struct $name($inner_type);
    };
}

macro_rules! impl_constructor {
    ($name:ident, $inner_type:ty, $condition:expr, $err_variant:ident) => {
        impl $name {
            #[doc = "Creates a new value with bounds checking."]
            pub fn new(value: $inner_type) -> Result<Self, ValidationError> {
                if $condition(value) {
                    Ok(Self(value))
                } else {
                    Err(ValidationError::$err_variant)
                }
            }
        }

        impl TryFrom<$inner_type> for $name {
            type Error = ValidationError;

            fn try_from(value: $inner_type) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

macro_rules! impl_default {
    ($name:ident, $default_value:expr) => {
        impl Default for $name {
            #[doc = concat!("Returns the default value of `", stringify!($default_value), "`")]
            fn default() -> Self {
                Self($default_value)
            }
        }
    };
}

macro_rules! create_validated_newtype {
    ($name:ident, $inner_type:ty, $condition:expr, $err_variant:ident, $default_value:expr) => {
        create_newtype!($name, $inner_type);
        impl_constructor!($name, $inner_type, $condition, $err_variant);
        impl_default!($name, $default_value);

        impl $name {
            pub const fn get(&self) -> $inner_type {
                self.0
            }
        }
    };
}

impl_default!(FuelPrice, 900);
impl_default!(Co2Price, 120);
create_validated_newtype!(
    AircraftLoad,
    f32,
    |v: f32| v.is_finite() && (0.1..=1.5).contains(&v),
    InvalidAircraftLoad,
    0.99
);
create_validated_newtype!(
    RevenueLossTol,
    f32,
    |v: f32| v.is_finite() && (0.0..=1.0).contains(&v),
    InvalidRevenueLossTol,
    0.0
);
create_validated_newtype!(WearTraining, u8, |v| v <= 5, InvalidWearTraining, 0);
create_validated_newtype!(RepairTraining, u8, |v| v <= 5, InvalidRepairTraining, 0);
create_validated_newtype!(LargeTraining, u8, |v| v <= 6, InvalidLargeTraining, 0);
create_validated_newtype!(HeavyTraining, u8, |v| v <= 6, InvalidHeavyTraining, 0);
create_validated_newtype!(FuelTraining, u8, |v| v <= 3, InvalidFuelTraining, 0);
create_validated_newtype!(Co2Training, u8, |v| v <= 5, InvalidCo2Training, 0);

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Role {
    #[default]
    User,
    TrustedUser,
    TrustedUser2,
    TopAllianceMember,
    TopAllianceAdmin,
    Helper,
    Moderator,
    Admin,
    GlobalAdmin,
}
