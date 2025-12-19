# bevy-multiscale

Bevy-based multiscale polio transmission simulation demo.

Visualizes disease transmission dynamics across a hierarchical population structure (individuals → households → neighborhoods → village) with interactive parameter controls.

![App Demo](docs/app_demo.mp4)

## Running

```bash
cargo run
```

## Controls

- **Start/Pause**: Toggle simulation
- **Reset**: Regenerate population with current parameters
- **Seed 1/5/10**: Introduce infections
- **Speed slider**: Adjust simulation speed (0.5x - 30x)

Hover over individuals for tooltips showing age, immunity, and infection status.

## Features

- Hierarchical population with realistic household composition
- Three-level transmission (household, neighborhood, village)
- Immunity initialization based on cessation timing and vaccine coverage
- Real-time visualization of immunity bars (blue) and shedding bars (red)
- Transmission arcs showing infection spread

## Related

Disease model and parameters adapted from [pybevy-polio](https://github.com/edwenger/pybevy-polio).

## Dependencies

- Bevy 0.13
- bevy_egui 0.25
