use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::isolated_impact::IsolatedImpactView;
use super::management_behavior::{
    build_bench_game_plan, BenchGamePlanInput, BenchGamePlanView, BenchScheduleLoad,
    OpponentTacticalStyle, PlayerMatchupRoleInput, TeamDecisionProfile, BENCH_GAME_PLAN_SCHEMA,
};
use super::matchup_evidence::{
    OpponentStyleEvidenceRow, TeamPlayerMatchupRoleEvidenceView, OPPONENT_STYLE_EVIDENCE_SCHEMA,
    TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA,
};
use super::scenario_registry::{scenario_content_sha256, ScenarioRegistryReferenceView};
use super::team_game_forecast::{
    TeamForecastParameters, TeamGameForecastAccuracySummary, TeamGameForecastRow,
    TeamGameForecastSummaryRow, TeamGameForecastView, TeamGameMembershipAnomalyRow,
    TeamGameMembershipIntervalRow, TeamGameOpeningRosterAuthorityRow, TeamGameOpeningStrengthRow,
    TeamGamePairedTradeRow, TeamGamePersonnelEvidenceRow, TEAM_GAME_FORECAST_SCHEMA,
};
use super::team_lineup::TeamLineupProjectionView;

pub const TEAM_SEASON_FORECAST_SCHEMA: &str = "team_season_forecast.v1";
pub const TEAM_SEASON_FORECAST_MOVEMENT_SCHEMA: &str = "team_season_forecast_movement.v1";
pub const TEAM_SEASON_FORECAST_HISTORY_SCHEMA: &str = "team_season_forecast_history.v1";
pub const TEAM_SEASON_SCENARIO_SCHEMA: &str = "team_season_scenario.v1";
pub const TEAM_SEASON_GAME_PLAN_SCHEDULE_SCHEMA: &str = "team_season_game_plan_schedule.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSeasonScenarioEventKind {
    Injury,
    Goalie,
    Trade,
    Return,
    Form,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenarioEvent {
    pub id: String,
    pub kind: TeamSeasonScenarioEventKind,
    pub team: String,
    #[serde(default)]
    pub player: Option<String>,
    pub effective_date: NaiveDate,
    #[serde(default)]
    pub end_date: Option<NaiveDate>,
    /// Signed 0-100 team-strength change while the event is active.
    pub strength_delta: f64,
    #[serde(default = "certain_probability")]
    pub occurrence_probability: f64,
    /// Events sharing a key use the same occurrence draw (for atomic paired effects).
    #[serde(default)]
    pub correlation_key: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenario {
    pub name: String,
    #[serde(default)]
    pub trade_deadline: Option<NaiveDate>,
    #[serde(default)]
    pub events: Vec<TeamSeasonScenarioEvent>,
    /// Result-aware lineup policies evaluated independently inside each trial.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adaptive_lineup_policies: Vec<TeamSeasonAdaptiveLineupPolicy>,
    /// Mutually exclusive opening rosters sampled once per season trial.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opening_roster_policies: Vec<TeamSeasonOpeningRosterPolicy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonOpponentGamePlanInput {
    pub opponent: String,
    pub opponent_style: OpponentTacticalStyle,
    #[serde(default)]
    pub opponent_primary_threat: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScheduledGamePlanRow {
    pub game_id: u64,
    pub date: NaiveDate,
    pub opponent: String,
    pub is_home: bool,
    pub plan: BenchGamePlanView,
    pub event: TeamSeasonScenarioEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonGamePlanScheduleView {
    pub schema: String,
    pub team: String,
    pub season: u32,
    pub games: Vec<TeamSeasonScheduledGamePlanRow>,
    /// Simulation-ready scenario containing one exact-date event per plan.
    pub scenario: TeamSeasonScenario,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonOpeningRosterChoice {
    pub id: String,
    pub label: String,
    pub probability: f64,
    /// Signed 0-100 team-strength change applied for the entire trial.
    pub strength_delta: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roster_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonOpeningRosterPolicy {
    pub team: String,
    pub choices: Vec<TeamSeasonOpeningRosterChoice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonAdaptiveLineupChoice {
    pub id: String,
    pub label: String,
    /// Signed 0-100 team-strength change while this lineup is selected.
    pub strength_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonAdaptiveLineupPolicy {
    pub team: String,
    /// Number of this team's games evaluated before a keep/change decision.
    pub review_games: u8,
    /// Keep the current choice when window standings-points percentage meets this value.
    pub minimum_points_percentage: f64,
    pub max_changes: u8,
    /// Ordered choices, beginning with the opening-night lineup.
    pub choices: Vec<TeamSeasonAdaptiveLineupChoice>,
}

fn certain_probability() -> f64 {
    1.0
}

/// Convert an opponent-specific Bench plan into a one-game IceCast scenario
/// event. The forecast is required so the plan cannot silently affect the
/// wrong opponent or date.
pub fn build_team_season_game_plan_event(
    forecast: &TeamGameForecastView,
    id: impl Into<String>,
    date: NaiveDate,
    plan: &BenchGamePlanView,
) -> Result<TeamSeasonScenarioEvent, String> {
    let id = id.into();
    if id.trim().is_empty() {
        return Err("IceCast game-plan event requires a non-empty ID".to_owned());
    }
    if plan.schema != BENCH_GAME_PLAN_SCHEMA {
        return Err("IceCast game-plan event requires bench_game_plan.v1".to_owned());
    }
    let team = plan.team.trim().to_ascii_uppercase();
    let opponent = plan.opponent.trim().to_ascii_uppercase();
    let matchup = forecast.games.iter().find(|game| {
        game.date == date
            && ((game.home_team.eq_ignore_ascii_case(&team)
                && game.away_team.eq_ignore_ascii_case(&opponent))
                || (game.away_team.eq_ignore_ascii_case(&team)
                    && game.home_team.eq_ignore_ascii_case(&opponent)))
    });
    if matchup.is_none() {
        return Err(format!(
            "IceCast game plan for {team} vs {opponent} does not match the schedule on {date}"
        ));
    }
    if !plan.tactical_matchup_edge.is_finite()
        || !(-3.0..=3.0).contains(&plan.tactical_matchup_edge)
    {
        return Err("IceCast tactical matchup edge must be finite and between -3 and 3".to_owned());
    }
    Ok(TeamSeasonScenarioEvent {
        id,
        kind: TeamSeasonScenarioEventKind::Custom,
        team,
        player: None,
        effective_date: date,
        end_date: Some(date),
        strength_delta: plan.tactical_matchup_edge,
        occurrence_probability: 1.0,
        correlation_key: None,
        label: format!(
            "The Bench: {} vs {} tactical plan ({})",
            plan.team, plan.opponent, plan.manager_profile_id
        ),
    })
}

/// Author a complete, renderer-neutral Bench plan for every scheduled game by
/// one team. Every distinct opponent needs an explicit style input so a data
/// gap cannot silently become a "balanced" tactical assumption.
pub fn build_team_season_game_plan_schedule(
    forecast: &TeamGameForecastView,
    lineup: &TeamLineupProjectionView,
    profile: &TeamDecisionProfile,
    player_roles: &[PlayerMatchupRoleInput],
    opponent_inputs: &[TeamSeasonOpponentGamePlanInput],
) -> Result<TeamSeasonGamePlanScheduleView, String> {
    let team = lineup.team.trim().to_ascii_uppercase();
    if forecast.season != lineup.roster_season || profile.season != lineup.roster_season {
        return Err(
            "The Bench schedule requires one matching forecast, lineup, and profile season"
                .to_owned(),
        );
    }
    if !profile.team.eq_ignore_ascii_case(&team) {
        return Err("The Bench schedule profile and lineup identify different teams".to_owned());
    }
    let team_games = forecast
        .games
        .iter()
        .filter(|game| {
            game.home_team.eq_ignore_ascii_case(&team) || game.away_team.eq_ignore_ascii_case(&team)
        })
        .collect::<Vec<_>>();
    if team_games.is_empty() {
        return Err(format!("The Bench schedule contains no games for {team}"));
    }
    let scheduled_opponents = team_games
        .iter()
        .map(|game| {
            if game.home_team.eq_ignore_ascii_case(&team) {
                game.away_team.to_ascii_uppercase()
            } else {
                game.home_team.to_ascii_uppercase()
            }
        })
        .collect::<BTreeSet<_>>();
    let mut inputs = BTreeMap::new();
    for input in opponent_inputs {
        let opponent = input.opponent.trim().to_ascii_uppercase();
        if inputs.insert(opponent.clone(), input).is_some() {
            return Err(format!(
                "The Bench schedule has duplicate opponent style input for {opponent}"
            ));
        }
    }
    let supplied_opponents = inputs.keys().cloned().collect::<BTreeSet<_>>();
    let missing = scheduled_opponents
        .difference(&supplied_opponents)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "The Bench schedule is missing opponent style input for {}",
            missing.join(", ")
        ));
    }
    let extra = supplied_opponents
        .difference(&scheduled_opponents)
        .cloned()
        .collect::<Vec<_>>();
    if !extra.is_empty() {
        return Err(format!(
            "The Bench schedule has unscheduled opponent style input for {}",
            extra.join(", ")
        ));
    }

    let mut games = Vec::with_capacity(team_games.len());
    for game in team_games {
        let is_home = game.home_team.eq_ignore_ascii_case(&team);
        let (opponent, team_context, opponent_context) = if is_home {
            (&game.away_team, &game.home_context, &game.away_context)
        } else {
            (&game.home_team, &game.away_context, &game.home_context)
        };
        let opponent_input = inputs
            .get(&opponent.to_ascii_uppercase())
            .expect("complete opponent inputs validated");
        let plan = build_bench_game_plan(
            lineup,
            profile,
            &BenchGamePlanInput {
                opponent: opponent.to_ascii_uppercase(),
                opponent_style: opponent_input.opponent_style,
                opponent_primary_threat: opponent_input.opponent_primary_threat.clone(),
                schedule_load: schedule_load(is_home, team_context),
                opponent_schedule_load: Some(schedule_load(!is_home, opponent_context)),
                player_roles: player_roles.to_vec(),
            },
        )?;
        let event = build_team_season_game_plan_event(
            forecast,
            format!("bench-{}-{}", team.to_ascii_lowercase(), game.game_id),
            game.date,
            &plan,
        )?;
        games.push(TeamSeasonScheduledGamePlanRow {
            game_id: game.game_id,
            date: game.date,
            opponent: opponent.to_ascii_uppercase(),
            is_home,
            plan,
            event,
        });
    }
    let scenario = TeamSeasonScenario {
        name: format!("The Bench — {team} opponent game plans"),
        trade_deadline: None,
        events: games.iter().map(|row| row.event.clone()).collect(),
        adaptive_lineup_policies: Vec::new(),
        opening_roster_policies: Vec::new(),
    };
    Ok(TeamSeasonGamePlanScheduleView {
        schema: TEAM_SEASON_GAME_PLAN_SCHEDULE_SCHEMA.to_owned(),
        team,
        season: forecast.season,
        games,
        scenario,
        disclosures: vec![
            "Every scheduled opponent requires an explicit tactical-style input; missing styles stop authorship instead of defaulting to balanced.".to_owned(),
            "Each plan uses IceCast's home/away, back-to-back, three-in-four, and travel context for both teams.".to_owned(),
            "Simulation events contain only the bounded tactical matchup edge; IceCast retains sole ownership of the direct schedule-fatigue effect.".to_owned(),
        ],
    })
}

/// Connect sealed historical evidence documents to the full-schedule Bench
/// author. Only opponents actually on this team's schedule are selected from
/// a league-wide style table, and any scheduled no-read stops authorship.
pub fn build_team_season_game_plan_schedule_from_evidence(
    forecast: &TeamGameForecastView,
    lineup: &TeamLineupProjectionView,
    profile: &TeamDecisionProfile,
    role_evidence: &TeamPlayerMatchupRoleEvidenceView,
    style_evidence: &[OpponentStyleEvidenceRow],
) -> Result<TeamSeasonGamePlanScheduleView, String> {
    if role_evidence.schema != TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA
        || !role_evidence.team.eq_ignore_ascii_case(&lineup.team)
    {
        return Err("The Bench requires matchup-role evidence for the lineup team".to_owned());
    }
    let team = lineup.team.trim().to_ascii_uppercase();
    let scheduled_opponents = forecast
        .games
        .iter()
        .filter_map(|game| {
            if game.home_team.eq_ignore_ascii_case(&team) {
                Some(game.away_team.to_ascii_uppercase())
            } else if game.away_team.eq_ignore_ascii_case(&team) {
                Some(game.home_team.to_ascii_uppercase())
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    let mut styles = BTreeMap::new();
    for row in style_evidence {
        if row.schema != OPPONENT_STYLE_EVIDENCE_SCHEMA {
            return Err("The Bench requires opponent_style_evidence.v1 inputs".to_owned());
        }
        if styles.insert(row.team.to_ascii_uppercase(), row).is_some() {
            return Err(format!(
                "The Bench received duplicate style evidence for {}",
                row.team
            ));
        }
    }
    let mut no_read = Vec::new();
    let mut opponent_inputs = Vec::with_capacity(scheduled_opponents.len());
    for opponent in scheduled_opponents {
        let Some(row) = styles.get(&opponent) else {
            no_read.push(opponent);
            continue;
        };
        let Some(opponent_style) = row.style else {
            no_read.push(opponent);
            continue;
        };
        opponent_inputs.push(TeamSeasonOpponentGamePlanInput {
            opponent,
            opponent_style,
            opponent_primary_threat: None,
        });
    }
    if !no_read.is_empty() {
        return Err(format!(
            "The Bench cannot author scheduled opponents without a style read: {}",
            no_read.join(", ")
        ));
    }
    let roles = role_evidence
        .roles
        .iter()
        .map(|row| row.role.clone())
        .collect::<Vec<_>>();
    let mut view =
        build_team_season_game_plan_schedule(forecast, lineup, profile, &roles, &opponent_inputs)?;
    let style_seasons = style_evidence
        .iter()
        .filter(|row| {
            opponent_inputs
                .iter()
                .any(|input| input.opponent == row.team)
        })
        .map(|row| row.season)
        .collect::<BTreeSet<_>>();
    view.disclosures.push(format!(
        "Player roles use season {} repository evidence; scheduled opponent styles use source season(s) {}.",
        role_evidence.season,
        style_seasons
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    ));
    Ok(view)
}

fn schedule_load(
    is_home: bool,
    context: &super::team_game_forecast::TeamGameScheduleContext,
) -> BenchScheduleLoad {
    BenchScheduleLoad {
        is_home,
        back_to_back: context.back_to_back,
        third_game_in_four_nights: context.three_in_four,
        travel_km: context.travel_km,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamSeasonPersonnelInput {
    pub team: String,
    pub player: String,
    pub position: String,
    pub is_goalie: bool,
    pub age: u8,
    pub games_played: u32,
    /// Comparable 0-100 multi-lens player rating.
    pub rating: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamSeasonAutoPersonnelConfig {
    pub max_players_per_team: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TeamSeasonTradeTeamInput {
    pub team: String,
    pub expected_points: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TeamSeasonPlausibleTradeConfig {
    pub max_trades: usize,
    pub occurrence_probability: f64,
}

impl Default for TeamSeasonPlausibleTradeConfig {
    fn default() -> Self {
        Self {
            max_trades: 6,
            occurrence_probability: 0.30,
        }
    }
}

impl Default for TeamSeasonAutoPersonnelConfig {
    fn default() -> Self {
        Self {
            max_players_per_team: 3,
        }
    }
}

pub fn build_team_season_auto_personnel_scenario(
    schedule_start: NaiveDate,
    schedule_end: NaiveDate,
    seed: u64,
    mut players: Vec<TeamSeasonPersonnelInput>,
    config: TeamSeasonAutoPersonnelConfig,
    trade_deadline: Option<NaiveDate>,
) -> Result<TeamSeasonScenario, String> {
    if schedule_end <= schedule_start {
        return Err(
            "IceCast automatic personnel window requires schedule_end after start".to_owned(),
        );
    }
    if config.max_players_per_team == 0 || config.max_players_per_team > 10 {
        return Err("IceCast automatic personnel max_players_per_team must be 1-10".to_owned());
    }
    if players.iter().any(|player| {
        player.team.trim().is_empty()
            || player.player.trim().is_empty()
            || !player.rating.is_finite()
            || !(0.0..=100.0).contains(&player.rating)
    }) {
        return Err(
            "IceCast automatic personnel inputs require team/player and a 0-100 rating".to_owned(),
        );
    }
    players.sort_by(|a, b| {
        a.team
            .cmp(&b.team)
            .then_with(|| b.rating.total_cmp(&a.rating))
            .then_with(|| a.player.cmp(&b.player))
    });
    let mut selected_per_team = BTreeMap::<String, usize>::new();
    let available_offsets = (schedule_end - schedule_start)
        .num_days()
        .saturating_sub(35)
        .max(1);
    let mut events = Vec::new();
    for player in players {
        let selected = selected_per_team.entry(player.team.clone()).or_default();
        if *selected >= config.max_players_per_team {
            continue;
        }
        *selected += 1;
        let hash = stable_hash(&format!("{}:{}:{seed}", player.team, player.player));
        let offset = 14 + (hash % available_offsets as u64) as i64;
        let duration = 7 + (hash.rotate_left(17) % 29) as i64;
        let effective_date = schedule_start + Duration::days(offset);
        let end_date = (effective_date + Duration::days(duration)).min(schedule_end);
        let age_risk = (f64::from(player.age) - 28.0).max(0.0) * 0.008;
        let durability_risk = if player.games_played < 40 {
            0.08
        } else if player.games_played < 65 {
            0.03
        } else {
            0.0
        };
        let base_risk = if player.is_goalie { 0.22 } else { 0.16 };
        let occurrence_probability = (base_risk + age_risk + durability_risk).clamp(0.08, 0.45);
        let impact_scale = if player.is_goalie { 10.0 } else { 7.0 };
        let strength_delta = -((player.rating / 100.0) * impact_scale).clamp(1.5, 10.0);
        events.push(TeamSeasonScenarioEvent {
            id: format!(
                "auto-{}-{}",
                player.team.to_ascii_lowercase(),
                slug(&player.player)
            ),
            kind: if player.is_goalie {
                TeamSeasonScenarioEventKind::Goalie
            } else {
                TeamSeasonScenarioEventKind::Injury
            },
            team: player.team,
            player: Some(player.player.clone()),
            effective_date,
            end_date: Some(end_date),
            strength_delta,
            occurrence_probability,
            correlation_key: None,
            label: format!(
                "automatic {} availability risk for {}",
                if player.is_goalie { "goalie" } else { "injury" },
                player.player
            ),
        });
    }
    Ok(TeamSeasonScenario {
        name: "Automatic personnel risk".to_owned(),
        trade_deadline,
        events,
        adaptive_lineup_policies: Vec::new(),
        opening_roster_policies: Vec::new(),
    })
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn build_team_season_plausible_trade_scenario(
    trade_deadline: NaiveDate,
    mut teams: Vec<TeamSeasonTradeTeamInput>,
    players: Vec<TeamSeasonPersonnelInput>,
    config: TeamSeasonPlausibleTradeConfig,
) -> Result<TeamSeasonScenario, String> {
    if config.max_trades == 0 || config.max_trades > 16 {
        return Err("IceCast plausible trade max_trades must be 1-16".to_owned());
    }
    if !config.occurrence_probability.is_finite()
        || !(0.0..=1.0).contains(&config.occurrence_probability)
    {
        return Err("IceCast plausible trade occurrence_probability must be 0-1".to_owned());
    }
    if teams.len() < 4 || players.is_empty() {
        return Err("IceCast plausible trades require team outlooks and player records".to_owned());
    }
    if teams
        .iter()
        .any(|team| team.team.trim().is_empty() || !team.expected_points.is_finite())
    {
        return Err("IceCast plausible trade team outlook is invalid".to_owned());
    }
    teams.sort_by(|a, b| {
        b.expected_points
            .total_cmp(&a.expected_points)
            .then_with(|| a.team.cmp(&b.team))
    });
    let trade_count = config.max_trades.min(teams.len() / 4).max(1);
    let buyers = teams.iter().take(trade_count).collect::<Vec<_>>();
    let sellers = teams.iter().rev().take(trade_count).collect::<Vec<_>>();
    let mut events = Vec::new();

    for (buyer, seller) in buyers.into_iter().zip(sellers) {
        let needed_position = weakest_position(&buyer.team, &players);
        let candidate = players
            .iter()
            .filter(|player| {
                player.team == seller.team
                    && position_bucket(&player.position) == needed_position
                    && plausible_deadline_candidate(player)
            })
            .max_by(|a, b| {
                a.rating
                    .total_cmp(&b.rating)
                    .then_with(|| b.player.cmp(&a.player))
            })
            .or_else(|| {
                players
                    .iter()
                    .filter(|player| {
                        player.team == seller.team
                            && !player.is_goalie
                            && plausible_deadline_candidate(player)
                    })
                    .max_by(|a, b| a.rating.total_cmp(&b.rating))
            });
        let Some(player) = candidate else {
            continue;
        };
        let correlation_key = format!(
            "plausible-trade-{}-{}-{}",
            buyer.team,
            seller.team,
            slug(&player.player)
        );
        let seller_impact = -((player.rating / 100.0) * 6.0).clamp(2.0, 6.0);
        let buyer_impact = -seller_impact * 0.80;
        events.push(TeamSeasonScenarioEvent {
            id: format!("{correlation_key}-buyer"),
            kind: TeamSeasonScenarioEventKind::Trade,
            team: buyer.team.clone(),
            player: Some(player.player.clone()),
            effective_date: trade_deadline,
            end_date: None,
            strength_delta: buyer_impact,
            occurrence_probability: config.occurrence_probability,
            correlation_key: Some(correlation_key.clone()),
            label: format!(
                "{} acquires {} ({}) from {} to address {} depth",
                buyer.team, player.player, player.position, seller.team, needed_position
            ),
        });
        events.push(TeamSeasonScenarioEvent {
            id: format!("{correlation_key}-seller"),
            kind: TeamSeasonScenarioEventKind::Trade,
            team: seller.team.clone(),
            player: Some(player.player.clone()),
            effective_date: trade_deadline,
            end_date: None,
            strength_delta: seller_impact,
            occurrence_probability: config.occurrence_probability,
            correlation_key: Some(correlation_key),
            label: format!(
                "{} sends {} ({}) to {}",
                seller.team, player.player, player.position, buyer.team
            ),
        });
    }
    Ok(TeamSeasonScenario {
        name: "Plausible trade market — roster-value proxy".to_owned(),
        trade_deadline: Some(trade_deadline),
        events,
        adaptive_lineup_policies: Vec::new(),
        opening_roster_policies: Vec::new(),
    })
}

fn plausible_deadline_candidate(player: &TeamSeasonPersonnelInput) -> bool {
    // Contract-expiry evidence is not yet part of this input. Keep the
    // generated market in a conservative veteran/rental-like band instead of
    // presenting young core players or elite franchise pieces as routine
    // deadline availability.
    (27..=36).contains(&player.age)
        && player.games_played >= 20
        && (35.0..=80.0).contains(&player.rating)
}

fn weakest_position(team: &str, players: &[TeamSeasonPersonnelInput]) -> &'static str {
    ["F", "D", "G"]
        .into_iter()
        .min_by(|a, b| {
            average_position_rating(team, a, players)
                .total_cmp(&average_position_rating(team, b, players))
        })
        .unwrap_or("F")
}

fn average_position_rating(
    team: &str,
    position: &str,
    players: &[TeamSeasonPersonnelInput],
) -> f64 {
    let values = players
        .iter()
        .filter(|player| player.team == team && position_bucket(&player.position) == position)
        .map(|player| player.rating)
        .collect::<Vec<_>>();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn position_bucket(position: &str) -> &'static str {
    match position {
        "D" => "D",
        "G" => "G",
        _ => "F",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamSeasonSimulationConfig {
    pub trials: u32,
    pub seed: u64,
}

impl Default for TeamSeasonSimulationConfig {
    fn default() -> Self {
        Self {
            trials: 10_000,
            seed: 20_262_027,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastRow {
    pub team: String,
    pub conference: String,
    pub division: String,
    pub average_wins: f64,
    pub average_losses: f64,
    pub average_overtime_losses: f64,
    pub average_points: f64,
    pub points_p10: u16,
    pub points_p50: u16,
    pub points_p90: u16,
    pub average_league_rank: f64,
    pub playoff_probability: f64,
    pub second_round_probability: f64,
    pub conference_final_probability: f64,
    pub stanley_cup_final_probability: f64,
    pub stanley_cup_probability: f64,
    pub presidents_trophy_probability: f64,
    pub average_longest_win_streak: f64,
    pub longest_win_streak_p90: u16,
    /// Includes trials where multiple teams tie for the league lead.
    pub longest_win_streak_leader_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonReplayCheckpointTeamRow {
    pub team: String,
    pub completed_games: usize,
    pub remaining_games: usize,
    pub wins: u16,
    pub losses: u16,
    pub overtime_losses: u16,
    pub standings_points: u16,
    pub expected_remaining_wins: f64,
    pub expected_remaining_losses: f64,
    pub expected_remaining_overtime_losses: f64,
    pub expected_remaining_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonReplayCheckpointView {
    pub as_of_date: NaiveDate,
    pub league_completed_games: usize,
    pub league_remaining_games: usize,
    pub teams: Vec<TeamSeasonReplayCheckpointTeamRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenarioImpactRow {
    pub team: String,
    pub average_points_delta: f64,
    pub playoff_probability_delta: f64,
    pub second_round_probability_delta: f64,
    pub conference_final_probability_delta: f64,
    pub stanley_cup_final_probability_delta: f64,
    pub stanley_cup_probability_delta: f64,
    pub presidents_trophy_probability_delta: f64,
    pub average_longest_win_streak_delta: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastMovementRow {
    pub team: String,
    pub average_points_delta: f64,
    pub playoff_probability_delta: f64,
    pub stanley_cup_probability_delta: f64,
    pub average_longest_win_streak_delta: f64,
    pub completed_games_delta: Option<i64>,
    pub observed_standings_points_delta: Option<i64>,
    pub expected_remaining_points_delta: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastMovementView {
    pub schema: String,
    pub season: u32,
    pub trials: u32,
    pub seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub earlier_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub later_label: Option<String>,
    pub earlier_as_of_date: Option<NaiveDate>,
    pub later_as_of_date: Option<NaiveDate>,
    pub earlier_fingerprint: String,
    pub later_fingerprint: String,
    pub teams: Vec<TeamSeasonForecastMovementRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastHistoryCheckpointRow {
    pub as_of_date: NaiveDate,
    pub fingerprint: String,
    pub league_completed_games: usize,
    pub league_remaining_games: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastHistoryPointRow {
    pub as_of_date: NaiveDate,
    pub average_points: f64,
    pub points_p10: u16,
    pub points_p50: u16,
    pub points_p90: u16,
    pub playoff_probability: f64,
    pub stanley_cup_probability: f64,
    pub average_longest_win_streak: f64,
    pub completed_games: usize,
    pub remaining_games: usize,
    pub observed_standings_points: u16,
    pub expected_remaining_points: f64,
    pub average_points_delta_from_previous: Option<f64>,
    pub playoff_probability_delta_from_previous: Option<f64>,
    pub stanley_cup_probability_delta_from_previous: Option<f64>,
    pub completed_games_delta_from_previous: Option<usize>,
    pub prior_expected_points_for_completed_interval_from_previous: Option<f64>,
    pub realized_points_vs_prior_remaining_pace_from_previous: Option<f64>,
    pub remaining_outlook_revaluation_from_previous: Option<f64>,
    pub pace_attribution_reconciliation_error_from_previous: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSeasonForecastHistoryTrend {
    Improving,
    Declining,
    Mixed,
    Stable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSeasonForecastHistoryMateriality {
    Small,
    Moderate,
    Large,
    Indeterminate,
}

impl TeamSeasonForecastHistoryMateriality {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Moderate => "moderate",
            Self::Large => "large",
            Self::Indeterminate => "indeterminate",
        }
    }
}

impl TeamSeasonForecastHistoryTrend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Improving => "improving",
            Self::Declining => "declining",
            Self::Mixed => "mixed",
            Self::Stable => "stable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastHistoryTeamRow {
    pub team: String,
    pub checkpoints: Vec<TeamSeasonForecastHistoryPointRow>,
    pub average_points_delta_first_to_last: f64,
    pub playoff_probability_delta_first_to_last: f64,
    pub stanley_cup_probability_delta_first_to_last: f64,
    pub projected_points_movement_rank: usize,
    pub league_team_count: usize,
    pub projected_points_trend: TeamSeasonForecastHistoryTrend,
    pub largest_projected_points_swing: f64,
    pub largest_swing_from_date: NaiveDate,
    pub largest_swing_to_date: NaiveDate,
    pub average_first_last_points_range_width: f64,
    pub net_points_movement_share_of_range: Option<f64>,
    pub net_points_movement_materiality: TeamSeasonForecastHistoryMateriality,
    pub observed_standings_points_delta_first_to_last: i64,
    pub expected_remaining_points_delta_first_to_last: f64,
    pub points_movement_reconciliation_error: f64,
    pub completed_games_delta_first_to_last: usize,
    pub prior_expected_points_per_remaining_game: Option<f64>,
    pub prior_expected_points_for_completed_interval: Option<f64>,
    pub realized_points_vs_prior_remaining_pace: Option<f64>,
    pub remaining_outlook_revaluation: Option<f64>,
    pub pace_attribution_reconciliation_error: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastHistoryMoverRow {
    pub rank: usize,
    pub team: String,
    pub average_points_delta_first_to_last: f64,
    pub playoff_probability_delta_first_to_last: f64,
    pub stanley_cup_probability_delta_first_to_last: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastHistoryView {
    pub schema: String,
    pub season: u32,
    pub trials: u32,
    pub seed: u64,
    pub checkpoints: Vec<TeamSeasonForecastHistoryCheckpointRow>,
    pub teams: Vec<TeamSeasonForecastHistoryTeamRow>,
    pub biggest_risers: Vec<TeamSeasonForecastHistoryMoverRow>,
    pub biggest_fallers: Vec<TeamSeasonForecastHistoryMoverRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScenarioOutcomeRow {
    pub team: String,
    pub positive_events: u16,
    pub negative_events: u16,
    pub trials: u32,
    pub probability: f64,
    /// Average sum of the sampled event deltas in this bucket. Dated events
    /// may apply for only part of the season.
    pub average_sampled_strength_delta: f64,
    pub average_points: f64,
    pub playoff_probability: f64,
    pub stanley_cup_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonAdaptiveLineupChoiceSummaryRow {
    pub id: String,
    pub label: String,
    pub strength_delta: f64,
    pub average_games: f64,
    pub finish_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonAdaptiveLineupSummaryRow {
    pub team: String,
    pub review_games: u8,
    pub minimum_points_percentage: f64,
    pub switch_probability: f64,
    pub average_changes: f64,
    pub choices: Vec<TeamSeasonAdaptiveLineupChoiceSummaryRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonOpeningRosterChoiceSummaryRow {
    pub id: String,
    pub label: String,
    pub configured_probability: f64,
    pub sampled_probability: f64,
    pub strength_delta: f64,
    pub roster_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonOpeningRosterSummaryRow {
    pub team: String,
    pub choices: Vec<TeamSeasonOpeningRosterChoiceSummaryRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonPivotalGameRow {
    pub game_id: u64,
    pub date: NaiveDate,
    pub away_team: String,
    pub home_team: String,
    pub hunt_probability: f64,
    pub spoiler_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonProbabilityLeaderRow {
    pub rank: usize,
    pub team: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonLeagueLeaders {
    pub presidents_trophy: Vec<TeamSeasonProbabilityLeaderRow>,
    pub stanley_cup: Vec<TeamSeasonProbabilityLeaderRow>,
    pub longest_win_streak: Vec<TeamSeasonProbabilityLeaderRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamSeasonStretchKind {
    Hardest,
    Easiest,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonScheduleStretchRow {
    pub team: String,
    pub kind: TeamSeasonStretchKind,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub opponents: Vec<String>,
    pub expected_wins: f64,
    pub average_win_probability: f64,
    pub away_games: usize,
    pub back_to_backs: usize,
    pub travel_km: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamSeasonForecastView {
    pub schema: String,
    pub season: u32,
    /// When present, final games through this date are fixed and only later games are sampled.
    #[serde(default)]
    pub as_of_date: Option<NaiveDate>,
    #[serde(default)]
    pub replay_checkpoint: Option<TeamSeasonReplayCheckpointView>,
    pub trials: u32,
    pub seed: u64,
    pub schedule_games: usize,
    pub scenario: Option<TeamSeasonScenario>,
    #[serde(default)]
    pub scenario_reference: Option<ScenarioRegistryReferenceView>,
    #[serde(default)]
    pub scenario_fingerprint: Option<String>,
    pub games: Vec<TeamGameForecastRow>,
    pub accuracy: Option<TeamGameForecastAccuracySummary>,
    pub personnel_evidence: Vec<TeamGamePersonnelEvidenceRow>,
    pub membership_intervals: Vec<TeamGameMembershipIntervalRow>,
    pub membership_anomalies: Vec<TeamGameMembershipAnomalyRow>,
    #[serde(default)]
    pub opening_roster_authority: Option<TeamGameOpeningRosterAuthorityRow>,
    #[serde(default)]
    pub opening_strengths: Vec<TeamGameOpeningStrengthRow>,
    #[serde(default)]
    pub paired_trades: Vec<TeamGamePairedTradeRow>,
    pub teams: Vec<TeamSeasonForecastRow>,
    /// Scenario minus a same-seed counterfactual; populated by comparison workflows.
    #[serde(default)]
    pub scenario_impacts: Vec<TeamSeasonScenarioImpactRow>,
    /// Same-seed impact with proposed trade events forced to occur.
    #[serde(default)]
    pub conditional_scenario_impacts: Vec<TeamSeasonScenarioImpactRow>,
    /// Optional same-seed one-event attribution document, built only on request.
    #[serde(default)]
    pub isolated_impact: Option<IsolatedImpactView>,
    /// Conditional results grouped by sampled positive/negative event counts.
    #[serde(default)]
    pub scenario_outcomes: Vec<TeamSeasonScenarioOutcomeRow>,
    /// Aggregate result-aware lineup decisions made across seeded trials.
    #[serde(default)]
    pub adaptive_lineup_summaries: Vec<TeamSeasonAdaptiveLineupSummaryRow>,
    /// Aggregate mutually exclusive opening-roster draws across season trials.
    #[serde(default)]
    pub opening_roster_summaries: Vec<TeamSeasonOpeningRosterSummaryRow>,
    pub pivotal_games: Vec<TeamSeasonPivotalGameRow>,
    pub league_leaders: TeamSeasonLeagueLeaders,
    pub schedule_stretches: Vec<TeamSeasonScheduleStretchRow>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
}

/// Rehydrate the game-forecast envelope retained inside an archived season
/// simulation so it can be replayed with a new scenario. Season artifacts
/// intentionally keep every game and evidence row, but the v1 schema predates
/// retention of the forecast parameters and summary envelope. Callers must
/// therefore provide the parameters explicitly.
pub fn rehydrate_team_game_forecast(
    season: &TeamSeasonForecastView,
    parameters: TeamForecastParameters,
) -> Result<TeamGameForecastView, String> {
    if season.schema != TEAM_SEASON_FORECAST_SCHEMA {
        return Err("IceCast season replay requires team_season_forecast.v1".to_owned());
    }
    let schedule_start = season
        .games
        .iter()
        .map(|game| game.date)
        .min()
        .ok_or_else(|| "IceCast season replay requires at least one retained game".to_owned())?;
    let schedule_end = season
        .games
        .iter()
        .map(|game| game.date)
        .max()
        .expect("non-empty retained games validated");
    if season.schedule_games != season.games.len() {
        return Err(format!(
            "IceCast season replay expected {} retained games but found {}",
            season.schedule_games,
            season.games.len()
        ));
    }

    let mut summaries = BTreeMap::<String, TeamGameForecastSummaryRow>::new();
    for game in &season.games {
        let home =
            summaries
                .entry(game.home_team.clone())
                .or_insert_with(|| TeamGameForecastSummaryRow {
                    team: game.home_team.clone(),
                    games: 0,
                    home_games: 0,
                    away_games: 0,
                    favored_games: 0,
                    expected_standings_points: 0.0,
                });
        home.games += 1;
        home.home_games += 1;
        home.favored_games += usize::from(game.favored_team == game.home_team);
        home.expected_standings_points += game.home_expected_standings_points;

        let away =
            summaries
                .entry(game.away_team.clone())
                .or_insert_with(|| TeamGameForecastSummaryRow {
                    team: game.away_team.clone(),
                    games: 0,
                    home_games: 0,
                    away_games: 0,
                    favored_games: 0,
                    expected_standings_points: 0.0,
                });
        away.games += 1;
        away.away_games += 1;
        away.favored_games += usize::from(game.favored_team == game.away_team);
        away.expected_standings_points += game.away_expected_standings_points;
    }

    let mut warnings = season.warnings.clone();
    warnings.push(format!(
        "Rehydrated from an archived season simulation; forecast parameters were supplied explicitly as '{}'.",
        parameters.name
    ));
    Ok(TeamGameForecastView {
        schema: TEAM_GAME_FORECAST_SCHEMA.to_owned(),
        season: season.season,
        schedule_games: season.schedule_games,
        schedule_start,
        schedule_end,
        parameters,
        forecast_mode: "rehydrated-season-artifact".to_owned(),
        games: season.games.clone(),
        teams: summaries.into_values().collect(),
        accuracy: season.accuracy.clone(),
        personnel_evidence: season.personnel_evidence.clone(),
        membership_intervals: season.membership_intervals.clone(),
        membership_anomalies: season.membership_anomalies.clone(),
        opening_roster_authority: season.opening_roster_authority.clone(),
        opening_strengths: season.opening_strengths.clone(),
        paired_trades: season.paired_trades.clone(),
        warnings,
        disclosures: season.disclosures.clone(),
    })
}

#[derive(Debug, Clone, Default)]
struct TrialTeam {
    wins: u16,
    losses: u16,
    overtime_losses: u16,
    current_win_streak: u16,
    longest_win_streak: u16,
    recent_points: Vec<u8>,
}

impl TrialTeam {
    fn points(&self) -> u16 {
        self.wins * 2 + self.overtime_losses
    }

    fn games_played(&self) -> u16 {
        self.wins + self.losses + self.overtime_losses
    }

    fn recent_points_percentage(&self) -> f64 {
        if self.recent_points.is_empty() {
            0.5
        } else {
            f64::from(self.recent_points.iter().sum::<u8>())
                / (self.recent_points.len() as f64 * 2.0)
        }
    }

    fn push_recent(&mut self, points: u8) {
        self.recent_points.push(points);
        if self.recent_points.len() > 5 {
            self.recent_points.remove(0);
        }
    }

    fn record_win(&mut self) {
        self.wins += 1;
        self.current_win_streak += 1;
        self.longest_win_streak = self.longest_win_streak.max(self.current_win_streak);
        self.push_recent(2);
    }

    fn record_loss(&mut self, overtime: bool) {
        if overtime {
            self.overtime_losses += 1;
            self.push_recent(1);
        } else {
            self.losses += 1;
            self.push_recent(0);
        }
        self.current_win_streak = 0;
    }
}

#[derive(Debug, Clone, Default)]
struct AggregateTeam {
    wins: u64,
    losses: u64,
    overtime_losses: u64,
    points: Vec<u16>,
    league_rank_sum: u64,
    playoffs: u64,
    second_rounds: u64,
    conference_finals: u64,
    stanley_cup_finals: u64,
    stanley_cups: u64,
    presidents: u64,
    longest_win_streaks: Vec<u16>,
    longest_win_streak_leads: u64,
}

#[derive(Debug, Clone, Default)]
struct AggregateScenarioOutcome {
    trials: u64,
    sampled_strength_delta_sum: f64,
    points: u64,
    playoffs: u64,
    stanley_cups: u64,
}

#[derive(Debug, Clone, Default)]
struct AggregateAdaptiveLineup {
    trials_with_switch: u64,
    changes: u64,
    games_by_choice: Vec<u64>,
    finishes_by_choice: Vec<u64>,
}

#[derive(Debug, Clone, Default)]
struct TrialAdaptiveLineup {
    choice_index: usize,
    games_in_window: u8,
    points_in_window: u16,
    changes: u8,
    games_by_choice: Vec<u16>,
}

#[derive(Debug, Clone, Default)]
struct TrialPlayoffResult {
    second_round: BTreeSet<String>,
    conference_final: BTreeSet<String>,
    stanley_cup_final: BTreeSet<String>,
    champion: Option<String>,
}

pub fn simulate_team_season_forecast(
    forecast: &TeamGameForecastView,
    config: TeamSeasonSimulationConfig,
) -> Result<TeamSeasonForecastView, String> {
    simulate_team_season_forecast_impl(forecast, config, None, None)
}

pub fn simulate_team_season_forecast_with_scenario(
    forecast: &TeamGameForecastView,
    config: TeamSeasonSimulationConfig,
    scenario: Option<TeamSeasonScenario>,
) -> Result<TeamSeasonForecastView, String> {
    simulate_team_season_forecast_impl(forecast, config, scenario, None)
}

/// Simulate only the unknown remainder of a season after fixing every final
/// result through `as_of_date`. Callers must remove all later result labels
/// before building `forecast`; this guard makes future-result leakage explicit.
pub fn simulate_team_season_forecast_as_of_with_scenario(
    forecast: &TeamGameForecastView,
    config: TeamSeasonSimulationConfig,
    scenario: Option<TeamSeasonScenario>,
    as_of_date: NaiveDate,
) -> Result<TeamSeasonForecastView, String> {
    simulate_team_season_forecast_impl(forecast, config, scenario, Some(as_of_date))
}

fn simulate_team_season_forecast_impl(
    forecast: &TeamGameForecastView,
    config: TeamSeasonSimulationConfig,
    scenario: Option<TeamSeasonScenario>,
    as_of_date: Option<NaiveDate>,
) -> Result<TeamSeasonForecastView, String> {
    if config.trials == 0 || config.trials > 1_000_000 {
        return Err("IceCast trials must be between 1 and 1,000,000".to_owned());
    }
    if forecast.games.is_empty() {
        return Err("IceCast cannot simulate an empty game forecast".to_owned());
    }
    if let Some(cutoff) = as_of_date {
        if cutoff < forecast.schedule_start || cutoff > forecast.schedule_end {
            return Err(format!(
                "IceCast as-of date {cutoff} must be within {} through {}",
                forecast.schedule_start, forecast.schedule_end
            ));
        }
        if let Some(game) = forecast
            .games
            .iter()
            .find(|game| game.date <= cutoff && game.actual_winner.is_none())
        {
            return Err(format!(
                "IceCast as-of replay requires a final result for game {} on {}",
                game.game_id, game.date
            ));
        }
        if let Some(game) = forecast.games.iter().find(|game| {
            game.date <= cutoff
                && game
                    .actual_winner
                    .as_deref()
                    .is_some_and(|winner| winner != game.home_team && winner != game.away_team)
        }) {
            return Err(format!(
                "IceCast as-of replay has an invalid winner for game {} on {}",
                game.game_id, game.date
            ));
        }
        if let Some(game) = forecast
            .games
            .iter()
            .find(|game| game.date > cutoff && game.actual_winner.is_some())
        {
            return Err(format!(
                "IceCast as-of replay detected a future result for game {} on {} after cutoff {cutoff}",
                game.game_id, game.date
            ));
        }
    }
    if forecast.season < 20212022 {
        return Err(format!(
            "IceCast playoff simulation supports 2021-22 and later alignment; season {} requires historical division and playoff-rule authority",
            forecast.season
        ));
    }
    let teams = forecast
        .games
        .iter()
        .flat_map(|game| [&game.home_team, &game.away_team])
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    for team in &teams {
        alignment(team)
            .ok_or_else(|| format!("IceCast playoff alignment is missing team {team}"))?;
    }
    if teams.len() != 32 {
        return Err(format!(
            "IceCast season playoff simulation requires all 32 NHL teams; forecast contains {}",
            teams.len()
        ));
    }
    for division in ["Atlantic", "Metropolitan", "Central", "Pacific"] {
        let members = teams
            .iter()
            .filter(|team| alignment(team).is_some_and(|value| value.1 == division))
            .count();
        if members != 8 {
            return Err(format!(
                "IceCast season playoff simulation requires 8 teams in the {division} division; forecast contains {members}"
            ));
        }
    }
    validate_scenario(scenario.as_ref(), &teams, forecast)?;
    let strength_by_team = forecast
        .games
        .iter()
        .flat_map(|game| {
            [
                (game.home_team.clone(), game.home_strength),
                (game.away_team.clone(), game.away_strength),
            ]
        })
        .collect::<BTreeMap<_, _>>();
    let mut aggregate = teams
        .iter()
        .map(|team| (team.clone(), AggregateTeam::default()))
        .collect::<BTreeMap<_, _>>();
    let scenario_teams = scenario
        .iter()
        .flat_map(|scenario| &scenario.events)
        .map(|event| event.team.trim().to_ascii_uppercase())
        .collect::<BTreeSet<_>>();
    let mut scenario_outcomes = BTreeMap::<(String, u16, u16), AggregateScenarioOutcome>::new();
    let mut adaptive_aggregates = scenario
        .iter()
        .flat_map(|value| &value.adaptive_lineup_policies)
        .map(|policy| {
            (
                policy.team.trim().to_ascii_uppercase(),
                AggregateAdaptiveLineup {
                    games_by_choice: vec![0; policy.choices.len()],
                    finishes_by_choice: vec![0; policy.choices.len()],
                    ..AggregateAdaptiveLineup::default()
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut opening_roster_aggregates = scenario
        .iter()
        .flat_map(|value| &value.opening_roster_policies)
        .map(|policy| {
            (
                policy.team.trim().to_ascii_uppercase(),
                vec![0_u64; policy.choices.len()],
            )
        })
        .collect::<BTreeMap<_, _>>();
    let opening_roster_cumulative = build_opening_roster_cumulative(scenario.as_ref());
    let race_window_start = forecast.schedule_end - Duration::days(45);
    let mut pivotal_counts = forecast
        .games
        .iter()
        .map(|game| (game.game_id, (0_u64, 0_u64)))
        .collect::<BTreeMap<_, _>>();

    for trial in 0..config.trials {
        let seed = trial_seed(config.seed, trial);
        let mut rng = SimRng::new(seed);
        let active_events = sample_scenario_events(scenario.as_ref(), seed);
        let opening_rosters =
            sample_opening_rosters(scenario.as_ref(), &opening_roster_cumulative, seed);
        for (team, choice_index) in &opening_rosters {
            opening_roster_aggregates
                .get_mut(team)
                .expect("validated opening-roster aggregate exists")[*choice_index] += 1;
        }
        let mut state = teams
            .iter()
            .map(|team| (team.clone(), TrialTeam::default()))
            .collect::<BTreeMap<_, _>>();
        let mut adaptive_states = scenario
            .iter()
            .flat_map(|value| &value.adaptive_lineup_policies)
            .map(|policy| {
                (
                    policy.team.trim().to_ascii_uppercase(),
                    TrialAdaptiveLineup {
                        games_by_choice: vec![0; policy.choices.len()],
                        ..TrialAdaptiveLineup::default()
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut race_rank_date = None;
        let mut race_ranks = BTreeMap::new();
        for game in &forecast.games {
            if as_of_date.is_some_and(|cutoff| game.date <= cutoff) {
                let home_before = state[&game.home_team].points();
                let away_before = state[&game.away_team].points();
                record_actual_game(game, &mut state);
                record_adaptive_result(
                    scenario.as_ref(),
                    &mut adaptive_states,
                    &game.home_team,
                    state[&game.home_team].points() - home_before,
                );
                record_adaptive_result(
                    scenario.as_ref(),
                    &mut adaptive_states,
                    &game.away_team,
                    state[&game.away_team].points() - away_before,
                );
                race_rank_date = None;
                continue;
            }
            let (hunt_game, spoiler_game, race_edge) = if game.date >= race_window_start {
                if race_rank_date != Some(game.date) {
                    race_ranks = conference_ranks(&state);
                    race_rank_date = Some(game.date);
                }
                race_game_context(&race_ranks, &game.home_team, &game.away_team)
            } else {
                (false, false, 0.0)
            };
            if hunt_game {
                pivotal_counts
                    .get_mut(&game.game_id)
                    .expect("game exists")
                    .0 += 1;
            }
            if spoiler_game {
                pivotal_counts
                    .get_mut(&game.game_id)
                    .expect("game exists")
                    .1 += 1;
            }
            let form_edge = recent_form_edge(&state, &game.home_team, &game.away_team);
            let (mut home_delta, mut away_delta) = active_strength_delta(
                scenario.as_ref(),
                &active_events,
                game.date,
                &game.home_team,
                &game.away_team,
            );
            home_delta +=
                adaptive_strength_delta(scenario.as_ref(), &adaptive_states, &game.home_team);
            away_delta +=
                adaptive_strength_delta(scenario.as_ref(), &adaptive_states, &game.away_team);
            home_delta +=
                opening_roster_strength_delta(scenario.as_ref(), &opening_rosters, &game.home_team);
            away_delta +=
                opening_roster_strength_delta(scenario.as_ref(), &opening_rosters, &game.away_team);
            let home_before = state[&game.home_team].points();
            let away_before = state[&game.away_team].points();
            sample_game(
                game,
                home_delta,
                away_delta,
                forecast.parameters.strength_edge_scale,
                race_edge + form_edge,
                &mut state,
                &mut rng,
            );
            record_adaptive_result(
                scenario.as_ref(),
                &mut adaptive_states,
                &game.home_team,
                state[&game.home_team].points() - home_before,
            );
            record_adaptive_result(
                scenario.as_ref(),
                &mut adaptive_states,
                &game.away_team,
                state[&game.away_team].points() - away_before,
            );
        }

        let ranked = rank_teams(&state);
        let playoff_teams = select_playoff_teams(&state);
        let mut playoff_strengths = strength_by_team.clone();
        for (team, adaptive_state) in &adaptive_states {
            if let Some(policy) = adaptive_policy(scenario.as_ref(), team) {
                *playoff_strengths.entry(team.clone()).or_insert(50.0) +=
                    policy.choices[adaptive_state.choice_index].strength_delta;
            }
        }
        for (team, choice_index) in &opening_rosters {
            if let Some(policy) = opening_roster_policy(scenario.as_ref(), team) {
                *playoff_strengths.entry(team.clone()).or_insert(50.0) +=
                    policy.choices[*choice_index].strength_delta;
            }
        }
        let playoff_result = simulate_playoffs(
            &state,
            scenario.as_ref(),
            &active_events,
            forecast.schedule_end + Duration::days(1),
            &playoff_strengths,
            &forecast.parameters,
            &mut rng,
        );
        for (team, adaptive_state) in &adaptive_states {
            let aggregate = adaptive_aggregates
                .get_mut(team)
                .expect("validated adaptive policy aggregate exists");
            aggregate.trials_with_switch += u64::from(adaptive_state.changes > 0);
            aggregate.changes += u64::from(adaptive_state.changes);
            aggregate.finishes_by_choice[adaptive_state.choice_index] += 1;
            for (index, games) in adaptive_state.games_by_choice.iter().enumerate() {
                aggregate.games_by_choice[index] += u64::from(*games);
            }
        }
        let best_points = state.values().map(TrialTeam::points).max().unwrap_or(0);
        let best_wins = state
            .values()
            .filter(|team| team.points() == best_points)
            .map(|team| team.wins)
            .max()
            .unwrap_or(0);
        let longest_streak = state
            .values()
            .map(|team| team.longest_win_streak)
            .max()
            .unwrap_or(0);

        for (rank, team) in ranked.iter().enumerate() {
            let trial_team = &state[team];
            let row = aggregate.get_mut(team).expect("aggregate team exists");
            row.wins += u64::from(trial_team.wins);
            row.losses += u64::from(trial_team.losses);
            row.overtime_losses += u64::from(trial_team.overtime_losses);
            row.points.push(trial_team.points());
            row.league_rank_sum += rank as u64 + 1;
            row.longest_win_streaks.push(trial_team.longest_win_streak);
            if playoff_teams.contains(team) {
                row.playoffs += 1;
            }
            if playoff_result.second_round.contains(team) {
                row.second_rounds += 1;
            }
            if playoff_result.conference_final.contains(team) {
                row.conference_finals += 1;
            }
            if playoff_result.stanley_cup_final.contains(team) {
                row.stanley_cup_finals += 1;
            }
            if playoff_result.champion.as_ref() == Some(team) {
                row.stanley_cups += 1;
            }
            if trial_team.points() == best_points && trial_team.wins == best_wins {
                row.presidents += 1;
            }
            if trial_team.longest_win_streak == longest_streak {
                row.longest_win_streak_leads += 1;
            }
            if scenario_teams.contains(team) {
                let sampled = scenario
                    .iter()
                    .flat_map(|scenario| &scenario.events)
                    .filter(|event| {
                        event.team.eq_ignore_ascii_case(team) && active_events.contains(&event.id)
                    })
                    .collect::<Vec<_>>();
                let positive_events = sampled
                    .iter()
                    .filter(|event| event.strength_delta > 0.0)
                    .count() as u16;
                let negative_events = sampled
                    .iter()
                    .filter(|event| event.strength_delta < 0.0)
                    .count() as u16;
                let sampled_strength_delta = sampled
                    .iter()
                    .map(|event| event.strength_delta)
                    .sum::<f64>();
                let outcome = scenario_outcomes
                    .entry((team.clone(), positive_events, negative_events))
                    .or_default();
                outcome.trials += 1;
                outcome.sampled_strength_delta_sum += sampled_strength_delta;
                outcome.points += u64::from(trial_team.points());
                outcome.playoffs += u64::from(playoff_teams.contains(team));
                outcome.stanley_cups += u64::from(playoff_result.champion.as_ref() == Some(team));
            }
        }
    }

    let denominator = f64::from(config.trials);
    let mut rows = aggregate
        .into_iter()
        .map(|(team, mut aggregate)| {
            aggregate.points.sort_unstable();
            aggregate.longest_win_streaks.sort_unstable();
            let (conference, division) = alignment(&team).expect("alignment validated");
            TeamSeasonForecastRow {
                team,
                conference: conference.to_owned(),
                division: division.to_owned(),
                average_wins: aggregate.wins as f64 / denominator,
                average_losses: aggregate.losses as f64 / denominator,
                average_overtime_losses: aggregate.overtime_losses as f64 / denominator,
                average_points: aggregate
                    .points
                    .iter()
                    .map(|value| f64::from(*value))
                    .sum::<f64>()
                    / denominator,
                points_p10: percentile(&aggregate.points, 0.10),
                points_p50: percentile(&aggregate.points, 0.50),
                points_p90: percentile(&aggregate.points, 0.90),
                average_league_rank: aggregate.league_rank_sum as f64 / denominator,
                playoff_probability: aggregate.playoffs as f64 / denominator,
                second_round_probability: aggregate.second_rounds as f64 / denominator,
                conference_final_probability: aggregate.conference_finals as f64 / denominator,
                stanley_cup_final_probability: aggregate.stanley_cup_finals as f64 / denominator,
                stanley_cup_probability: aggregate.stanley_cups as f64 / denominator,
                presidents_trophy_probability: aggregate.presidents as f64 / denominator,
                average_longest_win_streak: aggregate
                    .longest_win_streaks
                    .iter()
                    .map(|value| f64::from(*value))
                    .sum::<f64>()
                    / denominator,
                longest_win_streak_p90: percentile(&aggregate.longest_win_streaks, 0.90),
                longest_win_streak_leader_probability: aggregate.longest_win_streak_leads as f64
                    / denominator,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.average_points
            .total_cmp(&a.average_points)
            .then_with(|| a.team.cmp(&b.team))
    });
    let mut scenario_outcome_rows = scenario_outcomes
        .into_iter()
        .map(|((team, positive_events, negative_events), outcome)| {
            let bucket_denominator = outcome.trials as f64;
            TeamSeasonScenarioOutcomeRow {
                team,
                positive_events,
                negative_events,
                trials: outcome.trials as u32,
                probability: bucket_denominator / denominator,
                average_sampled_strength_delta: outcome.sampled_strength_delta_sum
                    / bucket_denominator,
                average_points: outcome.points as f64 / bucket_denominator,
                playoff_probability: outcome.playoffs as f64 / bucket_denominator,
                stanley_cup_probability: outcome.stanley_cups as f64 / bucket_denominator,
            }
        })
        .collect::<Vec<_>>();
    scenario_outcome_rows.sort_by(|a, b| {
        a.team
            .cmp(&b.team)
            .then_with(|| a.positive_events.cmp(&b.positive_events))
            .then_with(|| a.negative_events.cmp(&b.negative_events))
    });
    let adaptive_lineup_summaries = scenario
        .iter()
        .flat_map(|value| &value.adaptive_lineup_policies)
        .map(|policy| {
            let team = policy.team.trim().to_ascii_uppercase();
            let aggregate = &adaptive_aggregates[&team];
            TeamSeasonAdaptiveLineupSummaryRow {
                team,
                review_games: policy.review_games,
                minimum_points_percentage: policy.minimum_points_percentage,
                switch_probability: aggregate.trials_with_switch as f64 / denominator,
                average_changes: aggregate.changes as f64 / denominator,
                choices: policy
                    .choices
                    .iter()
                    .enumerate()
                    .map(|(index, choice)| TeamSeasonAdaptiveLineupChoiceSummaryRow {
                        id: choice.id.clone(),
                        label: choice.label.clone(),
                        strength_delta: choice.strength_delta,
                        average_games: aggregate.games_by_choice[index] as f64 / denominator,
                        finish_probability: aggregate.finishes_by_choice[index] as f64
                            / denominator,
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let opening_roster_summaries = scenario
        .iter()
        .flat_map(|value| &value.opening_roster_policies)
        .map(|policy| {
            let team = policy.team.trim().to_ascii_uppercase();
            let counts = &opening_roster_aggregates[&team];
            TeamSeasonOpeningRosterSummaryRow {
                team,
                choices: policy
                    .choices
                    .iter()
                    .enumerate()
                    .map(|(index, choice)| TeamSeasonOpeningRosterChoiceSummaryRow {
                        id: choice.id.clone(),
                        label: choice.label.clone(),
                        configured_probability: choice.probability,
                        sampled_probability: counts[index] as f64 / denominator,
                        strength_delta: choice.strength_delta,
                        roster_ids: choice.roster_ids.clone(),
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    let league_leaders = build_league_leaders(&rows);
    let schedule_stretches = build_schedule_stretches(&forecast.games, &teams);
    let mut pivotal_games = forecast
        .games
        .iter()
        .filter_map(|game| {
            let counts = pivotal_counts[&game.game_id];
            (counts.0 > 0 || counts.1 > 0).then(|| TeamSeasonPivotalGameRow {
                game_id: game.game_id,
                date: game.date,
                away_team: game.away_team.clone(),
                home_team: game.home_team.clone(),
                hunt_probability: counts.0 as f64 / denominator,
                spoiler_probability: counts.1 as f64 / denominator,
            })
        })
        .collect::<Vec<_>>();
    pivotal_games.sort_by(|a, b| {
        b.hunt_probability
            .total_cmp(&a.hunt_probability)
            .then_with(|| b.spoiler_probability.total_cmp(&a.spoiler_probability))
            .then_with(|| a.date.cmp(&b.date))
            .then_with(|| a.game_id.cmp(&b.game_id))
    });

    let mut disclosures = forecast.disclosures.clone();
    disclosures.extend([
        "IceCast samples one shared result for every scheduled game in every seeded trial; team records and league standings therefore reconcile within each trial.".to_owned(),
        "Playoff qualification uses the modern NHL top-three-per-division plus two conference wild cards format; simplified tiebreaks are points, wins, then team abbreviation.".to_owned(),
        "Each trial continues through the modern divisional playoff bracket using best-of-seven 2-2-1-1-1 home ice, roster/depth strength, and scenario events still active after the regular season.".to_owned(),
        "During the final 45 days, conference ranks 7-10 define the hunt and ranks 13-16 can create spoiler matchups; hunt motivation is capped at 0.4 probability points and five-game form at 1.5 points.".to_owned(),
        "League leaderboards rank Presidents' Trophy, Stanley Cup, and longest-win-streak leader probabilities; schedule stretches use each team's lowest and highest baseline average win probability across consecutive five-game windows.".to_owned(),
        "Only events embedded in the run's scenario are applied; automatic personnel events are modeled risks from player records, not live injury or goalie confirmations.".to_owned(),
    ]);
    if let Some(value) = &scenario {
        disclosures.push(format!(
            "Scenario '{}' contains {} dated event(s); occurrence decisions use an independent seeded stream and affect only games on or after each effective date.",
            value.name,
            value.events.len()
        ));
        if !value.adaptive_lineup_policies.is_empty() {
            disclosures.push(format!(
                "The Bench evaluated {} result-aware lineup policy/policies independently in every trial; choices are simulated strength assumptions, not causal chemistry estimates or predictions of a named coach's decisions.",
                value.adaptive_lineup_policies.len()
            ));
        }
        if !value.opening_roster_policies.is_empty() {
            disclosures.push(format!(
                "The Cut sampled exactly one opening roster for each of {} team policy/policies at the start of every trial; that roster's strength delta remains active through the playoffs.",
                value.opening_roster_policies.len()
            ));
        }
    }
    if let Some(cutoff) = as_of_date {
        disclosures.push(format!(
            "Point-in-time replay fixes final results through {cutoff}; later results are absent from forecast inputs and only the remaining schedule is sampled."
        ));
    }
    let scenario_fingerprint = scenario
        .as_ref()
        .map(scenario_content_sha256)
        .transpose()
        .map_err(|error| error.to_string())?;
    let replay_checkpoint =
        as_of_date.map(|cutoff| build_replay_checkpoint(forecast, &rows, cutoff));
    Ok(TeamSeasonForecastView {
        schema: TEAM_SEASON_FORECAST_SCHEMA.to_owned(),
        season: forecast.season,
        as_of_date,
        replay_checkpoint,
        trials: config.trials,
        seed: config.seed,
        schedule_games: forecast.schedule_games,
        scenario,
        scenario_reference: None,
        scenario_fingerprint,
        games: forecast.games.clone(),
        accuracy: forecast.accuracy.clone(),
        personnel_evidence: forecast.personnel_evidence.clone(),
        membership_intervals: forecast.membership_intervals.clone(),
        membership_anomalies: forecast.membership_anomalies.clone(),
        opening_roster_authority: forecast.opening_roster_authority.clone(),
        opening_strengths: forecast.opening_strengths.clone(),
        paired_trades: forecast.paired_trades.clone(),
        teams: rows,
        scenario_impacts: Vec::new(),
        conditional_scenario_impacts: Vec::new(),
        isolated_impact: None,
        scenario_outcomes: scenario_outcome_rows,
        adaptive_lineup_summaries,
        opening_roster_summaries,
        pivotal_games,
        league_leaders,
        schedule_stretches,
        warnings: forecast.warnings.clone(),
        disclosures,
    })
}

fn build_replay_checkpoint(
    forecast: &TeamGameForecastView,
    projections: &[TeamSeasonForecastRow],
    as_of_date: NaiveDate,
) -> TeamSeasonReplayCheckpointView {
    let mut teams = forecast
        .games
        .iter()
        .flat_map(|game| [&game.home_team, &game.away_team])
        .map(|team| {
            (
                team.clone(),
                TeamSeasonReplayCheckpointTeamRow {
                    team: team.clone(),
                    completed_games: 0,
                    remaining_games: 0,
                    wins: 0,
                    losses: 0,
                    overtime_losses: 0,
                    standings_points: 0,
                    expected_remaining_wins: 0.0,
                    expected_remaining_losses: 0.0,
                    expected_remaining_overtime_losses: 0.0,
                    expected_remaining_points: 0.0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut league_completed_games = 0;
    for game in &forecast.games {
        if game.date <= as_of_date {
            league_completed_games += 1;
            let overtime = matches!(game.actual_ending.as_deref(), Some("OT" | "SO"));
            let winner = game
                .actual_winner
                .as_deref()
                .expect("as-of simulation validates fixed results before checkpoint projection");
            for (team, won) in [
                (&game.home_team, winner == game.home_team),
                (&game.away_team, winner == game.away_team),
            ] {
                let row = teams.get_mut(team).expect("scheduled team exists");
                row.completed_games += 1;
                if won {
                    row.wins += 1;
                    row.standings_points += 2;
                } else if overtime {
                    row.overtime_losses += 1;
                    row.standings_points += 1;
                } else {
                    row.losses += 1;
                }
            }
        } else {
            teams
                .get_mut(&game.home_team)
                .expect("home team exists")
                .remaining_games += 1;
            teams
                .get_mut(&game.away_team)
                .expect("away team exists")
                .remaining_games += 1;
        }
    }
    for projection in projections {
        let row = teams
            .get_mut(&projection.team)
            .expect("projected team exists in checkpoint");
        row.expected_remaining_wins = (projection.average_wins - f64::from(row.wins)).max(0.0);
        row.expected_remaining_losses =
            (projection.average_losses - f64::from(row.losses)).max(0.0);
        row.expected_remaining_overtime_losses =
            (projection.average_overtime_losses - f64::from(row.overtime_losses)).max(0.0);
        row.expected_remaining_points =
            (projection.average_points - f64::from(row.standings_points)).max(0.0);
    }
    TeamSeasonReplayCheckpointView {
        as_of_date,
        league_completed_games,
        league_remaining_games: forecast.schedule_games - league_completed_games,
        teams: teams.into_values().collect(),
    }
}

fn build_league_leaders(rows: &[TeamSeasonForecastRow]) -> TeamSeasonLeagueLeaders {
    TeamSeasonLeagueLeaders {
        presidents_trophy: probability_leaders(rows, |row| row.presidents_trophy_probability),
        stanley_cup: probability_leaders(rows, |row| row.stanley_cup_probability),
        longest_win_streak: probability_leaders(rows, |row| {
            row.longest_win_streak_leader_probability
        }),
    }
}

fn probability_leaders(
    rows: &[TeamSeasonForecastRow],
    probability: impl Fn(&TeamSeasonForecastRow) -> f64,
) -> Vec<TeamSeasonProbabilityLeaderRow> {
    let mut ranked = rows.iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        probability(b)
            .total_cmp(&probability(a))
            .then_with(|| a.team.cmp(&b.team))
    });
    ranked
        .into_iter()
        .take(5)
        .enumerate()
        .map(|(index, row)| TeamSeasonProbabilityLeaderRow {
            rank: index + 1,
            team: row.team.clone(),
            probability: probability(row),
        })
        .collect()
}

#[derive(Debug, Clone)]
struct StretchGame {
    date: NaiveDate,
    opponent: String,
    win_probability: f64,
    away: bool,
    back_to_back: bool,
    travel_km: f64,
}

fn build_schedule_stretches(
    games: &[TeamGameForecastRow],
    teams: &[String],
) -> Vec<TeamSeasonScheduleStretchRow> {
    let mut stretches = Vec::new();
    for team in teams {
        let team_games = games
            .iter()
            .filter_map(|game| {
                if game.home_team == *team {
                    Some(StretchGame {
                        date: game.date,
                        opponent: game.away_team.clone(),
                        win_probability: game.home_overall_win_probability,
                        away: false,
                        back_to_back: game.home_context.back_to_back,
                        travel_km: game.home_context.travel_km,
                    })
                } else if game.away_team == *team {
                    Some(StretchGame {
                        date: game.date,
                        opponent: game.home_team.clone(),
                        win_probability: game.away_overall_win_probability,
                        away: true,
                        back_to_back: game.away_context.back_to_back,
                        travel_km: game.away_context.travel_km,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let windows = team_games
            .windows(5)
            .map(|window| stretch_row(team, TeamSeasonStretchKind::Hardest, window))
            .collect::<Vec<_>>();
        if let Some(mut hardest) = windows.iter().cloned().min_by(|a, b| {
            a.average_win_probability
                .total_cmp(&b.average_win_probability)
                .then_with(|| a.start_date.cmp(&b.start_date))
        }) {
            hardest.kind = TeamSeasonStretchKind::Hardest;
            stretches.push(hardest);
        }
        if let Some(mut easiest) = windows.iter().cloned().max_by(|a, b| {
            a.average_win_probability
                .total_cmp(&b.average_win_probability)
                .then_with(|| b.start_date.cmp(&a.start_date))
        }) {
            easiest.kind = TeamSeasonStretchKind::Easiest;
            stretches.push(easiest);
        }
    }
    stretches.sort_by(|a, b| {
        a.team.cmp(&b.team).then_with(|| {
            let order = |kind| match kind {
                TeamSeasonStretchKind::Hardest => 0,
                TeamSeasonStretchKind::Easiest => 1,
            };
            order(a.kind).cmp(&order(b.kind))
        })
    });
    stretches
}

fn stretch_row(
    team: &str,
    kind: TeamSeasonStretchKind,
    games: &[StretchGame],
) -> TeamSeasonScheduleStretchRow {
    let expected_wins = games.iter().map(|game| game.win_probability).sum::<f64>();
    TeamSeasonScheduleStretchRow {
        team: team.to_owned(),
        kind,
        start_date: games.first().expect("five-game window").date,
        end_date: games.last().expect("five-game window").date,
        opponents: games.iter().map(|game| game.opponent.clone()).collect(),
        expected_wins,
        average_win_probability: expected_wins / games.len() as f64,
        away_games: games.iter().filter(|game| game.away).count(),
        back_to_backs: games.iter().filter(|game| game.back_to_back).count(),
        travel_km: games.iter().map(|game| game.travel_km).sum(),
    }
}

pub fn compare_team_season_forecast_scenarios(
    baseline: &TeamSeasonForecastView,
    scenario: &TeamSeasonForecastView,
) -> Result<Vec<TeamSeasonScenarioImpactRow>, String> {
    if baseline.season != scenario.season
        || baseline.trials != scenario.trials
        || baseline.seed != scenario.seed
        || baseline.schedule_games != scenario.schedule_games
    {
        return Err(
            "IceCast scenario comparison requires the same season, schedule, trials, and seed"
                .to_owned(),
        );
    }
    let baseline_by_team = baseline
        .teams
        .iter()
        .map(|team| (team.team.as_str(), team))
        .collect::<BTreeMap<_, _>>();
    if baseline_by_team.len() != scenario.teams.len()
        || scenario
            .teams
            .iter()
            .any(|team| !baseline_by_team.contains_key(team.team.as_str()))
    {
        return Err("IceCast scenario comparison requires identical teams".to_owned());
    }
    let mut impacts = scenario
        .teams
        .iter()
        .map(|team| {
            let baseline = baseline_by_team[team.team.as_str()];
            TeamSeasonScenarioImpactRow {
                team: team.team.clone(),
                average_points_delta: team.average_points - baseline.average_points,
                playoff_probability_delta: team.playoff_probability - baseline.playoff_probability,
                second_round_probability_delta: team.second_round_probability
                    - baseline.second_round_probability,
                conference_final_probability_delta: team.conference_final_probability
                    - baseline.conference_final_probability,
                stanley_cup_final_probability_delta: team.stanley_cup_final_probability
                    - baseline.stanley_cup_final_probability,
                stanley_cup_probability_delta: team.stanley_cup_probability
                    - baseline.stanley_cup_probability,
                presidents_trophy_probability_delta: team.presidents_trophy_probability
                    - baseline.presidents_trophy_probability,
                average_longest_win_streak_delta: team.average_longest_win_streak
                    - baseline.average_longest_win_streak,
            }
        })
        .collect::<Vec<_>>();
    impacts.sort_by(|a, b| {
        b.playoff_probability_delta
            .abs()
            .total_cmp(&a.playoff_probability_delta.abs())
            .then_with(|| a.team.cmp(&b.team))
    });
    Ok(impacts)
}

pub fn build_team_season_forecast_movement(
    earlier: &TeamSeasonForecastView,
    later: &TeamSeasonForecastView,
) -> Result<TeamSeasonForecastMovementView, String> {
    if earlier
        .as_of_date
        .zip(later.as_of_date)
        .is_some_and(|(earlier, later)| earlier > later)
    {
        return Err(
            "IceCast movement requires the earlier cutoff to precede the later cutoff".to_owned(),
        );
    }
    validate_forecast_movement_input("earlier", earlier)?;
    validate_forecast_movement_input("later", later)?;
    if earlier.games.len() != later.games.len()
        || earlier
            .games
            .iter()
            .zip(&later.games)
            .any(|(earlier, later)| {
                earlier.game_id != later.game_id
                    || earlier.date != later.date
                    || earlier.away_team != later.away_team
                    || earlier.home_team != later.home_team
            })
    {
        return Err("IceCast movement requires both runs to use the same schedule".to_owned());
    }
    let impacts = compare_team_season_forecast_scenarios(earlier, later)?;
    let earlier_checkpoint = earlier.replay_checkpoint.as_ref().map(|checkpoint| {
        checkpoint
            .teams
            .iter()
            .map(|row| (row.team.as_str(), row))
            .collect::<BTreeMap<_, _>>()
    });
    let later_checkpoint = later.replay_checkpoint.as_ref().map(|checkpoint| {
        checkpoint
            .teams
            .iter()
            .map(|row| (row.team.as_str(), row))
            .collect::<BTreeMap<_, _>>()
    });
    let mut teams = impacts
        .into_iter()
        .map(|impact| {
            let checkpoints = earlier_checkpoint
                .as_ref()
                .zip(later_checkpoint.as_ref())
                .and_then(|(earlier, later)| {
                    Some((
                        *earlier.get(impact.team.as_str())?,
                        *later.get(impact.team.as_str())?,
                    ))
                });
            TeamSeasonForecastMovementRow {
                team: impact.team,
                average_points_delta: impact.average_points_delta,
                playoff_probability_delta: impact.playoff_probability_delta,
                stanley_cup_probability_delta: impact.stanley_cup_probability_delta,
                average_longest_win_streak_delta: impact.average_longest_win_streak_delta,
                completed_games_delta: checkpoints.map(|(earlier, later)| {
                    later.completed_games as i64 - earlier.completed_games as i64
                }),
                observed_standings_points_delta: checkpoints.map(|(earlier, later)| {
                    i64::from(later.standings_points) - i64::from(earlier.standings_points)
                }),
                expected_remaining_points_delta: checkpoints.map(|(earlier, later)| {
                    later.expected_remaining_points - earlier.expected_remaining_points
                }),
            }
        })
        .collect::<Vec<_>>();
    teams.sort_by(|a, b| {
        b.average_points_delta
            .abs()
            .total_cmp(&a.average_points_delta.abs())
            .then_with(|| a.team.cmp(&b.team))
    });
    Ok(TeamSeasonForecastMovementView {
        schema: TEAM_SEASON_FORECAST_MOVEMENT_SCHEMA.to_owned(),
        season: earlier.season,
        trials: earlier.trials,
        seed: earlier.seed,
        earlier_label: None,
        later_label: None,
        earlier_as_of_date: earlier.as_of_date,
        later_as_of_date: later.as_of_date,
        earlier_fingerprint: forecast_movement_fingerprint(earlier)?,
        later_fingerprint: forecast_movement_fingerprint(later)?,
        teams,
        disclosures: vec![
            "Movement is later minus earlier. Both league runs must share season, schedule size, trials, seed, and teams.".to_owned(),
            "Observed and expected-remainder deltas are present only when both runs contain typed replay checkpoints.".to_owned(),
            "Fingerprints cover each complete league artifact before any team is selected for display.".to_owned(),
        ],
    })
}

#[derive(Debug, Clone, Copy)]
struct HistoryPaceAttribution {
    completed_games_delta: usize,
    prior_expected_points_per_remaining_game: Option<f64>,
    prior_expected_points_for_completed_interval: Option<f64>,
    realized_points_vs_prior_remaining_pace: Option<f64>,
    remaining_outlook_revaluation: Option<f64>,
    reconciliation_error: Option<f64>,
}

fn history_pace_attribution(
    earlier: &TeamSeasonForecastHistoryPointRow,
    later: &TeamSeasonForecastHistoryPointRow,
) -> HistoryPaceAttribution {
    let completed_games_delta = later.completed_games - earlier.completed_games;
    let observed_points_delta =
        i64::from(later.observed_standings_points) - i64::from(earlier.observed_standings_points);
    let prior_expected_points_per_remaining_game = (earlier.remaining_games > 0)
        .then(|| earlier.expected_remaining_points / earlier.remaining_games as f64);
    let prior_expected_points_for_completed_interval =
        prior_expected_points_per_remaining_game.map(|pace| pace * completed_games_delta as f64);
    let realized_points_vs_prior_remaining_pace = prior_expected_points_for_completed_interval
        .map(|expected| observed_points_delta as f64 - expected);
    let remaining_outlook_revaluation =
        prior_expected_points_for_completed_interval.map(|expected| {
            later.expected_remaining_points - (earlier.expected_remaining_points - expected)
        });
    let reconciliation_error = realized_points_vs_prior_remaining_pace
        .zip(remaining_outlook_revaluation)
        .map(|(realized, revaluation)| {
            (later.average_points - earlier.average_points) - (realized + revaluation)
        });
    HistoryPaceAttribution {
        completed_games_delta,
        prior_expected_points_per_remaining_game,
        prior_expected_points_for_completed_interval,
        realized_points_vs_prior_remaining_pace,
        remaining_outlook_revaluation,
        reconciliation_error,
    }
}

pub fn build_team_season_forecast_history(
    forecasts: &[TeamSeasonForecastView],
) -> Result<TeamSeasonForecastHistoryView, String> {
    if forecasts.len() < 2 {
        return Err("IceCast history requires at least two forecast checkpoints".to_owned());
    }
    for (index, forecast) in forecasts.iter().enumerate() {
        if forecast.as_of_date.is_none() || forecast.replay_checkpoint.is_none() {
            return Err(format!(
                "IceCast history checkpoint {} must be a dated point-in-time replay",
                index + 1
            ));
        }
        validate_forecast_movement_input(&format!("history checkpoint {}", index + 1), forecast)?;
    }
    if forecasts.windows(2).any(|pair| {
        pair[0].as_of_date.expect("validated history date")
            >= pair[1].as_of_date.expect("validated history date")
    }) {
        return Err(
            "IceCast history checkpoints must be in strictly increasing date order".to_owned(),
        );
    }
    for pair in forecasts.windows(2) {
        let earlier = pair[0]
            .replay_checkpoint
            .as_ref()
            .expect("validated history checkpoint");
        let later = pair[1]
            .replay_checkpoint
            .as_ref()
            .expect("validated history checkpoint");
        if later.league_completed_games < earlier.league_completed_games
            || later.league_remaining_games > earlier.league_remaining_games
        {
            return Err("IceCast history league replay progress cannot regress".to_owned());
        }
        for earlier_team in &earlier.teams {
            let later_team = later
                .teams
                .iter()
                .find(|team| team.team == earlier_team.team)
                .expect("movement validation preserves checkpoint teams");
            if later_team.completed_games < earlier_team.completed_games
                || later_team.remaining_games > earlier_team.remaining_games
            {
                return Err(format!(
                    "IceCast history replay progress cannot regress for {}",
                    earlier_team.team
                ));
            }
        }
    }

    let movements = forecasts
        .windows(2)
        .map(|pair| build_team_season_forecast_movement(&pair[0], &pair[1]))
        .collect::<Result<Vec<_>, _>>()?;
    let checkpoints = forecasts
        .iter()
        .map(|forecast| {
            let checkpoint = forecast
                .replay_checkpoint
                .as_ref()
                .expect("validated history checkpoint");
            Ok(TeamSeasonForecastHistoryCheckpointRow {
                as_of_date: checkpoint.as_of_date,
                fingerprint: forecast_movement_fingerprint(forecast)?,
                league_completed_games: checkpoint.league_completed_games,
                league_remaining_games: checkpoint.league_remaining_games,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut teams = forecasts[0]
        .teams
        .iter()
        .map(|first_team| {
            let team = first_team.team.as_str();
            let mut points: Vec<TeamSeasonForecastHistoryPointRow> = forecasts
                .iter()
                .enumerate()
                .map(|(index, forecast)| {
                    let projection = forecast
                        .teams
                        .iter()
                        .find(|row| row.team == team)
                        .expect("movement validation preserves forecast teams");
                    let checkpoint = forecast
                        .replay_checkpoint
                        .as_ref()
                        .and_then(|checkpoint| checkpoint.teams.iter().find(|row| row.team == team))
                        .expect("history validation preserves checkpoint teams");
                    let delta = index.checked_sub(1).map(|movement_index| {
                        movements[movement_index]
                            .teams
                            .iter()
                            .find(|row| row.team == team)
                            .expect("movement validation preserves delta teams")
                    });
                    TeamSeasonForecastHistoryPointRow {
                        as_of_date: forecast.as_of_date.expect("validated history date"),
                        average_points: projection.average_points,
                        points_p10: projection.points_p10,
                        points_p50: projection.points_p50,
                        points_p90: projection.points_p90,
                        playoff_probability: projection.playoff_probability,
                        stanley_cup_probability: projection.stanley_cup_probability,
                        average_longest_win_streak: projection.average_longest_win_streak,
                        completed_games: checkpoint.completed_games,
                        remaining_games: checkpoint.remaining_games,
                        observed_standings_points: checkpoint.standings_points,
                        expected_remaining_points: checkpoint.expected_remaining_points,
                        average_points_delta_from_previous: delta
                            .map(|row| row.average_points_delta),
                        playoff_probability_delta_from_previous: delta
                            .map(|row| row.playoff_probability_delta),
                        stanley_cup_probability_delta_from_previous: delta
                            .map(|row| row.stanley_cup_probability_delta),
                        completed_games_delta_from_previous: None,
                        prior_expected_points_for_completed_interval_from_previous: None,
                        realized_points_vs_prior_remaining_pace_from_previous: None,
                        remaining_outlook_revaluation_from_previous: None,
                        pace_attribution_reconciliation_error_from_previous: None,
                    }
                })
                .collect();
            for index in 1..points.len() {
                let attribution = history_pace_attribution(&points[index - 1], &points[index]);
                let point = &mut points[index];
                point.completed_games_delta_from_previous = Some(attribution.completed_games_delta);
                point.prior_expected_points_for_completed_interval_from_previous =
                    attribution.prior_expected_points_for_completed_interval;
                point.realized_points_vs_prior_remaining_pace_from_previous =
                    attribution.realized_points_vs_prior_remaining_pace;
                point.remaining_outlook_revaluation_from_previous =
                    attribution.remaining_outlook_revaluation;
                point.pace_attribution_reconciliation_error_from_previous =
                    attribution.reconciliation_error;
            }
            let first = points.first().expect("history has checkpoints");
            let last = points.last().expect("history has checkpoints");
            let average_points_delta_first_to_last = last.average_points - first.average_points;
            let playoff_probability_delta_first_to_last =
                last.playoff_probability - first.playoff_probability;
            let stanley_cup_probability_delta_first_to_last =
                last.stanley_cup_probability - first.stanley_cup_probability;
            let (projected_points_trend, largest_swing) = history_trend(&points);
            let first_width = f64::from(first.points_p90.saturating_sub(first.points_p10));
            let last_width = f64::from(last.points_p90.saturating_sub(last.points_p10));
            let average_first_last_points_range_width = (first_width + last_width) / 2.0;
            let net_points_movement_share_of_range = (average_first_last_points_range_width > 0.0)
                .then(|| {
                    average_points_delta_first_to_last.abs() / average_first_last_points_range_width
                });
            let net_points_movement_materiality =
                history_movement_materiality(net_points_movement_share_of_range);
            let observed_standings_points_delta_first_to_last =
                i64::from(last.observed_standings_points)
                    - i64::from(first.observed_standings_points);
            let expected_remaining_points_delta_first_to_last =
                last.expected_remaining_points - first.expected_remaining_points;
            let points_movement_reconciliation_error = average_points_delta_first_to_last
                - (observed_standings_points_delta_first_to_last as f64
                    + expected_remaining_points_delta_first_to_last);
            let pace_attribution = history_pace_attribution(first, last);
            TeamSeasonForecastHistoryTeamRow {
                team: first_team.team.clone(),
                checkpoints: points,
                average_points_delta_first_to_last,
                playoff_probability_delta_first_to_last,
                stanley_cup_probability_delta_first_to_last,
                projected_points_movement_rank: 0,
                league_team_count: forecasts[0].teams.len(),
                projected_points_trend,
                largest_projected_points_swing: largest_swing.2,
                largest_swing_from_date: largest_swing.0,
                largest_swing_to_date: largest_swing.1,
                average_first_last_points_range_width,
                net_points_movement_share_of_range,
                net_points_movement_materiality,
                observed_standings_points_delta_first_to_last,
                expected_remaining_points_delta_first_to_last,
                points_movement_reconciliation_error,
                completed_games_delta_first_to_last: pace_attribution.completed_games_delta,
                prior_expected_points_per_remaining_game: pace_attribution
                    .prior_expected_points_per_remaining_game,
                prior_expected_points_for_completed_interval: pace_attribution
                    .prior_expected_points_for_completed_interval,
                realized_points_vs_prior_remaining_pace: pace_attribution
                    .realized_points_vs_prior_remaining_pace,
                remaining_outlook_revaluation: pace_attribution.remaining_outlook_revaluation,
                pace_attribution_reconciliation_error: pace_attribution.reconciliation_error,
            }
        })
        .collect::<Vec<_>>();
    teams.sort_by(|a, b| a.team.cmp(&b.team));
    assign_history_movement_ranks(&mut teams);
    let biggest_risers = history_movers(&teams, true);
    let biggest_fallers = history_movers(&teams, false);

    Ok(TeamSeasonForecastHistoryView {
        schema: TEAM_SEASON_FORECAST_HISTORY_SCHEMA.to_owned(),
        season: forecasts[0].season,
        trials: forecasts[0].trials,
        seed: forecasts[0].seed,
        checkpoints,
        teams,
        biggest_risers,
        biggest_fallers,
        disclosures: vec![
            "History is chronological and every checkpoint is a sealed point-in-time replay using the same season, schedule, trials, seed, and teams.".to_owned(),
            "Checkpoint deltas are later minus the immediately preceding checkpoint; the first checkpoint has no prior delta.".to_owned(),
            "First-to-last movement and league riser/faller rankings are computed in core; observed standings and expected remaining points are core-owned checkpoint fields, not renderer calculations.".to_owned(),
            "Movement materiality is a descriptive heuristic, not a significance test: absolute net points movement is divided by the average first/last P10-P90 points width; below 10% is small, 10%-25% moderate, and 25% or more large.".to_owned(),
            "The first-to-last movement bridge reconciles projected-points change to confirmed standings points gained plus the change in expected remaining points.".to_owned(),
            "Pace-normalized attribution is descriptive, not causal: newly completed games are valued at the first checkpoint's average expected remaining points per game; realized performance versus that pace plus revaluation of the still-unplayed outlook reconciles to net movement.".to_owned(),
        ],
    })
}

fn history_movement_materiality(share: Option<f64>) -> TeamSeasonForecastHistoryMateriality {
    match share {
        None => TeamSeasonForecastHistoryMateriality::Indeterminate,
        Some(value) if value < 0.10 => TeamSeasonForecastHistoryMateriality::Small,
        Some(value) if value < 0.25 => TeamSeasonForecastHistoryMateriality::Moderate,
        Some(_) => TeamSeasonForecastHistoryMateriality::Large,
    }
}

fn history_trend(
    points: &[TeamSeasonForecastHistoryPointRow],
) -> (TeamSeasonForecastHistoryTrend, (NaiveDate, NaiveDate, f64)) {
    const STABLE_EPSILON: f64 = 0.05;
    let swings = points
        .windows(2)
        .map(|pair| {
            (
                pair[0].as_of_date,
                pair[1].as_of_date,
                pair[1].average_points - pair[0].average_points,
            )
        })
        .collect::<Vec<_>>();
    let positive = swings.iter().any(|row| row.2 > STABLE_EPSILON);
    let negative = swings.iter().any(|row| row.2 < -STABLE_EPSILON);
    let trend = match (positive, negative) {
        (true, false) => TeamSeasonForecastHistoryTrend::Improving,
        (false, true) => TeamSeasonForecastHistoryTrend::Declining,
        (true, true) => TeamSeasonForecastHistoryTrend::Mixed,
        (false, false) => TeamSeasonForecastHistoryTrend::Stable,
    };
    let largest = swings
        .into_iter()
        .max_by(|a, b| a.2.abs().total_cmp(&b.2.abs()))
        .expect("history has at least two checkpoints");
    (trend, largest)
}

fn assign_history_movement_ranks(teams: &mut [TeamSeasonForecastHistoryTeamRow]) {
    let mut ranked = teams
        .iter()
        .map(|team| (team.team.clone(), team.average_points_delta_first_to_last))
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let ranks = ranked
        .into_iter()
        .enumerate()
        .map(|(index, (team, _))| (team, index + 1))
        .collect::<BTreeMap<_, _>>();
    let team_count = teams.len();
    for team in teams {
        team.projected_points_movement_rank = ranks[&team.team];
        team.league_team_count = team_count;
    }
}

fn history_movers(
    teams: &[TeamSeasonForecastHistoryTeamRow],
    descending: bool,
) -> Vec<TeamSeasonForecastHistoryMoverRow> {
    let mut ranked = teams.iter().collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        let order = a
            .average_points_delta_first_to_last
            .total_cmp(&b.average_points_delta_first_to_last);
        let order = if descending { order.reverse() } else { order };
        order.then_with(|| a.team.cmp(&b.team))
    });
    ranked
        .into_iter()
        .take(5)
        .enumerate()
        .map(|(index, team)| TeamSeasonForecastHistoryMoverRow {
            rank: index + 1,
            team: team.team.clone(),
            average_points_delta_first_to_last: team.average_points_delta_first_to_last,
            playoff_probability_delta_first_to_last: team.playoff_probability_delta_first_to_last,
            stanley_cup_probability_delta_first_to_last: team
                .stanley_cup_probability_delta_first_to_last,
        })
        .collect()
}

fn validate_forecast_movement_input(
    label: &str,
    view: &TeamSeasonForecastView,
) -> Result<(), String> {
    let Some(checkpoint) = view.replay_checkpoint.as_ref() else {
        return Ok(());
    };
    if view.as_of_date != Some(checkpoint.as_of_date) {
        return Err(format!(
            "IceCast movement {label} checkpoint date {} does not match artifact cutoff {}",
            checkpoint.as_of_date,
            view.as_of_date
                .map(|date| date.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ));
    }
    let forecast_teams = view
        .teams
        .iter()
        .map(|row| row.team.as_str())
        .collect::<BTreeSet<_>>();
    let checkpoint_teams = checkpoint
        .teams
        .iter()
        .map(|row| row.team.as_str())
        .collect::<BTreeSet<_>>();
    if checkpoint_teams.len() != checkpoint.teams.len() || checkpoint_teams != forecast_teams {
        return Err(format!(
            "IceCast movement {label} checkpoint teams must match the forecast teams exactly"
        ));
    }
    Ok(())
}

fn forecast_movement_fingerprint(view: &TeamSeasonForecastView) -> Result<String, String> {
    let bytes = serde_json::to_vec(view)
        .map_err(|error| format!("serialize IceCast movement input: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_scenario(
    scenario: Option<&TeamSeasonScenario>,
    teams: &[String],
    forecast: &TeamGameForecastView,
) -> Result<(), String> {
    let Some(scenario) = scenario else {
        return Ok(());
    };
    if scenario.name.trim().is_empty() {
        return Err("IceCast scenario name cannot be empty".to_owned());
    }
    let known_teams = teams.iter().cloned().collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    let mut correlation_probabilities = BTreeMap::<String, f64>::new();
    for event in &scenario.events {
        if event.id.trim().is_empty() || !ids.insert(event.id.trim()) {
            return Err(format!(
                "IceCast scenario event IDs must be non-empty and unique: '{}'",
                event.id
            ));
        }
        if !known_teams.contains(&event.team.trim().to_ascii_uppercase()) {
            return Err(format!(
                "IceCast scenario event '{}' has unknown team {}",
                event.id, event.team
            ));
        }
        if event.effective_date < forecast.schedule_start
            || event.effective_date > forecast.schedule_end
        {
            return Err(format!(
                "IceCast scenario event '{}' is outside the schedule",
                event.id
            ));
        }
        if event.end_date.is_some_and(|end| end < event.effective_date) {
            return Err(format!(
                "IceCast scenario event '{}' ends before it starts",
                event.id
            ));
        }
        if !event.strength_delta.is_finite() || !(-50.0..=50.0).contains(&event.strength_delta) {
            return Err(format!(
                "IceCast scenario event '{}' strength_delta must be between -50 and 50",
                event.id
            ));
        }
        if !event.occurrence_probability.is_finite()
            || !(0.0..=1.0).contains(&event.occurrence_probability)
        {
            return Err(format!(
                "IceCast scenario event '{}' occurrence_probability must be between 0 and 1",
                event.id
            ));
        }
        if let Some(key) = &event.correlation_key {
            if key.trim().is_empty() {
                return Err(format!(
                    "IceCast scenario event '{}' correlation_key cannot be empty",
                    event.id
                ));
            }
            if let Some(existing) =
                correlation_probabilities.insert(key.clone(), event.occurrence_probability)
            {
                if (existing - event.occurrence_probability).abs() > f64::EPSILON {
                    return Err(format!(
                        "IceCast correlated events using '{}' must share occurrence_probability",
                        key
                    ));
                }
            }
        }
        if event.kind == TeamSeasonScenarioEventKind::Trade {
            let deadline = scenario.trade_deadline.ok_or_else(|| {
                format!(
                    "IceCast trade event '{}' requires scenario trade_deadline",
                    event.id
                )
            })?;
            if event.effective_date > deadline {
                return Err(format!(
                    "IceCast trade event '{}' occurs after trade deadline {}",
                    event.id, deadline
                ));
            }
        }
    }
    let mut policy_teams = BTreeSet::new();
    for policy in &scenario.adaptive_lineup_policies {
        let team = policy.team.trim().to_ascii_uppercase();
        if !known_teams.contains(&team) {
            return Err(format!(
                "IceCast adaptive lineup policy has unknown team {}",
                policy.team
            ));
        }
        if !policy_teams.insert(team.clone()) {
            return Err(format!(
                "IceCast supports one adaptive lineup policy per team: {team}"
            ));
        }
        if !(2..=20).contains(&policy.review_games) {
            return Err(format!(
                "IceCast adaptive lineup review_games for {team} must be between 2 and 20"
            ));
        }
        if !policy.minimum_points_percentage.is_finite()
            || !(0.0..=1.0).contains(&policy.minimum_points_percentage)
        {
            return Err(format!(
                "IceCast adaptive lineup minimum_points_percentage for {team} must be between 0 and 1"
            ));
        }
        if policy.choices.is_empty() || policy.choices.len() > 12 {
            return Err(format!(
                "IceCast adaptive lineup policy for {team} requires 1-12 choices"
            ));
        }
        if usize::from(policy.max_changes) > policy.choices.len().saturating_sub(1) {
            return Err(format!(
                "IceCast adaptive lineup max_changes for {team} exceeds available transitions"
            ));
        }
        let mut choice_ids = BTreeSet::new();
        for choice in &policy.choices {
            if choice.id.trim().is_empty() || !choice_ids.insert(choice.id.trim()) {
                return Err(format!(
                    "IceCast adaptive lineup choice IDs for {team} must be non-empty and unique"
                ));
            }
            if choice.label.trim().is_empty() {
                return Err(format!(
                    "IceCast adaptive lineup choice '{}' for {team} requires a label",
                    choice.id
                ));
            }
            if !choice.strength_delta.is_finite()
                || !(-20.0..=20.0).contains(&choice.strength_delta)
            {
                return Err(format!(
                    "IceCast adaptive lineup choice '{}' strength_delta must be between -20 and 20",
                    choice.id
                ));
            }
        }
    }
    let mut roster_policy_teams = BTreeSet::new();
    for policy in &scenario.opening_roster_policies {
        let team = policy.team.trim().to_ascii_uppercase();
        if !known_teams.contains(&team) {
            return Err(format!(
                "IceCast opening-roster policy has unknown team {}",
                policy.team
            ));
        }
        if !roster_policy_teams.insert(team.clone()) {
            return Err(format!(
                "IceCast supports one opening-roster policy per team: {team}"
            ));
        }
        if policy.choices.is_empty() || policy.choices.len() > 10_000 {
            return Err(format!(
                "IceCast opening-roster policy for {team} requires 1-10,000 choices"
            ));
        }
        let mut ids = BTreeSet::new();
        let mut probability_sum = 0.0;
        for choice in &policy.choices {
            if choice.id.trim().is_empty() || !ids.insert(choice.id.trim()) {
                return Err(format!(
                    "IceCast opening-roster choice IDs for {team} must be non-empty and unique"
                ));
            }
            if choice.label.trim().is_empty()
                || !choice.probability.is_finite()
                || choice.probability <= 0.0
                || choice.probability > 1.0
            {
                return Err(format!(
                    "IceCast opening-roster choice '{}' for {team} has an invalid label or probability",
                    choice.id
                ));
            }
            if !choice.strength_delta.is_finite()
                || !(-20.0..=20.0).contains(&choice.strength_delta)
            {
                return Err(format!(
                    "IceCast opening-roster choice '{}' strength_delta must be between -20 and 20",
                    choice.id
                ));
            }
            let mut roster_ids = BTreeSet::new();
            if choice
                .roster_ids
                .iter()
                .any(|id| *id == 0 || !roster_ids.insert(*id))
            {
                return Err(format!(
                    "IceCast opening-roster choice '{}' contains invalid or duplicate player IDs",
                    choice.id
                ));
            }
            probability_sum += choice.probability;
        }
        if (probability_sum - 1.0).abs() > 1e-6 {
            return Err(format!(
                "IceCast opening-roster probabilities for {team} must sum to 1.0, got {probability_sum:.8}"
            ));
        }
    }
    Ok(())
}

fn sample_scenario_events(scenario: Option<&TeamSeasonScenario>, seed: u64) -> BTreeSet<String> {
    scenario
        .into_iter()
        .flat_map(|scenario| &scenario.events)
        .filter(|event| {
            let occurrence_key = event.correlation_key.as_deref().unwrap_or(&event.id);
            // Avalanche related IDs before taking the first xorshift draw.
            // FNV hashes with a shared prefix retain enough nearby structure
            // that a raw XOR seed can create artificial event correlation.
            let event_seed = mix_seed(seed ^ stable_hash(occurrence_key) ^ 0xE7E1_7A11_5EED_0001);
            let mut rng = SimRng::new(event_seed);
            rng.chance(event.occurrence_probability)
        })
        .map(|event| event.id.clone())
        .collect()
}

fn sample_opening_rosters(
    scenario: Option<&TeamSeasonScenario>,
    cumulative_by_team: &BTreeMap<String, Vec<f64>>,
    seed: u64,
) -> BTreeMap<String, usize> {
    scenario
        .into_iter()
        .flat_map(|scenario| &scenario.opening_roster_policies)
        .map(|policy| {
            let team = policy.team.trim().to_ascii_uppercase();
            let mut rng = SimRng::new(mix_seed(seed ^ stable_hash(&team) ^ 0xC0A5_7E12_0A57_E001));
            let draw = rng.unit();
            let cumulative = &cumulative_by_team[&team];
            let choice = cumulative
                .partition_point(|probability| *probability <= draw)
                .min(policy.choices.len() - 1);
            (team, choice)
        })
        .collect()
}

fn build_opening_roster_cumulative(
    scenario: Option<&TeamSeasonScenario>,
) -> BTreeMap<String, Vec<f64>> {
    scenario
        .into_iter()
        .flat_map(|scenario| &scenario.opening_roster_policies)
        .map(|policy| {
            let mut running = 0.0;
            let cumulative = policy
                .choices
                .iter()
                .map(|choice| {
                    running += choice.probability;
                    running
                })
                .collect();
            (policy.team.trim().to_ascii_uppercase(), cumulative)
        })
        .collect()
}

fn opening_roster_policy<'a>(
    scenario: Option<&'a TeamSeasonScenario>,
    team: &str,
) -> Option<&'a TeamSeasonOpeningRosterPolicy> {
    scenario?
        .opening_roster_policies
        .iter()
        .find(|policy| policy.team.trim().eq_ignore_ascii_case(team))
}

fn opening_roster_strength_delta(
    scenario: Option<&TeamSeasonScenario>,
    selected: &BTreeMap<String, usize>,
    team: &str,
) -> f64 {
    let team = team.trim().to_ascii_uppercase();
    let Some(choice_index) = selected.get(&team) else {
        return 0.0;
    };
    opening_roster_policy(scenario, &team)
        .map(|policy| policy.choices[*choice_index].strength_delta)
        .unwrap_or(0.0)
}

fn active_strength_delta(
    scenario: Option<&TeamSeasonScenario>,
    active_events: &BTreeSet<String>,
    date: NaiveDate,
    home_team: &str,
    away_team: &str,
) -> (f64, f64) {
    let mut home = 0.0;
    let mut away = 0.0;
    for event in scenario.into_iter().flat_map(|scenario| &scenario.events) {
        if !active_events.contains(&event.id)
            || date < event.effective_date
            || event.end_date.is_some_and(|end| date > end)
        {
            continue;
        }
        if event.team.eq_ignore_ascii_case(home_team) {
            home += event.strength_delta;
        } else if event.team.eq_ignore_ascii_case(away_team) {
            away += event.strength_delta;
        }
    }
    (home, away)
}

fn adaptive_policy<'a>(
    scenario: Option<&'a TeamSeasonScenario>,
    team: &str,
) -> Option<&'a TeamSeasonAdaptiveLineupPolicy> {
    scenario?
        .adaptive_lineup_policies
        .iter()
        .find(|policy| policy.team.eq_ignore_ascii_case(team))
}

fn adaptive_strength_delta(
    scenario: Option<&TeamSeasonScenario>,
    states: &BTreeMap<String, TrialAdaptiveLineup>,
    team: &str,
) -> f64 {
    let Some(policy) = adaptive_policy(scenario, team) else {
        return 0.0;
    };
    let Some(state) = states.get(&team.to_ascii_uppercase()) else {
        return 0.0;
    };
    policy.choices[state.choice_index].strength_delta
}

fn record_adaptive_result(
    scenario: Option<&TeamSeasonScenario>,
    states: &mut BTreeMap<String, TrialAdaptiveLineup>,
    team: &str,
    standings_points: u16,
) {
    let Some(policy) = adaptive_policy(scenario, team) else {
        return;
    };
    let state = states
        .get_mut(&team.to_ascii_uppercase())
        .expect("adaptive policy state exists");
    state.games_by_choice[state.choice_index] += 1;
    state.games_in_window += 1;
    state.points_in_window += standings_points;
    if state.games_in_window < policy.review_games {
        return;
    }
    let points_percentage =
        f64::from(state.points_in_window) / (f64::from(state.games_in_window) * 2.0);
    if points_percentage < policy.minimum_points_percentage
        && state.changes < policy.max_changes
        && state.choice_index + 1 < policy.choices.len()
    {
        state.choice_index += 1;
        state.changes += 1;
    }
    state.games_in_window = 0;
    state.points_in_window = 0;
}

fn active_team_strength_delta(
    scenario: Option<&TeamSeasonScenario>,
    active_events: &BTreeSet<String>,
    date: NaiveDate,
    team: &str,
) -> f64 {
    scenario
        .into_iter()
        .flat_map(|scenario| &scenario.events)
        .filter(|event| {
            active_events.contains(&event.id)
                && event.team.eq_ignore_ascii_case(team)
                && date >= event.effective_date
                && event.end_date.is_none_or(|end| date <= end)
        })
        .map(|event| event.strength_delta)
        .sum()
}

fn stable_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01B3)
        })
}

fn mix_seed(value: u64) -> u64 {
    let mut mixed = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^ (mixed >> 31)
}

fn race_game_context(
    ranks: &BTreeMap<String, usize>,
    home_team: &str,
    away_team: &str,
) -> (bool, bool, f64) {
    let home_rank = ranks.get(home_team).copied().unwrap_or(16);
    let away_rank = ranks.get(away_team).copied().unwrap_or(16);
    let home_hunt = (7..=10).contains(&home_rank);
    let away_hunt = (7..=10).contains(&away_rank);
    let home_spoiler = home_rank >= 13;
    let away_spoiler = away_rank >= 13;
    let hunt_game = home_hunt || away_hunt;
    let spoiler_game = (home_hunt && away_spoiler) || (away_hunt && home_spoiler);
    let race_edge = match (home_hunt, away_hunt) {
        (true, false) => 0.004,
        (false, true) => -0.004,
        _ => 0.0,
    };
    (hunt_game, spoiler_game, race_edge)
}

fn conference_ranks(state: &BTreeMap<String, TrialTeam>) -> BTreeMap<String, usize> {
    let mut ranks = BTreeMap::new();
    for conference in ["Eastern", "Western"] {
        let mut teams = state
            .keys()
            .filter(|team| alignment(team).is_some_and(|value| value.0 == conference))
            .cloned()
            .collect::<Vec<_>>();
        teams.sort_by(|a, b| {
            let a_row = &state[a];
            let b_row = &state[b];
            let a_pct =
                f64::from(a_row.points()) / (f64::from(a_row.games_played()).max(1.0) * 2.0);
            let b_pct =
                f64::from(b_row.points()) / (f64::from(b_row.games_played()).max(1.0) * 2.0);
            b_pct
                .total_cmp(&a_pct)
                .then_with(|| b_row.points().cmp(&a_row.points()))
                .then_with(|| b_row.wins.cmp(&a_row.wins))
                .then_with(|| a.cmp(b))
        });
        ranks.extend(
            teams
                .into_iter()
                .enumerate()
                .map(|(index, team)| (team, index + 1)),
        );
    }
    ranks
}

fn recent_form_edge(state: &BTreeMap<String, TrialTeam>, home_team: &str, away_team: &str) -> f64 {
    let home = &state[home_team];
    let away = &state[away_team];
    if home.games_played() < 5 || away.games_played() < 5 {
        return 0.0;
    }
    ((home.recent_points_percentage() - away.recent_points_percentage()) * 0.015)
        .clamp(-0.015, 0.015)
}

fn sample_game(
    game: &TeamGameForecastRow,
    home_strength_delta: f64,
    away_strength_delta: f64,
    strength_edge_scale: f64,
    dynamic_edge: f64,
    state: &mut BTreeMap<String, TrialTeam>,
    rng: &mut SimRng,
) {
    let overtime = game.overtime_probability;
    let attribution_scale = 1.0 - overtime * 0.5;
    let baseline_edge = (game.home_overall_win_probability - 0.5) / attribution_scale;
    let scenario_edge = ((home_strength_delta - away_strength_delta) / 100.0) * strength_edge_scale;
    let edge = (baseline_edge + scenario_edge + dynamic_edge).clamp(-0.24, 0.24);
    let home_regulation = (1.0 - overtime) * (0.5 + edge);
    let away_regulation = (1.0 - overtime) * (0.5 - edge);
    let home_overtime = (0.5 + edge * 0.5).clamp(0.25, 0.75);
    let draw = rng.unit();
    let (home_win, overtime_loss) = if draw < home_regulation {
        (true, false)
    } else if draw < home_regulation + away_regulation {
        (false, false)
    } else {
        (rng.chance(home_overtime), true)
    };
    if home_win {
        state
            .get_mut(&game.home_team)
            .expect("home team exists")
            .record_win();
        state
            .get_mut(&game.away_team)
            .expect("away team exists")
            .record_loss(overtime_loss);
    } else {
        state
            .get_mut(&game.away_team)
            .expect("away team exists")
            .record_win();
        state
            .get_mut(&game.home_team)
            .expect("home team exists")
            .record_loss(overtime_loss);
    }
}

fn record_actual_game(game: &TeamGameForecastRow, state: &mut BTreeMap<String, TrialTeam>) {
    let winner = game
        .actual_winner
        .as_deref()
        .expect("as-of replay validates every fixed game result");
    let overtime = matches!(game.actual_ending.as_deref(), Some("OT" | "SO"));
    let home_won = winner == game.home_team;
    debug_assert!(home_won || winner == game.away_team);
    if home_won {
        state
            .get_mut(&game.home_team)
            .expect("home team exists")
            .record_win();
        state
            .get_mut(&game.away_team)
            .expect("away team exists")
            .record_loss(overtime);
    } else {
        state
            .get_mut(&game.away_team)
            .expect("away team exists")
            .record_win();
        state
            .get_mut(&game.home_team)
            .expect("home team exists")
            .record_loss(overtime);
    }
}

fn rank_teams(state: &BTreeMap<String, TrialTeam>) -> Vec<String> {
    let mut teams = state.keys().cloned().collect::<Vec<_>>();
    teams.sort_by(|a, b| standing_cmp(a, b, state));
    teams
}

fn select_playoff_teams(state: &BTreeMap<String, TrialTeam>) -> BTreeSet<String> {
    let mut selected = BTreeSet::new();
    for conference in ["Eastern", "Western"] {
        for division in if conference == "Eastern" {
            ["Atlantic", "Metropolitan"]
        } else {
            ["Central", "Pacific"]
        } {
            let mut division_teams = state
                .keys()
                .filter(|team| alignment(team).is_some_and(|value| value.1 == division))
                .cloned()
                .collect::<Vec<_>>();
            division_teams.sort_by(|a, b| standing_cmp(a, b, state));
            selected.extend(division_teams.into_iter().take(3));
        }
        let mut wild_cards = state
            .keys()
            .filter(|team| {
                alignment(team).is_some_and(|value| value.0 == conference)
                    && !selected.contains(*team)
            })
            .cloned()
            .collect::<Vec<_>>();
        wild_cards.sort_by(|a, b| standing_cmp(a, b, state));
        selected.extend(wild_cards.into_iter().take(2));
    }
    selected
}

fn simulate_playoffs(
    state: &BTreeMap<String, TrialTeam>,
    scenario: Option<&TeamSeasonScenario>,
    active_events: &BTreeSet<String>,
    playoff_date: NaiveDate,
    strengths: &BTreeMap<String, f64>,
    parameters: &TeamForecastParameters,
    rng: &mut SimRng,
) -> TrialPlayoffResult {
    let mut result = TrialPlayoffResult::default();
    let mut conference_champions = Vec::new();
    for (conference, divisions) in [
        ("Eastern", ["Atlantic", "Metropolitan"]),
        ("Western", ["Central", "Pacific"]),
    ] {
        let mut ranked_divisions = divisions
            .iter()
            .map(|division| {
                let mut teams = state
                    .keys()
                    .filter(|team| alignment(team).is_some_and(|value| value.1 == *division))
                    .cloned()
                    .collect::<Vec<_>>();
                teams.sort_by(|a, b| standing_cmp(a, b, state));
                ((*division).to_owned(), teams)
            })
            .collect::<BTreeMap<_, _>>();
        let mut division_winners = ranked_divisions
            .values()
            .map(|teams| teams[0].clone())
            .collect::<Vec<_>>();
        division_winners.sort_by(|a, b| standing_cmp(a, b, state));
        let mut wild_cards = state
            .keys()
            .filter(|team| {
                alignment(team).is_some_and(|value| value.0 == conference)
                    && !ranked_divisions
                        .values()
                        .any(|division| division[..3].contains(*team))
            })
            .cloned()
            .collect::<Vec<_>>();
        wild_cards.sort_by(|a, b| standing_cmp(a, b, state));
        let wildcard_one = wild_cards[0].clone();
        let wildcard_two = wild_cards[1].clone();
        let mut conference_finalists = Vec::new();
        for (index, division_winner) in division_winners.iter().enumerate() {
            let division = alignment(division_winner).expect("alignment validated").1;
            let division_teams = ranked_divisions.remove(division).expect("division exists");
            let wildcard = if index == 0 {
                &wildcard_two
            } else {
                &wildcard_one
            };
            let first_winner = simulate_series(
                division_winner,
                wildcard,
                state,
                scenario,
                active_events,
                playoff_date,
                strengths,
                parameters,
                rng,
            );
            let second_winner = simulate_series(
                &division_teams[1],
                &division_teams[2],
                state,
                scenario,
                active_events,
                playoff_date,
                strengths,
                parameters,
                rng,
            );
            result.second_round.insert(first_winner.clone());
            result.second_round.insert(second_winner.clone());
            let finalist = simulate_series(
                &first_winner,
                &second_winner,
                state,
                scenario,
                active_events,
                playoff_date,
                strengths,
                parameters,
                rng,
            );
            result.conference_final.insert(finalist.clone());
            conference_finalists.push(finalist);
        }
        let champion = simulate_series(
            &conference_finalists[0],
            &conference_finalists[1],
            state,
            scenario,
            active_events,
            playoff_date,
            strengths,
            parameters,
            rng,
        );
        result.stanley_cup_final.insert(champion.clone());
        conference_champions.push(champion);
    }
    result.champion = Some(simulate_series(
        &conference_champions[0],
        &conference_champions[1],
        state,
        scenario,
        active_events,
        playoff_date,
        strengths,
        parameters,
        rng,
    ));
    result
}

#[allow(clippy::too_many_arguments)]
fn simulate_series(
    first: &str,
    second: &str,
    state: &BTreeMap<String, TrialTeam>,
    scenario: Option<&TeamSeasonScenario>,
    active_events: &BTreeSet<String>,
    playoff_date: NaiveDate,
    strengths: &BTreeMap<String, f64>,
    parameters: &TeamForecastParameters,
    rng: &mut SimRng,
) -> String {
    let (higher, lower) = if standing_cmp(first, second, state).is_lt() {
        (first, second)
    } else {
        (second, first)
    };
    let home_pattern = [higher, higher, lower, lower, higher, lower, higher];
    let mut first_wins = 0;
    let mut second_wins = 0;
    for home in home_pattern {
        if first_wins == 4 || second_wins == 4 {
            break;
        }
        let away = if home == first { second } else { first };
        let home_strength = strengths.get(home).copied().unwrap_or(50.0)
            + active_team_strength_delta(scenario, active_events, playoff_date, home);
        let away_strength = strengths.get(away).copied().unwrap_or(50.0)
            + active_team_strength_delta(scenario, active_events, playoff_date, away);
        let edge = (parameters.home_edge
            + ((home_strength - away_strength) / 100.0) * parameters.strength_edge_scale)
            .clamp(-0.24, 0.24);
        let home_win_probability = 0.5 + edge * (1.0 - parameters.overtime_probability * 0.5);
        let winner = if rng.chance(home_win_probability) {
            home
        } else {
            away
        };
        if winner == first {
            first_wins += 1;
        } else {
            second_wins += 1;
        }
    }
    if first_wins == 4 {
        first.to_owned()
    } else {
        second.to_owned()
    }
}

fn standing_cmp(a: &str, b: &str, state: &BTreeMap<String, TrialTeam>) -> std::cmp::Ordering {
    let a_row = &state[a];
    let b_row = &state[b];
    b_row
        .points()
        .cmp(&a_row.points())
        .then_with(|| b_row.wins.cmp(&a_row.wins))
        .then_with(|| a.cmp(b))
}

fn percentile(values: &[u16], percentile: f64) -> u16 {
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[index]
}

fn trial_seed(seed: u64, trial: u32) -> u64 {
    seed ^ (u64::from(trial) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

struct SimRng(u64);

impl SimRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn unit(&mut self) -> f64 {
        let mut value = self.0;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.0 = value;
        value as f64 / u64::MAX as f64
    }

    fn chance(&mut self, probability: f64) -> bool {
        self.unit() < probability
    }
}

fn alignment(team: &str) -> Option<(&'static str, &'static str)> {
    match team {
        "BOS" | "BUF" | "DET" | "FLA" | "MTL" | "OTT" | "TBL" | "TOR" => {
            Some(("Eastern", "Atlantic"))
        }
        "CAR" | "CBJ" | "NJD" | "NYI" | "NYR" | "PHI" | "PIT" | "WSH" => {
            Some(("Eastern", "Metropolitan"))
        }
        "ARI" | "CHI" | "COL" | "DAL" | "MIN" | "NSH" | "STL" | "UTA" | "WPG" => {
            Some(("Western", "Central"))
        }
        "ANA" | "CGY" | "EDM" | "LAK" | "SEA" | "SJS" | "VAN" | "VGK" => {
            Some(("Western", "Pacific"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::model::Position;
    use crate::view_model::management_behavior::{
        BehaviorTraitView, GeneralManagerBehaviorProfile, ManagerBehaviorProfile,
        TEAM_DECISION_PROFILE_SCHEMA,
    };
    use crate::view_model::team_game_forecast::{
        build_team_game_forecast, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput,
    };
    use crate::view_model::team_lineup::{
        build_team_lineup_projection, LineupAssignmentEvidence, TeamLineupPlayerInput,
    };
    use crate::view_model::{EvidenceLabel, TeamCeilingLens};

    fn bench_trait(value: f64) -> BehaviorTraitView {
        BehaviorTraitView {
            value,
            evidence_games: 82,
            evidence_label: EvidenceLabel::Estimated,
        }
    }

    fn bench_profile() -> TeamDecisionProfile {
        TeamDecisionProfile {
            schema: TEAM_DECISION_PROFILE_SCHEMA.to_owned(),
            id: "nyr-test-bench".to_owned(),
            team: "NYR".to_owned(),
            season: 20262027,
            general_manager: GeneralManagerBehaviorProfile {
                rookie_opportunity: bench_trait(0.0),
                veteran_preference: bench_trait(0.0),
                waiver_asset_protection: bench_trait(0.0),
                trade_aggression: bench_trait(0.0),
                deadline_buying_bias: bench_trait(0.0),
            },
            manager: ManagerBehaviorProfile {
                matchup_intensity: bench_trait(0.4),
                tactical_adaptability: bench_trait(0.4),
                lineup_patience: bench_trait(0.0),
                position_flexibility: bench_trait(0.0),
                physical_fourth_line_preference: bench_trait(0.0),
                four_line_usage: bench_trait(0.2),
                fatigue_rotation: bench_trait(0.3),
            },
            disclosures: Vec::new(),
        }
    }

    fn bench_lineup() -> TeamLineupProjectionView {
        let player = |player_id, primary_position| TeamLineupPlayerInput {
            player_id,
            display_name: format!("Player {player_id}"),
            team: "NYR".to_owned(),
            prior_team: None,
            primary_position,
            eligible_positions: vec![primary_position],
            headshot_canonical_url: None,
            games_played: 82,
            lens_scores: BTreeMap::from([(TeamCeilingLens::PointsPace, Some(60.0))]),
            score_evidence: EvidenceLabel::Estimated,
            power_play_role_score: None,
            penalty_kill_role_score: None,
            special_teams_evidence: None,
            requested_slot: None,
            assignment_evidence: LineupAssignmentEvidence::Scenario,
        };
        let mut players = (0..12)
            .map(|index| {
                player(
                    100 + index,
                    [Position::LeftWing, Position::Center, Position::RightWing][index as usize % 3],
                )
            })
            .collect::<Vec<_>>();
        players.extend((0..6).map(|index| player(200 + index, Position::Defense)));
        players.extend((0..2).map(|index| player(300 + index, Position::Goalie)));
        build_team_lineup_projection("NYR", 20262027, players).unwrap()
    }

    fn bench_opponents(forecast: &TeamGameForecastView) -> Vec<TeamSeasonOpponentGamePlanInput> {
        forecast
            .games
            .iter()
            .filter_map(|game| {
                if game.home_team == "NYR" {
                    Some(game.away_team.clone())
                } else if game.away_team == "NYR" {
                    Some(game.home_team.clone())
                } else {
                    None
                }
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|opponent| TeamSeasonOpponentGamePlanInput {
                opponent,
                opponent_style: OpponentTacticalStyle::Balanced,
                opponent_primary_threat: None,
            })
            .collect()
    }

    fn fixture() -> TeamGameForecastView {
        let teams = [
            "BOS", "BUF", "DET", "FLA", "MTL", "OTT", "TBL", "TOR", "CAR", "CBJ", "NJD", "NYI",
            "NYR", "PHI", "PIT", "WSH", "CHI", "COL", "DAL", "MIN", "NSH", "STL", "UTA", "WPG",
            "ANA", "CGY", "EDM", "LAK", "SEA", "SJS", "VAN", "VGK",
        ];
        let games = (0..84)
            .flat_map(|round| {
                teams.chunks_exact(2).enumerate().map(move |(pair, chunk)| {
                    let reverse = round % 2 == 1;
                    TeamForecastGameInput {
                        game_id: (round * 16 + pair) as u64,
                        date: NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()
                            + chrono::Duration::days(round as i64),
                        away_team: chunk[usize::from(reverse)].to_owned(),
                        home_team: chunk[usize::from(!reverse)].to_owned(),
                        away_score: None,
                        home_score: None,
                        final_result: false,
                        last_period: None,
                    }
                })
            })
            .collect();
        let strengths = teams
            .iter()
            .map(|team| TeamForecastStrengthInput {
                team: (*team).to_owned(),
                strength: 50.0,
            })
            .collect();
        build_team_game_forecast(
            20262027,
            games,
            strengths,
            TeamForecastParameters::default(),
            Some(1_344),
            Some(84),
        )
        .unwrap()
    }

    #[test]
    fn simulation_is_seeded_and_records_reconcile() {
        let forecast = fixture();
        let config = TeamSeasonSimulationConfig {
            trials: 50,
            seed: 7,
        };
        let first = simulate_team_season_forecast(&forecast, config).unwrap();
        let second = simulate_team_season_forecast(&forecast, config).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.teams.len(), 32);
        for team in &first.teams {
            assert!(
                (team.average_wins + team.average_losses + team.average_overtime_losses - 84.0)
                    .abs()
                    < 1e-9
            );
            assert!((0.0..=1.0).contains(&team.playoff_probability));
        }
    }

    #[test]
    fn archived_season_rehydrates_to_the_same_seeded_team_results() {
        let forecast = fixture();
        let config = TeamSeasonSimulationConfig {
            trials: 25,
            seed: 20262027,
        };
        let archived = simulate_team_season_forecast(&forecast, config).unwrap();
        let rehydrated =
            rehydrate_team_game_forecast(&archived, TeamForecastParameters::default()).unwrap();
        let replayed = simulate_team_season_forecast(&rehydrated, config).unwrap();

        assert_eq!(rehydrated.games, forecast.games);
        assert_eq!(rehydrated.schedule_start, forecast.schedule_start);
        assert_eq!(rehydrated.schedule_end, forecast.schedule_end);
        assert_eq!(replayed.teams, archived.teams);
        assert!(rehydrated
            .warnings
            .iter()
            .any(|warning| warning.contains("parameters were supplied explicitly")));
    }

    #[test]
    fn bench_plan_becomes_an_exact_game_event_without_schedule_double_counting() {
        let forecast = fixture();
        let game = &forecast.games[0];
        let plan = BenchGamePlanView {
            schema: super::super::management_behavior::BENCH_GAME_PLAN_SCHEMA.to_owned(),
            team: game.home_team.clone(),
            opponent: game.away_team.clone(),
            opponent_style: super::super::management_behavior::OpponentTacticalStyle::Balanced,
            tactical_response: super::super::management_behavior::BenchTacticalResponse::Balanced,
            manager_profile_id: "test-manager".to_owned(),
            hard_match_confidence: 0.75,
            tactical_matchup_edge: 1.5,
            schedule_fatigue_edge: -3.0,
            forward_assignments: Vec::new(),
            defense_assignments: Vec::new(),
            warnings: Vec::new(),
            disclosures: Vec::new(),
        };
        let event =
            build_team_season_game_plan_event(&forecast, "bench-plan", game.date, &plan).unwrap();

        assert_eq!(event.effective_date, game.date);
        assert_eq!(event.end_date, Some(game.date));
        assert_eq!(event.strength_delta, 1.5);
        let scenario = TeamSeasonScenario {
            name: "The Bench".to_owned(),
            trade_deadline: None,
            events: vec![event],
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        };
        let active = BTreeSet::from(["bench-plan".to_owned()]);
        assert_eq!(
            active_strength_delta(
                Some(&scenario),
                &active,
                game.date,
                &game.home_team,
                &game.away_team,
            ),
            (1.5, 0.0)
        );
        assert_eq!(
            active_strength_delta(
                Some(&scenario),
                &active,
                game.date + Duration::days(1),
                &game.home_team,
                &game.away_team,
            ),
            (0.0, 0.0)
        );
    }

    #[test]
    fn bench_plan_must_match_the_forecast_opponent_and_date() {
        let forecast = fixture();
        let game = &forecast.games[0];
        let plan = BenchGamePlanView {
            schema: super::super::management_behavior::BENCH_GAME_PLAN_SCHEMA.to_owned(),
            team: game.home_team.clone(),
            opponent: "NOT".to_owned(),
            opponent_style: super::super::management_behavior::OpponentTacticalStyle::Balanced,
            tactical_response: super::super::management_behavior::BenchTacticalResponse::Balanced,
            manager_profile_id: "test-manager".to_owned(),
            hard_match_confidence: 0.75,
            tactical_matchup_edge: 1.0,
            schedule_fatigue_edge: 0.0,
            forward_assignments: Vec::new(),
            defense_assignments: Vec::new(),
            warnings: Vec::new(),
            disclosures: Vec::new(),
        };

        let error = build_team_season_game_plan_event(&forecast, "bench-plan", game.date, &plan)
            .unwrap_err();
        assert!(error.contains("does not match the schedule"));
    }

    #[test]
    fn bench_schedule_authors_every_team_game_and_a_simulation_ready_scenario() {
        let forecast = fixture();
        let view = build_team_season_game_plan_schedule(
            &forecast,
            &bench_lineup(),
            &bench_profile(),
            &[],
            &bench_opponents(&forecast),
        )
        .unwrap();

        assert_eq!(view.team, "NYR");
        assert_eq!(view.games.len(), 84);
        assert_eq!(view.scenario.events.len(), view.games.len());
        for row in &view.games {
            assert_eq!(row.event.effective_date, row.date);
            assert_eq!(row.event.end_date, Some(row.date));
            assert_eq!(row.event.strength_delta, row.plan.tactical_matchup_edge);
            assert_eq!(row.plan.team, "NYR");
            assert_eq!(row.plan.opponent, row.opponent);
        }
    }

    #[test]
    fn bench_schedule_refuses_to_invent_a_missing_opponent_style() {
        let forecast = fixture();
        let mut opponents = bench_opponents(&forecast);
        let missing = opponents.pop().unwrap().opponent;
        let error = build_team_season_game_plan_schedule(
            &forecast,
            &bench_lineup(),
            &bench_profile(),
            &[],
            &opponents,
        )
        .unwrap_err();

        assert!(error.contains("missing opponent style input"));
        assert!(error.contains(&missing));
    }

    #[test]
    fn bench_schedule_consumes_sealed_role_and_style_evidence() {
        let forecast = fixture();
        let role_evidence = TeamPlayerMatchupRoleEvidenceView {
            schema: TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA.to_owned(),
            team: "NYR".to_owned(),
            season: 20252026,
            season_type: crate::season_stats::SeasonType::Regular,
            roster_skaters: 18,
            rated_skaters: 0,
            league_forward_peers: 300,
            league_defense_peers: 190,
            roles: Vec::new(),
            warnings: Vec::new(),
            disclosures: Vec::new(),
        };
        let styles = bench_opponents(&forecast)
            .into_iter()
            .map(|input| OpponentStyleEvidenceRow {
                schema: OPPONENT_STYLE_EVIDENCE_SCHEMA.to_owned(),
                team: input.opponent,
                season: 20252026,
                style: Some(input.opponent_style),
                confidence: 0.75,
                evidence_games: 82,
                event_coverage: 1.0,
                scores: Vec::new(),
                warnings: Vec::new(),
                disclosures: Vec::new(),
            })
            .collect::<Vec<_>>();

        let view = build_team_season_game_plan_schedule_from_evidence(
            &forecast,
            &bench_lineup(),
            &bench_profile(),
            &role_evidence,
            &styles,
        )
        .unwrap();
        assert_eq!(view.games.len(), 84);
        assert!(view
            .disclosures
            .iter()
            .any(|row| row.contains("source season(s) 20252026")));
    }

    #[test]
    fn bench_schedule_stops_when_a_scheduled_style_is_no_read() {
        let forecast = fixture();
        let role_evidence = TeamPlayerMatchupRoleEvidenceView {
            schema: TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA.to_owned(),
            team: "NYR".to_owned(),
            season: 20252026,
            season_type: crate::season_stats::SeasonType::Regular,
            roster_skaters: 18,
            rated_skaters: 0,
            league_forward_peers: 300,
            league_defense_peers: 190,
            roles: Vec::new(),
            warnings: Vec::new(),
            disclosures: Vec::new(),
        };
        let mut styles = bench_opponents(&forecast)
            .into_iter()
            .map(|input| OpponentStyleEvidenceRow {
                schema: OPPONENT_STYLE_EVIDENCE_SCHEMA.to_owned(),
                team: input.opponent,
                season: 20252026,
                style: Some(input.opponent_style),
                confidence: 0.75,
                evidence_games: 82,
                event_coverage: 1.0,
                scores: Vec::new(),
                warnings: Vec::new(),
                disclosures: Vec::new(),
            })
            .collect::<Vec<_>>();
        styles[0].style = None;
        let missing = styles[0].team.clone();

        let error = build_team_season_game_plan_schedule_from_evidence(
            &forecast,
            &bench_lineup(),
            &bench_profile(),
            &role_evidence,
            &styles,
        )
        .unwrap_err();
        assert!(error.contains("without a style read"));
        assert!(error.contains(&missing));
    }

    #[test]
    fn bench_schedule_load_preserves_icecast_congestion_and_travel() {
        let context = super::super::team_game_forecast::TeamGameScheduleContext {
            rest_days: Some(0),
            back_to_back: true,
            three_in_four: true,
            four_in_six: true,
            road_trip_index: 3,
            home_stand_index: 0,
            travel_km: 1_875.0,
            timezone_displacement_hours: 2,
            post_all_star_break: false,
        };
        let load = schedule_load(false, &context);

        assert!(!load.is_home);
        assert!(load.back_to_back);
        assert!(load.third_game_in_four_nights);
        assert_eq!(load.travel_km, 1_875.0);
    }

    fn adaptive_scenario() -> TeamSeasonScenario {
        TeamSeasonScenario {
            name: "The Bench — Rangers line review".to_owned(),
            trade_deadline: None,
            events: Vec::new(),
            adaptive_lineup_policies: vec![TeamSeasonAdaptiveLineupPolicy {
                team: "NYR".to_owned(),
                review_games: 4,
                minimum_points_percentage: 0.5,
                max_changes: 1,
                choices: vec![
                    TeamSeasonAdaptiveLineupChoice {
                        id: "opening".to_owned(),
                        label: "Opening-night lines".to_owned(),
                        strength_delta: 0.0,
                    },
                    TeamSeasonAdaptiveLineupChoice {
                        id: "balanced".to_owned(),
                        label: "Balanced scoring lines".to_owned(),
                        strength_delta: 2.0,
                    },
                ],
            }],
            opening_roster_policies: Vec::new(),
        }
    }

    #[test]
    fn opening_roster_is_sampled_once_per_trial_and_reconciles() {
        let scenario = TeamSeasonScenario {
            name: "The Cut — Rangers camp".to_owned(),
            trade_deadline: None,
            events: Vec::new(),
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: vec![TeamSeasonOpeningRosterPolicy {
                team: "NYR".to_owned(),
                choices: vec![
                    TeamSeasonOpeningRosterChoice {
                        id: "beaudoin-makes-it".to_owned(),
                        label: "Beaudoin breaks camp".to_owned(),
                        probability: 0.25,
                        strength_delta: 2.0,
                        roster_ids: vec![8484786],
                    },
                    TeamSeasonOpeningRosterChoice {
                        id: "other-roster".to_owned(),
                        label: "Other camp outcome".to_owned(),
                        probability: 0.75,
                        strength_delta: 0.0,
                        roster_ids: Vec::new(),
                    },
                ],
            }],
        };
        let config = TeamSeasonSimulationConfig {
            trials: 1_000,
            seed: 27,
        };
        let first =
            simulate_team_season_forecast_with_scenario(&fixture(), config, Some(scenario.clone()))
                .unwrap();
        let second =
            simulate_team_season_forecast_with_scenario(&fixture(), config, Some(scenario))
                .unwrap();
        assert_eq!(first, second);
        let summary = &first.opening_roster_summaries[0];
        assert_eq!(summary.team, "NYR");
        assert!((summary.choices[0].sampled_probability - 0.25).abs() < 0.05);
        assert!(
            (summary
                .choices
                .iter()
                .map(|choice| choice.sampled_probability)
                .sum::<f64>()
                - 1.0)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn adaptive_lineup_changes_after_a_bad_review_window_and_respects_limit() {
        let scenario = adaptive_scenario();
        let mut states = BTreeMap::from([(
            "NYR".to_owned(),
            TrialAdaptiveLineup {
                games_by_choice: vec![0; 2],
                ..TrialAdaptiveLineup::default()
            },
        )]);
        for _ in 0..4 {
            record_adaptive_result(Some(&scenario), &mut states, "NYR", 0);
        }
        assert_eq!(states["NYR"].choice_index, 1);
        assert_eq!(states["NYR"].changes, 1);
        assert_eq!(states["NYR"].games_by_choice, vec![4, 0]);

        for _ in 0..4 {
            record_adaptive_result(Some(&scenario), &mut states, "NYR", 0);
        }
        assert_eq!(states["NYR"].choice_index, 1);
        assert_eq!(states["NYR"].changes, 1);
        assert_eq!(states["NYR"].games_by_choice, vec![4, 4]);
    }

    #[test]
    fn adaptive_lineup_summary_is_seeded_and_usage_reconciles() {
        let config = TeamSeasonSimulationConfig {
            trials: 20,
            seed: 73,
        };
        let first = simulate_team_season_forecast_with_scenario(
            &fixture(),
            config,
            Some(adaptive_scenario()),
        )
        .unwrap();
        let second = simulate_team_season_forecast_with_scenario(
            &fixture(),
            config,
            Some(adaptive_scenario()),
        )
        .unwrap();
        assert_eq!(first, second);
        let summary = &first.adaptive_lineup_summaries[0];
        assert_eq!(summary.team, "NYR");
        assert_eq!(summary.choices.len(), 2);
        assert!(summary.switch_probability > 0.0);
        assert!(
            (summary
                .choices
                .iter()
                .map(|choice| choice.average_games)
                .sum::<f64>()
                - 84.0)
                .abs()
                < 1e-9
        );
        assert!(
            (summary
                .choices
                .iter()
                .map(|choice| choice.finish_probability)
                .sum::<f64>()
                - 1.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn adaptive_lineup_policy_validation_rejects_too_short_review() {
        let mut scenario = adaptive_scenario();
        scenario.adaptive_lineup_policies[0].review_games = 1;
        let error = simulate_team_season_forecast_with_scenario(
            &fixture(),
            TeamSeasonSimulationConfig { trials: 1, seed: 1 },
            Some(scenario),
        )
        .unwrap_err();
        assert!(error.contains("review_games"));
    }

    #[test]
    fn legacy_scenario_json_omits_and_defaults_adaptive_policies() {
        let scenario: TeamSeasonScenario =
            serde_json::from_str(r#"{"name":"legacy","events":[]}"#).unwrap();
        assert!(scenario.adaptive_lineup_policies.is_empty());
        let value = serde_json::to_value(&scenario).unwrap();
        assert!(value.get("adaptive_lineup_policies").is_none());
    }

    #[test]
    fn point_in_time_replay_fixes_known_results_and_simulates_only_the_remainder() {
        let mut forecast = fixture();
        let cutoff = forecast.schedule_start;
        for game in forecast.games.iter_mut().filter(|game| game.date == cutoff) {
            game.actual_away_score = Some(1);
            game.actual_home_score = Some(3);
            game.actual_winner = Some(game.home_team.clone());
            game.actual_ending = None;
            game.pick_correct = Some(true);
        }
        let known_home_team = forecast.games[0].home_team.clone();
        let view = simulate_team_season_forecast_as_of_with_scenario(
            &forecast,
            TeamSeasonSimulationConfig {
                trials: 50,
                seed: 29,
            },
            None,
            cutoff,
        )
        .unwrap();

        assert_eq!(view.as_of_date, Some(cutoff));
        let checkpoint = view.replay_checkpoint.as_ref().unwrap();
        assert_eq!(checkpoint.as_of_date, cutoff);
        assert_eq!(checkpoint.league_completed_games, 16);
        assert_eq!(checkpoint.league_remaining_games, 1_328);
        let known_checkpoint = checkpoint
            .teams
            .iter()
            .find(|team| team.team == known_home_team)
            .unwrap();
        assert_eq!(known_checkpoint.completed_games, 1);
        assert_eq!(known_checkpoint.remaining_games, 83);
        assert_eq!(known_checkpoint.wins, 1);
        assert_eq!(known_checkpoint.standings_points, 2);
        assert!(view
            .disclosures
            .iter()
            .any(|value| value.contains("only the remaining schedule is sampled")));
        let known = view
            .teams
            .iter()
            .find(|team| team.team == known_home_team)
            .unwrap();
        assert!(
            (known_checkpoint.expected_remaining_wins
                + known_checkpoint.expected_remaining_losses
                + known_checkpoint.expected_remaining_overtime_losses
                - known_checkpoint.remaining_games as f64)
                .abs()
                < 1e-9
        );
        assert!(
            (known_checkpoint.expected_remaining_points
                - (known.average_points - f64::from(known_checkpoint.standings_points)))
            .abs()
                < 1e-9
        );
        assert!(known.average_wins >= 1.0);
        assert!(
            (known.average_wins + known.average_losses + known.average_overtime_losses - 84.0)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn point_in_time_replay_rejects_future_result_labels() {
        let mut forecast = fixture();
        let cutoff = forecast.schedule_start;
        for game in forecast.games.iter_mut().filter(|game| game.date == cutoff) {
            game.actual_winner = Some(game.home_team.clone());
        }
        let future = forecast
            .games
            .iter_mut()
            .find(|game| game.date > cutoff)
            .unwrap();
        future.actual_winner = Some(future.away_team.clone());

        let error = simulate_team_season_forecast_as_of_with_scenario(
            &forecast,
            TeamSeasonSimulationConfig { trials: 1, seed: 1 },
            None,
            cutoff,
        )
        .unwrap_err();
        assert!(error.contains("detected a future result"));
    }

    #[test]
    fn pre_utah_arizona_alignment_remains_replayable() {
        assert_eq!(alignment("ARI"), Some(("Western", "Central")));
        assert_eq!(alignment("UTA"), Some(("Western", "Central")));
    }

    #[test]
    fn temporary_and_legacy_alignments_are_refused_instead_of_simulated_as_current() {
        let mut forecast = fixture();
        forecast.season = 20202021;
        let error = simulate_team_season_forecast(
            &forecast,
            TeamSeasonSimulationConfig { trials: 1, seed: 1 },
        )
        .unwrap_err();
        assert!(error.contains("historical division and playoff-rule authority"));
    }

    #[test]
    fn exactly_sixteen_teams_make_playoffs_per_trial() {
        let view = simulate_team_season_forecast(
            &fixture(),
            TeamSeasonSimulationConfig {
                trials: 100,
                seed: 11,
            },
        )
        .unwrap();
        let playoff_sum: f64 = view.teams.iter().map(|team| team.playoff_probability).sum();
        assert!((playoff_sum - 16.0).abs() < 1e-9);
        let second_round_sum: f64 = view
            .teams
            .iter()
            .map(|team| team.second_round_probability)
            .sum();
        let conference_final_sum: f64 = view
            .teams
            .iter()
            .map(|team| team.conference_final_probability)
            .sum();
        let final_sum: f64 = view
            .teams
            .iter()
            .map(|team| team.stanley_cup_final_probability)
            .sum();
        let cup_sum: f64 = view
            .teams
            .iter()
            .map(|team| team.stanley_cup_probability)
            .sum();
        assert!((second_round_sum - 8.0).abs() < 1e-9);
        assert!((conference_final_sum - 4.0).abs() < 1e-9);
        assert!((final_sum - 2.0).abs() < 1e-9);
        assert!((cup_sum - 1.0).abs() < 1e-9);
        assert!(!view.pivotal_games.is_empty());
        let race_start =
            view.games.iter().map(|game| game.date).max().unwrap() - Duration::days(45);
        assert!(view.pivotal_games.iter().all(|game| {
            game.date >= race_start
                && (0.0..=1.0).contains(&game.hunt_probability)
                && (0.0..=1.0).contains(&game.spoiler_probability)
        }));
        assert_eq!(view.league_leaders.presidents_trophy.len(), 5);
        assert_eq!(view.league_leaders.stanley_cup.len(), 5);
        assert_eq!(view.league_leaders.longest_win_streak.len(), 5);
        assert!(view
            .league_leaders
            .stanley_cup
            .windows(2)
            .all(|pair| pair[0].probability >= pair[1].probability));
        assert_eq!(view.schedule_stretches.len(), 64);
        for team in view.teams.iter().map(|team| &team.team) {
            let hardest = view
                .schedule_stretches
                .iter()
                .find(|row| row.team == *team && row.kind == TeamSeasonStretchKind::Hardest)
                .unwrap();
            let easiest = view
                .schedule_stretches
                .iter()
                .find(|row| row.team == *team && row.kind == TeamSeasonStretchKind::Easiest)
                .unwrap();
            assert_eq!(hardest.opponents.len(), 5);
            assert!(hardest.average_win_probability <= easiest.average_win_probability);
            assert!((hardest.expected_wins / 5.0 - hardest.average_win_probability).abs() < 1e-12);
        }
    }

    #[test]
    fn scenario_event_is_active_only_inside_its_date_window() {
        let scenario = TeamSeasonScenario {
            name: "goalie window".to_owned(),
            trade_deadline: None,
            events: vec![TeamSeasonScenarioEvent {
                id: "nyr-goalie".to_owned(),
                kind: TeamSeasonScenarioEventKind::Goalie,
                team: "NYR".to_owned(),
                player: Some("Test Goalie".to_owned()),
                effective_date: NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
                end_date: Some(NaiveDate::from_ymd_opt(2026, 10, 12).unwrap()),
                strength_delta: -8.0,
                occurrence_probability: 1.0,
                correlation_key: None,
                label: "backup expected".to_owned(),
            }],
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        };
        let active = BTreeSet::from(["nyr-goalie".to_owned()]);
        assert_eq!(
            active_strength_delta(
                Some(&scenario),
                &active,
                NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(),
                "NYR",
                "BOS"
            ),
            (0.0, 0.0)
        );
        assert_eq!(
            active_strength_delta(
                Some(&scenario),
                &active,
                NaiveDate::from_ymd_opt(2026, 10, 11).unwrap(),
                "NYR",
                "BOS"
            ),
            (-8.0, 0.0)
        );
    }

    #[test]
    fn scenario_outcomes_report_positive_and_negative_event_count_buckets() {
        let start = NaiveDate::from_ymd_opt(2026, 9, 29).unwrap();
        let scenario = TeamSeasonScenario {
            name: "development variance".to_owned(),
            trade_deadline: None,
            events: vec![
                TeamSeasonScenarioEvent {
                    id: "nyr-breakout".to_owned(),
                    kind: TeamSeasonScenarioEventKind::Form,
                    team: "NYR".to_owned(),
                    player: Some("Young Player".to_owned()),
                    effective_date: start,
                    end_date: None,
                    strength_delta: 3.0,
                    occurrence_probability: 1.0,
                    correlation_key: None,
                    label: "breakout".to_owned(),
                },
                TeamSeasonScenarioEvent {
                    id: "nyr-downturn".to_owned(),
                    kind: TeamSeasonScenarioEventKind::Form,
                    team: "NYR".to_owned(),
                    player: Some("Veteran Player".to_owned()),
                    effective_date: start,
                    end_date: None,
                    strength_delta: -2.0,
                    occurrence_probability: 0.0,
                    correlation_key: None,
                    label: "downturn".to_owned(),
                },
            ],
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        };
        let view = simulate_team_season_forecast_with_scenario(
            &fixture(),
            TeamSeasonSimulationConfig {
                trials: 20,
                seed: 29,
            },
            Some(scenario),
        )
        .unwrap();
        assert!(view.scenario_fingerprint.is_some());
        assert!(view.scenario_reference.is_none());
        assert_eq!(view.scenario_outcomes.len(), 1);
        let outcome = &view.scenario_outcomes[0];
        assert_eq!(outcome.team, "NYR");
        assert_eq!(outcome.positive_events, 1);
        assert_eq!(outcome.negative_events, 0);
        assert_eq!(outcome.trials, 20);
        assert_eq!(outcome.probability, 1.0);
        assert_eq!(outcome.average_sampled_strength_delta, 3.0);
        assert!((0.0..=1.0).contains(&outcome.playoff_probability));
        assert!((0.0..=1.0).contains(&outcome.stanley_cup_probability));
    }

    #[test]
    fn trade_after_deadline_is_rejected() {
        let scenario = TeamSeasonScenario {
            name: "late trade".to_owned(),
            trade_deadline: Some(NaiveDate::from_ymd_opt(2026, 10, 10).unwrap()),
            events: vec![TeamSeasonScenarioEvent {
                id: "nyr-late-trade".to_owned(),
                kind: TeamSeasonScenarioEventKind::Trade,
                team: "NYR".to_owned(),
                player: None,
                effective_date: NaiveDate::from_ymd_opt(2026, 10, 11).unwrap(),
                end_date: None,
                strength_delta: 5.0,
                occurrence_probability: 1.0,
                correlation_key: None,
                label: "deadline acquisition".to_owned(),
            }],
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        };
        let error = simulate_team_season_forecast_with_scenario(
            &fixture(),
            TeamSeasonSimulationConfig { trials: 1, seed: 7 },
            Some(scenario),
        )
        .unwrap_err();
        assert!(error.contains("after trade deadline 2026-10-10"));
    }

    #[test]
    fn automatic_personnel_events_are_seeded_bounded_and_player_aware() {
        let players = vec![
            TeamSeasonPersonnelInput {
                team: "NYR".to_owned(),
                player: "Top Skater".to_owned(),
                position: "C".to_owned(),
                is_goalie: false,
                age: 31,
                games_played: 72,
                rating: 92.0,
            },
            TeamSeasonPersonnelInput {
                team: "NYR".to_owned(),
                player: "Starting Goalie".to_owned(),
                position: "G".to_owned(),
                is_goalie: true,
                age: 30,
                games_played: 58,
                rating: 88.0,
            },
        ];
        let start = NaiveDate::from_ymd_opt(2026, 9, 29).unwrap();
        let end = NaiveDate::from_ymd_opt(2027, 4, 10).unwrap();
        let first = build_team_season_auto_personnel_scenario(
            start,
            end,
            17,
            players.clone(),
            TeamSeasonAutoPersonnelConfig::default(),
            Some(NaiveDate::from_ymd_opt(2027, 3, 5).unwrap()),
        )
        .unwrap();
        let second = build_team_season_auto_personnel_scenario(
            start,
            end,
            17,
            players,
            TeamSeasonAutoPersonnelConfig::default(),
            Some(NaiveDate::from_ymd_opt(2027, 3, 5).unwrap()),
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.events.len(), 2);
        assert!(first.events.iter().all(|event| {
            event.player.is_some()
                && event.effective_date >= start
                && event.end_date.is_some_and(|date| date <= end)
                && event.strength_delta < 0.0
                && (0.0..=1.0).contains(&event.occurrence_probability)
        }));
        assert!(first
            .events
            .iter()
            .any(|event| event.kind == TeamSeasonScenarioEventKind::Goalie));
    }

    #[test]
    fn plausible_trade_is_named_need_based_and_atomic() {
        let teams = vec![
            TeamSeasonTradeTeamInput {
                team: "NYR".to_owned(),
                expected_points: 110.0,
            },
            TeamSeasonTradeTeamInput {
                team: "SEA".to_owned(),
                expected_points: 102.0,
            },
            TeamSeasonTradeTeamInput {
                team: "BOS".to_owned(),
                expected_points: 76.0,
            },
            TeamSeasonTradeTeamInput {
                team: "VAN".to_owned(),
                expected_points: 68.0,
            },
        ];
        let players = vec![
            TeamSeasonPersonnelInput {
                team: "NYR".to_owned(),
                player: "Buyer Forward".to_owned(),
                position: "C".to_owned(),
                is_goalie: false,
                age: 27,
                games_played: 82,
                rating: 80.0,
            },
            TeamSeasonPersonnelInput {
                team: "VAN".to_owned(),
                player: "Young Core Defender".to_owned(),
                position: "D".to_owned(),
                is_goalie: false,
                age: 23,
                games_played: 82,
                rating: 96.0,
            },
            TeamSeasonPersonnelInput {
                team: "VAN".to_owned(),
                player: "Available Defender".to_owned(),
                position: "D".to_owned(),
                is_goalie: false,
                age: 29,
                games_played: 75,
                rating: 74.0,
            },
        ];
        let deadline = NaiveDate::from_ymd_opt(2027, 3, 5).unwrap();
        let scenario = build_team_season_plausible_trade_scenario(
            deadline,
            teams,
            players,
            TeamSeasonPlausibleTradeConfig {
                max_trades: 1,
                occurrence_probability: 0.35,
            },
        )
        .unwrap();
        assert_eq!(scenario.events.len(), 2);
        let buyer = scenario
            .events
            .iter()
            .find(|event| event.strength_delta > 0.0)
            .unwrap();
        let seller = scenario
            .events
            .iter()
            .find(|event| event.strength_delta < 0.0)
            .unwrap();
        assert_eq!(buyer.team, "NYR");
        assert_eq!(seller.team, "VAN");
        assert_eq!(buyer.player, Some("Available Defender".to_owned()));
        assert_eq!(buyer.correlation_key, seller.correlation_key);
        assert_eq!(buyer.effective_date, deadline);
        for seed in 1..100 {
            let active = sample_scenario_events(Some(&scenario), seed);
            assert_eq!(active.contains(&buyer.id), active.contains(&seller.id));
        }
    }

    #[test]
    fn related_event_ids_still_sample_independently() {
        let scenario = TeamSeasonScenario {
            name: "related event IDs".to_owned(),
            trade_deadline: None,
            events: (0..5)
                .map(|index| TeamSeasonScenarioEvent {
                    id: format!("nyr-prospect-breakout-{index}"),
                    kind: TeamSeasonScenarioEventKind::Form,
                    team: "NYR".to_owned(),
                    player: Some(format!("Prospect {index}")),
                    effective_date: NaiveDate::from_ymd_opt(2026, 9, 29).unwrap(),
                    end_date: None,
                    strength_delta: 2.0,
                    occurrence_probability: 0.5,
                    correlation_key: None,
                    label: "breakout".to_owned(),
                })
                .collect(),
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        };
        let mut observed_counts = BTreeSet::new();
        let mut event_hits = [0_u32; 5];
        let trials = 2_048_u32;
        for seed in 1..=u64::from(trials) {
            let active = sample_scenario_events(Some(&scenario), seed);
            observed_counts.insert(active.len());
            for (index, event) in scenario.events.iter().enumerate() {
                event_hits[index] += u32::from(active.contains(&event.id));
            }
        }
        assert_eq!(observed_counts, BTreeSet::from([0, 1, 2, 3, 4, 5]));
        for hits in event_hits {
            let rate = f64::from(hits) / f64::from(trials);
            assert!((0.45..=0.55).contains(&rate), "event rate was {rate}");
        }
    }

    #[test]
    fn paired_scenario_comparison_reports_deltas_and_rejects_seed_mismatch() {
        let baseline = simulate_team_season_forecast(
            &fixture(),
            TeamSeasonSimulationConfig {
                trials: 10,
                seed: 19,
            },
        )
        .unwrap();
        let mut scenario = baseline.clone();
        let nyr = scenario
            .teams
            .iter_mut()
            .find(|team| team.team == "NYR")
            .unwrap();
        nyr.average_points += 1.25;
        nyr.playoff_probability += 0.04;
        let impacts = compare_team_season_forecast_scenarios(&baseline, &scenario).unwrap();
        let impact = impacts.iter().find(|team| team.team == "NYR").unwrap();
        assert!((impact.average_points_delta - 1.25).abs() < 1e-12);
        assert!((impact.playoff_probability_delta - 0.04).abs() < 1e-12);

        scenario.seed += 1;
        let error = compare_team_season_forecast_scenarios(&baseline, &scenario).unwrap_err();
        assert!(error.contains("same season, schedule, trials, and seed"));
    }

    #[test]
    fn forecast_movement_compares_sealed_checkpoint_runs() {
        let mut forecast = fixture();
        let first_date = forecast.schedule_start;
        for game in forecast
            .games
            .iter_mut()
            .filter(|game| game.date == first_date)
        {
            game.actual_winner = Some(game.home_team.clone());
        }
        let config = TeamSeasonSimulationConfig {
            trials: 20,
            seed: 31,
        };
        let earlier =
            simulate_team_season_forecast_as_of_with_scenario(&forecast, config, None, first_date)
                .unwrap();
        let second_date = first_date + Duration::days(1);
        for game in forecast
            .games
            .iter_mut()
            .filter(|game| game.date == second_date)
        {
            game.actual_winner = Some(game.home_team.clone());
        }
        let later =
            simulate_team_season_forecast_as_of_with_scenario(&forecast, config, None, second_date)
                .unwrap();

        let movement = build_team_season_forecast_movement(&earlier, &later).unwrap();
        assert_eq!(movement.schema, TEAM_SEASON_FORECAST_MOVEMENT_SCHEMA);
        assert_eq!(movement.earlier_as_of_date, Some(first_date));
        assert_eq!(movement.later_as_of_date, Some(second_date));
        assert_ne!(movement.earlier_fingerprint, movement.later_fingerprint);
        assert_eq!(movement.teams.len(), 32);
        assert!(movement.teams.iter().all(|row| {
            row.completed_games_delta == Some(1)
                && row.observed_standings_points_delta.is_some()
                && row.expected_remaining_points_delta.is_some()
        }));

        let error = build_team_season_forecast_movement(&later, &earlier).unwrap_err();
        assert!(error.contains("earlier cutoff"));

        let mut mismatched_schedule = later.clone();
        mismatched_schedule.games[0].date += Duration::days(1);
        let error =
            build_team_season_forecast_movement(&earlier, &mismatched_schedule).unwrap_err();
        assert!(error.contains("same schedule"));

        let mut mismatched_checkpoint = later.clone();
        mismatched_checkpoint
            .replay_checkpoint
            .as_mut()
            .unwrap()
            .as_of_date = first_date;
        let error =
            build_team_season_forecast_movement(&earlier, &mismatched_checkpoint).unwrap_err();
        assert!(error.contains("checkpoint date"));
    }

    #[test]
    fn forecast_history_tracks_chronological_sealed_checkpoints() {
        let mut forecast = fixture();
        let dates = forecast
            .games
            .iter()
            .map(|game| game.date)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(3)
            .collect::<Vec<_>>();
        let config = TeamSeasonSimulationConfig {
            trials: 20,
            seed: 44,
        };
        let mut checkpoints = Vec::new();
        for date in dates {
            for game in forecast.games.iter_mut().filter(|game| game.date == date) {
                game.actual_winner = Some(game.home_team.clone());
            }
            checkpoints.push(
                simulate_team_season_forecast_as_of_with_scenario(&forecast, config, None, date)
                    .unwrap(),
            );
        }

        let history = build_team_season_forecast_history(&checkpoints).unwrap();
        assert_eq!(history.schema, TEAM_SEASON_FORECAST_HISTORY_SCHEMA);
        assert_eq!(history.checkpoints.len(), 3);
        assert_eq!(history.teams.len(), 32);
        assert_eq!(history.biggest_risers.len(), 5);
        assert_eq!(history.biggest_fallers.len(), 5);
        assert_eq!(
            history
                .teams
                .iter()
                .map(|team| team.projected_points_movement_rank)
                .collect::<BTreeSet<_>>(),
            (1..=32).collect()
        );
        assert!(history
            .teams
            .iter()
            .all(|team| team.league_team_count == 32));
        assert!(history.biggest_risers.windows(2).all(|pair| {
            pair[0].average_points_delta_first_to_last >= pair[1].average_points_delta_first_to_last
        }));
        assert!(history.biggest_fallers.windows(2).all(|pair| {
            pair[0].average_points_delta_first_to_last <= pair[1].average_points_delta_first_to_last
        }));
        assert!(history
            .checkpoints
            .windows(2)
            .all(|pair| pair[0].as_of_date < pair[1].as_of_date));
        assert!(history.teams.iter().all(|team| {
            team.checkpoints.len() == 3
                && team.checkpoints[0]
                    .average_points_delta_from_previous
                    .is_none()
                && team.checkpoints[0]
                    .completed_games_delta_from_previous
                    .is_none()
                && team.checkpoints[1]
                    .average_points_delta_from_previous
                    .is_some()
                && team.checkpoints[2]
                    .playoff_probability_delta_from_previous
                    .is_some()
                && (team.average_points_delta_first_to_last
                    - (team.checkpoints[2].average_points - team.checkpoints[0].average_points))
                    .abs()
                    < 1e-9
                && team.points_movement_reconciliation_error.abs() < 1e-9
                && (team.average_points_delta_first_to_last
                    - (team.observed_standings_points_delta_first_to_last as f64
                        + team.expected_remaining_points_delta_first_to_last))
                    .abs()
                    < 1e-9
                && team
                    .pace_attribution_reconciliation_error
                    .is_some_and(|error| error.abs() < 1e-9)
                && team
                    .realized_points_vs_prior_remaining_pace
                    .zip(team.remaining_outlook_revaluation)
                    .is_some_and(|(realized, revaluation)| {
                        (team.average_points_delta_first_to_last - (realized + revaluation)).abs()
                            < 1e-9
                    })
                && team.checkpoints[1..].iter().all(|point| {
                    point
                        .pace_attribution_reconciliation_error_from_previous
                        .is_some_and(|error| error.abs() < 1e-9)
                        && point
                            .realized_points_vs_prior_remaining_pace_from_previous
                            .zip(point.remaining_outlook_revaluation_from_previous)
                            .zip(point.average_points_delta_from_previous)
                            .is_some_and(|((realized, revaluation), delta)| {
                                (delta - (realized + revaluation)).abs() < 1e-9
                            })
                })
        }));

        let mut reversed = checkpoints.clone();
        reversed.swap(0, 1);
        let error = build_team_season_forecast_history(&reversed).unwrap_err();
        assert!(error.contains("strictly increasing"));
        let error = build_team_season_forecast_history(&checkpoints[..1]).unwrap_err();
        assert!(error.contains("at least two"));

        let mut regressed = checkpoints.clone();
        let team = &mut regressed[1].replay_checkpoint.as_mut().unwrap().teams[0];
        team.completed_games = team.completed_games.saturating_sub(2);
        team.remaining_games += 2;
        let error = build_team_season_forecast_history(&regressed).unwrap_err();
        assert!(error.contains("replay progress cannot regress"));
    }

    #[test]
    fn forecast_history_classifies_multi_checkpoint_trajectory() {
        let dates = [
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap(),
            NaiveDate::from_ymd_opt(2025, 3, 31).unwrap(),
        ];
        let points = |values: [f64; 3]| {
            dates
                .into_iter()
                .zip(values)
                .map(
                    |(as_of_date, average_points)| TeamSeasonForecastHistoryPointRow {
                        as_of_date,
                        average_points,
                        points_p10: 80,
                        points_p50: 90,
                        points_p90: 100,
                        playoff_probability: 0.5,
                        stanley_cup_probability: 0.02,
                        average_longest_win_streak: 5.0,
                        completed_games: 50,
                        remaining_games: 32,
                        observed_standings_points: 54,
                        expected_remaining_points: 36.0,
                        average_points_delta_from_previous: None,
                        playoff_probability_delta_from_previous: None,
                        stanley_cup_probability_delta_from_previous: None,
                        completed_games_delta_from_previous: None,
                        prior_expected_points_for_completed_interval_from_previous: None,
                        realized_points_vs_prior_remaining_pace_from_previous: None,
                        remaining_outlook_revaluation_from_previous: None,
                        pace_attribution_reconciliation_error_from_previous: None,
                    },
                )
                .collect::<Vec<_>>()
        };

        assert_eq!(
            history_trend(&points([90.0, 91.0, 92.0])).0,
            TeamSeasonForecastHistoryTrend::Improving
        );
        assert_eq!(
            history_trend(&points([92.0, 91.0, 90.0])).0,
            TeamSeasonForecastHistoryTrend::Declining
        );
        let mixed = history_trend(&points([90.0, 93.0, 91.0]));
        assert_eq!(mixed.0, TeamSeasonForecastHistoryTrend::Mixed);
        assert_eq!(mixed.1, (dates[0], dates[1], 3.0));
        assert_eq!(
            history_trend(&points([90.0, 90.04, 90.01])).0,
            TeamSeasonForecastHistoryTrend::Stable
        );
        assert_eq!(
            history_movement_materiality(Some(0.099)),
            TeamSeasonForecastHistoryMateriality::Small
        );
        assert_eq!(
            history_movement_materiality(Some(0.10)),
            TeamSeasonForecastHistoryMateriality::Moderate
        );
        assert_eq!(
            history_movement_materiality(Some(0.25)),
            TeamSeasonForecastHistoryMateriality::Large
        );
        assert_eq!(
            history_movement_materiality(None),
            TeamSeasonForecastHistoryMateriality::Indeterminate
        );
    }
}
