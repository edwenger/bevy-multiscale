use bevy::prelude::*;

use crate::disease::{Immunity, Infection, DiseaseParams};
use crate::population::Individual;
use super::time::SimulationTime;

/// System to step disease state each day
pub fn step_disease_state(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    params: Res<DiseaseParams>,
    mut query: Query<(Entity, &Individual, &mut Immunity, Option<&mut Infection>)>,
) {
    // Only run on timer tick
    if !sim_time.timer.just_finished() {
        return;
    }

    for (entity, individual, mut immunity, infection) in query.iter_mut() {
        if let Some(ti_infected) = immunity.ti_infected {
            let t_since_last_exposure = sim_time.day as f32 - ti_infected;

            // Update immunity waning
            immunity.calculate_waning(t_since_last_exposure, &params.immunity_waning);

            // Update infection state
            if let Some(mut inf) = infection {
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
}
