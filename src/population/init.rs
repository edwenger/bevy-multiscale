use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rand::Rng;
use rand_distr::{Poisson, Exp, Distribution};
use log::info;

use crate::disease::Immunity;
use super::{Individual, Sex, HouseholdMember, NeighborhoodMember, Household, Neighborhood};
use super::{IndividualVisual, IndividualBorder, IndividualFill, IndividualLabel, BariLayoutIndex};
use super::{BariBorder, BariFill};
use super::{HouseholdVisual, NeighborhoodVisual, BariLayout};

/// Configuration for population generation
#[derive(Resource)]
pub struct PopulationConfig {
    pub num_neighborhoods: usize,
    pub households_per_neighborhood: usize,
    /// Average number of children born per mother over her lifetime
    pub lifetime_births: f32,
    /// Years since vaccination/circulation stopped (children younger than this are naive)
    pub time_since_cessation: f32,
    /// Duration of elimination period before cessation (transition cohort)
    pub elimination_duration: f32,
    /// Fraction of transition cohort that received vaccination (0-1)
    pub vaccine_coverage: f32,
}

impl Default for PopulationConfig {
    fn default() -> Self {
        Self {
            num_neighborhoods: 2000,
            households_per_neighborhood: 6,
            lifetime_births: 5.0,
            time_since_cessation: 2.0,
            elimination_duration: 10.0,
            vaccine_coverage: 0.5,
        }
    }
}

/// Event to trigger population reset
#[derive(Event)]
pub struct ResetPopulationEvent;

