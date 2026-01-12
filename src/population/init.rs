use bevy::prelude::*;
use rand::Rng;
use rand_distr::{Poisson, Exp, Distribution};
use log::info;

use crate::disease::Immunity;
use super::{Individual, Sex, HouseholdMember, NeighborhoodMember, Household, Neighborhood};
use super::{IndividualVisual, ImmunityBar, HouseholdVisual, NeighborhoodVisual};

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
            num_neighborhoods: 5,
            households_per_neighborhood: 7,
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

/// Visual layout constants
const NEIGHBORHOOD_SPACING: f32 = 100.0;  // Reduced from 120 to fit more rows
const HOUSEHOLD_GAP: f32 = 15.0;          // Gap between households (reduced from 20)
const INDIVIDUAL_SPACING: f32 = 14.0;     // Tighter spacing within household
const INDIVIDUAL_WIDTH: f32 = 12.0;
const INDIVIDUAL_HEIGHT: f32 = 12.0;
const BAR_WIDTH: f32 = 4.0;
const GRID_LEFT_MARGIN: f32 = -580.0;     // Moved left for more horizontal space
const GRID_TOP_MARGIN: f32 = 310.0;       // Moved up, just below title
const HOUSEHOLD_BOX_PADDING: f32 = 6.0;   // Padding inside household box

/// Spawn initial population (Startup system)
pub fn spawn_population(
    mut commands: Commands,
    config: Res<PopulationConfig>,
    asset_server: Res<AssetServer>,
) {
    let mut rng = rand::thread_rng();
    spawn_population_internal(&mut commands, &config, &mut rng, &asset_server);
}

fn generate_household_members(config: &PopulationConfig, rng: &mut impl Rng) -> Vec<(f32, Sex)> {
    let mut members = Vec::new();

    // Mother age distribution: exponential favoring younger ages in growing populations
    // Growth rate r = ln(fertility/2) / generation_time, where generation_time ≈ 25 years
    // For fertility=2: flat; fertility=4: 2x more 20yo than 45yo
    const GENERATION_TIME: f32 = 25.0;
    const MIN_MOTHER_AGE: f32 = 20.0;
    const MAX_MOTHER_AGE: f32 = 45.0;

    let growth_rate = (config.lifetime_births / 2.0).max(1.0).ln() / GENERATION_TIME;

    let mother_age: f32 = if growth_rate > 0.001 {
        // Sample from truncated exponential: younger mothers more likely
        let exp_dist = Exp::new(growth_rate as f64).unwrap();
        loop {
            let offset: f64 = exp_dist.sample(rng);
            let age = MIN_MOTHER_AGE + offset as f32;
            if age <= MAX_MOTHER_AGE {
                break age;
            }
        }
    } else {
        // Near-replacement fertility: uniform distribution
        rng.gen_range(MIN_MOTHER_AGE..MAX_MOTHER_AGE)
    };

    let father_age: f32 = mother_age + rng.gen_range(0.0..8.0);
    members.push((mother_age, Sex::Female));
    members.push((father_age, Sex::Male));

    // Elder (20% chance): father's parent, 20-35 years older
    if rng.gen_bool(0.2) {
        let elder_age: f32 = (father_age + rng.gen_range(20.0_f32..35.0)).min(80.0);
        let elder_sex = if rng.gen_bool(0.4) { Sex::Male } else { Sex::Female };
        members.push((elder_age, elder_sex));
    }

    // Children: emergent from maternal birth history
    let poisson = Poisson::new(config.lifetime_births as f64).unwrap();
    let potential_children: usize = poisson.sample(rng) as usize;

    if potential_children > 0 {
        // First child born when mother was 18-25
        let mother_age_at_first: f32 = rng.gen_range(18.0..25.0);
        let mut mother_age_at_birth = mother_age_at_first;

        for _ in 0..potential_children {
            // Check if mother was too old for this birth
            if mother_age_at_birth > 45.0 {
                break;
            }

            // Child's current age
            let child_age = mother_age - mother_age_at_birth;

            // Discard if child would be > 20 (left household)
            if child_age <= 20.0 && child_age >= 0.0 {
                let sex = if rng.gen_bool(0.5) { Sex::Male } else { Sex::Female };
                members.push((child_age, sex));
            }

            // Next birth with 1-4 year spacing
            mother_age_at_birth += rng.gen_range(1.0..4.0);
        }
    }

    // Sort by age descending for display
    members.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    members
}

