pub mod bari;
mod components;
mod controls;
mod spawn;
mod viz;

use bevy::prelude::*;

use crate::population::{PopulationConfig, NeedsPopulationSpawn, handle_reset_population};
use crate::ui::{spawn_transmission_arcs, update_transmission_arcs, individual_tooltip};
use crate::ui::camera::{CameraState, camera_zoom_system, camera_pan_system};
use super::AppView;

pub struct RegionViewPlugin;

impl Plugin for RegionViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppView::Region), enter_region)
            .add_systems(OnExit(AppView::Region), exit_region)
            .add_systems(Update, (
                camera_zoom_system,
                camera_pan_system,
                controls::region_controls_ui,
                viz::update_individual_visuals,
                viz::update_bari_visuals,
                viz::update_label_visibility,
                spawn::update_bari_display,
                spawn_transmission_arcs,
                update_transmission_arcs,
                individual_tooltip,
                crate::ui::chart::infection_chart_ui,
                handle_reset_population,
                spawn::respawn_region_population.run_if(resource_exists::<NeedsPopulationSpawn>),
            ).run_if(in_state(AppView::Region)));
    }
}

fn enter_region(
    mut commands: Commands,
    mut pop_config: ResMut<PopulationConfig>,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    // Set region defaults
    pop_config.num_neighborhoods = 2000;
    pop_config.households_per_neighborhood = 6;

    // Insert BariLayout resource
    let layout = bari::BariLayout::from_csv();

    commands.insert_resource(CameraState::default());

    // Spawn population
    let mut rng = rand::thread_rng();
    spawn::spawn_region_population_internal(
        &mut commands, &pop_config, &layout, &mut rng, &asset_server, &mut images,
    );

    // Auto-center camera
    auto_center_camera(&layout, &mut cameras);

    commands.insert_resource(layout);
}

pub(super) fn auto_center_camera(
    bari_layout: &bari::BariLayout,
    cameras: &mut Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    if bari_layout.positions.is_empty() {
        return;
    }
    let Ok((mut transform, mut projection)) = cameras.get_single_mut() else { return };

    let n = bari_layout.positions.len() as f32;
    let cx: f32 = bari_layout.positions.iter().map(|p| p.x).sum::<f32>() / n;
    let cy: f32 = bari_layout.positions.iter().map(|p| p.y).sum::<f32>() / n;

    let min_x = bari_layout.positions.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = bari_layout.positions.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = bari_layout.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = bari_layout.positions.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

    let span_x = (max_x - min_x) + 200.0;
    let span_y = (max_y - min_y) + 150.0;

    transform.translation.x = cx;
    transform.translation.y = cy + 0.05 * span_y;

    let scale_x = span_x / 1200.0;
    let scale_y = span_y / 900.0;
    projection.scale = scale_x.max(scale_y).max(0.5).min(10.0);
}

fn exit_region(
    mut commands: Commands,
    mut cameras: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    commands.remove_resource::<bari::BariLayout>();
    commands.remove_resource::<CameraState>();

    // Reset camera
    if let Ok((mut transform, mut projection)) = cameras.get_single_mut() {
        transform.translation = Vec3::ZERO;
        projection.scale = 1.0;
    }
}
