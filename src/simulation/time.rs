use bevy::prelude::*;

/// Simulation time tracking
#[derive(Resource)]
pub struct SimulationTime {
    pub day: u32,
    pub timer: Timer,
}

impl Default for SimulationTime {
    fn default() -> Self {
        Self {
            day: 0,
            timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        }
    }
}

impl SimulationTime {
    pub fn reset(&mut self) {
        self.day = 0;
        self.timer.reset();
    }
}

/// Simulation speed multiplier
#[derive(Resource)]
pub struct SimulationSpeed {
    pub multiplier: f32,
}

impl Default for SimulationSpeed {
    fn default() -> Self {
        Self { multiplier: 7.0 }
    }
}

/// Simulation state (running or paused)
#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum SimState {
    #[default]
    Paused,
    Running,
}

/// System to advance simulation time
pub fn advance_simulation_time(
    mut sim_time: ResMut<SimulationTime>,
    time: Res<Time>,
    speed: Res<SimulationSpeed>,
) {
    sim_time.timer.tick(time.delta().mul_f32(speed.multiplier));

    if sim_time.timer.just_finished() {
        for _ in 0..sim_time.timer.times_finished_this_tick() {
            sim_time.day += 1;
        }
    }
}
