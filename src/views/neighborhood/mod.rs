mod controls;
mod spawn;
mod viz;

use bevy::prelude::*;

use crate::population::{PopulationConfig, NeedsPopulationSpawn, handle_reset_population};
use crate::ui::{spawn_transmission_arcs, update_transmission_arcs, individual_tooltip};
use crate::ui::camera::{CameraState, camera_zoom_system, camera_pan_system};
use crate::ui::components::IndividualLabel;
use super::AppView;

pub struct NeighborhoodViewPlugin;

impl Plugin for NeighborhoodViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppView::Neighborhood), enter_neighborhood)
            .add_systems(OnExit(AppView::Neighborhood), exit_neighborhood)
            .add_systems(Update, (
                camera_zoom_system,
                camera_pan_system,
                controls::neighborhood_controls_ui,
                viz::update_individual_visuals,
                viz::add_shedding_visuals,
                viz::remove_shedding_visuals,
                update_label_visibility,
                crate::ui::chart::infection_chart_ui,
                spawn_transmission_arcs,
                update_transmission_arcs,
                individual_tooltip,
                handle_reset_population,
                spawn::respawn_neighborhood_population.run_if(resource_exists::<NeedsPopulationSpawn>),
            ).run_if(in_state(AppView::Neighborhood)));
    }
}

fn enter_neighborhood(
    mut commands: Commands,
    mut pop_config: ResMut<PopulationConfig>,
    asset_server: Res<AssetServer>,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    // Set neighborhood defaults
    pop_config.num_neighborhoods = 5;
    pop_config.households_per_neighborhood = 7;

    commands.insert_resource(CameraState::default());

    // Start more zoomed in and shift down so all neighborhoods are visible
    if let Ok((mut transform, mut proj)) = cameras.get_single_mut() {
        proj.scale = 0.65;
        // Center on grid
        transform.translation.x = -150.0;
        transform.translation.y = 160.0;
    }

    let mut rng = rand::thread_rng();
    spawn::spawn_population_internal(&mut commands, &pop_config, &mut rng, &asset_server);
}

fn exit_neighborhood(
    mut commands: Commands,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    commands.remove_resource::<CameraState>();

    if let Ok((mut transform, mut projection)) = cameras.get_single_mut() {
        transform.translation = Vec3::ZERO;
        projection.scale = 1.0;
    }
}

/// Hide labels when zoomed out
fn update_label_visibility(
    projection: Query<&OrthographicProjection, With<Camera2d>>,
    mut labels: Query<&mut Visibility, With<IndividualLabel>>,
) {
    let Ok(proj) = projection.get_single() else { return };
    let visible = proj.scale <= 1.2;

    for mut vis in labels.iter_mut() {
        *vis = if visible { Visibility::Inherited } else { Visibility::Hidden };
    }
}
