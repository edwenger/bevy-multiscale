#!/usr/bin/env python3
"""Stage 1-2 diagnostic figures for review.

Generates a folder of figures that validate the snapshot infrastructure,
ported disease model formulas, and analytical metrics — and surface
questions for the Stage 3-4 sweep design.

Usage:
    python stage1_2_diagnostics.py
"""

import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.colors import LogNorm

sys.path.insert(0, str(Path(__file__).parent))
import disease_model as dm
import outbreak_stats as ost
from config import SAVEFIG_KWARGS

REPO_ROOT = Path(__file__).parent.parent
OUTPUT_DIR = REPO_ROOT / "scripts" / "output" / "stage1_2_review"
BIN = REPO_ROOT / "target" / "release" / "headless"
BASE_CONFIG = REPO_ROOT / "config" / "base_params.yaml"


def ensure_binary():
    subprocess.run(
        ["cargo", "build", "--release", "--bin", "headless"],
        cwd=REPO_ROOT, check=True,
    )


def run_sim(config_path, seed, output_dir):
    """Run one simulation, return (snapshot_dir, tx_path)."""
    import yaml
    with open(config_path) as f:
        params = yaml.safe_load(f)
    params["random_seed"] = seed

    run_dir = output_dir / f"seed{seed}"
    run_dir.mkdir(parents=True, exist_ok=True)
    snap_dir = run_dir / "snapshots"
    snap_dir.mkdir(exist_ok=True)
    tx_path = run_dir / "transmissions.csv"

    with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as tmp:
        yaml.dump(params, tmp)
        tmp_path = tmp.name

    subprocess.run(
        [str(BIN), "--config", tmp_path, "--output", str(tx_path),
         "--snapshot-dir", str(snap_dir)],
        check=True, capture_output=True,
    )
    Path(tmp_path).unlink()
    return snap_dir, tx_path


# -----------------------------------------------------------------------
# Fig 1: Population snapshot structure (unchanged)
# -----------------------------------------------------------------------

def fig1_population_snapshot(initial):
    fig, axes = plt.subplots(1, 3, figsize=(14, 4))

    axes[0].hist(initial["age"], bins=30, color="steelblue", edgecolor="white", alpha=0.8)
    axes[0].set_xlabel("Age (years)")
    axes[0].set_ylabel("Count")
    axes[0].set_title("Age distribution")

    axes[1].hist(initial["log2_prechallenge_titer"], bins=30, color="seagreen", edgecolor="white", alpha=0.8)
    axes[1].axvline(initial["log2_prechallenge_titer"].mean(), color="red", ls="--",
                     label=f'mean = {initial["log2_prechallenge_titer"].mean():.1f}')
    axes[1].set_xlabel("Log2 pre-challenge titer")
    axes[1].set_ylabel("Count")
    axes[1].set_title("Immunity distribution")
    axes[1].legend()

    axes[2].scatter(initial["age"], initial["log2_prechallenge_titer"], s=8, alpha=0.5, c="steelblue")
    axes[2].set_xlabel("Age (years)")
    axes[2].set_ylabel("Log2 pre-challenge titer")
    axes[2].set_title("Age vs immunity")
    axes[2].axvline(2, color="gray", ls=":", alpha=0.5)
    axes[2].axvline(12, color="gray", ls=":", alpha=0.5)
    axes[2].text(1, -0.5, "pre-\ncessation", ha="center", fontsize=7, color="gray")
    axes[2].text(7, -0.5, "transition", ha="center", fontsize=7, color="gray")
    axes[2].text(20, -0.5, "endemic", ha="center", fontsize=7, color="gray")

    fig.suptitle("Population snapshot (seed=42)", fontsize=12, y=1.02)
    fig.tight_layout()
    return fig


# -----------------------------------------------------------------------
# Fig 2: Disease model validation (unchanged)
# -----------------------------------------------------------------------

