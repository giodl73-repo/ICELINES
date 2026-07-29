//! Career-based cross-league value fallback for AHL preseason ordering.
//!
//! This evaluation method is used only when direct prior-season AHL value is
//! unavailable. Translation parameters are fitted from frozen player-season
//! pairs in the supplied career cohort; no universal NHLe table is embedded.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::ahl_player_value::AhlPlayerValuePositionGroup;

pub const AHL_CROSS_LEAGUE_VALUE_POLICY_SCHEMA: &str = "ahl_cross_league_value_policy.v1";
pub const AHL_CROSS_LEAGUE_VALUE_METHOD: &str = "career_paired_ahl_translation.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlCrossLeagueValuePolicy {
    pub schema: String,
    pub method_version: String,
    pub maximum_source_lookback_seasons: u32,
    pub maximum_pair_season_gap: u32,
    pub minimum_calibration_pairs: usize,
    pub minimum_calibration_players: usize,
    pub minimum_calibration_workload: u64,
    pub minimum_skater_pair_games: u32,
    pub minimum_goalie_pair_shots: u32,
    pub minimum_skater_source_games: u32,
    pub minimum_goalie_source_games: u32,
    pub minimum_goalie_source_shots: u32,
    pub skater_calibration_error_scale: f64,
    pub goalie_calibration_error_scale: f64,
    pub skater_schedule_games: u32,
    pub skater_prior_games: u32,
    pub forward_prior_points_per_game: f64,
    pub defense_prior_points_per_game: f64,
    pub goalie_prior_shots: u32,
    pub goalie_prior_save_percentage: f64,
}

