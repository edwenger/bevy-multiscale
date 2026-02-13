use bevy::prelude::*;
use rand::Rng;
use rand_distr::{Poisson, Exp, Distribution};

use super::{Individual, Household, Neighborhood};

/// Configuration for population generation
#[derive(Resource)]
pub struct PopulationConfig {
    pub num_neighborhoods: usize,
    pub households_per_neighborhood: usize,
    pub lifetime_births: f32,
    pub time_since_cessation: f32,
    pub elimination_duration: f32,
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

/// Marker resource indicating population needs to be spawned
#[derive(Resource, Default)]
pub struct NeedsPopulationSpawn;

/// Generate household members with age-structured demographics
pub fn generate_household_members(config: &PopulationConfig, rng: &mut impl Rng) -> Vec<(f32, super::Sex)> {
    let mut members = Vec::new();

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
    members.push((mother_age, super::Sex::Female));
    members.push((father_age, super::Sex::Male));

    if rng.gen_bool(0.2) {
        let elder_age: f32 = (father_age + rng.gen_range(20.0_f32..35.0)).min(80.0);
        let elder_sex = if rng.gen_bool(0.4) { super::Sex::Male } else { super::Sex::Female };
        members.push((elder_age, elder_sex));
    }

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
                let sex = if rng.gen_bool(0.5) { super::Sex::Male } else { super::Sex::Female };
                members.push((child_age, sex));
            }

            mother_age_at_birth += rng.gen_range(1.0..4.0);
        }
    }

    members.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    members
}

/// Calculate initial immunity based on age and 3-parameter model
pub fn calculate_initial_immunity(
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
        commands.insert_resource(NeedsPopulationSpawn);
    }
}