/// Compute (dx, dy) offset for the i-th member in a hex-spiral clump.
/// Index 0 is at center; indices 1..6 form ring 1; 7..18 ring 2; etc.
fn hex_spiral_offset(index: usize, spacing: f32) -> (f32, f32) {
    if index == 0 {
        return (0.0, 0.0);
    }

    // Determine which ring and position within that ring
    let mut ring = 1u32;
    let mut ring_start = 1usize;
    loop {
        let ring_size = 6 * ring as usize;
        if index < ring_start + ring_size {
            let pos_in_ring = index - ring_start;
            let r = ring as f32 * spacing;
            // 6 sides, each with `ring` positions
            let side = pos_in_ring / ring as usize;
            let offset_in_side = pos_in_ring % ring as usize;

            // Corner angles for hex ring (flat-top hex, starting at top-right)
            let corner_angle = std::f32::consts::TAU * side as f32 / 6.0;
            let next_corner = std::f32::consts::TAU * (side as f32 + 1.0) / 6.0;
            let t = offset_in_side as f32 / ring as f32;
            let angle = corner_angle + t * (next_corner - corner_angle);

            return (r * angle.cos(), r * angle.sin());
        }
        ring_start += ring_size;
        ring += 1;
        if ring > 20 {
            // Fallback for very large households
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

/// Spawn initial population (Startup system)
pub fn spawn_population(
    mut commands: Commands,
    config: Res<PopulationConfig>,
    bari_layout: Res<BariLayout>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    let mut rng = rand::thread_rng();
    spawn_population_internal(&mut commands, &config, &bari_layout, &mut rng, &asset_server, &mut images);
    auto_center_camera(&bari_layout, &mut cameras);
}

fn generate_household_members(config: &PopulationConfig, rng: &mut impl Rng) -> Vec<(f32, Sex)> {
    let mut members = Vec::new();

    // Mother age distribution: exponential favoring younger ages in growing populations
    const GENERATION_TIME: f32 = 25.0;
    const MIN_MOTHER_AGE: f32 = 20.0;
    const MAX_MOTHER_AGE: f32 = 45.0;

    let growth_rate = (config.lifetime_births / 2.0).max(1.0).ln() / GENERATION_TIME;

    let mother_age: f32 = if growth_rate > 0.001 {
        let exp_dist = Exp::new(growth_rate as f64).unwrap();
        loop {
            let offset: f64 = exp_dist.sample(rng);
            let age = MIN_MOTHER_AGE + offset as f32;
            if age <= MAX_MOTHER_AGE {
                break age;
            }
        }
    } else {
        rng.gen_range(MIN_MOTHER_AGE..MAX_MOTHER_AGE)
    };

    let father_age: f32 = mother_age + rng.gen_range(0.0..8.0);
    members.push((mother_age, Sex::Female));
    members.push((father_age, Sex::Male));

    // Elder (20% chance)
    if rng.gen_bool(0.2) {
        let elder_age: f32 = (father_age + rng.gen_range(20.0_f32..35.0)).min(80.0);
        let elder_sex = if rng.gen_bool(0.4) { Sex::Male } else { Sex::Female };
        members.push((elder_age, elder_sex));
    }

    // Children
    let poisson = Poisson::new(config.lifetime_births as f64).unwrap();
    let potential_children: usize = poisson.sample(rng) as usize;

    if potential_children > 0 {
        let mother_age_at_first: f32 = rng.gen_range(18.0..25.0);
        let mut mother_age_at_birth = mother_age_at_first;

        for _ in 0..potential_children {
            if mother_age_at_birth > 45.0 {
                break;
            }

            let child_age = mother_age - mother_age_at_birth;

            if child_age <= 20.0 && child_age >= 0.0 {
                let sex = if rng.gen_bool(0.5) { Sex::Male } else { Sex::Female };
                members.push((child_age, sex));
            }

            mother_age_at_birth += rng.gen_range(1.0..4.0);
        }
    }

    // Sort by age descending for display
    members.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    members
}

/// Calculate initial immunity based on age and 3-parameter model
fn calculate_initial_immunity(
    age: f32,
    time_since_cessation: f32,
    elimination_duration: f32,
    vaccine_coverage: f32,
    rng: &mut impl Rng,
) -> f32 {
    let age_at_cessation = age - time_since_cessation;

    if age_at_cessation < 0.0 {
        return 1.0;
    }

    if age_at_cessation >= elimination_duration {
        let log2_titer = rng.gen_range(5.0..10.0);
        return 2.0_f32.powf(log2_titer);
    }

    let mean_log2 = vaccine_coverage * 7.5;
    let variance = 2.5;
    let log2_titer = (mean_log2 + rng.gen_range(-variance..variance)).max(0.0);
    2.0_f32.powf(log2_titer)
}

/// Handle population reset - despawn all entities and flag for respawn
pub fn handle_reset_population(
    mut commands: Commands,
    mut events: EventReader<ResetPopulationEvent>,
    individuals: Query<Entity, With<Individual>>,
    households: Query<Entity, With<Household>>,
    neighborhoods: Query<Entity, With<Neighborhood>>,
    mut time_series: ResMut<crate::simulation::InfectionTimeSeries>,
) {
    for _ in events.read() {
        for entity in individuals.iter() {
            commands.entity(entity).despawn_recursive();
        }
        for entity in households.iter() {
            commands.entity(entity).despawn_recursive();
        }
        for entity in neighborhoods.iter() {
            commands.entity(entity).despawn_recursive();
        }

        *time_series = crate::simulation::InfectionTimeSeries::default();
        commands.insert_resource(super::NeedsPopulationSpawn);
    }
}

/// Respawn population after reset
pub fn respawn_population(
    mut commands: Commands,
    config: Res<PopulationConfig>,
    bari_layout: Res<BariLayout>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    existing: Query<Entity, With<Individual>>,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    if existing.iter().count() == 0 {
        commands.remove_resource::<super::NeedsPopulationSpawn>();
        let mut rng = rand::thread_rng();
        spawn_population_internal(&mut commands, &config, &bari_layout, &mut rng, &asset_server, &mut images);
        auto_center_camera(&bari_layout, &mut cameras);
    }
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

    // Compute centroid
    let n = bari_layout.positions.len() as f32;
    let cx: f32 = bari_layout.positions.iter().map(|p| p.x).sum::<f32>() / n;
    let cy: f32 = bari_layout.positions.iter().map(|p| p.y).sum::<f32>() / n;

    transform.translation.x = cx;
    transform.translation.y = cy;

    // Compute scale to fit all baris with margin
    let min_x = bari_layout.positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = bari_layout.positions.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = bari_layout.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = bari_layout.positions.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

    let span_x = (max_x - min_x) + 200.0; // margin for bari contents
    let span_y = (max_y - min_y) + 150.0;

    // Fit to ~1200x900 viewport
    let scale_x = span_x / 1200.0;
    let scale_y = span_y / 900.0;
    projection.scale = scale_x.max(scale_y).max(0.5).min(10.0);
}

/// Internal function to spawn population
pub fn spawn_population_internal(
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

    info!("=== Spawning Population ===");
    info!("Config: {} neighborhoods x {} households, {:.1} lifetime births",
          config.num_neighborhoods, config.households_per_neighborhood, config.lifetime_births);

    let num_baris = config.num_neighborhoods.min(bari_layout.positions.len());

    let mut total_individuals = 0;
    let mut all_ages: Vec<f32> = Vec::new();
    let mut all_immunities: Vec<(f32, f32)> = Vec::new();
    let mut hh_sizes: Vec<usize> = Vec::new();

    for nbhd_idx in 0..num_baris {
        let bari = &bari_layout.positions[nbhd_idx];

        // Spawn neighborhood entity at bari position
        let neighborhood_entity = commands.spawn((
            Neighborhood::new(nbhd_idx),
            NeighborhoodVisual,
            SpatialBundle {
                transform: Transform::from_xyz(bari.x, bari.y, 0.0),
                ..default()
            },
        )).id();

        // Pre-generate all households for this bari
        let mut household_data: Vec<Vec<(f32, Sex)>> = Vec::new();
        for _ in 0..config.households_per_neighborhood {
            let members = generate_household_members(config, rng);
            household_data.push(members);
        }

        // Spawn circular bari background: border (outer, 2x radius for wide shedding halo) + fill (inner)
        commands.entity(neighborhood_entity).with_children(|parent| {
            // Border sprite (outer) — shows aggregate shedding; radius-wide border visible at default zoom
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

            // Fill sprite (inner) — shows aggregate immunity
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

        // Place household clump centers using sunflower/Fibonacci spiral for even distribution
        let n_hh = household_data.len().max(1);
        let clump_spacing = 6.0; // spacing between individuals within a clump (>5px sprite size)
        // Use most of the bari circle for HH center placement; clumps may extend slightly past edge
        let pack_radius = bari_radius * 0.7;

        let golden_angle: f32 = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt()); // ~2.4 rad
        let hh_centers: Vec<(f32, f32)> = (0..n_hh).map(|i| {
            if n_hh == 1 {
                (0.0, 0.0)
            } else {
                // Sunflower spiral: r proportional to sqrt(i/(n-1)), outermost at pack_radius
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

            // Log household composition
            let member_str: Vec<String> = members.iter()
                .map(|(age, sex)| format!("{:.0}{}", age, sex.symbol()))
                .collect();
            info!("Bari {} HH {}: [{}]", nbhd_idx, hh_idx, member_str.join(", "));

            hh_sizes.push(members.len());
            total_individuals += members.len();
            for (age, _) in members {
                all_ages.push(*age);
                let imm = calculate_initial_immunity(
                    *age,
                    config.time_since_cessation,
                    config.elimination_duration,
                    config.vaccine_coverage,
                    rng,
                );
                all_immunities.push((*age, imm));
            }

            // Clump layout: arrange members in a tight hex-ish cluster around household center
            let (hh_cx, hh_cy) = hh_centers[hh_idx];

            for (member_idx, (age, sex)) in members.iter().enumerate() {
                // Hex-spiral placement: first member at center, then rings
                let (dx, dy) = hex_spiral_offset(member_idx, clump_spacing);
                let ind_x = bari.x + hh_cx + dx;
                let ind_y = bari.y + hh_cy + dy;

                let initial_immunity = calculate_initial_immunity(
                    *age,
                    config.time_since_cessation,
                    config.elimination_duration,
                    config.vaccine_coverage,
                    rng,
                );

                let age_label = format!("{:.0}{}", age, sex.symbol());

                // Spawn individual as parent SpatialBundle with absolute position
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
                    // Border sprite (outer) — shows shedding
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

                    // Fill sprite (inner) — shows immunity (brown→beige→green)
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

    // Summary statistics
    info!("=== Population Summary ===");
    info!("Total individuals: {}", total_individuals);

    if !all_ages.is_empty() {
        let mean_age: f32 = all_ages.iter().sum::<f32>() / all_ages.len() as f32;
        let children = all_ages.iter().filter(|&&a| a < 15.0).count();
        let adults = all_ages.iter().filter(|&&a| a >= 15.0 && a < 60.0).count();
        let elders = all_ages.iter().filter(|&&a| a >= 60.0).count();
        info!("Mean age: {:.1} years", mean_age);
        info!("Age distribution: {} children (<15), {} adults (15-59), {} elders (60+)",
              children, adults, elders);
    }

    if !hh_sizes.is_empty() {
        let mean_hh: f32 = hh_sizes.iter().sum::<usize>() as f32 / hh_sizes.len() as f32;
        let min_hh = hh_sizes.iter().min().unwrap_or(&0);
        let max_hh = hh_sizes.iter().max().unwrap_or(&0);
        info!("Household sizes: mean {:.1}, range {}-{}", mean_hh, min_hh, max_hh);
    }

    if !all_immunities.is_empty() {
        info!("=== Immunity by Age ===");
        let age_groups = [(0.0, 5.0, "<5"), (5.0, 15.0, "5-14"), (15.0, 30.0, "15-29"), (30.0, 100.0, "30+")];
        for (min_age, max_age, label) in age_groups {
            let group: Vec<f32> = all_immunities.iter()
                .filter(|(age, _)| *age >= min_age && *age < max_age)
                .map(|(_, imm)| *imm)
                .collect();
            if !group.is_empty() {
                let mean: f32 = group.iter().sum::<f32>() / group.len() as f32;
                let min = group.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = group.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                info!("  {}: n={}, mean={:.0}, range {:.0}-{:.0}", label, group.len(), mean, min, max);
            }
        }
    }
}

/// Compute household center offset from bari center for a given layout
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
/// when bari_radius changes.
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

    // Individual sprite sizes scale with radius (baseline: 5/4 at radius=20)
    let ind_border_size = r * 0.25;
    let ind_fill_size = r * 0.20;

    // Update bari circle sprites
    for mut sprite in bari_borders.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(4.0 * r));
    }
    for mut sprite in bari_fills.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(2.0 * r));
    }

    // Update individual sprite sizes
    for mut sprite in ind_borders.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(ind_border_size));
    }
    for mut sprite in ind_fills.iter_mut() {
        sprite.custom_size = Some(Vec2::splat(ind_fill_size));
    }

    // Reposition all individuals
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