def fig2_disease_model_validation():
    fig, axes = plt.subplots(1, 3, figsize=(14, 4))

    titers = np.linspace(0, 12, 100)
    for dose in [1e1, 1e2, 1e3, 1e4, 1e5]:
        axes[0].plot(titers, dm.p_infection(titers, dose), label=f"dose={dose:.0e}")
    axes[0].set_xlabel("Log2 pre-challenge titer")
    axes[0].set_ylabel("P(infection)")
    axes[0].set_title("A. Infection probability")
    axes[0].legend(fontsize=7)
    axes[0].set_ylim(-0.02, 1.02)

    ages = np.linspace(1, 240, 200)
    for titer in [0, 2, 5, 8, 10]:
        axes[1].plot(ages / 12, dm.log10_peak_cid50(ages, titer), label=f"log2_titer={titer}")
    axes[1].set_xlabel("Age (years)")
    axes[1].set_ylabel("Log10 peak CID50")
    axes[1].set_title("B. Peak shedding")
    axes[1].legend(fontsize=7)

    titers = np.linspace(0, 12, 100)
    axes[2].plot(titers, dm.median_shed_duration(titers, u=43.0, delta=1.16), label="WPV (u=43)")
    axes[2].plot(titers, dm.median_shed_duration(titers, u=30.3, delta=1.16), label="OPV (u=30.3)", ls="--")
    axes[2].set_xlabel("Log2 pre-challenge titer")
    axes[2].set_ylabel("Median shed duration (days)")
    axes[2].set_title("C. Shed duration")
    axes[2].legend(fontsize=7)

    fig.suptitle("Disease model validation (Python port)", fontsize=12, y=1.02)
    fig.tight_layout()
    return fig


# -----------------------------------------------------------------------
# Fig 3: Snapshot consistency — now with postchallenge titers
# -----------------------------------------------------------------------

def fig3_snapshot_consistency(initial, final, tx):
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))

    # Classify individuals:
    # - "seed (transmitted)": seed case that appears as source_id in transmissions
    # - "seed (dead-end)": seed case that never transmitted
    # - "infected (secondary)": appears as target_id in transmissions
    # - "uninfected": none of the above
    seed_ids = set(initial.loc[initial["strain"].notna(), "individual_id"])
    secondary_ids = set(tx["target_id"].unique()) if not tx.empty else set()
    source_ids = set(tx["source_id"].unique()) if not tx.empty else set()
    # Also detect seed cases that didn't show in initial snapshot but did transmit
    tx_source_only = source_ids - secondary_ids
    seed_ids = seed_ids | tx_source_only
    # Detect silent seed cases via off-diagonal postchallenge boost
    merged_all = initial[["individual_id", "log2_prechallenge_titer"]].merge(
        final[["individual_id", "log2_postchallenge_titer"]], on="individual_id",
    )
    boosted = merged_all[
        (merged_all["log2_postchallenge_titer"] - merged_all["log2_prechallenge_titer"]) > 0.5
    ]["individual_id"]
    silent_seeds = set(boosted) - secondary_ids - seed_ids
    seed_ids = seed_ids | silent_seeds

    seed_transmitted = seed_ids & source_ids
    seed_deadend = seed_ids - seed_transmitted
    secondary_transmitted = secondary_ids & source_ids
    secondary_deadend = secondary_ids - secondary_transmitted

    # Panel A: Initial pre-challenge titer vs final postchallenge titer
    merged = merged_all.copy()
    is_sec_tx = merged["individual_id"].isin(secondary_transmitted)
    is_sec_de = merged["individual_id"].isin(secondary_deadend)
    is_seed_tx = merged["individual_id"].isin(seed_transmitted)
    is_seed_de = merged["individual_id"].isin(seed_deadend)
    is_uninf = ~is_sec_tx & ~is_sec_de & ~is_seed_tx & ~is_seed_de

    axes[0].scatter(merged.loc[is_uninf, "log2_prechallenge_titer"],
                    merged.loc[is_uninf, "log2_postchallenge_titer"],
                    s=8, alpha=0.3, color="gray", label="uninfected", zorder=1)
    if is_sec_tx.any():
        axes[0].scatter(merged.loc[is_sec_tx, "log2_prechallenge_titer"],
                        merged.loc[is_sec_tx, "log2_postchallenge_titer"],
                        s=30, alpha=0.8, color="red", marker="^", label="secondary (transmitted)", zorder=3)
    if is_sec_de.any():
        axes[0].scatter(merged.loc[is_sec_de, "log2_prechallenge_titer"],
                        merged.loc[is_sec_de, "log2_postchallenge_titer"],
                        s=30, alpha=0.8, color="red", marker="^", facecolors="none",
                        linewidths=1.5, label="secondary (dead-end)", zorder=3)
    if is_seed_tx.any():
        axes[0].scatter(merged.loc[is_seed_tx, "log2_prechallenge_titer"],
                        merged.loc[is_seed_tx, "log2_postchallenge_titer"],
                        s=50, alpha=0.9, color="orange", marker="s", label="seed (transmitted)", zorder=4)
    if is_seed_de.any():
        axes[0].scatter(merged.loc[is_seed_de, "log2_prechallenge_titer"],
                        merged.loc[is_seed_de, "log2_postchallenge_titer"],
                        s=50, alpha=0.9, color="orange", marker="s", facecolors="none",
                        linewidths=1.5, label="seed (dead-end)", zorder=4)
    lims = [-0.5, max(merged["log2_prechallenge_titer"].max(),
                      merged["log2_postchallenge_titer"].max()) + 0.5]
    axes[0].plot(lims, lims, "k--", alpha=0.3)
    axes[0].set_xlabel("Initial pre-challenge log2 titer")
    axes[0].set_ylabel("Final postchallenge log2 titer")
    axes[0].set_title("A. Immunity boost from infection")
    axes[0].legend(fontsize=7)

    # Panel B: who got infected? age vs initial titer
    axes[1].scatter(initial["age"], initial["log2_prechallenge_titer"],
                    s=8, alpha=0.3, color="gray", label="all")
    for ids, color, marker, sz, label, fill in [
        (secondary_transmitted, "red", "^", 30, "secondary (transmitted)", True),
        (secondary_deadend, "red", "^", 30, "secondary (dead-end)", False),
        (seed_transmitted, "orange", "s", 50, "seed (transmitted)", True),
        (seed_deadend, "orange", "s", 50, "seed (dead-end)", False),
    ]:
        snap = initial[initial["individual_id"].isin(ids)]
        if snap.empty:
            continue
        kw = dict(s=sz, alpha=0.8 if fill else 0.9, color=color, marker=marker, label=label)
        if not fill:
            kw.update(facecolors="none", linewidths=1.5)
        axes[1].scatter(snap["age"], snap["log2_prechallenge_titer"], **kw)
    axes[1].set_xlabel("Age (years)")
    axes[1].set_ylabel("Log2 pre-challenge titer")
    axes[1].set_title("B. Who got infected?")
    axes[1].legend(fontsize=7)

    fig.suptitle("Snapshot consistency: initial vs final", fontsize=12, y=1.02)
    fig.tight_layout()
    return fig


