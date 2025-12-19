use bevy::prelude::*;

/// Component marking an entity as a household
#[derive(Component)]
pub struct Household {
    pub neighborhood_id: Entity,
    pub member_count: usize,
}

impl Household {
    pub fn new(neighborhood_id: Entity) -> Self {
        Self {
            neighborhood_id,
            member_count: 0,
        }
    }
}

/// Marker component for household visual (the cell/box)
#[derive(Component)]
pub struct HouseholdVisual;
