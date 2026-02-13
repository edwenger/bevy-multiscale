use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use egui_plot::{Plot, Line, PlotPoints};

use crate::disease::{Immunity, Infection, InfectionStrain};
use crate::population::Individual;
use crate::simulation::SimulationTime;
use crate::ui::components::*;
use crate::ui::viz::{immunity_to_fill_color, shedding_border_color, strain_color};

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

const BAR_WIDTH: f32 = 8.0;
const BAR_X_LEFT: f32 = -20.0;
const BAR_X_RIGHT: f32 = 20.0;
const BAR_Y_OFFSET: f32 = 30.0;

/// Update fill color from immunity and border from shedding
pub fn update_individual_fill_and_border(
    individuals: Query<(&Immunity, &Children, Option<&Infection>), With<Individual>>,
    mut fills: Query<&mut Sprite, (With<IndividualFill>, Without<IndividualBorder>)>,
    mut borders: Query<(&mut Sprite, &mut Transform), (With<IndividualBorder>, Without<IndividualFill>)>,
) {
    for (immunity, children, infection) in individuals.iter() {
        let fill_color = immunity_to_fill_color(immunity.current_immunity);
        let (border_color, border_size) = shedding_border_color(infection, 60.0);

        for &child in children.iter() {
            if let Ok(mut fill_sprite) = fills.get_mut(child) {
                fill_sprite.color = fill_color;
            }

            if let Ok((mut border_sprite, mut border_transform)) = borders.get_mut(child) {
                border_sprite.color = border_color;
                border_sprite.custom_size = Some(Vec2::new(border_size, border_size));
                border_transform.scale = Vec3::ONE;
            }
        }
    }
}

/// Update immunity bar height and color
pub fn update_immunity_bar(
    individuals: Query<(&Immunity, &Children), With<Individual>>,
    mut bar_query: Query<(&mut Transform, &mut Sprite), (With<ImmunityBar>, Without<IndividualFill>, Without<IndividualBorder>)>,
) {
    for (immunity, children) in individuals.iter() {
        let fill_color = immunity_to_fill_color(immunity.current_immunity);
        for &child in children.iter() {
            if let Ok((mut transform, mut sprite)) = bar_query.get_mut(child) {
                let height = (immunity.current_immunity.log10() * 30.0).max(10.0).min(200.0);
                sprite.custom_size = Some(Vec2::new(BAR_WIDTH, height));
                sprite.color = fill_color;
                transform.translation = Vec3::new(BAR_X_LEFT, height / 2.0 + BAR_Y_OFFSET, 0.1);
            }
        }
    }
}

/// Add shedding bar when infection starts
pub fn add_shedding_visuals(
    mut commands: Commands,
    new_infections: Query<(Entity, &Infection), Added<Infection>>,
) {
    for (entity, infection) in new_infections.iter() {
        let height = (infection.viral_shedding.log10() * 15.0).max(10.0).min(200.0);
        let color = strain_color(infection.strain);

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                SheddingBar,
                SpriteBundle {
                    sprite: Sprite {
                        color: color.with_a(0.8),
                        custom_size: Some(Vec2::new(BAR_WIDTH, height)),
                        ..default()
                    },
                    transform: Transform::from_xyz(BAR_X_RIGHT, height / 2.0 + BAR_Y_OFFSET, 0.1),
                    ..default()
                },
            ));
        });
    }
}

/// Update and remove shedding bars
pub fn remove_shedding_visuals(
    mut commands: Commands,
    mut removals: RemovedComponents<Infection>,
    individuals: Query<&Children, With<Individual>>,
    shedding_bars: Query<Entity, With<SheddingBar>>,
    infected: Query<(&Infection, &Children), With<Individual>>,
    mut bar_query: Query<(&mut Transform, &mut Sprite), With<SheddingBar>>,
) {
    // Update existing shedding bars
    for (infection, children) in infected.iter() {
        let color = strain_color(infection.strain);
        for &child in children.iter() {
            if let Ok((mut transform, mut sprite)) = bar_query.get_mut(child) {
                let height = (infection.viral_shedding.log10() * 15.0).max(10.0).min(200.0);
                sprite.custom_size = Some(Vec2::new(BAR_WIDTH, height));
                sprite.color = color.with_a(0.8);
                transform.translation = Vec3::new(BAR_X_RIGHT, height / 2.0 + BAR_Y_OFFSET, 0.1);
            }
        }
    }

    // Remove shedding bars from recovered individuals
    for recovered_entity in removals.read() {
        if let Ok(children) = individuals.get(recovered_entity) {
            for &child in children.iter() {
                if shedding_bars.contains(child) {
                    commands.entity(child).despawn();
                }
            }
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
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -10.0))
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
