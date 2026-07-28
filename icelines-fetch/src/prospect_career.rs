//! Multi-league career-history facts to UI-neutral prospect studies.
//!
//! The official NHL player landing feed contains regular-season totals from
//! major junior, NCAA conferences, and European professional leagues. This
//! adapter joins those cached facts to separately authored prospect context.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use icelines_core::career_history::{CareerGameType, CareerHistory, LeagueTier};
use icelines_core::{
    build_prospect_development_study, build_prospect_discovery_board,
    build_prospect_goalie_development_study, ProspectAvailabilityStatus,
    ProspectDevelopmentSeasonInput, ProspectDevelopmentStudyConfig, ProspectDevelopmentStudyInput,
    ProspectDevelopmentStudyView, ProspectDiscoveryBoardView, ProspectGoalieDevelopmentSeasonInput,
    ProspectGoalieDevelopmentStudyConfig, ProspectGoalieDevelopmentStudyInput,
    ProspectGoalieDevelopmentStudyView, ProspectNhlGamesAuthority, ProspectOpportunityStatus,
    ProspectStudyEvidenceInput, TrainingCampLeagueForecastView, TrainingCampPlayerView,
    TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA,
};
use serde::{Deserialize, Serialize};

use crate::career_landing::CareerHistoryStore;
use crate::prospect_discovery::{
    ProspectLeagueContext, ProspectLeagueContextAuthority, ProspectLeagueContextExclusionReason,
    ProspectLeagueContextExclusionView, ProspectLeaguePlayerContext,
    PROSPECT_LEAGUE_CONTEXT_SCHEMA,
};

pub const PROSPECT_CAREER_DISCOVERY_SCHEMA: &str = "prospect_career_discovery.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCareerContextIdentityInput {
    pub player_id: u32,
    pub birth_date: String,
    #[serde(default)]
    pub nhl_games_played: u32,
    #[serde(default)]
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCareerContextDraftConfig {
    pub as_of_date: NaiveDate,
    pub max_age: u8,
}

impl Default for ProspectCareerContextDraftConfig {
    fn default() -> Self {
        Self {
            as_of_date: NaiveDate::from_ymd_opt(2026, 9, 15).expect("valid default date"),
            max_age: 24,
        }
    }
}

