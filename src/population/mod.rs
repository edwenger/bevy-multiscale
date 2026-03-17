mod individual;
mod household;
mod neighborhood;
pub mod init;

pub use individual::*;
pub use household::*;
pub use neighborhood::*;
pub use init::{
    PopulationConfig, ResetPopulationEvent, NeedsPopulationSpawn,
    generate_household_members, calculate_initial_immunity,
    handle_reset_population,
};

use bevy::prelude::*;

pub struct PopulationPlugin;

impl Plugin for PopulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PopulationConfig>()
            .add_event::<ResetPopulationEvent>();
    }
}
