use bevy::prelude::*;
use rand::Rng;

use crate::disease::Immunity;
use crate::population::{
    Individual, Sex, HouseholdMember, NeighborhoodMember, Household, Neighborhood,
    IndividualVisual, HouseholdVisual, NeighborhoodVisual,
    PopulationConfig, calculate_initial_immunity,
};
use crate::ui::components::*;
use crate::ui::viz::immunity_to_fill_color;

/// Spawn a single individual with border/fill/label/immunity-bar child sprites
pub fn spawn_single_individual_internal(
    commands: &mut Commands,
    config: &PopulationConfig,
    rng: &mut impl Rng,
    asset_server: &AssetServer,
    age: f32,
    sex: Sex,
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

    let initial_immunity = calculate_initial_immunity(
        age, config.time_since_cessation, config.elimination_duration,
        config.vaccine_coverage, rng,
    );

    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    let age_label = format!("{:.0}{}", age, sex.symbol());
    let fill_color = immunity_to_fill_color(initial_immunity);

    commands.spawn((
        Individual::new(age, sex, 0.0),
        Immunity::with_titer(initial_immunity),
        HouseholdMember { household_id: household_entity },
        NeighborhoodMember { neighborhood_id: neighborhood_entity },
        IndividualVisual,
        SpatialBundle {
            transform: Transform::from_xyz(-200.0, 100.0, 0.0),
            ..default()
        },
    )).with_children(|parent| {
        // Border sprite (outer)
        parent.spawn((
            IndividualBorder,
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.3, 0.3, 0.3, 0.5),
                    custom_size: Some(Vec2::new(60.0, 60.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            },
        ));

        // Fill sprite (inner)
        parent.spawn((
            IndividualFill,
            SpriteBundle {
                sprite: Sprite {
                    color: fill_color,
                    custom_size: Some(Vec2::new(50.0, 50.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 0.05),
                ..default()
            },
        ));

        // Age/sex label
        parent.spawn((
            IndividualLabel,
            Text2dBundle {
                text: Text::from_section(
                    &age_label,
                    TextStyle {
                        font: font.clone(),
                        font_size: 14.0,
                        color: Color::rgba(0.15, 0.15, 0.15, 0.85),
                    },
                ),
                text_anchor: bevy::sprite::Anchor::Center,
                transform: Transform::from_xyz(0.0, 0.0, 0.1),
                ..default()
            },
        ));

        // Immunity bar (left side)
        let immunity_height = (initial_immunity.log10() * 30.0).max(10.0);
        parent.spawn((
            ImmunityBar,
            SpriteBundle {
                sprite: Sprite {
                    color: fill_color,
                    custom_size: Some(Vec2::new(8.0, immunity_height)),
                    ..default()
                },
                transform: Transform::from_xyz(-20.0, immunity_height / 2.0 + 30.0, 0.1),
                ..default()
            },
        ));
    });
}
