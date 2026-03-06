use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::disease::{InfectionStrain, InfectionSerotype};
use crate::population::{PopulationConfig, ResetPopulationEvent};
use crate::simulation::{
    SimulationTime, SimulationSpeed, SimState,
    TransmissionParams, SeedInfectionEvent
};
use crate::views::AppView;

const HH_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 204, 51);
const NBHD_COLOR: egui::Color32 = egui::Color32::from_rgb(204, 102, 255);
const VILLAGE_COLOR: egui::Color32 = egui::Color32::from_rgb(51, 255, 153);

pub fn neighborhood_controls_ui(
    mut contexts: EguiContexts,
    mut sim_time: ResMut<SimulationTime>,
    mut speed: ResMut<SimulationSpeed>,
    mut tx_params: ResMut<TransmissionParams>,
    mut pop_config: ResMut<PopulationConfig>,
    current_state: Res<State<SimState>>,
    mut next_state: ResMut<NextState<SimState>>,
    mut seed_events: EventWriter<SeedInfectionEvent>,
    mut reset_events: EventWriter<ResetPopulationEvent>,
    mut next_view: ResMut<NextState<AppView>>,
) {
    let ctx = contexts.ctx_mut();

    egui::Window::new("title")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 10.0))
        .show(ctx, |ui| {
            ui.heading("Neighborhood View");
            ui.label(egui::RichText::new("Hover for individual tooltips. Seed infections to watch transmission.").small());
            if ui.small_button("Back to Home").clicked() {
                next_view.set(AppView::Landing);
            }
        });

    let screen = ctx.screen_rect();
    egui::Window::new("Simulation Controls")
        .default_pos(egui::pos2(screen.max.x - 270.0, 10.0))
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.heading(format!("Day: {}", sim_time.day));
            ui.separator();

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
                    sim_time.reset();
                    next_state.set(SimState::Paused);
                    reset_events.send(ResetPopulationEvent);
                }
            });

            ui.add(egui::Slider::new(&mut speed.multiplier, 0.5..=30.0)
                .text("Speed").logarithmic(true));

            ui.separator();

            egui::CollapsingHeader::new("Transmission (contacts/day)")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(HH_COLOR));
                        ui.add(egui::Slider::new(&mut tx_params.beta_hh, 0.0..=10.0).text("Household"));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(NBHD_COLOR));
                        ui.add(egui::Slider::new(&mut tx_params.beta_neighborhood, 0.0..=5.0).text("Neighborhood"));
                    });
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(VILLAGE_COLOR));
                        ui.add(egui::Slider::new(&mut tx_params.beta_village, 0.0..=3.0).text("Village"));
                    });
                    let mut log_dose = tx_params.fecal_oral_dose.log10();
                    if ui.add(egui::Slider::new(&mut log_dose, -7.0..=-3.0).text("Log10 F-O dose")).changed() {
                        tx_params.fecal_oral_dose = 10f32.powf(log_dose);
                    }
                    ui.add(egui::Slider::new(&mut tx_params.opv_shedding_reduction, 0.1..=1.0).text("OPV shed reduction"));
                    ui.add(egui::Slider::new(&mut tx_params.mean_reversion_days, 3.0..=30.0).text("Mean reversion (days)"));
                });

            ui.collapsing("Population (on reset)", |ui| {
                let mut num_nbhd = pop_config.num_neighborhoods as f32;
                if ui.add(egui::Slider::new(&mut num_nbhd, 1.0..=8.0).text("Neighborhoods")).changed() {
                    pop_config.num_neighborhoods = num_nbhd as usize;
                }
                let mut hh_per_nbhd = pop_config.households_per_neighborhood as f32;
                if ui.add(egui::Slider::new(&mut hh_per_nbhd, 2.0..=10.0).text("HH per neighborhood")).changed() {
                    pop_config.households_per_neighborhood = hh_per_nbhd as usize;
                }
                ui.add(egui::Slider::new(&mut pop_config.lifetime_births, 1.0..=10.0).text("Lifetime births/mother"));
            });

            ui.collapsing("Immunity (on reset)", |ui| {
                ui.add(egui::Slider::new(&mut pop_config.time_since_cessation, 0.0..=10.0).text("Years since cessation"));
                ui.add(egui::Slider::new(&mut pop_config.elimination_duration, 0.0..=15.0).text("Elimination duration (yrs)"));
                ui.add(egui::Slider::new(&mut pop_config.vaccine_coverage, 0.0..=1.0).text("Vaccine coverage"));
            });

            ui.separator();

            egui::CollapsingHeader::new("Seed Infection")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (label, count) in [("Seed 1", 1), ("Seed 5", 5), ("Seed 10", 10)] {
                            if ui.button(label).clicked() {
                                seed_events.send(SeedInfectionEvent { count, dose: 1e6, ..default() });
                            }
                        }
                    });
                });

            egui::CollapsingHeader::new("OPV Campaign (under 5s)")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (label, cov) in [("OPV 20%", 0.2), ("OPV 50%", 0.5), ("OPV 80%", 0.8)] {
                            if ui.button(label).clicked() {
                                seed_events.send(SeedInfectionEvent {
                                    dose: 1e6,
                                    min_age: 0.0,
                                    max_age: 5.0,
                                    strain: Some(InfectionStrain::OPV),
                                    serotype: Some(InfectionSerotype::Type2),
                                    coverage: Some(cov),
                                    ..default()
                                });
                            }
                        }
                    });
                });
        });
}
