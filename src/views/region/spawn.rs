use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::Rng;
use log::info;

use crate::disease::Immunity;
use crate::population::{
    Individual, Sex, HouseholdMember, NeighborhoodMember, Household, Neighborhood,
    IndividualVisual, HouseholdVisual, NeighborhoodVisual,
    PopulationConfig, NeedsPopulationSpawn,
    generate_household_members, calculate_initial_immunity,
};
use super::bari::BariLayout;
use super::components::*;

/// Compute (dx, dy) offset for the i-th member in a hex-spiral clump.
fn hex_spiral_offset(index: usize, spacing: f32) -> (f32, f32) {
    if index == 0 {
        return (0.0, 0.0);
    }

    let mut ring = 1u32;
    let mut ring_start = 1usize;
    loop {
        let ring_size = 6 * ring as usize;
        if index < ring_start + ring_size {
            let pos_in_ring = index - ring_start;
            let r = ring as f32 * spacing;
            let side = pos_in_ring / ring as usize;
            let offset_in_side = pos_in_ring % ring as usize;

            let corner_angle = std::f32::consts::TAU * side as f32 / 6.0;
            let next_corner = std::f32::consts::TAU * (side as f32 + 1.0) / 6.0;
            let t = offset_in_side as f32 / ring as f32;
            let angle = corner_angle + t * (next_corner - corner_angle);

            return (r * angle.cos(), r * angle.sin());
        }
        ring_start += ring_size;
        ring += 1;
        if ring > 20 {
            let angle = index as f32 * 2.4;
            let r = spacing * (index as f32).sqrt();
            return (r * angle.cos(), r * angle.sin());
        }
    }
}

/// Create a white filled-circle image for use as a sprite texture
fn create_circle_image(size: u32) -> Image {
    let mut data = vec![0u8; (size * size * 4) as usize];
    let center = size as f32 / 2.0;
    let radius = center - 1.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = if dist <= radius { 255 } else { 0 };
            let idx = ((y * size + x) * 4) as usize;
            data[idx..idx + 4].copy_from_slice(&[255, 255, 255, alpha]);
        }
    }
    Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    )
}

/// Auto-center camera to fit all bari positions
fn auto_center_camera(
    bari_layout: &BariLayout,
    cameras: &mut Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    if bari_layout.positions.is_empty() {
        return;
    }

    let Ok((mut transform, mut projection)) = cameras.get_single_mut() else { return };

    let n = bari_layout.positions.len() as f32;
    let cx: f32 = bari_layout.positions.iter().map(|p| p.x).sum::<f32>() / n;
    let cy: f32 = bari_layout.positions.iter().map(|p| p.y).sum::<f32>() / n;

    transform.translation.x = cx;
    transform.translation.y = cy;

    let min_x = bari_layout.positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = bari_layout.positions.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = bari_layout.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = bari_layout.positions.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

    let span_x = (max_x - min_x) + 200.0;
    let span_y = (max_y - min_y) + 150.0;

    let scale_x = span_x / 1200.0;
    let scale_y = span_y / 900.0;
    projection.scale = scale_x.max(scale_y).max(0.5).min(10.0);
}

/// Respawn population after reset
pub fn respawn_region_population(
    mut commands: Commands,
    config: Res<PopulationConfig>,
    bari_layout: Res<BariLayout>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<Individual>>,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    if existing.iter().count() == 0 {
        commands.remove_resource::<NeedsPopulationSpawn>();
        let mut rng = rand::thread_rng();
        spawn_region_population_internal(&mut commands, &config, &bari_layout, &mut rng, &asset_server, &mut images);
        auto_center_camera(&bari_layout, &mut cameras);
    }
}

