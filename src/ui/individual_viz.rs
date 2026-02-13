use std::collections::HashMap;
use bevy::prelude::*;

use crate::disease::{Immunity, Infection, InfectionStrain};
use crate::population::{
    Individual, IndividualFill, IndividualBorder, IndividualLabel,
    BariBorder, BariFill, Neighborhood, NeighborhoodMember,
};

/// 3-stop gradient: brown (t=0) → beige (t=0.5) → green (t=1.0)
fn gradient_brown_beige_green(t: f32) -> Color {
    let (r, g, b) = if t < 0.5 {
        let s = t / 0.5;
        (0.55 + s * (0.96 - 0.55),
         0.35 + s * (0.90 - 0.35),
         0.15 + s * (0.75 - 0.15))
    } else {
        let s = (t - 0.5) / 0.5;
        (0.96 + s * (0.2 - 0.96),
         0.90 + s * (0.75 - 0.90),
         0.75 + s * (0.3 - 0.75))
    };
    Color::rgb(r, g, b)
}

/// Convert immunity titer to fill color: brown (naive) → beige → green (immune)
pub fn immunity_to_fill_color(titer: f32) -> Color {
    let t = (titer.log2() / 10.0).clamp(0.0, 1.0);
    gradient_brown_beige_green(t)
}

/// Bari-level fill color with stretched range [log2 2..8] for better discrimination
pub fn bari_immunity_to_fill_color(titer: f32) -> Color {
    let t = ((titer.log2() - 2.0) / 6.0).clamp(0.0, 1.0);
    gradient_brown_beige_green(t)
}

/// Update fill color from immunity and border color/size from shedding
pub fn update_individual_visuals(
    individuals: Query<(&Immunity, &Children, Option<&Infection>), With<Individual>>,
    mut fills: Query<&mut Sprite, (With<IndividualFill>, Without<IndividualBorder>, Without<BariFill>, Without<BariBorder>)>,
    mut borders: Query<(&mut Sprite, &mut Transform), (With<IndividualBorder>, Without<IndividualFill>, Without<BariFill>, Without<BariBorder>)>,
    mut timings: ResMut<crate::simulation::SystemTimings>,
) {
    let t0 = std::time::Instant::now();

    for (immunity, children, infection) in individuals.iter() {
        let fill_color = immunity_to_fill_color(immunity.current_immunity);

        for &child in children.iter() {
            // Update fill sprite
            if let Ok(mut fill_sprite) = fills.get_mut(child) {
                fill_sprite.color = fill_color;
            }

            // Update border sprite
            if let Ok((mut border_sprite, mut border_transform)) = borders.get_mut(child) {
                if let Some(inf) = infection {
                    // Shedding: color by strain, size/opacity by shedding magnitude
                    let base_color = match inf.strain {
                        InfectionStrain::WPV  => Color::rgb(0.9, 0.15, 0.15),  // Red
                        InfectionStrain::VDPV => Color::rgb(1.0, 0.6, 0.0),    // Orange
                        InfectionStrain::OPV  => Color::rgb(0.0, 0.85, 0.85),  // Cyan
                    };
                    let log_shed = inf.viral_shedding.log10().clamp(2.0, 8.0);
                    let t = (log_shed - 2.0) / 6.0; // 0..1
                    let alpha = 0.3 + 0.7 * t;
                    border_sprite.color = base_color.with_a(alpha);
                    // Border thickness: 0.5-1.5 → outer sprite 4+2*thickness
                    let thickness = 0.5 + 1.0 * t;
                    let outer_size = 4.0 + 2.0 * thickness;
                    border_sprite.custom_size = Some(Vec2::new(outer_size, outer_size));
                    border_transform.scale = Vec3::ONE;
                } else {
                    // No infection: thin dark gray border
                    border_sprite.color = Color::rgba(0.3, 0.3, 0.3, 0.5);
                    border_sprite.custom_size = Some(Vec2::new(5.0, 5.0));
                    border_transform.scale = Vec3::ONE;
                }
            }
        }
    }

    timings.individual_viz_ms = t0.elapsed().as_secs_f32() * 1000.0;
}