impl Default for AhlCrossLeagueValuePolicy {
    fn default() -> Self {
        Self {
            schema: AHL_CROSS_LEAGUE_VALUE_POLICY_SCHEMA.to_owned(),
            method_version: AHL_CROSS_LEAGUE_VALUE_METHOD.to_owned(),
            maximum_source_lookback_seasons: 2,
            maximum_pair_season_gap: 1,
            minimum_calibration_pairs: 30,
            minimum_calibration_players: 20,
            minimum_calibration_workload: 200,
            minimum_skater_pair_games: 5,
            minimum_goalie_pair_shots: 100,
            minimum_skater_source_games: 5,
            minimum_goalie_source_games: 5,
            minimum_goalie_source_shots: 100,
            skater_calibration_error_scale: 0.25,
            goalie_calibration_error_scale: 0.02,
            skater_schedule_games: 72,
            skater_prior_games: 20,
            forward_prior_points_per_game: 0.45,
            defense_prior_points_per_game: 0.25,
            goalie_prior_shots: 400,
            goalie_prior_save_percentage: 0.900,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlCrossLeagueCalibrationPair {
    pub player_id: u32,
    pub source_season: u32,
    pub ahl_season: u32,
    pub source_rate: f64,
    pub ahl_rate: f64,
    pub paired_workload: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlCrossLeagueTranslationKind {
    MultiplicativeRate,
    AdditiveSavePercentage,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlCrossLeagueCalibration {
    pub method_version: String,
    pub source_league: String,
    pub position_group: AhlPlayerValuePositionGroup,
    pub translation_kind: AhlCrossLeagueTranslationKind,
    pub translation_value: f64,
    pub rmse: f64,
    pub pair_count: usize,
    pub unique_players: usize,
    pub paired_workload: u64,
    pub sample_confidence: f64,
    pub fit_confidence: f64,
    pub calibration_confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlCrossLeagueValueEstimate {
    pub method_version: String,
    pub position_group: AhlPlayerValuePositionGroup,
    pub source_league: String,
    pub source_season: u32,
    pub source_games: u32,
    pub source_workload: u32,
    pub source_rate: f64,
    pub translated_ahl_rate: f64,
    pub effective_workload: u32,
    pub prior_rate: f64,
    pub shrunk_rate: f64,
    pub projected_score: f64,
    pub evidence_confidence: f64,
    pub calibration: AhlCrossLeagueCalibration,
}

pub fn calibrate_ahl_cross_league_value(
    policy: &AhlCrossLeagueValuePolicy,
    source_league: &str,
    position_group: AhlPlayerValuePositionGroup,
    pairs: &[AhlCrossLeagueCalibrationPair],
) -> Result<AhlCrossLeagueCalibration, String> {
    validate_ahl_cross_league_value_policy(policy)?;
    let source_league = source_league.trim().to_ascii_uppercase();
    if source_league.is_empty() || matches!(source_league.as_str(), "AHL" | "NHL") {
        return Err("cross-league calibration requires a non-NHL, non-AHL league".to_owned());
    }
    let mut players = BTreeSet::new();
    let mut paired_workload = 0_u64;
    for pair in pairs {
        let minimum_pair_workload = if position_group == AhlPlayerValuePositionGroup::Goalie {
            policy.minimum_goalie_pair_shots
        } else {
            policy.minimum_skater_pair_games
        };
        if pair.paired_workload < minimum_pair_workload
            || !pair.source_rate.is_finite()
            || !pair.ahl_rate.is_finite()
            || pair.source_season > pair.ahl_season
            || season_gap(pair.source_season, pair.ahl_season)? > policy.maximum_pair_season_gap
        {
            return Err("invalid cross-league calibration pair".to_owned());
        }
        match position_group {
            AhlPlayerValuePositionGroup::Goalie
                if !(0.0..=1.0).contains(&pair.source_rate)
                    || !(0.0..=1.0).contains(&pair.ahl_rate) =>
            {
                return Err("goalie calibration rates must be save percentages".to_owned());
            }
            _ if pair.source_rate < 0.0 || pair.ahl_rate < 0.0 => {
                return Err("skater calibration rates cannot be negative".to_owned());
            }
            _ => {}
        }
        players.insert(pair.player_id);
        paired_workload = paired_workload
            .checked_add(u64::from(pair.paired_workload))
            .ok_or_else(|| "cross-league calibration workload overflow".to_owned())?;
    }
    if pairs.len() < policy.minimum_calibration_pairs
        || players.len() < policy.minimum_calibration_players
        || paired_workload < policy.minimum_calibration_workload
    {
        return Err(format!(
            "unsupported cross-league calibration cohort: {} pairs, {} players, {} workload",
            pairs.len(),
            players.len(),
            paired_workload
        ));
    }

    let translation_kind;
    let translation_value;
    if position_group == AhlPlayerValuePositionGroup::Goalie {
        translation_kind = AhlCrossLeagueTranslationKind::AdditiveSavePercentage;
        let weighted_delta = pairs
            .iter()
            .map(|pair| f64::from(pair.paired_workload) * (pair.ahl_rate - pair.source_rate));
        translation_value = weighted_delta.sum::<f64>() / paired_workload as f64;
    } else {
        translation_kind = AhlCrossLeagueTranslationKind::MultiplicativeRate;
        let numerator = pairs
            .iter()
            .map(|pair| f64::from(pair.paired_workload) * pair.source_rate * pair.ahl_rate);
        let denominator = pairs
            .iter()
            .map(|pair| f64::from(pair.paired_workload) * pair.source_rate * pair.source_rate);
        let denominator = denominator.sum::<f64>();
        if denominator <= f64::EPSILON {
            return Err("cross-league calibration has no skater rate variation".to_owned());
        }
        translation_value = numerator.sum::<f64>() / denominator;
    }
    if !translation_value.is_finite()
        || (translation_kind == AhlCrossLeagueTranslationKind::MultiplicativeRate
            && translation_value <= 0.0)
    {
        return Err("cross-league calibration produced an invalid translation".to_owned());
    }
    let squared_error = pairs.iter().map(|pair| {
        let translated = match translation_kind {
            AhlCrossLeagueTranslationKind::MultiplicativeRate => {
                pair.source_rate * translation_value
            }
            AhlCrossLeagueTranslationKind::AdditiveSavePercentage => {
                pair.source_rate + translation_value
            }
        };
        f64::from(pair.paired_workload) * (translated - pair.ahl_rate).powi(2)
    });
    let rmse = (squared_error.sum::<f64>() / paired_workload as f64).sqrt();
    let sample_confidence =
        players.len() as f64 / (players.len() + policy.minimum_calibration_players) as f64;
    let error_scale = if position_group == AhlPlayerValuePositionGroup::Goalie {
        policy.goalie_calibration_error_scale
    } else {
        policy.skater_calibration_error_scale
    };
    let fit_confidence = error_scale / (error_scale + rmse);
    let calibration_confidence = sample_confidence * fit_confidence;
    Ok(AhlCrossLeagueCalibration {
        method_version: policy.method_version.clone(),
        source_league,
        position_group,
        translation_kind,
        translation_value,
        rmse,
        pair_count: pairs.len(),
        unique_players: players.len(),
        paired_workload,
        sample_confidence,
        fit_confidence,
        calibration_confidence,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn estimate_ahl_cross_league_value(
    policy: &AhlCrossLeagueValuePolicy,
    calibration: &AhlCrossLeagueCalibration,
    source_season: u32,
    source_games: u32,
    source_workload: u32,
    source_rate: f64,
) -> Result<AhlCrossLeagueValueEstimate, String> {
    validate_ahl_cross_league_value_policy(policy)?;
    if calibration.method_version != policy.method_version
        || source_games == 0
        || source_workload == 0
        || !source_rate.is_finite()
    {
        return Err("invalid cross-league value input".to_owned());
    }
    let (translated_ahl_rate, prior_rate, prior_workload, score_scale) =
        match calibration.position_group {
            AhlPlayerValuePositionGroup::Forward => {
                if source_games < policy.minimum_skater_source_games || source_rate < 0.0 {
                    return Err("insufficient cross-league forward workload".to_owned());
                }
                (
                    source_rate * calibration.translation_value,
                    policy.forward_prior_points_per_game,
                    policy.skater_prior_games,
                    f64::from(policy.skater_schedule_games),
                )
            }
            AhlPlayerValuePositionGroup::Defense => {
                if source_games < policy.minimum_skater_source_games || source_rate < 0.0 {
                    return Err("insufficient cross-league defense workload".to_owned());
                }
                (
                    source_rate * calibration.translation_value,
                    policy.defense_prior_points_per_game,
                    policy.skater_prior_games,
                    f64::from(policy.skater_schedule_games),
                )
            }
            AhlPlayerValuePositionGroup::Goalie => {
                if source_games < policy.minimum_goalie_source_games
                    || source_workload < policy.minimum_goalie_source_shots
                    || !(0.0..=1.0).contains(&source_rate)
                {
                    return Err("insufficient cross-league goalie workload".to_owned());
                }
                let translated = source_rate + calibration.translation_value;
                if !(0.0..=1.0).contains(&translated) {
                    return Err("translated goalie save percentage is out of range".to_owned());
                }
                (
                    translated,
                    policy.goalie_prior_save_percentage,
                    policy.goalie_prior_shots,
                    100.0,
                )
            }
        };
    let effective_workload =
        ((f64::from(source_workload) * calibration.calibration_confidence).round() as u32).max(1);
    let denominator = effective_workload
        .checked_add(prior_workload)
        .ok_or_else(|| "cross-league estimate workload overflow".to_owned())?;
    let shrunk_rate = (translated_ahl_rate * f64::from(effective_workload)
        + prior_rate * f64::from(prior_workload))
        / f64::from(denominator);
    let evidence_confidence =
        calibration.calibration_confidence * f64::from(effective_workload) / f64::from(denominator);
    Ok(AhlCrossLeagueValueEstimate {
        method_version: policy.method_version.clone(),
        position_group: calibration.position_group,
        source_league: calibration.source_league.clone(),
        source_season,
        source_games,
        source_workload,
        source_rate,
        translated_ahl_rate,
        effective_workload,
        prior_rate,
        shrunk_rate,
        projected_score: shrunk_rate * score_scale,
        evidence_confidence,
        calibration: calibration.clone(),
    })
}

fn season_gap(earlier: u32, later: u32) -> Result<u32, String> {
    let start = earlier / 10_000;
    let end = later / 10_000;
    end.checked_sub(start)
        .ok_or_else(|| "career seasons are not monotonic".to_owned())
}

pub fn validate_ahl_cross_league_value_policy(
    policy: &AhlCrossLeagueValuePolicy,
) -> Result<(), String> {
    if policy.schema != AHL_CROSS_LEAGUE_VALUE_POLICY_SCHEMA
        || policy.method_version != AHL_CROSS_LEAGUE_VALUE_METHOD
        || policy.maximum_source_lookback_seasons == 0
        || policy.maximum_pair_season_gap > 1
        || policy.minimum_calibration_pairs == 0
        || policy.minimum_calibration_players == 0
        || policy.minimum_calibration_workload == 0
        || policy.minimum_skater_pair_games == 0
        || policy.minimum_goalie_pair_shots == 0
        || policy.minimum_skater_source_games == 0
        || policy.minimum_goalie_source_games == 0
        || policy.minimum_goalie_source_shots == 0
        || !policy.skater_calibration_error_scale.is_finite()
        || policy.skater_calibration_error_scale <= 0.0
        || !policy.goalie_calibration_error_scale.is_finite()
        || policy.goalie_calibration_error_scale <= 0.0
        || policy.skater_schedule_games == 0
        || policy.skater_prior_games == 0
        || policy.goalie_prior_shots == 0
        || !policy.forward_prior_points_per_game.is_finite()
        || policy.forward_prior_points_per_game < 0.0
        || !policy.defense_prior_points_per_game.is_finite()
        || policy.defense_prior_points_per_game < 0.0
        || !policy.goalie_prior_save_percentage.is_finite()
        || !(0.0..=1.0).contains(&policy.goalie_prior_save_percentage)
    {
        return Err("invalid AHL cross-league value policy".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(count: u32, source: f64, ahl: f64) -> Vec<AhlCrossLeagueCalibrationPair> {
        (0..count)
            .map(|index| AhlCrossLeagueCalibrationPair {
                player_id: 1000 + index,
                source_season: 20242025,
                ahl_season: 20252026,
                source_rate: source,
                ahl_rate: ahl,
                paired_workload: 200,
            })
            .collect()
    }

    #[test]
    fn skater_calibration_fits_a_paired_rate_translation() {
        let policy = AhlCrossLeagueValuePolicy::default();
        let calibration = calibrate_ahl_cross_league_value(
            &policy,
            "ECHL",
            AhlPlayerValuePositionGroup::Forward,
            &pairs(30, 1.0, 0.5),
        )
        .expect("paired calibration");
        assert!((calibration.translation_value - 0.5).abs() < 1e-9);
        let estimate =
            estimate_ahl_cross_league_value(&policy, &calibration, 20252026, 40, 40, 1.0)
                .expect("translated estimate");
        assert!(estimate.projected_score > 32.0);
        assert!(estimate.projected_score < 36.0);
        assert!(estimate.evidence_confidence > 0.0);
    }

    #[test]
    fn goalie_calibration_uses_an_additive_save_percentage_delta() {
        let policy = AhlCrossLeagueValuePolicy::default();
        let calibration = calibrate_ahl_cross_league_value(
            &policy,
            "ECHL",
            AhlPlayerValuePositionGroup::Goalie,
            &pairs(30, 0.920, 0.905),
        )
        .expect("goalie calibration");
        assert!((calibration.translation_value + 0.015).abs() < 1e-9);
        let estimate =
            estimate_ahl_cross_league_value(&policy, &calibration, 20252026, 30, 900, 0.920)
                .expect("goalie estimate");
        assert!(estimate.projected_score > 90.0);
        assert!(estimate.projected_score < 91.0);
    }

    #[test]
    fn underpowered_calibration_and_source_samples_fail_closed() {
        let policy = AhlCrossLeagueValuePolicy::default();
        assert!(calibrate_ahl_cross_league_value(
            &policy,
            "ECHL",
            AhlPlayerValuePositionGroup::Forward,
            &pairs(19, 1.0, 0.5),
        )
        .is_err());
        let calibration = calibrate_ahl_cross_league_value(
            &policy,
            "ECHL",
            AhlPlayerValuePositionGroup::Forward,
            &pairs(30, 1.0, 0.5),
        )
        .expect("calibration");
        assert!(
            estimate_ahl_cross_league_value(&policy, &calibration, 20252026, 4, 4, 1.0,).is_err()
        );
    }
}
