use bevy::prelude::*;
use rand::Rng;
use log::info;

use crate::disease::Immunity;
use crate::population::{
    Individual, Sex, HouseholdMember, NeighborhoodMember, Household, Neighborhood,
    IndividualVisual, HouseholdVisual, NeighborhoodVisual,
    PopulationConfig, NeedsPopulationSpawn,
    generate_household_members, calculate_initial_immunity,
};
use crate::ui::components::*;
use crate::ui::viz::immunity_to_fill_color;

/// Visual layout constants
const NEIGHBORHOOD_SPACING: f32 = 100.0;
const HOUSEHOLD_GAP: f32 = 15.0;
const INDIVIDUAL_SPACING: f32 = 14.0;
const BORDER_SIZE: f32 = 14.0;
const FILL_SIZE: f32 = 12.0;
const BAR_WIDTH: f32 = 4.0;
const GRID_LEFT_MARGIN: f32 = -580.0;
const GRID_TOP_MARGIN: f32 = 310.0;
const HOUSEHOLD_BOX_PADDING: f32 = 6.0;

/// Respawn population after reset
pub fn respawn_neighborhood_population(
    mut commands: Commands,
    config: Res<PopulationConfig>,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<Individual>>,
) {
    if existing.iter().count() == 0 {
        commands.remove_resource::<NeedsPopulationSpawn>();
        let mut rng = rand::thread_rng();
        spawn_population_internal(&mut commands, &config, &mut rng, &asset_server);
    }
}

pub fn spawn_population_internal(
    commands: &mut Commands,
    config: &PopulationConfig,
    rng: &mut impl Rng,
    asset_server: &AssetServer,
) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    info!("=== Spawning Neighborhood Population ===");
    info!("Config: {} neighborhoods x {} households, {:.1} lifetime births",
          config.num_neighborhoods, config.households_per_neighborhood, config.lifetime_births);

    let mut total_individuals = 0;

    for nbhd_idx in 0..config.num_neighborhoods {
        let nbhd_y = GRID_TOP_MARGIN - (nbhd_idx as f32) * NEIGHBORHOOD_SPACING;

        let neighborhood_entity = commands.spawn((
            Neighborhood::new(nbhd_idx),
            NeighborhoodVisual,
            SpatialBundle {
                transform: Transform::from_xyz(0.0, nbhd_y, 0.0),
                ..default()
            },
        )).id();

        let mut household_data: Vec<Vec<(f32, Sex)>> = Vec::new();
        for _ in 0..config.households_per_neighborhood {
            let members = generate_household_members(config, rng);
            household_data.push(members);
        }

        let mut hh_x = GRID_LEFT_MARGIN;

        for (_hh_idx, members) in household_data.iter().enumerate() {
            let hh_width = (members.len() as f32) * INDIVIDUAL_SPACING;

            let household_entity = commands.spawn((
                Household::new(neighborhood_entity),
                HouseholdVisual,
                SpatialBundle {
                    transform: Transform::from_xyz(hh_x, nbhd_y, 0.0),
                    ..default()
                },
            )).with_children(|parent| {
                let box_width = hh_width + HOUSEHOLD_BOX_PADDING * 2.0;
                let box_height = 90.0;
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0.2, 0.2, 0.25, 0.4),
                        custom_size: Some(Vec2::new(box_width, box_height)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        hh_width / 2.0 - INDIVIDUAL_SPACING / 2.0,
                        25.0,
                        -0.1
                    ),
                    ..default()
                });
            }).id();

            total_individuals += members.len();

            for (member_idx, (age, sex)) in members.iter().enumerate() {
                let ind_x = (member_idx as f32) * INDIVIDUAL_SPACING;

                let initial_immunity = calculate_initial_immunity(
                    *age, config.time_since_cessation, config.elimination_duration,
                    config.vaccine_coverage, rng,
                );

                let age_label = format!("{:.0}{}", age, sex.symbol());
                let fill_color = immunity_to_fill_color(initial_immunity);

                commands.spawn((
                    Individual::new(*age, *sex, 0.0),
                    Immunity::with_titer(initial_immunity),
                    HouseholdMember { household_id: household_entity },
                    NeighborhoodMember { neighborhood_id: neighborhood_entity },
                    IndividualVisual,
                    SpatialBundle {
                        transform: Transform::from_xyz(hh_x + ind_x, nbhd_y, 0.0),
                        ..default()
                    },
                )).with_children(|parent| {
                    // Border sprite (outer)
                    parent.spawn((
                        IndividualBorder,
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgba(0.3, 0.3, 0.3, 0.5),
                                custom_size: Some(Vec2::new(BORDER_SIZE, BORDER_SIZE)),
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
                                custom_size: Some(Vec2::new(FILL_SIZE, FILL_SIZE)),
                                ..default()
                            },
                            transform: Transform::from_xyz(0.0, 0.0, 0.05),
                            ..default()
                        },
                    ));

                    // Immunity bar (left side)
                    let immunity_height = (initial_immunity.log10() * 15.0).max(5.0);
                    parent.spawn((
                        ImmunityBar,
                        SpriteBundle {
                            sprite: Sprite {
                                color: fill_color,
                                custom_size: Some(Vec2::new(BAR_WIDTH, immunity_height)),
                                ..default()
                            },
                            transform: Transform::from_xyz(-3.0, immunity_height / 2.0 + 6.0, 0.1),
                            ..default()
                        },
                    ));

                    // Age/gender label
                    parent.spawn((
                        IndividualLabel,
                        Text2dBundle {
                            text: Text::from_section(
                                &age_label,
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 7.0,
                                    color: Color::rgba(0.15, 0.15, 0.15, 0.85),
                                },
                            ),
                            text_anchor: bevy::sprite::Anchor::Center,
                            transform: Transform::from_xyz(0.0, 0.0, 0.2),
                            ..default()
                        },
                    ));
                });
            }

            commands.entity(household_entity).insert(Household {
                neighborhood_id: neighborhood_entity,
                member_count: members.len(),
            });

            hh_x += hh_width + HOUSEHOLD_GAP;
        }

        commands.entity(neighborhood_entity).insert(Neighborhood {
            household_count: config.households_per_neighborhood,
            index: nbhd_idx,
        });
    }

    info!("=== Neighborhood Population Summary ===");
    info!("Total individuals: {}", total_individuals);
}
