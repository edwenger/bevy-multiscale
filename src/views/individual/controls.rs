use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::disease::{Immunity, Infection, InfectionStrain, InfectionSerotype};
use crate::population::{Individual, Sex};
use crate::simulation::{
    SimulationTime, SimulationSpeed, SimState, SeedInfectionEvent, TransmissionParams,
};
use crate::views::AppView;


/// Event to trigger respawn of the individual
#[derive(Event)]
pub struct ResetIndividualEvent;

pub fn individual_controls_ui(
    mut contexts: EguiContexts,
    sim_time: Res<SimulationTime>,
    mut speed: ResMut<SimulationSpeed>,
    mut tx_params: ResMut<TransmissionParams>,
    current_state: Res<State<SimState>>,
    mut next_state: ResMut<NextState<SimState>>,
    mut seed_events: EventWriter<SeedInfectionEvent>,
    mut reset_events: EventWriter<ResetIndividualEvent>,
    mut next_view: ResMut<NextState<AppView>>,
    mut individual_q: Query<(&mut Individual, &Immunity, Option<&Infection>)>,
) {
    let ctx = contexts.ctx_mut();

    egui::Window::new("title")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 10.0))
        .show(ctx, |ui| {
            ui.heading("Individual View");
            ui.label(egui::RichText::new("Single person immune response to poliovirus challenge").small());
            if ui.small_button("Back to Home").clicked() {
                next_view.set(AppView::Landing);
            }
        });

    let screen = ctx.screen_rect();
    egui::Window::new("Individual Controls")
        .default_pos(egui::pos2(screen.max.x - 300.0, 10.0))
        .default_width(280.0)
        .show(ctx, |ui| {
            ui.heading(format!("Day: {}", sim_time.day));
            ui.separator();

            // Simulation controls
            ui.horizontal(|ui| {
                let is_running = *current_state.get() == SimState::Running;
                if ui.button(if is_running { "Pause" } else { "Start" }).clicked() {
                    if is_running {
                        next_state.set(SimState::Paused);
                    } else {
                        next_state.set(SimState::Running);
                    }
                }
                if ui.button("Reset").clicked() {
                    reset_events.send(ResetIndividualEvent);
                }
            });

            ui.add(egui::Slider::new(&mut speed.multiplier, 0.5..=30.0)
                .text("Speed").logarithmic(true));

            ui.separator();

            // Individual info & editors
            if let Ok((mut individual, immunity, infection)) = individual_q.get_single_mut() {
                ui.heading("Demographics");

                // Age editor
                let mut age = individual.age;
                if ui.add(egui::Slider::new(&mut age, 0.0..=80.0).text("Age (years)")).changed() {
                    individual.age = age;
                }

                // Sex selector
                ui.horizontal(|ui| {
                    ui.label("Sex:");
                    if ui.selectable_label(individual.sex == Sex::Male, "Male").clicked() {
                        individual.sex = Sex::Male;
                    }
                    if ui.selectable_label(individual.sex == Sex::Female, "Female").clicked() {
                        individual.sex = Sex::Female;
                    }
                });

                ui.separator();

                // Challenge buttons (above transient state display)
                ui.heading("Challenge");
                ui.horizontal(|ui| {
                    for (label, strain, dose) in [
                        ("WPV2 (1e6)", InfectionStrain::WPV, 1e6),
                        ("WPV2 (1e3)", InfectionStrain::WPV, 1e3),
                        ("OPV2 (1e6)", InfectionStrain::OPV, 1e6),
                        ("OPV2 (1e3)", InfectionStrain::OPV, 1e3),
                    ] {
                        if ui.button(label).clicked() {
                            seed_events.send(SeedInfectionEvent {
                                count: 1,
                                dose,
                                strain: Some(strain),
                                serotype: Some(InfectionSerotype::Type2),
                                ..default()
                            });
                        }
                    }
                });

                // OPV reversion parameters
                ui.collapsing("OPV Reversion", |ui| {
                    ui.add(egui::Slider::new(&mut tx_params.mean_reversion_days, 3.0..=30.0)
                        .text("Mean reversion (days)"));
                });

                ui.separator();

                // Current state display (transient — at bottom so it doesn't shift elements above)
                ui.heading("Immune State");
                ui.label(format!("log2(titer): {:.2}", immunity.current_immunity.log2()));
                ui.label(format!("Titer: {:.1}", immunity.current_immunity));

                if let Some(inf) = infection {
                    ui.separator();
                    ui.heading("Active Infection");
                    ui.label(format!("Strain: {:?}", inf.strain));
                    ui.label(format!("Serotype: {:?}", inf.serotype));
                    ui.label(format!("Shedding: {:.2e}", inf.viral_shedding));
                    ui.label(format!("Shed duration: {:.0} days", inf.shed_duration));
                    if inf.strain == InfectionStrain::OPV {
                        ui.label(format!("Mutations: {}/3", inf.mutations));
                    }
                }
            }
        });
}
