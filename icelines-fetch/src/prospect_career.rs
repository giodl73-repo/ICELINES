//! Multi-league career-history facts to UI-neutral prospect studies.
//!
//! The official NHL player landing feed contains regular-season totals from
//! major junior, NCAA conferences, and European professional leagues. This
//! adapter joins those cached facts to separately authored prospect context.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::career_history::{CareerGameType, CareerHistory, LeagueTier};
use icelines_core::{
    build_prospect_development_study, build_prospect_discovery_board,
    build_prospect_goalie_development_study, ProspectDevelopmentSeasonInput,
    ProspectDevelopmentStudyConfig, ProspectDevelopmentStudyInput, ProspectDevelopmentStudyView,
    ProspectDiscoveryBoardView, ProspectGoalieDevelopmentSeasonInput,
    ProspectGoalieDevelopmentStudyConfig, ProspectGoalieDevelopmentStudyInput,
    ProspectGoalieDevelopmentStudyView, ProspectStudyEvidenceInput,
};
use serde::{Deserialize, Serialize};

use crate::career_landing::CareerHistoryStore;
use crate::prospect_discovery::{ProspectLeagueContext, PROSPECT_LEAGUE_CONTEXT_SCHEMA};

pub const PROSPECT_CAREER_DISCOVERY_SCHEMA: &str = "prospect_career_discovery.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectCareerExclusionReason {
    MissingCareerHistory,
    FewerThanTwoEligibleSeasons,
    MissingGoalieRateStats,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCareerExclusionView {
    pub player_id: u32,
    pub player: String,
    pub reason: ProspectCareerExclusionReason,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectCareerDiscoveryView {
    pub schema: String,
    pub context_players: usize,
    pub studies: Vec<ProspectDevelopmentStudyView>,
    pub goalie_studies: Vec<ProspectGoalieDevelopmentStudyView>,
    pub excluded: Vec<ProspectCareerExclusionView>,
    pub board: ProspectDiscoveryBoardView,
    pub disclosures: Vec<String>,
}

/// Adapt recognized non-NHL/non-AHL regular-season career rows. Comparisons
/// remain same-league inside the canonical study builders; this adapter does
/// not apply league-equivalency multipliers.
pub fn build_prospect_career_discovery(
    context: ProspectLeagueContext,
    store: &CareerHistoryStore,
    skater_config: ProspectDevelopmentStudyConfig,
    goalie_config: ProspectGoalieDevelopmentStudyConfig,
) -> Result<ProspectCareerDiscoveryView, String> {
    if context.schema != PROSPECT_LEAGUE_CONTEXT_SCHEMA || context.players.is_empty() {
        return Err("invalid or empty prospect career context".to_owned());
    }
    let context_players = context.players.len();
    let mut ids = BTreeSet::new();
    if context.players.iter().any(|row| {
        row.player_id == 0
            || row.player.trim().is_empty()
            || row.organization.trim().is_empty()
            || row.position.trim().is_empty()
            || !ids.insert(row.player_id)
    }) {
        return Err("invalid or duplicate prospect career context player".to_owned());
    }

    let mut studies = Vec::new();
    let mut goalie_studies = Vec::new();
    let mut excluded = Vec::new();
    for player in context.players {
        let Some(history) = store.get(player.player_id) else {
            excluded.push(exclusion(
                player.player_id,
                player.player,
                ProspectCareerExclusionReason::MissingCareerHistory,
                "No cached official NHL landing career history was supplied",
            ));
            continue;
        };
        let mut evidence = player.evidence;
        evidence.push(ProspectStudyEvidenceInput {
            label: "Official NHL player landing career totals".to_owned(),
            source_url: format!(
                "https://api-web.nhle.com/v1/player/{}/landing",
                player.player_id
            ),
        });
        if player.position.eq_ignore_ascii_case("G")
            || player.position.eq_ignore_ascii_case("Goalie")
        {
            let seasons = goalie_seasons(history);
            if seasons.len() < 2 {
                excluded.push(exclusion(
                    player.player_id,
                    player.player,
                    ProspectCareerExclusionReason::MissingGoalieRateStats,
                    "Fewer than two eligible seasons supplied both save percentage and goals-against average",
                ));
                continue;
            }
            goalie_studies.push(build_prospect_goalie_development_study(
                ProspectGoalieDevelopmentStudyInput {
                    player_id: player.player_id,
                    player: player.player,
                    organization: player.organization,
                    age: player.age,
                    nhl_games_played: player.nhl_games_played,
                    seasons,
                    opportunity: player.opportunity,
                    availability: player.availability,
                    evidence,
                },
                goalie_config,
            )?);
        } else {
            let seasons = skater_seasons(history);
            if seasons.len() < 2 {
                excluded.push(exclusion(
                    player.player_id,
                    player.player,
                    ProspectCareerExclusionReason::FewerThanTwoEligibleSeasons,
                    "Fewer than two recognized CHL, NCAA, junior, or European-pro regular seasons had skater totals",
                ));
                continue;
            }
            studies.push(build_prospect_development_study(
                ProspectDevelopmentStudyInput {
                    player_id: player.player_id,
                    player: player.player,
                    organization: player.organization,
                    position: player.position,
                    age: player.age,
                    nhl_games_played: player.nhl_games_played,
                    seasons,
                    opportunity: player.opportunity,
                    availability: player.availability,
                    attention_score: player.attention_score,
                    attention_basis: player.attention_basis,
                    evidence,
                },
                skater_config,
            )?);
        }
    }
    studies.sort_by_key(|row| row.player_id);
    goalie_studies.sort_by_key(|row| row.player_id);
    excluded.sort_by_key(|row| row.player_id);
    let board = build_prospect_discovery_board(studies.clone())?;
    Ok(ProspectCareerDiscoveryView {
        schema: PROSPECT_CAREER_DISCOVERY_SCHEMA.to_owned(),
        context_players,
        studies,
        goalie_studies,
        excluded,
        board,
        disclosures: vec![
            "Facts come from cached official NHL player landing career totals; prospect opportunity, availability, attention, organization, and position remain separately authored context.".to_owned(),
            "Eligible rows are recognized junior, college, and non-North-American professional leagues. NHL, AHL, ECHL, international tournaments, playoffs, and unclassified leagues are excluded.".to_owned(),
            "Trajectory comparisons are same-league only. No league-equivalency multiplier is applied across CHL, NCAA, or European leagues.".to_owned(),
            "Multiple teams in the same season and league are aggregated. Goalie rates are games-played weighted because the landing feed does not provide enough shared fields to reconstruct every historical rate exactly.".to_owned(),
        ],
    })
}

fn eligible_league(history: &CareerHistory, index: usize) -> bool {
    let stint = &history.stints[index];
    if stint.game_type != CareerGameType::Regular {
        return false;
    }
    match stint.league.tier() {
        LeagueTier::Junior | LeagueTier::College => true,
        LeagueTier::Pro => !matches!(
            stint.league.as_str().to_ascii_uppercase().as_str(),
            "NHL" | "AHL" | "ECHL"
        ),
        LeagueTier::International | LeagueTier::Other => false,
    }
}

fn skater_seasons(history: &CareerHistory) -> Vec<ProspectDevelopmentSeasonInput> {
    let mut rows: BTreeMap<(u32, String), (u32, u32, u32)> = BTreeMap::new();
    for (index, stint) in history.stints.iter().enumerate() {
        if !eligible_league(history, index) {
            continue;
        }
        let (Some(goals), Some(assists)) = (stint.goals, stint.assists) else {
            continue;
        };
        let row = rows
            .entry((stint.season.0, stint.league.as_str().to_owned()))
            .or_default();
        row.0 = row.0.saturating_add(stint.gp);
        row.1 = row.1.saturating_add(goals);
        row.2 = row.2.saturating_add(assists);
    }
    rows.into_iter()
        .filter(|(_, (gp, _, _))| *gp > 0)
        .map(
            |((season, league), (games_played, goals, assists))| ProspectDevelopmentSeasonInput {
                season,
                league,
                games_played,
                goals,
                assists,
            },
        )
        .collect()
}

fn goalie_seasons(history: &CareerHistory) -> Vec<ProspectGoalieDevelopmentSeasonInput> {
    let mut rows: BTreeMap<(u32, String), (u32, f64, f64)> = BTreeMap::new();
    for (index, stint) in history.stints.iter().enumerate() {
        if !eligible_league(history, index) || stint.gp == 0 {
            continue;
        }
        let (Some(save), Some(gaa)) = (stint.save_pct, stint.goals_against_avg) else {
            continue;
        };
        let row = rows
            .entry((stint.season.0, stint.league.as_str().to_owned()))
            .or_default();
        row.0 = row.0.saturating_add(stint.gp);
        row.1 += f64::from(save) * f64::from(stint.gp);
        row.2 += f64::from(gaa) * f64::from(stint.gp);
    }
    rows.into_iter()
        .filter(|(_, (gp, _, _))| *gp > 0)
        .map(|((season, league), (games_played, save_sum, gaa_sum))| {
            ProspectGoalieDevelopmentSeasonInput {
                season,
                league,
                games_played,
                save_percentage: save_sum / f64::from(games_played),
                goals_against_average: gaa_sum / f64::from(games_played),
            }
        })
        .collect()
}

fn exclusion(
    player_id: u32,
    player: String,
    reason: ProspectCareerExclusionReason,
    detail: &str,
) -> ProspectCareerExclusionView {
    ProspectCareerExclusionView {
        player_id,
        player,
        reason,
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prospect_discovery::{ProspectLeagueContextAuthority, ProspectLeaguePlayerContext};
    use icelines_core::career_history::{CareerStint, LeagueAbbrev};
    use icelines_core::model::Season;
    use icelines_core::{
        ProspectAvailabilityStatus, ProspectOpportunityStatus, ProspectTrajectory,
    };

    fn stint(season: u32, league: &str, gp: u32, goals: u32, assists: u32) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: "Club".to_owned(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp,
            goals: Some(goals),
            assists: Some(assists),
            points: Some(goals + assists),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    fn context(players: Vec<ProspectLeaguePlayerContext>) -> ProspectLeagueContext {
        ProspectLeagueContext {
            schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
            authority: ProspectLeagueContextAuthority::Authored,
            as_of_date: Some("2026-07-27".to_owned()),
            snapshot_seasons: vec![],
            players,
            exclusions: vec![],
            disclosures: vec![],
        }
    }

    fn player(id: u32, name: &str) -> ProspectLeaguePlayerContext {
        ProspectLeaguePlayerContext {
            player_id: id,
            player: name.to_owned(),
            organization: "SEA".to_owned(),
            position: "C".to_owned(),
            age: 20,
            nhl_games_played: 0,
            opportunity: ProspectOpportunityStatus::Monitoring,
            availability: ProspectAvailabilityStatus::Healthy,
            attention_score: 0.3,
            attention_basis: "Authored test context".to_owned(),
            evidence: vec![],
        }
    }

    #[test]
    fn adapts_same_league_chl_growth_and_excludes_nhl_rows() {
        let mut store = CareerHistoryStore::new();
        store.upsert(CareerHistory {
            player_id: 7,
            stints: vec![
                stint(20232024, "WHL", 50, 10, 20),
                stint(20242025, "WHL", 50, 20, 30),
                stint(20252026, "NHL", 5, 1, 1),
            ],
        });
        let view = build_prospect_career_discovery(
            context(vec![player(7, "Prospect")]),
            &store,
            ProspectDevelopmentStudyConfig::default(),
            ProspectGoalieDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert_eq!(view.studies.len(), 1);
        assert_eq!(view.studies[0].trajectory, ProspectTrajectory::Rising);
        assert_eq!(view.studies[0].seasons.len(), 2);
        assert_eq!(
            view.studies[0].evidence[0].source_url,
            "https://api-web.nhle.com/v1/player/7/landing"
        );
    }

    #[test]
    fn league_change_does_not_manufacture_trajectory() {
        let mut store = CareerHistoryStore::new();
        store.upsert(CareerHistory {
            player_id: 8,
            stints: vec![
                stint(20232024, "OHL", 60, 20, 30),
                stint(20242025, "NCAA", 30, 20, 25),
            ],
        });
        let view = build_prospect_career_discovery(
            context(vec![player(8, "Mover")]),
            &store,
            ProspectDevelopmentStudyConfig::default(),
            ProspectGoalieDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert_eq!(view.studies[0].trajectory, ProspectTrajectory::Insufficient);
        assert!(view.studies[0].seasons[1].same_league_ppg_delta.is_none());
    }

    #[test]
    fn reports_missing_cache_rows_without_silent_drop() {
        let view = build_prospect_career_discovery(
            context(vec![player(9, "Missing")]),
            &CareerHistoryStore::new(),
            ProspectDevelopmentStudyConfig::default(),
            ProspectGoalieDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert!(view.studies.is_empty());
        assert_eq!(view.board.studies, 0);
        assert_eq!(
            view.excluded[0].reason,
            ProspectCareerExclusionReason::MissingCareerHistory
        );
    }

    #[test]
    fn reports_official_college_goalie_history_when_save_rate_is_absent() {
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/landing/hellebuyck_8476945.json"
        ))
        .unwrap();
        let history = crate::career_landing::parse_career_history(8_476_945, &raw).unwrap();
        let mut store = CareerHistoryStore::new();
        store.upsert(history);
        let mut goalie = player(8_476_945, "Connor Hellebuyck");
        goalie.position = "G".to_owned();
        let view = build_prospect_career_discovery(
            context(vec![goalie]),
            &store,
            ProspectDevelopmentStudyConfig::default(),
            ProspectGoalieDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert!(view.studies.is_empty());
        assert!(view.goalie_studies.is_empty());
        assert_eq!(
            view.excluded[0].reason,
            ProspectCareerExclusionReason::MissingGoalieRateStats
        );
    }

    #[test]
    fn adapts_goalie_history_when_both_native_rates_exist() {
        let mut first = stint(20232024, "SHL", 20, 0, 0);
        first.goals = None;
        first.assists = None;
        first.save_pct = Some(0.905);
        first.goals_against_avg = Some(2.7);
        let mut second = stint(20242025, "SHL", 30, 0, 0);
        second.goals = None;
        second.assists = None;
        second.save_pct = Some(0.920);
        second.goals_against_avg = Some(2.2);
        let mut store = CareerHistoryStore::new();
        store.upsert(CareerHistory {
            player_id: 10,
            stints: vec![first, second],
        });
        let mut goalie = player(10, "Goalie Prospect");
        goalie.position = "G".to_owned();
        let view = build_prospect_career_discovery(
            context(vec![goalie]),
            &store,
            ProspectDevelopmentStudyConfig::default(),
            ProspectGoalieDevelopmentStudyConfig::default(),
        )
        .unwrap();
        assert_eq!(view.goalie_studies.len(), 1);
        assert_eq!(
            view.goalie_studies[0].trajectory,
            ProspectTrajectory::Rising
        );
    }
}
