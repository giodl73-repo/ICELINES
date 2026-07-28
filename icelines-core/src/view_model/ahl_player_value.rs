//! Confidence-weighted prior-season AHL player value.
//!
//! This method orders players inside an affiliate position group. It is an
//! evaluation estimate, not an NHL-equivalency model or calibrated forecast.

use serde::{Deserialize, Serialize};

pub const AHL_PLAYER_VALUE_POLICY_SCHEMA: &str = "ahl_player_value_policy.v1";
pub const AHL_PLAYER_VALUE_METHOD: &str = "ahl_prior_performance_bayesian_rate.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlPlayerValuePositionGroup {
    Forward,
    Defense,
    Goalie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlPlayerValuePolicy {
    pub schema: String,
    pub method_version: String,
    pub skater_schedule_games: u32,
    pub skater_prior_games: u32,
    pub forward_prior_points_per_game: f64,
    pub defense_prior_points_per_game: f64,
    pub goalie_prior_shots: u32,
    pub goalie_prior_save_percentage: f64,
}

impl Default for AhlPlayerValuePolicy {
    fn default() -> Self {
        Self {
            schema: AHL_PLAYER_VALUE_POLICY_SCHEMA.to_owned(),
            method_version: AHL_PLAYER_VALUE_METHOD.to_owned(),
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
pub struct AhlPlayerValueEstimate {
    pub method_version: String,
    pub position_group: AhlPlayerValuePositionGroup,
    pub projected_score: f64,
    pub evidence_confidence: f64,
    pub sample_games: u32,
    pub observed_rate: f64,
    pub prior_rate: f64,
    pub shrunk_rate: f64,
}

pub fn estimate_ahl_skater_value(
    policy: &AhlPlayerValuePolicy,
    position_group: AhlPlayerValuePositionGroup,
    games_played: u32,
    points: u32,
) -> Result<AhlPlayerValueEstimate, String> {
    validate_policy(policy)?;
    if !matches!(
        position_group,
        AhlPlayerValuePositionGroup::Forward | AhlPlayerValuePositionGroup::Defense
    ) {
        return Err("AHL skater value requires a forward or defense position group".to_owned());
    }
    let prior_rate = if position_group == AhlPlayerValuePositionGroup::Defense {
        policy.defense_prior_points_per_game
    } else {
        policy.forward_prior_points_per_game
    };
    let observed_rate = if games_played == 0 {
        0.0
    } else {
        f64::from(points) / f64::from(games_played)
    };
    let denominator = f64::from(
        games_played
            .checked_add(policy.skater_prior_games)
            .ok_or_else(|| "AHL skater workload overflows policy range".to_owned())?,
    );
    let shrunk_rate =
        (f64::from(points) + prior_rate * f64::from(policy.skater_prior_games)) / denominator;
    Ok(AhlPlayerValueEstimate {
        method_version: policy.method_version.clone(),
        position_group,
        projected_score: shrunk_rate * f64::from(policy.skater_schedule_games),
        evidence_confidence: f64::from(games_played) / denominator,
        sample_games: games_played,
        observed_rate,
        prior_rate,
        shrunk_rate,
    })
}

pub fn estimate_ahl_goalie_value(
    policy: &AhlPlayerValuePolicy,
    games_played: u32,
    shots_against: u32,
    saves: u32,
) -> Result<AhlPlayerValueEstimate, String> {
    validate_policy(policy)?;
    if saves > shots_against {
        return Err("AHL goalie saves cannot exceed shots against".to_owned());
    }
    let observed_rate = if shots_against == 0 {
        0.0
    } else {
        f64::from(saves) / f64::from(shots_against)
    };
    let denominator = f64::from(
        shots_against
            .checked_add(policy.goalie_prior_shots)
            .ok_or_else(|| "AHL goalie workload overflows policy range".to_owned())?,
    );
    let shrunk_rate = (f64::from(saves)
        + policy.goalie_prior_save_percentage * f64::from(policy.goalie_prior_shots))
        / denominator;
    Ok(AhlPlayerValueEstimate {
        method_version: policy.method_version.clone(),
        position_group: AhlPlayerValuePositionGroup::Goalie,
        projected_score: shrunk_rate * 100.0,
        evidence_confidence: f64::from(shots_against) / denominator,
        sample_games: games_played,
        observed_rate,
        prior_rate: policy.goalie_prior_save_percentage,
        shrunk_rate,
    })
}

fn validate_policy(policy: &AhlPlayerValuePolicy) -> Result<(), String> {
    if policy.schema != AHL_PLAYER_VALUE_POLICY_SCHEMA
        || policy.method_version != AHL_PLAYER_VALUE_METHOD
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
        return Err("invalid AHL player-value policy".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skater_value_shrinks_short_samples_more_than_long_samples() {
        let policy = AhlPlayerValuePolicy::default();
        let short =
            estimate_ahl_skater_value(&policy, AhlPlayerValuePositionGroup::Forward, 5, 5).unwrap();
        let long = estimate_ahl_skater_value(&policy, AhlPlayerValuePositionGroup::Forward, 50, 50)
            .unwrap();
        assert!(long.projected_score > short.projected_score);
        assert!(long.evidence_confidence > short.evidence_confidence);
    }

    #[test]
    fn goalie_value_uses_shot_workload_not_games_as_confidence() {
        let policy = AhlPlayerValuePolicy::default();
        let light = estimate_ahl_goalie_value(&policy, 10, 100, 92).unwrap();
        let heavy = estimate_ahl_goalie_value(&policy, 10, 1_000, 920).unwrap();
        assert!(heavy.projected_score > light.projected_score);
        assert!(heavy.evidence_confidence > light.evidence_confidence);
    }

    #[test]
    fn invalid_goalie_totals_fail_closed() {
        assert!(estimate_ahl_goalie_value(&AhlPlayerValuePolicy::default(), 1, 10, 11).is_err());
    }
}
