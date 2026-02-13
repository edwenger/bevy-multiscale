# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Bevy-based multiscale polio transmission simulation that visualizes disease dynamics across a hierarchical population (individuals → households → neighborhoods → village). Built with Bevy 0.13 ECS game engine and bevy_egui for UI.

## Build and Run Commands

```bash
# Run the simulation (debug with optimized deps)
cargo run

# Build release binary
cargo build --release
```

No test suite exists; validation is through interactive demonstration.

## Architecture

### Plugin Structure

Four independent Bevy plugins with loose coupling via events:

- **DiseasePlugin** (`disease/`): Epidemiological modeling - immunity state, infection tracking, disease parameters
- **PopulationPlugin** (`population/`): Demographics and hierarchical structure - individuals, households, neighborhoods
- **SimulationPlugin** (`simulation/`): Time stepping, transmission dynamics, infection campaigns
- **UiPlugin** (`ui/`): egui control panel, sprite visualization, transmission arcs, tooltips

### Key Components (ECS)

- `Individual` - Demographics (age, sex)
- `Immunity` - Per-person immunity state with NAb calculations, waning, infection probability
- `Infection` - Active infection (added/removed as disease progresses), tracks viral shedding and strain
- `HouseholdMember`/`NeighborhoodMember` - Entity links for contact tracing

### Transmission Model

Three-level contact structure with Poisson-distributed daily contacts:
- **Household** (beta_hh = 3.0): Same household members
- **Neighborhood** (beta_neighborhood = 1.0): Cross-household, same neighborhood
- **Village** (beta_village = 0.5): Between neighborhoods

Transmission probability uses dose-response curve modified by recipient immunity.

### Core Simulation Loop

On each timer tick (when running):
1. `advance_simulation_time()` - Increment day counter
2. `step_disease_state()` - Update immunity waning, viral shedding, clear resolved infections
3. `transmission_system()` - Sample contacts at all levels, emit `TransmissionEvent` for new infections
4. `handle_seed_infection()` - Process manual infection seeding

### Important Files

| File | Purpose |
|------|---------|
| `population/init.rs` | Population generation, age-structured households, immunity initialization |
| `simulation/transmission.rs` | Contact sampling and transmission logic |
| `disease/params.rs` | Strain-specific disease parameters (OPV/WPV, serotypes) |
| `disease/immunity.rs` | Immunity calculations (theta NAbs, waning, infection probability) |
| `ui/controls.rs` | egui parameter panel and simulation controls |

### Events

- `TransmissionEvent` - Emitted on successful transmission (used for arc visualization)
- `SeedInfectionEvent` - Manual infection introduction
- `ResetPopulationEvent` - Clears and regenerates population

## Disease Model

Immunity waning: `I(t) = I_peak * (t/30)^(-rate)` for t ≥ 30 days post-exposure

Three age cohorts at initialization:
- Pre-cessation (<2y): Naive (titer = 1.0)
- Endemic (>12y): High immunity (log2 titer 5-10)
- Transition (2-12y): Vaccine coverage-dependent

Reference implementation: [pybevy-polio](https://github.com/edwenger/pybevy-polio)
