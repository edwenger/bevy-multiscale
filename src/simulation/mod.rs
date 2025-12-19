mod time;
mod transmission;
mod step;
mod campaign;

pub use time::*;
pub use transmission::*;
pub use step::*;
pub use campaign::*;

use bevy::prelude::*;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationTime::default())
            .insert_resource(SimulationSpeed::default())
            .insert_resource(TransmissionParams::default())
            .init_state::<SimState>()
            .add_event::<TransmissionEvent>()
            .add_event::<SeedInfectionEvent>()
            .add_systems(Update, (
                advance_simulation_time,
                step_disease_state,
                transmission_system,
                handle_seed_infection,
            ).chain().run_if(in_state(SimState::Running)));
    }
}
