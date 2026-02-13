use bevy::prelude::*;

/// Marker on outer sprite (border) — shows shedding status
#[derive(Component)]
pub struct IndividualBorder;

/// Marker on inner sprite (fill) — shows immunity level
#[derive(Component)]
pub struct IndividualFill;

/// Marker on age/sex text label
#[derive(Component)]
pub struct IndividualLabel;

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
