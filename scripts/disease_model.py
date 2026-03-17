"""Ported disease model formulas from the Rust simulation.

Functions mirror the calculations in:
  - src/disease/immunity.rs (infection probability, peak shedding, shed duration)
  - src/disease/params.rs (default parameter values)

All functions accept scalar or numpy array inputs for vectorized analysis.
"""

import math
import numpy as np
from config import PEAK_CID50, P_TRANSMIT, STRAIN_PARAMS


def p_infection(log2_titer, dose, strain="WPV",
                alpha=P_TRANSMIT["alpha"], gamma=P_TRANSMIT["gamma"],
                sabin_scale=None, take_mod=None):
    """Probability of infection given pre-challenge titer and fecal-oral dose.

    Mirrors Immunity::calculate_infection_probability (immunity.rs:94-112).

    Parameters
    ----------
    log2_titer : float or array
        Log2 of pre-challenge NAb titer.
    dose : float
        Fecal-oral dose (viral_shedding * fecal_oral_dose * shedding_reduction).
    strain : str
        Strain key for default sabin_scale/take_mod lookup.
    """
    if sabin_scale is None:
        sabin_scale = STRAIN_PARAMS[strain]["sabin_scale"]
    if take_mod is None:
        take_mod = STRAIN_PARAMS[strain]["take_mod"]

    immunity = np.power(2.0, log2_titer)
    return (1.0 - np.power(1.0 + dose / sabin_scale, -alpha * np.power(immunity, -gamma))) * take_mod


def log10_peak_cid50(age_months, log2_titer,
                     k=PEAK_CID50["k"], smax=PEAK_CID50["smax"],
                     smin=PEAK_CID50["smin"], tau=PEAK_CID50["tau"]):
    """Analytical peak shedding (log10 CID50) given age and pre-challenge titer.

    Mirrors Immunity::calculate_log10_peak_cid50 (immunity.rs:81-92).
    """
    age_months = np.asarray(age_months, dtype=float)
    log2_titer = np.asarray(log2_titer, dtype=float)
    naive_peak = np.where(
        age_months >= 6.0,
        (smax - smin) * np.exp((7.0 - age_months) / tau) + smin,
        smax,
    )
    return naive_peak * (1.0 - k * log2_titer)


def median_shed_duration(log2_titer, u=43.0, delta=1.16):
    """Median shedding duration (days) given pre-challenge titer.

    Derived from the log-normal shed duration in immunity.rs:60-68.
    The median of LogNormal(mu, sigma) is exp(mu), so sigma drops out.
    """
    log2_titer = np.asarray(log2_titer, dtype=float)
    mu = np.log(u) - np.log(delta) * log2_titer
    return np.exp(mu)


def viral_shedding(log10_peak, days_since_infection,
                   eta=1.65, v=0.17, epsilon=0.32):
    """Viral shedding concentration at a given day post-infection.

    Mirrors Immunity::calculate_viral_shedding (immunity.rs:70-79).
    Returns shedding in linear CID50 units.
    """
    days_since_infection = np.asarray(days_since_infection, dtype=float)
    log10_peak = np.asarray(log10_peak, dtype=float)
    # Avoid log(0) at day 0
    t = np.maximum(days_since_infection, 0.5)
    log_t = np.log(t)
    exponent = eta - 0.5 * v**2 - (log_t - eta)**2 / (2.0 * (v + epsilon * log_t)**2)
    concentration = np.power(10.0, log10_peak) * np.exp(exponent) / t
    floor = 10.0**2.6
    return np.maximum(concentration, floor)


def _p_inf_vectorized(log2_titers, doses, strain="WPV"):
    """P(infection) for all (dose, titer) pairs via broadcasting.

    Parameters
    ----------
    log2_titers : array, shape (N,)
    doses : array, shape (D,)

    Returns
    -------
    P_inf : array, shape (D, N) — P_inf[d, n] = P(infection | doses[d], titers[n])
    """
    sabin_scale = STRAIN_PARAMS[strain]["sabin_scale"]
    take_mod = STRAIN_PARAMS[strain]["take_mod"]
    alpha = P_TRANSMIT["alpha"]
    gamma = P_TRANSMIT["gamma"]

    immunity = np.power(2.0, log2_titers)[np.newaxis, :]   # (1, N)
    d = doses[:, np.newaxis]                                 # (D, 1)
    return (1.0 - np.power(1.0 + d / sabin_scale,
                           -alpha * np.power(immunity, -gamma))) * take_mod


