use bevy::prelude::*;
use bevy::log::LogPlugin;
use bevy_egui::EguiPlugin;

use bevy_multiscale::disease;
use bevy_multiscale::population;
use bevy_multiscale::simulation;
use bevy_multiscale::ui;

fn main() {
    env_logger::try_init().ok();

    App::new()
        .insert_resource(ClearColor(Color::rgb(0.1, 0.1, 0.12)))
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugins(EguiPlugin)
        .add_plugins(disease::DiseasePlugin)
        .add_plugins(population::PopulationPlugin)
        .add_plugins(simulation::SimulationPlugin)
        .add_plugins(ui::UiPlugin)
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}
