mod individual;
mod household;
mod neighborhood;
mod bari;
mod init;

pub use individual::*;
pub use household::*;
pub use neighborhood::*;
pub use bari::*;
pub use init::*;

use bevy::prelude::*;

/// Marker resource indicating population needs to be spawned
#[derive(Resource, Default)]
pub struct NeedsPopulationSpawn;

pub struct PopulationPlugin;

impl Plugin for PopulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PopulationConfig::default())
            .insert_resource(BariLayout::from_csv())
            .add_event::<ResetPopulationEvent>()
            .add_systems(Startup, spawn_population)
            .add_systems(Update, (
                handle_reset_population,
                respawn_population.run_if(resource_exists::<NeedsPopulationSpawn>),
                update_bari_display,
            ).chain());
    }
}
