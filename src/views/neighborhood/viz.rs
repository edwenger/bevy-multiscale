use bevy::prelude::*;

use crate::disease::{Immunity, Infection};
use crate::population::{Individual, IndividualVisual};
use crate::ui::components::*;
use crate::ui::viz::{immunity_to_fill_color, shedding_border_color, strain_color};

const BAR_WIDTH: f32 = 4.0;

/// Update individual fill/border and immunity bar
pub fn update_individual_visuals(
    individuals: Query<(&Immunity, &Children, Option<&Infection>), (With<Individual>, With<IndividualVisual>)>,
    mut fills: Query<&mut Sprite, (With<IndividualFill>, Without<IndividualBorder>)>,
    mut borders: Query<(&mut Sprite, &mut Transform), (With<IndividualBorder>, Without<IndividualFill>)>,
    mut bar_query: Query<(&mut Transform, &mut Sprite), (With<ImmunityBar>, Without<IndividualFill>, Without<IndividualBorder>)>,
) {
    for (immunity, children, infection) in individuals.iter() {
        let fill_color = immunity_to_fill_color(immunity.current_immunity);
        let (border_color, border_size) = shedding_border_color(infection, 14.0);

        for &child in children.iter() {
            if let Ok(mut fill_sprite) = fills.get_mut(child) {
                fill_sprite.color = fill_color;
            }

            if let Ok((mut border_sprite, mut border_transform)) = borders.get_mut(child) {
                border_sprite.color = border_color;
                border_sprite.custom_size = Some(Vec2::new(border_size, border_size));
                border_transform.scale = Vec3::ONE;
            }

            if let Ok((mut transform, mut bar_sprite)) = bar_query.get_mut(child) {
                let height = (immunity.current_immunity.log10() * 15.0).max(5.0).min(100.0);
                bar_sprite.custom_size = Some(Vec2::new(BAR_WIDTH, height));
                bar_sprite.color = fill_color;
                transform.translation = Vec3::new(-3.0, height / 2.0 + 6.0, 0.1);
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
                    transform: Transform::from_xyz(3.0, height / 2.0 + 6.0, 0.1),
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
        let color = strain_color(infection.strain);
        for &child in children.iter() {
            if let Ok((mut transform, mut sprite)) = bar_query.get_mut(child) {
                let height = (infection.viral_shedding.log10() * 8.0).max(5.0).min(80.0);
                sprite.custom_size = Some(Vec2::new(BAR_WIDTH, height));
                sprite.color = color.with_a(0.8);
                transform.translation = Vec3::new(3.0, height / 2.0 + 6.0, 0.1);
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
