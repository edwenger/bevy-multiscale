use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::disease::{Immunity, Infection, DiseaseParams};
use crate::population::Individual;
use super::time::SimulationTime;
use super::transmission::TransmissionParams;

/// Event to trigger infection seeding
#[derive(Event)]
pub struct SeedInfectionEvent {
    pub count: usize,
    pub dose: f32,
    pub min_age: f32,
    pub max_age: f32,
}

impl Default for SeedInfectionEvent {
    fn default() -> Self {
        Self {
            count: 1,
            dose: 1e6,
            min_age: 0.0,
            max_age: 100.0,
        }
    }
}

/// Handle infection seeding events
pub fn handle_seed_infection(
    mut commands: Commands,
    mut events: EventReader<SeedInfectionEvent>,
    sim_time: Res<SimulationTime>,
    tx_params: Res<TransmissionParams>,
    disease_params: Res<DiseaseParams>,
    mut susceptibles: Query<(Entity, &Individual, &mut Immunity), Without<Infection>>,
) {
    let mut rng = rand::thread_rng();

    for event in events.read() {
        // Filter to eligible individuals (by age, not already infected)
        let eligible: Vec<_> = susceptibles.iter()
            .filter(|(_, ind, _)| ind.age >= event.min_age && ind.age <= event.max_age)
            .map(|(e, _, _)| e)
            .collect();

        if eligible.is_empty() {
            continue;
        }

        // Sample targets
        let n = event.count.min(eligible.len());
        let targets: Vec<_> = eligible.choose_multiple(&mut rng, n).cloned().collect();

        for target_entity in targets {
            if let Ok((_, _, mut immunity)) = susceptibles.get_mut(target_entity) {
                // Calculate infection probability
                let p_inf = immunity.calculate_infection_probability(
                    event.dose,
                    tx_params.default_strain,
                    tx_params.default_serotype,
                    &disease_params,
                );

                // Attempt infection
                if rng.gen::<f32>() < p_inf {
                    let mut infection = Infection::new(
                        tx_params.default_strain,
                        tx_params.default_serotype
                    );
                    immunity.set_infection_prognoses(
                        &mut infection,
                        sim_time.day as f32,
                        &disease_params
                    );
                    commands.entity(target_entity).insert(infection);
                }
            }
        }
    }
}
