mod camera;
mod chart;
mod controls;
mod grid;
mod individual_viz;
mod arcs;
mod tooltip;

pub use camera::*;
pub use chart::*;
pub use controls::*;
pub use grid::*;
pub use individual_viz::*;
pub use arcs::*;
pub use tooltip::*;

use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(CameraState::default())
            .add_systems(Update, (
                camera_zoom_system,
                camera_pan_system,
                controls_ui,
                update_individual_visuals,
                update_bari_visuals,
                update_label_visibility,
                spawn_transmission_arcs,
                update_transmission_arcs,
                individual_tooltip,
                infection_chart_ui,
            ));
    }
}
