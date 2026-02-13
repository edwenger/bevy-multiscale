mod components;
mod controls;
mod spawn;
mod viz;

use bevy::prelude::*;

use crate::population::{PopulationConfig, NeedsPopulationSpawn, handle_reset_population};
use crate::ui::{spawn_transmission_arcs, update_transmission_arcs, individual_tooltip};
use super::AppView;

pub struct NeighborhoodViewPlugin;

impl Plugin for NeighborhoodViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppView::Neighborhood), enter_neighborhood)
            .add_systems(OnExit(AppView::Neighborhood), exit_neighborhood)
            .add_systems(Update, (
                controls::neighborhood_controls_ui,
                viz::update_individual_visuals,
                viz::add_shedding_visuals,
                viz::remove_shedding_visuals,
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
) {
    // Set neighborhood defaults
    pop_config.num_neighborhoods = 5;
    pop_config.households_per_neighborhood = 7;

    let mut rng = rand::thread_rng();
    spawn::spawn_population_internal(&mut commands, &pop_config, &mut rng, &asset_server);
}

fn exit_neighborhood(
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    if let Ok((mut transform, mut projection)) = cameras.get_single_mut() {
        transform.translation = Vec3::ZERO;
        projection.scale = 1.0;
    }
}
