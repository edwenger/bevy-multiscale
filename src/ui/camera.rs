use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseButton};
use bevy_egui::EguiContexts;

#[derive(Resource)]
pub struct CameraState {
    pub is_panning: bool,
    pub last_cursor_pos: Option<Vec2>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            is_panning: false,
            last_cursor_pos: None,
        }
    }
}

pub fn camera_zoom_system(
    mut scroll_events: EventReader<MouseWheel>,
    mut projection: Query<&mut OrthographicProjection, With<Camera2d>>,
    mut contexts: EguiContexts,
) {
    if contexts.ctx_mut().wants_pointer_input() {
        return;
    }

    let Ok(mut proj) = projection.get_single_mut() else { return };

    for event in scroll_events.read() {
        let zoom_delta = -event.y * 0.025;
        proj.scale = (proj.scale + zoom_delta * proj.scale).clamp(0.2, 10.0);
    }
}

pub fn camera_pan_system(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut camera_state: ResMut<CameraState>,
    mut camera_query: Query<(&mut Transform, &OrthographicProjection), With<Camera2d>>,
    mut contexts: EguiContexts,
) {
    let Ok(window) = windows.get_single() else { return };
    let Ok((mut transform, projection)) = camera_query.get_single_mut() else { return };

    let egui_wants_pointer = contexts.ctx_mut().wants_pointer_input();

    let pan_button_pressed = mouse_button.pressed(MouseButton::Middle)
        || (mouse_button.pressed(MouseButton::Left) && !egui_wants_pointer);

    if pan_button_pressed {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Some(last_pos) = camera_state.last_cursor_pos {
                let delta = cursor_pos - last_pos;
                transform.translation.x -= delta.x * projection.scale;
                transform.translation.y += delta.y * projection.scale;
            }
            camera_state.last_cursor_pos = Some(cursor_pos);
            camera_state.is_panning = true;
        }
    } else {
        camera_state.last_cursor_pos = None;
        camera_state.is_panning = false;
    }
}
