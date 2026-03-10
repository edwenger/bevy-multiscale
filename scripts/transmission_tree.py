#!/usr/bin/env python3
"""Plot a transmission tree from a single headless simulation CSV.

Usage:
    python scripts/transmission_tree.py output/seed0_dose1e-04.csv

Reads the CSV of transmission events and draws a tree where:
- x-axis = day of infection
- y-axis = branching layout (children spread below/above their parent)
- edges connect source infection → acquired infection
- nodes colored by contact level (household/neighborhood/village)
- handles re-infections by treating each transmission as a unique node
"""

import sys
from collections import defaultdict
from pathlib import Path

import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.patches as mpatches


LEVEL_COLORS = {
    "household": "#e74c3c",
    "neighborhood": "#f39c12",
    "village": "#3498db",
    "seed": "#2c3e50",
}


def build_tree(df):
    """Build a transmission tree where each node is a unique infection event.

    Each CSV row (a successful transmission) becomes a node. The parent of
    each node is the source individual's most recent infection event that
    precedes this transmission day. Seed cases (sources never seen as
    targets) get synthetic root nodes.
    """
    # Each row becomes a node. Node ID = row index.
    # Track: for each individual, the ordered list of node IDs where they
    # were infected, so we can find the "active infection" of a source.
    individual_infections = defaultdict(list)  # entity_id -> [(day, node_id)]

    nodes = {}  # node_id -> {day, individual, age, level}
    children = defaultdict(list)  # node_id -> [child_node_ids]

    # First pass: identify seed cases (sources that appear before any
    # infection of that individual as a target)
    source_first_day = {}
    target_first_day = {}
    for _, row in df.iterrows():
        src = int(row["source_id"])
        tgt = int(row["target_id"])
        day = row["day"]
        if src not in source_first_day or day < source_first_day[src]:
            source_first_day[src] = day
        if tgt not in target_first_day or day < target_first_day[tgt]:
            target_first_day[tgt] = day

    # Create synthetic root nodes for seed cases
    seed_node_id = -1
    for src, first_src_day in sorted(source_first_day.items()):
        if src not in target_first_day or target_first_day[src] >= first_src_day:
            # This source was active before ever being a recorded target = seed case
            nodes[seed_node_id] = {
                "day": first_src_day - 1,
                "individual": src,
                "age": None,  # filled below
                "level": "seed",
            }
            individual_infections[src].append((first_src_day - 1, seed_node_id))
            seed_node_id -= 1

    # Fill seed ages from first appearance as source
    for _, row in df.iterrows():
        src = int(row["source_id"])
        for nid, info in nodes.items():
            if info["individual"] == src and info["age"] is None:
                info["age"] = row["source_age"]

    # Second pass: create a node per transmission row and link to parent
    for idx, row in df.iterrows():
        src = int(row["source_id"])
        tgt = int(row["target_id"])
        day = row["day"]

        node_id = idx
        nodes[node_id] = {
            "day": day,
            "individual": tgt,
            "age": row["target_age"],
            "level": row["level"],
        }
        individual_infections[tgt].append((day, node_id))

        # Find source's active infection: most recent infection of src at or before this day
        src_infections = individual_infections.get(src, [])
        parent = None
        for inf_day, inf_node in reversed(src_infections):
            if inf_day <= day:
                parent = inf_node
                break

        if parent is not None:
            children[parent].append(node_id)

    return nodes, children


def layout_tree(nodes, children):
    """Assign y-positions with a depth-first leaf-packing layout."""
    # Find roots (nodes that are not children of anything)
    all_children = set()
    for kids in children.values():
        all_children.update(kids)
    roots = sorted([nid for nid in nodes if nid not in all_children],
                   key=lambda n: nodes[n]["day"])

    # Sort children by day
    for nid in children:
        children[nid].sort(key=lambda c: nodes[c]["day"])

    # Assign y by DFS leaf packing
    y_pos = {}
    current_y = [0.0]

    def assign_y(node):
        kids = children.get(node, [])
        if not kids:
            y_pos[node] = current_y[0]
            current_y[0] += 1.0
        else:
            for child in kids:
                assign_y(child)
            child_ys = [y_pos[c] for c in kids]
            y_pos[node] = (min(child_ys) + max(child_ys)) / 2.0

    for r in roots:
        assign_y(r)

    return y_pos, roots


def plot_tree(csv_path, ax=None):
    df = pd.read_csv(csv_path)
    if df.empty:
        print(f"No transmissions in {csv_path}")
        return

    nodes, children = build_tree(df)
    y_pos, roots = layout_tree(nodes, children)

    num_nodes = len(y_pos)
    if ax is None:
        height = max(4, min(num_nodes * 0.12, 20))
        fig, ax = plt.subplots(figsize=(14, height))
        standalone = True
    else:
        standalone = False

    # Draw edges
    for parent, kids in children.items():
        if parent not in y_pos:
            continue
        px, py = nodes[parent]["day"], y_pos[parent]
        for child in kids:
            if child not in y_pos:
                continue
            cx, cy = nodes[child]["day"], y_pos[child]
            level = nodes[child]["level"]
            color = LEVEL_COLORS.get(level, "#999999")
            # Draw as elbow: horizontal from parent, then vertical, then horizontal to child
            mid_x = (px + cx) / 2
            ax.plot([px, mid_x, mid_x, cx], [py, py, cy, cy],
                    color=color, linewidth=0.8, alpha=0.5, zorder=1)

    # Draw nodes
    for nid in y_pos:
        info = nodes[nid]
        level = info["level"]
        color = LEVEL_COLORS.get(level, "#2c3e50")
        marker = "s" if level == "seed" else "o"
        ax.scatter(info["day"], y_pos[nid], c=color, s=20, zorder=2,
                   edgecolors="white", linewidths=0.3, marker=marker)

    # Age labels for leaf nodes
    all_children_set = set()
    for kids in children.values():
        all_children_set.update(kids)
    for nid in y_pos:
        if nid not in children or not children[nid]:
            age = nodes[nid].get("age")
            if age is not None:
                ax.annotate(
                    f"{age:.0f}y", (nodes[nid]["day"], y_pos[nid]),
                    xytext=(4, 0), textcoords="offset points",
                    fontsize=5, color="#888888", va="center",
                )

    # Legend
    handles = [
        mpatches.Patch(color=LEVEL_COLORS["seed"], label="Seed"),
        mpatches.Patch(color=LEVEL_COLORS["household"], label="Household"),
        mpatches.Patch(color=LEVEL_COLORS["neighborhood"], label="Neighborhood"),
        mpatches.Patch(color=LEVEL_COLORS["village"], label="Village"),
    ]
    ax.legend(handles=handles, loc="upper left", fontsize=8)

    ax.set_xlabel("Day")
    ax.set_yticks([])
    ax.set_title(f"Transmission tree ({num_nodes} infections)")
    ax.margins(x=0.03, y=0.03)

    if standalone:
        fig.tight_layout()
        out_path = Path(csv_path).with_suffix(".tree.png")
        fig.savefig(out_path, dpi=150, bbox_inches="tight")
        print(f"Saved to {out_path}")
        plt.close(fig)


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python scripts/transmission_tree.py <transmissions.csv>")
        sys.exit(1)
    plot_tree(sys.argv[1])
