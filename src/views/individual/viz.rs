use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_plot::{Plot, Line, PlotPoints};

use crate::disease::{Immunity, Infection, InfectionStrain};
use crate::population::Individual;
use crate::simulation::SimulationTime;

/// Resource to store sampled time-series data for the individual
#[derive(Resource, Default)]
pub struct IndividualTimeSeries {
    /// (day, log2_titer, shedding_log10, strain_color_idx)
    pub samples: Vec<(f32, f32, f32, Option<u8>)>,
    pub last_sampled_day: u32,
}

/// Sample the individual's state each simulation day
pub fn sample_individual_state(
    sim_time: Res<SimulationTime>,
    individual_q: Query<(&Immunity, Option<&Infection>), With<Individual>>,
    mut time_series: ResMut<IndividualTimeSeries>,
) {
    if !sim_time.timer.just_finished() {
        return;
    }

    if sim_time.day <= time_series.last_sampled_day {
        return;
    }

    if let Ok((immunity, infection)) = individual_q.get_single() {
        let log2_titer = immunity.current_immunity.log2();
        let (shed_log10, strain_idx) = if let Some(inf) = infection {
            let shed = inf.viral_shedding.max(1.0).log10();
            let idx = match inf.strain {
                InfectionStrain::WPV => 0,
                InfectionStrain::VDPV => 1,
                InfectionStrain::OPV => 2,
            };
            (shed, Some(idx))
        } else {
            (0.0, None)
        };

        time_series.samples.push((sim_time.day as f32, log2_titer, shed_log10, strain_idx));
        time_series.last_sampled_day = sim_time.day;
    }
}

/// Update the individual sprite color based on immunity/infection
pub fn update_individual_sprite(
    mut individuals: Query<(&Immunity, &mut Sprite, Option<&Infection>), With<Individual>>,
) {
    for (immunity, mut sprite, infection) in individuals.iter_mut() {
        if let Some(inf) = infection {
            sprite.color = match inf.strain {
                InfectionStrain::WPV  => Color::rgb(0.9, 0.15, 0.15),
                InfectionStrain::VDPV => Color::rgb(1.0, 0.6, 0.0),
                InfectionStrain::OPV  => Color::rgb(0.0, 0.85, 0.85),
            };
        } else {
            // Color by immunity: brown (naive) -> green (immune)
            let t = (immunity.current_immunity.log2() / 10.0).clamp(0.0, 1.0);
            let color = if t < 0.5 {
                let s = t / 0.5;
                Color::rgb(0.55 + s * 0.41, 0.35 + s * 0.55, 0.15 + s * 0.60)
            } else {
                let s = (t - 0.5) / 0.5;
                Color::rgb(0.96 + s * (0.2 - 0.96), 0.90 + s * (0.75 - 0.90), 0.75 + s * (0.3 - 0.75))
            };
            sprite.color = color;
        }
    }
}

/// Render immunity time-series chart using egui_plot
pub fn individual_chart_ui(
    mut contexts: EguiContexts,
    time_series: Res<IndividualTimeSeries>,
    sim_time: Res<SimulationTime>,
) {
    let ctx = contexts.ctx_mut();

    egui::Window::new("Immune Response")
        .default_pos(egui::pos2(50.0, 400.0))
        .default_size(egui::vec2(700.0, 300.0))
        .collapsible(true)
        .resizable(true)
        .show(ctx, |ui| {
            if time_series.samples.is_empty() {
                ui.label(format!("Day {} — start simulation and challenge to see immune response", sim_time.day));
                return;
            }

            // Build line data
            let titer_points: PlotPoints = time_series.samples.iter()
                .map(|(day, log2_titer, _, _)| [*day as f64, *log2_titer as f64])
                .collect();

            let shedding_points: PlotPoints = time_series.samples.iter()
                .map(|(day, _, shed, _)| [*day as f64, *shed as f64])
                .collect();

            let titer_line = Line::new(titer_points)
                .color(egui::Color32::from_rgb(50, 180, 220))
                .name("log2(titer)")
                .width(2.0);

            let shedding_line = Line::new(shedding_points)
                .color(egui::Color32::from_rgb(230, 80, 80))
                .name("log10(shedding)")
                .width(2.0);

            Plot::new("individual_immune_response")
                .legend(egui_plot::Legend::default())
                .x_axis_label("Day")
                .y_axis_label("Value")
                .auto_bounds([true, true].into())
                .height(ui.available_height().max(150.0))
                .show(ui, |plot_ui| {
                    plot_ui.line(titer_line);
                    plot_ui.line(shedding_line);
                });
        });
}
