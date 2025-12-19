use bevy::prelude::*;

use crate::simulation::{TransmissionEvent, TransmissionLevel};

/// Component for transmission arc visuals
#[derive(Component)]
pub struct TransmissionArc {
    pub lifetime: Timer,
    pub level: TransmissionLevel,
}

/// Spawn transmission arc visuals on events
pub fn spawn_transmission_arcs(
    mut commands: Commands,
    mut events: EventReader<TransmissionEvent>,
    transforms: Query<&GlobalTransform>,
) {
    for event in events.read() {
        let Ok(source_transform) = transforms.get(event.source) else { continue };
        let Ok(target_transform) = transforms.get(event.target) else { continue };

        let start = source_transform.translation().truncate();
        let end = target_transform.translation().truncate();

        // Calculate arc control point (above the line)
        let mid = (start + end) / 2.0;
        let arc_height = match event.level {
            TransmissionLevel::Household => 30.0,
            TransmissionLevel::Neighborhood => 60.0,
            TransmissionLevel::Village => 100.0,
        };

        let color = match event.level {
            TransmissionLevel::Household => Color::rgba(1.0, 0.8, 0.2, 0.9),      // Yellow
            TransmissionLevel::Neighborhood => Color::rgba(0.8, 0.4, 1.0, 0.9),   // Purple
            TransmissionLevel::Village => Color::rgba(0.2, 1.0, 0.6, 0.9),        // Green
        };

        // Spawn arc as a series of small line segments
        spawn_arc_segments(&mut commands, start, end, mid + Vec2::new(0.0, arc_height), color, event.level);
    }
}

fn spawn_arc_segments(
    commands: &mut Commands,
    start: Vec2,
    end: Vec2,
    control: Vec2,
    color: Color,
    level: TransmissionLevel,
) {
    let segments = 8;

    for i in 0..segments {
        let t0 = i as f32 / segments as f32;
        let t1 = (i + 1) as f32 / segments as f32;

        let p0 = quadratic_bezier(start, control, end, t0);
        let p1 = quadratic_bezier(start, control, end, t1);

        let mid = (p0 + p1) / 2.0;
        let diff = p1 - p0;
        let length = diff.length();
        let angle = diff.y.atan2(diff.x);

        commands.spawn((
            TransmissionArc {
                lifetime: Timer::from_seconds(0.8, TimerMode::Once),
                level,
            },
            SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(Vec2::new(length, 2.0)),
                    ..default()
                },
                transform: Transform::from_xyz(mid.x, mid.y, 1.0)
                    .with_rotation(Quat::from_rotation_z(angle)),
                ..default()
            },
        ));
    }
}

fn quadratic_bezier(p0: Vec2, p1: Vec2, p2: Vec2, t: f32) -> Vec2 {
    let t_inv = 1.0 - t;
    p0 * t_inv * t_inv + p1 * 2.0 * t_inv * t + p2 * t * t
}

/// Update and fade out transmission arcs
pub fn update_transmission_arcs(
    mut commands: Commands,
    time: Res<Time>,
    mut arcs: Query<(Entity, &mut TransmissionArc, &mut Sprite)>,
) {
    for (entity, mut arc, mut sprite) in arcs.iter_mut() {
        arc.lifetime.tick(time.delta());

        // Fade out based on remaining time
        let alpha = arc.lifetime.fraction_remaining();
        sprite.color.set_a(alpha * 0.9);

        if arc.lifetime.finished() {
            commands.entity(entity).despawn();
        }
    }
}
