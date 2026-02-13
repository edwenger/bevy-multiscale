mod controls;
mod spawn;
mod viz;

use bevy::prelude::*;

use crate::population::{Individual, PopulationConfig, Sex};
use crate::simulation::{SimState, SimulationTime};
use crate::ui::camera::{CameraState, camera_zoom_system, camera_pan_system};

use super::AppView;

pub struct IndividualViewPlugin;

impl Plugin for IndividualViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<controls::ResetIndividualEvent>()
            .add_systems(OnEnter(AppView::Individual), enter_individual)
            .add_systems(OnExit(AppView::Individual), exit_individual)
            .add_systems(Update, (
                camera_zoom_system,
                camera_pan_system,
                controls::individual_controls_ui,
                viz::update_individual_fill_and_border,
                viz::update_immunity_bar,
                viz::add_shedding_visuals,
                viz::remove_shedding_visuals,
                viz::individual_chart_ui,
                viz::sample_individual_state.run_if(in_state(SimState::Running)),
                handle_reset_individual,
            ).run_if(in_state(AppView::Individual)));
    }
}

fn enter_individual(
    mut commands: Commands,
    config: Res<PopulationConfig>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(viz::IndividualTimeSeries::default());
    commands.insert_resource(CameraState::default());

    // Spawn single individual with defaults
    let mut rng = rand::thread_rng();
    spawn::spawn_single_individual_internal(&mut commands, &config, &mut rng, &asset_server, 1.0, Sex::Male);
}

fn exit_individual(
    mut commands: Commands,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    commands.remove_resource::<viz::IndividualTimeSeries>();
    commands.remove_resource::<CameraState>();

    // Reset camera
    if let Ok((mut transform, mut projection)) = cameras.get_single_mut() {
        transform.translation = Vec3::ZERO;
        projection.scale = 1.0;
    }
}

fn handle_reset_individual(
    mut commands: Commands,
    mut reset_events: EventReader<controls::ResetIndividualEvent>,
    mut sim_time: ResMut<SimulationTime>,
    mut next_state: ResMut<NextState<SimState>>,
    mut time_series: ResMut<viz::IndividualTimeSeries>,
    individuals: Query<(&Individual, Entity)>,
    config: Res<PopulationConfig>,
    asset_server: Res<AssetServer>,
) {
    for _event in reset_events.read() {
        // Read current age/sex before despawning
        let (age, sex) = individuals.iter().next()
            .map(|(ind, _)| (ind.age, ind.sex))
            .unwrap_or((1.0, Sex::Male));

        // Reset simulation state
        sim_time.reset();
        next_state.set(SimState::Paused);
        *time_series = viz::IndividualTimeSeries::default();

        // Despawn existing individual(s)
        for (_, entity) in individuals.iter() {
            commands.entity(entity).despawn_recursive();
        }

        // Respawn with preserved age/sex
        let mut rng = rand::thread_rng();
        spawn::spawn_single_individual_internal(&mut commands, &config, &mut rng, &asset_server, age, sex);
    }
}
