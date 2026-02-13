use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::{egui, EguiContexts};

use crate::disease::{Immunity, Infection, format_infection_type};
use crate::population::{Individual, IndividualVisual};

/// Bounding box for hover detection (matches compact sprite size)
const HOVER_WIDTH: f32 = 6.0;
const HOVER_HEIGHT: f32 = 6.0;
const HOVER_Y_OFFSET: f32 = 0.0;

/// System to display tooltips on individual hover
pub fn individual_tooltip(
    mut contexts: EguiContexts,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    individuals: Query<
        (Entity, &GlobalTransform, &Individual, &Immunity, Option<&Infection>),
        With<IndividualVisual>
    >,
) {
    let Ok(window) = windows.get_single() else { return };
    let Ok((camera, camera_transform)) = cameras.get_single() else { return };

    // Get cursor position in world coordinates
    let Some(cursor_pos) = window.cursor_position().and_then(|cursor| {
        camera.viewport_to_world_2d(camera_transform, cursor)
    }) else { return };

    // Find individual under cursor
    let mut hovered: Option<(&Individual, &Immunity, Option<&Infection>)> = None;

    for (_entity, transform, individual, immunity, infection) in individuals.iter() {
        let pos = transform.translation().truncate();

        // Check if cursor is within hover bounds
        let center = Vec2::new(pos.x, pos.y + HOVER_Y_OFFSET);
        let half_size = Vec2::new(HOVER_WIDTH / 2.0, HOVER_HEIGHT / 2.0);

        if cursor_pos.x >= center.x - half_size.x
            && cursor_pos.x <= center.x + half_size.x
            && cursor_pos.y >= center.y - half_size.y
            && cursor_pos.y <= center.y + half_size.y
        {
            hovered = Some((individual, immunity, infection));
            break;
        }
    }

    // Display tooltip if hovering
    if let Some((individual, immunity, infection)) = hovered {
        let cursor_screen = window.cursor_position().unwrap_or_default();

        egui::Window::new("Individual")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .fixed_pos(egui::pos2(cursor_screen.x + 15.0, cursor_screen.y + 15.0))
            .show(contexts.ctx_mut(), |ui| {
                egui::Grid::new("tooltip_grid")
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Age:");
                        ui.label(format!("{:.1} yrs", individual.age));
                        ui.end_row();

                        ui.label("Sex:");
                        ui.label(individual.sex.symbol());
                        ui.end_row();

                        ui.label("log2(titer):");
                        ui.label(format!("{:.1}", immunity.current_immunity.log2()));
                        ui.end_row();

                        if let Some(inf) = infection {
                            ui.label("Shedding:");
                            ui.label(format!("{:.2e}", inf.viral_shedding));
                            ui.end_row();

                            ui.label("Strain:");
                            let strain_label = if inf.strain == crate::disease::InfectionStrain::OPV {
                                format!("{} (mut {}/3)", format_infection_type(inf.strain, inf.serotype), inf.mutations)
                            } else {
                                format_infection_type(inf.strain, inf.serotype)
                            };
                            ui.label(strain_label);
                            ui.end_row();
                        }
                    });
            });
    }
}
