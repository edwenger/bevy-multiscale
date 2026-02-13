# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Bevy-based multiscale polio transmission simulation that visualizes disease dynamics across a hierarchical population (individuals → households → neighborhoods → village) at three interactive scales. Built with Bevy 0.13 ECS game engine and bevy_egui for UI. Deploys to web via WASM.

## Build and Run Commands

```bash
# Run the simulation (debug with optimized deps)
cargo run

# Build release binary
cargo build --release

# Build for WASM (used by GitHub Actions for web deployment)
cargo build --target wasm32-unknown-unknown
```

No test suite exists; validation is through interactive demonstration.

## Architecture

### Plugin Structure

Five Bevy plugins with loose coupling via events and shared resources:

- **DiseasePlugin** (`disease/`): Epidemiological modeling - immunity state, infection tracking, disease parameters
- **PopulationPlugin** (`population/`): Demographics and hierarchical structure - individuals, households, neighborhoods
- **SimulationPlugin** (`simulation/`): Time stepping, transmission dynamics, infection campaigns, infection time-series tracking
- **UiPlugin** (`ui/`): Shared visualization utilities - camera zoom/pan, sprite components, color mapping, transmission arcs, tooltips, infection chart
- **ViewPlugin** (`views/`): Three-view architecture with landing page, per-view controls/spawn/viz

### View Architecture

The app uses a state machine (`AppView`) with four states: `Landing`, `Individual`, `Neighborhood`, `Region`. Each view is an independent plugin under `views/` with its own:

- `mod.rs` — Plugin registration, OnEnter/OnExit setup, camera initialization
- `controls.rs` — egui control panel (simulation controls, parameter sliders, seed/OPV buttons)
- `spawn.rs` — Entity spawning (population layout, sprite hierarchy)
- `viz.rs` — Per-frame visual updates (fill colors, border colors, bar heights)

Shared UI code lives in `ui/`:
- `ui/camera.rs` — Zoom (scroll wheel) and pan (middle-click or left-click drag). Uses `wants_pointer_input()` to avoid conflict with egui widget interaction.
- `ui/components.rs` — Marker components (`IndividualBorder`, `IndividualFill`, `IndividualLabel`, `ImmunityBar`, `SheddingBar`)
- `ui/viz.rs` — Color functions: `immunity_to_fill_color` (brown→beige→green gradient), `strain_color` (WPV=red, VDPV=orange, OPV=cyan), `shedding_border_color`
- `ui/chart.rs` — Stacked bar chart of daily new infections by strain (shared by neighborhood and region views)
- `ui/arcs.rs` — Transmission arc visualization
- `ui/tooltip.rs` — Hover tooltips for individual entities

### Visual Encoding

- **Fill color**: Immunity level via brown→beige→green gradient (log2 titer / 10)
- **Border color**: Shedding status by strain — WPV=red, VDPV=orange, OPV=cyan; alpha/thickness scale with viral shedding
- **Two-sprite border technique**: Outer sprite (border) + inner sprite (fill) at slightly higher z-index
- **Immunity bars**: Left side, colored same as fill, height proportional to log10(titer)
- **Shedding bars**: Right side, colored by strain, height proportional to log10(shedding)

### Key Components (ECS)

- `Individual` - Demographics (age, sex)
- `Immunity` - Per-person immunity state with NAb calculations, waning, infection probability
- `Infection` - Active infection (added/removed as disease progresses), tracks viral shedding, strain, and OPV mutation count
- `HouseholdMember`/`NeighborhoodMember` - Entity links for contact tracing
- `IndividualVisual`/`HouseholdVisual`/`NeighborhoodVisual` - Marker components for view-specific queries

### Transmission Model

Three-level contact structure with Poisson-distributed daily contacts:
- **Household** (beta_hh = 3.0): Same household members
- **Neighborhood** (beta_neighborhood = 1.0): Cross-household, same neighborhood
- **Village** (beta_village = 0.5): Between neighborhoods

Transmission probability uses dose-response curve modified by recipient immunity.

### Core Simulation Loop

On each timer tick (when running):
1. `advance_simulation_time()` - Increment day counter
2. `step_disease_state()` - Update immunity waning, viral shedding, OPV→VDPV reversion, clear resolved infections
3. `transmission_system()` - Sample contacts at all levels, emit `TransmissionEvent` for new infections
4. `handle_seed_infection()` - Process manual infection seeding and OPV campaigns

### Important Files

| File | Purpose |
|------|---------|
| `views/mod.rs` | AppView state machine, landing page, cleanup on view exit |
| `views/*/controls.rs` | Per-view egui control panels |
| `views/*/spawn.rs` | Per-view population spawning and layout |
| `views/*/viz.rs` | Per-view sprite update systems |
| `views/region/bari.rs` | BariLayout resource loaded from CSV |
| `ui/camera.rs` | Shared zoom/pan camera system |
| `ui/viz.rs` | Shared color mapping functions |
| `ui/chart.rs` | Shared infection time-series chart |
| `population/init.rs` | Population generation, age-structured households, immunity initialization |
| `simulation/transmission.rs` | Contact sampling and transmission logic |
| `disease/params.rs` | Strain-specific disease parameters (OPV/WPV, serotypes) |
| `disease/immunity.rs` | Immunity calculations (theta NAbs, waning, infection probability) |

### Events

- `TransmissionEvent` - Emitted on successful transmission (used for arc visualization and time-series tracking)
- `SeedInfectionEvent` - Manual infection introduction or OPV campaign (with optional strain, serotype, coverage, age range)
- `ResetPopulationEvent` - Clears and regenerates population (neighborhood/region views)
- `ResetIndividualEvent` - Respawns individual preserving current age/sex (individual view)

### Key Design Decisions

- Individuals use absolute world positions (not children of neighborhoods) because tooltip and arc systems read `GlobalTransform`
- CSV data loaded via `include_str!` for WASM compatibility
- `BariLayout` resource holds spatial positions from CSV, used by region population init and live bari-size controls

## Disease Model

Immunity waning: `I(t) = I_peak * (t/30)^(-rate)` for t ≥ 30 days post-exposure

Three age cohorts at initialization:
- Pre-cessation (<2y): Naive (titer = 1.0)
- Endemic (>12y): High immunity (log2 titer 5-10)
- Transition (2-12y): Vaccine coverage-dependent

OPV → VDPV reversion: OPV strains accumulate mutations over time; at 3 mutations the strain converts to VDPV with increased transmissibility.

Reference implementation: [pybevy-polio](https://github.com/edwenger/pybevy-polio)
