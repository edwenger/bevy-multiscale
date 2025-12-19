mod immunity;
mod infection;
mod params;

pub use immunity::*;
pub use infection::*;
pub use params::*;

use bevy::prelude::*;

pub struct DiseasePlugin;

impl Plugin for DiseasePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DiseaseParams::default());
    }
}
