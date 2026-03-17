#!/usr/bin/env python3
"""Bridging analysis: do moderate-titer infections matter for outbreak persistence?

Decomposes two-generation transmission paths (K²[0,0]) to quantify whether
the naive-to-naive chain is self-sustaining or requires bridging through
moderate-titer intermediaries in other households.
"""

import sys
from pathlib import Path

import numpy as np
import matplotlib.pyplot as plt

sys.path.insert(0, str(Path(__file__).parent))
import disease_model as dm
import outbreak_stats as ost
from config import SAVEFIG_KWARGS

REPO_ROOT = Path(__file__).parent.parent
OUTPUT_DIR = REPO_ROOT / "scripts" / "output" / "stage1_2_review"


def compute_bridging(initial, doses, beta_hh=3.0, beta_nbhd=1.0, beta_village=0.5):
    """Compute bridging decomposition across a range of doses."""
    titers = initial["log2_prechallenge_titer"].values
    ages = initial["age_months"].values
    hh_ids = initial["household_id"].values

    results = []
    for dose in doses:
        ngm = dm.next_generation_matrix(
            titers, ages, dose,
            beta_hh=beta_hh, beta_neighborhood=beta_nbhd,
            beta_village=beta_village, household_ids=hh_ids,
        )
        K = ngm["K"]
        K2 = K @ K
        n = len(K)

        # Path decomposition: K²[0,0] = Σ_k K[0,k]·K[k,0]
        bridge = np.array([K[0, k] * K[k, 0] for k in range(n)])
        k2_00 = K2[0, 0]

        results.append({
            "dose": dose,
            "R0": ngm["R0"],
            "K00": K[0, 0],
            "K2_00": k2_00,
            "bridge": bridge,
            "bridge_frac": bridge / k2_00 if k2_00 > 0 else np.zeros(n),
            "K": K,
            "bin_edges": ngm["bin_edges"],
            "bin_fractions": ngm["bin_fractions"],
        })

    return results


def plot_bridging(results):
    """Generate bridging analysis figures."""
    doses = [r["dose"] for r in results]
    n_bins = len(results[0]["bridge"])
    bin_edges = results[0]["bin_edges"]
    labels = [f"{int(bin_edges[i])}-{int(bin_edges[i+1])}" for i in range(n_bins)]

    fig, axes = plt.subplots(2, 2, figsize=(13, 10))

    # A: R0 and K[0,0] vs dose — does the direct chain sustain?
    R0s = [r["R0"] for r in results]
    K00s = [r["K00"] for r in results]
    axes[0, 0].plot(doses, R0s, "ko-", markersize=6, label="R0 (full matrix)")
    axes[0, 0].plot(doses, K00s, "rs-", markersize=5, label="K[0-1, 0-1] (direct naive→naive)")
    axes[0, 0].axhline(1.0, color="gray", ls="--", alpha=0.5)
    axes[0, 0].set_xscale("log")
    axes[0, 0].set_xlabel("Fecal-oral dose")
    axes[0, 0].set_ylabel("Reproduction number")
    axes[0, 0].set_title("A. Direct chain vs full R0")
    axes[0, 0].legend(fontsize=8)
    # Shade the gap
    axes[0, 0].fill_between(doses, K00s, R0s, alpha=0.15, color="blue",
                            label="Bridging contribution")

    # B: Stacked area — bridging fraction by intermediate bin
    bridge_fracs = np.array([r["bridge_frac"] for r in results])  # (n_doses, n_bins)
    # Stack from bottom: bin 0 first, then 1, 2, ...
    colors = plt.cm.YlOrRd(np.linspace(0.15, 0.85, n_bins))
    axes[0, 1].stackplot(doses, bridge_fracs.T, labels=labels, colors=colors)
    axes[0, 1].set_xscale("log")
    axes[0, 1].set_xlabel("Fecal-oral dose")
    axes[0, 1].set_ylabel("Fraction of 2-gen naive→?→naive paths")
    axes[0, 1].set_title("B. Path decomposition by intermediate bin")
    axes[0, 1].legend(fontsize=6, loc="center left", ncol=1)
    axes[0, 1].set_ylim(0, 1)

    # C: For a few key doses, show the K[0,:] profile (where naive infections go)
    # Pick ~4 evenly spaced results
    idx_subset = np.linspace(0, len(results) - 1, 5, dtype=int)
    for idx in idx_subset:
        r = results[idx]
        K0_row = r["K"][0, :]
        axes[1, 0].plot(range(n_bins), K0_row, "o-", markersize=4,
                        label=f"dose={r['dose']:.1e}")
    axes[1, 0].set_xticks(range(n_bins))
    axes[1, 0].set_xticklabels(labels, fontsize=7, rotation=45)
    axes[1, 0].set_xlabel("Target titer bin")
    axes[1, 0].set_ylabel("K[0-1, j]")
    axes[1, 0].set_title("C. Where do naive-source infections land?")
    axes[1, 0].legend(fontsize=7)

    # D: K[:,0] profile — who infects naive targets?
    for idx in idx_subset:
        r = results[idx]
        K0_col = r["K"][:, 0]
        axes[1, 1].plot(range(n_bins), K0_col, "o-", markersize=4,
                        label=f"dose={r['dose']:.1e}")
    axes[1, 1].set_xticks(range(n_bins))
    axes[1, 1].set_xticklabels(labels, fontsize=7, rotation=45)
    axes[1, 1].set_xlabel("Source titer bin")
    axes[1, 1].set_ylabel("K[i, 0-1]")
    axes[1, 1].set_title("D. Who infects naive targets? (bridge potential)")
    axes[1, 1].legend(fontsize=7)

    fig.suptitle("Bridging analysis: are moderate-titer infections dead ends or bridges?",
                 fontsize=13, y=1.01)
    fig.tight_layout()
    return fig


def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    snap_dir = OUTPUT_DIR / "runs" / "seed42" / "snapshots"
    initial = ost.load_snapshot(snap_dir / "population_initial.csv")

    doses = np.logspace(-6, -2.5, 20).tolist()
    results = compute_bridging(initial, doses)

    fig = plot_bridging(results)
    fig.savefig(OUTPUT_DIR / "07_bridging_analysis.png", **SAVEFIG_KWARGS)
    print(f"Saved 07_bridging_analysis.png")
    plt.close("all")


if __name__ == "__main__":
    main()