# -----------------------------------------------------------------------
# Fig 4: Analytical R0 (NGM) vs empirical, across seeds
# -----------------------------------------------------------------------

def fig4_analytical_metrics_across_seeds(n_seeds=10):
    import yaml
    with open(BASE_CONFIG) as f:
        base = yaml.safe_load(f)
    dose = base.get("fecal_oral_dose", 1e-5)

    summaries = []
    for seed in range(n_seeds):
        snap_dir, tx_path = run_sim(BASE_CONFIG, seed, OUTPUT_DIR / "runs")
        s = ost.compute_summary(str(snap_dir), str(tx_path), dose)
        s["seed"] = seed
        summaries.append(s)

    df = pd.DataFrame(summaries)

    fig, axes = plt.subplots(2, 2, figsize=(10, 8))

    # A: Attack rate distribution
    axes[0, 0].bar(df["seed"], df["attack_rate"], color="steelblue", alpha=0.8)
    axes[0, 0].axhline(0.05, color="red", ls="--", alpha=0.5, label="fizzle threshold (5%)")
    axes[0, 0].set_xlabel("Seed")
    axes[0, 0].set_ylabel("Attack rate")
    axes[0, 0].set_title(f"A. Attack rate (dose={dose:.0e})")
    axes[0, 0].legend(fontsize=8)

    # B: NGM R0 vs empirical attack rate
    axes[0, 1].scatter(df["analytical_R0"], df["attack_rate"], s=40, alpha=0.7)
    axes[0, 1].axvline(1.0, color="red", ls="--", alpha=0.5, label="R0 = 1")
    axes[0, 1].set_xlabel("NGM R0 (dominant eigenvalue)")
    axes[0, 1].set_ylabel("Empirical attack rate")
    axes[0, 1].set_title("B. NGM R0 vs attack rate")
    axes[0, 1].legend(fontsize=8)

    # C: Mean susceptibility vs attack rate
    axes[1, 0].scatter(df["mean_susceptibility"], df["attack_rate"], s=40, alpha=0.7, color="seagreen")
    axes[1, 0].set_xlabel("Mean susceptibility")
    axes[1, 0].set_ylabel("Empirical attack rate")
    axes[1, 0].set_title("C. Mean susceptibility vs attack rate")

    # D: Summary table
    summary_text = (
        f"N simulations: {n_seeds}\n"
        f"Population size: {df['pop_size'].iloc[0]}\n"
        f"Fizzle rate: {df['fizzle'].mean():.0%}\n\n"
        f"NGM R0: {df['analytical_R0'].mean():.2f} ± {df['analytical_R0'].std():.2f}\n"
        f"Mean susceptibility: {df['mean_susceptibility'].mean():.4f} ± {df['mean_susceptibility'].std():.4f}\n"
        f"Attack rate: {df['attack_rate'].mean():.3f} ± {df['attack_rate'].std():.3f}\n"
        f"Total infections: {df['total_infections'].mean():.1f} ± {df['total_infections'].std():.1f}\n"
    )
    axes[1, 1].text(0.1, 0.5, summary_text, transform=axes[1, 1].transAxes,
                    fontsize=10, verticalalignment="center", fontfamily="monospace")
    axes[1, 1].set_title("D. Summary statistics")
    axes[1, 1].axis("off")

    fig.suptitle(f"Analytical metrics across {n_seeds} seeds (base config)", fontsize=12, y=1.02)
    fig.tight_layout()
    return fig, df


