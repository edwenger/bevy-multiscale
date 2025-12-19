mod controls;
mod grid;
mod individual_viz;
mod arcs;
mod tooltip;

pub use controls::*;
pub use grid::*;
pub use individual_viz::*;
pub use arcs::*;
pub use tooltip::*;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            controls_ui,
            update_individual_visuals,
            add_shedding_visuals,
            remove_shedding_visuals,
            spawn_transmission_arcs,
            update_transmission_arcs,
            individual_tooltip,
        ));
    }
}
