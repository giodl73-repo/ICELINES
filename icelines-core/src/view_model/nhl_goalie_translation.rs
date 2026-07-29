//! Career-paired AHL-to-NHL goalie translation for missing NHL samples.
//!
//! This is an evaluation fallback, not a universal equivalency table. The
//! translation is fitted from frozen player-season pairs in the supplied
//! career cohort and must never replace observed NHL goalie production.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const NHL_GOALIE_TRANSLATION_POLICY_SCHEMA: &str = "nhl_goalie_translation_policy.v1";
pub const NHL_GOALIE_TRANSLATION_METHOD: &str = "career_paired_ahl_to_nhl_goalie.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NhlGoalieTranslationPolicy {
    pub schema: String,
    pub method_version: String,
    pub maximum_pair_season_gap: u32,
    pub maximum_source_lookback_seasons: u32,
    pub minimum_calibration_pairs: usize,
    pub minimum_calibration_players: usize,
    pub minimum_pair_shots: u32,
    pub minimum_calibration_shots: u64,
    pub minimum_source_games: u32,
    pub minimum_source_shots: u32,
    pub calibration_error_scale: f64,
    pub prior_shots: u32,
    pub prior_save_percentage: f64,
}

impl Default for NhlGoalieTranslationPolicy {
    fn default() -> Self {
        Self {
            schema: NHL_GOALIE_TRANSLATION_POLICY_SCHEMA.to_owned(),
            method_version: NHL_GOALIE_TRANSLATION_METHOD.to_owned(),
            maximum_pair_season_gap: 1,
            maximum_source_lookback_seasons: 2,
            minimum_calibration_pairs: 20,
            minimum_calibration_players: 15,
            minimum_pair_shots: 100,
            minimum_calibration_shots: 5_000,
            minimum_source_games: 5,
            minimum_source_shots: 100,
            calibration_error_scale: 0.02,
            prior_shots: 500,
            prior_save_percentage: 0.900,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NhlGoalieTranslationPair {
    pub player_id: u32,
    pub ahl_season: u32,
    pub nhl_season: u32,
    pub ahl_save_percentage: f64,
    pub nhl_save_percentage: f64,
    pub paired_shots: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NhlGoalieTranslationCalibration {
    pub method_version: String,
    pub save_percentage_delta: f64,
    pub rmse: f64,
    pub pair_count: usize,
    pub unique_players: usize,
    pub paired_shots: u64,
    pub sample_confidence: f64,
    pub fit_confidence: f64,
    pub calibration_confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NhlGoalieTranslationEstimate {
    pub method_version: String,
    pub source_league: String,
    pub source_season: u32,
    pub source_games: u32,
    pub source_shots: u32,
    pub source_save_percentage: f64,
    pub translated_nhl_save_percentage: f64,
    pub effective_games: u32,
    pub effective_shots: u32,
    pub prior_save_percentage: f64,
    pub shrunk_nhl_save_percentage: f64,
    pub goalie_quality_score: f64,
    pub evidence_confidence: f64,
    pub calibration: NhlGoalieTranslationCalibration,
}

pub fn calibrate_nhl_goalie_translation(
    policy: &NhlGoalieTranslationPolicy,
    pairs: &[NhlGoalieTranslationPair],
) -> Result<NhlGoalieTranslationCalibration, String> {
    validate_nhl_goalie_translation_policy(policy)?;
    let mut players = BTreeSet::new();
    let mut paired_shots = 0_u64;
    for pair in pairs {
        if pair.paired_shots < policy.minimum_pair_shots
            || !(0.0..=1.0).contains(&pair.ahl_save_percentage)
            || !(0.0..=1.0).contains(&pair.nhl_save_percentage)
            || pair.ahl_season > pair.nhl_season
            || season_gap(pair.ahl_season, pair.nhl_season)? > policy.maximum_pair_season_gap
        {
            return Err("invalid AHL-to-NHL goalie calibration pair".to_owned());
        }
        players.insert(pair.player_id);
        paired_shots = paired_shots
            .checked_add(u64::from(pair.paired_shots))
            .ok_or_else(|| "goalie calibration workload overflow".to_owned())?;
    }
    if pairs.len() < policy.minimum_calibration_pairs
        || players.len() < policy.minimum_calibration_players
        || paired_shots < policy.minimum_calibration_shots
    {
        return Err(format!(
            "unsupported AHL-to-NHL goalie cohort: {} pairs, {} players, {} shots",
            pairs.len(),
            players.len(),
            paired_shots
        ));
    }
    let save_percentage_delta = pairs
        .iter()
        .map(|pair| {
            f64::from(pair.paired_shots) * (pair.nhl_save_percentage - pair.ahl_save_percentage)
        })
        .sum::<f64>()
        / paired_shots as f64;
    let rmse = (pairs
        .iter()
        .map(|pair| {
            let error = pair.ahl_save_percentage + save_percentage_delta - pair.nhl_save_percentage;
            f64::from(pair.paired_shots) * error.powi(2)
        })
        .sum::<f64>()
        / paired_shots as f64)
        .sqrt();
    let sample_confidence =
        players.len() as f64 / (players.len() + policy.minimum_calibration_players) as f64;
    let fit_confidence = policy.calibration_error_scale / (policy.calibration_error_scale + rmse);
    Ok(NhlGoalieTranslationCalibration {
        method_version: policy.method_version.clone(),
        save_percentage_delta,
        rmse,
        pair_count: pairs.len(),
        unique_players: players.len(),
        paired_shots,
        sample_confidence,
        fit_confidence,
        calibration_confidence: sample_confidence * fit_confidence,
    })
}

pub fn estimate_nhl_goalie_quality(
    policy: &NhlGoalieTranslationPolicy,
    calibration: &NhlGoalieTranslationCalibration,
    source_season: u32,
    source_games: u32,
    source_shots: u32,
    source_save_percentage: f64,
) -> Result<NhlGoalieTranslationEstimate, String> {
    validate_nhl_goalie_translation_policy(policy)?;
    if calibration.method_version != policy.method_version
        || source_games < policy.minimum_source_games
        || source_shots < policy.minimum_source_shots
        || !(0.0..=1.0).contains(&source_save_percentage)
    {
        return Err("insufficient AHL goalie translation input".to_owned());
    }
    let translated = source_save_percentage + calibration.save_percentage_delta;
    if !(0.0..=1.0).contains(&translated) {
        return Err("translated NHL save percentage is out of range".to_owned());
    }
    let effective_shots =
        ((f64::from(source_shots) * calibration.calibration_confidence).round() as u32).max(1);
    let effective_games =
        ((f64::from(source_games) * calibration.calibration_confidence).round() as u32).max(1);
    let total_shots = effective_shots
        .checked_add(policy.prior_shots)
        .ok_or_else(|| "goalie estimate workload overflow".to_owned())?;
    let shrunk = (translated * f64::from(effective_shots)
        + policy.prior_save_percentage * f64::from(policy.prior_shots))
        / f64::from(total_shots);
    let goalie_quality_score =
        (50.0 + (shrunk - 0.900) * 1_000.0 + f64::from(effective_games.min(50)) * 0.25)
            .clamp(0.0, 100.0);
    let evidence_confidence =
        calibration.calibration_confidence * f64::from(effective_shots) / f64::from(total_shots);
    Ok(NhlGoalieTranslationEstimate {
        method_version: policy.method_version.clone(),
        source_league: "AHL".to_owned(),
        source_season,
        source_games,
        source_shots,
        source_save_percentage,
        translated_nhl_save_percentage: translated,
        effective_games,
        effective_shots,
        prior_save_percentage: policy.prior_save_percentage,
        shrunk_nhl_save_percentage: shrunk,
        goalie_quality_score,
        evidence_confidence,
        calibration: calibration.clone(),
    })
}

pub fn validate_nhl_goalie_translation_policy(
    policy: &NhlGoalieTranslationPolicy,
) -> Result<(), String> {
    if policy.schema != NHL_GOALIE_TRANSLATION_POLICY_SCHEMA
        || policy.method_version != NHL_GOALIE_TRANSLATION_METHOD
        || policy.maximum_pair_season_gap > 1
        || policy.maximum_source_lookback_seasons == 0
        || policy.minimum_calibration_pairs == 0
        || policy.minimum_calibration_players == 0
        || policy.minimum_pair_shots == 0
        || policy.minimum_calibration_shots == 0
        || policy.minimum_source_games == 0
        || policy.minimum_source_shots == 0
        || !policy.calibration_error_scale.is_finite()
        || policy.calibration_error_scale <= 0.0
        || policy.prior_shots == 0
        || !policy.prior_save_percentage.is_finite()
        || !(0.0..=1.0).contains(&policy.prior_save_percentage)
    {
        return Err("invalid NHL goalie translation policy".to_owned());
    }
    Ok(())
}

fn season_gap(earlier: u32, later: u32) -> Result<u32, String> {
    (later / 10_000)
        .checked_sub(earlier / 10_000)
        .ok_or_else(|| "career seasons are not monotonic".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(count: u32, ahl: f64, nhl: f64) -> Vec<NhlGoalieTranslationPair> {
        (0..count)
            .map(|index| NhlGoalieTranslationPair {
                player_id: index + 1,
                ahl_season: 20242025,
                nhl_season: 20252026,
                ahl_save_percentage: ahl,
                nhl_save_percentage: nhl,
                paired_shots: 400,
            })
            .collect()
    }

    #[test]
    fn l0_calibration_is_workload_weighted_and_confidence_bounded() {
        let calibration = calibrate_nhl_goalie_translation(
            &NhlGoalieTranslationPolicy::default(),
            &pairs(20, 0.915, 0.905),
        )
        .unwrap();
        assert!((calibration.save_percentage_delta + 0.010).abs() < 1e-9);
        assert_eq!(calibration.pair_count, 20);
        assert!((0.0..=1.0).contains(&calibration.calibration_confidence));
    }

    #[test]
    fn l0_estimate_shrinks_and_never_copies_ahl_score_directly() {
        let policy = NhlGoalieTranslationPolicy::default();
        let calibration =
            calibrate_nhl_goalie_translation(&policy, &pairs(20, 0.915, 0.905)).unwrap();
        let estimate =
            estimate_nhl_goalie_quality(&policy, &calibration, 20252026, 30, 900, 0.920).unwrap();
        assert!((0.900..0.910).contains(&estimate.shrunk_nhl_save_percentage));
        assert!((50.0..70.0).contains(&estimate.goalie_quality_score));
        assert!(estimate.evidence_confidence < 1.0);
    }

    #[test]
    fn l0_weak_cohort_and_short_candidate_are_refused() {
        let policy = NhlGoalieTranslationPolicy::default();
        assert!(calibrate_nhl_goalie_translation(&policy, &pairs(10, 0.915, 0.905)).is_err());
        let calibration =
            calibrate_nhl_goalie_translation(&policy, &pairs(20, 0.915, 0.905)).unwrap();
        assert!(
            estimate_nhl_goalie_quality(&policy, &calibration, 20252026, 2, 50, 0.920).is_err()
        );
    }
}
