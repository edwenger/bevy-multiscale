use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sex {
    Male,
    Female,
}

impl Sex {
    pub fn symbol(&self) -> &'static str {
        match self {
            Sex::Male => "M",
            Sex::Female => "F",
        }
    }
}

/// Core individual component storing demographic info
#[derive(Component)]
pub struct Individual {
    pub age: f32,
    pub sex: Sex,
    pub birth_day: f32,
}

impl Individual {
    pub fn new(age: f32, sex: Sex, current_day: f32) -> Self {
        Self {
            age,
            sex,
            birth_day: current_day - age * 365.0,
        }
    }

    pub fn age_in_months(&self) -> f32 {
        self.age * 12.0
    }
}

/// Component linking individual to their household entity
#[derive(Component)]
pub struct HouseholdMember {
    pub household_id: Entity,
}

/// Component linking individual to their neighborhood entity
#[derive(Component)]
pub struct NeighborhoodMember {
    pub neighborhood_id: Entity,
}

/// Marker for visual representation of an individual
#[derive(Component)]
pub struct IndividualVisual;

/// Marker on outer sprite (border) — shows shedding status
#[derive(Component)]
pub struct IndividualBorder;

/// Marker on inner sprite (fill) — shows immunity level
#[derive(Component)]
pub struct IndividualFill;

/// Marker on age/sex text label
#[derive(Component)]
pub struct IndividualLabel;

/// Layout indices for live repositioning when bari_radius changes
#[derive(Component)]
pub struct BariLayoutIndex {
    pub hh_idx: usize,
    pub member_idx: usize,
    pub n_hh: usize,
}

/// Marker on bari-level outer sprite (border) — shows aggregate shedding
#[derive(Component)]
pub struct BariBorder;

/// Marker on bari-level inner sprite (fill) — shows aggregate immunity
#[derive(Component)]
pub struct BariFill;
