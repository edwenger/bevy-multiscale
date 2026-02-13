use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;

use crate::disease::{Immunity, Infection, InfectionStrain, InfectionSerotype, DiseaseParams};
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
    /// Override strain (None = use default_strain from TransmissionParams)
    pub strain: Option<InfectionStrain>,
    /// Override serotype (None = use default_serotype from TransmissionParams)
    pub serotype: Option<InfectionSerotype>,
    /// Fraction of eligible population to target (None = use count)
    pub coverage: Option<f32>,
}

impl Default for SeedInfectionEvent {
    fn default() -> Self {
        Self {
            count: 1,
            dose: 1e6,
            min_age: 0.0,
            max_age: 100.0,
            strain: None,
            serotype: None,
            coverage: None,
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
        let strain = event.strain.unwrap_or(tx_params.default_strain);
        let serotype = event.serotype.unwrap_or(tx_params.default_serotype);

        // Filter to eligible individuals (by age, not already infected)
        let eligible: Vec<_> = susceptibles.iter()
            .filter(|(_, ind, _)| ind.age >= event.min_age && ind.age <= event.max_age)
            .map(|(e, _, _)| e)
            .collect();

        if eligible.is_empty() {
            continue;
        }

        // Compute number of targets from coverage or count
        let n = if let Some(cov) = event.coverage {
            (cov * eligible.len() as f32).round() as usize
        } else {
            event.count
        };
        let n = n.min(eligible.len());
        let targets: Vec<_> = eligible.choose_multiple(&mut rng, n).cloned().collect();

        for target_entity in targets {
            if let Ok((_, _, mut immunity)) = susceptibles.get_mut(target_entity) {
                // Calculate infection probability
                let p_inf = immunity.calculate_infection_probability(
                    event.dose,
                    strain,
                    serotype,
                    &disease_params,
                );

                // Attempt infection
                if rng.gen::<f32>() < p_inf {
                    let mut infection = if strain == InfectionStrain::OPV {
                        Infection::new_opv(
                            serotype, 0,
                            tx_params.mean_reversion_days,
                            &mut rng,
                        )
                    } else {
                        Infection::new(strain, serotype)
                    };
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
