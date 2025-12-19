use bevy::prelude::*;
use rand::seq::SliceRandom;
use rand::Rng;
use rand_distr::{Poisson, Distribution};
use log::info;

use crate::disease::{Immunity, Infection, InfectionStrain, InfectionSerotype, DiseaseParams};
use crate::population::{Individual, HouseholdMember, NeighborhoodMember};
use super::time::SimulationTime;

/// Transmission parameters (contact rates are means for Poisson draws)
#[derive(Resource)]
pub struct TransmissionParams {
    pub beta_hh: f32,
    pub beta_neighborhood: f32,
    pub beta_village: f32,
    pub fecal_oral_dose: f32,
    pub default_strain: InfectionStrain,
    pub default_serotype: InfectionSerotype,
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
    pub time: f32,
}

/// Main transmission system
pub fn transmission_system(
    mut commands: Commands,
    sim_time: Res<SimulationTime>,
    params: Res<TransmissionParams>,
    disease_params: Res<DiseaseParams>,
    shedders: Query<(Entity, &Individual, &Infection, &HouseholdMember, &NeighborhoodMember)>,
    mut susceptibles: Query<
        (Entity, &Individual, &mut Immunity, &HouseholdMember, &NeighborhoodMember),
        Without<Infection>
    >,
    mut tx_events: EventWriter<TransmissionEvent>,
) {
    // Only run on timer tick
    if !sim_time.timer.just_finished() {
        return;
    }

    let mut rng = rand::thread_rng();

    // Collect all susceptible data to avoid borrow conflicts
    let susceptible_data: Vec<_> = susceptibles.iter()
        .map(|(e, ind, _, hh, nbhd)| (e, ind.age, hh.household_id, nbhd.neighborhood_id))
        .collect();

    // Track infections to apply after iteration
    let mut new_infections: Vec<(Entity, f32)> = Vec::new();
    let mut tx_counts = (0usize, 0usize, 0usize); // (hh, nbhd, village)

    let shedder_count = shedders.iter().count();
    for (src_entity, _src_ind, infection, src_hh, src_nbhd) in shedders.iter() {
        let dose = infection.viral_shedding * params.fecal_oral_dose;

        // Household transmission
        let hh_contacts: Vec<_> = susceptible_data.iter()
            .filter(|(_, _, hh_id, _)| *hh_id == src_hh.household_id)
            .collect();

        if !hh_contacts.is_empty() {
            let sampled = sample_contacts(&hh_contacts, params.beta_hh, &mut rng);
            for &&(target_entity, _, _, _) in sampled {
                if let Ok((_, _, immunity, _, _)) = susceptibles.get(target_entity) {
                    let p_inf = immunity.calculate_infection_probability(
                        dose,
                        params.default_strain,
                        params.default_serotype,
                        &disease_params,
                    );
                    if rng.gen::<f32>() < p_inf {
                        new_infections.push((target_entity, sim_time.day as f32));
                        tx_events.send(TransmissionEvent {
                            source: src_entity,
                            target: target_entity,
                            level: TransmissionLevel::Household,
                            time: sim_time.day as f32,
                        });
                        tx_counts.0 += 1;
                    }
                }
            }
        }

        // Neighborhood transmission (different households, same neighborhood)
        let nbhd_contacts: Vec<_> = susceptible_data.iter()
            .filter(|(_, _, hh_id, nbhd_id)|
                *nbhd_id == src_nbhd.neighborhood_id && *hh_id != src_hh.household_id)
            .collect();

        if !nbhd_contacts.is_empty() {
            let sampled = sample_contacts(&nbhd_contacts, params.beta_neighborhood, &mut rng);
            for &&(target_entity, _, _, _) in sampled {
                if let Ok((_, _, immunity, _, _)) = susceptibles.get(target_entity) {
                    let p_inf = immunity.calculate_infection_probability(
                        dose,
                        params.default_strain,
                        params.default_serotype,
                        &disease_params,
                    );
                    if rng.gen::<f32>() < p_inf {
                        new_infections.push((target_entity, sim_time.day as f32));
                        tx_events.send(TransmissionEvent {
                            source: src_entity,
                            target: target_entity,
                            level: TransmissionLevel::Neighborhood,
                            time: sim_time.day as f32,
                        });
                        tx_counts.1 += 1;
                    }
                }
            }
        }

        // Village transmission (different neighborhoods)
        let village_contacts: Vec<_> = susceptible_data.iter()
            .filter(|(_, _, _, nbhd_id)| *nbhd_id != src_nbhd.neighborhood_id)
            .collect();

        if !village_contacts.is_empty() {
            let sampled = sample_contacts(&village_contacts, params.beta_village, &mut rng);
            for &&(target_entity, _, _, _) in sampled {
                if let Ok((_, _, immunity, _, _)) = susceptibles.get(target_entity) {
                    let p_inf = immunity.calculate_infection_probability(
                        dose,
                        params.default_strain,
                        params.default_serotype,
                        &disease_params,
                    );
                    if rng.gen::<f32>() < p_inf {
                        new_infections.push((target_entity, sim_time.day as f32));
                        tx_events.send(TransmissionEvent {
                            source: src_entity,
                            target: target_entity,
                            level: TransmissionLevel::Village,
                            time: sim_time.day as f32,
                        });
                        tx_counts.2 += 1;
                    }
                }
            }
        }
    }

    // Log transmission summary
    let total_tx = tx_counts.0 + tx_counts.1 + tx_counts.2;
    if shedder_count > 0 || total_tx > 0 {
        info!("Day {}: {} shedders, {} susceptibles, {} new infections (HH:{}, Nbhd:{}, Village:{})",
              sim_time.day, shedder_count, susceptible_data.len(), total_tx,
              tx_counts.0, tx_counts.1, tx_counts.2);
    }

    // Apply new infections
    for (entity, sim_day) in new_infections {
        if let Ok((_, _ind, mut immunity, _, _)) = susceptibles.get_mut(entity) {
            let mut infection = Infection::new(params.default_strain, params.default_serotype);
            immunity.set_infection_prognoses(&mut infection, sim_day, &disease_params);
            commands.entity(entity).insert(infection);
        }
    }
}

/// Sample contacts with Poisson-distributed count
///
/// Draws n ~ Poisson(mean) contacts, then samples without replacement.
/// If n exceeds available contacts, truncates to pool size.
///
/// Note: Self-selection is prevented upstream - shedders have Infection component,
/// and the contact pool is built from susceptibles query (Without<Infection>).
///
/// TODO: Add age-assortivity weighting matrix for more realistic contact patterns.
/// Currently samples uniformly from available contacts.
fn sample_contacts<'a, T>(
    contacts: &'a [T],
    mean: f32,
    rng: &mut impl Rng,
) -> Vec<&'a T> {
    if contacts.is_empty() || mean <= 0.0 {
        return Vec::new();
    }

    // Draw number of contacts from Poisson distribution
    let n = if mean > 0.0 {
        let poisson = Poisson::new(mean as f64).unwrap();
        poisson.sample(rng) as usize
    } else {
        0
    };

    if n == 0 {
        return Vec::new();
    }

    // Truncate to available pool size (can't contact more people than exist)
    let n = n.min(contacts.len());
    contacts.choose_multiple(rng, n).collect()
}
