use std::collections::HashMap;
use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_distr::{Poisson, WeightedIndex, Distribution};
use log::info;

use crate::disease::{Immunity, Infection, InfectionStrain, InfectionSerotype, DiseaseParams};
use crate::population::{Individual, HouseholdMember, NeighborhoodMember, Neighborhood};
use super::time::SimulationTime;
use super::SimRng;

/// BariLayout trait — views that provide spatial layout implement this
/// Region view inserts a concrete BariLayout resource; others don't.
use crate::views::region::bari::BariLayout;

/// Transmission parameters (contact rates are means for Poisson draws)
#[derive(Resource)]
pub struct TransmissionParams {
    pub beta_hh: f32,
    pub beta_neighborhood: f32,
    pub beta_village: f32,
    pub fecal_oral_dose: f32,
    pub default_strain: InfectionStrain,
    pub default_serotype: InfectionSerotype,
    pub opv_shedding_reduction: f32,
    pub mean_reversion_days: f32,
    pub village_kernel_km: f32,
}

impl Default for TransmissionParams {
    fn default() -> Self {
        Self {
            beta_hh: 3.0,
            beta_neighborhood: 1.0,
            beta_village: 0.5,
            fecal_oral_dose: 1e-5,
            default_strain: InfectionStrain::WPV,
            default_serotype: InfectionSerotype::Type2,
            opv_shedding_reduction: 0.5,
            mean_reversion_days: 14.0,
            village_kernel_km: 2.0,
        }
    }
}

/// Level at which transmission occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmissionLevel {
    Household,
    Neighborhood,
    Village,
}

/// Event emitted when transmission occurs
#[derive(Event)]
pub struct TransmissionEvent {
    pub source: Entity,
    pub target: Entity,
    pub level: TransmissionLevel,
    pub strain: InfectionStrain,
    pub time: f32,
}

