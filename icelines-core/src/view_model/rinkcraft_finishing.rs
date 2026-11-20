//! Fixed-prior, conditional finishing replay. No prior fitting or capability grades.
use serde::{Deserialize, Serialize};
use statrs::distribution::{ContinuousCDF, Gamma};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    C,
    L,
    R,
    D,
}
impl Position {
    pub(super) fn factor(self, m: &Model) -> f64 {
        if self == Self::D {
            m.defense_factor
        } else {
            m.forward_factor
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub calibration_season: u32,
    pub shape: f64,
    pub rate: f64,
    pub forward_factor: f64,
    pub defense_factor: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryRow {
    pub player_id: u32,
    pub season: u32,
    pub position: Position,
    pub goals: u32,
    pub xg: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub player_id: u32,
    pub season: u32,
    pub position: Position,
    pub xg: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GammaPosterior {
    pub shape: f64,
    /// Rate parameter, never scale.
    pub rate: f64,
    pub mean: f64,
    pub variance: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredibleInterval {
    pub probability: f64,
    pub lower: f64,
    pub upper: f64,
    pub lower_cdf_probability: f64,
    pub upper_survival_probability: f64,
    pub interpretation: String,
}
// Work at unit rate; invert the small tail directly rather than subtracting from
// a rounded CDF near one. Statrs supplies the special-function evaluation.
fn gamma_tail_quantile(gamma: &Gamma, tail: f64, upper: bool) -> Result<f64, String> {
    let probability = |x| if upper { gamma.sf(x) } else { gamma.cdf(x) };
    let needs_larger = |p: f64| if upper { p > tail } else { p < tail };
    let mut low = 0.;
    let mut high = gamma.shape().max(1.);
    let mut bracketed = false;
    for _ in 0..128 {
        let p = probability(high);
        if !p.is_finite() || !(0. ..=1.).contains(&p) {
            return Err("Invalid Gamma tail evaluation".into());
        }
        if !needs_larger(p) {
            bracketed = true;
            break;
        }
        high *= 2.;
        if !high.is_finite() {
            break;
        }
    }
    if !bracketed {
        return Err("Gamma tail could not be bracketed".into());
    }
    for _ in 0..256 {
        let mid = low + (high - low) / 2.;
        if mid == low || mid == high || high - low <= 4e-14 * mid {
            return Ok(mid);
        }
        let p = probability(mid);
        if !p.is_finite() || !(0. ..=1.).contains(&p) {
            return Err("Invalid Gamma tail evaluation".into());
        }
        if needs_larger(p) {
            low = mid;
        } else {
            high = mid;
        }
    }
    Err("Gamma tail inversion did not converge".into())
}
/// Equal-tailed interval for the multiplier, not a goal-count prediction interval.
pub fn credible_interval(p: &GammaPosterior, probability: f64) -> Result<CredibleInterval, String> {
    if !probability.is_finite() || probability <= 0. || probability >= 1. {
        return Err("Credible probability must be strictly between zero and one".into());
    }
    let tail = (1. - probability) / 2.;
    let upper_probability = 1. - tail;
    if tail <= 0. || upper_probability >= 1. || tail >= upper_probability {
        return Err("Unrepresentable equal-tail probabilities".into());
    }
    let gamma = Gamma::new(p.shape, p.rate).map_err(|e| e.to_string())?;
    let unit = Gamma::new(p.shape, 1.).map_err(|e| e.to_string())?;
    let lower = gamma_tail_quantile(&unit, tail, false)? / p.rate;
    let upper = gamma_tail_quantile(&unit, tail, true)? / p.rate;
    if !positive(lower) || !positive(upper) || lower >= upper {
        return Err("Nonfinite, underflowed or unordered Gamma quantiles".into());
    }
    let lower_cdf = gamma.cdf(lower);
    let upper_sf = gamma.sf(upper);
    let tolerance = 2e-12_f64.max(2e-9 * tail);
    if !lower_cdf.is_finite()
        || !upper_sf.is_finite()
        || (lower_cdf - tail).abs() > tolerance
        || (upper_sf - tail).abs() > tolerance
    {
        return Err("Gamma quantiles failed tail-probability verification".into());
    }
    Ok(CredibleInterval { probability, lower, upper, lower_cdf_probability:lower_cdf,
        upper_survival_probability:upper_sf,
        interpretation:"Equal-tailed conditional Gamma multiplier interval, not a goal-count prediction interval, all-source uncertainty range or capability grade. Frozen prior/position factors and provider xG are treated as fixed.".into() })
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policies {
    pub global_baseline: f64,
    pub global_player: f64,
    pub position_baseline: f64,
    pub position_player: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub policies: Policies,
    pub global_multiplier: GammaPosterior,
    pub position_residual: GammaPosterior,
    pub position_multiplier: GammaPosterior,
    pub global_player_predictive_variance: f64,
    pub position_player_predictive_variance: f64,
    pub historical_rows: usize,
    pub position_group_switch: bool,
    pub headline_scores_released: bool,
    pub interval_quantiles_computed: bool,
    pub uncertainty_scope: String,
}
fn season(s: u32) -> bool {
    (1900..=2200).contains(&(s / 10000)) && s % 10000 == s / 10000 + 1
}
fn positive(x: f64) -> bool {
    x.is_finite() && x > 0.
}
fn nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.
}
fn posterior(shape: f64, rate: f64) -> Result<GammaPosterior, String> {
    let mean = shape / rate;
    let variance = mean / rate;
    if !positive(shape) || !positive(rate) || !positive(mean) || !positive(variance) {
        return Err("Nonfinite, underflowed or invalid Gamma posterior".into());
    }
    Ok(GammaPosterior {
        shape,
        rate,
        mean,
        variance,
    })
}
/// Conditional Gamma-Poisson update for one declared observation window.
/// The caller owns provenance/chronology; this is not a capability score.
pub fn update_multiplier(
    shape: f64,
    rate: f64,
    goals: u64,
    xg: f64,
) -> Result<GammaPosterior, String> {
    if !positive(shape)
        || !positive(rate)
        || !nonnegative(xg)
        || (xg == 0. && goals > 0)
        || goals > (1_u64 << 53)
    {
        return Err("invalid conditional finishing evidence".into());
    }
    posterior(shape + goals as f64, rate + xg)
}
pub fn predict(m: &Model, target: &Target, history: &[HistoryRow]) -> Result<Prediction, String> {
    if !season(m.calibration_season)
        || ![m.shape, m.rate, m.forward_factor, m.defense_factor]
            .into_iter()
            .all(positive)
    {
        return Err("Invalid frozen model".into());
    }
    if target.player_id == 0
        || !season(target.season)
        || target.season <= m.calibration_season
        || !nonnegative(target.xg)
    {
        return Err("Invalid target or calibration/target chronology".into());
    }
    let mut seen = BTreeSet::new();
    let (mut goals, mut xg, mut adjusted) = (0., 0., 0.);
    for h in history {
        if h.player_id != target.player_id
            || !season(h.season)
            || h.season >= target.season
            || !seen.insert(h.season)
            || !nonnegative(h.xg)
            || (h.xg == 0. && h.goals > 0)
        {
            return Err("Invalid, duplicated, wrong-player or future historical row".into());
        }
        goals += h.goals as f64;
        xg += h.xg;
        adjusted += h.xg * h.position.factor(m);
    }
    let global = posterior(m.shape + goals, m.rate + xg)?;
    let residual = posterior(m.shape + goals, m.shape + adjusted)?;
    let factor = target.position.factor(m);
    let multiplier = posterior(residual.shape, residual.rate / factor)?;
    let policies = Policies {
        global_baseline: m.shape / m.rate * target.xg,
        global_player: global.mean * target.xg,
        position_baseline: factor * target.xg,
        position_player: residual.mean * factor * target.xg,
    };
    let global_variance = policies.global_player + target.xg * (target.xg * global.variance);
    let position_variance =
        policies.position_player + target.xg * (target.xg * multiplier.variance);
    if ![
        policies.global_baseline,
        policies.global_player,
        policies.position_baseline,
        policies.position_player,
        global_variance,
        position_variance,
    ]
    .into_iter()
    .all(nonnegative)
    {
        return Err("Nonfinite prediction or predictive variance".into());
    }
    Ok(Prediction{policies,global_multiplier:global,position_residual:residual,position_multiplier:multiplier,
        global_player_predictive_variance:global_variance,position_player_predictive_variance:position_variance,
        historical_rows:history.len(),position_group_switch:history.iter().any(|h|(h.position==Position::D)!=(target.position==Position::D)),
        headline_scores_released:false,interval_quantiles_computed:false,
        uncertainty_scope:"Conditional on supplied frozen prior, position factors, history and realized target xG. Gamma parameters/variance are not interval quantiles. Predictive variance includes Poisson process variation and conditional multiplier uncertainty, excluding prior/factor estimation, provider-model error, context confounding and game/team dependence. No preseason forecast or complete Shooting assessment.".into()})
}
/// None is an infinite deviance for positive goals with zero expectation, never an epsilon repair.
pub fn poisson_deviance(goals: u32, expected: f64) -> Result<Option<f64>, String> {
    if !nonnegative(expected) {
        return Err("Invalid expected goals".into());
    }
    if expected == 0. {
        return Ok(if goals == 0 { Some(0.) } else { None });
    }
    let g = goals as f64;
    let d = if goals == 0 {
        2. * expected
    } else {
        2. * (g * (g.ln() - expected.ln()) - (g - expected))
    };
    if !d.is_finite() {
        return Err("Nonfinite deviance".into());
    }
    Ok(Some(d.max(0.)))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn model() -> Model {
        Model {
            calibration_season: 20232024,
            shape: 4.,
            rate: 2.,
            forward_factor: 1.,
            defense_factor: 2.,
        }
    }
    fn target() -> Target {
        Target {
            player_id: 1,
            season: 20252026,
            position: Position::D,
            xg: 3.,
        }
    }
    fn row() -> HistoryRow {
        HistoryRow {
            player_id: 1,
            season: 20242025,
            position: Position::C,
            goals: 2,
            xg: 4.,
        }
    }
    #[test]
    fn closed_form_and_predictive_uncertainty() {
        let p = predict(&model(), &target(), &[row()]).unwrap();
        assert_eq!(p.policies.global_player, 3.);
        assert_eq!(p.policies.position_player, 4.5);
        assert_eq!(p.global_multiplier.variance, 1. / 6.);
        assert_eq!(p.global_player_predictive_variance, 4.5);
        assert_eq!(p.position_residual.variance, 6. / 64.);
        assert_eq!(p.position_player_predictive_variance, 7.875);
        assert!(p.position_group_switch);
        assert!(!p.headline_scores_released);
        assert!(!p.interval_quantiles_computed);
    }
    #[test]
    fn no_history_returns_baselines_and_zero_target_does_not_prove_skill() {
        let p = predict(&model(), &target(), &[]).unwrap();
        assert_eq!(p.policies.global_baseline, p.policies.global_player);
        assert_eq!(p.policies.position_baseline, p.policies.position_player);
        let mut t = target();
        t.xg = 0.;
        let p = predict(&model(), &t, &[]).unwrap();
        assert_eq!(p.policies.position_player, 0.);
        assert_eq!(p.position_player_predictive_variance, 0.);
        assert!(p.position_multiplier.variance > 0.);
    }
    #[test]
    fn rejects_future_duplicate_and_wrong_player_evidence() {
        let mut h = row();
        h.season = target().season;
        assert!(predict(&model(), &target(), &[h]).is_err());
        assert!(predict(&model(), &target(), &[row(), row()]).is_err());
        let mut h = row();
        h.player_id = 2;
        assert!(predict(&model(), &target(), &[h]).is_err());
    }
    #[test]
    fn each_historical_position_sets_its_own_adjusted_exposure() {
        let mut h = row();
        h.position = Position::D;
        let p = predict(&model(), &target(), &[h]).unwrap();
        assert_eq!(p.policies.position_player, 3.);
        assert!(!p.position_group_switch);
    }
    #[test]
    fn rejects_invalid_exposure_model_and_chronology() {
        for x in [-1., f64::NAN, f64::INFINITY] {
            let mut t = target();
            t.xg = x;
            assert!(predict(&model(), &t, &[]).is_err());
        }
        let mut h = row();
        h.xg = 0.;
        assert!(predict(&model(), &target(), &[h]).is_err());
        let mut m = model();
        m.rate = 0.;
        assert!(predict(&m, &target(), &[]).is_err());
        let mut t = target();
        t.season = 20232024;
        assert!(predict(&model(), &t, &[]).is_err());
        assert!(serde_json::from_str::<Target>(
            r#"{"player_id":1,"season":20252026,"position":"G","xg":1}"#
        )
        .is_err());
    }
    #[test]
    fn zero_and_extreme_deviance_are_explicit() {
        assert_eq!(poisson_deviance(0, 0.).unwrap(), Some(0.));
        assert_eq!(poisson_deviance(1, 0.).unwrap(), None);
        assert_eq!(poisson_deviance(0, 2.).unwrap(), Some(4.));
        assert_eq!(poisson_deviance(2, 2.).unwrap(), Some(0.));
        assert!(poisson_deviance(1, 1e-320).unwrap().unwrap().is_finite());
        assert!(poisson_deviance(0, -1.).is_err());
    }
    #[test]
    fn gamma_interval_matches_exponential_closed_form_and_rate_scaling() {
        let p = posterior(1., 2.).unwrap();
        let i = credible_interval(&p, 0.9).unwrap();
        assert!((i.lower - (-0.95_f64.ln() / 2.)).abs() < 1e-10);
        assert!((i.upper - (-0.05_f64.ln() / 2.)).abs() < 1e-10);
        let half = credible_interval(&posterior(1., 4.).unwrap(), 0.9).unwrap();
        assert!((half.lower * 2. - i.lower).abs() < 1e-10);
        assert!((half.upper * 2. - i.upper).abs() < 1e-10);
        let narrow = credible_interval(&p, 0.5).unwrap();
        assert!(narrow.lower > i.lower && narrow.upper < i.upper);
    }
    #[test]
    fn gamma_interval_rejects_invalid_or_unrepresentable_probability() {
        let p = posterior(4., 2.).unwrap();
        for mass in [0., 1., -1., f64::NAN, f64::INFINITY, f64::MIN_POSITIVE] {
            assert!(credible_interval(&p, mass).is_err());
        }
        let mut invalid = p;
        invalid.rate = 0.;
        assert!(credible_interval(&invalid, 0.9).is_err());
    }
}
