use bevy::prelude::*;
use rand::Rng;
use rand_distr::{Exp, Distribution};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InfectionStrain {
    WPV,
    VDPV,
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

/// Format strain and serotype as "WPV2", "OPV1", "VDPV3", etc.
pub fn format_infection_type(strain: InfectionStrain, serotype: InfectionSerotype) -> String {
    format!("{:?}{}", strain, serotype.to_num())
}

pub fn parse_infection_type(s: &str) -> Option<(InfectionStrain, InfectionSerotype)> {
    let s = s.to_ascii_uppercase();
    if s.starts_with("WPV") {
        let sero = s[3..].parse::<u8>().ok()?;
        Some((InfectionStrain::WPV, InfectionSerotype::from_num(sero)?))
    } else if s.starts_with("VDPV") {
        let sero = s[4..].parse::<u8>().ok()?;
        Some((InfectionStrain::VDPV, InfectionSerotype::from_num(sero)?))
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
    /// Current mutation count (0-3, only meaningful for OPV; at 3 → becomes VDPV)
    pub mutations: u8,
    /// Days post-infection when next mutation occurs (OPV only)
    pub next_mutation_day: Option<f32>,
}

impl Infection {
    /// Create a WPV or VDPV infection (no mutation tracking)
    pub fn new(strain: InfectionStrain, serotype: InfectionSerotype) -> Self {
        Infection {
            shed_duration: 0.0,
            viral_shedding: 0.0,
            strain,
            serotype,
            mutations: 0,
            next_mutation_day: None,
        }
    }

    /// Create an OPV infection with stepwise mutation tracking.
    /// `mutations` is the inherited mutation count from the source (0 for fresh seeds).
    /// If mutations >= 3, creates a VDPV infection instead.
    pub fn new_opv(
        serotype: InfectionSerotype,
        mutations: u8,
        mean_reversion_days: f32,
        rng: &mut impl Rng,
    ) -> Self {
        if mutations >= 3 {
            // Already fully mutated — create as VDPV
            return Infection {
                shed_duration: 0.0,
                viral_shedding: 0.0,
                strain: InfectionStrain::VDPV,
                serotype,
                mutations,
                next_mutation_day: None,
            };
        }
        // Sample time to next mutation step
        let exp = Exp::new(1.0 / mean_reversion_days as f64).unwrap();
        let next_day = exp.sample(rng) as f32;
        Infection {
            shed_duration: 0.0,
            viral_shedding: 0.0,
            strain: InfectionStrain::OPV,
            serotype,
            mutations,
            next_mutation_day: Some(next_day),
        }
    }

    pub fn should_clear(&self, days_since_infection: f32) -> bool {
        days_since_infection > self.shed_duration
    }
}
