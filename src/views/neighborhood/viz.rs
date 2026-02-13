use bevy::prelude::*;

use crate::disease::{Immunity, Infection};
use crate::population::{Individual, IndividualVisual};
use super::components::{ImmunityBar, SheddingBar};

const BAR_WIDTH: f32 = 5.0;

/// Update individual visual representations based on state
pub fn update_individual_visuals(
    mut individuals: Query<(&Immunity, &Children, &mut Sprite, Option<&Infection>), (With<Individual>, With<IndividualVisual>)>,
    mut bar_query: Query<(&mut Transform, &mut Sprite), (With<ImmunityBar>, Without<IndividualVisual>)>,
) {
    for (immunity, children, mut sprite, infection) in individuals.iter_mut() {
        sprite.color = if infection.is_some() {
            Color::rgb(0.85, 0.25, 0.25)
        } else {
            Color::rgb(0.4, 0.4, 0.4)
        };

        for &child in children.iter() {
            if let Ok((mut transform, mut bar_sprite)) = bar_query.get_mut(child) {
                let height = (immunity.current_immunity.log10() * 15.0).max(5.0).min(100.0);
                bar_sprite.custom_size = Some(Vec2::new(BAR_WIDTH, height));
                transform.translation = Vec3::new(-3.0, height / 2.0 + 5.0, 0.1);
            }
        }
    }
}

/// Add shedding bars when infection is added
pub fn add_shedding_visuals(
    mut commands: Commands,
    new_infections: Query<(Entity, &Infection), Added<Infection>>,
) {
    for (entity, infection) in new_infections.iter() {
        let height = (infection.viral_shedding.log10() * 8.0).max(5.0).min(80.0);

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                SheddingBar,
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(0.9, 0.2, 0.2, 0.8),
                        custom_size: Some(Vec2::new(BAR_WIDTH, height)),
                        ..default()
                    },
                    transform: Transform::from_xyz(3.0, height / 2.0 + 5.0, 0.1),
                    ..default()
                },
            ));
        });
    }
}

/// Update shedding bars and remove when infection clears
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
        for &child in children.iter() {
            if let Ok((mut transform, mut sprite)) = bar_query.get_mut(child) {
                let height = (infection.viral_shedding.log10() * 8.0).max(5.0).min(80.0);
                sprite.custom_size = Some(Vec2::new(BAR_WIDTH, height));
                transform.translation = Vec3::new(3.0, height / 2.0 + 5.0, 0.1);
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
