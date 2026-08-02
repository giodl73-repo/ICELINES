//! Dated, replayable evidence packages for the IceCast game-prediction edge.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::{DateTime, Utc};
use icelines_core::{
    TeamGameEvidenceState, TeamGameForecastView, TeamGameForecastVintage,
    TeamGameOpeningStrengthRow, TeamGamePredictionEvidenceInput,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::atomic_write::write_json_atomic;
use crate::game_prediction_edge_assembler::{
    GamePredictionEvidenceAssemblerError, GamePredictionGameAssemblyInput,
    GamePredictionTeamAssemblyInput,
};

pub const GAME_PREDICTION_EDGE_PACKAGE_SCHEMA: &str = "game_prediction_edge_evidence_package.v1";
pub const GAME_PREDICTION_EDGE_PACKAGE_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/game_prediction_edge_evidence_package.v1.schema.json");
const PACKAGE_FLOAT_SCALE: f64 = 1_000_000_000.0;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePredictionEvidenceSourceAuthority {
    LiveCapture,
    HistoricalReconstruction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GamePredictionEvidenceSource {
    /// Stable feature/source name, for example `official.roster` or `moneypuck.xg`.
    pub source_key: String,
    pub evidence_cutoff_at: DateTime<Utc>,
    pub retrieved_at: DateTime<Utc>,
    pub authority: GamePredictionEvidenceSourceAuthority,
    pub source_uri: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionEdgeEvidencePackage {
    pub schema: String,
    pub season: u32,
    pub vintage: TeamGameForecastVintage,
    pub created_at: DateTime<Utc>,
    pub source_forecast_fingerprint: String,
    pub sources: Vec<GamePredictionEvidenceSource>,
    pub games: Vec<TeamGamePredictionEvidenceInput>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionEvidencePackageBuildInput {
    pub season: u32,
    pub vintage: TeamGameForecastVintage,
    pub created_at: DateTime<Utc>,
    pub sources: Vec<GamePredictionEvidenceSource>,
    pub games: Vec<GamePredictionGameAssemblyInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePredictionEvidencePackageBuildResult {
    pub package: GamePredictionEdgeEvidencePackage,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum GamePredictionEdgePackageError {
    #[error("invalid game-prediction evidence package: {0}")]
    Invalid(String),
    #[error("game-prediction evidence package fingerprint mismatch")]
    FingerprintMismatch,
    #[error("could not read game-prediction evidence package: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not decode game-prediction evidence package: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Assembly(#[from] GamePredictionEvidenceAssemblerError),
}

pub fn build_game_prediction_edge_evidence_package(
    source_forecast_fingerprint: impl Into<String>,
    input: GamePredictionEvidencePackageBuildInput,
) -> Result<GamePredictionEvidencePackageBuildResult, GamePredictionEdgePackageError> {
    let mut games = Vec::with_capacity(input.games.len());
    let mut warnings = Vec::new();
    for game in input.games {
        let assembled = game.assemble()?;
        warnings.extend(assembled.warnings);
        games.push(assembled.evidence);
    }
    warnings.sort();
    warnings.dedup();
    let package = GamePredictionEdgeEvidencePackage::build(
        input.season,
        input.vintage,
        input.created_at,
        source_forecast_fingerprint,
        input.sources,
        games,
    )?;
    Ok(GamePredictionEvidencePackageBuildResult { package, warnings })
}

/// Seal the authoritative opening-roster strengths already carried by a
/// complete preseason forecast as point-in-time evidence for every game.
pub fn build_preseason_game_prediction_edge_evidence_package(
    forecast: &TeamGameForecastView,
    created_at: DateTime<Utc>,
) -> Result<GamePredictionEvidencePackageBuildResult, GamePredictionEdgePackageError> {
    let authority = forecast.opening_roster_authority.as_ref().ok_or_else(|| {
        GamePredictionEdgePackageError::Invalid(
            "preseason forecast has no opening-roster authority".to_owned(),
        )
    })?;
    if authority.status != "authoritative" || !authority.player_value_effects_enabled {
        return Err(GamePredictionEdgePackageError::Invalid(
            "preseason evidence requires authoritative player-valued opening rosters".to_owned(),
        ));
    }
    if created_at.date_naive() >= forecast.schedule_start {
        return Err(GamePredictionEdgePackageError::Invalid(
            "preseason evidence must be created before the schedule starts".to_owned(),
        ));
    }
    let captured_at = authority
        .selected_snapshot_created_at
        .as_deref()
        .ok_or_else(|| {
            GamePredictionEdgePackageError::Invalid(
                "opening-roster authority has no selected snapshot timestamp".to_owned(),
            )
        })?
        .parse::<DateTime<Utc>>()
        .map_err(|error| {
            GamePredictionEdgePackageError::Invalid(format!(
                "opening-roster snapshot timestamp is invalid: {error}"
            ))
        })?;
    if captured_at > created_at {
        return Err(GamePredictionEdgePackageError::Invalid(
            "opening-roster snapshot is later than the preseason evidence freeze".to_owned(),
        ));
    }
    let strengths = forecast
        .opening_strengths
        .iter()
        .map(|row| (row.team.trim().to_ascii_uppercase(), row))
        .collect::<BTreeMap<_, _>>();
    let scheduled_teams = forecast
        .games
        .iter()
        .flat_map(|game| [&game.away_team, &game.home_team])
        .map(|team| team.trim().to_ascii_uppercase())
        .collect::<BTreeSet<_>>();
    if strengths.len() != forecast.opening_strengths.len()
        || scheduled_teams
            .iter()
            .any(|team| !strengths.contains_key(team))
    {
        return Err(GamePredictionEdgePackageError::Invalid(
            "opening strengths must contain one row for every scheduled team".to_owned(),
        ));
    }

    let source_bytes = serde_json::to_vec(&(authority, &forecast.opening_strengths))?;
    let source_fingerprint = format!("sha256:{:x}", Sha256::digest(source_bytes));
    let source_uri = format!(
        "icelines://opening-rosters/{}",
        authority
            .selected_snapshot
            .as_deref()
            .unwrap_or("authoritative-snapshot")
    );
    let source = GamePredictionEvidenceSource {
        source_key: "icelines.opening_roster_strength".to_owned(),
        evidence_cutoff_at: captured_at,
        retrieved_at: captured_at,
        authority: GamePredictionEvidenceSourceAuthority::LiveCapture,
        source_uri,
        fingerprint: source_fingerprint.clone(),
    };
    let games = forecast
        .games
        .iter()
        .map(|game| GamePredictionGameAssemblyInput {
            game_id: game.game_id,
            forecast_at: created_at,
            captured_at,
            away: preseason_team_input(
                &game.away_team,
                strengths[&game.away_team.trim().to_ascii_uppercase()],
                &source_fingerprint,
            ),
            home: preseason_team_input(
                &game.home_team,
                strengths[&game.home_team.trim().to_ascii_uppercase()],
                &source_fingerprint,
            ),
        })
        .collect();
    let forecast_fingerprint =
        format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(forecast)?));
    let mut result = build_game_prediction_edge_evidence_package(
        forecast_fingerprint,
        GamePredictionEvidencePackageBuildInput {
            season: forecast.season,
            vintage: TeamGameForecastVintage::Preseason,
            created_at,
            sources: vec![source],
            games,
        },
    )?;
    result
        .warnings
        .retain(|warning| !warning.ends_with("has no point-in-time eligible trailing xG form"));
    result.warnings.push(
        "preseason opening-roster evidence intentionally leaves lineup, goalie, xG, special-teams, and matchup signals unavailable"
            .to_owned(),
    );
    result.warnings.sort();
    Ok(result)
}

fn preseason_team_input(
    team: &str,
    strength: &TeamGameOpeningStrengthRow,
    source_fingerprint: &str,
) -> GamePredictionTeamAssemblyInput {
    GamePredictionTeamAssemblyInput {
        team: team.to_owned(),
        opening_strength: Some(strength.clone()),
        roster_state: TeamGameEvidenceState::Confirmed,
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
        source_fingerprints: vec![source_fingerprint.to_owned()],
    }
}

impl GamePredictionEdgeEvidencePackage {
    pub fn build(
        season: u32,
        vintage: TeamGameForecastVintage,
        created_at: DateTime<Utc>,
        source_forecast_fingerprint: impl Into<String>,
        mut sources: Vec<GamePredictionEvidenceSource>,
        mut games: Vec<TeamGamePredictionEvidenceInput>,
    ) -> Result<Self, GamePredictionEdgePackageError> {
        sources.sort_by(|left, right| left.source_key.cmp(&right.source_key));
        games.sort_by_key(|game| game.game_id);
        normalize_game_floats(&mut games);
        let mut package = Self {
            schema: GAME_PREDICTION_EDGE_PACKAGE_SCHEMA.to_owned(),
            season,
            vintage,
            created_at,
            source_forecast_fingerprint: source_forecast_fingerprint.into(),
            sources,
            games,
            fingerprint: String::new(),
        };
        package.validate_content()?;
        package.fingerprint = package_fingerprint(&package)?;
        Ok(package)
    }

    pub fn validate(&self) -> Result<(), GamePredictionEdgePackageError> {
        self.validate_content()?;
        if self.fingerprint != package_fingerprint(self)? {
            return Err(GamePredictionEdgePackageError::FingerprintMismatch);
        }
        Ok(())
    }

    fn validate_content(&self) -> Result<(), GamePredictionEdgePackageError> {
        if self.schema != GAME_PREDICTION_EDGE_PACKAGE_SCHEMA || self.season < 20_000_000 {
            return Err(GamePredictionEdgePackageError::Invalid(
                "schema or season is invalid".to_owned(),
            ));
        }
        if !valid_sha256(&self.source_forecast_fingerprint) {
            return Err(GamePredictionEdgePackageError::Invalid(
                "source forecast fingerprint must be sha256 authority".to_owned(),
            ));
        }
        let mut source_keys = BTreeSet::new();
        let mut authority = BTreeMap::new();
        for source in &self.sources {
            if source.source_key.trim().is_empty()
                || source.source_uri.trim().is_empty()
                || !valid_sha256(&source.fingerprint)
                || source.evidence_cutoff_at > source.retrieved_at
                || source.retrieved_at > self.created_at
                || (source.authority == GamePredictionEvidenceSourceAuthority::LiveCapture
                    && source.retrieved_at > source.evidence_cutoff_at)
                || !source_keys.insert(source.source_key.as_str())
            {
                return Err(GamePredictionEdgePackageError::Invalid(
                    "source rows require unique keys, URI, dated capture, and sha256 fingerprint"
                        .to_owned(),
                ));
            }
            authority.insert(source.fingerprint.as_str(), source.evidence_cutoff_at);
        }
        let mut game_ids = BTreeSet::new();
        for game in &self.games {
            if !game_ids.insert(game.game_id) {
                return Err(GamePredictionEdgePackageError::Invalid(format!(
                    "duplicate game {}",
                    game.game_id
                )));
            }
            if game.captured_at > game.forecast_at || game.forecast_at > self.created_at {
                return Err(GamePredictionEdgePackageError::Invalid(format!(
                    "game {} crosses its dated forecast boundary",
                    game.game_id
                )));
            }
            if !game_floats_are_canonical(game) {
                return Err(GamePredictionEdgePackageError::Invalid(format!(
                    "game {} contains non-canonical floating-point evidence",
                    game.game_id
                )));
            }
            let fingerprints = game
                .away
                .source_fingerprints
                .iter()
                .chain(&game.home.source_fingerprints);
            for fingerprint in fingerprints {
                let Some(evidence_cutoff_at) = authority.get(fingerprint.as_str()) else {
                    return Err(GamePredictionEdgePackageError::Invalid(format!(
                        "game {} references an undeclared source fingerprint",
                        game.game_id
                    )));
                };
                if *evidence_cutoff_at > game.forecast_at {
                    return Err(GamePredictionEdgePackageError::Invalid(format!(
                        "game {} references a source captured after its forecast",
                        game.game_id
                    )));
                }
            }
        }
        Ok(())
    }
}

pub fn store_game_prediction_edge_evidence_package(
    path: &Path,
    package: &GamePredictionEdgeEvidencePackage,
) -> Result<(), GamePredictionEdgePackageError> {
    package.validate()?;
    write_json_atomic(path, package)?;
    Ok(())
}

pub fn load_game_prediction_edge_evidence_package(
    path: &Path,
) -> Result<GamePredictionEdgeEvidencePackage, GamePredictionEdgePackageError> {
    let package =
        serde_json::from_slice::<GamePredictionEdgeEvidencePackage>(&std::fs::read(path)?)?;
    package.validate()?;
    Ok(package)
}

fn package_fingerprint(
    package: &GamePredictionEdgeEvidencePackage,
) -> Result<String, GamePredictionEdgePackageError> {
    let mut canonical = package.clone();
    canonical.fingerprint.clear();
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn normalize_game_floats(games: &mut [TeamGamePredictionEvidenceInput]) {
    for game in games {
        normalize_team_floats(&mut game.away);
        normalize_team_floats(&mut game.home);
    }
}

fn normalize_team_floats(team: &mut icelines_core::TeamGamePredictionTeamEvidence) {
    for value in [
        &mut team.roster_strength,
        &mut team.availability_strength,
        &mut team.lineup_impact,
        &mut team.goalie_quality,
        &mut team.goalie_form_quality,
        &mut team.goalie_workload_readiness,
        &mut team.xg_share,
        &mut team.opponent_adjusted_xg_share,
        &mut team.special_teams_strength,
        &mut team.matchup_suitability,
    ] {
        *value = value.map(canonical_float);
    }
}

fn game_floats_are_canonical(game: &TeamGamePredictionEvidenceInput) -> bool {
    [&game.away, &game.home]
        .into_iter()
        .flat_map(|team| {
            [
                team.roster_strength,
                team.availability_strength,
                team.lineup_impact,
                team.goalie_quality,
                team.goalie_form_quality,
                team.goalie_workload_readiness,
                team.xg_share,
                team.opponent_adjusted_xg_share,
                team.special_teams_strength,
                team.matchup_suitability,
            ]
        })
        .flatten()
        .all(|value| value == canonical_float(value))
}

fn canonical_float(value: f64) -> f64 {
    let normalized = (value * PACKAGE_FLOAT_SCALE).round() / PACKAGE_FLOAT_SCALE;
    if normalized == 0.0 {
        0.0
    } else {
        normalized
    }
}

fn valid_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|digest| digest.len() == 64 && digest.chars().all(|ch| ch.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use icelines_core::{
        build_team_game_forecast, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput, TeamGameEvidenceState, TeamGameOpeningRosterAuthorityRow,
        TeamGameOpeningStrengthRow, TeamGamePredictionTeamEvidence,
    };

    use super::*;

    fn sha(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn package() -> GamePredictionEdgeEvidencePackage {
        let captured_at = Utc.with_ymd_and_hms(2026, 10, 8, 15, 0, 0).unwrap();
        let forecast_at = Utc.with_ymd_and_hms(2026, 10, 8, 16, 0, 0).unwrap();
        let team = |name: &str| TeamGamePredictionTeamEvidence {
            team: name.to_owned(),
            roster_strength: Some(55.0),
            roster_state: TeamGameEvidenceState::Reported,
            availability_strength: Some(54.0),
            availability_state: TeamGameEvidenceState::Reported,
            lineup_impact: None,
            lineup_impact_state: TeamGameEvidenceState::Unavailable,
            goalie_quality: None,
            goalie_state: TeamGameEvidenceState::Unavailable,
            goalie_player_id: None,
            goalie_form_quality: None,
            goalie_form_appearances: 0,
            goalie_form_state: TeamGameEvidenceState::Unavailable,
            goalie_workload_readiness: None,
            // Regression input: serde_json's standard parser reparses this
            // long decimal one ULP lower unless the package canonicalizes it.
            xg_share: Some(0.495_452_944_710_293_55),
            xg_games: 8,
            opponent_adjusted_xg_share: None,
            opponent_adjusted_xg_games: 0,
            special_teams_strength: None,
            special_teams_games: 0,
            matchup_suitability: None,
            matchup_state: TeamGameEvidenceState::Unavailable,
            source_fingerprints: vec![sha('b')],
        };
        GamePredictionEdgeEvidencePackage::build(
            20_262_027,
            TeamGameForecastVintage::GameMorning,
            Utc.with_ymd_and_hms(2026, 10, 8, 17, 0, 0).unwrap(),
            sha('a'),
            vec![GamePredictionEvidenceSource {
                source_key: "official.roster".to_owned(),
                evidence_cutoff_at: captured_at,
                retrieved_at: captured_at,
                authority: GamePredictionEvidenceSourceAuthority::LiveCapture,
                source_uri: "https://api-web.nhle.com/roster".to_owned(),
                fingerprint: sha('b'),
            }],
            vec![TeamGamePredictionEvidenceInput {
                game_id: 1,
                forecast_at,
                captured_at,
                away: team("NYR"),
                home: team("SEA"),
            }],
        )
        .unwrap()
    }

    fn preseason_forecast() -> TeamGameForecastView {
        let mut forecast = build_team_game_forecast(
            20_262_027,
            vec![TeamForecastGameInput {
                game_id: 1,
                date: chrono::NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
                away_team: "SEA".to_owned(),
                home_team: "NYR".to_owned(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            }],
            vec![
                TeamForecastStrengthInput {
                    team: "SEA".to_owned(),
                    strength: 49.0,
                },
                TeamForecastStrengthInput {
                    team: "NYR".to_owned(),
                    strength: 52.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap();
        forecast.opening_roster_authority = Some(TeamGameOpeningRosterAuthorityRow {
            status: "authoritative".to_owned(),
            required_before_date: forecast.schedule_start,
            selected_snapshot: Some("20262027-2026-07-29-rosters".to_owned()),
            selected_snapshot_created_at: Some("2026-07-29T04:00:12Z".to_owned()),
            latest_observed_snapshot: Some("20262027-2026-07-29-rosters".to_owned()),
            latest_observed_snapshot_created_at: Some("2026-07-29T04:00:12Z".to_owned()),
            expected_teams: 2,
            verified_teams: 2,
            verified_team_abbrevs: vec!["NYR".to_owned(), "SEA".to_owned()],
            player_value_effects_enabled: true,
            personnel_events_effective_after: Some(
                chrono::NaiveDate::from_ymd_opt(2026, 7, 29).unwrap(),
            ),
            reason: "test authority".to_owned(),
        });
        forecast.opening_strengths = [("NYR", 52.0), ("SEA", 49.0)]
            .into_iter()
            .map(|(team, strength)| TeamGameOpeningStrengthRow {
                team: team.to_owned(),
                as_of_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 7, 29).unwrap()),
                strength,
                cohort_normalization_delta: 0.0,
                roster_players: 23,
                valued_players: 22,
                value_coverage: 22.0 / 23.0,
                forwards_used: 12,
                defensemen_used: 6,
                goalies_used: 2,
                players: Vec::new(),
            })
            .collect();
        forecast
    }

    #[test]
    fn l0_package_is_canonical_and_sealed() {
        let package = package();
        assert_eq!(package.games[0].away.xg_share, Some(0.495_452_945));
        package.validate().unwrap();
        assert!(package.fingerprint.starts_with("sha256:"));
    }

    #[test]
    fn l0_late_source_authority_is_refused() {
        let mut package = package();
        package.sources[0].evidence_cutoff_at =
            Utc.with_ymd_and_hms(2026, 10, 8, 16, 1, 0).unwrap();
        package.sources[0].retrieved_at = package.sources[0].evidence_cutoff_at;
        assert!(matches!(
            package.validate(),
            Err(GamePredictionEdgePackageError::Invalid(_))
        ));
    }

    #[test]
    fn l0_historical_retrieval_may_postdate_frozen_evidence_cutoff() {
        let mut package = package();
        package.sources[0].authority =
            GamePredictionEvidenceSourceAuthority::HistoricalReconstruction;
        package.sources[0].retrieved_at = package.created_at;
        package.fingerprint = package_fingerprint(&package).unwrap();
        package.validate().unwrap();
    }

    #[test]
    fn l1_store_load_round_trip_validates_fingerprint() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("edge-evidence.json");
        let package = package();
        store_game_prediction_edge_evidence_package(&path, &package).unwrap();
        assert_eq!(
            load_game_prediction_edge_evidence_package(&path).unwrap(),
            package
        );
        assert!(!dir.path().join("edge-evidence.json.tmp").exists());
    }

    #[test]
    fn l1_preseason_package_reuses_authoritative_opening_strengths() {
        let forecast = preseason_forecast();
        let result = build_preseason_game_prediction_edge_evidence_package(
            &forecast,
            Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0).unwrap(),
        )
        .unwrap();
        assert_eq!(result.package.vintage, TeamGameForecastVintage::Preseason);
        assert_eq!(result.package.games.len(), 1);
        assert_eq!(result.package.games[0].away.roster_strength, Some(49.0));
        assert_eq!(
            result.package.games[0].home.roster_state,
            TeamGameEvidenceState::Confirmed
        );
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("intentionally leaves lineup"));
        result.package.validate().unwrap();
    }

    #[test]
    fn l1_preseason_package_refuses_incomplete_team_coverage() {
        let mut forecast = preseason_forecast();
        forecast.opening_strengths.retain(|row| row.team == "NYR");
        let error = build_preseason_game_prediction_edge_evidence_package(
            &forecast,
            Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0).unwrap(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("every scheduled team"));
    }
}
