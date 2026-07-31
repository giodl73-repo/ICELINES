//! Historical, point-in-time goalie form enrichment for confirmed edge packages.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use icelines_core::{
    TeamGameEvidenceState, TeamGameForecastView, TeamGameForecastVintage,
    TeamGamePredictionTeamEvidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    derive_trailing_goalie_form,
    game_prediction_edge_package::{
        GamePredictionEdgeEvidencePackage, GamePredictionEdgePackageError,
        GamePredictionEvidenceSource, GamePredictionEvidenceSourceAuthority,
    },
    parse_moneypuck_goalie_games, MoneyPuckGoalieGameError, MoneyPuckGoalieGameRow,
};

#[derive(Debug, Clone, PartialEq)]
pub struct HistoricalMoneyPuckGoalieInput {
    pub player_id: u32,
    pub csv_text: String,
    pub source_uri: String,
    pub retrieved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalGoalieEdgeBuildResult {
    pub package: GamePredictionEdgeEvidencePackage,
    pub games: usize,
    pub requested_goalies: usize,
    pub form_sides: usize,
    pub workload_sides: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum HistoricalGoalieEdgeError {
    #[error("invalid historical goalie edge input: {0}")]
    Invalid(String),
    #[error(transparent)]
    Goalie(#[from] MoneyPuckGoalieGameError),
    #[error(transparent)]
    Package(#[from] GamePredictionEdgePackageError),
    #[error("historical goalie edge serialization failed: {0}")]
    Serialization(String),
}

struct ParsedGoalie {
    rows: Vec<MoneyPuckGoalieGameRow>,
    source_uri: String,
    retrieved_at: DateTime<Utc>,
}

pub fn build_historical_goalie_edge_package(
    forecast: &TeamGameForecastView,
    confirmed: &GamePredictionEdgeEvidencePackage,
    inputs: Vec<HistoricalMoneyPuckGoalieInput>,
    trailing_appearances: usize,
    created_at: DateTime<Utc>,
) -> Result<HistoricalGoalieEdgeBuildResult, HistoricalGoalieEdgeError> {
    confirmed.validate()?;
    let forecast_fingerprint = fingerprint(forecast)?;
    if trailing_appearances == 0
        || confirmed.season != forecast.season
        || confirmed.vintage != TeamGameForecastVintage::PregameConfirmed
        || confirmed.source_forecast_fingerprint != forecast_fingerprint
        || confirmed.games.len() != forecast.games.len()
    {
        return Err(HistoricalGoalieEdgeError::Invalid(
            "confirmed package must be complete, pregame-confirmed, and bind the exact forecast"
                .to_owned(),
        ));
    }
    let required_ids = confirmed
        .games
        .iter()
        .flat_map(|game| [&game.away, &game.home])
        .map(|team| {
            team.goalie_player_id.ok_or_else(|| {
                HistoricalGoalieEdgeError::Invalid(format!(
                    "game-side {} has no confirmed goalie player ID",
                    team.team
                ))
            })
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut parsed = BTreeMap::new();
    for input in inputs {
        if input.player_id == 0
            || input.source_uri.trim().is_empty()
            || input.retrieved_at > created_at
            || !required_ids.contains(&input.player_id)
            || parsed.contains_key(&input.player_id)
        {
            return Err(HistoricalGoalieEdgeError::Invalid(format!(
                "goalie {} has invalid identity, URI, retrieval time, or duplication",
                input.player_id
            )));
        }
        let rows = parse_moneypuck_goalie_games(&input.csv_text)?;
        if rows.iter().any(|row| row.player_id != input.player_id) {
            return Err(HistoricalGoalieEdgeError::Invalid(format!(
                "goalie file {} contains a different player identity",
                input.player_id
            )));
        }
        parsed.insert(
            input.player_id,
            ParsedGoalie {
                rows,
                source_uri: input.source_uri,
                retrieved_at: input.retrieved_at,
            },
        );
    }
    let missing = required_ids
        .iter()
        .filter(|player_id| !parsed.contains_key(player_id))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(HistoricalGoalieEdgeError::Invalid(format!(
            "MoneyPuck files missing for {} confirmed goalies (first: {:?})",
            missing.len(),
            &missing[..missing.len().min(10)]
        )));
    }

    let dates = forecast
        .games
        .iter()
        .map(|game| (game.game_id, game.date))
        .collect::<BTreeMap<_, _>>();
    let mut sources = confirmed.sources.clone();
    let mut games = confirmed.games.clone();
    let mut warnings = Vec::new();
    let mut form_sides = 0;
    let mut workload_sides = 0;
    for game in &mut games {
        let date = dates.get(&game.game_id).ok_or_else(|| {
            HistoricalGoalieEdgeError::Invalid(format!(
                "confirmed game {} is absent from forecast",
                game.game_id
            ))
        })?;
        for evidence in [&mut game.away, &mut game.home] {
            let player_id = evidence
                .goalie_player_id
                .expect("required goalie IDs were checked");
            let goalie = &parsed[&player_id];
            match derive_trailing_goalie_form(&goalie.rows, player_id, *date, trailing_appearances)
            {
                Ok(form) => {
                    apply_form(evidence, &form);
                    form_sides += 1;
                    workload_sides += 1;
                    sources.push(GamePredictionEvidenceSource {
                        source_key: format!("moneypuck.goalie_form.{}.{}", game.game_id, player_id),
                        evidence_cutoff_at: game.forecast_at,
                        retrieved_at: goalie.retrieved_at,
                        authority: GamePredictionEvidenceSourceAuthority::HistoricalReconstruction,
                        source_uri: goalie.source_uri.clone(),
                        fingerprint: form.source_fingerprint,
                    });
                }
                Err(MoneyPuckGoalieGameError::NoEligibleAppearances { .. }) => {
                    warnings.push(format!(
                        "game {} {} starter {} has no prior MoneyPuck appearance",
                        game.game_id, evidence.team, player_id
                    ));
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
    warnings.sort();
    warnings.dedup();
    let package = GamePredictionEdgeEvidencePackage::build(
        forecast.season,
        TeamGameForecastVintage::PregameConfirmed,
        created_at,
        forecast_fingerprint,
        sources,
        games,
    )?;
    Ok(HistoricalGoalieEdgeBuildResult {
        package,
        games: forecast.games.len(),
        requested_goalies: required_ids.len(),
        form_sides,
        workload_sides,
        warnings,
    })
}

fn apply_form(
    evidence: &mut TeamGamePredictionTeamEvidence,
    form: &crate::MoneyPuckTrailingGoalieForm,
) {
    evidence.goalie_form_quality = Some(form.form_quality);
    evidence.goalie_form_appearances = form.appearances;
    evidence.goalie_form_state = TeamGameEvidenceState::Confirmed;
    evidence.goalie_workload_readiness = Some(form.workload_readiness);
    evidence
        .source_fingerprints
        .push(form.source_fingerprint.clone());
    evidence.source_fingerprints.sort();
    evidence.source_fingerprints.dedup();
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, HistoricalGoalieEdgeError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| HistoricalGoalieEdgeError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};
    use icelines_core::{
        build_team_game_forecast, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput, TeamGamePredictionEvidenceInput,
    };

    use super::*;

    fn team(name: &str, goalie_id: u32, source: &str) -> TeamGamePredictionTeamEvidence {
        TeamGamePredictionTeamEvidence {
            team: name.to_owned(),
            roster_strength: None,
            roster_state: TeamGameEvidenceState::Unavailable,
            availability_strength: None,
            availability_state: TeamGameEvidenceState::Unavailable,
            lineup_impact: None,
            lineup_impact_state: TeamGameEvidenceState::Unavailable,
            goalie_quality: Some(50.0),
            goalie_state: TeamGameEvidenceState::Confirmed,
            goalie_player_id: Some(goalie_id),
            goalie_form_quality: None,
            goalie_form_appearances: 0,
            goalie_form_state: TeamGameEvidenceState::Unavailable,
            goalie_workload_readiness: None,
            xg_share: None,
            xg_games: 0,
            opponent_adjusted_xg_share: None,
            opponent_adjusted_xg_games: 0,
            special_teams_strength: None,
            special_teams_games: 0,
            matchup_suitability: None,
            matchup_state: TeamGameEvidenceState::Unavailable,
            source_fingerprints: vec![source.to_owned()],
        }
    }

    #[test]
    fn enrichment_uses_only_starter_rows_before_game_date() {
        let forecast = build_team_game_forecast(
            20_252_026,
            vec![TeamForecastGameInput {
                game_id: 10,
                date: NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
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
                    strength: 50.0,
                },
                TeamForecastStrengthInput {
                    team: "NYR".to_owned(),
                    strength: 50.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap();
        let forecast_fingerprint = fingerprint(&forecast).unwrap();
        let source_fingerprint = format!("sha256:{}", "a".repeat(64));
        let at = Utc.with_ymd_and_hms(2025, 10, 10, 23, 0, 0).unwrap();
        let confirmed = GamePredictionEdgeEvidencePackage::build(
            forecast.season,
            TeamGameForecastVintage::PregameConfirmed,
            at,
            forecast_fingerprint,
            vec![GamePredictionEvidenceSource {
                source_key: "official.game.10".to_owned(),
                evidence_cutoff_at: at,
                retrieved_at: at,
                authority: GamePredictionEvidenceSourceAuthority::HistoricalReconstruction,
                source_uri: "https://api-web.nhle.com/gamecenter/10/boxscore".to_owned(),
                fingerprint: source_fingerprint.clone(),
            }],
            vec![TeamGamePredictionEvidenceInput {
                game_id: 10,
                forecast_at: at,
                captured_at: at,
                away: team("SEA", 2, &source_fingerprint),
                home: team("NYR", 1, &source_fingerprint),
            }],
        )
        .unwrap();
        let csv = |id| {
            format!(
            "playerId,season,name,gameId,playerTeam,opposingTeam,home_or_away,gameDate,position,situation,icetime,xGoals,goals\n\
             {id},2025,G,1,NYR,MTL,HOME,20251008,G,all,3600,3,2\n\
             {id},2025,G,2,NYR,BOS,HOME,20251010,G,all,3600,0,9\n"
        )
        };
        let result = build_historical_goalie_edge_package(
            &forecast,
            &confirmed,
            vec![
                HistoricalMoneyPuckGoalieInput {
                    player_id: 1,
                    csv_text: csv(1),
                    source_uri: "https://moneypuck.com/goalies/1.csv".to_owned(),
                    retrieved_at: at,
                },
                HistoricalMoneyPuckGoalieInput {
                    player_id: 2,
                    csv_text: csv(2),
                    source_uri: "https://moneypuck.com/goalies/2.csv".to_owned(),
                    retrieved_at: at,
                },
            ],
            5,
            at,
        )
        .unwrap();
        assert_eq!(result.form_sides, 2);
        assert_eq!(result.package.games[0].home.goalie_form_quality, Some(75.0));
        assert_eq!(result.package.games[0].home.goalie_form_appearances, 1);
        result.package.validate().unwrap();
    }
}
