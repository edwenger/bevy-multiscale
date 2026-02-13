mod controls;
mod spawn;
mod viz;

use bevy::prelude::*;

use crate::simulation::SimState;
use super::AppView;

pub struct IndividualViewPlugin;

impl Plugin for IndividualViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppView::Individual), enter_individual)
            .add_systems(OnExit(AppView::Individual), exit_individual)
            .add_systems(Update, (
                controls::individual_controls_ui,
                viz::update_individual_sprite,
                viz::individual_chart_ui,
                viz::sample_individual_state.run_if(in_state(SimState::Running)),
            ).run_if(in_state(AppView::Individual)));
    }
}

fn enter_individual(
    mut commands: Commands,
    config: Res<crate::population::PopulationConfig>,
) {
    commands.insert_resource(viz::IndividualTimeSeries::default());

    // Spawn single individual
    let mut rng = rand::thread_rng();
    spawn::spawn_single_individual_internal(&mut commands, &config, &mut rng);
}

fn exit_individual(
    mut commands: Commands,
) {
    commands.remove_resource::<viz::IndividualTimeSeries>();
}