def expected_secondary_infections(source_peaks, source_durations,
                                  target_titers, fecal_oral_dose,
                                  total_beta, strain="WPV"):
    """Expected secondary infections per source, against the full target population.

    For each source, integrates daily P(infection) over their shedding curve
    against every target individual, then sums to get total expected infections.

    All inner operations are vectorized with numpy broadcasting:
      doses[d] × target_titers[n] → P_inf[d,n] via broadcasting
      sum over days → integrated_p_per_target[n]
      mean over targets × total_beta × N = R_source

    Parameters
    ----------
    source_peaks : array, shape (S,) — log10 peak CID50 per source
    source_durations : array, shape (S,) — median shed duration per source
    target_titers : array, shape (N,) — log2 titers of all targets
    fecal_oral_dose : float
    total_beta : float — total daily contact rate (sum of all levels)
    strain : str

    Returns
    -------
    R_per_source : array, shape (S,) — expected secondary infections per source
    integrated_p_per_source : array, shape (S, N) — per source-target pair
    """
    S = len(source_peaks)
    N = len(target_titers)

    R_per_source = np.zeros(S)
    integrated_p_per_source = np.zeros((S, N))

    # Group sources by duration to batch the computation
    # (sources with same duration share the same day grid)
    dur_to_sources = {}
    for s in range(S):
        dur = max(2, int(source_durations[s]) + 1)
        dur_to_sources.setdefault(dur, []).append(s)

    for dur, source_indices in dur_to_sources.items():
        days = np.arange(1, dur)  # shape (D,)
        src_idx = np.array(source_indices)
        peaks = source_peaks[src_idx]  # shape (batch,)

        # Compute shedding curves for all sources in this batch
        # viral_shedding broadcasts over peaks: peaks[:, None] × days[None, :]
        # → shed shape (batch, D)
        t = days[np.newaxis, :]  # (1, D)
        pk = peaks[:, np.newaxis]  # (batch, 1)
        log_t = np.log(t)
        eta, v, epsilon = 1.65, 0.17, 0.32
        exponent = eta - 0.5*v**2 - (log_t - eta)**2 / (2.0*(v + epsilon*log_t)**2)
        shed = np.power(10.0, pk) * np.exp(exponent) / t  # (batch, D)
        shed = np.maximum(shed, 10.0**2.6)

        doses_all = shed * fecal_oral_dose  # (batch, D)

        # For each source in the batch, compute P_inf against all targets
        for local_idx, s in enumerate(source_indices):
            doses = doses_all[local_idx]  # (D,)
            # Broadcasting: P_inf shape (D, N)
            p_matrix = _p_inf_vectorized(target_titers, doses, strain=strain)
            # Sum over days → (N,)
            ip = p_matrix.sum(axis=0)
            integrated_p_per_source[s] = ip
            # Expected secondary infections: beta contacts/day, each is a random
            # draw from the target population
            R_per_source[s] = total_beta * np.mean(ip)

    return R_per_source, integrated_p_per_source


