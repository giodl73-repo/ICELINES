//! Adapters that assemble roster, lineup, goalie, xG, special-teams, and
//! matchup primitives into the shared game-prediction evidence contract.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use icelines_core::{
    resolve_fantasy_goalie_start, validate_player_line_matchup_forecast,
    FantasyGoalieStartObservation, FantasyGoalieStartState, PlayerLineMatchupForecastView,
    TeamGameEvidenceState, TeamGameOpeningStrengthRow, TeamGamePredictionEvidenceInput,
    TeamGamePredictionTeamEvidence, TeamLineupPlayerView, TeamLineupProjectionView,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    MoneyPuckOpponentAdjustedXgForm, MoneyPuckTrailingSpecialTeamsForm, MoneyPuckTrailingXgForm,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionGoalieCandidate {
    pub player_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<u32>,
    pub quality: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionSpecialTeamsScore {
    pub team: String,
    pub strength: f64,
    pub games: usize,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionTeamAssemblyInput {
    pub team: String,
    pub opening_strength: Option<TeamGameOpeningStrengthRow>,
    pub roster_state: TeamGameEvidenceState,
    pub lineup: Option<TeamLineupProjectionView>,
    pub lineup_state: TeamGameEvidenceState,
    pub goalie_candidates: Vec<GamePredictionGoalieCandidate>,
    pub modeled_starter_key: Option<String>,
    pub goalie_observations: Vec<FantasyGoalieStartObservation>,
    pub xg_form: Option<MoneyPuckTrailingXgForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opponent_adjusted_xg_form: Option<MoneyPuckOpponentAdjustedXgForm>,
    pub special_teams: Option<GamePredictionSpecialTeamsScore>,
    pub matchup_suitability: Option<f64>,
    pub matchup_state: TeamGameEvidenceState,
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionEvidenceAssembly {
    pub evidence: TeamGamePredictionEvidenceInput,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionGameAssemblyInput {
    pub game_id: u64,
    pub forecast_at: DateTime<Utc>,
    pub captured_at: DateTime<Utc>,
    pub away: GamePredictionTeamAssemblyInput,
    pub home: GamePredictionTeamAssemblyInput,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum GamePredictionEvidenceAssemblerError {
    #[error("invalid game prediction assembly: {0}")]
    Invalid(String),
    #[error("special-teams ranking requires at least 16 complete teams")]
    InsufficientSpecialTeamsCoverage,
}

pub fn rank_special_teams_forms(
    forms: &[MoneyPuckTrailingSpecialTeamsForm],
) -> Result<Vec<GamePredictionSpecialTeamsScore>, GamePredictionEvidenceAssemblerError> {
    let complete = forms
        .iter()
        .filter_map(|form| {
            Some((
                form,
                form.power_play_xg_for_per_60?,
                form.penalty_kill_xg_against_per_60?,
            ))
        })
        .collect::<Vec<_>>();
    let unique = complete
        .iter()
        .map(|(form, _, _)| form.team.as_str())
        .collect::<BTreeSet<_>>();
    if complete.len() < 16 || unique.len() != complete.len() {
        return Err(GamePredictionEvidenceAssemblerError::InsufficientSpecialTeamsCoverage);
    }
    let pp = complete
        .iter()
        .map(|(_, value, _)| *value)
        .collect::<Vec<_>>();
    let pk = complete
        .iter()
        .map(|(_, _, value)| *value)
        .collect::<Vec<_>>();
    let mut scores = complete
        .iter()
        .map(|(form, power_play, penalty_kill)| {
            let pp_percentile = percentile(*power_play, &pp, true);
            let pk_percentile = percentile(*penalty_kill, &pk, false);
            GamePredictionSpecialTeamsScore {
                team: form.team.clone(),
                strength: (pp_percentile + pk_percentile) / 2.0,
                games: form.games,
                source_fingerprint: form.source_fingerprint.clone(),
            }
        })
        .collect::<Vec<_>>();
    scores.sort_by(|left, right| left.team.cmp(&right.team));
    Ok(scores)
}

pub fn assemble_game_prediction_evidence(
    game_id: u64,
    forecast_at: DateTime<Utc>,
    captured_at: DateTime<Utc>,
    away: GamePredictionTeamAssemblyInput,
    home: GamePredictionTeamAssemblyInput,
) -> Result<GamePredictionEvidenceAssembly, GamePredictionEvidenceAssemblerError> {
    if game_id == 0 || captured_at > forecast_at {
        return Err(GamePredictionEvidenceAssemblerError::Invalid(
            "game ID and evidence timestamps are invalid".to_owned(),
        ));
    }
    let mut warnings = Vec::new();
    let away = assemble_team(away, forecast_at, &mut warnings)?;
    let home = assemble_team(home, forecast_at, &mut warnings)?;
    if away.team == home.team {
        return Err(GamePredictionEvidenceAssemblerError::Invalid(
            "away and home teams must differ".to_owned(),
        ));
    }
    Ok(GamePredictionEvidenceAssembly {
        evidence: TeamGamePredictionEvidenceInput {
            game_id,
            forecast_at,
            captured_at,
            away,
            home,
        },
        warnings,
    })
}

/// Attach one sealed player/line matchup forecast to the raw evidence assembly
/// consumed by the existing game-edge package. This is a typed adapter: it
/// neither recomputes matchup scores nor changes the game probability model.
pub fn attach_player_line_matchup_forecast(
    input: &mut GamePredictionGameAssemblyInput,
    matchup: &PlayerLineMatchupForecastView,
) -> Result<(), GamePredictionEvidenceAssemblerError> {
    validate_player_line_matchup_forecast(matchup)
        .map_err(GamePredictionEvidenceAssemblerError::Invalid)?;
    if input.game_id != matchup.game_id
        || input.forecast_at != matchup.forecast_at
        || input.captured_at != matchup.captured_at
        || !input.away.team.eq_ignore_ascii_case(&matchup.away.team)
        || !input.home.team.eq_ignore_ascii_case(&matchup.home.team)
    {
        return Err(GamePredictionEvidenceAssemblerError::Invalid(
            "player-line matchup and game assembly identity or timestamps differ".to_owned(),
        ));
    }
    input.away.matchup_suitability = matchup.away.matchup_suitability;
    input.away.matchup_state = matchup.away.matchup_state;
    input.home.matchup_suitability = matchup.home.matchup_suitability;
    input.home.matchup_state = matchup.home.matchup_state;
    for team in [&mut input.away, &mut input.home] {
        if !team.source_fingerprints.contains(&matchup.fingerprint) {
            team.source_fingerprints.push(matchup.fingerprint.clone());
        }
    }
    Ok(())
}

impl GamePredictionGameAssemblyInput {
    pub fn assemble(
        self,
    ) -> Result<GamePredictionEvidenceAssembly, GamePredictionEvidenceAssemblerError> {
        assemble_game_prediction_evidence(
            self.game_id,
            self.forecast_at,
            self.captured_at,
            self.away,
            self.home,
        )
    }
}

fn assemble_team(
    input: GamePredictionTeamAssemblyInput,
    forecast_at: DateTime<Utc>,
    warnings: &mut Vec<String>,
) -> Result<TeamGamePredictionTeamEvidence, GamePredictionEvidenceAssemblerError> {
    let team = input.team.trim().to_ascii_uppercase();
    if team.is_empty()
        || input
            .source_fingerprints
            .iter()
            .any(|fingerprint| !valid_sha256(fingerprint))
        || input.source_fingerprints.is_empty()
    {
        return Err(GamePredictionEvidenceAssemblerError::Invalid(format!(
            "{team} has invalid identity or source seals"
        )));
    }
    let roster_strength = input.opening_strength.as_ref().and_then(|row| {
        (row.team.trim().eq_ignore_ascii_case(&team)
            && row.value_coverage >= 0.70
            && (0.0..=100.0).contains(&row.strength))
        .then_some(row.strength)
    });
    let roster_state = state_for_value(input.roster_state, roster_strength);
    if input.opening_strength.is_some() && roster_strength.is_none() {
        warnings.push(format!(
            "{team} opening roster strength was withheld because identity, range, or 70% value coverage failed"
        ));
    }

    let availability_strength = input
        .lineup
        .as_ref()
        .and_then(|lineup| lineup_strength(lineup, &team));
    let availability_state = state_for_value(input.lineup_state, availability_strength);
    let lineup_impact = input
        .lineup
        .as_ref()
        .zip(input.opening_strength.as_ref())
        .and_then(|(lineup, opening)| lineup_replacement_impact(lineup, opening, &team));
    let lineup_impact_state = state_for_value(input.lineup_state, lineup_impact);
    if input.lineup.is_some() && availability_strength.is_none() {
        warnings.push(format!(
            "{team} lineup strength was withheld because team identity or 75% score coverage failed"
        ));
    }

    let candidates = input
        .goalie_candidates
        .iter()
        .map(|goalie| (goalie.player_key.as_str(), goalie.quality))
        .collect::<BTreeMap<_, _>>();
    if candidates.len() != input.goalie_candidates.len()
        || input.goalie_candidates.iter().any(|goalie| {
            goalie.player_key.trim().is_empty()
                || goalie.player_id == Some(0)
                || !goalie.quality.is_finite()
                || !(0.0..=100.0).contains(&goalie.quality)
        })
    {
        return Err(GamePredictionEvidenceAssemblerError::Invalid(format!(
            "{team} goalie candidate qualities are invalid"
        )));
    }
    let (goalie_quality, goalie_state, goalie_player_id) = resolve_goalie(
        &input.goalie_candidates,
        input.modeled_starter_key.as_deref(),
        &input.goalie_observations,
        forecast_at,
    );

    let xg_form = input.xg_form.filter(|form| {
        form.team.eq_ignore_ascii_case(&team)
            && form.latest_game_date < forecast_at.date_naive()
            && (0.0..=1.0).contains(&form.xg_share)
            && input.source_fingerprints.contains(&form.source_fingerprint)
    });
    if xg_form.is_none() {
        warnings.push(format!(
            "{team} has no point-in-time eligible trailing xG form"
        ));
    }
    let special = input.special_teams.filter(|form| {
        form.team.eq_ignore_ascii_case(&team)
            && form.strength.is_finite()
            && (0.0..=100.0).contains(&form.strength)
            && form.games > 0
            && input.source_fingerprints.contains(&form.source_fingerprint)
    });
    let opponent_adjusted_xg_form = input.opponent_adjusted_xg_form.filter(|form| {
        form.team.eq_ignore_ascii_case(&team)
            && form.latest_game_date < forecast_at.date_naive()
            && (0.0..=1.0).contains(&form.adjusted_xg_share)
            && form.games > 0
            && input.source_fingerprints.contains(&form.source_fingerprint)
    });
    let matchup_suitability = input
        .matchup_suitability
        .filter(|value| value.is_finite() && (-1.0..=1.0).contains(value));
    let matchup_state = state_for_value(input.matchup_state, matchup_suitability);

    Ok(TeamGamePredictionTeamEvidence {
        team,
        roster_strength,
        roster_state,
        availability_strength,
        availability_state,
        lineup_impact,
        lineup_impact_state,
        goalie_quality,
        goalie_state,
        goalie_player_id,
        goalie_form_quality: None,
        goalie_form_appearances: 0,
        goalie_form_state: TeamGameEvidenceState::Unavailable,
        goalie_workload_readiness: None,
        xg_share: xg_form.as_ref().map(|form| form.xg_share),
        xg_games: xg_form.as_ref().map_or(0, |form| form.games),
        opponent_adjusted_xg_share: opponent_adjusted_xg_form
            .as_ref()
            .map(|form| form.adjusted_xg_share),
        opponent_adjusted_xg_games: opponent_adjusted_xg_form
            .as_ref()
            .map_or(0, |form| form.games),
        special_teams_strength: special.as_ref().map(|form| form.strength),
        special_teams_games: special.as_ref().map_or(0, |form| form.games),
        matchup_suitability,
        matchup_state,
        source_fingerprints: input.source_fingerprints,
    })
}

fn lineup_strength(lineup: &TeamLineupProjectionView, team: &str) -> Option<f64> {
    if !lineup.team.eq_ignore_ascii_case(team) {
        return None;
    }
    let mut weighted_sum = 0.0;
    let mut covered_weight = 0.0;
    let mut total_weight = 0.0;
    let mut add = |player: Option<&TeamLineupPlayerView>, weight: f64| {
        total_weight += weight;
        if let Some(value) = player.and_then(|player| player.score.value) {
            if value.is_finite() && (0.0..=100.0).contains(&value) {
                weighted_sum += value * weight;
                covered_weight += weight;
            }
        }
    };
    for line in &lineup.forward_lines {
        let weight = match line.line {
            1 => 1.25,
            2 => 1.05,
            3 => 0.85,
            _ => 0.65,
        };
        add(line.left_wing.as_ref(), weight);
        add(line.center.as_ref(), weight);
        add(line.right_wing.as_ref(), weight);
    }
    for pair in &lineup.defense_pairs {
        let weight = match pair.pair {
            1 => 1.25,
            2 => 1.0,
            _ => 0.75,
        };
        add(pair.left.as_ref(), weight);
        add(pair.right.as_ref(), weight);
    }
    (total_weight > 0.0 && covered_weight / total_weight >= 0.75)
        .then(|| weighted_sum / covered_weight)
}

fn lineup_replacement_impact(
    lineup: &TeamLineupProjectionView,
    opening: &TeamGameOpeningStrengthRow,
    team: &str,
) -> Option<f64> {
    if !lineup.team.eq_ignore_ascii_case(team) || !opening.team.eq_ignore_ascii_case(team) {
        return None;
    }
    let actual_forwards = lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .map(|player| player.as_ref().and_then(|player| player.score.value))
        .collect::<Vec<_>>();
    let actual_defense = lineup
        .defense_pairs
        .iter()
        .flat_map(|pair| [&pair.left, &pair.right])
        .map(|player| player.as_ref().and_then(|player| player.score.value))
        .collect::<Vec<_>>();
    let covered = actual_forwards
        .iter()
        .chain(&actual_defense)
        .filter(|value| value.is_some_and(|value| value.is_finite()))
        .count();
    let total = actual_forwards.len() + actual_defense.len();
    if total == 0 || covered * 4 < total * 3 {
        return None;
    }
    let expected_forwards = opening
        .players
        .iter()
        .filter(|player| player.position_group == "forward")
        .map(|player| player.modeled_value)
        .collect::<Vec<_>>();
    let expected_defense = opening
        .players
        .iter()
        .filter(|player| player.position_group == "defense")
        .map(|player| player.modeled_value)
        .collect::<Vec<_>>();
    let actual = replacement_adjusted_lineup_value(
        actual_forwards
            .into_iter()
            .map(|value| value.unwrap_or(50.0)),
        actual_defense
            .into_iter()
            .map(|value| value.unwrap_or(50.0)),
    );
    let expected = replacement_adjusted_lineup_value(expected_forwards, expected_defense);
    Some((actual - expected).clamp(-55.0, 55.0))
}

fn replacement_adjusted_lineup_value(
    forwards: impl IntoIterator<Item = f64>,
    defense: impl IntoIterator<Item = f64>,
) -> f64 {
    let mut forwards = forwards.into_iter().collect::<Vec<_>>();
    let mut defense = defense.into_iter().collect::<Vec<_>>();
    forwards.sort_by(|left, right| right.total_cmp(left));
    defense.sort_by(|left, right| right.total_cmp(left));
    forwards.truncate(12);
    defense.truncate(6);
    forwards.resize(12, 50.0);
    defense.resize(6, 50.0);
    forwards
        .into_iter()
        .chain(defense)
        .map(|value| (value - 45.0).max(0.0))
        .sum::<f64>()
        / 18.0
}

fn resolve_goalie(
    candidates: &[GamePredictionGoalieCandidate],
    modeled_starter_key: Option<&str>,
    observations: &[FantasyGoalieStartObservation],
    forecast_at: DateTime<Utc>,
) -> (Option<f64>, TeamGameEvidenceState, Option<u32>) {
    let priority = |state: FantasyGoalieStartState| match state {
        FantasyGoalieStartState::ConfirmedStarting => 3,
        FantasyGoalieStartState::ReportedStarting => 2,
        FantasyGoalieStartState::EstimatedStarting => 1,
        _ => 0,
    };
    let resolved = candidates
        .iter()
        .filter_map(|candidate| {
            let row = resolve_fantasy_goalie_start(
                candidate.player_key.clone(),
                forecast_at.date_naive(),
                observations,
                forecast_at,
                12 * 60,
            );
            let priority = priority(row.effective_state);
            (priority > 0).then_some((priority, candidate, row.effective_state))
        })
        .max_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.player_key.cmp(&right.1.player_key))
        });
    if let Some((_, candidate, state)) = resolved {
        let state = match state {
            FantasyGoalieStartState::ConfirmedStarting => TeamGameEvidenceState::Confirmed,
            FantasyGoalieStartState::ReportedStarting => TeamGameEvidenceState::Reported,
            FantasyGoalieStartState::EstimatedStarting => TeamGameEvidenceState::Modeled,
            _ => TeamGameEvidenceState::Unavailable,
        };
        return (Some(candidate.quality), state, candidate.player_id);
    }
    let modeled = modeled_starter_key.and_then(|key| {
        candidates
            .iter()
            .find(|candidate| candidate.player_key == key)
    });
    modeled.map_or(
        (None, TeamGameEvidenceState::Unavailable, None),
        |candidate| {
            (
                Some(candidate.quality),
                TeamGameEvidenceState::Modeled,
                candidate.player_id,
            )
        },
    )
}

fn state_for_value(state: TeamGameEvidenceState, value: Option<f64>) -> TeamGameEvidenceState {
    if value.is_some() && state != TeamGameEvidenceState::Unavailable {
        state
    } else {
        TeamGameEvidenceState::Unavailable
    }
}

fn percentile(value: f64, values: &[f64], higher_is_better: bool) -> f64 {
    let better_direction = values
        .iter()
        .filter(|candidate| {
            if higher_is_better {
                **candidate < value
            } else {
                **candidate > value
            }
        })
        .count() as f64;
    let tied = values
        .iter()
        .filter(|candidate| (**candidate - value).abs() < 1e-12)
        .count() as f64;
    (better_direction + tied * 0.5) / values.len() as f64 * 100.0
}

fn valid_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

pub fn fingerprint_special_teams_scores(scores: &[GamePredictionSpecialTeamsScore]) -> String {
    let bytes = serde_json::to_vec(scores).expect("special-teams scores serialize");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};
    use icelines_core::{
        build_player_line_matchup_forecast, OpponentTacticalStyle, PlayerForecastProfileDimensions,
        PlayerForecastProfileInput, PlayerLineMatchupForecastInput, PlayerLineMatchupTeamInput,
        TeamGameForecastVintage, PLAYER_FORECAST_PROFILE_SCHEMA,
    };

    use super::*;

    fn sha() -> String {
        format!("sha256:{}", "a".repeat(64))
    }

    #[test]
    fn l0_special_teams_rank_is_league_relative_and_directional() {
        let forms = (0..16)
            .map(|index| MoneyPuckTrailingSpecialTeamsForm {
                team: format!("T{index:02}"),
                before_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                games: 20,
                latest_game_date: NaiveDate::from_ymd_opt(2025, 12, 30).unwrap(),
                power_play_xg_for_per_60: Some(index as f64),
                penalty_kill_xg_against_per_60: Some((15 - index) as f64),
                source_fingerprint: sha(),
            })
            .collect::<Vec<_>>();
        let scores = rank_special_teams_forms(&forms).unwrap();
        assert!(scores.last().unwrap().strength > scores.first().unwrap().strength);
    }

    #[test]
    fn l0_confirmed_starter_beats_modeled_starter_without_changing_quality_scale() {
        let forecast_at = Utc.with_ymd_and_hms(2026, 10, 10, 16, 0, 0).unwrap();
        let observations = vec![FantasyGoalieStartObservation {
            player_key: "confirmed".to_owned(),
            game_date: forecast_at.date_naive(),
            state: FantasyGoalieStartState::ConfirmedStarting,
            source: "official-report".to_owned(),
            source_url: Some("https://example.test/goalie".to_owned()),
            observed_at: forecast_at - chrono::Duration::minutes(10),
            fetched_at: forecast_at - chrono::Duration::minutes(5),
            detail: None,
        }];
        let (quality, state, player_id) = resolve_goalie(
            &[
                GamePredictionGoalieCandidate {
                    player_key: "modeled".to_owned(),
                    player_id: Some(1),
                    quality: 60.0,
                },
                GamePredictionGoalieCandidate {
                    player_key: "confirmed".to_owned(),
                    player_id: Some(2),
                    quality: 55.0,
                },
            ],
            Some("modeled"),
            &observations,
            forecast_at,
        );
        assert_eq!(quality, Some(55.0));
        assert_eq!(state, TeamGameEvidenceState::Confirmed);
        assert_eq!(player_id, Some(2));
    }

    #[test]
    fn l0_future_xg_is_withheld_instead_of_leaking() {
        let forecast_at = Utc.with_ymd_and_hms(2026, 9, 15, 12, 0, 0).unwrap();
        let side = |team: &str| GamePredictionTeamAssemblyInput {
            team: team.to_owned(),
            opening_strength: None,
            roster_state: TeamGameEvidenceState::Unavailable,
            lineup: None,
            lineup_state: TeamGameEvidenceState::Unavailable,
            goalie_candidates: Vec::new(),
            modeled_starter_key: None,
            goalie_observations: Vec::new(),
            xg_form: Some(MoneyPuckTrailingXgForm {
                team: team.to_owned(),
                before_date: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                requested_games: 10,
                games: 10,
                latest_game_date: NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
                score_venue_adjusted_xg_for: 20.0,
                score_venue_adjusted_xg_against: 20.0,
                xg_share: 0.5,
                source_fingerprint: sha(),
            }),
            opponent_adjusted_xg_form: None,
            special_teams: None,
            matchup_suitability: None,
            matchup_state: TeamGameEvidenceState::Unavailable,
            source_fingerprints: vec![sha()],
        };
        let assembly = assemble_game_prediction_evidence(
            1,
            forecast_at,
            forecast_at,
            side("NYR"),
            side("SEA"),
        )
        .unwrap();
        assert_eq!(assembly.evidence.home.xg_share, None);
        assert!(assembly
            .warnings
            .iter()
            .any(|warning| warning.contains("NYR")));
    }

    #[test]
    fn l0_sealed_player_line_matchup_attaches_to_the_exact_game_only() {
        let away: TeamLineupProjectionView =
            serde_json::from_str(include_str!("../../examples/team-lineup-sea-2026-27.json"))
                .unwrap();
        let home: TeamLineupProjectionView =
            serde_json::from_str(include_str!("../../examples/team-lineup-nyr-2026-27.json"))
                .unwrap();
        let forecast_at = Utc.with_ymd_and_hms(2026, 10, 10, 16, 0, 0).unwrap();
        let captured_at = Utc.with_ymd_and_hms(2026, 10, 10, 15, 0, 0).unwrap();
        let matchup = build_player_line_matchup_forecast(PlayerLineMatchupForecastInput {
            game_id: 2026020001,
            season: 20262027,
            game_date: NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
            vintage: TeamGameForecastVintage::GameMorning,
            forecast_at,
            captured_at,
            away: matchup_team(away),
            home: matchup_team(home),
        })
        .unwrap();
        let side = |team: &str| GamePredictionTeamAssemblyInput {
            team: team.to_owned(),
            opening_strength: None,
            roster_state: TeamGameEvidenceState::Unavailable,
            lineup: None,
            lineup_state: TeamGameEvidenceState::Unavailable,
            goalie_candidates: Vec::new(),
            modeled_starter_key: None,
            goalie_observations: Vec::new(),
            xg_form: None,
            opponent_adjusted_xg_form: None,
            special_teams: None,
            matchup_suitability: None,
            matchup_state: TeamGameEvidenceState::Unavailable,
            source_fingerprints: vec![sha()],
        };
        let mut assembly = GamePredictionGameAssemblyInput {
            game_id: matchup.game_id,
            forecast_at,
            captured_at,
            away: side("SEA"),
            home: side("NYR"),
        };
        attach_player_line_matchup_forecast(&mut assembly, &matchup).unwrap();
        assert_eq!(
            assembly.home.matchup_suitability,
            matchup.home.matchup_suitability
        );
        assert!(assembly
            .home
            .source_fingerprints
            .contains(&matchup.fingerprint));

        assembly.game_id += 1;
        assert!(attach_player_line_matchup_forecast(&mut assembly, &matchup).is_err());
    }

    fn matchup_team(lineup: TeamLineupProjectionView) -> PlayerLineMatchupTeamInput {
        let mut players = lineup
            .forward_lines
            .iter()
            .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
            .flatten()
            .chain(
                lineup
                    .defense_pairs
                    .iter()
                    .flat_map(|pair| [&pair.left, &pair.right])
                    .flatten(),
            )
            .collect::<Vec<_>>();
        players.sort_by_key(|player| player.player_id);
        let profiles = players
            .into_iter()
            .map(|player| PlayerForecastProfileInput {
                schema: PLAYER_FORECAST_PROFILE_SCHEMA.to_owned(),
                player_id: player.player_id,
                team: lineup.team.clone(),
                evidence_cutoff_at: Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
                games_played: 82,
                even_strength_minutes: 1_000.0,
                observed_shifts: 1_500,
                recency: 1.0,
                dimensions: PlayerForecastProfileDimensions {
                    scoring_creation: Some(60.0),
                    finishing: Some(60.0),
                    passing_transition: Some(60.0),
                    forecheck_retrieval: Some(60.0),
                    defensive_suppression: Some(60.0),
                    physical_matchup: Some(60.0),
                    discipline_puck_security: Some(60.0),
                    faceoffs: Some(60.0),
                    power_play: Some(60.0),
                    penalty_kill: Some(60.0),
                },
                source_fingerprints: vec![sha()],
            })
            .collect();
        PlayerLineMatchupTeamInput {
            lineup,
            lineup_state: TeamGameEvidenceState::Reported,
            profiles,
            chemistry: Vec::new(),
            opponent_style: OpponentTacticalStyle::Balanced,
            manager_execution_confidence: 0.5,
            forward_line_shares_pct: None,
            source_fingerprints: vec![sha()],
        }
    }
}
