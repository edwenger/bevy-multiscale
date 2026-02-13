use bevy::prelude::*;

use crate::disease::{Infection, InfectionStrain};

/// 3-stop gradient: brown (t=0) -> beige (t=0.5) -> green (t=1.0)
pub fn gradient_brown_beige_green(t: f32) -> Color {
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

/// Map current immunity titer to fill color using log2/10 scaling
pub fn immunity_to_fill_color(titer: f32) -> Color {
    let t = (titer.log2() / 10.0).clamp(0.0, 1.0);
    gradient_brown_beige_green(t)
}

/// Map infection strain to a base color
pub fn strain_color(strain: InfectionStrain) -> Color {
    match strain {
        InfectionStrain::WPV  => Color::rgb(0.9, 0.15, 0.15),
        InfectionStrain::VDPV => Color::rgb(1.0, 0.6, 0.0),
        InfectionStrain::OPV  => Color::rgb(0.0, 0.85, 0.85),
    }
}

/// Compute border color and outer size for an individual given optional infection.
/// Returns (color, outer_sprite_size) for the border sprite.
pub fn shedding_border_color(infection: Option<&Infection>, default_size: f32) -> (Color, f32) {
    if let Some(inf) = infection {
        let base_color = strain_color(inf.strain);
        let log_shed = inf.viral_shedding.log10().clamp(2.0, 8.0);
        let t = (log_shed - 2.0) / 6.0;
        let alpha = 0.3 + 0.7 * t;
        let thickness = 0.5 + 1.0 * t;
        let outer_size = default_size + 2.0 * thickness;
        (base_color.with_a(alpha), outer_size)
    } else {
        (Color::rgba(0.3, 0.3, 0.3, 0.5), default_size)
    }
}
