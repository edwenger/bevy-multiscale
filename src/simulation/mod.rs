mod time;
mod transmission;
mod step;
mod campaign;

pub use time::*;
pub use transmission::*;
pub use step::*;
pub use campaign::*;

use std::collections::VecDeque;
use bevy::app::AppExit;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use crate::disease::{Immunity, Infection, InfectionStrain};
use crate::population::Individual;

#[derive(Resource, Default)]
pub struct SystemTimings {
    pub transmission_ms: f32,
    pub disease_step_ms: f32,
    pub individual_viz_ms: f32,
    pub bari_viz_ms: f32,
    pub arc_update_ms: f32,
    pub arc_count: usize,
    pub shedder_count: usize,
}

/// Seeded RNG resource for reproducible simulations
#[derive(Resource)]
pub struct SimRng(pub StdRng);

impl Default for SimRng {
    fn default() -> Self {
        Self(StdRng::from_entropy())
    }
}

/// Marker resource for headless mode (no rendering)
#[derive(Resource)]
pub struct HeadlessMode;

/// Configuration for headless simulation
#[derive(Resource)]
pub struct HeadlessConfig {
    pub end_time: u32,
    pub output_path: String,
}

/// A single transmission record for CSV output
pub struct TransmissionRecord {
    pub day: u32,
    pub source_id: u32,
    pub target_id: u32,
    pub source_age: f32,
    pub target_age: f32,
    pub source_log2_titer: f32,
    pub target_log2_titer: f32,
    pub level: String,
    pub strain: String,
}

/// Log of all transmission events (headless mode)
#[derive(Resource, Default)]
pub struct TransmissionLog {
    pub records: Vec<TransmissionRecord>,
}

/// System to log transmission events to TransmissionLog (headless only)
fn log_transmissions(
    mut events: EventReader<TransmissionEvent>,
    sim_time: Res<SimulationTime>,
    individuals: Query<(&Individual, &Immunity)>,
    log: Option<ResMut<TransmissionLog>>,
) {
    let Some(mut log) = log else { return; };
    if !sim_time.timer.just_finished() {
        return;
    }

    for ev in events.read() {
        let (source_age, source_log2_titer) = individuals.get(ev.source)
            .map(|(i, imm)| (i.age, imm.prechallenge_immunity.log2()))
            .unwrap_or((0.0, 0.0));
        let (target_age, target_log2_titer) = individuals.get(ev.target)
            .map(|(i, imm)| (i.age, imm.prechallenge_immunity.log2()))
            .unwrap_or((0.0, 0.0));
        let level = match ev.level {
            TransmissionLevel::Household => "household",
            TransmissionLevel::Neighborhood => "neighborhood",
            TransmissionLevel::Village => "village",
        };
        log.records.push(TransmissionRecord {
            day: sim_time.day,
            source_id: ev.source.index(),
            target_id: ev.target.index(),
            source_age,
            target_age,
            source_log2_titer,
            target_log2_titer,
            level: level.to_string(),
            strain: format!("{:?}", ev.strain),
        });
    }
}

/// System to check stop conditions in headless mode
fn check_stop_condition(
    sim_time: Res<SimulationTime>,
    config: Option<Res<HeadlessConfig>>,
    log: Option<Res<TransmissionLog>>,
    infections: Query<&Infection>,
    mut exit: EventWriter<AppExit>,
) {
    let (Some(config), Some(log)) = (config, log) else { return; };
    if !sim_time.timer.just_finished() {
        return;
    }

    let should_stop = sim_time.day >= config.end_time
        || (sim_time.day > 30 && infections.is_empty());

    if should_stop {
        // Write CSV output
        let path = &config.output_path;
        if let Ok(mut wtr) = csv::Writer::from_path(path) {
            let _ = wtr.write_record(&["day", "source_id", "target_id", "source_age", "target_age", "source_log2_titer", "target_log2_titer", "level", "strain"]);
            for rec in &log.records {
                let _ = wtr.write_record(&[
                    rec.day.to_string(),
                    rec.source_id.to_string(),
                    rec.target_id.to_string(),
                    format!("{:.1}", rec.source_age),
                    format!("{:.1}", rec.target_age),
                    format!("{:.2}", rec.source_log2_titer),
                    format!("{:.2}", rec.target_log2_titer),
                    rec.level.clone(),
                    rec.strain.clone(),
                ]);
            }
            let _ = wtr.flush();
            eprintln!("Wrote {} transmission records to {}", log.records.len(), path);
        } else {
            eprintln!("Failed to open output file: {}", path);
        }

        exit.send(AppExit);
    }
}