# -----------------------------------------------------------------------
# Fig 5: Next-generation matrix diagnostics
# -----------------------------------------------------------------------

def _bin_range_labels(bin_edges):
    """Generate range labels like '0-1', '1-2', ..., '10-13' from bin edges."""
    return [f"{int(bin_edges[i])}-{int(bin_edges[i+1])}" for i in range(len(bin_edges) - 1)]


def fig5_ngm_diagnostics(initial):
    """Visualize the next-generation matrix with household structure."""
    import yaml
    with open(BASE_CONFIG) as f:
        base = yaml.safe_load(f)

    log2_titers = initial["log2_prechallenge_titer"].values
    ages_months = initial["age_months"].values
    hh_ids = initial["household_id"].values
    dose = base.get("fecal_oral_dose", 1e-5)

    ngm = dm.next_generation_matrix(
        log2_titers, ages_months, dose,
        beta_hh=base.get("beta_hh", 3.0),
        beta_neighborhood=base.get("beta_neighborhood", 1.0),
        beta_village=base.get("beta_village", 0.5),
        household_ids=hh_ids,
    )

    K = ngm["K"]
    K_hh = ngm["K_hh"]
    K_comm = ngm["K_community"]
    fracs = ngm["bin_fractions"]
    integ_p = ngm["integrated_p"]
    n = len(ngm["bin_centers"])
    labels = _bin_range_labels(ngm["bin_edges"])

    fig, axes = plt.subplots(2, 3, figsize=(16, 10))

    # A: K matrix (total = HH + community)
    im = axes[0, 0].imshow(K, origin="lower", cmap="YlOrRd", aspect="equal")
    for i in range(n):
        for j in range(n):
            if K[i, j] > 0.005:
                axes[0, 0].text(j, i, f"{K[i,j]:.2f}", ha="center", va="center",
                                fontsize=5, color="black" if K[i,j] < K.max()*0.6 else "white")
    axes[0, 0].set_xticks(range(n))
    axes[0, 0].set_xticklabels(labels, fontsize=6, rotation=45)
    axes[0, 0].set_yticks(range(n))
    axes[0, 0].set_yticklabels(labels, fontsize=6)
    axes[0, 0].set_xlabel("Target titer bin (log2)")
    axes[0, 0].set_ylabel("Source titer bin (log2)")
    axes[0, 0].set_title(f"A. K = K_hh + K_community  (R0 = {ngm['R0']:.2f})")
    fig.colorbar(im, ax=axes[0, 0], label="Expected secondary infections")

    # B: Row sums — stacked HH vs community
    row_hh = K_hh.sum(axis=1)
    row_comm = K_comm.sum(axis=1)
    axes[0, 1].barh(range(n), row_hh, color="salmon", alpha=0.7, label="Household")
    axes[0, 1].barh(range(n), row_comm, left=row_hh, color="cornflowerblue", alpha=0.7, label="Community")
    axes[0, 1].set_yticks(range(n))
    axes[0, 1].set_yticklabels(labels, fontsize=6)
    axes[0, 1].set_xlabel("R_i (total secondary infections)")
    axes[0, 1].set_ylabel("Source titer bin (log2)")
    axes[0, 1].set_title("B. Row sums: HH vs community")
    axes[0, 1].axvline(1.0, color="red", ls="--", alpha=0.5, label="R=1")
    axes[0, 1].legend(fontsize=7)

    # C: Population fraction vs infection eigenvector
    ax_pop = axes[0, 2]
    ax_pop.bar(range(n), fracs, color="steelblue", alpha=0.6, label="Population fraction (f_j)")
    ax_eig = ax_pop.twinx()
    ax_eig.plot(range(n), ngm["eigenvector"], "ro-", markersize=5,
                label="Infection distribution\n(dom. eigenvector)")
    ax_pop.set_xticks(range(n))
    ax_pop.set_xticklabels(labels, fontsize=6, rotation=45)
    ax_pop.set_xlabel("Titer bin (log2)")
    ax_pop.set_ylabel("Population fraction", color="steelblue")
    ax_eig.set_ylabel("Relative infection risk", color="red")
    ax_pop.set_title("C. Population vs infection distribution")
    lines1, labels1 = ax_pop.get_legend_handles_labels()
    lines2, labels2 = ax_eig.get_legend_handles_labels()
    ax_pop.legend(lines1 + lines2, labels1 + labels2, fontsize=7, loc="upper right")

    # D: Per-individual R scatter — age × titer, colored by R
    R = ngm["R_per_individual"]
    sc = axes[1, 0].scatter(initial["age"], log2_titers, c=R, cmap="YlOrRd",
                            s=15, alpha=0.7, vmin=0, vmax=min(R.max(), 30))
    axes[1, 0].set_xlabel("Age (years)")
    axes[1, 0].set_ylabel("Log2 pre-challenge titer")
    axes[1, 0].set_title("D. Per-individual R (age × titer)")
    fig.colorbar(sc, ax=axes[1, 0], label="Expected secondary infections")

    # E: Per-individual R_hh vs R_community
    axes[1, 1].scatter(ngm["R_community"], ngm["R_hh"], c=log2_titers, cmap="viridis",
                       s=15, alpha=0.7)
    axes[1, 1].plot([0, max(ngm["R_community"].max(), ngm["R_hh"].max())],
                    [0, max(ngm["R_community"].max(), ngm["R_hh"].max())],
                    "k--", alpha=0.3)
    axes[1, 1].set_xlabel("R (community contacts)")
    axes[1, 1].set_ylabel("R (household contacts)")
    axes[1, 1].set_title("E. Household vs community R")
    fig.colorbar(axes[1, 1].collections[0], ax=axes[1, 1], label="Log2 titer")

    # F: Shedding curves for representative source bins
    centers = ngm["bin_centers"]
    source_bins_to_show = [0, 2, 4, 7]
    for src_b in source_bins_to_show:
        if fracs[src_b] == 0:
            continue
        peak = ngm["bin_mean_log10_peak_shed"][src_b]
        dur = ngm["bin_mean_shed_duration"][src_b]
        days = np.arange(1, int(dur) + 1)
        shed = dm.viral_shedding(peak, days)
        axes[1, 2].plot(days, np.log10(shed),
                        label=f"{labels[src_b]} (peak={peak:.1f}, dur={dur:.0f}d)")
    axes[1, 2].set_xlabel("Days since infection")
    axes[1, 2].set_ylabel("Log10 shedding (CID50)")
    axes[1, 2].set_title("F. Shedding curves by source bin")
    axes[1, 2].legend(fontsize=6)

    fig.suptitle("Next-generation matrix diagnostics (initial population, with household structure)",
                 fontsize=13, y=1.01)
    fig.tight_layout()
    return fig


