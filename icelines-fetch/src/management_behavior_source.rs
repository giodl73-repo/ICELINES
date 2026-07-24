//! Three-season league evidence adapter for management-behavior rankings.
//!
//! This module translates public NHL season tables into the count-fact contract
//! owned by `icelines-core`.  It intentionally leaves traits requiring
//! transactions, opening-night snapshots, or shift-aligned deployment as
//! `NoRead` until those sources are available.

use std::collections::{BTreeMap, HashMap};

use chrono::{NaiveDate, Utc};
use icelines_core::{
    build_team_behavior_season_observation, calibrate_team_decision_profile,
    rank_team_decision_profiles, EvidenceLabel, RelativeBehaviorCountFact,
    TeamBehaviorCalibrationInput, TeamBehaviorCalibrationView, TeamBehaviorRankingView,
    TeamBehaviorSeasonFactsInput,
};
use serde::{Deserialize, Serialize};

use crate::{
    error::FetchError,
    nhl_api::NhlApiClient,
    schema::{SkaterBio, SkaterRealtime, SkaterStats},
    teams::nhl_teams_for_season,
};

pub const TEAM_BEHAVIOR_LEAGUE_EVIDENCE_SCHEMA: &str = "team_behavior_league_evidence.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorEvidenceSourceView {
    pub source_id: String,
    pub url: String,
    pub seasons: Vec<u32>,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorSeasonEvidenceView {
    pub team: String,
    pub season: u32,
    pub skaters: u32,
    pub player_games: u32,
    pub facts: TeamBehaviorSeasonFactsInput,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamBehaviorLeagueEvidenceView {
    pub schema: String,
    pub target_season: u32,
    pub window_seasons: u8,
    pub generated_at: String,
    pub sources: Vec<BehaviorEvidenceSourceView>,
    pub season_evidence: Vec<TeamBehaviorSeasonEvidenceView>,
    pub calibrations: Vec<TeamBehaviorCalibrationView>,
    pub rankings: TeamBehaviorRankingView,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct TeamCounts {
    skaters: u32,
    player_games: u32,
    rookie_games: u32,
    veteran_games: u32,
    continuity_games: u32,
    bottom_forward_games: u32,
    physical_bottom_forward_games: u32,
    forward_games: u32,
    regular_rotation_forward_games: u32,
}

#[derive(Debug, Clone)]
struct PlayerRow {
    team: String,
    games: u32,
    position: String,
    toi_seconds_per_game: f64,
    birth_date: Option<NaiveDate>,
    first_season: Option<u32>,
    hits: u32,
}

/// Fetch the completed seasons immediately preceding `target_season` and build
/// an all-team, renderer-neutral ranking document.
pub async fn fetch_team_behavior_league_evidence(
    client: &NhlApiClient,
    target_season: u32,
    window_seasons: u8,
) -> Result<TeamBehaviorLeagueEvidenceView, FetchError> {
    if !(1..=3).contains(&window_seasons) || !valid_season(target_season) {
        return Err(FetchError::SchemaChanged {
            detail: "behavior evidence requires a valid target season and a 1-3 season window"
                .to_owned(),
        });
    }

    let seasons = (1..=u32::from(window_seasons))
        .map(|offset| target_season - 10_001 * offset)
        .collect::<Vec<_>>();
    let mut by_team: BTreeMap<String, Vec<TeamBehaviorSeasonEvidenceView>> = BTreeMap::new();

    for season in &seasons {
        let season_text = season.to_string();
        let (bios, stats, realtime) = tokio::try_join!(
            client.fetch_all_bios(
                &season_text,
                icelines_core::season_stats::SeasonType::Regular
            ),
            client.fetch_all_stats(
                &season_text,
                icelines_core::season_stats::SeasonType::Regular
            ),
            client.fetch_all_realtime(&season_text),
        )?;
        let rows = player_rows(*season, &bios, &stats, &realtime);
        let evidence = build_season_evidence(*season, &rows);
        for row in evidence {
            by_team.entry(row.team.clone()).or_default().push(row);
        }
    }

    let mut season_evidence = by_team
        .values()
        .flat_map(|rows| rows.iter().cloned())
        .collect::<Vec<_>>();
    season_evidence.sort_by(|a, b| a.season.cmp(&b.season).then_with(|| a.team.cmp(&b.team)));

    let mut calibrations = Vec::new();
    for team in nhl_teams_for_season(&target_season.to_string()) {
        let observations = by_team
            .get(team)
            .into_iter()
            .flatten()
            .map(|row| build_team_behavior_season_observation(&row.facts))
            .collect::<Result<Vec<_>, _>>()
            .map_err(core_error)?;
        let calibration = calibrate_team_decision_profile(&TeamBehaviorCalibrationInput {
            team: team.to_owned(),
            target_season,
            window_seasons,
            observations,
        })
        .map_err(core_error)?;
        calibrations.push(calibration);
    }
    calibrations.sort_by(|a, b| a.team.cmp(&b.team));
    let profiles = calibrations
        .iter()
        .map(|row| row.profile.clone())
        .collect::<Vec<_>>();
    let rankings = rank_team_decision_profiles(&profiles).map_err(core_error)?;

    Ok(TeamBehaviorLeagueEvidenceView {
        schema: TEAM_BEHAVIOR_LEAGUE_EVIDENCE_SCHEMA.to_owned(),
        target_season,
        window_seasons,
        generated_at: Utc::now().to_rfc3339(),
        sources: source_manifest(&seasons),
        season_evidence,
        calibrations,
        rankings,
        disclosures: vec![
            "Rookie opportunity is measured as first-NHL-season skater games divided by all skater games; it is season usage, not an opening-night roster claim.".to_owned(),
            "Veteran preference is measured as age-30-or-older skater games divided by all skater games, with age measured on October 1.".to_owned(),
            "Lineup patience uses the share of team skater-games supplied by the 18 most-used skaters.".to_owned(),
            "Physical fourth-line preference uses bottom-half forward games from players producing at least eight hits per 60 minutes.".to_owned(),
            "Four-line usage uses the share of forward games supplied by forwards averaging at least eight minutes.".to_owned(),
            "Players listing multiple teams are assigned to the final team in the NHL summary row; this avoids double-counting but does not split traded-player statistics by stint.".to_owned(),
            "Waiver protection, trades, deadline buying, matchups, tactical changes, off-position use, and fatigue rotation remain NoRead pending their authoritative adapters.".to_owned(),
        ],
    })
}

fn valid_season(season: u32) -> bool {
    let start = season / 10_000;
    season % 10_000 == start + 1
}

fn core_error(detail: String) -> FetchError {
    FetchError::SchemaChanged { detail }
}

fn source_manifest(seasons: &[u32]) -> Vec<BehaviorEvidenceSourceView> {
    [
        (
            "nhl-skater-summary",
            "skater/summary",
            vec![
                "gamesPlayed",
                "positionCode",
                "teamAbbrevs",
                "timeOnIcePerGame",
            ],
        ),
        (
            "nhl-skater-bios",
            "skater/bios",
            vec!["birthDate", "firstSeasonForGameType"],
        ),
        ("nhl-skater-realtime", "skater/realtime", vec!["hits"]),
    ]
    .into_iter()
    .map(|(source_id, path, fields)| BehaviorEvidenceSourceView {
        source_id: source_id.to_owned(),
        url: format!("https://api.nhle.com/stats/rest/en/{path}"),
        seasons: seasons.to_vec(),
        fields: fields.into_iter().map(str::to_owned).collect(),
    })
    .collect()
}

fn player_rows(
    season: u32,
    bios: &[SkaterBio],
    stats: &[SkaterStats],
    realtime: &[SkaterRealtime],
) -> Vec<PlayerRow> {
    let bios = bios
        .iter()
        .map(|row| (row.player_id, row))
        .collect::<HashMap<_, _>>();
    let realtime = realtime
        .iter()
        .map(|row| (row.player_id, row))
        .collect::<HashMap<_, _>>();
    stats
        .iter()
        .filter_map(|row| {
            let team = row.team_abbrevs.as_deref()?.split(',').next_back()?.trim();
            if !nhl_teams_for_season(&season.to_string()).contains(&team) {
                return None;
            }
            let bio = bios.get(&row.player_id);
            Some(PlayerRow {
                team: team.to_owned(),
                games: row.games_played,
                position: bio.map_or_else(|| "?".to_owned(), |b| b.position_code.clone()),
                toi_seconds_per_game: f64::from(row.time_on_ice_per_game.unwrap_or(0.0)),
                birth_date: bio
                    .and_then(|b| b.birth_date.as_deref())
                    .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
                first_season: bio.and_then(|b| b.first_season_for_game_type),
                hits: realtime.get(&row.player_id).map_or(0, |r| r.hits),
            })
        })
        .collect()
}

fn build_season_evidence(
    season: u32,
    players: &[PlayerRow],
) -> Vec<TeamBehaviorSeasonEvidenceView> {
    let teams = nhl_teams_for_season(&season.to_string());
    let mut counts = BTreeMap::new();
    for team in &teams {
        let mut team_players = players
            .iter()
            .filter(|p| p.team == *team)
            .collect::<Vec<_>>();
        team_players.sort_by_key(|p| std::cmp::Reverse(p.games));
        let mut row = TeamCounts::default();
        row.skaters = team_players.len() as u32;
        row.player_games = team_players.iter().map(|p| p.games).sum();
        row.continuity_games = team_players.iter().take(18).map(|p| p.games).sum();
        let cutoff =
            NaiveDate::from_ymd_opt((season / 10_000) as i32, 10, 1).expect("valid season date");
        for player in &team_players {
            if player.first_season == Some(season) {
                row.rookie_games += player.games;
            }
            if player
                .birth_date
                .is_some_and(|birth| cutoff.years_since(birth).unwrap_or(0) >= 30)
            {
                row.veteran_games += player.games;
            }
        }
        let mut forwards = team_players
            .iter()
            .copied()
            .filter(|p| p.position != "D" && p.position != "G")
            .collect::<Vec<_>>();
        forwards.sort_by(|a, b| b.toi_seconds_per_game.total_cmp(&a.toi_seconds_per_game));
        row.forward_games = forwards.iter().map(|p| p.games).sum();
        row.regular_rotation_forward_games = forwards
            .iter()
            .filter(|p| p.toi_seconds_per_game >= 480.0)
            .map(|p| p.games)
            .sum();
        for player in forwards.iter().skip(forwards.len() / 2) {
            row.bottom_forward_games += player.games;
            let total_hours = player.toi_seconds_per_game * f64::from(player.games) / 3600.0;
            let hits_per_60 = if total_hours > 0.0 {
                f64::from(player.hits) / total_hours
            } else {
                0.0
            };
            if hits_per_60 >= 8.0 {
                row.physical_bottom_forward_games += player.games;
            }
        }
        counts.insert((*team).to_owned(), row);
    }
    let league = counts
        .values()
        .fold(TeamCounts::default(), |mut total, row| {
            total.player_games += row.player_games;
            total.rookie_games += row.rookie_games;
            total.veteran_games += row.veteran_games;
            total.continuity_games += row.continuity_games;
            total.bottom_forward_games += row.bottom_forward_games;
            total.physical_bottom_forward_games += row.physical_bottom_forward_games;
            total.forward_games += row.forward_games;
            total.regular_rotation_forward_games += row.regular_rotation_forward_games;
            total
        });
    counts
        .into_iter()
        .map(|(team, row)| {
            let fact =
                |team_successes, team_opportunities, league_successes, league_opportunities| {
                    (team_opportunities > 0 && league_opportunities > 0).then_some(
                        RelativeBehaviorCountFact {
                            team_successes,
                            team_opportunities,
                            league_successes,
                            league_opportunities,
                            evidence_label: EvidenceLabel::Confirmed,
                        },
                    )
                };
            TeamBehaviorSeasonEvidenceView {
                team,
                season,
                skaters: row.skaters,
                player_games: row.player_games,
                facts: TeamBehaviorSeasonFactsInput {
                    season,
                    rookie_opening_roster_decisions: fact(
                        row.rookie_games,
                        row.player_games,
                        league.rookie_games,
                        league.player_games,
                    ),
                    veteran_retention_decisions: fact(
                        row.veteran_games,
                        row.player_games,
                        league.veteran_games,
                        league.player_games,
                    ),
                    lineup_continuity_decisions: fact(
                        row.continuity_games,
                        row.player_games,
                        league.continuity_games,
                        league.player_games,
                    ),
                    physical_fourth_line_deployments: fact(
                        row.physical_bottom_forward_games,
                        row.bottom_forward_games,
                        league.physical_bottom_forward_games,
                        league.bottom_forward_games,
                    ),
                    balanced_four_line_games: fact(
                        row.regular_rotation_forward_games,
                        row.forward_games,
                        league.regular_rotation_forward_games,
                        league.forward_games,
                    ),
                    ..TeamBehaviorSeasonFactsInput::default()
                },
                notes: vec![format!(
                    "{} skaters supplied {} player-games",
                    row.skaters, row.player_games
                )],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_seasons() {
        assert!(valid_season(20262027));
        assert!(!valid_season(20262028));
    }

    #[test]
    fn source_manifest_names_exact_public_endpoints() {
        let sources = source_manifest(&[20252026, 20242025, 20232024]);
        assert_eq!(sources.len(), 3);
        assert!(sources
            .iter()
            .all(|row| row.url.starts_with("https://api.nhle.com/")));
    }
}
