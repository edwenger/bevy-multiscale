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

/// Child component for immunity bar visualization
#[derive(Component)]
pub struct ImmunityBar;

/// Child component for shedding bar visualization
#[derive(Component)]
pub struct SheddingBar;
