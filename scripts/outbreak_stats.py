#!/usr/bin/env python3
"""Outbreak statistics from headless simulation outputs.

Reads population snapshots (initial/final) and transmission CSV to compute
both empirical and analytical outbreak metrics.

Usage:
    python outbreak_stats.py --snapshot-dir output/snapshots --transmissions output/tx.csv
    python outbreak_stats.py --snapshot-dir output/snapshots  # snapshots only (analytical)
"""

import argparse
from pathlib import Path

import numpy as np
import pandas as pd

import disease_model as dm
from config import AGE_BIN_EDGES, AGE_BIN_LABELS, TITER_BIN_EDGES, TITER_BIN_LABELS


# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------

def load_snapshot(path):
    """Load a population snapshot CSV."""
    df = pd.read_csv(path)
    df["age_months"] = df["age"] * 12.0
    return df


def load_transmissions(path):
    """Load a transmissions CSV."""
    return pd.read_csv(path)


# ---------------------------------------------------------------------------
# Empirical metrics (from transmission CSV + snapshots)
# ---------------------------------------------------------------------------

def attack_rate(tx, pop_size):
    """Fraction of population that was infected."""
    if tx.empty:
        return 0.0
    return tx["target_id"].nunique() / pop_size


def outbreak_duration(tx):
    """Days between first and last transmission."""
    if tx.empty:
        return 0
    return int(tx["day"].max() - tx["day"].min())


def total_infections(tx):
    """Total number of transmission events."""
    return len(tx)


def peak_weekly_incidence(tx):
    """Maximum weekly new infections."""
    if tx.empty:
        return 0
    tx = tx.copy()
    tx["week"] = tx["day"] // 7
    weekly = tx.groupby("week").size()
    return int(weekly.max())


def penetration_by_neighborhood(tx, initial_snap):
    """Attack rate per neighborhood."""
    if tx.empty:
        return pd.Series(dtype=float)
    pop_by_nbhd = initial_snap.groupby("neighborhood_id").size()
    infected = tx.merge(
        initial_snap[["individual_id", "neighborhood_id"]],
        left_on="target_id", right_on="individual_id",
    )
    inf_by_nbhd = infected.groupby("neighborhood_id")["target_id"].nunique()
    return (inf_by_nbhd / pop_by_nbhd).fillna(0)


def penetration_by_household(tx, initial_snap):
    """Attack rate per household."""
    if tx.empty:
        return pd.Series(dtype=float)
    pop_by_hh = initial_snap.groupby("household_id").size()
    infected = tx.merge(
        initial_snap[["individual_id", "household_id"]],
        left_on="target_id", right_on="individual_id",
    )
    inf_by_hh = infected.groupby("household_id")["target_id"].nunique()
    return (inf_by_hh / pop_by_hh).fillna(0)


def generation_intervals(tx):
    """Day deltas along transmission chains (source→target timing)."""
    if tx.empty:
        return np.array([])
    # Build source-to-target day mapping
    target_days = tx.set_index("target_id")["day"].to_dict()
    intervals = []
    for _, row in tx.iterrows():
        source_day = target_days.get(row["source_id"])
        if source_day is not None:
            intervals.append(row["day"] - source_day)
    return np.array(intervals)


def immunity_transitions(initial_snap, final_snap):
    """Cross-tab of age bin × titer bin changes between initial and final snapshots."""
    merged = initial_snap[["individual_id", "age", "log2_prechallenge_titer"]].merge(
        final_snap[["individual_id", "log2_prechallenge_titer"]],
        on="individual_id", suffixes=("_initial", "_final"),
    )
    merged["age_bin"] = pd.cut(merged["age"], bins=AGE_BIN_EDGES, labels=AGE_BIN_LABELS, right=False)
    merged["titer_initial_bin"] = pd.cut(
        merged["log2_prechallenge_titer_initial"], bins=TITER_BIN_EDGES, labels=TITER_BIN_LABELS, right=False,
    )
    merged["titer_final_bin"] = pd.cut(
        merged["log2_prechallenge_titer_final"], bins=TITER_BIN_EDGES, labels=TITER_BIN_LABELS, right=False,
    )
    merged["boosted"] = merged["log2_prechallenge_titer_final"] > merged["log2_prechallenge_titer_initial"] + 0.5
    return merged


def mean_secondary_infections(tx):
    """Mean number of secondary infections per source (empirical R)."""
    if tx.empty:
        return 0.0
    return tx.groupby("source_id").size().mean()


# ---------------------------------------------------------------------------
# Analytical metrics (from initial snapshot + disease_model.py)
# ---------------------------------------------------------------------------

def compute_analytical_metrics(initial_snap, fecal_oral_dose, **kwargs):
    """Compute analytical metrics from the initial population snapshot."""
    log2_titers = initial_snap["log2_prechallenge_titer"].values
    ages_months = initial_snap["age_months"].values

    # Mean peak shedding gives reference dose
    peak_shed = np.power(10.0, dm.log10_peak_cid50(ages_months, log2_titers))
    ref_dose = np.mean(peak_shed) * fecal_oral_dose

    return {
        "mean_log2_titer": float(np.mean(log2_titers)),
        "mean_susceptibility": dm.mean_susceptibility(log2_titers, ref_dose),
        "mean_peak_shedding_log10": float(np.mean(dm.log10_peak_cid50(ages_months, log2_titers))),
        "analytical_R0": dm.analytical_R0(log2_titers, ages_months, fecal_oral_dose, **kwargs),
    }


# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

def compute_summary(snapshot_dir, transmissions_path=None, fecal_oral_dose=1e-5, **kwargs):
    """Compute full summary statistics for a single simulation run."""
    snapshot_dir = Path(snapshot_dir)
    initial_snap = load_snapshot(snapshot_dir / "population_initial.csv")
    final_snap = load_snapshot(snapshot_dir / "population_final.csv")
    pop_size = len(initial_snap)

    summary = {"pop_size": pop_size}

    # Analytical (always available from snapshots)
    summary.update(compute_analytical_metrics(initial_snap, fecal_oral_dose, **kwargs))

    # Empirical (requires transmission CSV)
    if transmissions_path is not None:
        tx = load_transmissions(transmissions_path)
        summary["attack_rate"] = attack_rate(tx, pop_size)
        summary["outbreak_duration"] = outbreak_duration(tx)
        summary["total_infections"] = total_infections(tx)
        summary["peak_weekly_incidence"] = peak_weekly_incidence(tx)
        summary["mean_secondary_infections"] = mean_secondary_infections(tx)
        summary["fizzle"] = summary["attack_rate"] < 0.05

        gi = generation_intervals(tx)
        summary["mean_generation_interval"] = float(np.mean(gi)) if len(gi) > 0 else None

    return summary


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="Compute outbreak statistics")
    parser.add_argument("--snapshot-dir", required=True, help="Directory with population_initial/final.csv")
    parser.add_argument("--transmissions", default=None, help="Path to transmissions.csv")
    parser.add_argument("--fecal-oral-dose", type=float, default=1e-5, help="Fecal-oral dose parameter")
    args = parser.parse_args()

    summary = compute_summary(args.snapshot_dir, args.transmissions, args.fecal_oral_dose)

    print("\n=== Outbreak Summary ===")
    for k, v in summary.items():
        if isinstance(v, float):
            print(f"  {k}: {v:.4f}")
        else:
            print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