/// Create neutral prospect context from the league training-camp pool. The
/// camp's prospect flag selects the pool, while identity facts only establish
/// age and optional NHL workload. Forecast probability never becomes authored
/// opportunity, availability, or public-attention evidence.
pub fn build_prospect_career_context_draft(
    forecast: TrainingCampLeagueForecastView,
    identities: Vec<ProspectCareerContextIdentityInput>,
    config: ProspectCareerContextDraftConfig,
) -> Result<ProspectLeagueContext, String> {
    if forecast.schema != TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA
        || forecast.season == 0
        || forecast.teams.is_empty()
        || config.max_age == 0
    {
        return Err("invalid prospect career context draft inputs".to_owned());
    }
    let mut identity_by_id = BTreeMap::new();
    for identity in identities {
        if identity.player_id == 0
            || identity.birth_date.trim().is_empty()
            || identity.evidence.iter().any(|item| {
                item.label.trim().is_empty()
                    || !(item.source_url.starts_with("https://")
                        || item.source_url.starts_with("http://"))
            })
            || identity_by_id
                .insert(identity.player_id, identity)
                .is_some()
        {
            return Err("invalid or duplicate prospect career identity".to_owned());
        }
    }

    let mut candidates = BTreeMap::<u32, Vec<(String, TrainingCampPlayerView)>>::new();
    for team in forecast.teams {
        let Some(team_forecast) = team.forecast else {
            continue;
        };
        if !team_forecast.team.eq_ignore_ascii_case(&team.team)
            || team_forecast.season != forecast.season
        {
            return Err("team training-camp forecast does not match league envelope".to_owned());
        }
        for player in team_forecast
            .players
            .into_iter()
            .filter(|player| player.prospect)
        {
            candidates
                .entry(player.player_id)
                .or_default()
                .push((team.team.clone(), player));
        }
    }

    let mut players = Vec::new();
    let mut exclusions = Vec::new();
    for (player_id, mut appearances) in candidates {
        if appearances.len() != 1 {
            appearances.sort_by(|left, right| left.0.cmp(&right.0));
            exclusions.push(ProspectLeagueContextExclusionView {
                player_id,
                player: appearances
                    .first()
                    .map(|(_, player)| player.display_name.clone())
                    .unwrap_or_else(|| player_id.to_string()),
                reason: ProspectLeagueContextExclusionReason::AmbiguousOrganization,
                detail: format!(
                    "Prospect appears in multiple camp organizations: {}",
                    appearances
                        .iter()
                        .map(|(team, _)| team.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
            continue;
        }
        let (organization, player) = appearances.pop().expect("one validated appearance");
        let Some(identity) = identity_by_id.get(&player_id) else {
            exclusions.push(ProspectLeagueContextExclusionView {
                player_id,
                player: player.display_name,
                reason: ProspectLeagueContextExclusionReason::MissingBirthDate,
                detail: "No dated roster or candidate-overlay identity supplied a birth date"
                    .to_owned(),
            });
            continue;
        };
        let Ok(birth_date) = NaiveDate::parse_from_str(&identity.birth_date, "%Y-%m-%d") else {
            exclusions.push(ProspectLeagueContextExclusionView {
                player_id,
                player: player.display_name,
                reason: ProspectLeagueContextExclusionReason::InvalidBirthDate,
                detail: format!("Invalid identity birth date {}", identity.birth_date),
            });
            continue;
        };
        let age = age_on(birth_date, config.as_of_date);
        if age > u32::from(config.max_age) {
            exclusions.push(ProspectLeagueContextExclusionView {
                player_id,
                player: player.display_name,
                reason: ProspectLeagueContextExclusionReason::AboveMaximumAge,
                detail: format!("Age {age} exceeds maximum {}", config.max_age),
            });
            continue;
        }
        players.push(ProspectLeaguePlayerContext {
            player_id,
            player: player.display_name,
            organization,
            position: player.primary_position.abbreviation().to_owned(),
            age: age as u8,
            nhl_games_played: identity.nhl_games_played,
            opportunity: ProspectOpportunityStatus::None,
            availability: ProspectAvailabilityStatus::Unknown,
            attention_score: 0.5,
            attention_basis:
                "Neutral observed-draft placeholder; authored attention research not supplied"
                    .to_owned(),
            evidence: identity.evidence.clone(),
        });
    }
    players.sort_by_key(|player| player.player_id);
    exclusions.sort_by_key(|row| row.player_id);
    Ok(ProspectLeagueContext {
        schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
        authority: ProspectLeagueContextAuthority::ObservedDraft,
        as_of_date: Some(config.as_of_date.to_string()),
        snapshot_seasons: vec![forecast.season],
        players,
        exclusions,
        disclosures: vec![
            "The pool contains players marked as prospects by the supplied league training-camp artifact. Automatic camp pools use age-based prospect estimates; this is a coverage draft, not an authored scouting list.".to_owned(),
            "Birth date and optional NHL workload come from supplied identity facts. Camp make probability and projected score do not become opportunity, attention, or development evidence.".to_owned(),
            "Opportunity is none, availability is unknown, and attention is neutral 0.5 until separately sourced research promotes this draft to authored context.".to_owned(),
            "Players missing usable identity facts, above the age ceiling, or assigned to multiple organizations remain visible as typed exclusions.".to_owned(),
        ],
    })
}

fn age_on(birth_date: NaiveDate, as_of: NaiveDate) -> u32 {
    let mut age = as_of.year() - birth_date.year();
    if (as_of.month(), as_of.day()) < (birth_date.month(), birth_date.day()) {
        age -= 1;
    }
    age.max(0) as u32
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nhl_games_played: Option<u32>,
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
    if context.authority == ProspectLeagueContextAuthority::ObservedDraft
        && context.players.iter().any(|row| {
            row.opportunity != ProspectOpportunityStatus::None
                || row.availability != ProspectAvailabilityStatus::Unknown
                || (row.attention_score - 0.5).abs() > f64::EPSILON
        })
    {
        return Err(
            "observed prospect context draft contains non-neutral authored fields".to_owned(),
        );
    }
    if context.players.iter().any(|row| {
        row.player_id == 0
            || row.player.trim().is_empty()
            || row.organization.trim().is_empty()
            || row.position.trim().is_empty()
            || !row.attention_score.is_finite()
            || !(0.0..=1.0).contains(&row.attention_score)
            || row.attention_basis.trim().is_empty()
            || row.evidence.iter().any(|item| {
                item.label.trim().is_empty()
                    || !(item.source_url.starts_with("https://")
                        || item.source_url.starts_with("http://"))
            })
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
                None,
                ProspectCareerExclusionReason::MissingCareerHistory,
                "No cached official NHL landing career history was supplied",
            ));
            continue;
        };
        let nhl_games_played = history
            .stints
            .iter()
            .filter(|stint| {
                stint.game_type == CareerGameType::Regular
                    && stint.league.as_str().eq_ignore_ascii_case("NHL")
            })
            .fold(0_u32, |total, stint| total.saturating_add(stint.gp));
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
                    Some(nhl_games_played),
                    ProspectCareerExclusionReason::MissingGoalieRateStats,
                    "Fewer than two eligible seasons supplied both save percentage and goals-against average",
                ));
                continue;
            }
            let mut study = build_prospect_goalie_development_study(
                ProspectGoalieDevelopmentStudyInput {
                    player_id: player.player_id,
                    player: player.player,
                    organization: player.organization,
                    age: player.age,
                    nhl_games_played,
                    seasons,
                    opportunity: player.opportunity,
                    availability: player.availability,
                    evidence,
                },
                goalie_config,
            )?;
            study.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            goalie_studies.push(study);
        } else {
            let seasons = skater_seasons(history);
            if seasons.len() < 2 {
                excluded.push(exclusion(
                    player.player_id,
                    player.player,
                    Some(nhl_games_played),
                    ProspectCareerExclusionReason::FewerThanTwoEligibleSeasons,
                    "Fewer than two recognized CHL, NCAA, junior, or European-pro regular seasons had skater totals",
                ));
                continue;
            }
            let mut study = build_prospect_development_study(
                ProspectDevelopmentStudyInput {
                    player_id: player.player_id,
                    player: player.player,
                    organization: player.organization,
                    position: player.position,
                    age: player.age,
                    nhl_games_played,
                    seasons,
                    opportunity: player.opportunity,
                    availability: player.availability,
                    attention_score: player.attention_score,
                    attention_basis: player.attention_basis,
                    evidence,
                },
                skater_config,
            )?;
            study.nhl_games_authority = ProspectNhlGamesAuthority::Observed;
            studies.push(study);
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
            "Multiple teams in the same season and league are aggregated. If multiple eligible leagues appear in one season, the highest-workload league is retained so unlike competition is not blended into one rate.".to_owned(),
            "Goalie rates are games-played weighted because the landing feed does not provide enough shared fields to reconstruct every historical rate exactly.".to_owned(),
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
    let mut seasons = BTreeMap::<u32, ProspectDevelopmentSeasonInput>::new();
    for ((season, league), (games_played, goals, assists)) in
        rows.into_iter().filter(|(_, (gp, _, _))| *gp > 0)
    {
        let candidate = ProspectDevelopmentSeasonInput {
            season,
            league,
            games_played,
            goals,
            assists,
        };
        let replace = seasons.get(&season).is_none_or(|current| {
            candidate.games_played > current.games_played
                || (candidate.games_played == current.games_played
                    && candidate.league < current.league)
        });
        if replace {
            seasons.insert(season, candidate);
        }
    }
    seasons.into_values().collect()
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
    let mut seasons = BTreeMap::<u32, ProspectGoalieDevelopmentSeasonInput>::new();
    for ((season, league), (games_played, save_sum, gaa_sum)) in
        rows.into_iter().filter(|(_, (gp, _, _))| *gp > 0)
    {
        let candidate = ProspectGoalieDevelopmentSeasonInput {
            season,
            league,
            games_played,
            save_percentage: save_sum / f64::from(games_played),
            goals_against_average: gaa_sum / f64::from(games_played),
        };
        let replace = seasons.get(&season).is_none_or(|current| {
            candidate.games_played > current.games_played
                || (candidate.games_played == current.games_played
                    && candidate.league < current.league)
        });
        if replace {
            seasons.insert(season, candidate);
        }
    }
    seasons.into_values().collect()
}

