pub mod individual;
pub mod neighborhood;
pub mod region;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::population::{Individual, Household, Neighborhood};
use crate::simulation::{SimState, SimulationTime, InfectionTimeSeries};
use crate::ui::TransmissionArc;

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppView {
    #[default]
    Landing,
    Individual,
    Neighborhood,
    Region,
}

pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<AppView>()
            .add_systems(Update, landing_page_ui.run_if(in_state(AppView::Landing)))
            .add_systems(OnExit(AppView::Individual), cleanup_all_entities)
            .add_systems(OnExit(AppView::Neighborhood), cleanup_all_entities)
            .add_systems(OnExit(AppView::Region), cleanup_all_entities)
            .add_plugins(individual::IndividualViewPlugin)
            .add_plugins(neighborhood::NeighborhoodViewPlugin)
            .add_plugins(region::RegionViewPlugin);
    }
}

fn cleanup_all_entities(
    mut commands: Commands,
    individuals: Query<Entity, With<Individual>>,
    households: Query<Entity, With<Household>>,
    neighborhoods: Query<Entity, With<Neighborhood>>,
    arcs: Query<Entity, With<TransmissionArc>>,
    mut sim_time: ResMut<SimulationTime>,
    mut next_sim_state: ResMut<NextState<SimState>>,
    mut time_series: ResMut<InfectionTimeSeries>,
) {
    for entity in individuals.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in households.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in neighborhoods.iter() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in arcs.iter() {
        commands.entity(entity).despawn_recursive();
    }

    sim_time.reset();
    next_sim_state.set(SimState::Paused);
    *time_series = InfectionTimeSeries::default();
}

fn landing_page_ui(
    mut contexts: EguiContexts,
    mut next_view: ResMut<NextState<AppView>>,
) {
    let ctx = contexts.ctx_mut();

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(80.0);
            ui.heading(egui::RichText::new("Polio Multi-Scale Demo").size(32.0));
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Explore poliovirus transmission dynamics at three scales").size(16.0));
            ui.add_space(40.0);

            let cards: Vec<(&str, &str, &str, AppView)> = vec![
                ("Individual", "Single person immune response",
                 "Edit age & sex, challenge with WPV or OPV, and watch real-time immunity dynamics on a time-series chart.",
                 AppView::Individual),
                ("Neighborhood", "Household transmission grid",
                 "5 neighborhoods in a grid layout with blue immunity bars and red shedding bars. Seed infections and watch transmission arcs.",
                 AppView::Neighborhood),
                ("Region", "2000-bari spatial landscape",
                 "Spatial bari layout from CSV, zoom & pan, OPV campaigns with VDPV mutation emergence, and daily infection time-series chart.",
                 AppView::Region),
            ];

            // Constrain cards to middle half of window
            let avail = ui.available_width();
            let card_width = (avail * 0.5).max(400.0);
            let margin = (avail - card_width) / 2.0;

            for (title, subtitle, description, view) in cards {
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(16.0))
                        .show(ui, |ui| {
                            ui.set_width(card_width);
                            ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.heading(title);
                                ui.label(subtitle);
                                ui.label(egui::RichText::new(description).small());
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(egui::RichText::new("Launch").size(16.0)).clicked() {
                                    next_view.set(view);
                                }
                            });
                        });
                    });
                });
                ui.add_space(8.0);
            }
        });
    });
}