/// Calculate initial immunity based on age and 3-parameter model
///
/// Parameters:
/// - time_since_cessation: Years since vaccination stopped (children younger = naive)
/// - elimination_duration: Years of transition period before cessation
/// - vaccine_coverage: Fraction that modulates immunity in transition cohort (0-1)
///
/// Age cohorts:
/// - age < time_since_cessation: Born after cessation → naive (titer = 1)
/// - age > time_since_cessation + elimination_duration: Endemic → log2 titer 5-10
/// - age in between: Transition → log2 titer scaled by coverage with wider variance
fn calculate_initial_immunity(
    age: f32,
    time_since_cessation: f32,
    elimination_duration: f32,
    vaccine_coverage: f32,
    rng: &mut impl Rng,
) -> f32 {
    let age_at_cessation = age - time_since_cessation;

    if age_at_cessation < 0.0 {
        // Born after cessation - naive
        return 1.0;
    }

    if age_at_cessation >= elimination_duration {
        // Experienced full endemic regime - log2 titer uniform 5-10
        let log2_titer = rng.gen_range(5.0..10.0);
        return 2.0_f32.powf(log2_titer);
    }

    // Transition cohort - coverage scales mean titer, wider variance
    // Mean log2 titer: 0% coverage → ~0, 100% coverage → ~7.5 (endemic mean)
    // Variance ±2.5 to give spread from 0 to ~10 at full coverage
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
) {
    for _ in events.read() {
        // Despawn all existing entities
        for entity in individuals.iter() {
            commands.entity(entity).despawn_recursive();
        }
        for entity in households.iter() {
            commands.entity(entity).despawn_recursive();
        }
        for entity in neighborhoods.iter() {
            commands.entity(entity).despawn_recursive();
        }

        // Flag that we need to respawn population next frame
        commands.insert_resource(super::NeedsPopulationSpawn);
    }
}

/// Respawn population after reset (runs when NeedsPopulationSpawn exists)
pub fn respawn_population(
    mut commands: Commands,
    config: Res<PopulationConfig>,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<Individual>>,
) {
    // Only spawn if there are no existing individuals (despawn completed)
    if existing.iter().count() == 0 {
        // Remove the flag
        commands.remove_resource::<super::NeedsPopulationSpawn>();

        // Spawn new population
        let mut rng = rand::thread_rng();
        spawn_population_internal(&mut commands, &config, &mut rng, &asset_server);
    }
}