/// Internal function to spawn population
pub fn spawn_region_population_internal(
    commands: &mut Commands,
    config: &PopulationConfig,
    bari_layout: &BariLayout,
    rng: &mut impl Rng,
    asset_server: &AssetServer,
    images: &mut Assets<Image>,
) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    let circle_texture: Handle<Image> = images.add(create_circle_image(64));
    let bari_radius = bari_layout.bari_radius;

    info!("=== Spawning Region Population ===");
    info!("Config: {} neighborhoods x {} households, {:.1} lifetime births",
          config.num_neighborhoods, config.households_per_neighborhood, config.lifetime_births);

    let num_baris = config.num_neighborhoods.min(bari_layout.positions.len());

    let mut total_individuals = 0;
    let mut hh_sizes: Vec<usize> = Vec::new();

    for nbhd_idx in 0..num_baris {
        let bari = &bari_layout.positions[nbhd_idx];

        let neighborhood_entity = commands.spawn((
            Neighborhood::new(nbhd_idx),
            NeighborhoodVisual,
            SpatialBundle {
                transform: Transform::from_xyz(bari.x, bari.y, 0.0),
                ..default()
            },
        )).id();

        let mut household_data: Vec<Vec<(f32, Sex)>> = Vec::new();
        for _ in 0..config.households_per_neighborhood {
            let members = generate_household_members(config, rng);
            household_data.push(members);
        }

        // Bari background sprites
        commands.entity(neighborhood_entity).with_children(|parent| {
            parent.spawn((
                BariBorder,
                SpriteBundle {
                    texture: circle_texture.clone(),
                    sprite: Sprite {
                        color: Color::rgba(0.25, 0.25, 0.30, 0.15),
                        custom_size: Some(Vec2::splat(4.0 * bari_radius)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, -0.3),
                    ..default()
                },
            ));

            parent.spawn((
                BariFill,
                SpriteBundle {
                    texture: circle_texture.clone(),
                    sprite: Sprite {
                        color: Color::rgba(0.25, 0.25, 0.30, 0.3),
                        custom_size: Some(Vec2::splat(2.0 * bari_radius)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, -0.2),
                    ..default()
                },
            ));
        });

        let n_hh = household_data.len().max(1);
        let clump_spacing = 6.0;
        let pack_radius = bari_radius * 0.7;

        let golden_angle: f32 = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
        let hh_centers: Vec<(f32, f32)> = (0..n_hh).map(|i| {
            if n_hh == 1 {
                (0.0, 0.0)
            } else {
                let r = pack_radius * ((i as f32 + 0.5) / n_hh as f32).sqrt();
                let theta = i as f32 * golden_angle;
                (r * theta.cos(), r * theta.sin())
            }
        }).collect();

        for (hh_idx, members) in household_data.iter().enumerate() {
            let household_entity = commands.spawn((
                Household::new(neighborhood_entity),
                HouseholdVisual,
                SpatialBundle::default(),
            )).id();

            hh_sizes.push(members.len());
            total_individuals += members.len();

            let (hh_cx, hh_cy) = hh_centers[hh_idx];

            for (member_idx, (age, sex)) in members.iter().enumerate() {
                let (dx, dy) = hex_spiral_offset(member_idx, clump_spacing);
                let ind_x = bari.x + hh_cx + dx;
                let ind_y = bari.y + hh_cy + dy;

                let initial_immunity = calculate_initial_immunity(
                    *age, config.time_since_cessation, config.elimination_duration,
                    config.vaccine_coverage, rng,
                );

                let age_label = format!("{:.0}{}", age, sex.symbol());

                commands.spawn((
                    Individual::new(*age, *sex, 0.0),
                    Immunity::with_titer(initial_immunity),
                    HouseholdMember { household_id: household_entity },
                    NeighborhoodMember { neighborhood_id: neighborhood_entity },
                    BariLayoutIndex { hh_idx, member_idx, n_hh },
                    IndividualVisual,
                    SpatialBundle {
                        transform: Transform::from_xyz(ind_x, ind_y, 0.0),
                        ..default()
                    },
                )).with_children(|parent| {
                    // Border sprite (outer)
                    parent.spawn((
                        IndividualBorder,
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgba(0.3, 0.3, 0.3, 0.5),
                                custom_size: Some(Vec2::new(5.0, 5.0)),
                                ..default()
                            },
                            transform: Transform::from_xyz(0.0, 0.0, 0.0),
                            ..default()
                        },
                    ));

                    // Fill sprite (inner)
                    let fill_t = (initial_immunity.log2() / 10.0).clamp(0.0, 1.0);
                    let fill_color = if fill_t < 0.5 {
                        let s = fill_t / 0.5;
                        Color::rgb(0.55 + s * 0.41, 0.35 + s * 0.55, 0.15 + s * 0.60)
                    } else {
                        let s = (fill_t - 0.5) / 0.5;
                        Color::rgb(0.96 + s * (0.2 - 0.96), 0.90 + s * (0.75 - 0.90), 0.75 + s * (0.3 - 0.75))
                    };
                    parent.spawn((
                        IndividualFill,
                        SpriteBundle {
                            sprite: Sprite {
                                color: fill_color,
                                custom_size: Some(Vec2::new(4.0, 4.0)),
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
                                    font_size: 5.0,
                                    color: Color::rgba(0.9, 0.9, 0.9, 0.7),
                                },
                            ),
                            text_anchor: bevy::sprite::Anchor::Center,
                            transform: Transform::from_xyz(0.0, 0.0, 0.1),
                            ..default()
                        },
                    ));
                });
            }

            commands.entity(household_entity).insert(Household {
                neighborhood_id: neighborhood_entity,
                member_count: members.len(),
            });
        }

        commands.entity(neighborhood_entity).insert(Neighborhood {
            household_count: config.households_per_neighborhood,
            index: nbhd_idx,
        });
    }

    info!("=== Region Population Summary ===");
    info!("Total individuals: {}", total_individuals);

    if !hh_sizes.is_empty() {
        let mean_hh: f32 = hh_sizes.iter().sum::<usize>() as f32 / hh_sizes.len() as f32;
        info!("Household sizes: mean {:.1}", mean_hh);
    }
}

