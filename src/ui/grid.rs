// Grid layout is handled in population/init.rs during spawn
// This file reserved for future grid-related utilities

use bevy::prelude::*;

use crate::population::{Neighborhood, Household, HouseholdVisual, NeighborhoodVisual};

/// Draw neighborhood row backgrounds (optional visual enhancement)
pub fn draw_neighborhood_backgrounds(
    mut _commands: Commands,
    _neighborhoods: Query<(Entity, &Neighborhood, &Transform), With<NeighborhoodVisual>>,
) {
    // Could add row background sprites here
}

/// Draw household cell backgrounds (optional visual enhancement)
pub fn draw_household_backgrounds(
    mut _commands: Commands,
    _households: Query<(Entity, &Household, &Transform), With<HouseholdVisual>>,
) {
    // Could add cell background sprites here
}
