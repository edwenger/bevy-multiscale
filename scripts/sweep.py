#!/usr/bin/env python3
"""Parameter sweep for headless polio simulation.

Runs all combinations of random_seed × fecal_oral_dose,
parses transmission CSVs, and plots weekly epidemic curves
with faint individual traces and bold seed-averaged lines.
"""

import itertools
import os
import subprocess
import tempfile
from pathlib import Path

import yaml
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

# Sweep parameters
SEEDS = list(range(10))
DOSES = [1e-5, 2e-5, 5e-5, 1e-4]

BASE_CONFIG = Path(__file__).parent.parent / "config" / "base_params.yaml"
OUTPUT_DIR = Path(__file__).parent.parent / "output"
HEADLESS_BIN = "headless"


def run_sweep():
    OUTPUT_DIR.mkdir(exist_ok=True)

    with open(BASE_CONFIG) as f:
        base = yaml.safe_load(f)

    # Build release binary once
    print("Building headless binary (release)...")
    subprocess.run(
        ["cargo", "build", "--release", "--bin", "headless"],
        cwd=Path(__file__).parent.parent,
        check=True,
    )
    bin_path = Path(__file__).parent.parent / "target" / "release" / HEADLESS_BIN

    combos = list(itertools.product(SEEDS, DOSES))
    print(f"Running {len(combos)} simulations...")

    for i, (seed, dose) in enumerate(combos):
        params = {**base, "random_seed": seed, "fecal_oral_dose": dose}
        out_csv = OUTPUT_DIR / f"seed{seed}_dose{dose:.0e}.csv"

        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as tmp:
            yaml.dump(params, tmp)
            tmp_path = tmp.name

        try:
            subprocess.run(
                [str(bin_path), "--config", tmp_path, "--output", str(out_csv)],
                check=True,
            )
        finally:
            os.unlink(tmp_path)

        if (i + 1) % 10 == 0:
            print(f"  {i + 1}/{len(combos)} complete")

    print("All runs complete. Generating plot...")
    plot_results(base.get("simulation_end_time", 365))


def plot_results(end_time=365):
    fig, ax = plt.subplots(figsize=(10, 5))

    dose_colors = {1e-5: "C0", 2e-5: "C1", 5e-5: "C2", 1e-4: "C3"}
    num_weeks = end_time // 7

    # Collect weekly series grouped by dose
    dose_weekly = {dose: [] for dose in DOSES}

    for csv_path in sorted(OUTPUT_DIR.glob("seed*_dose*.csv")):
        name = csv_path.stem
        parts = name.split("_")
        dose = float(parts[1].replace("dose", ""))
        if dose not in dose_weekly:
            continue

        df = pd.read_csv(csv_path)
        if df.empty:
            weekly = pd.Series(0, index=range(num_weeks), dtype=float)
        else:
            df["week"] = df["day"] // 7
            weekly = df.groupby("week").size()
            weekly = weekly.reindex(range(num_weeks), fill_value=0).astype(float)

        dose_weekly[dose].append(weekly)

    # Plot individual traces (faint) and averages (bold)
    for dose in DOSES:
        traces = dose_weekly[dose]
        if not traces:
            continue

        color = dose_colors.get(dose, "gray")

        # Faint individual traces
        for trace in traces:
            ax.plot(trace.index, trace.values, color=color, alpha=0.15, linewidth=0.8)

        # Bold average line
        stacked = np.column_stack([t.values for t in traces])
        mean = stacked.mean(axis=1)
        ax.plot(range(num_weeks), mean, color=color, linewidth=2.0,
                label=f"dose={dose:.0e} (n={len(traces)})")

    # Set x-axis to last non-zero week across all traces
    last_nonzero = 0
    for traces in dose_weekly.values():
        for trace in traces:
            nonzero = trace[trace > 0]
            if len(nonzero) > 0:
                last_nonzero = max(last_nonzero, nonzero.index.max())
    if last_nonzero > 0:
        ax.set_xlim(-0.5, last_nonzero + 0.5)

    ax.set_xlabel("Week")
    ax.set_ylabel("Weekly transmissions")
    ax.set_title("Epidemic curves across fecal-oral dose sweep")
    ax.legend()
    fig.tight_layout()

    out_path = OUTPUT_DIR / "sweep_results.png"
    fig.savefig(out_path, dpi=150)
    print(f"Plot saved to {out_path}")


if __name__ == "__main__":
    run_sweep()
