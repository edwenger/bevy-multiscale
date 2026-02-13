use bevy::prelude::*;

/// Layout indices for live repositioning when bari_radius changes
#[derive(Component)]
pub struct BariLayoutIndex {
    pub hh_idx: usize,
    pub member_idx: usize,
    pub n_hh: usize,
}

/// Marker on bari-level outer sprite (border) — shows aggregate shedding
#[derive(Component)]
pub struct BariBorder;

/// Marker on bari-level inner sprite (fill) — shows aggregate immunity
#[derive(Component)]
pub struct BariFill;
