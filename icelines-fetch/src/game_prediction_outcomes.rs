//! Sealed official NHL result snapshots for joining outcomes after forecasts.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use icelines_core::TeamGamePredictionOutcomeInput;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::nhl_api::ScheduledGame;

pub const OFFICIAL_GAME_OUTCOME_SET_SCHEMA: &str = "official_game_outcome_set.v1";
pub const OFFICIAL_GAME_OUTCOME_SET_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/official_game_outcome_set.v1.schema.json");

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfficialGameOutcomeSet {
    pub schema: String,
    pub season: u32,
    pub captured_at: DateTime<Utc>,
    pub source_url: String,
    pub complete: bool,
    pub scheduled_games: usize,
    pub final_games: usize,
    pub schedule_fingerprint: String,
    pub outcomes: Vec<TeamGamePredictionOutcomeInput>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OfficialGameOutcomeError {
    #[error("invalid official game outcome evidence: {0}")]
    Invalid(String),
    #[error("official game outcome serialization failed: {0}")]
    Serialization(String),
    #[error("official game outcome fingerprint mismatch")]
    FingerprintMismatch,
}

pub fn build_official_game_outcome_set(
    schedule: &[ScheduledGame],
    season: u32,
    captured_at: DateTime<Utc>,
    source_url: impl Into<String>,
    require_complete: bool,
) -> Result<OfficialGameOutcomeSet, OfficialGameOutcomeError> {
    let source_url = source_url.into();
    if season < 20_000_000 || source_url.trim().is_empty() {
        return Err(OfficialGameOutcomeError::Invalid(
            "season and source URL are required".to_owned(),
        ));
    }
    let mut regular = schedule
        .iter()
        .filter(|game| game.game_type == 2)
        .cloned()
        .collect::<Vec<_>>();
    regular.sort_by_key(|game| game.game_id);
    let mut ids = BTreeSet::new();
    if regular.is_empty() || regular.iter().any(|game| !ids.insert(game.game_id)) {
        return Err(OfficialGameOutcomeError::Invalid(
            "regular-season schedule is empty or duplicated".to_owned(),
        ));
    }
    let schedule_fingerprint = fingerprint_value(&regular)?;
    let mut outcomes = Vec::new();
    for game in &regular {
        if !game.is_final() {
            continue;
        }
        let (Some(away), Some(home)) = (game.away_score, game.home_score) else {
            return Err(OfficialGameOutcomeError::Invalid(format!(
                "final game {} is missing its score",
                game.game_id
            )));
        };
        if away == home {
            return Err(OfficialGameOutcomeError::Invalid(format!(
                "final NHL game {} cannot end tied",
                game.game_id
            )));
        }
        let start = DateTime::parse_from_rfc3339(&game.start_time_utc)
            .map_err(|_| {
                OfficialGameOutcomeError::Invalid(format!(
                    "game {} has invalid start time",
                    game.game_id
                ))
            })?
            .with_timezone(&Utc);
        if start >= captured_at {
            return Err(OfficialGameOutcomeError::Invalid(format!(
                "final game {} does not predate result capture",
                game.game_id
            )));
        }
        outcomes.push(TeamGamePredictionOutcomeInput {
            season,
            game_id: game.game_id,
            outcome_recorded_at: captured_at,
            home_won: home > away,
            source_fingerprint: schedule_fingerprint.clone(),
        });
    }
    let complete = outcomes.len() == regular.len();
    if require_complete && !complete {
        return Err(OfficialGameOutcomeError::Invalid(format!(
            "only {} of {} regular-season games are final",
            outcomes.len(),
            regular.len()
        )));
    }
    let mut view = OfficialGameOutcomeSet {
        schema: OFFICIAL_GAME_OUTCOME_SET_SCHEMA.to_owned(),
        season,
        captured_at,
        source_url,
        complete,
        scheduled_games: regular.len(),
        final_games: outcomes.len(),
        schedule_fingerprint,
        outcomes,
        fingerprint: String::new(),
    };
    view.fingerprint = outcome_fingerprint(&view)?;
    Ok(view)
}

impl OfficialGameOutcomeSet {
    pub fn validate(&self) -> Result<(), OfficialGameOutcomeError> {
        if self.schema != OFFICIAL_GAME_OUTCOME_SET_SCHEMA
            || self.scheduled_games == 0
            || self.final_games != self.outcomes.len()
            || self.complete != (self.final_games == self.scheduled_games)
            || self
                .outcomes
                .iter()
                .any(|row| row.season != self.season || row.outcome_recorded_at != self.captured_at)
            || self.fingerprint != outcome_fingerprint(self)?
        {
            return Err(OfficialGameOutcomeError::FingerprintMismatch);
        }
        Ok(())
    }
}

fn outcome_fingerprint(view: &OfficialGameOutcomeSet) -> Result<String, OfficialGameOutcomeError> {
    let mut canonical = view.clone();
    canonical.fingerprint.clear();
    fingerprint_value(&canonical)
}

fn fingerprint_value<T: Serialize>(value: &T) -> Result<String, OfficialGameOutcomeError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| OfficialGameOutcomeError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn game(id: u64, final_game: bool) -> ScheduledGame {
        ScheduledGame {
            game_id: id,
            date: "2025-10-01".to_owned(),
            game_type: 2,
            away_abbrev: "SEA".to_owned(),
            away_name: "Seattle".to_owned(),
            home_abbrev: "NYR".to_owned(),
            home_name: "New York".to_owned(),
            start_time_utc: "2025-10-01T23:00:00Z".to_owned(),
            away_score: final_game.then_some(2),
            home_score: final_game.then_some(3),
            game_state: Some(if final_game { "FINAL" } else { "FUT" }.to_owned()),
            last_period: final_game.then(|| "OT".to_owned()),
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    #[test]
    fn l0_official_outcomes_are_sealed_after_the_result() {
        let captured = Utc.with_ymd_and_hms(2025, 10, 2, 12, 0, 0).unwrap();
        let view = build_official_game_outcome_set(
            &[game(1, true)],
            20_252_026,
            captured,
            "https://api-web.nhle.com/v1/club-schedule-season/NYR/20252026",
            true,
        )
        .unwrap();
        assert!(view.complete);
        assert!(view.outcomes[0].home_won);
        view.validate().unwrap();
    }

    #[test]
    fn l0_incomplete_completed_season_is_refused() {
        let error = build_official_game_outcome_set(
            &[game(1, false)],
            20_252_026,
            Utc.with_ymd_and_hms(2025, 10, 2, 12, 0, 0).unwrap(),
            "https://api-web.nhle.com",
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("0 of 1"));
    }
}
