use bevy::prelude::*;
use rand_distr::{Exp, Distribution};

use crate::disease::{Immunity, Infection, InfectionStrain, DiseaseParams};
use crate::population::Individual;
use super::time::SimulationTime;
use super::transmission::TransmissionParams;

/// System to step disease state each day
pub fn step_disease_state(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    params: Res<DiseaseParams>,
    tx_params: Res<TransmissionParams>,
    mut query: Query<(Entity, &Individual, &mut Immunity, Option<&mut Infection>)>,
    mut timings: ResMut<super::SystemTimings>,
) {
    // Only run on timer tick
    if !sim_time.timer.just_finished() {
        return;
    }

    let t0 = bevy::utils::Instant::now();
    let mut rng = rand::thread_rng();

    for (entity, individual, mut immunity, infection) in query.iter_mut() {
        if let Some(ti_infected) = immunity.ti_infected {
            let t_since_last_exposure = sim_time.day as f32 - ti_infected;

            // Update immunity waning
            immunity.calculate_waning(t_since_last_exposure, &params.immunity_waning);

            // Update infection state
            if let Some(mut inf) = infection {
                // OPV stepwise mutation check
                if inf.strain == InfectionStrain::OPV && inf.mutations < 3 {
                    if let Some(next_day) = inf.next_mutation_day {
                        if t_since_last_exposure >= next_day {
                            inf.mutations += 1;
                            if inf.mutations >= 3 {
                                inf.strain = InfectionStrain::VDPV;
                                inf.next_mutation_day = None;
                            } else {
                                // Sample next mutation wait, accumulate on current day
                                let exp = Exp::new(1.0 / tx_params.mean_reversion_days as f64).unwrap();
                                let wait: f32 = exp.sample(&mut rng) as f32;
                                inf.next_mutation_day = Some(next_day + wait);
                            }
                        }
                    }
                }

                if inf.should_clear(t_since_last_exposure) {
                    commands.entity(entity).remove::<Infection>();
                } else {
                    let age_in_months = individual.age_in_months();
                    inf.viral_shedding = immunity.calculate_viral_shedding(
                        age_in_months,
                        t_since_last_exposure,
                        &params
                    );
                }
            }
        }
    }

    timings.disease_step_ms = t0.elapsed().as_secs_f32() * 1000.0;
}