/// Compute household center offset from bari center
fn hh_center_offset(hh_idx: usize, n_hh: usize, pack_radius: f32) -> (f32, f32) {
    if n_hh <= 1 {
        return (0.0, 0.0);
    }
    let golden_angle: f32 = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt());
    let r = pack_radius * ((hh_idx as f32 + 0.5) / n_hh as f32).sqrt();
    let theta = hh_idx as f32 * golden_angle;
    (r * theta.cos(), r * theta.sin())
}

/// Live-update bari sprite sizes, individual positions, and individual sprite sizes
pub fn update_bari_display(
    bari_layout: Res<BariLayout>,
    neighborhoods: Query<&Transform, With<Neighborhood>>,
    mut bari_borders: Query<
        &mut Sprite,
        (With<BariBorder>, Without<BariFill>, Without<IndividualBorder>, Without<IndividualFill>),
    >,
    mut bari_fills: Query<
        &mut Sprite,
        (With<BariFill>, Without<BariBorder>, Without<IndividualBorder>, Without<IndividualFill>),
    >,
    mut individuals: Query<
        (&mut Transform, &BariLayoutIndex, &NeighborhoodMember),
        (With<Individual>, Without<Neighborhood>),
    >,
    mut ind_borders: Query<
        &mut Sprite,
        (With<IndividualBorder>, Without<IndividualFill>, Without<BariBorder>, Without<BariFill>),
    >,
    mut ind_fills: Query<
        &mut Sprite,
        (With<IndividualFill>, Without<IndividualBorder>, Without<BariBorder>, Without<BariFill>),
    >,
) {
    if !bari_layout.is_changed() {
        return;
    }

    let r = bari_layout.bari_radius;
    let pack_radius = r * 0.7;
    let clump_spacing = r * 0.3;

    let ind_border_size = r * 0.25;
    let ind_fill_size = r * 0.20;

    for mut sprite in bari_borders.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(4.0 * r));
    }
    for mut sprite in bari_fills.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(2.0 * r));
    }

    for mut sprite in ind_borders.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(ind_border_size));
    }
    for mut sprite in ind_fills.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(ind_fill_size));
    }

    for (mut transform, layout, nbhd_member) in individuals.iter_mut() {
        let Ok(nbhd_tf) = neighborhoods.get(nbhd_member.neighborhood_id) else { continue };
        let bari_x = nbhd_tf.translation.x;
        let bari_y = nbhd_tf.translation.y;

        let (hh_cx, hh_cy) = hh_center_offset(layout.hh_idx, layout.n_hh, pack_radius);
        let (dx, dy) = hex_spiral_offset(layout.member_idx, clump_spacing);

        transform.translation.x = bari_x + hh_cx + dx;
        transform.translation.y = bari_y + hh_cy + dy;
    }
}
