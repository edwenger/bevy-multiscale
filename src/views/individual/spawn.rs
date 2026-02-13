use bevy::prelude::*;
use rand::Rng;

use crate::disease::Immunity;
use crate::population::{
    Individual, Sex, HouseholdMember, NeighborhoodMember, Household, Neighborhood,
    IndividualVisual, HouseholdVisual, NeighborhoodVisual,
    PopulationConfig, calculate_initial_immunity,
};

/// Spawn a single individual with a large centered sprite
pub fn spawn_single_individual_internal(
    commands: &mut Commands,
    config: &PopulationConfig,
    rng: &mut impl Rng,
) {
    let neighborhood_entity = commands.spawn((
        Neighborhood::new(0),
        NeighborhoodVisual,
        SpatialBundle::default(),
    )).id();

    let household_entity = commands.spawn((
        Household { neighborhood_id: neighborhood_entity, member_count: 1 },
        HouseholdVisual,
        SpatialBundle::default(),
    )).id();

    commands.entity(neighborhood_entity).insert(Neighborhood {
        household_count: 1,
        index: 0,
    });

    let age = 1.0_f32;
    let sex = Sex::Male;
    let initial_immunity = calculate_initial_immunity(
        age, config.time_since_cessation, config.elimination_duration,
        config.vaccine_coverage, rng,
    );

    commands.spawn((
        Individual::new(age, sex, 0.0),
        Immunity::with_titer(initial_immunity),
        HouseholdMember { household_id: household_entity },
        NeighborhoodMember { neighborhood_id: neighborhood_entity },
        IndividualVisual,
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.4, 0.4, 0.4),
                custom_size: Some(Vec2::new(80.0, 80.0)),
                ..default()
            },
            transform: Transform::from_xyz(-200.0, 0.0, 0.0),
            ..default()
        },
    ));
}
