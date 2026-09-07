use crate::aircraft::EnginePriority;
use crate::aircraft::{Aircraft, AircraftError};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// An container holding the base [Aircraft] and its [Modification]s.
///
/// To get the aircraft with modifiers applied (e.g. modified speed, fuel, cost),
/// use [CustomAircraft::effective].
#[derive(Debug, Clone, PartialEq)]
pub struct CustomAircraft {
    pub aircraft: Aircraft,
    pub modifiers: Modification,
}

/// A bitset of the specific modification and engine variant
#[derive(Debug, Clone, PartialEq)]
pub struct Modification {
    pub mods: HashSet<Modifier>, // not using Vec to avoid duplicates
    pub engine: EnginePriority,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Modifier {
    Speed,
    Fuel,
    Co2,
    FourX,
    EasyBoost,
}

impl CustomAircraft {
    /// Create a new `CustomAircraft` from a base aircraft and modifiers.
    pub fn new(aircraft: Aircraft, modifiers: Modification) -> Self {
        Self {
            aircraft,
            modifiers,
        }
    }

    /// Apply the modifiers to the base aircraft and return the result.
    pub fn effective(&self) -> Aircraft {
        let mut ac = self.aircraft.clone();
        let mut cost_mul = 1.0;

        for modifier in self.modifiers.mods.iter() {
            modifier.apply(&mut ac);
            cost_mul *= modifier.cost_multiplier();
        }
        ac.cost = (ac.cost as f32 * cost_mul).ceil() as u32;
        ac
    }
}

impl Modifier {
    fn apply(&self, aircraft: &mut Aircraft) {
        match self {
            Modifier::Speed => aircraft.speed *= 1.1,
            Modifier::Fuel => aircraft.fuel *= 0.9,
            Modifier::Co2 => aircraft.co2 *= 0.9,
            Modifier::FourX => aircraft.speed *= 4.0,
            Modifier::EasyBoost => aircraft.speed *= 1.5,
        }
    }

    fn cost_multiplier(&self) -> f32 {
        match self {
            Modifier::Speed => 1.07,
            Modifier::Fuel => 1.10,
            Modifier::Co2 => 1.05,
            Modifier::FourX | Modifier::EasyBoost => 1.0,
        }
    }
}

impl Default for Modification {
    fn default() -> Self {
        Modification {
            mods: HashSet::new(),
            engine: EnginePriority(0),
        }
    }
}

impl Modification {
    pub fn is_empty(&self) -> bool {
        self.engine.get() == 0 && self.mods.is_empty()
    }
}

impl Display for Modification {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.engine.get() != 0 {
            write!(f, "{}", self.engine.get())?;
        }

        if self.mods.contains(&Modifier::Speed) {
            f.write_str("s")?;
        }
        if self.mods.contains(&Modifier::Fuel) {
            f.write_str("f")?;
        }
        if self.mods.contains(&Modifier::Co2) {
            f.write_str("c")?;
        }
        if self.mods.contains(&Modifier::FourX) {
            f.write_str("x")?;
        }
        if self.mods.contains(&Modifier::EasyBoost) {
            f.write_str("e")?;
        }

        Ok(())
    }
}

impl Display for CustomAircraft {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.aircraft.shortname)?;
        if !self.modifiers.is_empty() {
            write!(f, "[{}]", self.modifiers)?;
        }
        Ok(())
    }
}

impl FromStr for Modification {
    type Err = AircraftError;

    /// Parse modifiers between square brackets: engine priority (digits) and modifier letters.
    /// Examples: "1", "sfc", "1sfc", "10", "2sfcx"
    fn from_str(s: &str) -> Result<Modification, Self::Err> {
        let mut modifi = Modification::default();
        let s_lower = s.to_lowercase();

        let mut digit_start = 0;
        let mut digit_end = 0;
        let mut found_digit = false;

        for (i, c) in s_lower.chars().enumerate() {
            if c.is_ascii_digit() {
                if !found_digit {
                    digit_start = i;
                    found_digit = true;
                }
                digit_end = i + 1;
            }
        }

        if found_digit {
            let digit_str = &s_lower[digit_start..digit_end];
            modifi.engine = EnginePriority::from_str(digit_str)?;
        }

        for c in s_lower.chars() {
            match c {
                's' => modifi.mods.insert(Modifier::Speed),
                'f' => modifi.mods.insert(Modifier::Fuel),
                'c' => modifi.mods.insert(Modifier::Co2),
                'x' => modifi.mods.insert(Modifier::FourX),
                'e' => modifi.mods.insert(Modifier::EasyBoost),
                ' ' | ',' => continue,
                _ => true,
            };
        }

        Ok(modifi)
    }
}
