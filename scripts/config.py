"""Shared constants and parameters for outbreak analysis scripts.

Disease model parameter defaults match the Rust implementation in
src/disease/params.rs and src/disease/immunity.rs.
"""

# --- Disease model defaults (from params.rs) ---

# Immunity waning: I(t) = I_peak * (t/30)^(-rate), for t >= 30
IMMUNITY_WANING_RATE = 0.87

# Theta NAbs boost: log-normal with mean = a + b*log2(titer)
THETA_NABS = dict(a=4.82, b=-0.30, c=3.31, d=-0.32)

# Viral shedding shape
VIRAL_SHEDDING = dict(eta=1.65, v=0.17, epsilon=0.32)

# Peak CID50 (shedding magnitude)
PEAK_CID50 = dict(k=0.056, smax=6.7, smin=4.3, tau=12.0)

# Infection probability dose-response
P_TRANSMIT = dict(alpha=0.44, gamma=0.46)

# Strain-specific parameters
STRAIN_PARAMS = {
    "WPV": dict(sabin_scale=2.3, take_mod=1.0,
                shed_duration=dict(u=43.0, delta=1.16, sigma=1.69)),
    "VDPV": dict(sabin_scale=2.3, take_mod=1.0,
                 shed_duration=dict(u=43.0, delta=1.16, sigma=1.69)),
    "OPV1": dict(sabin_scale=14.0, take_mod=0.79,
                 shed_duration=dict(u=30.3, delta=1.16, sigma=1.86)),
    "OPV2": dict(sabin_scale=8.0, take_mod=0.92,
                 shed_duration=dict(u=30.3, delta=1.16, sigma=1.86)),
    "OPV3": dict(sabin_scale=18.0, take_mod=0.81,
                 shed_duration=dict(u=30.3, delta=1.16, sigma=1.86)),
}

# Default transmission contact rates (Poisson means)
BETA_HH = 3.0
BETA_NEIGHBORHOOD = 1.0
BETA_VILLAGE = 0.5

# --- Analysis bin edges ---

AGE_BIN_EDGES = [0, 2, 5, 12, 20, 40, 80]
AGE_BIN_LABELS = ["0-2", "2-5", "5-12", "12-20", "20-40", "40+"]

TITER_BIN_EDGES = [0, 2, 4, 6, 8, 10, 15]
TITER_BIN_LABELS = ["0-2", "2-4", "4-6", "6-8", "8-10", "10+"]

# --- Matplotlib defaults ---

import matplotlib as mpl

mpl.rcParams.update({
    "figure.dpi": 150,
    "savefig.dpi": 150,
    "font.size": 10,
    "axes.titlesize": 12,
    "axes.labelsize": 10,
})

# savefig.bbox_inches is set per-call since it's not a valid rcParam in all versions
SAVEFIG_KWARGS = dict(dpi=150, bbox_inches="tight")
