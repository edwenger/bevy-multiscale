use bevy::prelude::*;
use rand::Rng;
use rand_distr::{LogNormal, Normal, Distribution};
use log::debug;
use super::params::*;
use super::infection::{InfectionStrain, InfectionSerotype, Infection};

#[derive(Component)]
pub struct Immunity {
    pub prechallenge_immunity: f32,
    pub postchallenge_peak_immunity: f32,
    pub current_immunity: f32,
    pub ti_infected: Option<f32>,
}

impl Default for Immunity {
    fn default() -> Self {
        Self {
            prechallenge_immunity: 1.0,
            postchallenge_peak_immunity: 0.0,
            current_immunity: 1.0,
            ti_infected: None,
        }
    }
}

impl Immunity {
    pub fn with_titer(titer: f32) -> Self {
        Self {
            prechallenge_immunity: titer,
            postchallenge_peak_immunity: titer,
            current_immunity: titer,
            ti_infected: None,
        }
    }

    pub fn calculate_theta_nab(&self, theta_nabs: &ThetaNabsParams, rng: &mut impl Rng) -> f32 {
        let nabs = self.prechallenge_immunity;
        let mean = theta_nabs.a + theta_nabs.b * nabs.log2();
        let stdev = (theta_nabs.c + theta_nabs.d * nabs.log2()).max(0.0).sqrt();
        let normal_dist = Normal::new(mean, stdev).unwrap();
        normal_dist.sample(rng).exp()
    }

    pub fn update_peak_immunity(&mut self, theta_nabs: &ThetaNabsParams, rng: &mut impl Rng) {
        self.prechallenge_immunity = self.current_immunity;
        let theta_nabs_value = self.calculate_theta_nab(theta_nabs, rng);
        self.postchallenge_peak_immunity = self.prechallenge_immunity * theta_nabs_value.max(1.0);
        self.current_immunity = self.postchallenge_peak_immunity.max(1.0);
        debug!("Updated current immunity: {}", self.current_immunity);
    }

    pub fn calculate_waning(&mut self, t_since_last_exposure: f32, immunity_waning: &ImmunityWaningParams) {
        if t_since_last_exposure >= 30.0 {
            self.current_immunity = (self.postchallenge_peak_immunity
                * (t_since_last_exposure / 30.0).powf(-immunity_waning.rate)).max(1.0);
        }
    }

    pub fn calculate_shed_duration(&self, shed_duration: &ShedDurationParams, rng: &mut impl Rng) -> f32 {
        let u = shed_duration.u;
        let delta = shed_duration.delta;
        let sigma = shed_duration.sigma;
        let mu = u.ln() - delta.ln() * self.prechallenge_immunity.log2();
        let std = sigma.ln();
        let log_normal_dist = LogNormal::new(mu, std).unwrap();
        log_normal_dist.sample(rng)
    }

    pub fn calculate_viral_shedding(&self, age_in_months: f32, days_since_infection: f32, params: &DiseaseParams) -> f32 {
        let log10_peak_cid50 = self.calculate_log10_peak_cid50(age_in_months, &params.peak_cid50);
        let log_t_inf = days_since_infection.ln();
        let eta = params.viral_shedding.eta;
        let v = params.viral_shedding.v;
        let epsilon = params.viral_shedding.epsilon;
        let exponent = eta - (0.5 * v.powi(2)) - ((log_t_inf - eta).powi(2)) / (2.0 * (v + epsilon * log_t_inf).powi(2));
        let predicted_concentration = 10f32.powf(log10_peak_cid50) * exponent.exp() / days_since_infection;
        predicted_concentration.max(10f32.powf(2.6))
    }

    fn calculate_log10_peak_cid50(&self, age_in_months: f32, peak_cid50_params: &PeakCid50Params) -> f32 {
        let k = peak_cid50_params.k;
        let smax = peak_cid50_params.smax;
        let smin = peak_cid50_params.smin;
        let tau = peak_cid50_params.tau;
        let peak_cid50_naive = if age_in_months >= 6.0 {
            (smax - smin) * ((7.0 - age_in_months) / tau).exp() + smin
        } else {
            smax
        };
        peak_cid50_naive * (1.0 - k * self.prechallenge_immunity.log2())
    }

    pub fn calculate_infection_probability(
        &self,
        dose: f32,
        strain: InfectionStrain,
        serotype: InfectionSerotype,
        params: &DiseaseParams,
    ) -> f32 {
        let (Some(sabin_scale), Some(take_modifier)) = (
            params.sabin_scale_for(strain, serotype),
            params.take_modifier_for(strain, serotype)
        ) else {
            return 0.0;
        };

        let gamma = params.p_transmit.gamma;
        let alpha = params.p_transmit.alpha;

        (1.0 - (1.0 + dose / sabin_scale).powf(-alpha * self.current_immunity.powf(-gamma))) * take_modifier
    }

    pub fn set_infection_prognoses(
        &mut self,
        infection: &mut Infection,
        sim_time: f32,
        params: &DiseaseParams,
        rng: &mut impl Rng,
    ) {
        self.update_peak_immunity(&params.theta_nabs, rng);
        self.ti_infected = Some(sim_time);

        infection.shed_duration = if let Some(shed_params) = params.shed_duration_for(infection.strain, infection.serotype) {
            self.calculate_shed_duration(shed_params, rng)
        } else {
            30.0
        };
    }
}
