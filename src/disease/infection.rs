use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InfectionStrain {
    WPV,
    OPV,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InfectionSerotype {
    Type1,
    Type2,
    Type3,
}

impl InfectionSerotype {
    pub fn from_num(n: u8) -> Option<Self> {
        match n {
            1 => Some(InfectionSerotype::Type1),
            2 => Some(InfectionSerotype::Type2),
            3 => Some(InfectionSerotype::Type3),
            _ => None,
        }
    }

    pub fn to_num(&self) -> u8 {
        match self {
            InfectionSerotype::Type1 => 1,
            InfectionSerotype::Type2 => 2,
            InfectionSerotype::Type3 => 3,
        }
    }
}

/// Format strain and serotype as "WPV2", "OPV1", etc.
pub fn format_infection_type(strain: InfectionStrain, serotype: InfectionSerotype) -> String {
    format!("{:?}{}", strain, serotype.to_num())
}

pub fn parse_infection_type(s: &str) -> Option<(InfectionStrain, InfectionSerotype)> {
    let s = s.to_ascii_uppercase();
    if s.starts_with("WPV") {
        let sero = s[3..].parse::<u8>().ok()?;
        Some((InfectionStrain::WPV, InfectionSerotype::from_num(sero)?))
    } else if s.starts_with("OPV") {
        let sero = s[3..].parse::<u8>().ok()?;
        Some((InfectionStrain::OPV, InfectionSerotype::from_num(sero)?))
    } else {
        None
    }
}

#[derive(Component)]
pub struct Infection {
    pub shed_duration: f32,
    pub viral_shedding: f32,
    pub strain: InfectionStrain,
    pub serotype: InfectionSerotype,
}

impl Infection {
    pub fn new(strain: InfectionStrain, serotype: InfectionSerotype) -> Self {
        Infection {
            shed_duration: 0.0,
            viral_shedding: 0.0,
            strain,
            serotype,
        }
    }

    pub fn should_clear(&self, days_since_infection: f32) -> bool {
        days_since_infection > self.shed_duration
    }
}
