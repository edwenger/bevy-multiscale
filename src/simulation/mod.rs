mod time;
mod transmission;
mod step;
mod campaign;

pub use time::*;
pub use transmission::*;
pub use step::*;
pub use campaign::*;

use std::collections::VecDeque;
use bevy::prelude::*;
use crate::disease::InfectionStrain;

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

/// Rolling time-series of daily new infections by strain
#[derive(Resource)]
pub struct InfectionTimeSeries {
    pub daily_opv: VecDeque<u32>,
    pub daily_vdpv: VecDeque<u32>,
    pub daily_wpv: VecDeque<u32>,
    pub start_day: u32,
    /// Accumulators for current simulation day
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

/// Accumulate transmission events into pending counts
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

/// Flush pending counts into the rolling VecDeque when simulation day advances
fn flush_daily_counts(
    mut time_series: ResMut<InfectionTimeSeries>,
    sim_time: Res<SimulationTime>,
) {
    if !sim_time.timer.just_finished() {
        return;
    }

    let current_day = sim_time.day;
    let ts = time_series.as_mut();

    // Copy pending values to avoid borrow conflicts
    let p_opv = ts.pending_opv;
    let p_vdpv = ts.pending_vdpv;
    let p_wpv = ts.pending_wpv;

    // Fill any skipped days with zeros, then push current pending
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

        // Trim to rolling window
        if ts.daily_opv.len() > MAX_CHART_DAYS {
            ts.daily_opv.pop_front();
            ts.daily_vdpv.pop_front();
            ts.daily_wpv.pop_front();
            ts.start_day += 1;
        }
    }

    // Reset accumulators
    ts.pending_opv = 0;
    ts.pending_vdpv = 0;
    ts.pending_wpv = 0;
}

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SimulationTime::default())
            .insert_resource(SimulationSpeed::default())
            .insert_resource(TransmissionParams::default())
            .insert_resource(SystemTimings::default())
            .insert_resource(InfectionTimeSeries::default())
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
            ).chain().run_if(in_state(SimState::Running)));
    }
}
