use bevy::prelude::*;

/// Component marking an entity as a neighborhood
#[derive(Component)]
pub struct Neighborhood {
    pub household_count: usize,
    pub index: usize,
}

impl Neighborhood {
    pub fn new(index: usize) -> Self {
        Self {
            household_count: 0,
            index,
        }
    }
}

/// Marker component for neighborhood visual (the row)
#[derive(Component)]
pub struct NeighborhoodVisual;