def next_generation_matrix(log2_titers, ages_months, fecal_oral_dose,
                           beta_hh=3.0, beta_neighborhood=1.0, beta_village=0.5,
                           titer_bin_edges=None, strain="WPV",
                           household_ids=None):
    """Compute the next-generation matrix K[i,j] in discretized titer bins.

    Uses vectorized per-individual computation on both source and target sides,
    then bins the results. This preserves age-titer correlations and the
    full within-bin titer distributions.

    Contact structure:
    - **Household** (beta_hh): contacts drawn from the source's actual household
      members. A naive infant's household contacts are mostly immune parents.
    - **Community** (beta_neighborhood + beta_village): contacts drawn randomly
      from the full population, where P(target in bin j) = f_j.

    If household_ids is provided, household mixing uses actual household
    composition. Otherwise falls back to random mixing for all contacts.
    """
    log2_titers = np.asarray(log2_titers, dtype=float)
    ages_months = np.asarray(ages_months, dtype=float)
    N = len(log2_titers)

    if titer_bin_edges is None:
        titer_bin_edges = np.array([0, 1, 2, 3, 4, 5, 6, 8, 10, 13])
    titer_bin_edges = np.asarray(titer_bin_edges, dtype=float)

    n_bins = len(titer_bin_edges) - 1
    bin_centers = 0.5 * (titer_bin_edges[:-1] + titer_bin_edges[1:])

    # Assign individuals to bins
    bin_idx = np.clip(np.digitize(log2_titers, titer_bin_edges) - 1, 0, n_bins - 1)
    bin_members = {b: np.where(bin_idx == b)[0] for b in range(n_bins)}
    bin_fractions = np.array([len(bin_members[b]) / N for b in range(n_bins)])

    beta_community = beta_neighborhood + beta_village

    # Per-individual source properties
    all_peaks = log10_peak_cid50(ages_months, log2_titers)
    all_durations = median_shed_duration(log2_titers)

    # Per-bin diagnostics
    bin_mean_log10_peak_shed = np.zeros(n_bins)
    bin_mean_shed_duration = np.zeros(n_bins)
    for b in range(n_bins):
        idx = bin_members[b]
        if len(idx) > 0:
            bin_mean_log10_peak_shed[b] = np.mean(all_peaks[idx])
            bin_mean_shed_duration[b] = np.mean(all_durations[idx])

    # Compute per-individual integrated P against all targets
    # (using total_beta=1 to get raw ip; we'll apply betas separately)
    _, ip_per_individual = expected_secondary_infections(
        all_peaks, all_durations, log2_titers, fecal_oral_dose,
        total_beta=1.0, strain=strain,
    )
    # ip_per_individual[s,t] = sum_days P_inf(dose_s(day), titer_t)

    # Build household membership: for each individual, indices of housemates
    if household_ids is not None:
        household_ids = np.asarray(household_ids)
        hh_members = {}  # source_idx → array of housemate indices (excluding self)
        for hh_id in np.unique(household_ids):
            members = np.where(household_ids == hh_id)[0]
            for s in members:
                hh_members[s] = members[members != s]
    else:
        hh_members = None

    # Per-individual R, split by household vs community
    R_hh = np.zeros(N)
    R_community = np.zeros(N)
    for s in range(N):
        # Community: random contacts from full population
        R_community[s] = beta_community * np.mean(ip_per_individual[s, :])
        # Household: contacts from actual housemates
        if hh_members is not None and s in hh_members and len(hh_members[s]) > 0:
            hm = hh_members[s]
            R_hh[s] = beta_hh * np.mean(ip_per_individual[s, hm])
        else:
            # Fallback: random mixing
            R_hh[s] = beta_hh * np.mean(ip_per_individual[s, :])

    R_per_individual = R_hh + R_community

    # Bin into K[i,j], keeping household and community components separate
    integrated_p = np.zeros((n_bins, n_bins))
    K = np.zeros((n_bins, n_bins))
    K_hh = np.zeros((n_bins, n_bins))
    K_community = np.zeros((n_bins, n_bins))

    for i in range(n_bins):
        src_idx = bin_members[i]
        if len(src_idx) == 0:
            continue
        for j in range(n_bins):
            tgt_idx = bin_members[j]
            if len(tgt_idx) == 0:
                continue

            # Mean integrated P (for diagnostics)
            mean_ip = np.mean(ip_per_individual[np.ix_(src_idx, tgt_idx)])
            integrated_p[i, j] = mean_ip

            # Community component: random mixing
            K_community[i, j] = beta_community * bin_fractions[j] * mean_ip

            # Household component: actual household composition
            if hh_members is not None:
                hh_contributions = []
                for s in src_idx:
                    hm = hh_members.get(s, np.array([], dtype=int))
                    if len(hm) == 0:
                        continue
                    # Which housemates are in target bin j?
                    hm_in_j = hm[bin_idx[hm] == j]
                    if len(hm_in_j) > 0:
                        # Fraction of housemates in bin j × mean ip to them
                        frac_hm_in_j = len(hm_in_j) / len(hm)
                        mean_ip_hm_j = np.mean(ip_per_individual[s, hm_in_j])
                        hh_contributions.append(beta_hh * frac_hm_in_j * mean_ip_hm_j)
                    else:
                        hh_contributions.append(0.0)
                K_hh[i, j] = np.mean(hh_contributions) if hh_contributions else 0.0
            else:
                K_hh[i, j] = beta_hh * bin_fractions[j] * mean_ip

            K[i, j] = K_hh[i, j] + K_community[i, j]

    # Dominant eigenvalue = R0
    eigenvalues = np.linalg.eigvals(K)
    R0 = float(np.max(np.real(eigenvalues)))

    # Dominant eigenvector
    eigvals, eigvecs = np.linalg.eig(K)
    dom_idx = np.argmax(np.real(eigvals))
    dom_eigvec = np.real(eigvecs[:, dom_idx])
    if dom_eigvec.sum() != 0:
        dom_eigvec = dom_eigvec / dom_eigvec.sum()

    return {
        "K": K,
        "K_hh": K_hh,
        "K_community": K_community,
        "R0": R0,
        "bin_edges": titer_bin_edges,
        "bin_centers": bin_centers,
        "bin_fractions": bin_fractions,
        "bin_mean_log10_peak_shed": bin_mean_log10_peak_shed,
        "bin_mean_shed_duration": bin_mean_shed_duration,
        "integrated_p": integrated_p,
        "eigenvector": dom_eigvec,
        # Per-individual results for diagnostics
        "R_per_individual": R_per_individual,
        "R_hh": R_hh,
        "R_community": R_community,
        "ip_per_individual": ip_per_individual,
    }


def analytical_R0(log2_titers, ages_months, fecal_oral_dose,
                  beta_hh=3.0, beta_neighborhood=1.0, beta_village=0.5,
                  strain="WPV", household_ids=None, **kwargs):
    """R0 via dominant eigenvalue of the next-generation matrix."""
    result = next_generation_matrix(
        log2_titers, ages_months, fecal_oral_dose,
        beta_hh=beta_hh, beta_neighborhood=beta_neighborhood,
        beta_village=beta_village, strain=strain,
        household_ids=household_ids,
    )
    return result["R0"]


def mean_susceptibility(log2_titers, dose, strain="WPV"):
    """Average P(infection | random exposure at given dose) across population."""
    log2_titers = np.asarray(log2_titers, dtype=float)
    return float(np.mean(p_infection(log2_titers, dose, strain=strain)))