/// Hide labels when zoomed out (projection.scale > 0.8)
pub fn update_label_visibility(
    projection: Query<&OrthographicProjection, With<Camera2d>>,
    mut labels: Query<&mut Visibility, With<IndividualLabel>>,
) {
    let Ok(proj) = projection.get_single() else { return };
    let visible = proj.scale <= 0.8;

    for mut vis in labels.iter_mut() {
        *vis = if visible { Visibility::Inherited } else { Visibility::Hidden };
    }
}

/// Update bari-level aggregate visuals: fill=geometric mean immunity, border=total shedding
pub fn update_bari_visuals(
    individuals: Query<(&Immunity, &NeighborhoodMember, Option<&Infection>), With<Individual>>,
    neighborhoods: Query<(Entity, &Children), With<Neighborhood>>,
    mut fills: Query<&mut Sprite, (With<BariFill>, Without<BariBorder>, Without<IndividualFill>, Without<IndividualBorder>)>,
    mut borders: Query<&mut Sprite, (With<BariBorder>, Without<BariFill>, Without<IndividualFill>, Without<IndividualBorder>)>,
    mut timings: ResMut<crate::simulation::SystemTimings>,
) {
    let t0 = std::time::Instant::now();

    // Build per-neighborhood aggregates: (log_titers_sum, count, wpv_shed, vdpv_shed, opv_shed)
    let mut nbhd_data: HashMap<Entity, (f32, usize, f32, f32, f32)> = HashMap::new();

    for (immunity, nbhd_member, infection) in individuals.iter() {
        let entry = nbhd_data.entry(nbhd_member.neighborhood_id).or_insert((0.0, 0, 0.0, 0.0, 0.0));
        entry.0 += immunity.current_immunity.max(1.0).ln();
        entry.1 += 1;
        if let Some(inf) = infection {
            match inf.strain {
                InfectionStrain::WPV  => entry.2 += inf.viral_shedding,
                InfectionStrain::VDPV => entry.3 += inf.viral_shedding,
                InfectionStrain::OPV  => entry.4 += inf.viral_shedding,
            }
        }
    }

    for (nbhd_entity, children) in neighborhoods.iter() {
        let Some(&(log_sum, count, wpv_shed, vdpv_shed, opv_shed)) = nbhd_data.get(&nbhd_entity) else {
            continue;
        };
        if count == 0 {
            continue;
        }

        // Geometric mean immunity
        let geo_mean = (log_sum / count as f32).exp();
        let fill_color = bari_immunity_to_fill_color(geo_mean);

        // Border color priority: WPV (red) > VDPV (orange) > OPV (cyan)
        let total_shed = wpv_shed + vdpv_shed + opv_shed;

        for &child in children.iter() {
            // Update fill sprite
            if let Ok(mut fill_sprite) = fills.get_mut(child) {
                fill_sprite.color = fill_color;
            }

            // Update border sprite
            if let Ok(mut border_sprite) = borders.get_mut(child) {
                if total_shed > 0.0 {
                    let base_color = if wpv_shed > 0.0 {
                        Color::rgb(0.9, 0.15, 0.15) // Red for WPV
                    } else if vdpv_shed > 0.0 {
                        Color::rgb(1.0, 0.6, 0.0)   // Orange for VDPV
                    } else {
                        Color::rgb(0.0, 0.85, 0.85)  // Cyan for OPV
                    };
                    // Log-scaled alpha: 0.15 at low shedding, 0.8 at high
                    let log_shed = total_shed.log10().clamp(2.0, 8.0);
                    let t = (log_shed - 2.0) / 6.0;
                    let alpha = 0.15 + 0.65 * t;
                    border_sprite.color = base_color.with_a(alpha);
                } else {
                    border_sprite.color = Color::rgba(0.25, 0.25, 0.30, 0.15);
                }
            }
        }
    }

    timings.bari_viz_ms = t0.elapsed().as_secs_f32() * 1000.0;
}
