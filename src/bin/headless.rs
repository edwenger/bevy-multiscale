use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use clap::Parser;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Deserialize;
use std::time::Duration;

use bevy_multiscale::disease::{Immunity, InfectionStrain, InfectionSerotype};
use bevy_multiscale::population::{
    Individual, HouseholdMember, NeighborhoodMember, Household, Neighborhood,
    PopulationConfig, generate_household_members, calculate_initial_immunity,
};
use bevy_multiscale::simulation::{
    HeadlessMode, HeadlessConfig, SimRng, TransmissionParams, TransmissionLog,
    SeedInfectionEvent,
};

#[derive(Parser)]
#[command(name = "headless", about = "Headless polio transmission simulation")]
struct Cli {
    /// Path to YAML configuration file
    #[arg(short, long)]
    config: String,

    /// Path for CSV output
    #[arg(short, long, default_value = "transmissions.csv")]
    output: String,
}

#[derive(Deserialize)]
struct Config {
    // Population
    num_neighborhoods: usize,
    households_per_neighborhood: usize,
    lifetime_births: f32,
    time_since_cessation: f32,
    elimination_duration: f32,
    vaccine_coverage: f32,

    // Transmission
    beta_hh: f32,
    beta_neighborhood: f32,
    beta_village: f32,
    fecal_oral_dose: f32,

    // Initial conditions
    initial_wpv_cases: usize,
    #[serde(default)]
    under5_opv_coverage: f32,

    // Simulation
    simulation_end_time: u32,
    random_seed: u64,
}

fn main() {
    let cli = Cli::parse();

    let yaml_str = std::fs::read_to_string(&cli.config)
        .unwrap_or_else(|e| panic!("Failed to read config file '{}': {}", cli.config, e));
    let config: Config = serde_yaml::from_str(&yaml_str)
        .unwrap_or_else(|e| panic!("Failed to parse config: {}", e));

    let pop_config = PopulationConfig {
        num_neighborhoods: config.num_neighborhoods,
        households_per_neighborhood: config.households_per_neighborhood,
        lifetime_births: config.lifetime_births,
        time_since_cessation: config.time_since_cessation,
        elimination_duration: config.elimination_duration,
        vaccine_coverage: config.vaccine_coverage,
    };

    let tx_params = TransmissionParams {
        beta_hh: config.beta_hh,
        beta_neighborhood: config.beta_neighborhood,
        beta_village: config.beta_village,
        fecal_oral_dose: config.fecal_oral_dose,
        ..Default::default()
    };

    let headless_config = HeadlessConfig {
        end_time: config.simulation_end_time,
        output_path: cli.output,
    };

    let initial_wpv_cases = config.initial_wpv_cases;
    let under5_opv_coverage = config.under5_opv_coverage;

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs(0),
        )))
        .insert_resource(HeadlessMode)
        .insert_resource(SimRng(StdRng::seed_from_u64(config.random_seed)))
        .insert_resource(headless_config)
        .insert_resource(pop_config)
        .insert_resource(tx_params)
        .insert_resource(TransmissionLog::default())
        .add_plugins(bevy_multiscale::disease::DiseasePlugin)
        .add_plugins(bevy_multiscale::population::PopulationPlugin)
        .add_plugins(bevy_multiscale::simulation::SimulationPlugin)
        .add_systems(Startup, move |
            mut commands: Commands,
            config: Res<PopulationConfig>,
            mut sim_rng: ResMut<SimRng>,
            mut seed_events: EventWriter<SeedInfectionEvent>,
        | {
            spawn_headless_population(&mut commands, &config, &mut sim_rng.0);

            // Seed initial WPV infections
            if initial_wpv_cases > 0 {
                seed_events.send(SeedInfectionEvent {
                    count: initial_wpv_cases,
                    strain: Some(InfectionStrain::WPV),
                    serotype: Some(InfectionSerotype::Type2),
                    ..Default::default()
                });
            }

            // Seed OPV campaign if coverage > 0
            if under5_opv_coverage > 0.0 {
                seed_events.send(SeedInfectionEvent {
                    strain: Some(InfectionStrain::OPV),
                    serotype: Some(InfectionSerotype::Type2),
                    coverage: Some(under5_opv_coverage),
                    max_age: 5.0,
                    ..Default::default()
                });
            }
        })
        .run();
}

/// Spawn population entities without any visual components
fn spawn_headless_population(
    commands: &mut Commands,
    config: &PopulationConfig,
    rng: &mut impl rand::Rng,
) {
    let mut total_individuals = 0;

    for nbhd_idx in 0..config.num_neighborhoods {
        let neighborhood_entity = commands.spawn(
            Neighborhood::new(nbhd_idx),
        ).id();

        for _ in 0..config.households_per_neighborhood {
            let members = generate_household_members(config, rng);

            let household_entity = commands.spawn(
                Household::new(neighborhood_entity),
            ).id();

            for (age, sex) in &members {
                let initial_immunity = calculate_initial_immunity(
                    *age, config.time_since_cessation, config.elimination_duration,
                    config.vaccine_coverage, rng,
                );

                commands.spawn((
                    Individual::new(*age, *sex, 0.0),
                    Immunity::with_titer(initial_immunity),
                    HouseholdMember { household_id: household_entity },
                    NeighborhoodMember { neighborhood_id: neighborhood_entity },
                ));

                total_individuals += 1;
            }

            commands.entity(household_entity).insert(Household {
                neighborhood_id: neighborhood_entity,
                member_count: members.len(),
            });
        }

        commands.entity(neighborhood_entity).insert(Neighborhood {
            household_count: config.households_per_neighborhood,
            index: nbhd_idx,
        });
    }

    eprintln!("Spawned {} individuals across {} neighborhoods",
        total_individuals, config.num_neighborhoods);
}