# -----------------------------------------------------------------------
# Fig 6: R0 landscape with NGM approach
# -----------------------------------------------------------------------

def fig6_ngm_landscape():
    """R0 (NGM) across dose × mean_titer, compared to old mean-field."""
    fig, axes = plt.subplots(1, 3, figsize=(15, 4.5))

    dose_range = np.logspace(-6, -3, 25)
    titer_range = np.linspace(1, 10, 25)
    R0_grid = np.zeros((len(titer_range), len(dose_range)))
    rng = np.random.default_rng(42)

    for i, mt in enumerate(titer_range):
        pop_titers = np.clip(rng.normal(mt, 1.5, 200), 0, 12)
        pop_ages = rng.uniform(6, 600, 200)
        for j, d in enumerate(dose_range):
            R0_grid[i, j] = dm.analytical_R0(pop_titers, pop_ages, d)

    # A: Heatmap
    im = axes[0].imshow(R0_grid, aspect="auto", origin="lower",
                        extent=[np.log10(dose_range[0]), np.log10(dose_range[-1]),
                                titer_range[0], titer_range[-1]],
                        cmap="RdYlGn_r", vmin=0, vmax=max(3, R0_grid.max()))
    cs = axes[0].contour(np.log10(dose_range), titer_range, R0_grid,
                         levels=[1.0], colors="black", linewidths=2)
    axes[0].clabel(cs, fmt="R0=%.0f", fontsize=10)
    axes[0].set_xlabel("Log10(fecal-oral dose)")
    axes[0].set_ylabel("Mean log2 titer")
    axes[0].set_title("A. NGM R0 (dose × mean titer)")
    fig.colorbar(im, ax=axes[0], label="R0")

    # B: 1D slices at fixed dose
    for d in [1e-5, 5e-5, 1e-4, 5e-4]:
        r0_vals = []
        for mt in titer_range:
            pop_titers = np.clip(rng.normal(mt, 1.5, 200), 0, 12)
            pop_ages = rng.uniform(6, 600, 200)
            r0_vals.append(dm.analytical_R0(pop_titers, pop_ages, d))
        axes[1].plot(titer_range, r0_vals, label=f"dose={d:.0e}")
    axes[1].axhline(1.0, color="red", ls="--", alpha=0.5)
    axes[1].set_xlabel("Mean log2 titer")
    axes[1].set_ylabel("NGM R0")
    axes[1].set_title("B. R0 vs mean titer (fixed dose)")
    axes[1].legend(fontsize=7)

    # C: 1D slices at fixed titer
    for mt in [2, 4, 6, 8]:
        r0_vals = []
        for d in dose_range:
            pop_titers = np.clip(rng.normal(mt, 1.5, 200), 0, 12)
            pop_ages = rng.uniform(6, 600, 200)
            r0_vals.append(dm.analytical_R0(pop_titers, pop_ages, d))
        axes[2].plot(dose_range, r0_vals, label=f"mean titer={mt}")
    axes[2].axhline(1.0, color="red", ls="--", alpha=0.5)
    axes[2].set_xscale("log")
    axes[2].set_xlabel("Fecal-oral dose")
    axes[2].set_ylabel("NGM R0")
    axes[2].set_title("C. R0 vs dose (fixed mean titer)")
    axes[2].legend(fontsize=7)

    fig.suptitle("NGM R0 landscape — informing Stage 4 sweep design", fontsize=12, y=1.02)
    fig.tight_layout()
    return fig


