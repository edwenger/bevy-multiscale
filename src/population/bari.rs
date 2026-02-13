use bevy::prelude::*;
use serde::Deserialize;

/// Raw CSV row — just x,y pixel coordinates
#[derive(Debug, Clone, Deserialize)]
struct CsvRow {
    x: f32,
    y: f32,
}

/// A bari position in world space
#[derive(Debug, Clone)]
pub struct BariPosition {
    pub bari_id: usize,
    pub x: f32,
    pub y: f32,
    /// Centered, unscaled pixel x coordinate
    pub pixel_x: f32,
    /// Centered, unscaled pixel y coordinate
    pub pixel_y: f32,
}

/// Resource holding all bari positions from CSV
#[derive(Resource)]
pub struct BariLayout {
    pub positions: Vec<BariPosition>,
    /// Auto-computed scale factor: pixels → world units
    pub world_scale: f32,
    /// Pixels per kilometer (averaged from both axes; physical region ~12x16 km)
    pub pixels_per_km: f32,
    /// Radius of each circular bari in world units
    pub bari_radius: f32,
}

/// Default visual radius of each circular bari in world units
const BARI_RADIUS: f32 = 60.0;
/// Fixed reference radius used to compute world_scale (keeps pixel→world mapping stable)
const SCALE_REFERENCE_RADIUS: f32 = 20.0;
/// Gap between adjacent bari circles after repulsion
const BARI_GAP: f32 = 2.0;

impl BariLayout {
    /// Load bari positions from embedded CSV (WASM-compatible)
    ///
    /// CSV has columns `x,y` in pixel coordinates (image convention: y increases downward).
    /// We center the coordinates and flip y so that the world layout matches spatial intuition.
    /// The world scale is auto-computed from nearest-neighbor distances so baris don't overlap.
    pub fn from_csv() -> Self {
        let csv_data = include_str!("../../assets/bari_positions.csv");
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(csv_data.as_bytes());

        let rows: Vec<CsvRow> = reader
            .deserialize()
            .filter_map(|result| result.ok())
            .collect();

        let bari_radius = BARI_RADIUS;

        if rows.is_empty() {
            return Self { positions: vec![], world_scale: 1.0, pixels_per_km: 1.0, bari_radius };
        }

        // Compute world_scale from nearest-neighbor distances using fixed reference radius
        // so that changing bari_radius only affects visual size, not the world mapping
        let world_scale = compute_world_scale(&rows, SCALE_REFERENCE_RADIUS);

        // Compute center of all positions
        let n = rows.len() as f32;
        let cx = rows.iter().map(|r| r.x).sum::<f32>() / n;
        let cy = rows.iter().map(|r| r.y).sum::<f32>() / n;

        // Compute pixel extent for pixels_per_km
        let min_x = rows.iter().map(|r| r.x).fold(f32::INFINITY, f32::min);
        let max_x = rows.iter().map(|r| r.x).fold(f32::NEG_INFINITY, f32::max);
        let min_y = rows.iter().map(|r| r.y).fold(f32::INFINITY, f32::min);
        let max_y = rows.iter().map(|r| r.y).fold(f32::NEG_INFINITY, f32::max);
        let extent_x = max_x - min_x;
        let extent_y = max_y - min_y;
        // Physical region ~12x16 km, average both axes
        let pixels_per_km = ((extent_x / 12.0) + (extent_y / 16.0)) / 2.0;

        // Center and scale, flipping y (image y-down → world y-up)
        let mut positions: Vec<BariPosition> = rows
            .iter()
            .enumerate()
            .map(|(i, r)| BariPosition {
                bari_id: i,
                x: (r.x - cx) * world_scale,
                y: -(r.y - cy) * world_scale,
                pixel_x: r.x - cx,
                pixel_y: -(r.y - cy),
            })
            .collect();

        // Repel overlapping baris so circles don't overlap
        let min_separation = 2.0 * SCALE_REFERENCE_RADIUS + BARI_GAP;
        separate_baris(&mut positions, min_separation);

        Self { positions, world_scale, pixels_per_km, bari_radius }
    }
}

/// Compute world scale so that circular bari footprints fit within inter-bari spacing.
///
/// Uses the 10th-percentile nearest-neighbor distance as reference.
/// Post-scaling repulsion (separate_baris) handles remaining overlaps.
fn compute_world_scale(rows: &[CsvRow], bari_radius: f32) -> f32 {
    if rows.len() < 2 {
        return 1.0;
    }

    // Compute nearest-neighbor distances in pixel space
    let mut nn_distances: Vec<f32> = Vec::with_capacity(rows.len());
    for i in 0..rows.len() {
        let mut min_dist = f32::INFINITY;
        for j in 0..rows.len() {
            if i == j { continue; }
            let dx = rows[i].x - rows[j].x;
            let dy = rows[i].y - rows[j].y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < min_dist {
                min_dist = dist;
            }
        }
        nn_distances.push(min_dist);
    }

    nn_distances.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // 10th percentile
    let p10_idx = (nn_distances.len() as f32 * 0.10) as usize;
    let p10_nn = nn_distances[p10_idx.min(nn_distances.len() - 1)];

    // Target clearance: diameter of circle
    let target_clearance = 2.0 * bari_radius;

    let scale = if p10_nn > 0.01 {
        target_clearance / p10_nn
    } else {
        1.0
    };

    info!("BariLayout: {} positions, p10 NN = {:.2} px, bari_radius = {:.0}, world_scale = {:.4}",
          rows.len(), p10_nn, bari_radius, scale);

    scale
}

/// Iterative pairwise repulsion to ensure no two bari centers are closer than min_separation.
/// Only modifies world positions (x, y); pixel_x/pixel_y are left unchanged for distance kernels.
fn separate_baris(positions: &mut [BariPosition], min_separation: f32) {
    let n = positions.len();
    if n < 2 {
        return;
    }

    let min_sep_sq = min_separation * min_separation;

    for iteration in 0..100 {
        let mut any_moved = false;

        for i in 0..n {
            for j in (i + 1)..n {
                let dx = positions[j].x - positions[i].x;
                let dy = positions[j].y - positions[i].y;
                let dist_sq = dx * dx + dy * dy;

                if dist_sq < min_sep_sq && dist_sq > 0.001 {
                    let dist = dist_sq.sqrt();
                    let overlap = min_separation - dist;
                    let push = overlap / 2.0 / dist; // normalize
                    let push_x = dx * push;
                    let push_y = dy * push;

                    positions[i].x -= push_x;
                    positions[i].y -= push_y;
                    positions[j].x += push_x;
                    positions[j].y += push_y;
                    any_moved = true;
                }
            }
        }

        if !any_moved {
            info!("separate_baris: converged in {} iterations", iteration + 1);
            return;
        }
    }

    info!("separate_baris: reached max 100 iterations");
}