/// Rolling time-series of daily new infections by strain
#[derive(Resource)]
pub struct InfectionTimeSeries {
    pub daily_opv: VecDeque<u32>,
    pub daily_vdpv: VecDeque<u32>,
    pub daily_wpv: VecDeque<u32>,
    pub start_day: u32,
    pub pending_opv: u32,
    pub pending_vdpv: u32,
    pub pending_wpv: u32,
    pub last_flushed_day: u32,
}

impl Default for InfectionTimeSeries {
    fn default() -> Self {
        Self {
            daily_opv: VecDeque::new(),
            daily_vdpv: VecDeque::new(),
            daily_wpv: VecDeque::new(),
            start_day: 0,
            pending_opv: 0,
            pending_vdpv: 0,
            pending_wpv: 0,
            last_flushed_day: 0,
        }
    }
}

const MAX_CHART_DAYS: usize = 365;

fn record_infections(
    mut time_series: ResMut<InfectionTimeSeries>,
    mut events: EventReader<TransmissionEvent>,
) {
    for ev in events.read() {
        match ev.strain {
            InfectionStrain::OPV => time_series.pending_opv += 1,
            InfectionStrain::VDPV => time_series.pending_vdpv += 1,
            InfectionStrain::WPV => time_series.pending_wpv += 1,
        }
    }
}

fn flush_daily_counts(
    mut time_series: ResMut<InfectionTimeSeries>,
    sim_time: Res<SimulationTime>,
) {
    if !sim_time.timer.just_finished() {
        return;
    }

    let current_day = sim_time.day;
    let ts = time_series.as_mut();

    let p_opv = ts.pending_opv;
    let p_vdpv = ts.pending_vdpv;
    let p_wpv = ts.pending_wpv;

    while ts.last_flushed_day < current_day {
        let is_current = ts.last_flushed_day == current_day - 1;
        if is_current {
            ts.daily_opv.push_back(p_opv);
            ts.daily_vdpv.push_back(p_vdpv);
            ts.daily_wpv.push_back(p_wpv);
        } else {
            ts.daily_opv.push_back(0);
            ts.daily_vdpv.push_back(0);
            ts.daily_wpv.push_back(0);
        }
        ts.last_flushed_day += 1;

        if ts.daily_opv.len() > MAX_CHART_DAYS {
            ts.daily_opv.pop_front();
            ts.daily_vdpv.pop_front();
            ts.daily_wpv.pop_front();
            ts.start_day += 1;
        }
    }

    ts.pending_opv = 0;
    ts.pending_vdpv = 0;
    ts.pending_wpv = 0;
}

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationTime::default())
            .insert_resource(SimulationSpeed::default())
            .init_resource::<TransmissionParams>()
            .insert_resource(SystemTimings::default())
            .insert_resource(InfectionTimeSeries::default())
            .init_resource::<SimRng>()
            .init_state::<SimState>()
            .add_event::<TransmissionEvent>()
            .add_event::<SeedInfectionEvent>()
            .add_systems(Update, (
                advance_simulation_time,
                step_disease_state,
                transmission_system,
                record_infections,
                flush_daily_counts,
                handle_seed_infection,
                log_transmissions,
                check_stop_condition,
            ).chain().run_if(
                in_state(SimState::Running).or_else(resource_exists::<HeadlessMode>)
            ));
    }
}