/// Main transmission system — branches on presence of BariLayout
pub fn transmission_system(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    params: Res<TransmissionParams>,
    disease_params: Res<DiseaseParams>,
    bari_layout: Option<Res<BariLayout>>,
    shedders: Query<(Entity, &Individual, &Infection, &HouseholdMember, &NeighborhoodMember)>,
    mut susceptibles: Query<
        (Entity, &Individual, &mut Immunity, &HouseholdMember, &NeighborhoodMember),
        Without<Infection>
    >,
    neighborhoods_q: Query<(Entity, &Neighborhood)>,
    mut tx_events: EventWriter<TransmissionEvent>,
    mut timings: ResMut<super::SystemTimings>,
    mut sim_rng: ResMut<SimRng>,
) {
    if !sim_time.timer.just_finished() {
        return;
    }

    let t0 = bevy::utils::Instant::now();

    let rng = &mut sim_rng.0;

    let nbhd_to_bari: HashMap<Entity, usize> = neighborhoods_q.iter()
        .map(|(e, nbhd)| (e, nbhd.index))
        .collect();
    let mut susceptibles_by_hh: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let mut susceptibles_by_nbhd: HashMap<Entity, Vec<(Entity, Entity)>> = HashMap::new();
    let mut susceptibles_by_bari: HashMap<usize, Vec<Entity>> = HashMap::new();
    let mut susceptible_count = 0usize;

    for (entity, _ind, _, hh, nbhd) in susceptibles.iter() {
        susceptible_count += 1;
        susceptibles_by_hh.entry(hh.household_id).or_default().push(entity);
        susceptibles_by_nbhd.entry(nbhd.neighborhood_id).or_default().push((entity, hh.household_id));
        if let Some(&bari_idx) = nbhd_to_bari.get(&nbhd.neighborhood_id) {
            susceptibles_by_bari.entry(bari_idx).or_default().push(entity);
        }
    }

    let kernel_px = bari_layout.as_ref().map(|bl| params.village_kernel_km * bl.pixels_per_km);

    let mut new_infections: Vec<(Entity, f32, InfectionStrain, InfectionSerotype, u8)> = Vec::new();
    let mut tx_counts = (0usize, 0usize, 0usize);

    let shedder_count = shedders.iter().count();
    for (src_entity, _src_ind, infection, src_hh, src_nbhd) in shedders.iter() {
        let shedding_mult = match infection.strain {
            InfectionStrain::OPV => params.opv_shedding_reduction,
            _ => 1.0,
        };
        let dose = infection.viral_shedding * params.fecal_oral_dose * shedding_mult;

        // Household transmission
        if let Some(hh_pool) = susceptibles_by_hh.get(&src_hh.household_id) {
            let sampled = sample_contacts(hh_pool, params.beta_hh, rng);
            for &target_entity in sampled {
                if let Ok((_, _, immunity, _, _)) = susceptibles.get(target_entity) {
                    let p_inf = immunity.calculate_infection_probability(
                        dose, infection.strain, infection.serotype, &disease_params,
                    );
                    if rng.gen::<f32>() < p_inf {
                        new_infections.push((target_entity, sim_time.day as f32, infection.strain, infection.serotype, infection.mutations));
                        tx_events.send(TransmissionEvent {
                            source: src_entity, target: target_entity,
                            level: TransmissionLevel::Household,
                            strain: infection.strain, time: sim_time.day as f32,
                        });
                        tx_counts.0 += 1;
                    }
                }
            }
        }

        // Neighborhood transmission
        if let Some(nbhd_pool) = susceptibles_by_nbhd.get(&src_nbhd.neighborhood_id) {
            let nbhd_contacts: Vec<Entity> = nbhd_pool.iter()
                .filter(|(_, hh_id)| *hh_id != src_hh.household_id)
                .map(|(entity, _)| *entity)
                .collect();
            let sampled = sample_contacts(&nbhd_contacts, params.beta_neighborhood, rng);
            for &target_entity in sampled {
                if let Ok((_, _, immunity, _, _)) = susceptibles.get(target_entity) {
                    let p_inf = immunity.calculate_infection_probability(
                        dose, infection.strain, infection.serotype, &disease_params,
                    );
                    if rng.gen::<f32>() < p_inf {
                        new_infections.push((target_entity, sim_time.day as f32, infection.strain, infection.serotype, infection.mutations));
                        tx_events.send(TransmissionEvent {
                            source: src_entity, target: target_entity,
                            level: TransmissionLevel::Neighborhood,
                            strain: infection.strain, time: sim_time.day as f32,
                        });
                        tx_counts.1 += 1;
                    }
                }
            }
        }

        // Village transmission
        if let Some(&src_bari_idx) = nbhd_to_bari.get(&src_nbhd.neighborhood_id) {
            if let (Some(ref bl), Some(k_px)) = (&bari_layout, kernel_px) {
                // Distance-weighted village transmission (Region view)
                let src_pos = &bl.positions[src_bari_idx];
                let mut bari_weights: Vec<(usize, f64)> = Vec::new();
                for (&bari_idx, sus) in &susceptibles_by_bari {
                    if bari_idx == src_bari_idx || sus.is_empty() { continue; }
                    let other = &bl.positions[bari_idx];
                    let dx = src_pos.pixel_x - other.pixel_x;
                    let dy = src_pos.pixel_y - other.pixel_y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let weight = (-(dist as f64) / k_px as f64).exp() * sus.len() as f64;
                    if weight > 1e-10 {
                        bari_weights.push((bari_idx, weight));
                    }
                }
                // Sort for deterministic sampling order (HashMap iteration is random)
                bari_weights.sort_by_key(|(idx, _)| *idx);

                if !bari_weights.is_empty() {
                    let n_village = if params.beta_village > 0.0 {
                        let poisson = Poisson::new(params.beta_village as f64).unwrap();
                        poisson.sample(rng) as usize
                    } else { 0 };

                    if n_village > 0 {
                        let weights: Vec<f64> = bari_weights.iter().map(|(_, w)| *w).collect();
                        if let Ok(dist) = WeightedIndex::new(&weights) {
                            for _ in 0..n_village {
                                let chosen_idx = dist.sample(&mut *rng);
                                let target_bari = bari_weights[chosen_idx].0;
                                let pool = &susceptibles_by_bari[&target_bari];
                                let &target_entity = pool.choose(&mut *rng).unwrap();

                                if let Ok((_, _, immunity, _, _)) = susceptibles.get(target_entity) {
                                    let p_inf = immunity.calculate_infection_probability(
                                        dose, infection.strain, infection.serotype, &disease_params,
                                    );
                                    if rng.gen::<f32>() < p_inf {
                                        new_infections.push((target_entity, sim_time.day as f32, infection.strain, infection.serotype, infection.mutations));
                                        tx_events.send(TransmissionEvent {
                                            source: src_entity, target: target_entity,
                                            level: TransmissionLevel::Village,
                                            strain: infection.strain, time: sim_time.day as f32,
                                        });
                                        tx_counts.2 += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Uniform village transmission (Neighborhood view — no BariLayout)
                // Pick from all susceptibles NOT in the same neighborhood
                let mut other_sus: Vec<Entity> = Vec::new();
                // Collect bari indices in sorted order for deterministic sampling
                let mut other_bari_idxs: Vec<usize> = susceptibles_by_bari.keys()
                    .filter(|&&idx| idx != src_bari_idx)
                    .cloned()
                    .collect();
                other_bari_idxs.sort();
                for bari_idx in other_bari_idxs {
                    other_sus.extend(&susceptibles_by_bari[&bari_idx]);
                }

                if !other_sus.is_empty() {
                    let sampled = sample_contacts(&other_sus, params.beta_village, rng);
                    for &target_entity in sampled {
                        if let Ok((_, _, immunity, _, _)) = susceptibles.get(target_entity) {
                            let p_inf = immunity.calculate_infection_probability(
                                dose, infection.strain, infection.serotype, &disease_params,
                            );
                            if rng.gen::<f32>() < p_inf {
                                new_infections.push((target_entity, sim_time.day as f32, infection.strain, infection.serotype, infection.mutations));
                                tx_events.send(TransmissionEvent {
                                    source: src_entity, target: target_entity,
                                    level: TransmissionLevel::Village,
                                    strain: infection.strain, time: sim_time.day as f32,
                                });
                                tx_counts.2 += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let total_tx = tx_counts.0 + tx_counts.1 + tx_counts.2;
    if shedder_count > 0 || total_tx > 0 {
        info!("Day {}: {} shedders, {} susceptibles, {} new infections (HH:{}, Nbhd:{}, Village:{})",
              sim_time.day, shedder_count, susceptible_count, total_tx,
              tx_counts.0, tx_counts.1, tx_counts.2);
    }

    // Apply new infections
    for (entity, sim_day, strain, serotype, src_mutations) in new_infections {
        if let Ok((_, _ind, mut immunity, _, _)) = susceptibles.get_mut(entity) {
            let mut inf = match strain {
                InfectionStrain::OPV => {
                    Infection::new_opv(serotype, src_mutations, params.mean_reversion_days, rng)
                }
                InfectionStrain::VDPV => Infection::new(InfectionStrain::VDPV, serotype),
                InfectionStrain::WPV => Infection::new(InfectionStrain::WPV, serotype),
            };
            immunity.set_infection_prognoses(&mut inf, sim_day, &disease_params, rng);
            commands.entity(entity).insert(inf);
        }
    }

    timings.transmission_ms = t0.elapsed().as_secs_f32() * 1000.0;
    timings.shedder_count = shedder_count;
}

/// Sample contacts with Poisson-distributed count
fn sample_contacts<'a, T>(
    contacts: &'a [T],
    mean: f32,
    rng: &mut impl Rng,
) -> Vec<&'a T> {
    if contacts.is_empty() || mean <= 0.0 {
        return Vec::new();
    }

    let n = if mean > 0.0 {
        let poisson = Poisson::new(mean as f64).unwrap();
        poisson.sample(rng) as usize
    } else {
        0
    };

    if n == 0 {
        return Vec::new();
    }

    let n = n.min(contacts.len());
    contacts.choose_multiple(rng, n).collect()
}