/// Internal function to spawn population (shared between startup and respawn)
fn spawn_population_internal(
    commands: &mut Commands,
    config: &PopulationConfig,
    rng: &mut impl Rng,
    asset_server: &AssetServer,
) {
    let font = asset_server.load("fonts/FiraMono-Medium.ttf");
    info!("=== Spawning Population ===");
    info!("Config: {} neighborhoods x {} households, {:.1} lifetime births",
          config.num_neighborhoods, config.households_per_neighborhood, config.lifetime_births);

    let mut total_individuals = 0;
    let mut all_ages: Vec<f32> = Vec::new();
    let mut all_immunities: Vec<(f32, f32)> = Vec::new(); // (age, immunity)
    let mut hh_sizes: Vec<usize> = Vec::new();

    for nbhd_idx in 0..config.num_neighborhoods {
        let nbhd_y = GRID_TOP_MARGIN - (nbhd_idx as f32) * NEIGHBORHOOD_SPACING;

        // Spawn neighborhood entity
        let neighborhood_entity = commands.spawn((
            Neighborhood::new(nbhd_idx),
            NeighborhoodVisual,
            SpatialBundle {
                transform: Transform::from_xyz(0.0, nbhd_y, 0.0),
                ..default()
            },
        )).id();

        // Pre-generate all households for this neighborhood to calculate positions
        let mut household_data: Vec<Vec<(f32, Sex)>> = Vec::new();
        for _ in 0..config.households_per_neighborhood {
            let members = generate_household_members(config, rng);
            household_data.push(members);
        }

        // Calculate cumulative X positions based on actual household widths
        let mut hh_x = GRID_LEFT_MARGIN;

        for (hh_idx, members) in household_data.iter().enumerate() {
            let hh_width = (members.len() as f32) * INDIVIDUAL_SPACING;

            // Spawn household entity with box visual
            let household_entity = commands.spawn((
                Household::new(neighborhood_entity),
                HouseholdVisual,
                SpatialBundle {
                    transform: Transform::from_xyz(hh_x, nbhd_y, 0.0),
                    ..default()
                },
            )).with_children(|parent| {
                // Subtle box behind the household
                let box_width = hh_width + HOUSEHOLD_BOX_PADDING * 2.0;
                let box_height = 90.0;  // Enough for bars
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0.2, 0.2, 0.25, 0.4),
                        custom_size: Some(Vec2::new(box_width, box_height)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        hh_width / 2.0 - INDIVIDUAL_SPACING / 2.0,
                        25.0,  // Center vertically around bars
                        -0.1   // Behind individuals
                    ),
                    ..default()
                });
            }).id();

            // Log household composition and collect stats
            let member_str: Vec<String> = members.iter()
                .map(|(age, sex)| format!("{:.0}{}", age, sex.symbol()))
                .collect();
            info!("Nbhd {} HH {}: [{}]", nbhd_idx, hh_idx, member_str.join(", "));

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

            for (member_idx, (age, sex)) in members.iter().enumerate() {
                let ind_x = (member_idx as f32) * INDIVIDUAL_SPACING;

                // Calculate initial immunity
                let initial_immunity = calculate_initial_immunity(
                    *age,
                    config.time_since_cessation,
                    config.elimination_duration,
                    config.vaccine_coverage,
                    rng,
                );

                // Spawn individual
                let age_label = format!("{:.0}{}", age, sex.symbol());
                commands.spawn((
                    Individual::new(*age, *sex, 0.0),
                    Immunity::with_titer(initial_immunity),
                    HouseholdMember { household_id: household_entity },
                    NeighborhoodMember { neighborhood_id: neighborhood_entity },
                    IndividualVisual,
                    SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.4, 0.4, 0.4),
                            custom_size: Some(Vec2::new(INDIVIDUAL_WIDTH, INDIVIDUAL_HEIGHT)),
                            ..default()
                        },
                        transform: Transform::from_xyz(hh_x + ind_x, nbhd_y, 0.0),
                        ..default()
                    },
                )).with_children(|parent| {
                    // Immunity bar
                    let immunity_height = (initial_immunity.log10() * 15.0).max(5.0);
                    parent.spawn((
                        ImmunityBar,
                        SpriteBundle {
                            sprite: Sprite {
                                color: Color::rgba(0.32, 0.71, 0.89, 0.7),
                                custom_size: Some(Vec2::new(BAR_WIDTH, immunity_height)),
                                ..default()
                            },
                            transform: Transform::from_xyz(-3.0, immunity_height / 2.0 + 5.0, 0.1),
                            ..default()
                        },
                    ));

                    // Age/gender label inside sprite
                    parent.spawn(Text2dBundle {
                        text: Text::from_section(
                            &age_label,
                            TextStyle {
                                font: font.clone(),
                                font_size: 7.0,
                                color: Color::rgba(0.9, 0.9, 0.9, 0.7),
                            },
                        ),
                        text_anchor: bevy::sprite::Anchor::Center,
                        transform: Transform::from_xyz(0.0, 0.0, 0.2),
                        ..default()
                    });
                });
            }

            // Update household member count
            commands.entity(household_entity).insert(Household {
                neighborhood_id: neighborhood_entity,
                member_count: members.len(),
            });

            // Advance X position for next household
            hh_x += hh_width + HOUSEHOLD_GAP;
        }

        // Update neighborhood household count
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

    // Immunity summary by age group
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
