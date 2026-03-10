#!/usr/bin/env python3
"""Parameter sweep for headless polio simulation.

Runs all combinations of random_seed × fecal_oral_dose,
parses transmission CSVs, and plots epidemic curves.
"""

import itertools
import os
import subprocess
import tempfile
from pathlib import Path

import yaml
import pandas as pd
import matplotlib.pyplot as plt

# Sweep parameters
SEEDS = list(range(5))
DOSES = [1e-5, 3e-5, 1e-4]

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

    for seed, dose in combos:
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

    print("All runs complete. Generating plot...")
    plot_results()


def plot_results():
    fig, ax = plt.subplots(figsize=(10, 5))

    dose_colors = {1e-5: "C0", 3e-5: "C1", 1e-4: "C2"}
    dose_labels_used = set()

    for csv_path in sorted(OUTPUT_DIR.glob("*.csv")):
        # Parse seed and dose from filename
        name = csv_path.stem
        parts = name.split("_")
        dose_str = parts[1].replace("dose", "")
        dose = float(dose_str)
        color = dose_colors.get(dose, "gray")

        df = pd.read_csv(csv_path)
        if df.empty:
            continue

        daily = df.groupby("day").size()
        daily = daily.reindex(range(int(daily.index.max()) + 1), fill_value=0)

        label = f"dose={dose:.0e}" if dose not in dose_labels_used else None
        dose_labels_used.add(dose)
        ax.plot(daily.index, daily.values, color=color, alpha=0.5, label=label)

    ax.set_xlabel("Day")
    ax.set_ylabel("Daily transmissions")
    ax.set_title("Epidemic curves across parameter sweep")
    ax.legend()
    fig.tight_layout()

    out_path = OUTPUT_DIR / "sweep_results.png"
    fig.savefig(out_path, dpi=150)
    print(f"Plot saved to {out_path}")


if __name__ == "__main__":
    run_sweep()