# -----------------------------------------------------------------------
# Main
# -----------------------------------------------------------------------

def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    ensure_binary()

    # Run one baseline simulation for snapshot review
    snap_dir, tx_path = run_sim(BASE_CONFIG, seed=42, output_dir=OUTPUT_DIR / "runs")
    initial = ost.load_snapshot(snap_dir / "population_initial.csv")
    final = ost.load_snapshot(snap_dir / "population_final.csv")
    tx = ost.load_transmissions(tx_path)

    fig1 = fig1_population_snapshot(initial)
    fig1.savefig(OUTPUT_DIR / "01_population_snapshot.png", **SAVEFIG_KWARGS)
    print("Saved 01_population_snapshot.png")

    fig2 = fig2_disease_model_validation()
    fig2.savefig(OUTPUT_DIR / "02_disease_model_validation.png", **SAVEFIG_KWARGS)
    print("Saved 02_disease_model_validation.png")

    fig3 = fig3_snapshot_consistency(initial, final, tx)
    fig3.savefig(OUTPUT_DIR / "03_snapshot_consistency.png", **SAVEFIG_KWARGS)
    print("Saved 03_snapshot_consistency.png")

    fig4, seed_df = fig4_analytical_metrics_across_seeds(n_seeds=10)
    fig4.savefig(OUTPUT_DIR / "04_analytical_vs_empirical.png", **SAVEFIG_KWARGS)
    seed_df.to_csv(OUTPUT_DIR / "04_seed_summary.csv", index=False)
    print("Saved 04_analytical_vs_empirical.png + CSV")

    fig5 = fig5_ngm_diagnostics(initial)
    fig5.savefig(OUTPUT_DIR / "05_ngm_diagnostics.png", **SAVEFIG_KWARGS)
    print("Saved 05_ngm_diagnostics.png")

    fig6 = fig6_ngm_landscape()
    fig6.savefig(OUTPUT_DIR / "06_ngm_landscape.png", **SAVEFIG_KWARGS)
    print("Saved 06_ngm_landscape.png")

    plt.close("all")
    print(f"\nAll figures saved to {OUTPUT_DIR}/")


if __name__ == "__main__":
    main()
