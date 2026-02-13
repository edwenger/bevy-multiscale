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

/// Child component for immunity bar visualization
#[derive(Component)]
pub struct ImmunityBar;

/// Child component for shedding bar visualization
#[derive(Component)]
pub struct SheddingBar;
