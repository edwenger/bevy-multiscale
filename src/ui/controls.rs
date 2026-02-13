use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::disease::{InfectionStrain, InfectionSerotype};
use crate::population::{PopulationConfig, ResetPopulationEvent, BariLayout};
use crate::simulation::{
    SimulationTime, SimulationSpeed, SimState,
    TransmissionParams, SeedInfectionEvent, SystemTimings
};

// Arc colors matching arcs.rs
const HH_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 204, 51);      // Yellow
const NBHD_COLOR: egui::Color32 = egui::Color32::from_rgb(204, 102, 255);   // Purple
const VILLAGE_COLOR: egui::Color32 = egui::Color32::from_rgb(51, 255, 153); // Green

/// Main UI controls panel
pub fn controls_ui(
    mut contexts: EguiContexts,
    mut sim_time: ResMut<SimulationTime>,
    mut speed: ResMut<SimulationSpeed>,
    mut tx_params: ResMut<TransmissionParams>,
    mut pop_config: ResMut<PopulationConfig>,
    mut bari_layout: ResMut<BariLayout>,
    current_state: Res<State<SimState>>,
    mut next_state: ResMut<NextState<SimState>>,
    mut seed_events: EventWriter<SeedInfectionEvent>,
    mut reset_events: EventWriter<ResetPopulationEvent>,
    timings: Res<SystemTimings>,
) {
    let ctx = contexts.ctx_mut();

    // Title in upper-left
    egui::Window::new("title")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 10.0))
        .show(ctx, |ui| {
            ui.heading("Polio multi-scale demo");
            ui.label(egui::RichText::new("Press 'Start' to begin, then 'Seed' infections. Hover for individual tooltips.").small());
        });

    // Controls in upper-right
    egui::Window::new("Simulation Controls")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-10.0, 10.0))
        .default_width(250.0)
        .show(ctx, |ui| {
            // Time display
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
                    sim_time.reset();
                    next_state.set(SimState::Paused);
                    reset_events.send(ResetPopulationEvent);
                }
            });

            ui.add(egui::Slider::new(&mut speed.multiplier, 0.5..=30.0)
                .text("Speed")
                .logarithmic(true));

            ui.add(egui::Slider::new(&mut bari_layout.bari_radius, 5.0..=200.0)
                .text("Bari size"));

            ui.separator();

            // Transmission parameters (expanded by default)
            egui::CollapsingHeader::new("Transmission (contacts/day)")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(HH_COLOR));
                        ui.add(egui::Slider::new(&mut tx_params.beta_hh, 0.0..=10.0)
                            .text("Household"));
                    });

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(NBHD_COLOR));
                        ui.add(egui::Slider::new(&mut tx_params.beta_neighborhood, 0.0..=5.0)
                            .text("Neighborhood"));
                    });

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("●").color(VILLAGE_COLOR));
                        ui.add(egui::Slider::new(&mut tx_params.beta_village, 0.0..=3.0)
                            .text("Village"));
                    });

                    ui.add(egui::Slider::new(&mut tx_params.village_kernel_km, 0.5..=10.0)
                        .text("Village kernel (km)").logarithmic(true));

                    let mut log_dose = tx_params.fecal_oral_dose.log10();
                    if ui.add(egui::Slider::new(&mut log_dose, -7.0..=-3.0)
                        .text("Log10 F-O dose")).changed() {
                        tx_params.fecal_oral_dose = 10f32.powf(log_dose);
                    }

                    ui.add(egui::Slider::new(&mut tx_params.opv_shedding_reduction, 0.1..=1.0)
                        .text("OPV shed reduction"));

                    ui.add(egui::Slider::new(&mut tx_params.mean_reversion_days, 3.0..=30.0)
                        .text("Mean reversion (days)"));
                });

            // Population parameters (for reset)
            ui.collapsing("Population (on reset)", |ui| {
                let max_baris = bari_layout.positions.len().max(1) as f32;
                let mut num_nbhd = pop_config.num_neighborhoods as f32;
                if ui.add(egui::Slider::new(&mut num_nbhd, 1.0..=max_baris)
                    .text("Neighborhoods")).changed() {
                    pop_config.num_neighborhoods = num_nbhd as usize;
                }

                let mut hh_per_nbhd = pop_config.households_per_neighborhood as f32;
                if ui.add(egui::Slider::new(&mut hh_per_nbhd, 2.0..=10.0)
                    .text("HH per neighborhood")).changed() {
                    pop_config.households_per_neighborhood = hh_per_nbhd as usize;
                }

                ui.add(egui::Slider::new(&mut pop_config.lifetime_births, 1.0..=10.0)
                    .text("Lifetime births/mother"));
            });

            // Immunity initialization (for reset)
            ui.collapsing("Immunity (on reset)", |ui| {
                ui.add(egui::Slider::new(&mut pop_config.time_since_cessation, 0.0..=10.0)
                    .text("Years since cessation"));

                ui.add(egui::Slider::new(&mut pop_config.elimination_duration, 0.0..=15.0)
                    .text("Elimination duration (yrs)"));

                ui.add(egui::Slider::new(&mut pop_config.vaccine_coverage, 0.0..=1.0)
                    .text("Vaccine coverage"));
            });

            ui.separator();

            // Seed infection (expanded by default)
            egui::CollapsingHeader::new("Seed Infection")
                .default_open(true)
                .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Seed 1").clicked() {
                        seed_events.send(SeedInfectionEvent {
                            count: 1,
                            dose: 1e6,
                            ..default()
                        });
                    }
                    if ui.button("Seed 5").clicked() {
                        seed_events.send(SeedInfectionEvent {
                            count: 5,
                            dose: 1e6,
                            ..default()
                        });
                    }
                    if ui.button("Seed 10").clicked() {
                        seed_events.send(SeedInfectionEvent {
                            count: 10,
                            dose: 1e6,
                            ..default()
                        });
                    }
                });
            });

            // OPV Campaign (expanded by default)
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

            ui.separator();

            ui.collapsing("Performance", |ui| {
                ui.label(format!("Shedders: {}", timings.shedder_count));
                ui.label(format!("Arcs: {}", timings.arc_count));
                ui.label(format!("Transmission: {:.1} ms", timings.transmission_ms));
                ui.label(format!("Disease step: {:.1} ms", timings.disease_step_ms));
                ui.label(format!("Individual viz: {:.1} ms", timings.individual_viz_ms));
                ui.label(format!("Bari viz: {:.1} ms", timings.bari_viz_ms));
                ui.label(format!("Arc update: {:.1} ms", timings.arc_update_ms));
            });
        });
}