fn exclusion(
    player_id: u32,
    player: String,
    nhl_games_played: Option<u32>,
    reason: ProspectCareerExclusionReason,
    detail: &str,
) -> ProspectCareerExclusionView {
    ProspectCareerExclusionView {
        player_id,
        player,
        nhl_games_played,
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

    fn camp_forecast() -> TrainingCampLeagueForecastView {
        serde_json::from_str(
            r#"{
            "schema": "training_camp_league_forecast.v1",
            "season": 20262027,
            "teams_requested": 1,
            "teams_simulated": 1,
            "teams_degraded": 0,
            "teams_augmented": 0,
            "teams_failed": 0,
            "teams": [{
                "team": "SEA",
                "authority_status": "confirmed_pool",
                "competition_pool_status": "authored",
                "current_roster_candidates": 1,
                "sourced_overlay_candidates": 0,
                "fallback_candidates": 0,
                "forecast": {
                    "schema": "training_camp_forecast.v1",
                    "method": "seeded_constrained_camp.v2",
                    "team": "SEA",
                    "season": 20262027,
                    "trials": 1,
                    "seed": 1,
                    "decision_profile_id": null,
                    "valid_trials": 1,
                    "incomplete_trials": 0,
                    "roster_shape": "test",
                    "opening_roster_size": 1,
                    "dressed_roster_size": 1,
                    "salary_cap_upper_limit": null,
                    "salary_cap_status": "no_read",
                    "players": [{
                        "player_id": 11,
                        "display_name": "Camp Prospect",
                        "primary_position": "Center",
                        "source_league": "WHL",
                        "incumbent": false,
                        "rookie_eligible": true,
                        "prospect": true,
                        "pre_camp_make_probability": null,
                        "pre_camp_track": "bubble",
                        "roster_prior_delta": 0.0,
                        "minimum_forward_role": null,
                        "waiver_exempt": true,
                        "cap_hit": null,
                        "cap_hit_source": null,
                        "projected_score": 50.0,
                        "gp_confidence": 0.5,
                        "camp_mean": 50.0,
                        "management_behavior_delta": 0.0,
                        "average_sampled_camp_score": 50.0,
                        "make_probability": 0.5,
                        "cut_probability": 0.5,
                        "unavailable_probability": 0.0,
                        "selection_loss_probability": 0.5,
                        "dressed_probability": 0.4,
                        "healthy_scratch_probability": 0.1,
                        "waiver_exposure_probability": 0.0,
                        "status": "bubble",
                        "displaced_incumbents": [],
                        "evidence_label": "estimated"
                    }],
                    "most_common_rosters": [],
                    "modal_opening_roster_ids": [11],
                    "warnings": [],
                    "disclosures": []
                },
                "error": null,
                "authority_warnings": []
            }],
            "disclosures": []
        }"#,
        )
        .expect("minimal league camp forecast")
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
        assert_eq!(view.studies[0].nhl_games_played, 5);
        assert_eq!(
            view.studies[0].nhl_games_authority,
            ProspectNhlGamesAuthority::Observed
        );
        assert_eq!(view.studies[0].trajectory, ProspectTrajectory::Rising);
        assert_eq!(view.studies[0].seasons.len(), 2);
        assert_eq!(
            view.studies[0].evidence[0].source_url,
            "https://api-web.nhle.com/v1/player/7/landing"
        );
    }

    #[test]
    fn drafts_neutral_context_from_camp_prospect_identity() {
        let view = build_prospect_career_context_draft(
            camp_forecast(),
            vec![ProspectCareerContextIdentityInput {
                player_id: 11,
                birth_date: "2005-01-02".to_owned(),
                nhl_games_played: 7,
                evidence: vec![],
            }],
            ProspectCareerContextDraftConfig::default(),
        )
        .unwrap();
        assert_eq!(
            view.authority,
            ProspectLeagueContextAuthority::ObservedDraft
        );
        assert_eq!(view.players.len(), 1);
        assert_eq!(view.players[0].organization, "SEA");
        assert_eq!(view.players[0].nhl_games_played, 7);
        assert_eq!(view.players[0].opportunity, ProspectOpportunityStatus::None);
        assert_eq!(
            view.players[0].availability,
            ProspectAvailabilityStatus::Unknown
        );
        assert_eq!(view.players[0].attention_score, 0.5);
    }

    #[test]
    fn same_season_uses_highest_workload_league() {
        let history = CareerHistory {
            player_id: 12,
            stints: vec![
                stint(20232024, "WHL", 10, 5, 5),
                stint(20232024, "OHL", 40, 10, 20),
                stint(20242025, "OHL", 50, 20, 30),
            ],
        };
        let seasons = skater_seasons(&history);
        assert_eq!(seasons.len(), 2);
        assert_eq!(seasons[0].league, "OHL");
        assert_eq!(seasons[0].games_played, 40);
    }

    #[test]
    fn observed_context_rejects_authored_signal_leakage() {
        let mut draft = context(vec![player(13, "Leaked")]);
        draft.authority = ProspectLeagueContextAuthority::ObservedDraft;
        let error = build_prospect_career_discovery(
            draft,
            &CareerHistoryStore::new(),
            ProspectDevelopmentStudyConfig::default(),
            ProspectGoalieDevelopmentStudyConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("non-neutral"));
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
        assert_eq!(view.excluded[0].nhl_games_played, None);
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
        assert!(view.excluded[0].nhl_games_played.is_some());
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
