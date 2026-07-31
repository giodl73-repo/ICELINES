//! Retrospective point-in-time reconstruction from MoneyPuck team-game files.
//!
//! This builder intentionally leaves lineup availability and starter quality
//! unavailable. It creates an xG/special-teams challenger for ablation and
//! source-coverage work, but cannot pass the full production promotion gate.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, TimeZone, Utc};
use icelines_core::{
    TeamGameEvidenceState, TeamGameForecastView, TeamGameForecastVintage,
    TeamGamePredictionEvidenceInput, TeamGamePredictionTeamEvidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    derive_opponent_adjusted_xg_form, derive_trailing_special_teams_form, derive_trailing_xg_form,
    game_prediction_edge_package::{
        GamePredictionEdgeEvidencePackage, GamePredictionEdgePackageError,
        GamePredictionEvidenceSource, GamePredictionEvidenceSourceAuthority,
    },
    parse_moneypuck_team_games, rank_special_teams_forms, GamePredictionSpecialTeamsScore,
    MoneyPuckTeamGameError, MoneyPuckTeamGameRow,
};

#[derive(Debug, Clone, PartialEq)]
pub struct HistoricalMoneyPuckTeamInput {
    pub team: String,
    pub csv_text: String,
    pub source_uri: String,
    pub retrieved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalPredictionEdgeBuildResult {
    pub package: GamePredictionEdgeEvidencePackage,
    pub games: usize,
    pub roster_sides: usize,
    pub xg_sides: usize,
    pub opponent_adjusted_xg_sides: usize,
    pub special_teams_sides: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum HistoricalPredictionEdgeError {
    #[error("invalid historical edge input: {0}")]
    Invalid(String),
    #[error(transparent)]
    MoneyPuck(#[from] MoneyPuckTeamGameError),
    #[error(transparent)]
    Package(#[from] GamePredictionEdgePackageError),
    #[error("historical edge serialization failed: {0}")]
    Serialization(String),
}

pub fn build_historical_moneypuck_edge_package(
    forecast: &TeamGameForecastView,
    inputs: Vec<HistoricalMoneyPuckTeamInput>,
    created_at: DateTime<Utc>,
    trailing_games: usize,
) -> Result<HistoricalPredictionEdgeBuildResult, HistoricalPredictionEdgeError> {
    if trailing_games == 0
        || forecast.games.is_empty()
        || forecast.schedule_games != forecast.games.len()
    {
        return Err(HistoricalPredictionEdgeError::Invalid(
            "forecast must be complete and trailing window must be positive".to_owned(),
        ));
    }
    let source_forecast_fingerprint = fingerprint(forecast)?;
    let mut teams = BTreeMap::<String, ParsedTeam>::new();
    for input in inputs {
        let team = input.team.trim().to_ascii_uppercase();
        if team.is_empty() || input.source_uri.trim().is_empty() || input.retrieved_at > created_at
        {
            return Err(HistoricalPredictionEdgeError::Invalid(
                "MoneyPuck team identity, URI, or retrieval time is invalid".to_owned(),
            ));
        }
        let rows = parse_moneypuck_team_games(&input.csv_text)?;
        if rows.is_empty() || rows.iter().any(|row| row.team != team) || teams.contains_key(&team) {
            return Err(HistoricalPredictionEdgeError::Invalid(format!(
                "MoneyPuck rows do not uniquely belong to {team}"
            )));
        }
        teams.insert(
            team,
            ParsedTeam {
                rows,
                source_uri: input.source_uri,
                retrieved_at: input.retrieved_at,
            },
        );
    }
    let scheduled_teams = forecast
        .games
        .iter()
        .flat_map(|game| [game.away_team.as_str(), game.home_team.as_str()])
        .collect::<BTreeSet<_>>();
    let missing = scheduled_teams
        .iter()
        .filter(|team| !teams.contains_key(**team))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(HistoricalPredictionEdgeError::Invalid(format!(
            "MoneyPuck team files missing: {}",
            missing.join(", ")
        )));
    }

    let opening = forecast
        .opening_strengths
        .iter()
        .map(|row| (row.team.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let unique_dates = forecast
        .games
        .iter()
        .map(|game| game.date)
        .collect::<BTreeSet<_>>();
    let mut special_by_date = BTreeMap::new();
    for date in unique_dates.iter().copied() {
        let forms = teams
            .values()
            .filter_map(|team| {
                derive_trailing_special_teams_form(
                    &team.rows,
                    team.rows[0].team.as_str(),
                    date,
                    trailing_games,
                )
                .ok()
            })
            .collect::<Vec<_>>();
        if let Ok(scores) = rank_special_teams_forms(&forms) {
            special_by_date.insert(
                date,
                scores
                    .into_iter()
                    .map(|score| (score.team.clone(), score))
                    .collect::<BTreeMap<_, _>>(),
            );
        }
    }
    let rows_by_team = teams
        .iter()
        .map(|(team, parsed)| (team.clone(), parsed.rows.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let latest_retrieved_at = teams
        .values()
        .map(|team| team.retrieved_at)
        .max()
        .expect("scheduled teams are non-empty");
    let opponent_adjusted_by_team_date = forecast
        .games
        .iter()
        .flat_map(|game| {
            [game.away_team.as_str(), game.home_team.as_str()]
                .into_iter()
                .map(move |team| (team, game.date))
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|(team, date)| {
            derive_opponent_adjusted_xg_form(
                &teams[team].rows,
                &rows_by_team,
                team,
                date,
                trailing_games,
            )
            .ok()
            .map(|form| ((team.to_owned(), date), form))
        })
        .collect::<BTreeMap<_, _>>();

    let mut sources = BTreeMap::<String, GamePredictionEvidenceSource>::new();
    let mut evidence = Vec::with_capacity(forecast.games.len());
    let mut warnings = Vec::new();
    let mut roster_sides = 0;
    let mut xg_sides = 0;
    let mut opponent_adjusted_xg_sides = 0;
    let mut special_teams_sides = 0;
    for game in &forecast.games {
        let forecast_at = Utc
            .with_ymd_and_hms(
                game.date.year(),
                game.date.month(),
                game.date.day(),
                12,
                0,
                0,
            )
            .single()
            .expect("valid forecast date");
        let build_side = |team: &str,
                          sources: &mut BTreeMap<String, GamePredictionEvidenceSource>,
                          warnings: &mut Vec<String>,
                          roster_sides: &mut usize,
                          xg_sides: &mut usize,
                          opponent_adjusted_xg_sides: &mut usize,
                          special_teams_sides: &mut usize|
         -> TeamGamePredictionTeamEvidence {
            let parsed = &teams[team];
            let roster = opening
                .get(team)
                .filter(|row| row.value_coverage >= 0.70 && (0.0..=100.0).contains(&row.strength));
            let roster_fingerprint = roster.and_then(|row| fingerprint(*row).ok());
            if let (Some(row), Some(fingerprint)) = (roster, roster_fingerprint.as_ref()) {
                *roster_sides += 1;
                sources.entry(fingerprint.clone()).or_insert_with(|| {
                    GamePredictionEvidenceSource {
                        source_key: format!("icelines.opening_roster.{team}"),
                        evidence_cutoff_at: forecast_at,
                        retrieved_at: created_at,
                        authority: GamePredictionEvidenceSourceAuthority::HistoricalReconstruction,
                        source_uri: "icelines:team_game_forecast/opening_strengths".to_owned(),
                        fingerprint: fingerprint.clone(),
                    }
                });
                let _ = row;
            }
            let xg = derive_trailing_xg_form(&parsed.rows, team, game.date, trailing_games).ok();
            if let Some(form) = &xg {
                *xg_sides += 1;
                sources
                    .entry(form.source_fingerprint.clone())
                    .or_insert_with(|| GamePredictionEvidenceSource {
                        source_key: format!("moneypuck.xg.{team}.{}", game.game_id),
                        evidence_cutoff_at: forecast_at,
                        retrieved_at: parsed.retrieved_at,
                        authority: GamePredictionEvidenceSourceAuthority::HistoricalReconstruction,
                        source_uri: parsed.source_uri.clone(),
                        fingerprint: form.source_fingerprint.clone(),
                    });
            } else {
                warnings.push(format!(
                    "game {} {team} has no trailing xG form",
                    game.game_id
                ));
            }
            let opponent_adjusted_xg =
                opponent_adjusted_by_team_date.get(&(team.to_owned(), game.date));
            if let Some(form) = opponent_adjusted_xg {
                *opponent_adjusted_xg_sides += 1;
                sources
                    .entry(form.source_fingerprint.clone())
                    .or_insert_with(|| GamePredictionEvidenceSource {
                        source_key: format!(
                            "moneypuck.opponent_adjusted_xg.{team}.{}",
                            game.game_id
                        ),
                        evidence_cutoff_at: forecast_at,
                        retrieved_at: latest_retrieved_at,
                        authority: GamePredictionEvidenceSourceAuthority::HistoricalReconstruction,
                        source_uri: "icelines:moneypuck/opponent-adjusted-xg.v1".to_owned(),
                        fingerprint: form.source_fingerprint.clone(),
                    });
            }
            let special: Option<&GamePredictionSpecialTeamsScore> = special_by_date
                .get(&game.date)
                .and_then(|scores| scores.get(team));
            if let Some(form) = special {
                *special_teams_sides += 1;
                sources
                    .entry(form.source_fingerprint.clone())
                    .or_insert_with(|| GamePredictionEvidenceSource {
                        source_key: format!("moneypuck.special_teams.{team}.{}", game.game_id),
                        evidence_cutoff_at: forecast_at,
                        retrieved_at: parsed.retrieved_at,
                        authority: GamePredictionEvidenceSourceAuthority::HistoricalReconstruction,
                        source_uri: parsed.source_uri.clone(),
                        fingerprint: form.source_fingerprint.clone(),
                    });
            }
            let mut source_fingerprints = Vec::new();
            if let Some(fingerprint) = roster_fingerprint {
                source_fingerprints.push(fingerprint);
            }
            if let Some(form) = &xg {
                source_fingerprints.push(form.source_fingerprint.clone());
            }
            if let Some(form) = opponent_adjusted_xg {
                source_fingerprints.push(form.source_fingerprint.clone());
            }
            if let Some(form) = special {
                source_fingerprints.push(form.source_fingerprint.clone());
            }
            source_fingerprints.sort();
            source_fingerprints.dedup();
            TeamGamePredictionTeamEvidence {
                team: team.to_owned(),
                roster_strength: roster.map(|row| row.strength),
                roster_state: if roster.is_some() {
                    TeamGameEvidenceState::Modeled
                } else {
                    TeamGameEvidenceState::Unavailable
                },
                availability_strength: None,
                availability_state: TeamGameEvidenceState::Unavailable,
                lineup_impact: None,
                lineup_impact_state: TeamGameEvidenceState::Unavailable,
                goalie_quality: None,
                goalie_state: TeamGameEvidenceState::Unavailable,
                goalie_player_id: None,
                goalie_form_quality: None,
                goalie_form_appearances: 0,
                goalie_form_state: TeamGameEvidenceState::Unavailable,
                goalie_workload_readiness: None,
                xg_share: xg.as_ref().map(|form| form.xg_share),
                xg_games: xg.as_ref().map_or(0, |form| form.games),
                opponent_adjusted_xg_share: opponent_adjusted_xg.map(|form| form.adjusted_xg_share),
                opponent_adjusted_xg_games: opponent_adjusted_xg.map_or(0, |form| form.games),
                special_teams_strength: special.map(|form| form.strength),
                special_teams_games: special.map_or(0, |form| form.games),
                matchup_suitability: None,
                matchup_state: TeamGameEvidenceState::Unavailable,
                source_fingerprints,
            }
        };
        evidence.push(TeamGamePredictionEvidenceInput {
            game_id: game.game_id,
            forecast_at,
            captured_at: forecast_at,
            away: build_side(
                &game.away_team,
                &mut sources,
                &mut warnings,
                &mut roster_sides,
                &mut xg_sides,
                &mut opponent_adjusted_xg_sides,
                &mut special_teams_sides,
            ),
            home: build_side(
                &game.home_team,
                &mut sources,
                &mut warnings,
                &mut roster_sides,
                &mut xg_sides,
                &mut opponent_adjusted_xg_sides,
                &mut special_teams_sides,
            ),
        });
    }
    warnings.sort();
    warnings.dedup();
    let package = GamePredictionEdgeEvidencePackage::build(
        forecast.season,
        TeamGameForecastVintage::GameMorning,
        created_at,
        source_forecast_fingerprint,
        sources.into_values().collect(),
        evidence,
    )?;
    Ok(HistoricalPredictionEdgeBuildResult {
        package,
        games: forecast.games.len(),
        roster_sides,
        xg_sides,
        opponent_adjusted_xg_sides,
        special_teams_sides,
        warnings,
    })
}

struct ParsedTeam {
    rows: Vec<MoneyPuckTeamGameRow>,
    source_uri: String,
    retrieved_at: DateTime<Utc>,
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, HistoricalPredictionEdgeError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| HistoricalPredictionEdgeError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}
