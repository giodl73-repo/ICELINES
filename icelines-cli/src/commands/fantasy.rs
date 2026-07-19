//! Fantasy league commands — leagues, teams, scoring, trades, HTTP server.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::io::Read as _;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json,
};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use icelines_core::timeframe::Timeframe;
use icelines_core::view_model::{
    FantasyDailyDeltaView, FantasyDailyPlayerStatus, FantasyDailyTeamRow, FantasyImportRowStatus,
    FantasyImportView, FantasyMatchupOutcome, FantasyMatchupSideRow, FantasyMatchupWeekView,
};
use icelines_core::{
    apply_fantasy_pickup_reserve, build_fantasy_category_matchup, build_fantasy_daily_lineup,
    build_fantasy_draft_board, build_fantasy_goalie_plan, build_fantasy_matchup_strategy,
    build_fantasy_morning_briefing, build_fantasy_schedule_view, build_fantasy_simulation_view,
    build_fantasy_sleeper_board, build_fantasy_week_budget,
    build_fantasy_weekly_pickups_with_reserve_override, fantasy_acquisition_availability,
    goalie_scheme_stats_from_view, import_fantasy_platform_eligibility,
    import_fantasy_taken_players,
    model::{Position, Season},
    name::normalize_name,
    resolve_fantasy_goalie_start, resolve_fantasy_player_status,
    resolve_fantasy_scenario_roster_details,
    scheme::{compute_fantasy_score, Scheme},
    score_fantasy_roster,
    season_stats::SeasonType,
    simulate_fantasy_season, skater_scheme_stats_from_view,
    stats_repository::{PlayerView, StatsRepository},
    FantasyAcquisitionInput, FantasyAcquisitionKind, FantasyActiveSlotKind, FantasyAssistantRules,
    FantasyCategoryAggregation, FantasyCategoryDirection, FantasyCategoryMatchupInput,
    FantasyCategoryMatchupView, FantasyCategoryPlayerInput, FantasyCategoryRateInput,
    FantasyCategoryRule, FantasyCategoryScope, FantasyCategorySnapshotInput,
    FantasyCategoryTeamInput, FantasyCompetitionMode, FantasyCompetitionRules,
    FantasyDraftCandidateInput, FantasyDraftIdentityInput, FantasyGoalieGameInput,
    FantasyGoaliePlanInput, FantasyGoaliePlanPlayerInput, FantasyGoalieStartObservation,
    FantasyGoalieStartState, FantasyInjuryPlanView, FantasyLeagueInput, FantasyLeagueTeamInput,
    FantasyLeagueView, FantasyLineupPlayerInput, FantasyMatchupPointsSnapshotInput,
    FantasyMatchupStrategy, FantasyMatchupStrategyInput, FantasyMatchupStrategyPlayerInput,
    FantasyMatchupStrategyTeamInput, FantasyMatchupStrategyView, FantasyMatchupSwingInput,
    FantasyMatchupTiePolicy, FantasyObservationConfidence, FantasyPlayerAvailabilityStatus,
    FantasyRosterGapInput, FantasyRosterGapView, FantasySeasonEventKind, FantasySeasonSimConfig,
    FantasySeasonSimPlayerInput, FantasySeasonSimView, FantasySimulationBuildInput,
    FantasySimulationConfidence, FantasySimulationHorizon, FantasySimulationRosterTeamInput,
    FantasySimulationScenarioRosterInput, FantasySimulationView, FantasySleeperBoardView,
    FantasySleeperInput, FantasyStatusObservation, FantasyWeeklyMoveInput, RosterShape,
    RosterShapeStatus, RosterShapeValidationView, ViewContext, ViewWindow, CURRENT_SEASON,
    FANTASY_COMPETITION_RULES_SCHEMA,
};
use icelines_fetch::datastore::DataStore;
use icelines_fetch::fantasy_daily::build_fantasy_daily_delta_view;
use icelines_fetch::fantasy_import::{
    import_yahoo_roster_csv, import_yahoo_roster_rows, parse_yahoo_roster_csv_text,
    FantasyRosterImportOptions,
};
use icelines_fetch::fantasy_matchup::build_fantasy_matchup_week_view;
use icelines_fetch::nhl_api::{NhlApiClient, ScheduledGame};
use icelines_fetch::schedule_remaining::{default_data_root, remaining_games_by_team_from_cache};
use icelines_fetch::schema::RosterResponse;
use icelines_fetch::snapshot::{SnapshotStore, SnapshotTier};
use icelines_fetch::stats_loader::LoadOutcome;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::fantasy_db::{resolve_roster_shape, FantasyDb, LeagueRow, TeamRow};

const CREASE_STARTER_EVIDENCE_HEADER: &str = "THE CREASE — STARTER EVIDENCE";
const CREASE_GOALIE_PLAN_HEADER: &str = "THE CREASE — WHO GETS THE NET?";
const PENALTY_BOX_AVAILABILITY_HEADER: &str = "THE PENALTY BOX — AVAILABILITY REPORT";
const INSIDER_MORNING_SKATE_HEADER: &str = "THE INSIDER — MORNING SKATE";
const SCOREBOARD_FANTASY_STANDINGS_HEADER: &str = "THE SCOREBOARD — FANTASY STANDINGS";
const BENCH_SCHEDULE_EDGE_HEADER: &str = "THE BENCH — THE GAUNTLET — FANTASY SCHEDULE EDGE";
const FACEOFF_MATCHUP_HEADER: &str = "THE FACEOFF CIRCLE — TALE OF THE TAPE — MATCHUP PLAN";
const FACEOFF_CATEGORY_MATCHUP_HEADER: &str =
    "THE FACEOFF CIRCLE — TALE OF THE TAPE — CATEGORY MATCHUP PLAN";
const BENCH_WAIVER_WIRE_HEADER: &str = "THE BENCH — WAIVER WIRE — WEEKLY PICKUPS";
const BENCH_CALL_UP_BOARD_HEADER: &str = "THE BENCH — CALL-UP BOARD — SLEEPERS";
const BENCH_WAR_ROOM_DRAFT_HEADER: &str = "THE BENCH — WAR ROOM — DRAFT BOARD";
const BOARDS_TRADE_READINESS_HEADER: &str = "THE BOARDS — TRADE READINESS";
const BOARDS_TRADE_ANALYSIS_HEADER: &str = "THE BOARDS — TRADE DESK — ANALYSIS";
const BOARDS_TRADE_HISTORY_HEADER: &str = "THE BOARDS — TRADE HISTORY";
const BOARDS_TRADE_OFFERS_HEADER: &str = "THE BOARDS — TRADE OFFERS";
const BOARDS_TRADE_FINDER_HEADER: &str = "THE BOARDS — HOT STOVE — TRADE FINDER";

#[derive(Debug, Clone)]
pub struct ScheduleEdgeArgs {
    pub season: u32,
    pub week: Option<NaiveDate>,
    pub teams: Vec<String>,
    pub league: Option<String>,
    pub off_night_max_games: usize,
    pub classes: usize,
    pub refresh: bool,
    pub json: bool,
    pub out: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SeasonSimArgs {
    pub league: Option<String>,
    pub team: Option<String>,
    pub teams: usize,
    pub playoff_teams: usize,
    pub trials: usize,
    pub seed: u64,
    pub injury_rate: f64,
    pub trade_probability: f64,
    pub opponent_pickup_accuracy: f64,
    pub pickup_reserve: u8,
    pub exceptional_reserve_min_value: f64,
    pub exceptional_reserve_min_games: i8,
    pub strict_pickup_reserve: bool,
    pub scenario_matrix: bool,
    pub manager_matrix: bool,
    pub reserve_matrix: bool,
    pub season: u32,
    pub stats_season: String,
    pub json: bool,
}

#[derive(Debug, Clone, Serialize)]
struct SeasonSimScenarioResult {
    scenario: String,
    view: FantasySeasonSimView,
}

/// Find a view by partial normalized name in the given slice.
fn fuzzy_find_view_in<'a, 'r>(
    views: &'a [PlayerView<'r>],
    query: &str,
) -> Option<&'a PlayerView<'r>> {
    let norm = normalize_name(query);
    views
        .iter()
        .find(|v| v.identity.name_normalized.contains(norm.as_str()))
}

/// Skater-pool fuzzy find with anyhow context for the team-add / trade
/// flows. Mirrors the previous `fuzzy_find_player` shape.
fn fuzzy_find_skater<'a, 'r>(
    skaters: &'a [PlayerView<'r>],
    query: &str,
) -> anyhow::Result<&'a PlayerView<'r>> {
    fuzzy_find_view_in(skaters, query).with_context(|| {
        format!(
            "no skater found matching '{query}' — try `icelines query goalies` if it's a goalie"
        )
    })
}

/// Load both pools at once as a `LoadOutcome` (which owns the
/// `StatsRepository`) plus the `Season` callers should use to query.
/// Hart.5c.4: replaces the previous `(Vec<Player>, Vec<Goalie>)` shape.
/// Hart.6.9: load_repo_for_season now returns a 3-tuple including
/// season_type — fantasy is regular-only, drop the type.
fn load_pools() -> anyhow::Result<(LoadOutcome, Season)> {
    let (outcome, season, _ty) = crate::commands::players::load_repo_for_season(None, None)?;
    Ok((outcome, season))
}

/// Convenience: collect skaters and goalies from a repo into Vecs.
/// Each caller holds the repo for the scope of its handler so the
/// returned views borrow from it; this wraps the two collect calls.
fn pools_views<'r>(
    repo: &'r StatsRepository,
    season: Season,
) -> (Vec<PlayerView<'r>>, Vec<PlayerView<'r>>) {
    (
        repo.skaters(season, SeasonType::Regular).collect(),
        repo.goalies(season, SeasonType::Regular).collect(),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Resolve a scheme name to a `Scheme` struct.
fn resolve_scheme(name: &str) -> anyhow::Result<Scheme> {
    Scheme::builtin_named(name).with_context(|| {
        let names = Scheme::all_builtins()
            .into_iter()
            .map(|scheme| scheme.name)
            .collect::<Vec<_>>()
            .join(", ");
        format!("unknown scheme '{name}'. Try: {names}")
    })
}

/// Legacy CLI/server adapter over the shared core scoring contract.
///
/// League-management text and the legacy `fantasy serve` routes still consume
/// `(full_name, score)` pairs, but scoring itself must stay in
/// `score_fantasy_roster` so CLI/TUI/web/report surfaces do not fork fantasy
/// math or skater-vs-goalie lookup behavior.
fn score_team(
    roster_norms: &[String],
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
    scheme: &Scheme,
) -> Vec<(String, f32)> {
    score_fantasy_roster(roster_norms, skaters, goalies, scheme)
        .into_iter()
        .map(|row| (row.player.identity.full_name.clone(), row.score))
        .collect()
}

fn fantasy_league_view(
    leagues: Vec<LeagueRow>,
    active_league: Option<&LeagueRow>,
    teams: Vec<TeamRow>,
) -> FantasyLeagueView {
    let context = ViewContext::new(ViewWindow::new(Season(CURRENT_SEASON), SeasonType::Regular));
    FantasyLeagueView::from_rows(
        context,
        active_league.map(|league| league.name.clone()),
        leagues
            .into_iter()
            .map(|league| FantasyLeagueInput {
                name: league.name,
                scoring_scheme: league.scheme,
                is_active: league.is_active,
                team_count: league.team_count,
            })
            .collect(),
        teams
            .into_iter()
            .map(|team| FantasyLeagueTeamInput {
                name: team.name,
                owner: team.owner,
                is_user_team: team.is_user_team,
                player_count: team.player_count,
            })
            .collect(),
    )
}

/// Require an active league, or use the given override.
fn require_league(db: &FantasyDb, league_override: &Option<String>) -> anyhow::Result<LeagueRow> {
    if let Some(name) = league_override {
        let leagues = db.list_leagues()?;
        leagues
            .into_iter()
            .find(|l| &l.name == name)
            .with_context(|| {
                format!("league '{name}' not found — run `icelines fantasy league-list`")
            })
    } else {
        db.get_active_league()?.ok_or_else(|| {
            anyhow::anyhow!(
                "no active league. Use `icelines fantasy league-use <name>` or --league."
            )
        })
    }
}

/// Require a named team within a league.
fn require_team(db: &FantasyDb, league_id: &str, team_name: &str) -> anyhow::Result<TeamRow> {
    db.get_team_by_name(league_id, team_name)?
        .with_context(|| format!("team '{team_name}' not found in this league"))
}

// ── League commands ───────────────────────────────────────────────────────────

/// `icelines fantasy league-create <name> [--scheme <scheme>]`
pub async fn run_league_create(name: String, scheme_name: String) -> anyhow::Result<()> {
    // Validate the scheme name.
    resolve_scheme(&scheme_name)?;

    let db = FantasyDb::open()?;
    db.create_league(&name, &scheme_name)?;

    // Always activate the newly created league — user just created it, they want to work with it.
    db.set_active_league(&name)?;
    println!("League '{name}' created (scheme: {scheme_name}). Set as active.");
    Ok(())
}

/// `icelines fantasy league-list`
pub async fn run_league_list() -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let leagues = db.list_leagues()?;
    let view = fantasy_league_view(leagues, None, Vec::new());

    if view.leagues.is_empty() {
        println!("No leagues yet. Create one with `icelines fantasy league-create <name>`.");
        return Ok(());
    }

    println!("{:<28} {:<18} {:<7} Active", "Name", "Scheme", "Teams");
    println!("{}", "─".repeat(60));
    for l in &view.leagues {
        let active_marker = if l.is_active { "<—" } else { "" };
        println!(
            "{:<28} {:<18} {:<7} {}",
            l.name, l.scoring_scheme, l.team_count, active_marker
        );
    }
    Ok(())
}

/// `icelines fantasy league-use <name>` / `icelines fantasy league-switch <name>`
pub async fn run_league_use(name: String) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    db.set_active_league(&name).with_context(|| {
        format!("league '{name}' not found — run `icelines fantasy league-list`")
    })?;
    println!("Active league set to '{name}'.");
    Ok(())
}

pub async fn run_league_scheme_set(
    scheme_name: String,
    league_override: Option<String>,
) -> anyhow::Result<()> {
    resolve_scheme(&scheme_name)?;
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    db.set_league_scheme(&league.id, &scheme_name)?;
    println!(
        "League '{}' scoring scheme set to '{}'.",
        league.name, scheme_name
    );
    Ok(())
}

pub async fn run_competition_show(
    league_override: Option<String>,
    json_output: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db.get_competition_rules(&league.id)?;
    if json_output {
        println!("{}", serde_json::to_string_pretty(&rules)?);
        return Ok(());
    }
    println!("Competition — {} · {}", league.name, rules.mode.label());
    if rules.mode == FantasyCompetitionMode::Points {
        println!("  Scoring scheme: {}", league.scheme);
        return Ok(());
    }
    println!(
        "  Goalie minimum: {} appearance(s) · matchup ties: {:?}",
        rules.minimum_goalie_appearances, rules.matchup_tie_policy
    );
    for category in &rules.categories {
        println!(
            "  {:<28} {:<12} {:<8} tie ±{}",
            category.key,
            match category.direction {
                FantasyCategoryDirection::HigherWins => "higher wins",
                FantasyCategoryDirection::LowerWins => "lower wins",
            },
            match category.aggregation {
                FantasyCategoryAggregation::Sum => "sum",
                FantasyCategoryAggregation::Ratio => "ratio",
            },
            category.tie_epsilon
        );
    }
    Ok(())
}

pub async fn run_competition_set(
    mode: String,
    category_specs: Vec<String>,
    minimum_goalie_appearances: u8,
    tie_policy: String,
    league_override: Option<String>,
) -> anyhow::Result<()> {
    let mode = match mode.trim().to_ascii_lowercase().as_str() {
        "points" => FantasyCompetitionMode::Points,
        "categories" => FantasyCompetitionMode::Categories,
        other => bail!("unknown competition mode '{other}'; expected points or categories"),
    };
    let matchup_tie_policy = match tie_policy.trim().to_ascii_lowercase().as_str() {
        "tie" => FantasyMatchupTiePolicy::Tie,
        "higher_seed_wins" | "higher-seed-wins" => FantasyMatchupTiePolicy::HigherSeedWins,
        other => bail!("unknown tie policy '{other}'; expected tie or higher_seed_wins"),
    };
    let categories = category_specs
        .iter()
        .map(|spec| parse_category_rule_spec(spec))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let rules = FantasyCompetitionRules {
        schema: FANTASY_COMPETITION_RULES_SCHEMA.to_owned(),
        mode,
        categories,
        minimum_goalie_appearances,
        matchup_tie_policy,
    };
    rules.validate().map_err(anyhow::Error::msg)?;

    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    db.set_competition_rules(&league.id, &rules)?;
    println!(
        "League '{}' competition mode set to '{}' with {} category rule(s).",
        league.name,
        rules.mode.label(),
        rules.categories.len()
    );
    Ok(())
}

fn parse_category_rule_spec(spec: &str) -> anyhow::Result<FantasyCategoryRule> {
    let parts = spec.split(':').map(str::trim).collect::<Vec<_>>();
    if !(3..=4).contains(&parts.len()) {
        bail!("invalid category '{spec}'; expected KEY:DIRECTION:AGGREGATION[:TIE_EPSILON]");
    }
    let key = parts[0].to_ascii_lowercase().replace('-', "_");
    let direction = match parts[1].to_ascii_lowercase().as_str() {
        "higher" | "higher_wins" => FantasyCategoryDirection::HigherWins,
        "lower" | "lower_wins" => FantasyCategoryDirection::LowerWins,
        other => bail!("invalid direction '{other}' in category '{spec}'"),
    };
    let aggregation = match parts[2].to_ascii_lowercase().as_str() {
        "sum" | "counting" => FantasyCategoryAggregation::Sum,
        "ratio" => FantasyCategoryAggregation::Ratio,
        other => bail!("invalid aggregation '{other}' in category '{spec}'"),
    };
    let tie_epsilon = parts
        .get(3)
        .map(|value| {
            value
                .parse::<f64>()
                .with_context(|| format!("invalid tie epsilon '{value}' in category '{spec}'"))
        })
        .transpose()?
        .unwrap_or_default();
    Ok(FantasyCategoryRule {
        label: key.to_ascii_uppercase(),
        key,
        direction,
        aggregation,
        tie_epsilon,
    })
}

fn read_category_snapshot(path: &PathBuf) -> anyhow::Result<FantasyCategorySnapshotInput> {
    let text = if path.as_os_str() == "-" {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .context("read category snapshot JSON from stdin")?;
        input
    } else {
        std::fs::read_to_string(path)
            .with_context(|| format!("read category snapshot {}", path.display()))?
    };
    serde_json::from_str(&text).with_context(|| {
        format!(
            "parse category snapshot JSON from {}",
            if path.as_os_str() == "-" {
                "stdin".to_owned()
            } else {
                path.display().to_string()
            }
        )
    })
}

/// `icelines fantasy league-delete <name>`
pub async fn run_league_delete(name: String) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let deleted = db.delete_league(&name)?;
    if deleted {
        println!("League '{name}' and all its teams deleted.");
    } else {
        bail!("league '{name}' not found");
    }
    Ok(())
}

// ── Team commands ─────────────────────────────────────────────────────────────

/// `icelines fantasy team-create <name> [--owner <owner>] [--league <league>]`
pub async fn run_team_create(
    name: String,
    owner: Option<String>,
    league_override: Option<String>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let owner_str = owner.as_deref().unwrap_or("");
    db.create_team(&league.id, &name, owner_str)?;
    println!("Team '{name}' created in league '{}'.", league.name);
    Ok(())
}

/// `icelines fantasy team-list [--league <league>]`
pub async fn run_team_list(league_override: Option<String>) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let teams = db.list_teams(&league.id)?;
    let view = fantasy_league_view(vec![league.clone()], Some(&league), teams);

    if view.teams.is_empty() {
        println!(
            "No teams in '{}'. Add one with `icelines fantasy team-create <name>`.",
            league.name
        );
        return Ok(());
    }

    println!("League: {} ({})", league.name, league.scheme);
    println!(
        "{:<28} {:<20} {:<6} {:<8}",
        "Team", "Owner", "Mine", "Players"
    );
    println!("{}", "─".repeat(58));
    for t in &view.teams {
        println!(
            "{:<28} {:<20} {:<6} {:<8}",
            t.name,
            t.owner,
            if t.is_user_team { "yes" } else { "-" },
            t.player_count
        );
    }
    Ok(())
}

/// `icelines fantasy team-use <name> [--league <league>]`
pub async fn run_team_use(name: String, league_override: Option<String>) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    if !db.set_user_team(&league.id, &name)? {
        bail!("team '{name}' not found in league '{}'", league.name);
    }
    println!("User team set to '{name}' in league '{}'.", league.name);
    Ok(())
}

/// `icelines fantasy team-show <name> [--league <league>]`
pub async fn run_team_show(
    name: String,
    league_override: Option<String>,
    stats_season: Option<String>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let team = require_team(&db, &league.id, &name)?;
    let scheme = resolve_scheme(&league.scheme)?;
    let (outcome, season, _) =
        crate::commands::players::load_repo_for_season(stats_season.as_deref(), None)?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);
    let roster_norms = db.list_roster(&team.id)?;

    println!(
        "\nRoster: {}{} | League: {} | Scheme: {}",
        team.name,
        if team.is_user_team { " (mine)" } else { "" },
        league.name,
        league.scheme
    );
    println!("{}", "─".repeat(72));
    println!(
        "  {:<4} {:<24} {:<5} {:<4} {:<5} {:<7} Fantasy",
        "#", "Player", "Team", "Pos", "GP", "Pts"
    );
    println!("{}", "─".repeat(72));

    let mut total_score = 0.0f32;
    let scored = score_team(&roster_norms, &all_skaters, &all_goalies, &scheme);

    for (rank, (full_name, fscore)) in scored.iter().enumerate() {
        let norm_q = normalize_name(full_name);
        let skater = all_skaters
            .iter()
            .find(|v| v.identity.name_normalized.contains(norm_q.as_str()));
        let goalie = all_goalies
            .iter()
            .find(|v| v.identity.name_normalized.contains(norm_q.as_str()));

        let (team_abbr, pos, gp_str, pts_str) = match (skater, goalie) {
            (Some(view), _) => (
                view.team_display().to_owned(),
                view.position().abbreviation().to_owned(),
                view.gp().to_string(),
                view.stats.totals.points.to_string(),
            ),
            (None, Some(view)) => (
                view.team_display().to_owned(),
                Position::Goalie.abbreviation().to_owned(),
                view.gp().to_string(),
                "—".to_owned(),
            ),
            (None, None) => (
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
                "—".to_owned(),
            ),
        };

        println!(
            "  {:<4} {:<24} {:<5} {:<4} {:<5} {:<7} {:.1}",
            rank + 1,
            full_name,
            team_abbr,
            pos,
            gp_str,
            pts_str,
            fscore
        );
        total_score += fscore;
    }

    println!("{}", "─".repeat(72));
    println!("  Total fantasy score: {total_score:.1}");
    Ok(())
}

/// `icelines fantasy team-add <team> <player> [--league <league>]`
pub async fn run_team_add(
    team_name: String,
    player_query: String,
    league_override: Option<String>,
    stats_season: Option<String>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let team = require_team(&db, &league.id, &team_name)?;
    let (outcome, season, _) =
        crate::commands::players::load_repo_for_season(stats_season.as_deref(), None)?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);

    // Search both pools — skater first, goalie fallback.
    let (full_name, norm_owned, kind) = match fuzzy_find_skater(&all_skaters, &player_query) {
        Ok(v) => (
            v.identity.full_name.clone(),
            v.identity.name_normalized.clone(),
            "Skater",
        ),
        Err(skater_err) => match fuzzy_find_view_in(&all_goalies, &player_query) {
            Some(v) => (
                v.identity.full_name.clone(),
                v.identity.name_normalized.clone(),
                "Goalie",
            ),
            None => return Err(skater_err),
        },
    };
    let norm = norm_owned.as_str();

    if let Some(taken_by) = db.is_on_any_team(&league.id, norm)? {
        bail!("'{full_name}' is already on team '{taken_by}'. Drop them first.");
    }

    db.add_player(&team.id, norm)?;
    println!("Added {full_name} ({kind}) to '{team_name}'.");
    Ok(())
}

/// `icelines fantasy team-drop <team> <player> [--league <league>]`
pub async fn run_team_drop(
    team_name: String,
    player_query: String,
    league_override: Option<String>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let team = require_team(&db, &league.id, &team_name)?;
    let (outcome, season) = load_pools()?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);

    let (full_name, norm_owned) = match fuzzy_find_skater(&all_skaters, &player_query) {
        Ok(v) => (
            v.identity.full_name.clone(),
            v.identity.name_normalized.clone(),
        ),
        Err(skater_err) => match fuzzy_find_view_in(&all_goalies, &player_query) {
            Some(v) => (
                v.identity.full_name.clone(),
                v.identity.name_normalized.clone(),
            ),
            None => return Err(skater_err),
        },
    };
    let norm = norm_owned.as_str();

    let dropped = db.drop_player(&team.id, norm)?;
    if dropped {
        println!("Dropped {full_name} from '{team_name}'.");
    } else {
        bail!("'{full_name}' is not on team '{team_name}'.");
    }
    Ok(())
}

// ── Standings ─────────────────────────────────────────────────────────────────

/// `icelines fantasy standings [--league <league>] [--scheme <scheme>]`
pub async fn run_standings(
    league_override: Option<String>,
    scheme_override: Option<String>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let scheme_name = scheme_override.as_deref().unwrap_or(&league.scheme);
    let scheme = resolve_scheme(scheme_name)?;
    let (outcome, season) = load_pools()?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);
    let teams = db.list_teams(&league.id)?;

    // Compute scores for each team.
    let mut standings: Vec<(String, String, f32, f32)> = Vec::new(); // (team, owner, total, per_g)
    for team in &teams {
        let roster = db.list_roster(&team.id)?;
        let scored = score_team(&roster, &all_skaters, &all_goalies, &scheme);
        let total: f32 = scored.iter().map(|(_, s)| s).sum();
        let gp_total: u32 = scored
            .iter()
            .filter_map(|(name, _)| {
                let norm = normalize_name(name);
                all_skaters
                    .iter()
                    .find(|v| v.identity.name_normalized.contains(norm.as_str()))
                    .map(|v| v.gp())
            })
            .sum();
        let per_g = if gp_total > 0 {
            total / gp_total as f32
        } else {
            0.0
        };
        standings.push((team.name.clone(), team.owner.clone(), total, per_g));
    }

    standings.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    println!(
        "\n{} — {} ({})",
        SCOREBOARD_FANTASY_STANDINGS_HEADER, league.name, scheme_name
    );
    println!("{}", "─".repeat(60));
    println!(
        "{:<5} {:<22} {:<16} {:<10} Per/G",
        "Rank", "Team", "Owner", "Score"
    );
    println!("{}", "─".repeat(60));
    for (rank, (team_name, owner, total, per_g)) in standings.iter().enumerate() {
        // Avoid -0.0 display for empty teams (IEEE 754 negative zero)
        let total_display = if *total == 0.0 { 0.0f32 } else { *total };
        let perg_display = if *per_g == 0.0 { 0.0f32 } else { *per_g };
        println!(
            "{:<5} {:<22} {:<16} {:<10.1} {:.2}",
            rank + 1,
            team_name,
            owner,
            total_display,
            perg_display
        );
    }
    Ok(())
}

// ── Trade ─────────────────────────────────────────────────────────────────────

/// `icelines fantasy gaps [--league <league>] [--scheme <scheme>]`
pub async fn run_gaps(
    league_override: Option<String>,
    scheme_override: Option<String>,
    categories: Vec<String>,
    top: usize,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let snapshot = db.league_snapshot(league_override.as_deref())?;
    let scheme_name = scheme_override
        .as_deref()
        .unwrap_or(&snapshot.scoring_scheme);
    resolve_scheme(scheme_name)?;
    let (outcome, season) = load_pools()?;
    let all_rostered = snapshot.all_rostered();
    let user_roster = snapshot.user_rostered();
    let view = FantasyRosterGapView::from_repository(
        &outcome.repo,
        FantasyRosterGapInput {
            season,
            season_type: SeasonType::Regular,
            league: &snapshot.league,
            team: &snapshot.user_team,
            scoring_scheme: scheme_name,
            categories,
            user_roster_keys: user_roster,
            all_rostered_keys: all_rostered,
            limit: top,
        },
    );

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing fantasy gaps")?
        );
        return Ok(());
    }

    print_gaps(&view);
    Ok(())
}

fn print_gaps(view: &FantasyRosterGapView) {
    println!(
        "Fantasy gaps - {} / {} ({})",
        view.league, view.team, view.scoring_scheme
    );
    for warning in &view.warnings {
        println!("warning: {warning}");
    }
    println!(
        "{:<10} {:<16} {:>10} {:>6} {:<24} {:<4} {:>8} {:>8} {:<24} Recommendation",
        "Action",
        "Category",
        "Roster",
        "Weight",
        "Best Available",
        "Pos",
        "Value",
        "WDelta",
        "Drop"
    );
    for row in &view.rows {
        let candidate = row.best_available.as_ref();
        let target = row.replacement_target.as_ref();
        println!(
            "{:<10} {:<16} {:>10.1} {:>6.2} {:<24} {:<4} {:>8.1} {:>8.1} {:<24} {}",
            format!("{:?}", row.action).to_ascii_lowercase(),
            row.category,
            row.user_total,
            row.weight,
            candidate
                .map(|candidate| candidate.display_name.as_str())
                .unwrap_or("-"),
            candidate
                .map(|candidate| candidate.position.as_str())
                .unwrap_or("-"),
            candidate.map(|candidate| candidate.value).unwrap_or(0.0),
            target
                .map(|target| target.weighted_delta)
                .unwrap_or(row.weighted_gap_score),
            target
                .map(|target| target.display_name.as_str())
                .unwrap_or("-"),
            row.recommendation
        );
    }
}

/// `icelines fantasy simulate [--league <league>] [--weeks N] [--add P --drop P]`
pub async fn run_simulate(
    league_override: Option<String>,
    scheme_override: Option<String>,
    weeks: u8,
    add_player: Option<String>,
    drop_player: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let snapshot = db.league_snapshot(league_override.as_deref())?;
    let scheme_name = scheme_override
        .as_deref()
        .unwrap_or(&snapshot.scoring_scheme);
    let scheme = resolve_scheme(scheme_name)?;
    let (outcome, season) = load_pools()?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);
    let (remaining_by_team, schedule_warning) = remaining_games_by_team(season).await;
    let schedule_available = !remaining_by_team.is_empty();

    let mut scenario_rosters = Vec::new();
    if add_player.is_some() || drop_player.is_some() {
        let baseline = snapshot
            .teams
            .iter()
            .find(|team| team.name == snapshot.user_team)
            .map(|team| team.roster.clone())
            .unwrap_or_default();
        let scenario = resolve_fantasy_scenario_roster_details(
            &baseline,
            add_player.as_deref(),
            drop_player.as_deref(),
            &all_skaters,
            &all_goalies,
        )
        .map_err(|message| anyhow::anyhow!(message))?;
        scenario_rosters.push(FantasySimulationScenarioRosterInput {
            id: "cli-add-drop".to_string(),
            label: "CLI add/drop scenario".to_string(),
            add_player: scenario.resolved_add_player.or(add_player),
            drop_player: scenario.resolved_drop_player.or(drop_player),
            baseline_roster: baseline,
            scenario_roster: scenario.roster,
            confidence: FantasySimulationConfidence::Low,
        });
    }

    let mut assumptions = vec![
        "projects each roster from season-to-date fantasy points per played game".to_string(),
        "games remaining are summed from each resolved player's NHL team schedule".to_string(),
    ];
    let mut warnings = Vec::new();
    if let Some(warning) = schedule_warning {
        warnings.push(warning);
        assumptions.push(
            "schedule unavailable; projection falls back to current fantasy score".to_string(),
        );
    }

    let view = build_fantasy_simulation_view(
        FantasySimulationBuildInput {
            season,
            season_type: SeasonType::Regular,
            league: snapshot.league,
            scoring_scheme: scheme_name.to_string(),
            horizon: FantasySimulationHorizon::Weeks(weeks.max(1)),
            user_team: snapshot.user_team,
            teams: snapshot
                .teams
                .into_iter()
                .map(|team| FantasySimulationRosterTeamInput {
                    team: team.name,
                    owner: team.owner,
                    roster: team.roster,
                })
                .collect(),
            remaining_by_team,
            scenarios: Vec::new(),
            scenario_rosters,
            assumptions,
            warnings,
            schedule_available,
        },
        &all_skaters,
        &all_goalies,
        &scheme,
    );

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing fantasy simulation")?
        );
        return Ok(());
    }

    print_simulation(&view);
    Ok(())
}

/// Run a seeded, non-mutating whole-season stress simulation over a synthetic league.
pub async fn run_season_sim(args: SeasonSimArgs) -> anyhow::Result<()> {
    if !(2..=32).contains(&args.teams) {
        bail!("--teams must be between 2 and 32");
    }
    if args.playoff_teams == 0 || args.playoff_teams > args.teams {
        bail!("--playoff-teams must be between 1 and --teams");
    }
    if args.trials == 0 || args.trials > 10_000 {
        bail!("--trials must be between 1 and 10000");
    }
    if !(0.0..=1.0).contains(&args.injury_rate) {
        bail!("--injury-rate must be between 0 and 1");
    }
    if !(0.0..=1.0).contains(&args.trade_probability) {
        bail!("--trade-probability must be between 0 and 1");
    }
    if !(0.0..=1.0).contains(&args.opponent_pickup_accuracy) {
        bail!("--opponent-pickup-accuracy must be between 0 and 1");
    }
    if !args.exceptional_reserve_min_value.is_finite()
        || args.exceptional_reserve_min_value < 0.0
        || args.exceptional_reserve_min_games < 0
    {
        bail!("exceptional reserve thresholds must be finite and non-negative");
    }
    if [
        args.scenario_matrix,
        args.manager_matrix,
        args.reserve_matrix,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count()
        > 1
    {
        bail!("--scenario-matrix, --manager-matrix, and --reserve-matrix are mutually exclusive");
    }

    let db = FantasyDb::open()?;
    let league = require_league(&db, &args.league)?;
    let selected_team = if let Some(team_name) = args.team.as_deref() {
        Some(require_team(&db, &league.id, team_name)?)
    } else {
        db.get_user_team(&league.id)?
    };
    let selected_roster = selected_team
        .as_ref()
        .map(|team| db.list_roster(&team.id))
        .transpose()?
        .unwrap_or_default();
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let scheme = resolve_scheme(&league.scheme)?;
    let season = Season(args.season);
    let schedule = load_fantasy_schedule(season, false).await?;
    let (team_dates, _) = draft_schedule_metrics(&schedule);
    let current_teams = load_current_player_team_map(season).ok();
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    let (outcome, stats_season, _) =
        crate::commands::players::load_repo_for_season(Some(&args.stats_season), None)?;
    let (skaters, goalies) = pools_views(&outcome.repo, stats_season);
    let mut players = Vec::new();

    for view in skaters {
        let key = view.identity.name_normalized.clone();
        let team = current_teams
            .as_ref()
            .and_then(|teams| teams.get(&key))
            .cloned()
            .unwrap_or_else(|| view.team_display().to_owned());
        let Some(dates) = team_dates.get(&team) else {
            continue;
        };
        let Some(score) = compute_fantasy_score(
            &skater_scheme_stats_from_view(&view),
            &scheme.skater,
            view.gp(),
        ) else {
            continue;
        };
        players.push(FantasySeasonSimPlayerInput {
            player_key: key.clone(),
            player: view.full_name().to_owned(),
            nhl_team: team,
            positions: eligibility
                .get(&key)
                .cloned()
                .unwrap_or_else(|| vec![view.position()]),
            fantasy_points_per_game: f64::from(score.per_game.max(0.0)),
            game_dates: dates.iter().copied().collect(),
        });
    }
    for view in goalies {
        let key = view.identity.name_normalized.clone();
        let team = current_teams
            .as_ref()
            .and_then(|teams| teams.get(&key))
            .cloned()
            .unwrap_or_else(|| view.team_display().to_owned());
        let Some(dates) = team_dates.get(&team) else {
            continue;
        };
        let Some(goalie_stats) = goalie_scheme_stats_from_view(&view) else {
            continue;
        };
        let Some(score) = icelines_core::scheme::compute_goalie_fantasy_score(
            &goalie_stats,
            &scheme.goalie,
            view.gp(),
        ) else {
            continue;
        };
        players.push(FantasySeasonSimPlayerInput {
            player_key: key.clone(),
            player: view.full_name().to_owned(),
            nhl_team: team,
            positions: eligibility
                .get(&key)
                .cloned()
                .unwrap_or_else(|| vec![Position::Goalie]),
            fantasy_points_per_game: f64::from(score.per_game.max(0.0)),
            game_dates: dates.iter().copied().collect(),
        });
    }

    let pool_keys = players
        .iter()
        .map(|player| player.player_key.clone())
        .collect::<BTreeSet<_>>();
    let locked_user_roster = selected_roster
        .iter()
        .filter(|key| pool_keys.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    let unresolved_user_roster = selected_roster
        .iter()
        .filter(|key| !pool_keys.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    let profiles = if args.scenario_matrix {
        vec![
            (
                "clean",
                0.0,
                0.0,
                args.opponent_pickup_accuracy,
                args.pickup_reserve,
                !args.strict_pickup_reserve,
            ),
            (
                "baseline",
                args.injury_rate,
                args.trade_probability,
                args.opponent_pickup_accuracy,
                args.pickup_reserve,
                !args.strict_pickup_reserve,
            ),
            (
                "high-chaos",
                args.injury_rate.max(0.003).min(1.0),
                args.trade_probability.max(0.35).min(1.0),
                args.opponent_pickup_accuracy,
                args.pickup_reserve,
                !args.strict_pickup_reserve,
            ),
        ]
    } else if args.manager_matrix {
        vec![
            (
                "parity",
                args.injury_rate,
                args.trade_probability,
                1.0,
                args.pickup_reserve,
                !args.strict_pickup_reserve,
            ),
            (
                "edge-15",
                args.injury_rate,
                args.trade_probability,
                0.85,
                args.pickup_reserve,
                !args.strict_pickup_reserve,
            ),
            (
                "edge-30",
                args.injury_rate,
                args.trade_probability,
                0.70,
                args.pickup_reserve,
                !args.strict_pickup_reserve,
            ),
        ]
    } else if args.reserve_matrix {
        vec![
            (
                "all-in",
                args.injury_rate,
                args.trade_probability,
                args.opponent_pickup_accuracy,
                0,
                false,
            ),
            (
                "strict",
                args.injury_rate,
                args.trade_probability,
                args.opponent_pickup_accuracy,
                args.pickup_reserve,
                false,
            ),
            (
                "adaptive",
                args.injury_rate,
                args.trade_probability,
                args.opponent_pickup_accuracy,
                args.pickup_reserve,
                true,
            ),
        ]
    } else {
        vec![(
            "baseline",
            args.injury_rate,
            args.trade_probability,
            args.opponent_pickup_accuracy,
            args.pickup_reserve,
            !args.strict_pickup_reserve,
        )]
    };
    let target_team = selected_team
        .as_ref()
        .map(|team| team.name.clone())
        .unwrap_or_else(|| "Gio Simulation".to_owned());
    let mut results = Vec::with_capacity(profiles.len());
    for (
        scenario,
        injury_rate,
        trade_probability,
        opponent_pickup_accuracy,
        pickup_reserve,
        exceptional_reserve_enabled,
    ) in profiles
    {
        let mut view = simulate_fantasy_season(
            season.as_str(),
            league.scheme.clone(),
            rules.clone(),
            players.clone(),
            FantasySeasonSimConfig {
                fantasy_teams: args.teams,
                playoff_teams: args.playoff_teams,
                trials: args.trials,
                seed: args.seed,
                daily_injury_rate: injury_rate,
                weekly_trade_probability: trade_probability,
                weekly_pickup_limit: rules.weekly_acquisition_limit,
                user_proactive_pickup_reserve: pickup_reserve,
                user_exceptional_reserve_enabled: exceptional_reserve_enabled,
                user_exceptional_reserve_min_value: args.exceptional_reserve_min_value,
                user_exceptional_reserve_min_games: args.exceptional_reserve_min_games,
                opponent_pickup_accuracy,
                user_roster_player_keys: locked_user_roster.clone(),
                ..FantasySeasonSimConfig::default()
            },
        )
        .map_err(anyhow::Error::msg)?;

        if let Some(team) = selected_team.as_ref() {
            for row in &mut view.teams {
                if row.team == "Gio Simulation" {
                    row.team.clone_from(&team.name);
                }
            }
            for event in &mut view.sample_events {
                if event.team == "Gio Simulation" {
                    event.team.clone_from(&team.name);
                }
                event.message = rewrite_simulated_team_name(&event.message, &team.name);
            }
            view.assumptions.push(format!(
                "{} of {} player(s) from '{}' were resolved and locked; remaining roster spots use a legal synthetic fill",
                view.locked_user_roster.len(),
                selected_roster.len(),
                team.name
            ));
            if !unresolved_user_roster.is_empty() {
                view.warnings.push(format!(
                    "{} selected roster player(s) were unavailable in the stats/schedule pool: {}",
                    unresolved_user_roster.len(),
                    unresolved_user_roster.join(", ")
                ));
            }
            if selected_roster.len() < rules.standard_roster_capacity() {
                view.warnings.push(format!(
                    "'{}' is partial ({}/{} players); unlocked spots are synthetic",
                    team.name,
                    selected_roster.len(),
                    rules.standard_roster_capacity()
                ));
            }
        } else {
            view.warnings.push(
                "no --team was selected and no user team is marked; team one remains a neutral synthetic control"
                    .to_owned(),
            );
        }
        results.push(SeasonSimScenarioResult {
            scenario: scenario.to_owned(),
            view,
        });
    }

    if args.json {
        if args.scenario_matrix || args.manager_matrix || args.reserve_matrix {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&results[0].view)?);
        }
    } else if args.scenario_matrix || args.manager_matrix || args.reserve_matrix {
        print_season_scenario_matrix(&results, &target_team);
    } else {
        print_season_sim(&results[0].view);
    }
    Ok(())
}

fn rewrite_simulated_team_name(value: &str, actual_team: &str) -> String {
    value.replace("Gio Simulation", actual_team)
}

fn print_season_scenario_matrix(results: &[SeasonSimScenarioResult], target_team: &str) {
    let scenarios = results
        .iter()
        .map(|result| result.scenario.as_str())
        .collect::<Vec<_>>();
    let reference = matrix_reference_index(&scenarios).and_then(|index| results.get(index));
    let baseline_points = reference
        .and_then(|result| {
            result
                .view
                .teams
                .iter()
                .find(|team| team.team == target_team)
        })
        .map(|team| team.average_points)
        .unwrap_or(0.0);
    let reference_name = reference
        .map(|result| result.scenario.as_str())
        .unwrap_or("reference");
    println!("Fantasy season comparison matrix - {target_team}");
    println!("Point deltas use '{reference_name}' as the reference.");
    println!(
        "{:<12} {:>8} {:>8} {:>9} {:>10} {:>9} {:>13} {:>9} {:>9} {:>9}",
        "Scenario",
        "Injury",
        "Trades",
        "Opp acc",
        "Pts delta",
        "Avg seed",
        "Avg W-L-T",
        "No. 1",
        "Playoffs",
        "Champion"
    );
    for result in results {
        let Some(team) = result
            .view
            .teams
            .iter()
            .find(|team| team.team == target_team)
        else {
            continue;
        };
        println!(
            "{:<12} {:>8.4} {:>7.0}% {:>8.0}% {:>+10.1} {:>9.2} {:>4.1}-{:>4.1}-{:>3.1} {:>8.1}% {:>8.1}% {:>8.1}%",
            result.scenario,
            result.view.config.daily_injury_rate,
            result.view.config.weekly_trade_probability * 100.0,
            result.view.config.opponent_pickup_accuracy * 100.0,
            team.average_points - baseline_points,
            team.average_finish,
            team.average_wins,
            team.average_losses,
            team.average_ties,
            team.first_place_probability * 100.0,
            team.playoff_probability * 100.0,
            team.championship_probability * 100.0,
        );
    }
}

fn matrix_reference_index(scenarios: &[&str]) -> Option<usize> {
    scenarios
        .iter()
        .position(|scenario| *scenario == "baseline")
        .or_else(|| scenarios.iter().position(|scenario| *scenario == "parity"))
        .or_else(|| scenarios.iter().position(|scenario| *scenario == "all-in"))
        .or_else(|| (!scenarios.is_empty()).then_some(0))
}

fn print_season_sim(view: &FantasySeasonSimView) {
    println!(
        "Fantasy season stress test - {} / {} ({} trials, seed {})",
        view.season, view.scoring_scheme, view.config.trials, view.config.seed
    );
    println!(
        "{} to {} · {} regular weeks + {} playoff rounds · {} teams · injuries {:.4}/player-game · trade chance {:.0}%/team-week",
        view.start_date,
        view.end_date,
        view.regular_season_weeks,
        view.playoff_rounds,
        view.config.fantasy_teams,
        view.config.daily_injury_rate,
        view.config.weekly_trade_probability * 100.0
    );
    println!(
        "{:<5} {:<14} {:>10} {:>8} {:>7} {:>13} {:>9} {:>9} {:>9} {:>7} {:>7} {:>9} {:>10} {:>11}",
        "Rank",
        "Team",
        "Avg pts",
        "Avg seed",
        "No. 1",
        "Avg W-L-T",
        "Playoffs",
        "R1 exit",
        "Champion",
        "Adds",
        "Trades",
        "Injuries",
        "IR blocked",
        "Starts lost"
    );
    for team in &view.teams {
        println!(
            "{:<5} {:<14} {:>10.1} {:>8.2} {:>6.1}% {:>4.1}-{:>4.1}-{:>3.1} {:>8.1}% {:>8.1}% {:>8.1}% {:>7.1} {:>7.1} {:>9.1} {:>10.1} {:>11.1}",
            team.rank,
            team.team,
            team.average_points,
            team.average_finish,
            team.first_place_probability * 100.0,
            team.average_wins,
            team.average_losses,
            team.average_ties,
            team.playoff_probability * 100.0,
            team.first_round_exit_probability * 100.0,
            team.championship_probability * 100.0,
            team.average_adds,
            team.average_trades,
            team.average_injuries,
            team.average_injury_replacements_blocked,
            team.average_injury_starts_lost,
        );
    }
    println!("Sample trial events (first {}):", view.sample_events.len());
    for event in view.sample_events.iter().take(20) {
        let kind = match event.kind {
            FantasySeasonEventKind::Injury => "injury",
            FantasySeasonEventKind::Recovery => "recovery",
            FantasySeasonEventKind::AddDrop => "add/drop",
            FantasySeasonEventKind::InjuryReplacement => "IR replace",
            FantasySeasonEventKind::Trade => "trade",
        };
        println!(
            "  {} W{:02} {:<10} {:<10} {}",
            event.date, event.week, event.team, kind, event.message
        );
    }
    for warning in &view.warnings {
        println!("warning: {warning}");
    }
}

/// `icelines fantasy daily --date YYYY-MM-DD [--league <league>] [--json]`
pub async fn run_daily(
    date: NaiveDate,
    league_override: Option<String>,
    season: u32,
    season_type: SeasonType,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let data_root = icelines_data_root()?;
    let store = DataStore::open(&data_root).context("open DataStore")?;
    let view = build_fantasy_daily_delta_view(
        &db,
        &store,
        date,
        Season(season),
        season_type,
        league_override.as_deref(),
    )?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing fantasy daily delta")?
        );
        return Ok(());
    }

    print_daily(&view);
    Ok(())
}

fn print_daily(view: &FantasyDailyDeltaView) {
    println!(
        "Fantasy daily delta - {} / {} ({})",
        view.league, view.date, view.scoring_scheme
    );
    for warning in &view.warnings {
        println!("warning: {warning}");
    }
    println!(
        "{:<5} {:<24} {:<18} {:>10} {:>8} {:>8}",
        "Rank", "Team", "Owner", "Points", "Scored", "Missing"
    );
    for team in &view.teams {
        print_daily_team(team);
    }
}

fn print_daily_team(team: &FantasyDailyTeamRow) {
    println!(
        "{:<5} {:<24} {:<18} {:>10.1} {:>8} {:>8}",
        team.rank,
        if team.is_user_team {
            format!("{} (mine)", team.team)
        } else {
            team.team.clone()
        },
        team.owner,
        team.daily_points,
        team.scored_players,
        team.unscored_players
    );
    for player in &team.players {
        println!(
            "      {:<24} {:<4} {:>8.1} {:<12} {}",
            player.display_name,
            player.position,
            player.daily_points,
            daily_status_label(player.status),
            player.nhl_team.as_deref().unwrap_or("-")
        );
    }
}

fn daily_status_label(status: FantasyDailyPlayerStatus) -> &'static str {
    match status {
        FantasyDailyPlayerStatus::Scored => "scored",
        FantasyDailyPlayerStatus::NoFinalLine => "no-final",
        FantasyDailyPlayerStatus::Unfinalized => "unfinalized",
    }
}

pub async fn run_schedule_edge(args: ScheduleEdgeArgs) -> anyhow::Result<()> {
    if !(1..=16).contains(&args.off_night_max_games) {
        bail!("--off-night-max-games must be between 1 and 16");
    }
    if !(1..=16).contains(&args.classes) {
        bail!("--classes must be between 1 and 16");
    }
    let season = Season(args.season);
    let games = load_fantasy_schedule(season, args.refresh).await?;
    let roster_teams = if args.teams.is_empty() {
        resolve_user_roster_teams(season, &args.league)?
    } else {
        args.teams.clone()
    };
    let mut view = build_fantasy_schedule_view(
        games
            .into_iter()
            .filter(|game| game.game_type == 2)
            .map(|game| {
                let date = NaiveDate::parse_from_str(&game.date, "%Y-%m-%d")
                    .with_context(|| format!("invalid NHL schedule date '{}'", game.date))?;
                Ok(icelines_core::FantasyScheduleGameInput {
                    game_id: game.game_id,
                    date,
                    away_team: game.away_abbrev,
                    home_team: game.home_abbrev,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?,
        args.season,
        args.off_night_max_games,
        args.classes,
        roster_teams,
    )
    .map_err(anyhow::Error::msg)?;
    if let Some(date) = args.week {
        let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
        view.weeks.retain(|week| week.week_start == monday);
        if view.weeks.is_empty() {
            bail!("no regular-season games in the Monday-Sunday week of {monday}");
        }
    }

    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_schedule_edge(&view)
    };
    if let Some(path) = args.out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        std::fs::write(&path, output.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        println!("Wrote fantasy schedule report to {}", path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}

async fn load_fantasy_schedule(
    season: Season,
    refresh: bool,
) -> anyhow::Result<Vec<ScheduledGame>> {
    let root = default_data_root().ok_or_else(|| anyhow::anyhow!("cannot determine data root"))?;
    let store = DataStore::open(root)?;
    if !refresh {
        if let Ok(games) = store.load_schedule(season) {
            if !games.is_empty() {
                return Ok(games);
            }
        }
    }

    eprintln!(
        "icelines: loading all 32 official NHL team schedules for {}…",
        season.0
    );
    let client = NhlApiClient::production();
    let mut by_id = BTreeMap::<u64, ScheduledGame>::new();
    let mut failures = Vec::new();
    for (team, _) in icelines_core::CANONICAL_TEAMS {
        match client
            .fetch_team_season_schedule(team, &season.as_str())
            .await
        {
            Ok(games) => {
                for game in games.into_iter().filter(|game| game.game_type == 2) {
                    by_id.entry(game.game_id).or_insert(game);
                }
            }
            Err(error) => failures.push(format!("{team}: {error}")),
        }
    }
    if !failures.is_empty() {
        bail!(
            "official schedule load incomplete ({} of 32 teams failed): {}",
            failures.len(),
            failures.join("; ")
        );
    }
    let games = by_id.into_values().collect::<Vec<_>>();
    if games.is_empty() {
        bail!(
            "the NHL API returned no regular-season games for {}",
            season.0
        );
    }
    store.persist_schedule(season, &games)?;
    Ok(games)
}

fn resolve_user_roster_teams(
    season: Season,
    league_override: &Option<String>,
) -> anyhow::Result<Vec<String>> {
    let db = FantasyDb::open()?;
    let league = match require_league(&db, league_override) {
        Ok(league) => league,
        Err(_) if league_override.is_none() => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let Some(user_team) = db
        .list_teams(&league.id)?
        .into_iter()
        .find(|team| team.is_user_team)
    else {
        return Ok(Vec::new());
    };
    let roster = db.list_roster(&user_team.id)?;
    if roster.is_empty() {
        return Ok(Vec::new());
    }

    let player_team = load_current_player_team_map(season)?;
    let mut unresolved = Vec::new();
    let mut teams = Vec::new();
    for player in roster {
        if let Some(team) = player_team.get(&player) {
            teams.push(team.clone());
        } else {
            unresolved.push(player);
        }
    }
    if !unresolved.is_empty() {
        eprintln!(
            "warning: {} fantasy roster player(s) were not on a current NHL roster: {}",
            unresolved.len(),
            unresolved.join(", ")
        );
    }
    Ok(teams)
}

fn load_current_player_team_map(season: Season) -> anyhow::Result<HashMap<String, String>> {
    let cfg = crate::config::Config::load()?;
    let snapshots = SnapshotStore::new(cfg.snapshot_dir());
    let mut player_team = HashMap::new();
    for (team, _) in icelines_core::CANONICAL_TEAMS {
        let roster_snapshot = snapshots
            .read_tier_file_any_for_season::<RosterResponse>(
                &SnapshotTier::Rosters,
                &format!("{team}.json"),
                &season.as_str(),
            )
            .with_context(|| {
                format!(
                    "resolve current fantasy roster teams; run `icelines fetch rosters --season {} --refresh`",
                    season.0
                )
            })?;
        for player in roster_snapshot
            .forwards
            .iter()
            .chain(&roster_snapshot.defensemen)
            .chain(&roster_snapshot.goalies)
        {
            let full_name = format!(
                "{} {}",
                player.first_name.as_str(),
                player.last_name.as_str()
            );
            player_team.insert(normalize_name(&full_name), (*team).to_owned());
        }
    }
    Ok(player_team)
}

fn render_schedule_edge(view: &icelines_core::FantasyScheduleView) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{} — {}", BENCH_SCHEDULE_EDGE_HEADER, view.season);
    let _ = writeln!(
        out,
        "{} games · {} through {} · quiet slate ≤ {} games",
        view.game_count, view.season_start, view.season_end, view.off_night_max_games
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "SEASON OFF-NIGHT VALUE");
    let _ = writeln!(
        out,
        "{:<5} {:>5} {:>7} {:>9} {:>7}",
        "Team", "Games", "Quiet", "Scarcity", "Class"
    );
    for team in &view.teams {
        let _ = writeln!(
            out,
            "{:<5} {:>5} {:>7} {:>9.2} {:>7}",
            team.team,
            team.games,
            team.quiet_slate_games,
            team.scarcity_score,
            team.equivalence_class
        );
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "MONDAY-SUNDAY WEEKLY LEADERS");
    for week in &view.weeks {
        let max_games = week.teams.first().map_or(0, |team| team.games);
        let leaders = week
            .teams
            .iter()
            .filter(|team| team.games == max_games)
            .map(|team| team.team.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let max_quiet = week
            .teams
            .iter()
            .map(|team| team.quiet_slate_games)
            .max()
            .unwrap_or(0);
        let quiet = week
            .teams
            .iter()
            .filter(|team| max_quiet > 0 && team.quiet_slate_games == max_quiet)
            .map(|team| team.team.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(
            out,
            "{}  {:>2} NHL games  volume {}: {:<24} quiet {}: {}",
            week.week_start,
            week.league_games,
            max_games,
            leaders,
            max_quiet,
            if quiet.is_empty() { "-" } else { &quiet }
        );
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "SCHEDULE EQUIVALENCE CLASSES");
    for class in &view.equivalence_classes {
        let _ = writeln!(
            out,
            "Class {} ({:.1}% within overlap): {}",
            class.class_id,
            class.average_within_overlap_pct,
            class.teams.join(", ")
        );
    }

    if let Some(roster) = &view.roster {
        let _ = writeln!(out);
        let _ = writeln!(out, "MY ROSTER CALENDAR FIT");
        let team_counts = roster
            .teams
            .iter()
            .map(|team| {
                let count = roster.team_player_counts[team];
                if count > 1 {
                    format!("{team}×{count}")
                } else {
                    team.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(out, "Teams: {team_counts}");
        let _ = writeln!(
            out,
            "{} player schedule slots · {} team-games across {} distinct dates · {:.1}% date utilization · {} collision dates",
            roster.roster_player_slots,
            roster.total_team_games,
            roster.distinct_active_dates,
            roster.utilization_pct,
            roster.collision_dates
        );
        if !roster.highest_overlap_pairs.is_empty() {
            let _ = writeln!(out, "Highest-overlap pairs:");
            for pair in &roster.highest_overlap_pairs {
                let _ = writeln!(
                    out,
                    "  {} + {}: {:.1}% ({} shared dates)",
                    pair.team_a, pair.team_b, pair.overlap_pct, pair.shared_game_dates
                );
            }
        }
        let _ = writeln!(out, "Best low-overlap additions:");
        for team in &roster.best_complements {
            let _ = writeln!(
                out,
                "  {:<4} overlap {:>5.1}% · quiet {:>2} · class {}",
                team.team,
                team.average_roster_overlap_pct,
                team.quiet_slate_games,
                team.equivalence_class
            );
        }
    } else {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "No marked fantasy roster found. Use --teams NYR,COL,... or `fantasy team-use`."
        );
    }
    let _ = writeln!(out);
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    out
}

async fn remaining_games_by_team(season: Season) -> (HashMap<String, u32>, Option<String>) {
    let cache = remaining_games_by_team_from_cache(season);
    let mut remaining = cache.remaining_by_team;
    let complete_cache_teams = cache.complete_teams;
    let client = NhlApiClient::production();
    let season_string = season.0.to_string();
    let mut failures = 0usize;

    for (team, _) in icelines_core::CANONICAL_TEAMS {
        if complete_cache_teams.contains(*team) {
            continue;
        }
        match client
            .fetch_team_season_schedule(team, &season_string)
            .await
        {
            Ok(games) => {
                let count = games
                    .into_iter()
                    .filter(|game| game.game_type == 2 && !game.is_final())
                    .count() as u32;
                remaining.insert((*team).to_string(), count);
            }
            Err(_) => failures += 1,
        }
    }

    if remaining.is_empty() {
        (
            remaining,
            Some("could not fetch NHL team schedules; games remaining set to 0".to_string()),
        )
    } else if failures > 0 {
        (
            remaining,
            Some(format!(
                "could not fetch schedules for {failures} teams; affected roster games may be undercounted"
            )),
        )
    } else {
        (remaining, None)
    }
}

fn print_simulation(view: &FantasySimulationView) {
    println!(
        "Fantasy simulation - {} / {} ({:?})",
        view.league, view.scoring_scheme, view.horizon
    );
    for warning in &view.warnings {
        println!("warning: {warning}");
    }
    for assumption in &view.assumptions {
        println!("assumption: {assumption}");
    }
    println!(
        "{:<5} {:<24} {:<18} {:>10} {:>8} {:>8}",
        "Rank", "Team", "Owner", "Score", "Gap", "Players"
    );
    for row in &view.rows {
        println!(
            "{:<5} {:<24} {:<18} {:>10.1} {:>8.1} {:>8}",
            row.rank,
            if row.is_user_team {
                format!("{} (mine)", row.team)
            } else {
                row.team.clone()
            },
            row.owner,
            row.projected_score,
            row.score_gap_to_leader,
            row.rostered_players
        );
    }
    if !view.scenarios.is_empty() {
        println!();
        println!(
            "{:<10} {:<24} {:>10} {:<18} Explanation",
            "Action", "Scenario", "Delta", "Confidence"
        );
        for scenario in &view.scenarios {
            println!(
                "{:<10} {:<24} {:>10.1} {:<18} {}",
                format!("{:?}", scenario.action).to_ascii_lowercase(),
                scenario.label,
                scenario.projected_score_delta,
                format!("{:?}", scenario.confidence).to_ascii_lowercase(),
                scenario.explanation
            );
        }
    }
}

/// `icelines fantasy matchup --date YYYY-MM-DD [--league <league>] [--json]`
pub async fn run_matchup(
    date: NaiveDate,
    league_override: Option<String>,
    season: u32,
    season_type: SeasonType,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let data_root = icelines_data_root()?;
    let store = DataStore::open(&data_root).context("open DataStore")?;
    let view = build_fantasy_matchup_week_view(
        &db,
        &store,
        date,
        Season(season),
        season_type,
        league_override.as_deref(),
    )?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing fantasy matchup week")?
        );
        return Ok(());
    }

    print_matchup(&view);
    Ok(())
}

/// Build the points-mode pre-week matchup strategy war-room view.
pub async fn run_matchup_plan(
    week: NaiveDate,
    team_override: Option<String>,
    opponent_override: Option<String>,
    strategy: String,
    user_higher_seed: Option<bool>,
    category_snapshot_path: Option<PathBuf>,
    through: Option<NaiveDate>,
    user_current: Option<f64>,
    opponent_current: Option<f64>,
    current_source: String,
    status_max_age_minutes: i64,
    league_override: Option<String>,
    stats_season: String,
    candidate_limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    if candidate_limit == 0 {
        bail!("--candidates must be at least 1");
    }
    if status_max_age_minutes < 0 {
        bail!("--status-max-age-minutes cannot be negative");
    }
    let current_points = match (through, user_current, opponent_current) {
        (None, None, None) => None,
        (Some(through_date), Some(user_points), Some(opponent_points)) => {
            Some(FantasyMatchupPointsSnapshotInput {
                through_date,
                user_points,
                opponent_points,
                source: current_source,
            })
        }
        _ => bail!(
            "in-week planning requires --through, --user-current, and --opponent-current together"
        ),
    };
    let category_snapshot = category_snapshot_path
        .as_ref()
        .map(read_category_snapshot)
        .transpose()?;
    if current_points.is_some() && category_snapshot.is_some() {
        bail!("points totals and --category-snapshot cannot be supplied together");
    }
    let strategy = match strategy.to_ascii_lowercase().as_str() {
        "floor" => FantasyMatchupStrategy::Floor,
        "balanced" => FantasyMatchupStrategy::Balanced,
        "upside" => FantasyMatchupStrategy::Upside,
        other => bail!("unknown strategy '{other}'; expected floor, balanced, or upside"),
    };
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let competition_rules = db.get_competition_rules(&league.id)?;
    if competition_rules.mode == FantasyCompetitionMode::Categories && current_points.is_some() {
        bail!(
            "category matchup-to-date snapshots require per-category values; --user-current and --opponent-current are points-mode inputs"
        );
    }
    if competition_rules.mode == FantasyCompetitionMode::Points && category_snapshot.is_some() {
        bail!("--category-snapshot requires a league configured in category mode");
    }
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let user_team = if let Some(team) = team_override {
        require_team(&db, &league.id, &team)?
    } else {
        db.get_user_team(&league.id)?.ok_or_else(|| {
            anyhow::anyhow!(
                "no user team marked; pass --team or run `icelines fantasy team-use <name>`"
            )
        })?
    };
    let (week_start, week_end) = Timeframe::Week.range(week);
    let opponent = if let Some(name) = opponent_override {
        require_team(&db, &league.id, &name)?
    } else {
        let matchups = db.list_matchups(&league.id, week_start)?;
        let opponent_name = matchups.iter().find_map(|row| {
            if row.home_team == user_team.name {
                row.away_team.clone()
            } else if row.away_team.as_deref() == Some(user_team.name.as_str()) {
                Some(row.home_team.clone())
            } else {
                None
            }
        });
        let opponent_name = opponent_name.ok_or_else(|| {
            anyhow::anyhow!(
                "no saved opponent for {} in week {}; pass --opponent or run `icelines fantasy matchup-set`",
                user_team.name,
                week_start
            )
        })?;
        require_team(&db, &league.id, &opponent_name)?
    };
    if opponent.id == user_team.id {
        bail!("matchup-plan opponent must differ from the marked user team");
    }
    let user_roster = db.list_roster(&user_team.id)?;
    let opponent_roster = db.list_roster(&opponent.id)?;
    if user_roster.is_empty() || opponent_roster.is_empty() {
        bail!("matchup-plan requires non-empty saved rosters for both teams");
    }

    let observations = db.list_latest_status_observations(&league.id)?;
    let now = Utc::now();
    let league_timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let evaluation_date = now.with_timezone(&league_timezone).date_naive();
    let apply_status_evidence =
        evaluation_date >= week_start - Duration::days(1) && evaluation_date <= week_end;
    let roster_keys = user_roster
        .iter()
        .chain(&opponent_roster)
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut status_by_key = HashMap::new();
    let mut status_refresh_count = 0usize;
    let mut actionable_status_count = 0usize;
    for key in roster_keys {
        if !apply_status_evidence {
            status_by_key.insert(key, FantasyPlayerAvailabilityStatus::Healthy);
            continue;
        }
        let resolved =
            resolve_fantasy_player_status(key.clone(), &observations, now, status_max_age_minutes);
        if resolved.effective_status == FantasyPlayerAvailabilityStatus::Unknown {
            // Completed-season projections need a baseline when evidence is absent. Keep the
            // assumption visible and never convert missing evidence into a sourced health claim.
            status_by_key.insert(key, FantasyPlayerAvailabilityStatus::Healthy);
            status_refresh_count += 1;
        } else {
            if resolved.effective_status != FantasyPlayerAvailabilityStatus::Healthy {
                actionable_status_count += 1;
            }
            if resolved.requires_pregame_refresh {
                status_refresh_count += 1;
            }
            status_by_key.insert(key, resolved.effective_status);
        }
    }

    let scheme = resolve_scheme(&league.scheme)?;
    let (outcome, stats_window, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let (skaters, goalies) = pools_views(&outcome.repo, stats_window);
    let all_keys = skaters
        .iter()
        .chain(&goalies)
        .map(|view| view.identity.name_normalized.clone())
        .collect::<Vec<_>>();
    let scores = score_team(&all_keys, &skaters, &goalies, &scheme)
        .into_iter()
        .map(|(name, score)| (normalize_name(&name), f64::from(score)))
        .collect::<HashMap<_, _>>();
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    let current_teams = load_current_player_team_map(Season(CURRENT_SEASON)).ok();
    let pool = skaters
        .iter()
        .chain(&goalies)
        .map(|view| {
            let key = view.identity.name_normalized.clone();
            (
                key.clone(),
                DraftPoolPlayer {
                    key: key.clone(),
                    player: view.full_name().to_owned(),
                    team: current_teams
                        .as_ref()
                        .and_then(|teams| teams.get(&key))
                        .cloned()
                        .unwrap_or_else(|| view.team_display().to_owned()),
                    positions: eligibility
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| vec![view.position()]),
                    quality: scores.get(&key).copied().unwrap_or_default(),
                    games_played: view.gp(),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let schedule = load_fantasy_schedule(Season(CURRENT_SEASON), false).await?;
    let (team_dates, _) = draft_schedule_metrics(&schedule);
    let build_team = |team: &TeamRow, roster: &[String]| -> anyhow::Result<_> {
        let mut players = Vec::with_capacity(roster.len());
        for key in roster {
            let player = pool.get(key).with_context(|| {
                format!(
                    "'{}' on {} is absent from the {stats_season} stats pool",
                    key, team.name
                )
            })?;
            players.push(FantasyMatchupStrategyPlayerInput {
                player_key: player.key.clone(),
                player: player.player.clone(),
                nhl_team: player.team.clone(),
                positions: player.positions.clone(),
                projected_value_per_game: if player.games_played == 0 {
                    0.0
                } else {
                    player.quality / f64::from(player.games_played)
                },
                game_dates: team_dates.get(&player.team).cloned().unwrap_or_default(),
                status: status_by_key
                    .get(&player.key)
                    .copied()
                    .unwrap_or(FantasyPlayerAvailabilityStatus::Healthy),
            });
        }
        Ok(FantasyMatchupStrategyTeamInput {
            team: team.name.clone(),
            players,
        })
    };

    let mut warnings = vec![
        "remaining-game projection uses completed-season per-game rates".to_owned(),
        "starting-goalie confirmations are not yet applied".to_owned(),
    ];
    if current_points.is_none() && category_snapshot.is_none() {
        warnings.push(
            "no matchup-to-date totals supplied; report remains a pre-week projection".to_owned(),
        );
    }
    if actionable_status_count > 0 {
        warnings.push(format!(
            "{actionable_status_count} fresh saved non-healthy roster status(es) were applied"
        ));
    }
    if status_refresh_count > 0 {
        warnings.push(format!(
            "{status_refresh_count} roster status(es) lack definitive fresh evidence; baseline availability is assumed and a pregame refresh is required"
        ));
    }
    if !apply_status_evidence {
        warnings.push(format!(
            "saved injury/status evidence is not applied outside the current matchup window (league date {evaluation_date}); refresh it when this week approaches"
        ));
    }
    for (team, roster) in [(&user_team, &user_roster), (&opponent, &opponent_roster)] {
        if roster.len() < rules.standard_roster_capacity() {
            warnings.push(format!(
                "{} has a partial saved roster ({}/{})",
                team.name,
                roster.len(),
                rules.standard_roster_capacity()
            ));
        }
    }
    if competition_rules.mode == FantasyCompetitionMode::Categories {
        if category_snapshot.is_none() {
            warnings.push(
                "category matchup-to-date values are not supplied; this is a pre-week projection"
                    .to_owned(),
            );
        }
        let views_by_key = skaters
            .iter()
            .chain(&goalies)
            .map(|view| (view.identity.name_normalized.clone(), *view))
            .collect::<HashMap<_, _>>();
        for rule in &competition_rules.categories {
            if matches!(
                rule.key.as_str(),
                "hits" | "blocks" | "takeaways" | "giveaways"
            ) {
                let missing = user_roster
                    .iter()
                    .chain(&opponent_roster)
                    .filter_map(|key| views_by_key.get(key))
                    .filter(|view| !view.is_goalie() && view.stats.realtime.is_none())
                    .count();
                if missing > 0 {
                    warnings.push(format!(
                        "category '{}' is missing realtime evidence for {missing} rostered skater(s); their contribution is omitted, not asserted as zero",
                        rule.key
                    ));
                }
            }
        }
        let category_scopes = competition_rules
            .categories
            .iter()
            .map(|rule| Ok((rule.key.clone(), category_scope_for_rule(rule)?)))
            .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
        let build_category_team = |team: &TeamRow, roster: &[String]| -> anyhow::Result<_> {
            let mut players = Vec::with_capacity(roster.len());
            for key in roster {
                let pool_player = pool.get(key).with_context(|| {
                    format!(
                        "'{}' on {} is absent from the {stats_season} stats pool",
                        key, team.name
                    )
                })?;
                let player_view = views_by_key.get(key).with_context(|| {
                    format!("'{}' is missing its completed-season stats view", key)
                })?;
                let mut category_rates = BTreeMap::new();
                let mut lineup_priority = 0.0;
                for rule in &competition_rules.categories {
                    if let Some(rate) = category_rate_from_view(player_view, rule)? {
                        let raw = if rate.denominator_per_game > f64::EPSILON {
                            rate.numerator_per_game / rate.denominator_per_game
                        } else {
                            rate.numerator_per_game
                        };
                        lineup_priority += match rule.direction {
                            FantasyCategoryDirection::HigherWins => raw,
                            FantasyCategoryDirection::LowerWins => -raw,
                        };
                        category_rates.insert(rule.key.clone(), rate);
                    }
                }
                let appearance_probability = if player_view.is_goalie() {
                    player_view
                        .stats
                        .goalie
                        .as_ref()
                        .map(|goalie| f64::from(goalie.games_started) / 82.0)
                        .unwrap_or_default()
                        .clamp(0.0, 1.0)
                } else {
                    1.0
                };
                players.push(FantasyCategoryPlayerInput {
                    player_key: pool_player.key.clone(),
                    player: pool_player.player.clone(),
                    nhl_team: pool_player.team.clone(),
                    positions: pool_player.positions.clone(),
                    lineup_priority_per_game: lineup_priority * appearance_probability,
                    appearance_probability,
                    game_dates: team_dates
                        .get(&pool_player.team)
                        .cloned()
                        .unwrap_or_default(),
                    status: status_by_key
                        .get(&pool_player.key)
                        .copied()
                        .unwrap_or(FantasyPlayerAvailabilityStatus::Healthy),
                    category_rates,
                });
            }
            Ok(FantasyCategoryTeamInput {
                team: team.name.clone(),
                players,
            })
        };
        let category_user = build_category_team(&user_team, &user_roster)?;
        let category_opponent = build_category_team(&opponent, &opponent_roster)?;
        let view = build_fantasy_category_matchup(FantasyCategoryMatchupInput {
            league: league.name,
            week_start,
            week_end,
            rules: competition_rules,
            roster_rules: rules,
            strategy,
            user_is_higher_seed: user_higher_seed,
            category_scopes,
            user: category_user,
            opponent: category_opponent,
            current_snapshot: category_snapshot,
            warnings,
        })
        .map_err(anyhow::Error::msg)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&view)?);
        } else {
            print_category_matchup_plan(&view);
        }
        return Ok(());
    }
    let pickup = build_weekly_pickups_view(
        &db,
        &league,
        rules.clone(),
        Some(user_team.name.as_str()),
        Some(week_start.to_string()),
        stats_season.clone(),
        candidate_limit,
        1,
        None,
        false,
        true,
    )
    .await;
    let largest_legal_swing = match pickup {
        Ok((view, _, _)) => view.rows.first().map(|row| FantasyMatchupSwingInput {
            add_player_key: row.add_player_key.clone(),
            add_player: row.add_player.clone(),
            drop_player_key: row.drop_player_key.clone(),
            drop_player: row.drop_player.clone(),
            incremental_usable_starts: row.incremental_usable_starts,
            projected_value_delta: row.projected_value_delta,
            reasons: row.reasons.clone(),
        }),
        Err(error) => {
            warnings.push(format!("one-move swing unavailable: {error}"));
            None
        }
    };
    let view = build_fantasy_matchup_strategy(FantasyMatchupStrategyInput {
        league: league.name,
        scoring_scheme: league.scheme,
        week_start,
        week_end,
        strategy,
        rules,
        user: build_team(&user_team, &user_roster)?,
        opponent: build_team(&opponent, &opponent_roster)?,
        current_points,
        largest_legal_swing,
        warnings,
    })
    .map_err(anyhow::Error::msg)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        print_matchup_plan(&view);
    }
    Ok(())
}

fn print_matchup_plan(view: &FantasyMatchupStrategyView) {
    println!(
        "{} — {} · {} through {} · {:?} · {}",
        FACEOFF_MATCHUP_HEADER,
        view.league,
        view.week_start,
        view.week_end,
        view.strategy,
        view.matchup_state
    );
    if let Some(through) = view.current_through_date {
        println!(
            "  Current totals through {} · source: {}",
            through,
            view.current_totals_source
                .as_deref()
                .unwrap_or("unspecified")
        );
    }
    println!(
        "  {:<24} {:>9} {:>10} {:>10} {:>10} {:>8} {:>8} {:>10}",
        "Team", "Current", "Remaining", "Expected", "Floor", "Starts", "Benched", "Lost value"
    );
    for team in [&view.user, &view.opponent] {
        println!(
            "  {:<24} {:>9.1} {:>10.1} {:>10.1} {:>10.1} {:>8} {:>8} {:>10.1}",
            team.team,
            team.current_points,
            team.remaining_projected_points,
            team.projected_points,
            team.floor_points,
            team.usable_starts,
            team.benched_player_games,
            team.bench_collision_value
        );
    }
    println!(
        "  Margin: expected {:+.1}, downside {:+.1}, upside {:+.1} · modeled win {:.1}%",
        view.expected_margin,
        view.downside_margin,
        view.upside_margin,
        view.modeled_win_probability * 100.0
    );
    if let Some(swing) = &view.largest_legal_swing {
        println!(
            "  Best move: add {} / drop {} · {:+.1} starts · {:+.1} value",
            swing.add_player,
            swing.drop_player,
            swing.incremental_usable_starts,
            swing.projected_value_delta
        );
    }
    println!("  Recommendation: {}", view.recommendation);
    for warning in &view.warnings {
        println!("  warning: {warning}");
    }
    for note in &view.model_notes {
        println!("  model: {note}");
    }
}

fn print_category_matchup_plan(view: &FantasyCategoryMatchupView) {
    println!(
        "{} — {} · {} through {} · {:?} · {}",
        FACEOFF_CATEGORY_MATCHUP_HEADER,
        view.league,
        view.week_start,
        view.week_end,
        view.strategy,
        view.matchup_state
    );
    if let Some(through) = view.current_through_date {
        println!(
            "  Current components through {} · source: {}",
            through,
            view.current_totals_source
                .as_deref()
                .unwrap_or("unspecified")
        );
    }
    println!(
        "  {} vs {} · projected {}-{}-{} · {:?} · modeled matchup win {:.1}%",
        view.user_team,
        view.opponent_team,
        view.projected_category_wins,
        view.projected_category_losses,
        view.projected_category_ties,
        view.projected_matchup_result,
        view.modeled_matchup_win_probability * 100.0
    );
    if view.minimum_goalie_appearances > 0 {
        println!(
            "  Goalie appearances: {:.1} vs {:.1} · minimum {} · met: {} / {}",
            view.user_goalie_appearances,
            view.opponent_goalie_appearances,
            view.minimum_goalie_appearances,
            view.user_meets_goalie_minimum,
            view.opponent_meets_goalie_minimum
        );
    }
    println!(
        "  {:<22} {:>22} {:>22} {:>10} {:>9} {:>10}",
        "Category", "You C+R=Final", "Opp C+R=Final", "Result", "Win %", "Class"
    );
    for row in &view.categories {
        let format_value = |value: Option<f64>| {
            value
                .map(|value| format!("{value:.3}"))
                .unwrap_or_else(|| "—".to_owned())
        };
        let triplet = |current: Option<f64>, remaining: Option<f64>, total: Option<f64>| {
            format!(
                "{}+{}={}",
                format_value(current),
                format_value(remaining),
                format_value(total)
            )
        };
        println!(
            "  {:<22} {:>22} {:>22} {:>10?} {:>8.1}% {:>10?}",
            row.label,
            triplet(
                row.user_current.value,
                row.user_remaining.value,
                row.user.value
            ),
            triplet(
                row.opponent_current.value,
                row.opponent_remaining.value,
                row.opponent.value
            ),
            row.projected_result,
            row.user_win_probability * 100.0,
            row.classification
        );
    }
    println!("  Recommendation: {}", view.recommendation);
    for warning in &view.warnings {
        println!("  warning: {warning}");
    }
    for note in &view.model_notes {
        println!("  model: {note}");
    }
}

fn category_scope_for_rule(rule: &FantasyCategoryRule) -> anyhow::Result<FantasyCategoryScope> {
    let scope = match rule.key.as_str() {
        "goals" | "assists" | "points" | "plus_minus" | "shots" | "hits" | "blocks"
        | "pp_goals" | "pp_assists" | "sh_goals" | "sh_assists" | "gwg" | "ot_goals"
        | "takeaways" | "giveaways" => FantasyCategoryScope::Skater,
        "wins"
        | "losses"
        | "saves"
        | "goals_against"
        | "shutouts"
        | "save_percentage"
        | "goals_against_average" => FantasyCategoryScope::Goalie,
        other => {
            bail!("category '{other}' is not available from the completed-season stats contract")
        }
    };
    let expected_aggregation = match rule.key.as_str() {
        "save_percentage" | "goals_against_average" => FantasyCategoryAggregation::Ratio,
        _ => FantasyCategoryAggregation::Sum,
    };
    if rule.aggregation != expected_aggregation {
        bail!(
            "category '{}' requires {:?} aggregation, not {:?}",
            rule.key,
            expected_aggregation,
            rule.aggregation
        );
    }
    Ok(scope)
}

fn category_rate_from_view(
    player: &PlayerView<'_>,
    rule: &FantasyCategoryRule,
) -> anyhow::Result<Option<FantasyCategoryRateInput>> {
    let scope = category_scope_for_rule(rule)?;
    if (scope == FantasyCategoryScope::Goalie) != player.is_goalie() {
        return Ok(None);
    }
    if player.is_goalie() {
        let Some(goalie) = player.stats.goalie.as_ref() else {
            return Ok(None);
        };
        let starts = f64::from(goalie.games_started);
        if starts <= f64::EPSILON {
            return Ok(None);
        }
        let (numerator, denominator) = match rule.key.as_str() {
            "wins" => (f64::from(goalie.wins) / starts, 0.0),
            "losses" => (f64::from(goalie.losses) / starts, 0.0),
            "saves" => (f64::from(goalie.saves) / starts, 0.0),
            "goals_against" => (f64::from(goalie.goals_against) / starts, 0.0),
            "shutouts" => (f64::from(goalie.shutouts) / starts, 0.0),
            "save_percentage" => (
                f64::from(goalie.saves) / starts,
                f64::from(goalie.shots_against) / starts,
            ),
            "goals_against_average" => (
                f64::from(goalie.goals_against) / starts,
                f64::from(goalie.time_on_ice_sec) / 3_600.0 / starts,
            ),
            _ => unreachable!("goalie category scope validated"),
        };
        return Ok(Some(FantasyCategoryRateInput {
            numerator_per_game: numerator,
            denominator_per_game: denominator,
        }));
    }

    let gp = f64::from(player.gp());
    if gp <= f64::EPSILON {
        return Ok(None);
    }
    let totals = &player.stats.totals;
    let numerator = match rule.key.as_str() {
        "goals" => f64::from(totals.goals),
        "assists" => f64::from(totals.assists),
        "points" => f64::from(totals.points),
        "plus_minus" => f64::from(totals.plus_minus),
        "shots" => f64::from(totals.shots),
        "hits" => match player.hits() {
            Some(value) => f64::from(value),
            None => return Ok(None),
        },
        "blocks" => match player.blocked_shots() {
            Some(value) => f64::from(value),
            None => return Ok(None),
        },
        "pp_goals" => f64::from(totals.pp_goals),
        "pp_assists" => f64::from(totals.pp_points.saturating_sub(totals.pp_goals)),
        "sh_goals" => f64::from(totals.sh_goals),
        "sh_assists" => f64::from(totals.sh_points.saturating_sub(totals.sh_goals)),
        "gwg" => f64::from(totals.gwg),
        "ot_goals" => f64::from(totals.ot_goals),
        "takeaways" => match player.takeaways() {
            Some(value) => f64::from(value),
            None => return Ok(None),
        },
        "giveaways" => match player.giveaways() {
            Some(value) => f64::from(value),
            None => return Ok(None),
        },
        _ => unreachable!("skater category scope validated"),
    };
    Ok(Some(FantasyCategoryRateInput {
        numerator_per_game: numerator / gp,
        denominator_per_game: 0.0,
    }))
}

/// `icelines fantasy matchup-set --week YYYY-MM-DD --home A [--away B]`
pub async fn run_matchup_set(
    week: NaiveDate,
    home: String,
    away: Option<String>,
    league_override: Option<String>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let (week_start, week_end) = Timeframe::Week.range(week);
    let id = db.schedule_matchup(&league.id, week_start, &home, away.as_deref())?;
    println!(
        "Scheduled fantasy matchup {} for {} through {}: {}{}",
        id,
        week_start,
        week_end,
        home,
        away.as_ref()
            .map(|opponent| format!(" vs {opponent}"))
            .unwrap_or_else(|| " (bye)".to_string())
    );
    Ok(())
}

fn print_matchup(view: &FantasyMatchupWeekView) {
    println!(
        "Fantasy matchups - {} / {} to {} ({})",
        view.league, view.week_start, view.week_end, view.scoring_scheme
    );
    for warning in &view.warnings {
        println!("warning: {warning}");
    }
    if let Some(empty) = &view.empty_state {
        println!("{}: {}", empty.title, empty.detail.as_deref().unwrap_or(""));
        return;
    }
    println!(
        "{:<5} {:<24} {:>10} {:<10} {:<24} {:>10} {:<10}",
        "Rank", "Home", "Points", "Result", "Away", "Points", "Result"
    );
    for matchup in &view.matchups {
        let away = matchup.away.as_ref();
        println!(
            "{:<5} {:<24} {:>10} {:<10} {:<24} {:>10} {:<10}",
            matchup.rank,
            matchup_side_label(&matchup.home),
            matchup_points_label(matchup.home.weekly_points),
            matchup_outcome_label(matchup.home.outcome),
            away.map(matchup_side_label)
                .unwrap_or_else(|| "-".to_string()),
            away.map(|side| matchup_points_label(side.weekly_points))
                .unwrap_or_else(|| "-".to_string()),
            away.map(|side| matchup_outcome_label(side.outcome))
                .unwrap_or("-")
        );
    }
}

fn matchup_side_label(side: &FantasyMatchupSideRow) -> String {
    if side.is_user_team {
        format!("{} (mine)", side.team)
    } else {
        side.team.clone()
    }
}

fn matchup_points_label(points: Option<f32>) -> String {
    points
        .map(|points| format!("{points:.1}"))
        .unwrap_or_else(|| "pending".to_string())
}

fn matchup_outcome_label(outcome: FantasyMatchupOutcome) -> &'static str {
    match outcome {
        FantasyMatchupOutcome::Win => "win",
        FantasyMatchupOutcome::Loss => "loss",
        FantasyMatchupOutcome::Tie => "tie",
        FantasyMatchupOutcome::Bye => "bye",
        FantasyMatchupOutcome::Pending => "pending",
    }
}

/// `icelines fantasy import-yahoo --file rosters.csv --league "My League" [--my-team "Me"] [--dry-run] [--json]`
pub async fn run_import_yahoo(
    file: PathBuf,
    league: String,
    my_team: Option<String>,
    dry_run: bool,
    replace: bool,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let known_player_positions = known_player_positions()?;
    let known_player_keys = known_player_positions.keys().cloned().collect();
    let mut options = if dry_run {
        FantasyRosterImportOptions::dry_run(league)
    } else {
        FantasyRosterImportOptions::apply(league)
    };
    options.user_team = my_team;
    options.known_player_keys = Some(known_player_keys);
    options.known_player_positions = Some(known_player_positions);
    options.replace_rosters = replace;

    let view = if file.as_os_str() == "-" {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .context("read Yahoo roster CSV from stdin")?;
        let rows = parse_yahoo_roster_csv_text(&input, "stdin")?;
        import_yahoo_roster_rows(&db, rows, options)
            .context("import Yahoo roster CSV from stdin")?
    } else {
        import_yahoo_roster_csv(&db, &file, options)
            .with_context(|| format!("import Yahoo roster CSV from {}", file.display()))?
    };

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing fantasy import view")?
        );
        return Ok(());
    }

    print_import_yahoo(&view);
    Ok(())
}

fn known_player_positions() -> anyhow::Result<BTreeMap<String, Vec<Position>>> {
    let (outcome, season) = load_pools()?;
    let (skaters, goalies) = pools_views(&outcome.repo, season);
    Ok(skaters
        .iter()
        .chain(goalies.iter())
        .map(|view| (view.identity.name_normalized.clone(), vec![view.position()]))
        .collect())
}

/// `icelines fantasy roster-shape [--league <league>] [--json]`
pub async fn run_roster_shape_show(
    league_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = if league_override.is_some() || db.get_active_league()?.is_some() {
        Some(require_league(&db, &league_override)?)
    } else {
        None
    };
    let shapes = RosterShape::all_builtins();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "league": league.as_ref().map(|league| league.name.clone()),
                "roster_shape": league.as_ref().map(|league| league.roster_shape.clone()),
                "available_shapes": shapes,
            }))
            .context("serializing roster shape summary")?
        );
        return Ok(());
    }

    if let Some(league) = &league {
        println!(
            "League: {} | roster shape: {}",
            league.name, league.roster_shape
        );
    } else {
        println!("No active fantasy league; available roster shapes:");
    }
    println!("{:<18} Description", "Shape");
    println!("{}", "-".repeat(64));
    for shape in shapes {
        println!("{:<18} {}", shape.name, shape.description);
    }
    Ok(())
}

/// `icelines fantasy roster-shape-set <shape> [--league <league>]`
pub async fn run_roster_shape_set(
    shape_name: String,
    league_override: Option<String>,
) -> anyhow::Result<()> {
    resolve_roster_shape(&shape_name)?;
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    db.set_league_roster_shape(&league.id, &shape_name)?;
    println!(
        "League '{}' roster shape set to '{}'.",
        league.name, shape_name
    );
    Ok(())
}

pub async fn run_assistant_setup(
    league_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = FantasyAssistantRules::configured_2026();
    db.set_assistant_rules(&league.id, &rules)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "league": league.name,
                "persisted": true,
                "rules": rules,
            }))?
        );
    } else {
        println!("Fantasy assistant configured for '{}'.", league.name);
        print_assistant_rules(&rules);
    }
    Ok(())
}

pub async fn run_assistant_rules(
    league_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let persisted = db.get_assistant_rules(&league.id)?;
    let rules = persisted
        .clone()
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "league": league.name,
                "persisted": persisted.is_some(),
                "rules": rules,
            }))?
        );
    } else {
        println!(
            "League: {} | assistant rules: {}",
            league.name,
            if persisted.is_some() {
                "persisted"
            } else {
                "default preview"
            }
        );
        print_assistant_rules(&rules);
        if persisted.is_none() {
            println!("Run `icelines fantasy assistant-setup` to persist these rules.");
        }
    }
    Ok(())
}

pub async fn run_weekly_budget(
    league_override: Option<String>,
    at: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let now = parse_fantasy_effective_at(at.as_deref())?;
    let budget = load_week_budget(&db, &league.id, &rules, now)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&budget)?);
    } else {
        println!(
            "WEEKLY ACQUISITIONS — {} through {} ({})",
            budget.week_start, budget.week_end, budget.timezone
        );
        println!(
            "Used {} of {} · {} remaining · {}",
            budget.acquisitions_used,
            budget.acquisition_limit,
            budget.acquisitions_remaining,
            if budget.can_add {
                "add available"
            } else {
                "limit reached"
            }
        );
        if budget.injury_reserve_active > 0 {
            println!(
                "Safe proactive budget: {} · {} held for injury until {}",
                budget.proactive_acquisitions_remaining,
                budget.injury_reserve_active,
                budget
                    .injury_reserve_releases_on
                    .map(|date| date.to_string())
                    .unwrap_or_else(|| "release day".to_owned())
            );
        }
    }
    Ok(())
}

pub async fn run_acquisition_record(
    add: String,
    drop: Option<String>,
    kind: String,
    at: Option<String>,
    league_override: Option<String>,
    no_count: bool,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let effective_at = parse_fantasy_effective_at(at.as_deref())?;
    let acquisition_kind = match kind.trim().to_ascii_lowercase().as_str() {
        "free-agent" | "free_agent" | "fa" => FantasyAcquisitionKind::FreeAgentAdd,
        "waiver" | "waiver-claim" | "claim" => FantasyAcquisitionKind::WaiverClaim,
        _ => bail!("unknown acquisition kind '{kind}'; use free-agent or waiver"),
    };
    let counts_toward_limit = !no_count;
    let before = load_week_budget(&db, &league.id, &rules, effective_at)?;
    if counts_toward_limit && !before.can_add {
        bail!(
            "weekly acquisition limit reached ({} of {} used for {} through {}); event was not recorded",
            before.acquisitions_used,
            before.acquisition_limit,
            before.week_start,
            before.week_end
        );
    }
    let player_added = normalize_name(&add);
    if player_added.is_empty() {
        bail!("--add requires a player name");
    }
    let player_dropped = drop.as_deref().map(normalize_name);
    if player_dropped.as_deref() == Some(player_added.as_str()) {
        bail!("the added and dropped player cannot be the same");
    }
    let id = db.record_acquisition(
        &league.id,
        &player_added,
        player_dropped.as_deref(),
        acquisition_kind,
        effective_at,
        counts_toward_limit,
        rules.waiver_days,
    )?;
    let after = load_week_budget(&db, &league.id, &rules, effective_at)?;
    let waiver = match player_dropped.as_deref() {
        Some(player) => db.get_waiver(&league.id, player)?,
        None => None,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "league": league.name,
                "player_added": player_added,
                "player_dropped": player_dropped,
                "kind": acquisition_kind,
                "effective_at": effective_at,
                "counts_toward_limit": counts_toward_limit,
                "budget": after,
                "waiver": waiver,
            }))?
        );
    } else {
        println!("Recorded {:?}: {}", acquisition_kind, add);
        if let (Some(drop), Some(waiver)) = (drop, waiver) {
            println!("Dropped {drop}; waivers clear {}", waiver.clears_at);
        }
        println!(
            "Weekly budget: {} of {} used ({} remaining)",
            after.acquisitions_used, after.acquisition_limit, after.acquisitions_remaining
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run_status_record(
    player: String,
    status: String,
    source: String,
    source_url: Option<String>,
    observed_at: Option<String>,
    confidence: String,
    detail: Option<String>,
    league_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let player_key = normalize_name(&player);
    if player_key.is_empty() || source.trim().is_empty() {
        bail!("player and --source are required");
    }
    let observation = FantasyStatusObservation {
        player_key,
        status: parse_fantasy_status(&status)?,
        source,
        source_url,
        observed_at: parse_fantasy_effective_at(observed_at.as_deref())?,
        fetched_at: Utc::now(),
        confidence: parse_fantasy_confidence(&confidence)?,
        detail,
    };
    let id = db.record_status_observation(&league.id, &observation)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id,
                "league": league.name,
                "observation": observation,
            }))?
        );
    } else {
        println!(
            "Recorded {:?} for {} from {} at {} ({:?}).",
            observation.status,
            player,
            observation.source,
            observation.observed_at,
            observation.confidence
        );
    }
    Ok(())
}

pub async fn run_status_show(
    player: Option<String>,
    league_override: Option<String>,
    max_age_minutes: i64,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let observations = db.list_latest_status_observations(&league.id)?;
    let keys = if let Some(player) = player {
        vec![normalize_name(&player)]
    } else if let Some(team) = db.get_user_team(&league.id)? {
        db.list_roster(&team.id)?
    } else {
        observations
            .iter()
            .map(|observation| observation.player_key.clone())
            .collect()
    };
    let now = Utc::now();
    let rows = keys
        .into_iter()
        .map(|key| resolve_fantasy_player_status(key, &observations, now, max_age_minutes.max(0)))
        .collect::<Vec<_>>();
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!("AVAILABILITY STATUS — {}", league.name);
        println!(
            "{:<28} {:<12} {:<12} {:<11} Source",
            "Player", "Effective", "Reported", "Freshness"
        );
        for row in &rows {
            println!(
                "{:<28} {:<12?} {:<12?} {:<11?} {}{}",
                row.player_key,
                row.effective_status,
                row.reported_status,
                row.freshness,
                row.source.as_deref().unwrap_or("-"),
                if row.requires_pregame_refresh {
                    " · refresh"
                } else {
                    ""
                }
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run_goalie_start_record(
    player: String,
    date: String,
    state: String,
    source: String,
    source_url: Option<String>,
    observed_at: Option<String>,
    detail: Option<String>,
    league_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let game_date = parse_fantasy_date(&date)?;
    let observation = FantasyGoalieStartObservation {
        player_key: normalize_name(&player),
        game_date,
        state: parse_goalie_start_state_cli(&state)?,
        source,
        source_url,
        observed_at: parse_fantasy_effective_at(observed_at.as_deref())?,
        fetched_at: Utc::now(),
        detail,
    };
    let id = db.record_goalie_start_observation(&league.id, &observation)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": id, "league": league.name, "observation": observation,
            }))?
        );
    } else {
        println!(
            "Recorded {:?} for {} on {} from {}.",
            observation.state, player, game_date, observation.source
        );
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct GoalieStartImportCsvRow {
    player: String,
    date: String,
    state: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    observed_at: Option<String>,
    #[serde(default)]
    detail: Option<String>,
}

pub async fn run_goalie_start_import(
    file: PathBuf,
    source_fallback: Option<String>,
    observed_at_fallback: Option<String>,
    league_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let text = read_taken_player_input(Some(&file))?;
    if text.trim().is_empty() {
        bail!("goalie starter import is empty");
    }
    let fallback_time = parse_fantasy_effective_at(observed_at_fallback.as_deref())?;
    let fetched_at = Utc::now();
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(text.as_bytes());
    let mut observations = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, row) in reader.deserialize::<GoalieStartImportCsvRow>().enumerate() {
        let row = row.with_context(|| format!("parse goalie starter CSV row {}", index + 2))?;
        let player_key = normalize_name(&row.player);
        let game_date = parse_fantasy_date(&row.date)
            .with_context(|| format!("goalie starter CSV row {}", index + 2))?;
        if player_key.is_empty() {
            bail!("goalie starter CSV row {} has an empty player", index + 2);
        }
        if !seen.insert((player_key.clone(), game_date)) {
            bail!(
                "goalie starter CSV repeats '{}' on {}; keep exactly one latest row per import",
                row.player,
                game_date
            );
        }
        let source = row
            .source
            .filter(|value| !value.trim().is_empty())
            .or_else(|| source_fallback.clone())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "goalie starter CSV row {} requires source or --source",
                    index + 2
                )
            })?;
        let observed_at = row
            .observed_at
            .as_deref()
            .map(|value| parse_fantasy_effective_at(Some(value)))
            .transpose()?
            .unwrap_or(fallback_time);
        observations.push(FantasyGoalieStartObservation {
            player_key,
            game_date,
            state: parse_goalie_start_state_cli(&row.state)
                .with_context(|| format!("goalie starter CSV row {}", index + 2))?,
            source,
            source_url: row.source_url,
            observed_at,
            fetched_at,
            detail: row.detail,
        });
    }
    if observations.is_empty() {
        bail!("goalie starter import has a header but no data rows");
    }
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let ids = db.record_goalie_start_observations(&league.id, &observations)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "league": league.name,
                "imported": observations.len(),
                "ids": ids,
                "observations": observations,
            }))?
        );
    } else {
        println!(
            "Imported {} goalie starter observation(s) into {}.",
            observations.len(),
            league.name
        );
        for observation in &observations {
            println!(
                "{} {:<26} {:?} · {}",
                observation.game_date,
                observation.player_key,
                observation.state,
                observation.source
            );
        }
    }
    Ok(())
}

pub async fn run_goalie_start_template(
    date: Option<String>,
    team_override: Option<String>,
    league_override: Option<String>,
    stats_season: String,
    top_streams: usize,
    out: Option<PathBuf>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let evaluated_at = Utc::now();
    let checklist_date = date
        .as_deref()
        .map(parse_fantasy_date)
        .transpose()?
        .unwrap_or_else(|| evaluated_at.with_timezone(&timezone).date_naive());
    let team = if let Some(name) = team_override {
        require_team(&db, &league.id, &name)?
    } else {
        db.get_user_team(&league.id)?.ok_or_else(|| {
            anyhow::anyhow!("no user team marked; run `icelines fantasy team-use <name>`")
        })?
    };
    let (week_start, week_end) = Timeframe::Week.range(checklist_date);
    let plan = build_goalie_plan_view_for_context(
        &db,
        &league,
        &team,
        rules,
        week_start,
        week_end,
        Some(checklist_date),
        &stats_season,
        FantasyMatchupStrategy::Balanced,
        0.0,
        360,
        evaluated_at,
        evaluated_at,
    )
    .await?;
    let latest_states = db
        .list_latest_goalie_start_observations(&league.id, checklist_date, checklist_date)?
        .into_iter()
        .map(|observation| (observation.player_key, observation.state))
        .collect::<BTreeMap<_, _>>();
    let mut checklist = BTreeMap::<String, (String, FantasyGoalieStartState)>::new();
    for row in plan.rows.iter().filter(|row| row.date == checklist_date) {
        checklist.insert(
            row.player_key.clone(),
            (row.player.clone(), row.evidence.reported_state),
        );
    }
    for candidate in plan
        .stream_candidates
        .iter()
        .filter(|candidate| {
            candidate.acquisition_eligible && candidate.game_dates.contains(&checklist_date)
        })
        .take(top_streams)
    {
        let state = latest_states
            .get(&candidate.player_key)
            .copied()
            .unwrap_or(FantasyGoalieStartState::Unknown);
        checklist
            .entry(candidate.player_key.clone())
            .or_insert_with(|| (candidate.player.clone(), state));
    }
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "player",
        "date",
        "state",
        "source",
        "source_url",
        "observed_at",
        "detail",
    ])?;
    for (_, (player, state)) in checklist {
        writer.write_record([
            player,
            checklist_date.to_string(),
            goalie_start_state_cli_label(state).to_owned(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ])?;
    }
    let csv = String::from_utf8(writer.into_inner()?)?;
    if out
        .as_ref()
        .is_none_or(|path| path.to_string_lossy() == "-")
    {
        print!("{csv}");
    } else if let Some(path) = out {
        std::fs::write(&path, csv).with_context(|| format!("write {}", path.display()))?;
        println!("Wrote goalie starter checklist to {}.", path.display());
    }
    Ok(())
}

pub async fn run_goalie_start_show(
    player: Option<String>,
    week: Option<String>,
    date: Option<String>,
    league_override: Option<String>,
    max_age_minutes: i64,
    json: bool,
) -> anyhow::Result<()> {
    if max_age_minutes < 0 {
        bail!("--max-age-minutes cannot be negative");
    }
    let focus = date.as_deref().map(parse_fantasy_date).transpose()?;
    let anchor = week
        .as_deref()
        .map(parse_fantasy_date)
        .transpose()?
        .or(focus)
        .unwrap_or_else(|| Utc::now().date_naive());
    let (from, through) = if let Some(date) = focus {
        (date, date)
    } else {
        Timeframe::Week.range(anchor)
    };
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let observations = db.list_latest_goalie_start_observations(&league.id, from, through)?;
    let player_key = player.map(|value| normalize_name(&value));
    let now = Utc::now();
    let rows = if let (Some(key), Some(date)) = (player_key.as_ref(), focus) {
        vec![resolve_fantasy_goalie_start(
            key.clone(),
            date,
            &observations,
            now,
            max_age_minutes,
        )]
    } else {
        observations
            .iter()
            .filter(|row| player_key.as_ref().is_none_or(|key| &row.player_key == key))
            .map(|row| {
                resolve_fantasy_goalie_start(
                    row.player_key.clone(),
                    row.game_date,
                    &observations,
                    now,
                    max_age_minutes,
                )
            })
            .collect::<Vec<_>>()
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        println!(
            "{} — {} — {} to {}",
            CREASE_STARTER_EVIDENCE_HEADER, league.name, from, through
        );
        for row in &rows {
            println!(
                "{} {:<26} reported={:?} effective={:?} {:?}{}",
                row.game_date,
                row.player_key,
                row.reported_state,
                row.effective_state,
                row.freshness,
                if row.requires_refresh {
                    " · refresh"
                } else {
                    ""
                }
            );
        }
        if rows.is_empty() {
            println!("No goalie starter observations recorded for this range.");
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn build_goalie_plan_view_for_context(
    db: &FantasyDb,
    league: &LeagueRow,
    team: &TeamRow,
    assistant_rules: FantasyAssistantRules,
    week_start: NaiveDate,
    week_end: NaiveDate,
    focus_date: Option<NaiveDate>,
    stats_season: &str,
    strategy: FantasyMatchupStrategy,
    current_appearances: f64,
    max_age_minutes: i64,
    evaluated_at: DateTime<Utc>,
    budget_at: DateTime<Utc>,
) -> anyhow::Result<icelines_core::FantasyGoaliePlanView> {
    if max_age_minutes < 0 {
        bail!("--max-age-minutes cannot be negative");
    }
    let roster = db
        .list_roster(&team.id)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let all_rostered = db
        .list_teams(&league.id)?
        .into_iter()
        .map(|team| db.list_roster(&team.id))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let competition = db.get_competition_rules(&league.id)?;
    let goalie_slots = assistant_rules
        .active_slots
        .get(&FantasyActiveSlotKind::Goalie)
        .copied()
        .unwrap_or(0);
    let scheme = resolve_scheme(&league.scheme)?;
    let (outcome, stats_window, _) =
        crate::commands::players::load_repo_for_season(Some(stats_season), None)?;
    let (skater_views, goalie_views) = pools_views(&outcome.repo, stats_window);
    let current_teams = load_current_player_team_map(Season(CURRENT_SEASON)).ok();
    let schedule = load_fantasy_schedule(Season(CURRENT_SEASON), false).await?;
    let mut offense_by_team = HashMap::<String, f64>::new();
    for view in &skater_views {
        if view.gp() == 0 {
            continue;
        }
        let key = view.name_normalized();
        let nhl_team = current_teams
            .as_ref()
            .and_then(|teams| teams.get(key))
            .cloned()
            .unwrap_or_else(|| view.team_display().to_owned());
        *offense_by_team.entry(nhl_team).or_default() +=
            f64::from(view.stats.totals.goals) / f64::from(view.gp());
    }
    let offense_sum = offense_by_team.values().sum::<f64>();
    let league_offense = if offense_by_team.is_empty() || offense_sum <= f64::EPSILON {
        1.0
    } else {
        offense_sum / offense_by_team.len() as f64
    };
    let offense_index = offense_by_team
        .into_iter()
        .map(|(team, offense)| (team, (offense / league_offense).clamp(0.75, 1.25)))
        .collect::<HashMap<_, _>>();
    let budget = load_week_budget(db, &league.id, &assistant_rules, budget_at)?;
    let mut goalies = Vec::new();
    for view in goalie_views {
        let Some(stats) = view.stats.goalie.as_ref() else {
            continue;
        };
        let Some(score_stats) = goalie_scheme_stats_from_view(&view) else {
            continue;
        };
        let score_per_start = icelines_core::scheme::compute_goalie_fantasy_score(
            &score_stats,
            &scheme.goalie,
            view.gp(),
        )
        .map(|score| {
            if stats.games_started == 0 {
                0.0
            } else {
                f64::from(score.total) / f64::from(stats.games_started)
            }
        })
        .unwrap_or_default();
        let key = view.name_normalized().to_owned();
        let nhl_team = current_teams
            .as_ref()
            .and_then(|teams| teams.get(&key))
            .cloned()
            .unwrap_or_else(|| view.team_display().to_owned());
        let mut games =
            goalie_schedule_contexts(&schedule, &nhl_team, week_start, week_end, &offense_index);
        if games.is_empty() {
            continue;
        }
        let rostered = roster.contains(&key);
        let owned_elsewhere = all_rostered.contains(&key) && !rostered;
        if !rostered && !owned_elsewhere {
            if let Some(waiver) = db.get_waiver(&league.id, &key)? {
                games.retain(|game| {
                    game.start_time_utc
                        .is_some_and(|start| waiver.clears_at <= start)
                        || (game.start_time_utc.is_none()
                            && waiver.clears_at.date_naive() < game.date)
                });
            }
        }
        if games.is_empty() {
            continue;
        }
        goalies.push(FantasyGoaliePlanPlayerInput {
            player_key: key,
            player: view.full_name().to_owned(),
            nhl_team: nhl_team.clone(),
            rostered,
            acquisition_eligible: !rostered && !owned_elsewhere,
            games,
            projected_points_per_start: score_per_start,
            historical_start_probability: (f64::from(stats.games_started) / 82.0).clamp(0.0, 1.0),
            expected_save_percentage: stats.save_pct.map(f64::from),
            expected_goals_against_average: stats.goals_against_average.map(f64::from),
        });
    }
    let observations =
        db.list_latest_goalie_start_observations(&league.id, week_start, week_end)?;
    let mut warnings = vec![
        format!(
            "player rates use the {stats_season} sample; schedule uses {}",
            CURRENT_SEASON
        ),
        "opponent offense index uses current-team skater goals/game relative to league average; it is descriptive, not a betting model".to_owned(),
        format!(
            "stream ranking preserves the configured injury reserve: {} proactive acquisition(s) remain of {} total",
            budget.proactive_acquisitions_remaining, budget.acquisitions_remaining
        ),
    ];
    if focus_date.is_some_and(|date| date > week_start) && current_appearances == 0.0 {
        warnings.push(
            "current goalie appearances are zero for an in-progress week; verify --current-goalie-appearances before relying on minimum-risk advice"
                .to_owned(),
        );
    }
    if !goalies.iter().any(|goalie| goalie.rostered) {
        warnings.push("no rostered goalies matched the selected stats pool".to_owned());
    }
    let view = build_fantasy_goalie_plan(FantasyGoaliePlanInput {
        league: league.name.clone(),
        team: team.name.clone(),
        week_start,
        week_end,
        focus_date,
        strategy,
        competition_mode: competition.mode,
        goalie_slots,
        minimum_goalie_appearances: competition.minimum_goalie_appearances,
        current_goalie_appearances: current_appearances,
        evaluated_at,
        max_age_minutes,
        acquisitions_remaining: budget.proactive_acquisitions_remaining,
        goalies,
        observations,
        warnings,
    })
    .map_err(anyhow::Error::msg)?;
    Ok(view)
}

#[allow(clippy::too_many_arguments)]
pub async fn run_goalie_plan(
    week: Option<String>,
    date: Option<String>,
    team_override: Option<String>,
    league_override: Option<String>,
    stats_season: String,
    strategy: String,
    current_appearances: f64,
    max_age_minutes: i64,
    json: bool,
) -> anyhow::Result<()> {
    let focus_date = date.as_deref().map(parse_fantasy_date).transpose()?;
    let anchor = week
        .as_deref()
        .map(parse_fantasy_date)
        .transpose()?
        .or(focus_date)
        .unwrap_or_else(|| Utc::now().date_naive());
    let (week_start, week_end) = Timeframe::Week.range(anchor);
    let strategy = parse_goalie_plan_strategy(&strategy)?;
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let team = if let Some(name) = team_override {
        require_team(&db, &league.id, &name)?
    } else {
        db.get_user_team(&league.id)?.ok_or_else(|| {
            anyhow::anyhow!(
                "no user team marked; run `icelines fantasy team-use <name>` or pass --team"
            )
        })?
    };
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let budget_at = timezone
        .from_local_datetime(&anchor.and_hms_opt(12, 0, 0).expect("noon is valid"))
        .single()
        .ok_or_else(|| anyhow::anyhow!("ambiguous local noon on {anchor}"))?
        .with_timezone(&Utc);
    let view = build_goalie_plan_view_for_context(
        &db,
        &league,
        &team,
        rules,
        week_start,
        week_end,
        focus_date,
        &stats_season,
        strategy,
        current_appearances,
        max_age_minutes,
        Utc::now(),
        budget_at,
    )
    .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        print_goalie_plan(&view);
    }
    Ok(())
}

fn print_goalie_plan(view: &icelines_core::FantasyGoaliePlanView) {
    println!(
        "{} — {} — {} to {}",
        CREASE_GOALIE_PLAN_HEADER, view.team, view.week_start, view.week_end
    );
    println!("{}", view.recommendation);
    println!(
        "Expected total {:.1}; confirmed floor {:.1}; minimum {}.",
        view.expected_total_appearances,
        view.confirmed_floor_total_appearances,
        view.minimum_goalie_appearances
    );
    if let Some(refresh) = view.next_required_refresh_utc {
        println!(
            "Next evidence refresh: {} ({} due now; {} unresolved rostered on focus date).",
            refresh.to_rfc3339(),
            view.refreshes_due_now,
            view.unresolved_rostered_goalies_on_focus_date
        );
    }
    if let Some(check) = view.next_safety_check_utc {
        println!(
            "Next goalie safety check: {} ({} due now).",
            check.to_rfc3339(),
            view.safety_checks_due_now
        );
    }
    if let Some(lock) = view.next_game_lock_utc {
        println!("Next goalie game lock: {}.", lock.to_rfc3339());
    }
    for row in view
        .rows
        .iter()
        .filter(|row| view.focus_date.is_none_or(|date| row.date == date))
    {
        println!(
            "{} {:<24} vs {:<4} {:<8?} p={:.0}% {:?}{} — {}",
            row.date,
            row.player,
            row.opponent,
            row.action,
            row.start_probability * 100.0,
            row.evidence.effective_state,
            if row.team_back_to_back { " · B2B" } else { "" },
            row.reason
        );
    }
    println!("THIRD GOALIE: {}", view.portfolio.recommendation);
    for candidate in view.stream_candidates.iter().take(5) {
        println!(
            "  {:<24} +{:.2} starts, {:+.2} pts{} — {}",
            candidate.player,
            candidate.expected_appearance_gain,
            candidate.projected_points_gain,
            if candidate.acquisition_eligible {
                ""
            } else {
                " · unavailable"
            },
            candidate.recommendation
        );
    }
    for warning in &view.warnings {
        println!("Warning: {warning}");
    }
}

async fn build_injury_plan_view(
    db: &FantasyDb,
    league: &LeagueRow,
    rules: FantasyAssistantRules,
    date: Option<String>,
    stats_season: String,
    max_age_minutes: i64,
    evaluation_at: Option<DateTime<Utc>>,
) -> anyhow::Result<FantasyInjuryPlanView> {
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let plan_date = date
        .as_deref()
        .map(|value| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .with_context(|| format!("invalid date '{value}'; expected YYYY-MM-DD"))
        })
        .transpose()?
        .unwrap_or_else(|| {
            evaluation_at
                .unwrap_or_else(Utc::now)
                .with_timezone(&timezone)
                .date_naive()
        });
    if let Some(evaluation_at) = evaluation_at {
        let evaluation_date = evaluation_at.with_timezone(&timezone).date_naive();
        if evaluation_date != plan_date {
            bail!(
                "--date {plan_date} does not match --at local date {evaluation_date} in {}",
                rules.timezone
            );
        }
    }
    let plan_time = evaluation_at.unwrap_or(
        timezone
            .from_local_datetime(&plan_date.and_hms_opt(7, 0, 0).unwrap())
            .single()
            .ok_or_else(|| anyhow::anyhow!("07:00 local time is ambiguous on {plan_date}"))?
            .with_timezone(&Utc),
    );
    let team = db.get_user_team(&league.id)?.ok_or_else(|| {
        anyhow::anyhow!("no user team marked; run `icelines fantasy team-use <name>`")
    })?;
    let roster = db.list_roster(&team.id)?;
    let observations = db.list_latest_status_observations(&league.id)?;
    let statuses = roster
        .iter()
        .map(|key| {
            resolve_fantasy_player_status(
                key.clone(),
                &observations,
                plan_time,
                max_age_minutes.max(0),
            )
        })
        .collect::<Vec<_>>();
    let status_by_key = statuses
        .iter()
        .map(|status| (status.player_key.clone(), status.effective_status))
        .collect::<HashMap<_, _>>();

    let scheme = resolve_scheme(&league.scheme)?;
    let (outcome, season, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let (skaters, goalies) = pools_views(&outcome.repo, season);
    let scores = score_team(&roster, &skaters, &goalies, &scheme)
        .into_iter()
        .map(|(name, score)| (normalize_name(&name), f64::from(score)))
        .collect::<HashMap<_, _>>();
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    let current_teams = load_current_player_team_map(Season(CURRENT_SEASON)).ok();
    let schedule = load_fantasy_schedule(Season(CURRENT_SEASON), false).await?;
    let (team_dates, _) = draft_schedule_metrics(&schedule);
    let views = skaters
        .iter()
        .chain(&goalies)
        .map(|view| (view.identity.name_normalized.clone(), view))
        .collect::<HashMap<_, _>>();
    let mut unresolved = Vec::new();
    let players = roster
        .iter()
        .filter_map(|key| {
            let Some(view) = views.get(key).copied() else {
                unresolved.push(key.clone());
                return None;
            };
            let team = current_teams
                .as_ref()
                .and_then(|teams| teams.get(key))
                .cloned()
                .unwrap_or_else(|| view.team_display().to_owned());
            Some(FantasyLineupPlayerInput {
                player_key: key.clone(),
                display_name: view.full_name().to_owned(),
                nhl_team: team.clone(),
                platform_positions: eligibility
                    .get(key)
                    .cloned()
                    .unwrap_or_else(|| vec![view.position()]),
                projected_value: if view.gp() == 0 {
                    0.0
                } else {
                    scores.get(key).copied().unwrap_or_default() / f64::from(view.gp())
                },
                has_game: team_dates
                    .get(&team)
                    .is_some_and(|dates| dates.contains(&plan_date)),
                status: status_by_key
                    .get(key)
                    .copied()
                    .unwrap_or(FantasyPlayerAvailabilityStatus::Unknown),
                locked_slot: None,
                locked: false,
            })
        })
        .collect::<Vec<_>>();
    let lineup = build_fantasy_daily_lineup(rules, players).map_err(anyhow::Error::msg)?;
    let refresh_count = statuses
        .iter()
        .filter(|status| status.requires_pregame_refresh)
        .count();
    let mut warnings = lineup.warnings.clone();
    if refresh_count > 0 {
        warnings.push(format!(
            "{refresh_count} roster status(es) require a pregame refresh before a definitive start decision"
        ));
    }
    if !unresolved.is_empty() {
        warnings.push(format!(
            "{} roster player(s) were absent from the {stats_season} stats pool",
            unresolved.len()
        ));
    }
    warnings
        .push("IR/IR+ placements are advisory and do not mutate the fantasy platform".to_owned());
    Ok(FantasyInjuryPlanView {
        schema: icelines_core::view_model::FANTASY_INJURY_PLAN_SCHEMA.to_owned(),
        date: plan_date,
        lineup,
        statuses,
        warnings,
    })
}

pub async fn run_injury_plan(
    date: Option<String>,
    league_override: Option<String>,
    stats_season: String,
    max_age_minutes: i64,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let plan = build_injury_plan_view(
        &db,
        &league,
        rules,
        date,
        stats_season,
        max_age_minutes,
        None,
    )
    .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        print_injury_plan(&plan);
    }
    Ok(())
}

fn print_injury_plan(plan: &FantasyInjuryPlanView) {
    println!("{} — {}", PENALTY_BOX_AVAILABILITY_HEADER, plan.date);
    for row in &plan.lineup.injured_reserve {
        println!(
            "Move {} to {} ({:?})",
            row.player, row.reserve_slot, row.status
        );
    }
    for row in &plan.lineup.injured_reserve_plus {
        println!(
            "Move {} to {} ({:?})",
            row.player, row.reserve_slot, row.status
        );
    }
    if plan.lineup.injured_reserve.is_empty() && plan.lineup.injured_reserve_plus.is_empty() {
        println!("No fresh evidence supports an IR or IR+ move.");
    }
    for status in plan
        .statuses
        .iter()
        .filter(|status| status.requires_pregame_refresh)
    {
        println!(
            "Refresh {}: {:?} / {:?} from {}",
            status.player_key,
            status.reported_status,
            status.freshness,
            status.source.as_deref().unwrap_or("no source")
        );
    }
    for warning in &plan.warnings {
        println!("warning: {warning}");
    }
}

pub async fn run_morning(
    date: Option<String>,
    at: Option<String>,
    league_override: Option<String>,
    stats_season: String,
    max_age_minutes: i64,
    current_goalie_appearances: f64,
    material_only: bool,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let pickup_date = date.clone();
    let pickup_stats_season = stats_season.clone();
    let sleeper_stats_season = stats_season.clone();
    let goalie_stats_season = stats_season.clone();
    let sleeper_baseline_season = previous_season_id(&stats_season)?;
    let evaluation_at = at
        .as_deref()
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .with_context(|| format!("invalid RFC3339 timestamp '{value}'"))
        })
        .transpose()?;
    let plan = build_injury_plan_view(
        &db,
        &league,
        rules.clone(),
        date,
        stats_season,
        max_age_minutes,
        evaluation_at,
    )
    .await?;
    let has_uncertain_status = plan
        .statuses
        .iter()
        .any(|status| status.requires_pregame_refresh);
    let user_team = db.get_user_team(&league.id)?.ok_or_else(|| {
        anyhow::anyhow!("no user team marked; run `icelines fantasy team-use <name>`")
    })?;
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let goalie_evaluation_at = evaluation_at.unwrap_or(
        timezone
            .from_local_datetime(&plan.date.and_hms_opt(7, 0, 0).expect("07:00 is valid"))
            .single()
            .ok_or_else(|| anyhow::anyhow!("07:00 local time is ambiguous on {}", plan.date))?
            .with_timezone(&Utc),
    );
    let (goalie_week_start, goalie_week_end) = Timeframe::Week.range(plan.date);
    let goalie_plan = build_goalie_plan_view_for_context(
        &db,
        &league,
        &user_team,
        rules.clone(),
        goalie_week_start,
        goalie_week_end,
        Some(plan.date),
        &goalie_stats_season,
        FantasyMatchupStrategy::Balanced,
        current_goalie_appearances,
        max_age_minutes,
        goalie_evaluation_at,
        goalie_evaluation_at,
    )
    .await?;
    let (pickup_plan, _, _) = build_weekly_pickups_view(
        &db,
        &league,
        rules.clone(),
        None,
        pickup_date,
        pickup_stats_season,
        50,
        5,
        evaluation_at,
        !plan.lineup.injured_reserve.is_empty() || !plan.lineup.injured_reserve_plus.is_empty(),
        !has_uncertain_status,
    )
    .await?;
    let budget = pickup_plan.budget.clone();
    let sleeper_plan = build_sleepers_view(
        &db,
        &league,
        sleeper_stats_season,
        sleeper_baseline_season,
        Vec::new(),
        5,
    )
    .await?;
    let generated_at = Utc::now();
    let mut briefing = build_fantasy_morning_briefing(
        generated_at,
        goalie_evaluation_at,
        rules.timezone.clone(),
        plan,
        Some(goalie_plan),
        budget,
        Some(pickup_plan),
        Some(sleeper_plan),
    );
    let prior = db.get_morning_briefing_fingerprint(&league.id, briefing.date)?;
    briefing.suppressed_unchanged =
        material_only && prior.as_deref() == Some(briefing.material_fingerprint.as_str());
    if prior.as_deref() != Some(briefing.material_fingerprint.as_str()) {
        db.upsert_morning_briefing_fingerprint(
            &league.id,
            briefing.date,
            &briefing.material_fingerprint,
            generated_at,
        )?;
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&briefing)?);
    } else if briefing.suppressed_unchanged {
        println!(
            "{} — {} · no material recommendation changes",
            INSIDER_MORNING_SKATE_HEADER, briefing.date
        );
    } else {
        println!("{} — {}", INSIDER_MORNING_SKATE_HEADER, briefing.date);
        println!(
            "Adds: {}/{} used · {} hard-limit remaining · {} safe for proactive use",
            briefing.budget.acquisitions_used,
            briefing.budget.acquisition_limit,
            briefing.budget.acquisitions_remaining,
            briefing.budget.proactive_acquisitions_remaining
        );
        if let Some(check) = briefing.next_goalie_safety_check_utc {
            println!(
                "Goalie checkpoint: {} · {} safety check(s) due now · {} unresolved refresh(es) due now{}",
                check.to_rfc3339(),
                briefing.goalie_safety_checks_due_now,
                briefing.goalie_refreshes_due_now,
                briefing
                    .next_goalie_lock_utc
                    .map(|lock| format!(" · next lock {}", lock.to_rfc3339()))
                    .unwrap_or_default()
            );
        } else if let Some(lock) = briefing.next_goalie_lock_utc {
            println!("Next goalie lock: {}", lock.to_rfc3339());
        }
        for action in &briefing.actions {
            println!(
                "{:>2}. {}{}",
                action.priority,
                action.message,
                if action.conditional {
                    " (conditional)"
                } else {
                    ""
                }
            );
        }
        if briefing.actions.is_empty() {
            println!("No actions recommended.");
        }
        for warning in &briefing.warnings {
            println!("warning: {warning}");
        }
    }
    Ok(())
}

fn previous_season_id(season: &str) -> anyhow::Result<String> {
    if season.len() != 8 || !season.chars().all(|ch| ch.is_ascii_digit()) {
        bail!("season '{season}' is not a YYYYZZZZ id");
    }
    let start = season[..4].parse::<u32>()?;
    let end = season[4..].parse::<u32>()?;
    if end != start + 1 || start == 0 {
        bail!("season '{season}' is not a consecutive YYYYZZZZ id");
    }
    Ok(format!("{:04}{:04}", start - 1, end - 1))
}

fn parse_fantasy_status(value: &str) -> anyhow::Result<FantasyPlayerAvailabilityStatus> {
    match value.trim().to_ascii_lowercase().replace(['-', ' '], "_").as_str() {
        "healthy" | "active" => Ok(FantasyPlayerAvailabilityStatus::Healthy),
        "dtd" | "day_to_day" => Ok(FantasyPlayerAvailabilityStatus::DayToDay),
        "gtd" | "game_time_decision" => {
            Ok(FantasyPlayerAvailabilityStatus::GameTimeDecision)
        }
        "out" => Ok(FantasyPlayerAvailabilityStatus::Out),
        "ir" | "injured_reserve" => Ok(FantasyPlayerAvailabilityStatus::InjuredReserve),
        "ltir" | "long_term_injured_reserve" => {
            Ok(FantasyPlayerAvailabilityStatus::LongTermInjuredReserve)
        }
        "suspended" => Ok(FantasyPlayerAvailabilityStatus::Suspended),
        "personal" => Ok(FantasyPlayerAvailabilityStatus::Personal),
        "unknown" => Ok(FantasyPlayerAvailabilityStatus::Unknown),
        _ => bail!(
            "unknown status '{value}'; use healthy, dtd, gtd, out, ir, ltir, suspended, personal, or unknown"
        ),
    }
}

fn parse_fantasy_confidence(value: &str) -> anyhow::Result<FantasyObservationConfidence> {
    match value.trim().to_ascii_lowercase().as_str() {
        "confirmed" => Ok(FantasyObservationConfidence::Confirmed),
        "reported" => Ok(FantasyObservationConfidence::Reported),
        "estimated" => Ok(FantasyObservationConfidence::Estimated),
        "unknown" => Ok(FantasyObservationConfidence::Unknown),
        _ => bail!("unknown confidence '{value}'; use confirmed, reported, estimated, or unknown"),
    }
}

fn parse_fantasy_effective_at(value: Option<&str>) -> anyhow::Result<DateTime<Utc>> {
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|timestamp| timestamp.with_timezone(&Utc))
                .with_context(|| format!("invalid RFC3339 timestamp '{value}'"))
        })
        .transpose()
        .map(|value| value.unwrap_or_else(Utc::now))
}

fn parse_fantasy_date(value: &str) -> anyhow::Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("invalid date '{value}'; expected YYYY-MM-DD"))
}

fn parse_goalie_start_state_cli(value: &str) -> anyhow::Result<FantasyGoalieStartState> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "confirmed" | "confirmed_start" | "confirmed_starting" =>
            Ok(FantasyGoalieStartState::ConfirmedStarting),
        "reported" | "reported_start" | "reported_starting" =>
            Ok(FantasyGoalieStartState::ReportedStarting),
        "estimated" | "estimated_start" | "estimated_starting" =>
            Ok(FantasyGoalieStartState::EstimatedStarting),
        "confirmed_backup" => Ok(FantasyGoalieStartState::ConfirmedBackup),
        "reported_backup" | "backup" => Ok(FantasyGoalieStartState::ReportedBackup),
        "unknown" => Ok(FantasyGoalieStartState::Unknown),
        _ => bail!(
            "unknown goalie state '{value}'; use confirmed-starting, reported-starting, estimated-starting, confirmed-backup, reported-backup, or unknown"
        ),
    }
}

fn goalie_start_state_cli_label(state: FantasyGoalieStartState) -> &'static str {
    match state {
        FantasyGoalieStartState::ConfirmedStarting => "confirmed-starting",
        FantasyGoalieStartState::ReportedStarting => "reported-starting",
        FantasyGoalieStartState::EstimatedStarting => "estimated-starting",
        FantasyGoalieStartState::ConfirmedBackup => "confirmed-backup",
        FantasyGoalieStartState::ReportedBackup => "reported-backup",
        FantasyGoalieStartState::Unknown => "unknown",
    }
}

fn parse_goalie_plan_strategy(value: &str) -> anyhow::Result<FantasyMatchupStrategy> {
    match value.trim().to_ascii_lowercase().as_str() {
        "floor" => Ok(FantasyMatchupStrategy::Floor),
        "balanced" => Ok(FantasyMatchupStrategy::Balanced),
        "upside" => Ok(FantasyMatchupStrategy::Upside),
        _ => bail!("unknown strategy '{value}'; expected floor, balanced, or upside"),
    }
}

fn load_week_budget(
    db: &FantasyDb,
    league_id: &str,
    rules: &FantasyAssistantRules,
    now: DateTime<Utc>,
) -> anyhow::Result<icelines_core::FantasyWeekBudgetView> {
    let rows = db.list_acquisitions(league_id, now - Duration::days(8), now + Duration::days(8))?;
    let inputs = rows
        .into_iter()
        .map(|row| FantasyAcquisitionInput {
            effective_at: row.effective_at,
            kind: row.kind,
            counts_toward_limit: row.counts_toward_limit,
        })
        .collect::<Vec<_>>();
    let budget = build_fantasy_week_budget(
        now,
        &rules.timezone,
        rules.weekly_acquisition_limit,
        &inputs,
    )
    .map_err(anyhow::Error::msg)?;
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    apply_fantasy_pickup_reserve(
        budget,
        now.with_timezone(&timezone).date_naive(),
        rules.injury_pickup_reserve,
        rules.injury_reserve_release_weekday,
    )
    .map_err(anyhow::Error::msg)
}

async fn build_weekly_pickups_view(
    db: &FantasyDb,
    league: &LeagueRow,
    rules: FantasyAssistantRules,
    team_override: Option<&str>,
    date: Option<String>,
    stats_season: String,
    candidate_limit: usize,
    top: usize,
    evaluation_at: Option<DateTime<Utc>>,
    allow_injury_reserve: bool,
    allow_exceptional_override: bool,
) -> anyhow::Result<(icelines_core::FantasyWeeklyPickupView, NaiveDate, NaiveDate)> {
    let timezone = rules
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| anyhow::anyhow!("unsupported IANA timezone '{}'", rules.timezone))?;
    let evaluation_date = date
        .as_deref()
        .map(|value| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .with_context(|| format!("invalid date '{value}'; expected YYYY-MM-DD"))
        })
        .transpose()?
        .unwrap_or_else(|| {
            evaluation_at
                .unwrap_or_else(Utc::now)
                .with_timezone(&timezone)
                .date_naive()
        });
    if let Some(evaluation_at) = evaluation_at {
        let at_date = evaluation_at.with_timezone(&timezone).date_naive();
        if at_date != evaluation_date {
            bail!(
                "--date {evaluation_date} does not match --at local date {at_date} in {}",
                rules.timezone
            );
        }
    }
    let sunday = evaluation_date
        + Duration::days(6 - i64::from(evaluation_date.weekday().num_days_from_monday()));
    let evaluation_time = evaluation_at.unwrap_or(
        timezone
            .from_local_datetime(
                &evaluation_date
                    .and_hms_opt(7, 0, 0)
                    .expect("07:00 is a valid local time"),
            )
            .single()
            .ok_or_else(|| anyhow::anyhow!("07:00 local time is ambiguous on {evaluation_date}"))?
            .with_timezone(&Utc),
    );
    let budget = load_week_budget(&db, &league.id, &rules, evaluation_time)?;
    let user_team = if let Some(team) = team_override {
        require_team(db, &league.id, team)?
    } else {
        db.get_user_team(&league.id)?.ok_or_else(|| {
            anyhow::anyhow!("no user team marked; run `icelines fantasy team-use <name>`")
        })?
    };
    let user_roster = db.list_roster(&user_team.id)?;
    if user_roster.is_empty() {
        bail!("the marked fantasy roster is empty");
    }
    let all_rostered = db
        .list_teams(&league.id)?
        .into_iter()
        .map(|team| db.list_roster(&team.id))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();

    let scheme = resolve_scheme(&league.scheme)?;
    let (outcome, season, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let (skaters, goalies) = pools_views(&outcome.repo, season);
    let keys = skaters
        .iter()
        .chain(&goalies)
        .map(|view| view.identity.name_normalized.clone())
        .collect::<Vec<_>>();
    let scores = score_team(&keys, &skaters, &goalies, &scheme)
        .into_iter()
        .map(|(name, score)| (normalize_name(&name), f64::from(score)))
        .collect::<HashMap<_, _>>();
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    let current_teams = load_current_player_team_map(Season(CURRENT_SEASON)).ok();
    let mut pool = skaters
        .iter()
        .chain(&goalies)
        .map(|view| {
            let key = view.identity.name_normalized.clone();
            DraftPoolPlayer {
                key: key.clone(),
                player: view.full_name().to_owned(),
                team: current_teams
                    .as_ref()
                    .and_then(|teams| teams.get(&key))
                    .cloned()
                    .unwrap_or_else(|| view.team_display().to_owned()),
                positions: eligibility
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| vec![view.position()]),
                quality: scores.get(&key).copied().unwrap_or_default(),
                games_played: view.gp(),
            }
        })
        .collect::<Vec<_>>();
    pool.sort_by(|a, b| a.key.cmp(&b.key));
    pool.dedup_by(|a, b| a.key == b.key);
    let pool_by_key = pool
        .iter()
        .map(|player| (player.key.clone(), player))
        .collect::<HashMap<_, _>>();
    let unresolved = user_roster
        .iter()
        .filter(|key| !pool_by_key.contains_key(*key))
        .count();
    if unresolved > 0 {
        bail!("{unresolved} roster player(s) are absent from the {stats_season} stats pool");
    }

    let schedule = load_fantasy_schedule(Season(CURRENT_SEASON), false).await?;
    let (team_dates, _) = draft_schedule_metrics(&schedule);
    let dates = (0..=(sunday - evaluation_date).num_days())
        .map(|offset| evaluation_date + Duration::days(offset))
        .collect::<Vec<_>>();
    let baseline = simulate_weekly_roster(&user_roster, &pool_by_key, &team_dates, &dates, &rules)?;

    let mut available = pool
        .iter()
        .filter(|player| !all_rostered.contains(&player.key))
        .collect::<Vec<_>>();
    available.sort_by(|a, b| {
        b.quality
            .total_cmp(&a.quality)
            .then_with(|| a.key.cmp(&b.key))
    });
    available.truncate(candidate_limit.max(1));
    let open_roster_slot = user_roster.len() < rules.standard_roster_capacity();
    let mut moves = Vec::new();
    for candidate in available {
        let waiver = db.get_waiver(&league.id, &candidate.key)?;
        let availability = fantasy_acquisition_availability(
            candidate.key.clone(),
            evaluation_time,
            waiver.as_ref(),
        );
        let drops = if open_roster_slot {
            std::iter::once(None)
                .chain(user_roster.iter().map(Some))
                .collect::<Vec<_>>()
        } else {
            user_roster.iter().map(Some).collect::<Vec<_>>()
        };
        for drop_key in drops {
            let mut after_roster = user_roster.clone();
            if let Some(drop_key) = drop_key {
                after_roster.retain(|key| key != drop_key);
            }
            after_roster.push(candidate.key.clone());
            let after =
                simulate_weekly_roster(&after_roster, &pool_by_key, &team_dates, &dates, &rules)?;
            let drop_value = drop_key
                .and_then(|key| baseline.player_values.get(key).copied())
                .unwrap_or_default();
            moves.push(FantasyWeeklyMoveInput {
                add_player_key: candidate.key.clone(),
                add_player: candidate.player.clone(),
                drop_player_key: drop_key.cloned().unwrap_or_default(),
                drop_player: drop_key
                    .and_then(|key| pool_by_key.get(key).map(|player| player.player.clone()))
                    .unwrap_or_else(|| "Open roster slot".to_owned()),
                availability: availability.clone(),
                incremental_usable_starts: after.usable_starts as f64
                    - baseline.usable_starts as f64,
                projected_points_from_incremental_starts: after
                    .player_values
                    .get(&candidate.key)
                    .copied()
                    .unwrap_or_default(),
                category_gap_delta: 0.0,
                future_schedule_option_value: 0.0,
                dropped_player_rest_of_week_value: drop_value,
                waiver_reacquisition_cost: drop_value * 0.10,
                pickup_budget_cost: if budget.acquisitions_remaining <= 1 {
                    1.0
                } else {
                    0.25
                },
                uncertainty_discount: 0.0,
            });
        }
    }
    let mut view = build_fantasy_weekly_pickups_with_reserve_override(
        budget,
        moves,
        top,
        allow_injury_reserve,
        allow_exceptional_override,
    )
    .map_err(anyhow::Error::msg)?;
    view.warnings.push(
        "values use completed-season per-game scoring; injuries, confirmed starts, and same-day locks require the morning evidence phase"
            .to_owned(),
    );
    Ok((view, evaluation_date, sunday))
}

pub async fn run_weekly_pickups(
    date: Option<String>,
    league_override: Option<String>,
    stats_season: String,
    candidate_limit: usize,
    top: usize,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let (view, evaluation_date, sunday) = build_weekly_pickups_view(
        &db,
        &league,
        rules,
        None,
        date,
        stats_season,
        candidate_limit,
        top,
        None,
        false,
        true,
    )
    .await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        print_weekly_pickups(&view, evaluation_date, sunday);
    }
    Ok(())
}

#[derive(Debug, Default)]
struct WeeklyRosterSimulation {
    usable_starts: usize,
    player_values: HashMap<String, f64>,
}

fn simulate_weekly_roster(
    roster: &[String],
    pool: &HashMap<String, &DraftPoolPlayer>,
    team_dates: &HashMap<String, BTreeSet<NaiveDate>>,
    dates: &[NaiveDate],
    rules: &FantasyAssistantRules,
) -> anyhow::Result<WeeklyRosterSimulation> {
    let mut result = WeeklyRosterSimulation::default();
    for date in dates {
        let players = roster
            .iter()
            .filter_map(|key| pool.get(key).copied())
            .map(|player| FantasyLineupPlayerInput {
                player_key: player.key.clone(),
                display_name: player.player.clone(),
                nhl_team: player.team.clone(),
                platform_positions: player.positions.clone(),
                projected_value: if player.games_played == 0 {
                    0.0
                } else {
                    player.quality / f64::from(player.games_played)
                },
                has_game: team_dates
                    .get(&player.team)
                    .is_some_and(|team_dates| team_dates.contains(date)),
                status: FantasyPlayerAvailabilityStatus::Healthy,
                locked_slot: None,
                locked: false,
            })
            .collect::<Vec<_>>();
        let lineup =
            build_fantasy_daily_lineup(rules.clone(), players).map_err(anyhow::Error::msg)?;
        result.usable_starts += lineup.usable_starts;
        for row in lineup
            .active
            .iter()
            .filter(|row| row.has_game && row.status == FantasyPlayerAvailabilityStatus::Healthy)
        {
            *result
                .player_values
                .entry(row.player_key.clone())
                .or_default() += row.projected_value;
        }
    }
    Ok(result)
}

fn print_weekly_pickups(
    view: &icelines_core::FantasyWeeklyPickupView,
    start: NaiveDate,
    end: NaiveDate,
) {
    println!("{} — {start} through {end}", BENCH_WAIVER_WIRE_HEADER);
    println!(
        "Budget: {} used, {} hard-limit remaining, {} safe for proactive use",
        view.budget.acquisitions_used,
        view.budget.acquisitions_remaining,
        view.budget.proactive_acquisitions_remaining
    );
    for warning in &view.warnings {
        println!("warning: {warning}");
    }
    println!(
        "{:<4} {:<24} {:<24} {:>8} {:>9}",
        "#", "Add", "Drop", "+Starts", "Net value"
    );
    for row in &view.rows {
        println!(
            "{:<4} {:<24} {:<24} {:>8.1} {:>9.2}",
            row.rank,
            row.add_player,
            row.drop_player,
            row.incremental_usable_starts,
            row.projected_value_delta
        );
    }
}

#[derive(Debug, Clone)]
struct DraftPoolPlayer {
    key: String,
    player: String,
    team: String,
    positions: Vec<Position>,
    quality: f64,
    games_played: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct SleeperBaseline {
    gp: u32,
    fantasy_per_game: f64,
    shots_per_game: f64,
    hits_per_game: Option<f64>,
    blocks_per_game: Option<f64>,
    pp_points_per_game: f64,
}

async fn build_sleepers_view(
    db: &FantasyDb,
    league: &LeagueRow,
    stats_season: String,
    baseline_season: String,
    positions: Vec<String>,
    top: usize,
) -> anyhow::Result<FantasySleeperBoardView> {
    let scheme = resolve_scheme(&league.scheme)?;
    let requested_positions = positions
        .iter()
        .map(|value| parse_sleeper_position(value))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let all_rostered = db
        .list_teams(&league.id)?
        .into_iter()
        .map(|team| db.list_roster(&team.id))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();

    let (current_outcome, current_season, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let current = current_outcome
        .repo
        .skaters(current_season, SeasonType::Regular)
        .collect::<Vec<_>>();
    let (baseline_outcome, baseline_key, _) =
        crate::commands::players::load_repo_for_season(Some(&baseline_season), None)?;
    let baseline_views = baseline_outcome
        .repo
        .skaters(baseline_key, SeasonType::Regular)
        .collect::<Vec<_>>();
    let baseline_stats_ids = DataStore::open(icelines_data_root()?)?
        .with_live_feeds(false)
        .load_stats(baseline_key, SeasonType::Regular)?
        .into_iter()
        .map(|row| row.player_id)
        .collect::<BTreeSet<_>>();
    let baseline = baseline_views
        .iter()
        .map(|view| {
            (
                view.id().0,
                SleeperBaseline {
                    gp: view.gp(),
                    fantasy_per_game: sleeper_fantasy_per_game(
                        view,
                        &scheme,
                        view.hits().is_some(),
                        view.blocked_shots().is_some(),
                    ),
                    shots_per_game: per_game(f64::from(view.shots()), view.gp()),
                    hits_per_game: view
                        .hits()
                        .map(|value| per_game(f64::from(value), view.gp())),
                    blocks_per_game: view
                        .blocked_shots()
                        .map(|value| per_game(f64::from(value), view.gp())),
                    pp_points_per_game: per_game(f64::from(view.stats.totals.pp_points), view.gp()),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let current_teams = load_current_player_team_map(Season(CURRENT_SEASON)).ok();
    let schedule = load_fantasy_schedule(Season(CURRENT_SEASON), false).await?;
    let (team_dates, quiet_games) = draft_schedule_metrics(&schedule);

    let inputs = current
        .iter()
        .filter(|view| !all_rostered.contains(view.name_normalized()))
        .filter_map(|view| {
            let key = view.identity.name_normalized.clone();
            let platform_positions = eligibility
                .get(&key)
                .cloned()
                .unwrap_or_else(|| vec![view.position()]);
            if !requested_positions.is_empty()
                && !platform_positions
                    .iter()
                    .any(|position| requested_positions.contains(position))
            {
                return None;
            }
            let team = current_teams
                .as_ref()
                .and_then(|teams| teams.get(&key))
                .cloned()
                .unwrap_or_else(|| view.team_display().to_owned());
            let prior = baseline.get(&view.id().0).copied();
            let baseline = prior.unwrap_or_default();
            let scheduled = team_dates.get(&team).map_or(0, BTreeSet::len);
            let quiet = quiet_games.get(&team).copied().unwrap_or_default();
            Some(FantasySleeperInput {
                player_key: key.clone(),
                player: view.full_name().to_owned(),
                nhl_team: team,
                platform_positions,
                current_gp: view.gp(),
                current_fantasy_per_game: sleeper_fantasy_per_game(
                    view,
                    &scheme,
                    prior.is_none() || baseline.hits_per_game.is_some(),
                    prior.is_none() || baseline.blocks_per_game.is_some(),
                ),
                prior_gp: baseline.gp,
                prior_player_existed: baseline_stats_ids.contains(&view.id().0),
                prior_rate_available: prior.is_some(),
                prior_fantasy_per_game: baseline.fantasy_per_game,
                current_shots_per_game: per_game(f64::from(view.shots()), view.gp()),
                prior_shots_per_game: baseline.shots_per_game,
                current_hits_per_game: view
                    .hits()
                    .map(|value| per_game(f64::from(value), view.gp())),
                prior_hits_per_game: baseline.hits_per_game,
                current_blocks_per_game: view
                    .blocked_shots()
                    .map(|value| per_game(f64::from(value), view.gp())),
                prior_blocks_per_game: baseline.blocks_per_game,
                current_pp_points_per_game: per_game(
                    f64::from(view.stats.totals.pp_points),
                    view.gp(),
                ),
                prior_pp_points_per_game: baseline.pp_points_per_game,
                quiet_slate_rate: if scheduled == 0 {
                    0.0
                } else {
                    quiet as f64 / scheduled as f64
                },
            })
        })
        .collect::<Vec<_>>();
    Ok(build_fantasy_sleeper_board(
        league.scheme.clone(),
        stats_season,
        baseline_season,
        inputs,
        top,
    ))
}

pub async fn run_sleepers(
    league_override: Option<String>,
    stats_season: String,
    baseline_season: String,
    positions: Vec<String>,
    top: usize,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let view =
        build_sleepers_view(&db, &league, stats_season, baseline_season, positions, top).await?;
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        println!(
            "{} — {} vs {} · {}",
            BENCH_CALL_UP_BOARD_HEADER,
            view.stats_season,
            view.baseline_season,
            view.scoring_scheme
        );
        println!(
            "{:<4} {:<25} {:<5} {:<10} {:>7} {:>8}  Why",
            "#", "Player", "Team", "Pos", "Score", "FP/G"
        );
        for row in &view.rows {
            let positions = row
                .platform_positions
                .iter()
                .map(|position| position.abbreviation())
                .collect::<Vec<_>>()
                .join("/");
            println!(
                "{:<4} {:<25} {:<5} {:<10} {:>7.1} {:>8.2}  {}",
                row.rank,
                row.player,
                row.nhl_team,
                positions,
                row.score,
                row.current_fantasy_per_game,
                row.reasons.first().map(String::as_str).unwrap_or("-")
            );
        }
        for warning in &view.warnings {
            println!("warning: {warning}");
        }
    }
    Ok(())
}

fn per_game(total: f64, gp: u32) -> f64 {
    if gp == 0 {
        0.0
    } else {
        total / f64::from(gp)
    }
}

fn sleeper_fantasy_per_game(
    view: &PlayerView<'_>,
    scheme: &Scheme,
    include_hits: bool,
    include_blocks: bool,
) -> f64 {
    let mut stats = skater_scheme_stats_from_view(view);
    if !include_hits {
        stats.hits = 0;
    }
    if !include_blocks {
        stats.blocks = 0;
    }
    compute_fantasy_score(&stats, &scheme.skater, view.gp())
        .map(|score| f64::from(score.per_game))
        .unwrap_or_default()
}

fn parse_sleeper_position(value: &str) -> anyhow::Result<Position> {
    match value.trim().to_ascii_uppercase().as_str() {
        "C" => Ok(Position::Center),
        "LW" => Ok(Position::LeftWing),
        "RW" => Ok(Position::RightWing),
        "D" => Ok(Position::Defense),
        other => bail!("unknown sleeper position '{other}'; use C, LW, RW, or D"),
    }
}

pub async fn run_draft_board(
    taken_file: Option<PathBuf>,
    eligibility_file: Option<PathBuf>,
    hypothetical_pick: Option<String>,
    league_override: Option<String>,
    stats_season: String,
    top: usize,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let scheme = resolve_scheme(&league.scheme)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let teams = db.list_teams(&league.id)?;
    let user_team = teams.iter().find(|team| team.is_user_team);
    let all_rostered = teams
        .iter()
        .map(|team| db.list_roster(&team.id))
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<BTreeSet<_>>();
    let mut user_roster = match user_team {
        Some(team) => db.list_roster(&team.id)?,
        None => Vec::new(),
    };

    let (outcome, season, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let (skaters, goalies) = pools_views(&outcome.repo, season);
    let all_player_keys = skaters
        .iter()
        .chain(&goalies)
        .map(|view| view.identity.name_normalized.clone())
        .collect::<Vec<_>>();
    let score_by_name = score_team(&all_player_keys, &skaters, &goalies, &scheme)
        .into_iter()
        .map(|(name, score)| (normalize_name(&name), f64::from(score)))
        .collect::<HashMap<_, _>>();
    if taken_file
        .as_ref()
        .is_some_and(|path| path.to_string_lossy() == "-")
        && eligibility_file
            .as_ref()
            .is_some_and(|path| path.to_string_lossy() == "-")
    {
        bail!("--taken-file and --eligibility-file cannot both read stdin");
    }
    let mut identities = skaters
        .iter()
        .chain(&goalies)
        .map(|view| FantasyDraftIdentityInput {
            player_key: view.identity.name_normalized.clone(),
            display_name: view.full_name().to_owned(),
            aliases: Vec::new(),
        })
        .collect::<Vec<_>>();
    identities.sort_by(|a, b| a.player_key.cmp(&b.player_key));
    identities.dedup_by(|a, b| a.player_key == b.player_key);
    let eligibility_import = if let Some(path) = eligibility_file.as_ref() {
        let input = read_taken_player_input(Some(path))?;
        let view =
            import_fantasy_platform_eligibility(&input, &identities).map_err(anyhow::Error::msg)?;
        let source = path.to_string_lossy();
        for row in view
            .rows
            .iter()
            .filter(|row| row.status == icelines_core::FantasyEligibilityImportStatus::Imported)
        {
            db.upsert_player_eligibility(
                &league.id,
                row.matched_player_key
                    .as_deref()
                    .expect("imported eligibility has a player key"),
                &row.positions,
                &source,
            )?;
        }
        Some(view)
    } else {
        None
    };
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    let current_team_map = load_current_player_team_map(Season(CURRENT_SEASON)).ok();
    let mut pool = skaters
        .iter()
        .chain(&goalies)
        .map(|view| {
            let key = view.identity.name_normalized.clone();
            DraftPoolPlayer {
                key: key.clone(),
                player: view.full_name().to_owned(),
                team: current_team_map
                    .as_ref()
                    .and_then(|teams| teams.get(&key))
                    .cloned()
                    .unwrap_or_else(|| view.team_display().to_owned()),
                positions: eligibility
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| vec![view.position()]),
                quality: score_by_name.get(&key).copied().unwrap_or_default(),
                games_played: view.gp(),
            }
        })
        .collect::<Vec<_>>();
    pool.sort_by(|a, b| a.key.cmp(&b.key));
    pool.dedup_by(|a, b| a.key == b.key);

    if let Some(pick) = hypothetical_pick {
        let normalized = normalize_name(&pick);
        let matched = pool
            .iter()
            .filter(|player| player.key == normalized)
            .collect::<Vec<_>>();
        if matched.len() != 1 {
            bail!(
                "hypothetical pick '{pick}' did not resolve to exactly one current player; use the full platform name"
            );
        }
        if !user_roster.contains(&normalized) {
            user_roster.push(normalized);
        }
    }

    let taken_text = read_taken_player_input(taken_file.as_ref())?;
    let taken_import =
        import_fantasy_taken_players(&taken_text, &identities).map_err(anyhow::Error::msg)?;

    let lineup_players = user_roster
        .iter()
        .filter_map(|key| pool.iter().find(|player| &player.key == key))
        .map(|player| FantasyLineupPlayerInput {
            player_key: player.key.clone(),
            display_name: player.player.clone(),
            nhl_team: player.team.clone(),
            platform_positions: player.positions.clone(),
            projected_value: player.quality,
            has_game: true,
            status: FantasyPlayerAvailabilityStatus::Healthy,
            locked_slot: None,
            locked: false,
        })
        .collect::<Vec<_>>();
    let lineup =
        build_fantasy_daily_lineup(rules.clone(), lineup_players).map_err(anyhow::Error::msg)?;
    let open_slots = lineup.missing_active_slots;

    let schedule = load_fantasy_schedule(Season(CURRENT_SEASON), false)
        .await
        .ok();
    let (team_dates, quiet_games) = draft_schedule_metrics(schedule.as_deref().unwrap_or(&[]));
    let roster_dates = user_roster
        .iter()
        .filter_map(|key| pool.iter().find(|player| &player.key == key))
        .filter_map(|player| team_dates.get(&player.team))
        .flat_map(|dates| dates.iter().copied())
        .collect::<BTreeSet<_>>();
    let replacement_team_count = if teams.len() >= 2 { teams.len() } else { 12 };
    let replacement = draft_replacement_levels(&pool, &rules, replacement_team_count);
    let candidates = pool
        .iter()
        .filter(|player| !all_rostered.contains(&player.key) && !user_roster.contains(&player.key))
        .map(|player| {
            let dates = team_dates.get(&player.team);
            let games = dates.map_or(0, BTreeSet::len);
            let collisions = dates.map_or(0, |dates| dates.intersection(&roster_dates).count());
            let collision_rate = if games == 0 {
                0.0
            } else {
                collisions as f64 / games as f64
            };
            let replacement_level = player
                .positions
                .iter()
                .filter_map(|position| replacement.get(position).copied())
                .min_by(f64::total_cmp)
                .unwrap_or_default();
            FantasyDraftCandidateInput {
                player_key: player.key.clone(),
                player: player.player.clone(),
                nhl_team: player.team.clone(),
                platform_positions: player.positions.clone(),
                league_scored_quality: player.quality,
                replacement_level,
                incremental_usable_starts: games.saturating_sub(collisions) as f64,
                quiet_slate_games: quiet_games.get(&player.team).copied().unwrap_or_default()
                    as f64,
                schedule_collision_rate: collision_rate,
                risk_penalty: 0.0,
            }
        })
        .collect::<Vec<_>>();
    let mut board = build_fantasy_draft_board(
        league.scheme.clone(),
        stats_season,
        open_slots,
        candidates,
        taken_import,
        top,
    )
    .map_err(anyhow::Error::msg)?;
    board.eligibility_import = eligibility_import;
    if user_team.is_none() {
        board
            .warnings
            .push("no user team is marked; all active roster slots are treated as open".to_owned());
    }
    if teams.len() < 2 {
        board.warnings.push(
            "opponent teams are not imported; positional replacement level assumes a 12-team league"
                .to_owned(),
        );
    }
    if eligibility.is_empty() {
        board.warnings.push(
            "no platform eligibility is loaded; canonical NHL positions are used until a player-pool import"
                .to_owned(),
        );
    }
    if schedule.is_none() {
        board
            .warnings
            .push("2026-27 schedule was unavailable; schedule-fit components are zero".to_owned());
    }
    if current_team_map.is_none() {
        board.warnings.push(
            "current 2026-27 roster snapshots were unavailable; NHL team labels fall back to the scoring season"
                .to_owned(),
        );
    }
    board.warnings.push(
        "injury and role risk are not yet deducted; verify late news before drafting".to_owned(),
    );
    let unresolved_roster = user_roster
        .iter()
        .filter(|key| !pool.iter().any(|player| &player.key == *key))
        .count();
    if unresolved_roster > 0 {
        board.warnings.push(format!(
            "{unresolved_roster} roster player(s) were absent from the current stats pool"
        ));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&board)?);
    } else {
        print_draft_board(&board);
    }
    Ok(())
}

fn read_taken_player_input(path: Option<&PathBuf>) -> anyhow::Result<String> {
    let Some(path) = path else {
        return Ok(String::new());
    };
    if path.to_string_lossy() == "-" {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        return Ok(input);
    }
    std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}

fn draft_schedule_metrics(
    games: &[ScheduledGame],
) -> (HashMap<String, BTreeSet<NaiveDate>>, HashMap<String, usize>) {
    let mut dates = HashMap::<String, BTreeSet<NaiveDate>>::new();
    let mut games_per_date = HashMap::<NaiveDate, usize>::new();
    for game in games.iter().filter(|game| game.game_type == 2) {
        let Ok(date) = NaiveDate::parse_from_str(&game.date, "%Y-%m-%d") else {
            continue;
        };
        *games_per_date.entry(date).or_default() += 1;
        dates
            .entry(game.away_abbrev.clone())
            .or_default()
            .insert(date);
        dates
            .entry(game.home_abbrev.clone())
            .or_default()
            .insert(date);
    }
    let quiet_games = dates
        .iter()
        .map(|(team, dates)| {
            let count = dates
                .iter()
                .filter(|date| games_per_date.get(date).copied().unwrap_or_default() <= 4)
                .count();
            (team.clone(), count)
        })
        .collect();
    (dates, quiet_games)
}

fn goalie_schedule_contexts(
    games: &[ScheduledGame],
    team: &str,
    week_start: NaiveDate,
    week_end: NaiveDate,
    opponent_offense_index: &HashMap<String, f64>,
) -> Vec<FantasyGoalieGameInput> {
    let team_dates = games
        .iter()
        .filter(|game| {
            game.game_type == 2 && (game.home_abbrev == team || game.away_abbrev == team)
        })
        .filter_map(|game| NaiveDate::parse_from_str(&game.date, "%Y-%m-%d").ok())
        .collect::<BTreeSet<_>>();
    let mut rows = games
        .iter()
        .filter(|game| {
            game.game_type == 2 && (game.home_abbrev == team || game.away_abbrev == team)
        })
        .filter_map(|game| {
            let date = NaiveDate::parse_from_str(&game.date, "%Y-%m-%d").ok()?;
            if date < week_start || date > week_end {
                return None;
            }
            let home = game.home_abbrev == team;
            let opponent = if home {
                game.away_abbrev.clone()
            } else {
                game.home_abbrev.clone()
            };
            Some(FantasyGoalieGameInput {
                date,
                start_time_utc: DateTime::parse_from_rfc3339(&game.start_time_utc)
                    .ok()
                    .map(|value| value.with_timezone(&Utc)),
                opponent_offense_index: opponent_offense_index
                    .get(&opponent)
                    .copied()
                    .unwrap_or(1.0),
                opponent,
                home,
                team_back_to_back: team_dates.contains(&(date - Duration::days(1))),
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|game| game.date);
    rows
}

fn draft_replacement_levels(
    pool: &[DraftPoolPlayer],
    rules: &FantasyAssistantRules,
    fantasy_teams: usize,
) -> HashMap<Position, f64> {
    let slots = [
        (
            Position::Center,
            icelines_core::FantasyActiveSlotKind::Center,
        ),
        (
            Position::LeftWing,
            icelines_core::FantasyActiveSlotKind::LeftWing,
        ),
        (
            Position::RightWing,
            icelines_core::FantasyActiveSlotKind::RightWing,
        ),
        (
            Position::Defense,
            icelines_core::FantasyActiveSlotKind::Defense,
        ),
        (
            Position::Goalie,
            icelines_core::FantasyActiveSlotKind::Goalie,
        ),
    ];
    slots
        .into_iter()
        .map(|(position, slot)| {
            let mut values = pool
                .iter()
                .filter(|player| player.positions.contains(&position))
                .map(|player| player.quality)
                .collect::<Vec<_>>();
            values.sort_by(|a, b| b.total_cmp(a));
            let starter_count = rules.active_slots.get(&slot).copied().unwrap_or_default() as usize;
            let index = starter_count
                .saturating_mul(fantasy_teams)
                .saturating_sub(1);
            let level = values
                .get(index)
                .copied()
                .or_else(|| values.last().copied())
                .unwrap_or_default();
            (position, level)
        })
        .collect()
}

fn print_draft_board(board: &icelines_core::FantasyDraftBoardView) {
    println!(
        "{} — {} scoring on {} stats",
        BENCH_WAR_ROOM_DRAFT_HEADER, board.scoring_scheme, board.scoring_season
    );
    println!(
        "{} available · {} pasted taken · {} open starter slots",
        board.available_players,
        board.excluded_taken_players,
        board.open_slots.len()
    );
    for warning in &board.warnings {
        println!("warning: {warning}");
    }
    if let Some(import) = &board.eligibility_import {
        println!(
            "Eligibility import: {} saved, {} duplicate, {} ambiguous, {} unresolved, {} invalid",
            import.imported, import.duplicates, import.ambiguous, import.unresolved, import.invalid
        );
    }
    println!(
        "{:<4} {:<25} {:<5} {:<10} {:>9}  Why",
        "#", "Player", "Team", "Pos", "Value"
    );
    for row in &board.rows {
        let positions = row
            .platform_positions
            .iter()
            .map(|position| position.abbreviation())
            .collect::<Vec<_>>()
            .join("/");
        println!(
            "{:<4} {:<25} {:<5} {:<10} {:>9.1}  {}",
            row.rank,
            row.player,
            row.nhl_team,
            positions,
            row.draft_value,
            row.reasons.join("; ")
        );
    }
    if !board.position_leaders.is_empty() {
        println!("\nPOSITION ALTERNATIVES");
        for leader in &board.position_leaders {
            println!(
                "  {:<4} {:<25} {:>9.1}",
                leader.slot_kind.label(),
                leader.player,
                leader.draft_value
            );
        }
    }
    if let Some(fallback) = &board.fallback_pick {
        println!(
            "Fallback: {} ({:.1})",
            fallback.player, fallback.draft_value
        );
    }
    if board.taken_import.ambiguous > 0 || board.taken_import.unresolved > 0 {
        println!(
            "Resolve taken list: {} ambiguous, {} unresolved",
            board.taken_import.ambiguous, board.taken_import.unresolved
        );
    }
}

fn print_assistant_rules(rules: &FantasyAssistantRules) {
    let active = [
        icelines_core::FantasyActiveSlotKind::Center,
        icelines_core::FantasyActiveSlotKind::LeftWing,
        icelines_core::FantasyActiveSlotKind::RightWing,
        icelines_core::FantasyActiveSlotKind::Defense,
        icelines_core::FantasyActiveSlotKind::Utility,
        icelines_core::FantasyActiveSlotKind::Goalie,
    ]
    .into_iter()
    .map(|kind| {
        format!(
            "{} {}",
            rules.active_slots.get(&kind).copied().unwrap_or(0),
            kind.label()
        )
    })
    .collect::<Vec<_>>()
    .join(" · ");
    println!("Active: {active}");
    println!(
        "Bench: {} · IR: {} · IR+: {} · Total capacity: {}",
        rules.bench_slots,
        rules.ir_slots,
        rules.ir_plus_slots,
        rules.total_capacity_with_reserve()
    );
    println!(
        "Weekly acquisitions: {} · Waivers: {} days · Same-day free agents",
        rules.weekly_acquisition_limit, rules.waiver_days
    );
    println!(
        "Injury pickup reserve: {} through Friday · releases Saturday",
        rules.injury_pickup_reserve
    );
    println!("Morning: {} {}", rules.morning_time, rules.timezone);
}

/// `icelines fantasy roster-shape-validate [--league <league>] [--team <team>] [--json]`
pub async fn run_roster_shape_validate(
    league_override: Option<String>,
    team: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let teams = if let Some(team_name) = team {
        vec![require_team(&db, &league.id, &team_name)?]
    } else {
        db.list_teams(&league.id)?
    };
    let positions = if teams_have_rostered_players(&db, &teams)? {
        known_player_positions()?
    } else {
        BTreeMap::<String, Vec<Position>>::new()
    };
    let views = teams
        .iter()
        .map(|team| db.validate_team_roster_shape(&league, team, &positions))
        .collect::<anyhow::Result<Vec<_>>>()?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&views).context("serializing roster shape validation")?
        );
        return Ok(());
    }

    print_roster_shape_validation(&league, &views);
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct FantasyTradeReadinessTeam {
    team: String,
    is_user_team: bool,
    roster_size: usize,
    standard_capacity: usize,
    missing_active_slots: usize,
    unknown_players: Vec<String>,
    unvalued_players: Vec<String>,
    ready: bool,
    issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FantasyTradeReadinessView {
    schema: String,
    league: String,
    ready: bool,
    teams_checked: usize,
    teams_ready: usize,
    teams: Vec<FantasyTradeReadinessTeam>,
    warnings: Vec<String>,
}

fn trade_roster_readiness_counts(
    roster: &[String],
    positions: &HashMap<String, Vec<Position>>,
    valued_players: &BTreeSet<String>,
    rules: &FantasyAssistantRules,
) -> anyhow::Result<(Vec<String>, Vec<String>, usize)> {
    let unknown_players = roster
        .iter()
        .filter(|key| !positions.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let unvalued_players = roster
        .iter()
        .filter(|key| !valued_players.contains(*key))
        .cloned()
        .collect::<Vec<_>>();
    let inputs = roster
        .iter()
        .filter_map(|key| {
            Some(FantasyLineupPlayerInput {
                player_key: key.clone(),
                display_name: key.clone(),
                nhl_team: String::new(),
                platform_positions: positions.get(key)?.clone(),
                projected_value: 1.0,
                has_game: true,
                status: FantasyPlayerAvailabilityStatus::Healthy,
                locked_slot: None,
                locked: false,
            })
        })
        .collect::<Vec<_>>();
    let missing_active_slots = build_fantasy_daily_lineup(rules.clone(), inputs)
        .map_err(anyhow::Error::msg)?
        .missing_active_slots
        .len();
    Ok((unknown_players, unvalued_players, missing_active_slots))
}

pub async fn run_trade_readiness(
    league_override: Option<String>,
    team_override: Option<String>,
    stats_season: String,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let teams = if let Some(team) = team_override.as_deref() {
        vec![require_team(&db, &league.id, team)?]
    } else {
        db.list_teams(&league.id)?
    };
    let (outcome, stats_window, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let (skaters, goalies) = pools_views(&outcome.repo, stats_window);
    let valued_players = skaters
        .iter()
        .chain(goalies.iter())
        .map(|player| player.identity.name_normalized.clone())
        .collect::<BTreeSet<_>>();
    let mut positions = skaters
        .iter()
        .chain(goalies.iter())
        .map(|player| {
            (
                player.identity.name_normalized.clone(),
                vec![player.position()],
            )
        })
        .collect::<HashMap<_, _>>();
    for eligibility in db.list_player_eligibility(&league.id)? {
        positions.insert(eligibility.player_normalized, eligibility.positions);
    }

    let capacity = rules.standard_roster_capacity();
    let mut rows = Vec::with_capacity(teams.len());
    for team in teams {
        let roster = db.list_roster(&team.id)?;
        let (unknown_players, unvalued_players, missing_active_slots) =
            trade_roster_readiness_counts(&roster, &positions, &valued_players, &rules)?;
        let mut issues = Vec::new();
        if roster.len() < capacity {
            issues.push(format!(
                "{} open standard roster spot(s)",
                capacity - roster.len()
            ));
        } else if roster.len() > capacity {
            issues.push(format!(
                "{} player(s) over standard capacity",
                roster.len() - capacity
            ));
        }
        if missing_active_slots > 0 {
            issues.push(format!("{missing_active_slots} unfillable active slot(s)"));
        }
        if !unknown_players.is_empty() {
            issues.push(format!(
                "{} player(s) lack position eligibility",
                unknown_players.len()
            ));
        }
        if !unvalued_players.is_empty() {
            issues.push(format!(
                "{} player(s) lack {stats_season} scoring data",
                unvalued_players.len()
            ));
        }
        let ready = issues.is_empty();
        rows.push(FantasyTradeReadinessTeam {
            team: team.name,
            is_user_team: team.is_user_team,
            roster_size: roster.len(),
            standard_capacity: capacity,
            missing_active_slots,
            unknown_players,
            unvalued_players,
            ready,
            issues,
        });
    }
    let teams_ready = rows.iter().filter(|team| team.ready).count();
    let mut warnings = Vec::new();
    if rows.len() < 2 && team_override.is_none() {
        warnings.push("a league-wide trade search requires at least two saved teams".to_owned());
    }
    let ready = !rows.is_empty()
        && teams_ready == rows.len()
        && (team_override.is_some() || rows.len() >= 2);
    let view = FantasyTradeReadinessView {
        schema: "fantasy_trade_readiness.v1".to_owned(),
        league: league.name,
        ready,
        teams_checked: rows.len(),
        teams_ready,
        teams: rows,
        warnings,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        println!("\n{} — {}", BOARDS_TRADE_READINESS_HEADER, view.league);
        println!(
            "  {} · {}/{} teams ready",
            if view.ready { "READY" } else { "NOT READY" },
            view.teams_ready,
            view.teams_checked
        );
        for team in &view.teams {
            println!(
                "  {:<28} {:>2}/{:<2} · missing {:>2} · unknown {:>2} · unvalued {:>2} · {}",
                if team.is_user_team {
                    format!("{} (mine)", team.team)
                } else {
                    team.team.clone()
                },
                team.roster_size,
                team.standard_capacity,
                team.missing_active_slots,
                team.unknown_players.len(),
                team.unvalued_players.len(),
                if team.ready { "ready" } else { "BLOCKED" }
            );
            for issue in &team.issues {
                println!("      - {issue}");
            }
        }
        for warning in &view.warnings {
            println!("  warning: {warning}");
        }
    }
    Ok(())
}

fn teams_have_rostered_players(db: &FantasyDb, teams: &[TeamRow]) -> anyhow::Result<bool> {
    for team in teams {
        if !db.list_roster(&team.id)?.is_empty() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn print_roster_shape_validation(league: &LeagueRow, views: &[RosterShapeValidationView]) {
    println!(
        "Fantasy roster shape - {} ({})",
        league.name, league.roster_shape
    );
    if views.is_empty() {
        println!("No teams found. Create one with `icelines fantasy team-create <name>`.");
        return;
    }
    for view in views {
        println!(
            "\n{}: {} ({} players, {} missing groups, {} over-cap groups, {} issues)",
            view.team,
            roster_shape_status_label(view.status),
            view.summary.rostered_players,
            view.summary.missing_slots,
            view.summary.overflow_slots,
            view.summary.unknown_players
                + view.summary.duplicate_players
                + view.summary.ineligible_players
        );
        println!(
            "{:<8} {:>5} {:>5} {:>5} Status",
            "Slot", "Have", "Min", "Max"
        );
        for slot in &view.slots {
            println!(
                "{:<8} {:>5} {:>5} {:>5} {}",
                slot.label,
                slot.count,
                slot.min,
                slot.max
                    .map(|max| max.to_string())
                    .unwrap_or_else(|| "-".to_string()),
                format!("{:?}", slot.status).to_ascii_lowercase()
            );
        }
        for issue in &view.player_issues {
            println!("  warning: {}", issue.message);
        }
    }
}

fn roster_shape_status_label(status: RosterShapeStatus) -> &'static str {
    match status {
        RosterShapeStatus::Legal => "legal",
        RosterShapeStatus::Invalid => "invalid",
    }
}

fn print_import_yahoo(view: &FantasyImportView) {
    println!(
        "Fantasy Yahoo roster import - {} / {}",
        view.league, view.mode_label
    );
    println!(
        "Teams: {} seen, {} created, {} updated, {} unchanged, {} errors",
        view.summary.teams_seen,
        view.summary.teams_created,
        view.summary.teams_updated,
        view.summary.teams_unchanged,
        view.summary.teams_error
    );
    println!(
        "Players: {} seen, {} imported, {} skipped, {} unresolved, {} duplicate, {} errors",
        view.summary.players_seen,
        view.summary.players_imported,
        view.summary.players_skipped,
        view.summary.players_unresolved,
        view.summary.players_duplicate,
        view.summary.players_error
    );
    if view.mode_label == "dry-run" {
        println!("Dry-run only: no FantasyDb changes were written.");
    }
    for warning in &view.warnings {
        println!("warning: {}", warning.message);
    }
    if let Some(empty) = &view.empty_state {
        println!("{}: {}", empty.title, empty.detail.as_deref().unwrap_or(""));
        return;
    }

    if !view.teams.is_empty() {
        println!();
        println!(
            "{:<4} {:<28} {:<16} {:>8} {:>8} {:>8}",
            "Rank", "Team", "Status", "Import", "Skipped", "Errors"
        );
        for team in &view.teams {
            let team_name = if team.is_user_team {
                format!("{} (mine)", team.team)
            } else {
                team.team.clone()
            };
            println!(
                "{:<4} {:<28} {:<16} {:>8} {:>8} {:>8}",
                team.rank,
                team_name,
                format!("{:?}", team.status).to_ascii_lowercase(),
                team.imported_players,
                team.skipped_rows,
                team.error_rows
            );
        }
    }

    let diagnostic_rows = view
        .rows
        .iter()
        .filter(|row| row.status != FantasyImportRowStatus::Imported)
        .collect::<Vec<_>>();
    println!();
    if diagnostic_rows.is_empty() {
        println!("Diagnostics: none");
        return;
    }
    println!(
        "{:<5} {:<28} {:<24} {:<12} Message",
        "Row", "Player", "Team", "Status"
    );
    for row in diagnostic_rows {
        println!(
            "{:<5} {:<28} {:<24} {:<12} {}",
            row.row_number,
            row.player_name,
            row.fantasy_team.as_deref().unwrap_or("-"),
            row_status_label(row.status),
            row.message.as_deref().unwrap_or("-")
        );
    }
}

fn row_status_label(status: FantasyImportRowStatus) -> &'static str {
    match status {
        FantasyImportRowStatus::Imported => "imported",
        FantasyImportRowStatus::Skipped => "skipped",
        FantasyImportRowStatus::Unresolved => "unresolved",
        FantasyImportRowStatus::Duplicate => "duplicate",
        FantasyImportRowStatus::Error => "error",
    }
}

#[derive(Debug, Clone, Serialize)]
struct FantasyTradePlayerEvaluation {
    player_key: String,
    player: String,
    nhl_team: String,
    positions: Vec<Position>,
    league_value: f64,
    league_value_per_game: f64,
    remaining_games: u32,
    projected_remaining_value: f64,
}

#[derive(Debug, Clone, Serialize)]
struct FantasyTradeTeamEvaluation {
    team: String,
    before_value: f64,
    after_value: f64,
    value_delta: f64,
    remaining_games_delta: i32,
    roster_size_after: usize,
    standard_capacity: usize,
    missing_active_slots_before: usize,
    missing_active_slots_after: usize,
    legal: bool,
}

#[derive(Debug, Clone, Serialize)]
struct FantasyTradeEvaluation {
    schema: String,
    executed: bool,
    saved_offer_id: Option<String>,
    league: String,
    scoring_scheme: String,
    sending_team: String,
    receiving_team: String,
    sends: Vec<FantasyTradePlayerEvaluation>,
    receives: Vec<FantasyTradePlayerEvaluation>,
    sending_team_result: FantasyTradeTeamEvaluation,
    receiving_team_result: FantasyTradeTeamEvaluation,
    package_value_gap: f64,
    package_value_gap_percent: f64,
    recommendation: String,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct FantasyTradeOfferCandidate {
    rank: usize,
    opponent: String,
    sends: Vec<FantasyTradePlayerEvaluation>,
    receives: Vec<FantasyTradePlayerEvaluation>,
    projected_value_delta: f64,
    active_lineup_value_delta: f64,
    opponent_projected_value_delta: f64,
    opponent_active_lineup_value_delta: f64,
    value_gap_percent: f64,
    remaining_games_delta: i32,
    quiet_slate_games_delta: i32,
    schedule_class_diversity_delta: i32,
    opponent_quiet_slate_games_delta: i32,
    opponent_schedule_class_diversity_delta: i32,
    user_missing_slots_after: usize,
    opponent_missing_slots_after: usize,
    fit_score: f64,
    counterpart_fit_score: f64,
    mutual_fit_score: f64,
    rationale: String,
}

#[derive(Debug, Clone, Serialize)]
struct FantasyTradeFinderView {
    schema: String,
    league: String,
    scoring_scheme: String,
    team: String,
    max_package: usize,
    fairness_percent: f64,
    require_complete: bool,
    protected_players: Vec<String>,
    candidates_considered: usize,
    offers: Vec<FantasyTradeOfferCandidate>,
    warnings: Vec<String>,
}

fn trade_packages(roster: &[String], max_package: usize) -> Vec<Vec<String>> {
    let mut packages = roster
        .iter()
        .cloned()
        .map(|player| vec![player])
        .collect::<Vec<_>>();
    if max_package >= 2 {
        for first in 0..roster.len() {
            for second in first + 1..roster.len() {
                packages.push(vec![roster[first].clone(), roster[second].clone()]);
            }
        }
    }
    packages
}

fn trade_roster_projected_value(
    roster: &[String],
    players: &HashMap<String, FantasyTradePlayerEvaluation>,
) -> f64 {
    roster
        .iter()
        .filter_map(|key| players.get(key))
        .map(|player| player.projected_remaining_value)
        .sum()
}

fn trade_roster_active_value(
    roster: &[String],
    players: &HashMap<String, FantasyTradePlayerEvaluation>,
    rules: &FantasyAssistantRules,
) -> anyhow::Result<f64> {
    let inputs = roster
        .iter()
        .filter_map(|key| players.get(key))
        .map(|player| FantasyLineupPlayerInput {
            player_key: player.player_key.clone(),
            display_name: player.player.clone(),
            nhl_team: player.nhl_team.clone(),
            platform_positions: player.positions.clone(),
            projected_value: player.league_value_per_game,
            has_game: true,
            status: FantasyPlayerAvailabilityStatus::Healthy,
            locked_slot: None,
            locked: false,
        })
        .collect::<Vec<_>>();
    Ok(build_fantasy_daily_lineup(rules.clone(), inputs)
        .map_err(anyhow::Error::msg)?
        .projected_active_value)
}

fn trade_roster_schedule_fit(
    roster: &[String],
    players: &HashMap<String, FantasyTradePlayerEvaluation>,
    quiet_games: &HashMap<String, usize>,
    schedule_classes: &HashMap<String, usize>,
) -> (i32, i32) {
    let quiet = roster
        .iter()
        .filter_map(|key| players.get(key))
        .map(|player| quiet_games.get(&player.nhl_team).copied().unwrap_or(0) as i32)
        .sum();
    let classes = roster
        .iter()
        .filter_map(|key| players.get(key))
        .filter_map(|player| schedule_classes.get(&player.nhl_team).copied())
        .collect::<BTreeSet<_>>()
        .len() as i32;
    (quiet, classes)
}

fn trade_roster_missing_slots(
    roster: &[String],
    players: &HashMap<String, FantasyTradePlayerEvaluation>,
    rules: &FantasyAssistantRules,
) -> anyhow::Result<usize> {
    let inputs = roster
        .iter()
        .filter_map(|key| players.get(key))
        .map(|player| FantasyLineupPlayerInput {
            player_key: player.player_key.clone(),
            display_name: player.player.clone(),
            nhl_team: player.nhl_team.clone(),
            platform_positions: player.positions.clone(),
            projected_value: player.league_value,
            has_game: true,
            status: FantasyPlayerAvailabilityStatus::Healthy,
            locked_slot: None,
            locked: false,
        })
        .collect::<Vec<_>>();
    Ok(build_fantasy_daily_lineup(rules.clone(), inputs)
        .map_err(anyhow::Error::msg)?
        .missing_active_slots
        .len())
}

/// Evaluate or atomically execute a one-for-one or comma-separated package trade.
pub async fn run_trade(
    player1_q: String,
    to_team_name: String,
    player2_q: String,
    execute: bool,
    save_offer: bool,
    league_override: Option<String>,
    stats_season: String,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let scheme = resolve_scheme(&league.scheme)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let (outcome, season, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    let current_season = Season(CURRENT_SEASON);
    let current_teams = load_current_player_team_map(current_season).ok();
    let (remaining_by_team, schedule_warning) = remaining_games_by_team(current_season).await;
    let schedule_available = !remaining_by_team.is_empty();
    let player_details = all_skaters
        .iter()
        .chain(all_goalies.iter())
        .map(|view| {
            let key = view.identity.name_normalized.clone();
            let value = score_team(
                std::slice::from_ref(&key),
                &all_skaters,
                &all_goalies,
                &scheme,
            )
            .first()
            .map(|(_, score)| f64::from(*score))
            .unwrap_or_default();
            let value_per_game = if view.gp() == 0 {
                0.0
            } else {
                value / f64::from(view.gp())
            };
            let nhl_team = current_teams
                .as_ref()
                .and_then(|teams| teams.get(&key))
                .cloned()
                .unwrap_or_else(|| view.team_display().to_owned());
            let remaining_games = remaining_by_team.get(&nhl_team).copied().unwrap_or(0);
            (
                key.clone(),
                FantasyTradePlayerEvaluation {
                    player_key: key.clone(),
                    player: view.identity.full_name.clone(),
                    nhl_team: nhl_team.clone(),
                    positions: eligibility
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| vec![view.position()]),
                    league_value: value,
                    league_value_per_game: value_per_game,
                    remaining_games,
                    projected_remaining_value: if schedule_available && remaining_games > 0 {
                        value_per_game * f64::from(remaining_games)
                    } else {
                        value
                    },
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let resolve_package = |raw: &str| -> anyhow::Result<Vec<FantasyTradePlayerEvaluation>> {
        let queries = raw
            .split(',')
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .collect::<Vec<_>>();
        if queries.is_empty() || queries.len() > 3 {
            bail!("trade packages must contain between one and three comma-separated players");
        }
        let mut resolved = Vec::with_capacity(queries.len());
        for query in queries {
            let key = match fuzzy_find_skater(&all_skaters, query) {
                Ok(view) => view.identity.name_normalized.clone(),
                Err(skater_error) => match fuzzy_find_view_in(&all_goalies, query) {
                    Some(view) => view.identity.name_normalized.clone(),
                    None => return Err(skater_error),
                },
            };
            let player = player_details
                .get(&key)
                .cloned()
                .with_context(|| format!("resolved player '{query}' is absent from trade pool"))?;
            if resolved
                .iter()
                .any(|existing: &FantasyTradePlayerEvaluation| existing.player_key == key)
            {
                bail!("trade package repeats '{}'", player.player);
            }
            resolved.push(player);
        }
        Ok(resolved)
    };
    let sends = resolve_package(&player1_q)?;
    let receives = resolve_package(&player2_q)?;
    let send_keys = sends
        .iter()
        .map(|player| player.player_key.clone())
        .collect::<Vec<_>>();
    let receive_keys = receives
        .iter()
        .map(|player| player.player_key.clone())
        .collect::<Vec<_>>();

    let team1_name = db
        .is_on_any_team(&league.id, &send_keys[0])?
        .with_context(|| format!("'{}' is not on any team in this league", sends[0].player))?;
    let team1 = require_team(&db, &league.id, &team1_name)?;
    let team2 = require_team(&db, &league.id, &to_team_name)?;
    if team1.id == team2.id {
        bail!("trade teams must be different");
    }
    for player in &sends {
        if db
            .is_on_any_team(&league.id, &player.player_key)?
            .as_deref()
            != Some(team1_name.as_str())
        {
            bail!("'{}' is not on team '{}'", player.player, team1_name);
        }
    }
    for player in &receives {
        if db
            .is_on_any_team(&league.id, &player.player_key)?
            .as_deref()
            != Some(to_team_name.as_str())
        {
            bail!("'{}' is not on team '{to_team_name}'", player.player);
        }
    }

    let roster1_before = db.list_roster(&team1.id)?;
    let roster2_before = db.list_roster(&team2.id)?;
    let score1_before = trade_roster_projected_value(&roster1_before, &player_details);
    let score2_before = trade_roster_projected_value(&roster2_before, &player_details);
    let mut roster1_after = roster1_before
        .iter()
        .filter(|key| !send_keys.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    roster1_after.extend(receive_keys.iter().cloned());
    let mut roster2_after = roster2_before
        .iter()
        .filter(|key| !receive_keys.contains(key))
        .cloned()
        .collect::<Vec<_>>();
    roster2_after.extend(send_keys.iter().cloned());
    let score1_after = trade_roster_projected_value(&roster1_after, &player_details);
    let score2_after = trade_roster_projected_value(&roster2_after, &player_details);
    let missing1_before = trade_roster_missing_slots(&roster1_before, &player_details, &rules)?;
    let missing2_before = trade_roster_missing_slots(&roster2_before, &player_details, &rules)?;
    let missing1_after = trade_roster_missing_slots(&roster1_after, &player_details, &rules)?;
    let missing2_after = trade_roster_missing_slots(&roster2_after, &player_details, &rules)?;
    let capacity = rules.standard_roster_capacity();
    let legal1 = roster1_after.len() <= capacity && missing1_after <= missing1_before;
    let legal2 = roster2_after.len() <= capacity && missing2_after <= missing2_before;
    let sent_value = sends
        .iter()
        .map(|player| player.projected_remaining_value)
        .sum::<f64>();
    let received_value = receives
        .iter()
        .map(|player| player.projected_remaining_value)
        .sum::<f64>();
    let value_gap = received_value - sent_value;
    let value_gap_percent = if sent_value > 0.0 {
        value_gap / sent_value * 100.0
    } else {
        0.0
    };
    let remaining_games_delta1 = receives
        .iter()
        .map(|player| player.remaining_games as i32)
        .sum::<i32>()
        - sends
            .iter()
            .map(|player| player.remaining_games as i32)
            .sum::<i32>();
    let result1 = FantasyTradeTeamEvaluation {
        team: team1_name.clone(),
        before_value: score1_before,
        after_value: score1_after,
        value_delta: score1_after - score1_before,
        remaining_games_delta: remaining_games_delta1,
        roster_size_after: roster1_after.len(),
        standard_capacity: capacity,
        missing_active_slots_before: missing1_before,
        missing_active_slots_after: missing1_after,
        legal: legal1,
    };
    let result2 = FantasyTradeTeamEvaluation {
        team: to_team_name.clone(),
        before_value: score2_before,
        after_value: score2_after,
        value_delta: score2_after - score2_before,
        remaining_games_delta: -remaining_games_delta1,
        roster_size_after: roster2_after.len(),
        standard_capacity: capacity,
        missing_active_slots_before: missing2_before,
        missing_active_slots_after: missing2_after,
        legal: legal2,
    };
    let recommendation = if !legal1 || !legal2 {
        "Reject: the package worsens active-slot legality or exceeds roster capacity".to_owned()
    } else if value_gap_percent > 20.0 {
        "Strong value for the sending team; the other manager may require a better return"
            .to_owned()
    } else if value_gap_percent < -20.0 {
        "Reject or renegotiate: the sending team gives up materially more league value".to_owned()
    } else {
        "Reasonable offer range; decide on positional need, schedule fit, and injury risk"
            .to_owned()
    };
    let mut warnings = Vec::new();
    if let Some(warning) = schedule_warning {
        warnings.push(warning);
    }
    if roster1_before.len() < capacity || roster2_before.len() < capacity {
        warnings.push(
            "one or both saved rosters are partial; legality compares missing slots before versus after"
                .to_owned(),
        );
    }
    if execute {
        if !legal1 || !legal2 {
            bail!("cannot execute a positionally illegal trade");
        }
        db.execute_trade(&team1.id, &send_keys, &team2.id, &receive_keys)?;
    }
    let saved_offer_id = if save_offer {
        if !legal1 || !legal2 {
            bail!("cannot save a positionally illegal trade offer");
        }
        Some(db.save_trade_offer(&team1.id, &send_keys, &team2.id, &receive_keys)?)
    } else {
        None
    };

    let view = FantasyTradeEvaluation {
        schema: "fantasy_trade_evaluation.v1".to_owned(),
        executed: execute,
        saved_offer_id,
        league: league.name.clone(),
        scoring_scheme: league.scheme.clone(),
        sending_team: team1_name.clone(),
        receiving_team: to_team_name.clone(),
        sends,
        receives,
        sending_team_result: result1,
        receiving_team_result: result2,
        package_value_gap: value_gap,
        package_value_gap_percent: value_gap_percent,
        recommendation,
        warnings,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        println!(
            "\n{} — {} ({})",
            BOARDS_TRADE_ANALYSIS_HEADER, view.league, view.scoring_scheme
        );
        println!(
            "  {} sends: {}",
            view.sending_team,
            view.sends
                .iter()
                .map(|player| player.player.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  {} sends: {}",
            view.receiving_team,
            view.receives
                .iter()
                .map(|player| player.player.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "  {:<22} {:>10} {:>10} {:>10} {:>8} {:>8}",
            "Team", "ROS before", "ROS after", "Delta", "Games", "Legal"
        );
        for result in [&view.sending_team_result, &view.receiving_team_result] {
            println!(
                "  {:<22} {:>10.1} {:>10.1} {:>+10.1} {:>+8} {:>8}",
                result.team,
                result.before_value,
                result.after_value,
                result.value_delta,
                result.remaining_games_delta,
                if result.legal { "yes" } else { "NO" }
            );
        }
        println!(
            "  Package gap for {}: {:+.1} ({:+.1}%)",
            view.sending_team, view.package_value_gap, view.package_value_gap_percent
        );
        println!("  Recommendation: {}", view.recommendation);
        for warning in &view.warnings {
            println!("  warning: {warning}");
        }
    }

    if execute && !json {
        println!("\nTrade executed.");
    } else if let Some(id) = &view.saved_offer_id {
        if !json {
            println!("\nOffer saved as pending: {id}");
        }
    } else if !json {
        println!("\n(use --execute to commit this trade)");
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct FantasyTradeHistoryView {
    schema: String,
    league: String,
    trades: Vec<icelines_fetch::fantasy_db::FantasyTradeHistoryRow>,
}

pub fn run_trade_history(
    league_override: Option<String>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    if !(1..=1_000).contains(&limit) {
        bail!("--limit must be between 1 and 1000");
    }
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let view = FantasyTradeHistoryView {
        schema: "fantasy_trade_history.v1".to_owned(),
        league: league.name,
        trades: db.list_trade_history(&league.id, limit)?,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
        return Ok(());
    }

    println!("\n{} — {}", BOARDS_TRADE_HISTORY_HEADER, view.league);
    if view.trades.is_empty() {
        println!("  No locally executed trades recorded.");
        return Ok(());
    }
    for trade in &view.trades {
        let sends = trade.sends.join(", ").replace('_', " ");
        let receives = trade.receives.join(", ").replace('_', " ");
        println!(
            "  {} · {} sent [{}] to {} for [{}]",
            trade.executed_at, trade.sending_team, sends, trade.receiving_team, receives
        );
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct FantasyTradeOffersView {
    schema: String,
    league: String,
    status_filter: Option<String>,
    actionable_only: bool,
    offers: Vec<icelines_fetch::fantasy_db::FantasyTradeOfferRow>,
}

pub fn run_trade_offers(
    status: Option<String>,
    actionable_only: bool,
    league_override: Option<String>,
    limit: usize,
    json: bool,
) -> anyhow::Result<()> {
    if !(1..=1_000).contains(&limit) {
        bail!("--limit must be between 1 and 1000");
    }
    let status = status.map(|value| value.to_ascii_lowercase());
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let fetch_limit = if actionable_only { 1_000 } else { limit };
    let mut offers = db.list_trade_offers(&league.id, status.as_deref(), fetch_limit)?;
    if actionable_only {
        offers.retain(|offer| offer.roster_current);
        offers.truncate(limit);
    }
    let view = FantasyTradeOffersView {
        schema: "fantasy_trade_offers.v1".to_owned(),
        league: league.name,
        status_filter: status.clone(),
        actionable_only,
        offers,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
        return Ok(());
    }
    println!("\n{} — {}", BOARDS_TRADE_OFFERS_HEADER, view.league);
    if view.offers.is_empty() {
        println!("  No matching saved offers.");
    }
    for offer in &view.offers {
        let readiness = if offer.roster_current {
            "actionable"
        } else {
            "STALE"
        };
        println!(
            "  {} · {} · {} · {} sends [{}] to {} for [{}] · {}",
            offer.id,
            offer.status,
            readiness,
            offer.sending_team,
            offer.sends.join(", ").replace('_', " "),
            offer.receiving_team,
            offer.receives.join(", ").replace('_', " "),
            offer.updated_at
        );
        for issue in &offer.roster_issues {
            println!("    warning: {issue}");
        }
    }
    Ok(())
}

pub fn run_trade_offer_close(
    id: String,
    status: String,
    league_override: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let status = status.to_ascii_lowercase();
    if !db.close_trade_offer(&league.id, &id, &status)? {
        bail!("pending trade offer '{id}' was not found");
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema": "fantasy_trade_offer_status.v1",
                "league": league.name,
                "offer_id": id,
                "status": status,
                "rosters_changed": false
            }))?
        );
    } else {
        println!("Trade offer {id} marked {status}; rosters were not changed.");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn run_trade_finder(
    team_override: Option<String>,
    opponent_override: Option<String>,
    max_package: usize,
    fairness_percent: f64,
    protect: Vec<String>,
    include_anchors: bool,
    require_complete: bool,
    top: usize,
    league_override: Option<String>,
    stats_season: String,
    json: bool,
) -> anyhow::Result<()> {
    if !(1..=2).contains(&max_package) {
        bail!("--max-package must be one or two");
    }
    if !(0.0..=50.0).contains(&fairness_percent) {
        bail!("--fairness-percent must be between 0 and 50");
    }
    if top == 0 || top > 100 {
        bail!("--top must be between 1 and 100");
    }

    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let scheme = resolve_scheme(&league.scheme)?;
    let rules = db
        .get_assistant_rules(&league.id)?
        .unwrap_or_else(FantasyAssistantRules::configured_2026);
    let teams = db.list_teams(&league.id)?;
    let user_team = if let Some(name) = team_override {
        require_team(&db, &league.id, &name)?
    } else {
        teams
            .iter()
            .find(|team| team.is_user_team)
            .cloned()
            .with_context(|| {
                format!(
                    "no user team marked in '{}'; pass --team or run `icelines fantasy team-use <name>`",
                    league.name
                )
            })?
    };
    let opponents = if let Some(name) = opponent_override {
        vec![require_team(&db, &league.id, &name)?]
    } else {
        teams
            .into_iter()
            .filter(|team| team.id != user_team.id)
            .collect::<Vec<_>>()
    };
    if opponents.iter().any(|team| team.id == user_team.id) {
        bail!("trade finder opponent must differ from the selected team");
    }

    let (outcome, stats_window, _) =
        crate::commands::players::load_repo_for_season(Some(&stats_season), None)?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, stats_window);
    let eligibility = db
        .list_player_eligibility(&league.id)?
        .into_iter()
        .map(|row| (row.player_normalized, row.positions))
        .collect::<HashMap<_, _>>();
    if require_complete {
        let valued_players = all_skaters
            .iter()
            .chain(all_goalies.iter())
            .map(|player| player.identity.name_normalized.clone())
            .collect::<BTreeSet<_>>();
        let mut readiness_positions = all_skaters
            .iter()
            .chain(all_goalies.iter())
            .map(|player| {
                (
                    player.identity.name_normalized.clone(),
                    vec![player.position()],
                )
            })
            .collect::<HashMap<_, _>>();
        readiness_positions.extend(eligibility.clone());
        let capacity = rules.standard_roster_capacity();
        for team in std::iter::once(&user_team).chain(opponents.iter()) {
            let roster = db.list_roster(&team.id)?;
            let (unknown, unvalued, missing) = trade_roster_readiness_counts(
                &roster,
                &readiness_positions,
                &valued_players,
                &rules,
            )?;
            if roster.len() != capacity
                || !unknown.is_empty()
                || !unvalued.is_empty()
                || missing > 0
            {
                bail!(
                    "{} is not trade-ready (roster {}/{}, {} unknown, {} unvalued, {} missing active slots); run `icelines fantasy trade-readiness --team \"{}\"`",
                    team.name,
                    roster.len(),
                    capacity,
                    unknown.len(),
                    unvalued.len(),
                    missing,
                    team.name
                );
            }
        }
    }
    let current_season = Season(CURRENT_SEASON);
    let current_teams = load_current_player_team_map(current_season).ok();
    let (remaining_by_team, remaining_warning) = remaining_games_by_team(current_season).await;
    let schedule_available = !remaining_by_team.is_empty();
    let player_details = all_skaters
        .iter()
        .chain(all_goalies.iter())
        .map(|view| {
            let key = view.identity.name_normalized.clone();
            let league_value = score_team(
                std::slice::from_ref(&key),
                &all_skaters,
                &all_goalies,
                &scheme,
            )
            .first()
            .map(|(_, score)| f64::from(*score))
            .unwrap_or_default();
            let league_value_per_game = if view.gp() == 0 {
                0.0
            } else {
                league_value / f64::from(view.gp())
            };
            let nhl_team = current_teams
                .as_ref()
                .and_then(|map| map.get(&key))
                .cloned()
                .unwrap_or_else(|| view.team_display().to_owned());
            let remaining_games = remaining_by_team.get(&nhl_team).copied().unwrap_or(0);
            (
                key.clone(),
                FantasyTradePlayerEvaluation {
                    player_key: key.clone(),
                    player: view.identity.full_name.clone(),
                    nhl_team,
                    positions: eligibility
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| vec![view.position()]),
                    league_value,
                    league_value_per_game,
                    remaining_games,
                    projected_remaining_value: if schedule_available && remaining_games > 0 {
                        league_value_per_game * f64::from(remaining_games)
                    } else {
                        league_value
                    },
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let mut warnings = Vec::new();
    if let Some(warning) = remaining_warning {
        warnings.push(warning);
    }
    let (quiet_games, schedule_classes) = match load_fantasy_schedule(current_season, false).await {
        Ok(games) => {
            let inputs = games
                .into_iter()
                .filter(|game| game.game_type == 2)
                .map(|game| {
                    Ok(icelines_core::FantasyScheduleGameInput {
                        game_id: game.game_id,
                        date: NaiveDate::parse_from_str(&game.date, "%Y-%m-%d")?,
                        away_team: game.away_abbrev,
                        home_team: game.home_abbrev,
                    })
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let view = build_fantasy_schedule_view(inputs, CURRENT_SEASON, 4, 8, Vec::new())
                .map_err(anyhow::Error::msg)?;
            (
                view.teams
                    .iter()
                    .map(|row| (row.team.clone(), row.quiet_slate_games))
                    .collect::<HashMap<_, _>>(),
                view.teams
                    .iter()
                    .map(|row| (row.team.clone(), row.equivalence_class))
                    .collect::<HashMap<_, _>>(),
            )
        }
        Err(error) => {
            warnings.push(format!(
                "schedule-class fit unavailable; ranking uses value and positions only: {error}"
            ));
            (HashMap::new(), HashMap::new())
        }
    };

    let user_roster = db.list_roster(&user_team.id)?;
    let unresolved_user = user_roster
        .iter()
        .filter(|key| !player_details.contains_key(*key))
        .count();
    if unresolved_user > 0 {
        warnings.push(format!(
            "{unresolved_user} player(s) on {} are absent from the {stats_season} stats pool",
            user_team.name
        ));
    }
    let mut protected_keys = BTreeSet::new();
    if !include_anchors {
        if let Some(anchor) = user_roster
            .iter()
            .filter_map(|key| player_details.get(key))
            .max_by(|left, right| {
                left.projected_remaining_value
                    .total_cmp(&right.projected_remaining_value)
            })
        {
            protected_keys.insert(anchor.player_key.clone());
            warnings.push(format!(
                "automatically protected anchor {}; use --include-anchors to search offers containing that player",
                anchor.player
            ));
        }
    }
    for query in protect {
        let needle = normalize_name(&query);
        let matches = user_roster
            .iter()
            .filter(|key| {
                **key == needle
                    || player_details
                        .get(*key)
                        .is_some_and(|player| normalize_name(&player.player).contains(&needle))
            })
            .cloned()
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [key] => {
                protected_keys.insert(key.clone());
            }
            [] => bail!("protected player '{query}' is not on {}", user_team.name),
            _ => bail!("protected player query '{query}' is ambiguous"),
        }
    }
    let protected_players = protected_keys
        .iter()
        .filter_map(|key| player_details.get(key))
        .map(|player| player.player.clone())
        .collect::<Vec<_>>();
    let user_packages = trade_packages(&user_roster, max_package)
        .into_iter()
        .filter(|package| !package.iter().any(|key| protected_keys.contains(key)))
        .collect::<Vec<_>>();
    let user_missing_before = trade_roster_missing_slots(&user_roster, &player_details, &rules)?;
    let user_active_before = trade_roster_active_value(&user_roster, &player_details, &rules)?;
    let user_schedule_before = trade_roster_schedule_fit(
        &user_roster,
        &player_details,
        &quiet_games,
        &schedule_classes,
    );
    let capacity = rules.standard_roster_capacity();
    if require_complete
        && (user_roster.len() != capacity || unresolved_user > 0 || user_missing_before > 0)
    {
        bail!(
            "{} is not trade-ready (roster {}/{}, {} unresolved, {} missing active slots); run `icelines fantasy trade-readiness --team \"{}\"`",
            user_team.name,
            user_roster.len(),
            capacity,
            unresolved_user,
            user_missing_before,
            user_team.name
        );
    }
    if user_roster.len() < capacity {
        warnings.push(format!(
            "{} has a partial saved roster ({}/{}); offers are provisional until the roster is complete",
            user_team.name,
            user_roster.len(),
            capacity
        ));
    }
    let mut candidates_considered = 0usize;
    let mut partial_opponents = 0usize;
    let mut offers = Vec::new();

    for opponent in opponents {
        let opponent_roster = db.list_roster(&opponent.id)?;
        if opponent_roster.len() < capacity {
            partial_opponents += 1;
        }
        let opponent_packages = trade_packages(&opponent_roster, max_package);
        let opponent_missing_before =
            trade_roster_missing_slots(&opponent_roster, &player_details, &rules)?;
        let opponent_unresolved = opponent_roster
            .iter()
            .filter(|key| !player_details.contains_key(*key))
            .count();
        if require_complete
            && (opponent_roster.len() != capacity
                || opponent_unresolved > 0
                || opponent_missing_before > 0)
        {
            bail!(
                "{} is not trade-ready (roster {}/{}, {} unresolved, {} missing active slots); synchronize the league or omit --require-complete for provisional analysis",
                opponent.name,
                opponent_roster.len(),
                capacity,
                opponent_unresolved,
                opponent_missing_before
            );
        }
        let opponent_active_before =
            trade_roster_active_value(&opponent_roster, &player_details, &rules)?;
        let opponent_schedule_before = trade_roster_schedule_fit(
            &opponent_roster,
            &player_details,
            &quiet_games,
            &schedule_classes,
        );
        for send_keys in &user_packages {
            let Some(sends) = send_keys
                .iter()
                .map(|key| player_details.get(key).cloned())
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            for receive_keys in &opponent_packages {
                let Some(receives) = receive_keys
                    .iter()
                    .map(|key| player_details.get(key).cloned())
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                candidates_considered += 1;
                let sent_value = sends
                    .iter()
                    .map(|player| player.projected_remaining_value)
                    .sum::<f64>();
                let received_value = receives
                    .iter()
                    .map(|player| player.projected_remaining_value)
                    .sum::<f64>();
                let projected_value_delta = received_value - sent_value;
                let comparison_value = sent_value.max(received_value).max(1.0);
                let gap_percent = projected_value_delta / comparison_value * 100.0;
                if gap_percent.abs() > fairness_percent || gap_percent < -2.0 {
                    continue;
                }

                let mut user_after = user_roster
                    .iter()
                    .filter(|key| !send_keys.contains(key))
                    .cloned()
                    .collect::<Vec<_>>();
                user_after.extend(receive_keys.iter().cloned());
                let mut opponent_after = opponent_roster
                    .iter()
                    .filter(|key| !receive_keys.contains(key))
                    .cloned()
                    .collect::<Vec<_>>();
                opponent_after.extend(send_keys.iter().cloned());
                let user_missing_after =
                    trade_roster_missing_slots(&user_after, &player_details, &rules)?;
                let opponent_missing_after =
                    trade_roster_missing_slots(&opponent_after, &player_details, &rules)?;
                if user_after.len() > capacity
                    || opponent_after.len() > capacity
                    || user_missing_after > user_missing_before
                    || opponent_missing_after > opponent_missing_before
                {
                    continue;
                }
                let user_active_after =
                    trade_roster_active_value(&user_after, &player_details, &rules)?;
                let opponent_active_after =
                    trade_roster_active_value(&opponent_after, &player_details, &rules)?;
                let active_lineup_value_delta = user_active_after - user_active_before;
                let opponent_active_lineup_value_delta =
                    opponent_active_after - opponent_active_before;

                let user_schedule_after = trade_roster_schedule_fit(
                    &user_after,
                    &player_details,
                    &quiet_games,
                    &schedule_classes,
                );
                let quiet_delta = user_schedule_after.0 - user_schedule_before.0;
                let class_delta = user_schedule_after.1 - user_schedule_before.1;
                let opponent_schedule_after = trade_roster_schedule_fit(
                    &opponent_after,
                    &player_details,
                    &quiet_games,
                    &schedule_classes,
                );
                let opponent_quiet_delta = opponent_schedule_after.0 - opponent_schedule_before.0;
                let opponent_class_delta = opponent_schedule_after.1 - opponent_schedule_before.1;
                let remaining_games_delta = receives
                    .iter()
                    .map(|player| player.remaining_games as i32)
                    .sum::<i32>()
                    - sends
                        .iter()
                        .map(|player| player.remaining_games as i32)
                        .sum::<i32>();
                let slot_improvement = user_missing_before as i32 - user_missing_after as i32;
                let opponent_slot_improvement =
                    opponent_missing_before as i32 - opponent_missing_after as i32;
                let opponent_projected_value_delta = -projected_value_delta;
                let fit_score = projected_value_delta
                    + active_lineup_value_delta * 10.0
                    + f64::from(quiet_delta) * 0.35
                    + f64::from(class_delta) * 5.0
                    + f64::from(slot_improvement) * 15.0;
                let counterpart_fit_score = opponent_projected_value_delta
                    + opponent_active_lineup_value_delta * 10.0
                    + f64::from(opponent_quiet_delta) * 0.35
                    + f64::from(opponent_class_delta) * 5.0
                    + f64::from(opponent_slot_improvement) * 15.0;
                let mutual_fit_score = fit_score + counterpart_fit_score * 0.25;
                if fit_score < 0.5 || counterpart_fit_score < 0.0 {
                    continue;
                }
                let rationale = format!(
                    "{:+.1} projected value, {:+.2} active-lineup value/game, {:+} quiet-slate games, {:+} schedule classes, {:+} open active slots",
                    projected_value_delta,
                    active_lineup_value_delta,
                    quiet_delta,
                    class_delta,
                    slot_improvement
                );
                offers.push(FantasyTradeOfferCandidate {
                    rank: 0,
                    opponent: opponent.name.clone(),
                    sends: sends.clone(),
                    receives,
                    projected_value_delta,
                    active_lineup_value_delta,
                    opponent_projected_value_delta,
                    opponent_active_lineup_value_delta,
                    value_gap_percent: gap_percent,
                    remaining_games_delta,
                    quiet_slate_games_delta: quiet_delta,
                    schedule_class_diversity_delta: class_delta,
                    opponent_quiet_slate_games_delta: opponent_quiet_delta,
                    opponent_schedule_class_diversity_delta: opponent_class_delta,
                    user_missing_slots_after: user_missing_after,
                    opponent_missing_slots_after: opponent_missing_after,
                    fit_score,
                    counterpart_fit_score,
                    mutual_fit_score,
                    rationale,
                });
            }
        }
    }

    offers.sort_by(|left, right| {
        right
            .mutual_fit_score
            .total_cmp(&left.mutual_fit_score)
            .then_with(|| {
                right
                    .projected_value_delta
                    .total_cmp(&left.projected_value_delta)
            })
            .then_with(|| left.opponent.cmp(&right.opponent))
    });
    if partial_opponents > 0 {
        warnings.push(format!(
            "{partial_opponents} searched opponent roster(s) are partial; their offers are provisional"
        ));
    }
    offers.truncate(top);
    for (index, offer) in offers.iter_mut().enumerate() {
        offer.rank = index + 1;
    }
    if offers.is_empty() {
        warnings.push(
            "no mutually plausible legal offers matched the current fairness threshold".to_owned(),
        );
    }
    let view = FantasyTradeFinderView {
        schema: "fantasy_trade_finder.v1".to_owned(),
        league: league.name,
        scoring_scheme: league.scheme,
        team: user_team.name,
        max_package,
        fairness_percent,
        require_complete,
        protected_players,
        candidates_considered,
        offers,
        warnings,
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&view)?);
    } else {
        println!(
            "\n{} — {} · {} ({})",
            BOARDS_TRADE_FINDER_HEADER, view.team, view.league, view.scoring_scheme
        );
        println!(
            "  considered {} packages · fair gap ±{:.1}% · {} · showing {}",
            view.candidates_considered,
            view.fairness_percent,
            if view.require_complete {
                "complete rosters required"
            } else {
                "provisional rosters allowed"
            },
            view.offers.len()
        );
        if !view.protected_players.is_empty() {
            println!("  protected: {}", view.protected_players.join(", "));
        }
        for offer in &view.offers {
            println!(
                "\n  {:>2}. {} — send {} · receive {}",
                offer.rank,
                offer.opponent,
                offer
                    .sends
                    .iter()
                    .map(|player| player.player.as_str())
                    .collect::<Vec<_>>()
                    .join(" + "),
                offer
                    .receives
                    .iter()
                    .map(|player| player.player.as_str())
                    .collect::<Vec<_>>()
                    .join(" + ")
            );
            println!(
                "      mutual {:>+7.1} · my fit {:>+7.1} · value {:>+7.1} ({:>+5.1}%) · active {:>+5.2}/g",
                offer.mutual_fit_score,
                offer.fit_score,
                offer.projected_value_delta,
                offer.value_gap_percent,
                offer.active_lineup_value_delta
            );
            println!(
                "      games {:>+3} · quiet {:>+3} · classes {:>+2}",
                offer.remaining_games_delta,
                offer.quiet_slate_games_delta,
                offer.schedule_class_diversity_delta
            );
            println!(
                "      counterpart fit {:>+7.1} · value {:>+7.1} · active {:>+5.2}/g · quiet {:>+3} · classes {:>+2}",
                offer.counterpart_fit_score,
                offer.opponent_projected_value_delta,
                offer.opponent_active_lineup_value_delta,
                offer.opponent_quiet_slate_games_delta,
                offer.opponent_schedule_class_diversity_delta
            );
        }
        for warning in &view.warnings {
            println!("  warning: {warning}");
        }
        if !view.offers.is_empty() {
            println!("\n(re-run a candidate with `fantasy trade` for the full two-team audit)");
        }
    }
    Ok(())
}

fn icelines_data_root() -> anyhow::Result<std::path::PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    Ok(home.join(".icelines").join("data"))
}

// ── HTTP server ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    db_path: std::path::PathBuf,
    league_name: Option<String>,
}

impl AppState {
    fn open_db(&self) -> anyhow::Result<FantasyDb> {
        FantasyDb::open_path(self.db_path.clone())
    }
}

// ── Helper to load league + scheme for handlers ───────────────────────────────

fn get_league_and_scheme(
    db: &FantasyDb,
    league_name: &Option<String>,
) -> anyhow::Result<(LeagueRow, Scheme)> {
    let league = require_league(db, league_name)?;
    let scheme = resolve_scheme(&league.scheme)?;
    Ok((league, scheme))
}

// ── Route handlers ────────────────────────────────────────────────────────────

async fn handle_root(
    State(state): State<Arc<AppState>>,
) -> Result<axum::response::Html<String>, (StatusCode, String)> {
    let db = state
        .open_db()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (league, scheme) = get_league_and_scheme(&db, &state.league_name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (outcome, season) =
        load_pools().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);
    let teams = db
        .list_teams(&league.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Build standings rows.
    let mut standings: Vec<(String, String, f32)> = Vec::new();
    for team in &teams {
        let roster = db
            .list_roster(&team.id)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let total: f32 = score_team(&roster, &all_skaters, &all_goalies, &scheme)
            .iter()
            .map(|(_, s)| s)
            .sum();
        standings.push((team.name.clone(), team.owner.clone(), total));
    }
    standings.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    let rows: String = standings
        .iter()
        .enumerate()
        .map(|(i, (name, owner, score))| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.1}</td></tr>",
                i + 1,
                name,
                owner,
                score
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>IceLines Fantasy — {league_name}</title>
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
            background: #0d1117; color: #c9d1d9; max-width: 900px; margin: 40px auto; padding: 0 20px; }}
    h1 {{ color: #58a6ff; border-bottom: 1px solid #30363d; padding-bottom: 12px; }}
    table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
    th {{ background: #161b22; color: #8b949e; text-align: left; padding: 10px 14px; border-bottom: 2px solid #30363d; }}
    td {{ padding: 10px 14px; border-bottom: 1px solid #21262d; }}
    tr:hover td {{ background: #161b22; }}
    .badge {{ background: #1f6feb; color: #fff; padding: 2px 8px; border-radius: 12px; font-size: 0.8em; }}
    footer {{ margin-top: 40px; color: #8b949e; font-size: 0.85em; }}
  </style>
</head>
<body>
  <h1>IceLines Fantasy <span class="badge">{league_name}</span></h1>
  <p>Scoring scheme: <strong>{scheme_name}</strong></p>
  <table>
    <thead>
      <tr><th>Rank</th><th>Team</th><th>Owner</th><th>Score</th></tr>
    </thead>
    <tbody>
{rows}
    </tbody>
  </table>
  <footer>
    Refresh stats: <code>icelines fetch stats</code> &nbsp;|&nbsp;
    IceLines Fantasy Server
  </footer>
</body>
</html>"#,
        league_name = league.name,
        scheme_name = league.scheme,
        rows = rows,
    );

    Ok(axum::response::Html(html))
}

async fn handle_api_standings(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let db = state
        .open_db()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (league, scheme) = get_league_and_scheme(&db, &state.league_name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (outcome, season) =
        load_pools().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);
    let teams = db
        .list_teams(&league.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut result: Vec<Value> = Vec::new();
    for team in &teams {
        let roster = db
            .list_roster(&team.id)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let scored = score_team(&roster, &all_skaters, &all_goalies, &scheme);
        let total: f32 = scored.iter().map(|(_, s)| s).sum();

        let players_json: Vec<Value> = scored
            .iter()
            .map(|(name, score)| {
                let norm = normalize_name(name);
                let v = all_skaters
                    .iter()
                    .find(|v| v.identity.name_normalized.contains(norm.as_str()));
                json!({
                    "name": name,
                    "pos": v.map(|view| view.position().abbreviation()).unwrap_or("—"),
                    "gp": v.map(|view| view.gp()).unwrap_or(0),
                    "score": score,
                })
            })
            .collect();

        result.push(json!({
            "team": team.name,
            "owner": team.owner,
            "is_user_team": team.is_user_team,
            "score": total,
            "players": players_json,
        }));
    }

    result.sort_by(|a, b| {
        let sa = a["score"].as_f64().unwrap_or(0.0);
        let sb = b["score"].as_f64().unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(Json(json!(result)))
}

async fn handle_api_teams(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let db = state
        .open_db()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (league, _) = get_league_and_scheme(&db, &state.league_name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let teams = db
        .list_teams(&league.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let list: Vec<Value> = teams
        .iter()
        .map(|t| {
            json!({
                "id": t.id,
                "name": t.name,
                "owner": t.owner,
                "is_user_team": t.is_user_team,
                "player_count": t.player_count,
            })
        })
        .collect();

    Ok(Json(json!(list)))
}

/// Phase Lindsay L.5.6 — build the per-player JSON record for the
/// `/api/team/:name/roster` response. Every stat field is keyed by
/// `StatId::cli_key()`. KEEL-B1 round-trip contract: every emitted key
/// parses back via `StatId::from_cli_key`; values match `StatId::read(view)`.
///
/// **Top-level keys** (identity / derived — NOT stat reads):
///   - `name`, `pos`, `score` (fantasy score is a derived metric)
///
/// **`stats` sub-object** (every key is a `StatId::cli_key`):
///   - Games, Goals, Assists, Points (initial v1 corpus, L.5.6)
///   - Hits, BlockedShots, PlusMinus, ShootingPct, TotalToiPerGame,
///     FaceoffWinPct (C4 L.5b post-fix expansion — gives consumers
///     enough breadth to render a useful skater card)
///
/// **WIRE-2 (L.5b post-fix)**: the top-level `gp` key was removed.
/// Games count now lives only at `stats.games` (single source of
/// truth, keyed by StatId::cli_key). API consumers reading `gp` must
/// migrate to `stats.games`. The wrapping response carries
/// `schema_version: 1` (WIRE-1) so any future shape change is
/// negotiable rather than silent.
///
/// Extracted as a pure helper (no DB, no async) so the JSON shape is
/// L0-testable without spinning up the axum server.
pub(crate) fn build_roster_player_json(
    name: &str,
    score: f32,
    view: Option<&PlayerView<'_>>,
) -> Value {
    use icelines_core::stats_catalog::StatId;
    // C4 L.5b post-fix expansion — extended from 4 to 10 stats so the
    // roster response carries enough breadth to render a player card
    // without a second round-trip.
    const ROSTER_STATS: &[StatId] = &[
        StatId::Games,
        StatId::Goals,
        StatId::Assists,
        StatId::Points,
        StatId::PlusMinus,
        StatId::Shots,
        StatId::ShootingPct,
        StatId::TotalToiPerGame,
        StatId::Hits,
        StatId::BlockedShots,
        StatId::FaceoffWinPct,
    ];

    let mut stats_map = serde_json::Map::new();
    if let Some(v) = view {
        for sid in ROSTER_STATS {
            let val = sid.read(v);
            stats_map.insert(
                sid.cli_key().to_owned(),
                match val {
                    Some(x) => json!(x),
                    None => Value::Null,
                },
            );
        }
    }

    json!({
        "name": name,
        "pos": view.map(|v| v.position().abbreviation()).unwrap_or("—"),
        "score": score,
        "stats": Value::Object(stats_map),
    })
}

async fn handle_api_team_roster(
    State(state): State<Arc<AppState>>,
    Path(team_name): Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let db = state
        .open_db()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (league, scheme) = get_league_and_scheme(&db, &state.league_name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let team = require_team(&db, &league.id, &team_name)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let (outcome, season) =
        load_pools().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);
    let roster = db
        .list_roster(&team.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let scored = score_team(&roster, &all_skaters, &all_goalies, &scheme);

    let players_json: Vec<Value> = scored
        .iter()
        .map(|(name, score)| {
            let norm = normalize_name(name);
            let v = all_skaters
                .iter()
                .find(|v| v.identity.name_normalized.contains(norm.as_str()));
            build_roster_player_json(name, *score, v)
        })
        .collect();

    // WIRE-1 (L.5b post-fix) — `schema_version` field on the response
    // top-level. Forward-compat versioning before any consumer hardens
    // against the shape; future shape changes (rename a key, replace
    // `players` with a paginated envelope, etc.) bump this value.
    Ok(Json(json!({
        "schema_version": 1,
        "team": team.name,
        "owner": team.owner,
        "players": players_json,
    })))
}

async fn handle_api_team_add(
    State(state): State<Arc<AppState>>,
    Path(team_name): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let player_q = body["player"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing 'player' field".to_owned()))?
        .to_owned();

    let db = state
        .open_db()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (league, _) = get_league_and_scheme(&db, &state.league_name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let team = require_team(&db, &league.id, &team_name)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let (outcome, season) =
        load_pools().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (all_skaters, _all_goalies) = pools_views(&outcome.repo, season);

    let player = fuzzy_find_skater(&all_skaters, &player_q)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let norm = player.identity.name_normalized.clone();
    let full_name = player.identity.full_name.clone();

    if let Some(taken) = db
        .is_on_any_team(&league.id, &norm)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        return Err((
            StatusCode::CONFLICT,
            format!("'{full_name}' is already on team '{taken}'"),
        ));
    }

    db.add_player(&team.id, &norm)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        json!({ "status": "added", "player": full_name, "team": team_name }),
    ))
}

async fn handle_api_team_drop(
    State(state): State<Arc<AppState>>,
    Path(team_name): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let player_q = body["player"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing 'player' field".to_owned()))?
        .to_owned();

    let db = state
        .open_db()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (league, _) = get_league_and_scheme(&db, &state.league_name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let team = require_team(&db, &league.id, &team_name)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let (outcome, season) =
        load_pools().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (all_skaters, _all_goalies) = pools_views(&outcome.repo, season);

    let player = fuzzy_find_skater(&all_skaters, &player_q)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let norm = player.identity.name_normalized.clone();
    let full_name = player.identity.full_name.clone();

    let dropped = db
        .drop_player(&team.id, &norm)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if dropped {
        Ok(Json(
            json!({ "status": "dropped", "player": full_name, "team": team_name }),
        ))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            format!("'{full_name}' is not on team '{team_name}'"),
        ))
    }
}

async fn handle_api_trade(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let player1_q = body["player1"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing 'player1'".to_owned()))?
        .to_owned();
    let to_team = body["to_team"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing 'to_team'".to_owned()))?
        .to_owned();
    let player2_q = body["player2"]
        .as_str()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing 'player2'".to_owned()))?
        .to_owned();

    let db = state
        .open_db()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (league, scheme) = get_league_and_scheme(&db, &state.league_name)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (outcome, season) =
        load_pools().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);

    let p1 = fuzzy_find_skater(&all_skaters, &player1_q)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let p2 = fuzzy_find_skater(&all_skaters, &player2_q)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let p1_norm = p1.identity.name_normalized.clone();
    let p2_norm = p2.identity.name_normalized.clone();
    let p1_name = p1.identity.full_name.clone();
    let p2_name = p2.identity.full_name.clone();

    let team1_name = db
        .is_on_any_team(&league.id, &p1_norm)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("'{p1_name}' is not on any team"),
            )
        })?;
    let team1 = require_team(&db, &league.id, &team1_name)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let team2 = require_team(&db, &league.id, &to_team)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    let roster1 = db
        .list_roster(&team1.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let roster2 = db
        .list_roster(&team2.id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let score1_before: f32 = score_team(&roster1, &all_skaters, &all_goalies, &scheme)
        .iter()
        .map(|(_, s)| s)
        .sum();
    let score2_before: f32 = score_team(&roster2, &all_skaters, &all_goalies, &scheme)
        .iter()
        .map(|(_, s)| s)
        .sum();

    let roster1_after: Vec<String> = roster1
        .iter()
        .map(|n| {
            if n == &p1_norm {
                p2_norm.clone()
            } else {
                n.clone()
            }
        })
        .collect();
    let roster2_after: Vec<String> = roster2
        .iter()
        .map(|n| {
            if n == &p2_norm {
                p1_norm.clone()
            } else {
                n.clone()
            }
        })
        .collect();
    let score1_after: f32 = score_team(&roster1_after, &all_skaters, &all_goalies, &scheme)
        .iter()
        .map(|(_, s)| s)
        .sum();
    let score2_after: f32 = score_team(&roster2_after, &all_skaters, &all_goalies, &scheme)
        .iter()
        .map(|(_, s)| s)
        .sum();

    Ok(Json(json!({
        "trade": {
            "player1": p1_name,
            "from_team": team1_name,
            "player2": p2_name,
            "to_team": to_team,
        },
        "before": {
            team1_name.clone(): score1_before,
            to_team.clone(): score2_before,
        },
        "after": {
            team1_name.clone(): score1_after,
            to_team.clone(): score2_after,
        },
        "delta": {
            team1_name.clone(): score1_after - score1_before,
            to_team.clone(): score2_after - score2_before,
        },
    })))
}

// ── Serve ─────────────────────────────────────────────────────────────────────

/// `icelines fantasy serve [--port <port>] [--league <league>]`
pub async fn run_serve(port: u16, league_override: Option<String>) -> anyhow::Result<()> {
    // Determine the DB path the same way FantasyDb::open() does.
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let db_path = home.join(".icelines").join("icelines.db");

    // Validate connection before starting.
    {
        let db = FantasyDb::open_path(db_path.clone())?;
        let league = require_league(&db, &league_override)?;
        let league_display = league.name.clone();
        println!("Legacy fantasy server running at http://0.0.0.0:{port}");
        println!("League: {league_display} | Press Ctrl-C to stop");
        println!(
            "Note: use `icelines serve` for /fantasy roster gaps and simulation parity views."
        );
    }

    let state = Arc::new(AppState {
        db_path,
        league_name: league_override,
    });

    let app = axum::Router::new()
        .route("/", get(handle_root))
        .route("/api/standings", get(handle_api_standings))
        .route("/api/teams", get(handle_api_teams))
        .route("/api/team/:name/roster", get(handle_api_team_roster))
        .route("/api/team/:name/add", post(handle_api_team_add))
        .route("/api/team/:name/drop", post(handle_api_team_drop))
        .route("/api/trade", post(handle_api_trade))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{fixtures, identity::PlayerId};

    #[test]
    fn category_rule_spec_preserves_direction_aggregation_and_epsilon() {
        let rule = parse_category_rule_spec("goals-against-average:lower:ratio:0.001")
            .expect("parse category rule");
        assert_eq!(rule.key, "goals_against_average");
        assert_eq!(rule.direction, FantasyCategoryDirection::LowerWins);
        assert_eq!(rule.aggregation, FantasyCategoryAggregation::Ratio);
        assert_eq!(rule.tie_epsilon, 0.001);
        assert!(parse_category_rule_spec("goals:higher").is_err());
    }

    #[test]
    fn trade_legality_respects_goalie_slots_and_multi_position_forwards() {
        let rules = FantasyAssistantRules::configured_2026();
        let specifications = [
            ("c1", vec![Position::Center]),
            ("c2", vec![Position::Center]),
            ("util", vec![Position::Center]),
            ("lw1", vec![Position::LeftWing]),
            ("lw2", vec![Position::LeftWing]),
            ("rw1", vec![Position::RightWing]),
            ("rw2", vec![Position::RightWing]),
            ("d1", vec![Position::Defense]),
            ("d2", vec![Position::Defense]),
            ("d3", vec![Position::Defense]),
            ("g1", vec![Position::Goalie]),
            ("g2", vec![Position::Goalie]),
            ("flex", vec![Position::Center, Position::RightWing]),
            ("c4", vec![Position::Center]),
        ];
        let players = specifications
            .into_iter()
            .map(|(key, positions)| {
                (
                    key.to_owned(),
                    FantasyTradePlayerEvaluation {
                        player_key: key.to_owned(),
                        player: key.to_owned(),
                        nhl_team: "NYR".to_owned(),
                        positions,
                        league_value: 1.0,
                        league_value_per_game: 1.0,
                        remaining_games: 84,
                        projected_remaining_value: 84.0,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        let complete = [
            "c1", "c2", "util", "lw1", "lw2", "rw1", "rw2", "d1", "d2", "d3", "g1", "g2",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        let positions = players
            .iter()
            .map(|(key, player)| (key.clone(), player.positions.clone()))
            .collect::<HashMap<_, _>>();
        let valued = players.keys().cloned().collect::<BTreeSet<_>>();

        assert_eq!(
            trade_roster_missing_slots(&complete, &players, &rules).unwrap(),
            0
        );
        assert_eq!(
            trade_roster_active_value(&complete, &players, &rules).unwrap(),
            12.0
        );
        assert_eq!(
            trade_roster_readiness_counts(&complete, &positions, &valued, &rules).unwrap(),
            (Vec::new(), Vec::new(), 0)
        );

        let flex_swap = complete
            .iter()
            .map(|key| {
                if key == "rw2" {
                    "flex".to_owned()
                } else {
                    key.clone()
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            trade_roster_missing_slots(&flex_swap, &players, &rules).unwrap(),
            0,
            "C/RW eligibility must preserve the RW slot"
        );

        let goalie_for_center = complete
            .iter()
            .map(|key| {
                if key == "g2" {
                    "c4".to_owned()
                } else {
                    key.clone()
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            trade_roster_missing_slots(&goalie_for_center, &players, &rules).unwrap(),
            1,
            "a same-size package cannot hide the loss of a required goalie"
        );
        assert_eq!(
            trade_roster_active_value(&goalie_for_center, &players, &rules).unwrap(),
            11.0,
            "the active-lineup score must exclude the extra center that cannot fill goalie"
        );
    }

    #[test]
    fn trade_finder_builds_unique_single_and_two_player_packages() {
        let roster = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        assert_eq!(trade_packages(&roster, 1).len(), 3);
        assert_eq!(
            trade_packages(&roster, 2),
            vec![
                vec!["a".to_owned()],
                vec!["b".to_owned()],
                vec!["c".to_owned()],
                vec!["a".to_owned(), "b".to_owned()],
                vec!["a".to_owned(), "c".to_owned()],
                vec!["b".to_owned(), "c".to_owned()],
            ]
        );
    }

    /// Hart.5c.4 cold-start parity (per spec D5).
    ///
    /// A view with no realtime tier (cold-start) must produce a
    /// `scheme::SkaterStats` whose realtime fields are zero — preserving
    /// the legacy behavior where `to_scheme_stats(&player)` got
    /// `player.hits/blocked_shots/takeaways/giveaways` zero-initialized
    /// from `player_from_view(view).realtime.unwrap_or(zero)`. If a
    /// future Option-aware fantasy rewrite changes this contract (e.g.
    /// promotes None to a "missing data" signal instead of 0), this
    /// test surfaces the change.
    #[test]
    fn l0_hart5c4_to_scheme_stats_view_cold_start_zeroes_realtime() {
        let id = fixtures::identity(8478402).build();
        // Default `fixtures::stats(...)` does NOT call .realtime(...) so
        // realtime is None → cold-start path.
        let stats = fixtures::stats(8478402, 20242025, "EDM").build();
        assert!(stats.realtime.is_none(), "fixture must be cold-start");
        let repo = fixtures::test_repo_with(id, stats);
        let v = repo
            .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
            .unwrap();

        let s = icelines_core::skater_scheme_stats_from_view(&v);
        assert_eq!(s.hits, 0, "cold-start hits must map to 0");
        assert_eq!(s.blocks, 0, "cold-start blocks must map to 0");
        assert_eq!(s.takeaways, 0, "cold-start takeaways must map to 0");
        assert_eq!(s.giveaways, 0, "cold-start giveaways must map to 0");
        // Counter fields from totals are still populated.
        assert_eq!(s.goals, 30, "fixture goals = 30");
        assert_eq!(s.assists, 50, "fixture assists = 50");
    }

    /// Realtime present: view's hits etc. flow through to SkaterStats.
    #[test]
    fn l0_hart5c4_to_scheme_stats_view_with_realtime_passes_through() {
        let id = fixtures::identity(8478402).build();
        let stats = fixtures::stats(8478402, 20242025, "EDM")
            .realtime(48, 22, 65, 41) // hits, blocks, takeaways, giveaways
            .build();
        let repo = fixtures::test_repo_with(id, stats);
        let v = repo
            .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
            .unwrap();

        let s = icelines_core::skater_scheme_stats_from_view(&v);
        assert_eq!(s.hits, 48);
        assert_eq!(s.blocks, 22);
        assert_eq!(s.takeaways, 65);
        assert_eq!(s.giveaways, 41);
    }

    #[test]
    fn l0_fantasy_sim_projection_uses_resolved_remaining_games() {
        let id = fixtures::identity(8478402).build();
        let stats = fixtures::stats(8478402, 20242025, "EDM").build();
        let repo = fixtures::test_repo_with(id, stats);
        let skaters: Vec<PlayerView<'_>> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let goalies: Vec<PlayerView<'_>> = Vec::new();
        let roster = vec!["connor mcdavid".to_string()];
        let mut remaining = HashMap::new();
        remaining.insert("EDM".to_string(), 10);

        assert_eq!(
            icelines_core::fantasy_roster_games_remaining(&roster, &skaters, &goalies, &remaining),
            10
        );
        assert!(
            icelines_core::project_fantasy_roster_score(
                100.0, &roster, &skaters, &goalies, &remaining
            ) > 100.0
        );
    }

    /// Hart.5c.4: `score_team` end-to-end on a mixed roster.
    /// Builds 1 skater + 1 goalie in a fresh repo; rosters 3 names
    /// (skater hit, goalie hit, missing). Asserts:
    /// - 2 results (missing player is dropped, not zero-scored)
    /// - Results are sorted by score descending
    /// - Both real players appear with non-empty names
    ///
    /// Pinned to catch regressions in the dual-pool lookup contract:
    /// the post-Hart.5c shape must still treat the goalie pool as
    /// fallback-only and silently skip rows that match neither.
    #[test]
    fn l0_hart5c4_score_team_mixes_skater_goalie_and_skips_missing() {
        use icelines_core::TeamAbbr;
        let mut repo = StatsRepository::new();

        // Skater: McDavid-shape default fixture, normalized name "mcdavid".
        let skater_id = fixtures::identity(8478402)
            .name("McDavid", "mcdavid")
            .build();
        let skater_stats = fixtures::stats(8478402, 20242025, "EDM").build();
        repo.upsert_identity(skater_id).unwrap();
        repo.upsert_stats(skater_stats).unwrap();

        // Goalie: solo_goalie fixture (50 GP, 30W, 18L, .913 SV%).
        let goalie_id = fixtures::identity(8478406)
            .name("Test Goalie", "test_goalie")
            .build();
        let goalie_stats = fixtures::solo_goalie(8478406, 20242025, TeamAbbr("OTT".into())).build();
        repo.upsert_identity(goalie_id).unwrap();
        repo.upsert_stats(goalie_stats).unwrap();

        let skaters: Vec<PlayerView<'_>> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let goalies: Vec<PlayerView<'_>> = repo
            .goalies(Season(20242025), SeasonType::Regular)
            .collect();
        assert_eq!(
            skaters.len(),
            1,
            "must have exactly one skater in fixture repo"
        );
        assert_eq!(
            goalies.len(),
            1,
            "must have exactly one goalie in fixture repo"
        );

        let roster = vec![
            "mcdavid".to_string(),
            "test_goalie".to_string(),
            "definitely_missing".to_string(),
        ];
        let scheme = Scheme::yahoo_standard();
        let results = score_team(&roster, &skaters, &goalies, &scheme);

        assert_eq!(
            results.len(),
            2,
            "missing roster entry must be skipped, not zero-scored: {results:?}"
        );
        // Sorted descending by score.
        assert!(
            results[0].1 >= results[1].1,
            "results must be sorted desc by score: {results:?}"
        );
        // Both real players appear; names come from view.identity.full_name.
        let names: Vec<&str> = results.iter().map(|(n, _)| n.as_str()).collect();
        assert!(
            names.contains(&"McDavid"),
            "missing skater in results: {names:?}"
        );
        assert!(
            names.contains(&"Test Goalie"),
            "missing goalie in results: {names:?}"
        );
        // Both real players score > 0 on Yahoo standard (a 30G/50A skater
        // and a 30W goalie both have positive components).
        assert!(results.iter().all(|(_, s)| *s > 0.0));
    }

    /// Hart.5c.7: the parity test that compared
    /// `to_scheme_stats_view(view)` against
    /// `to_scheme_stats(&flat_view_legacy(view))` was the load-bearing
    /// safety net during the 5c.4 migration. With the legacy path now
    /// deleted, the parity test had no second path to compare against
    /// and was removed. The view path is exercised by every other
    /// fantasy test in this module that calls to_scheme_stats_view
    /// against fixture views (Beniers known-value asserts, etc.).
    #[test]
    fn l0_to_scheme_stats_view_pins_known_field_mapping() {
        // Pinned known-value: McDavid 2024-25 fixture realtime block
        // (hits=48, blocked_shots=22, takeaways=65, giveaways=41).
        // Plus the default StatTotals fixture (goals=30, assists=50).
        let id = fixtures::identity(8478402).build();
        let stats = fixtures::stats(8478402, 20242025, "EDM")
            .realtime(48, 22, 65, 41)
            .build();
        let repo = fixtures::test_repo_with(id, stats);
        let v = repo
            .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
            .unwrap();
        let s = icelines_core::skater_scheme_stats_from_view(&v);
        assert_eq!(s.goals, 30);
        assert_eq!(s.assists, 50);
        assert_eq!(s.hits, 48);
        assert_eq!(s.blocks, 22);
        assert_eq!(s.takeaways, 65);
        assert_eq!(s.giveaways, 41);
    }

    // ── Phase Lindsay L.5.6 — axum roster JSON keys via StatId::cli_key ──

    /// KEEL-B1 round-trip — every key emitted in the roster `stats`
    /// sub-object parses back via `StatId::from_cli_key`. This is the
    /// contract: API consumers can call `StatId::from_cli_key(key)` on
    /// any key in `players[*].stats` and get a Some.
    #[test]
    fn l0_lindsay_l5_roster_stats_keys_round_trip_via_from_cli_key() {
        use icelines_core::stats_catalog::StatId;
        // Mirror the `roster_stats` slice in handle_api_team_roster.
        // If the slice changes there, this test must change too — that's
        // the explicit contract pin.
        let roster_stats: &[StatId] = &[
            StatId::Games,
            StatId::Goals,
            StatId::Assists,
            StatId::Points,
        ];
        for sid in roster_stats {
            let key = sid.cli_key();
            let parsed = StatId::from_cli_key(key);
            assert_eq!(
                parsed,
                Some(*sid),
                "KEEL-B1: roster JSON key `{key}` must round-trip via \
                 StatId::from_cli_key"
            );
        }
    }

    /// L.5.6 (gap-fill) — `build_roster_player_json` for a player WITH
    /// a matched view emits the full shape: name, pos, gp, score, and
    /// a populated `stats` object keyed by `StatId::cli_key`.
    #[test]
    fn l0_lindsay_l5_build_roster_player_json_with_view() {
        use icelines_core::model::{Position, Season, TeamAbbr};
        use icelines_core::season_stats::SeasonType;
        use icelines_core::season_stats::{SeasonStatsBuilder, TeamStint};
        use icelines_core::stats_catalog::StatId;

        let identity = fixtures::identity(8478402).build();
        let stats = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            goalie: None,
        })
        .with_totals(icelines_core::season_stats::StatTotals {
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            ..Default::default()
        })
        .build();
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };

        let json = build_roster_player_json("Connor McDavid", 42.5, Some(&view));
        // Top-level keys (post-WIRE-2: `gp` removed; lives at stats.games).
        assert_eq!(json["name"], "Connor McDavid");
        assert_eq!(json["pos"], "C");
        assert_eq!(json["score"], 42.5);
        assert!(
            json.get("gp").is_none(),
            "WIRE-2: top-level `gp` removed — use stats.games"
        );
        // The `stats` object: every key parses via `StatId::from_cli_key`,
        // values match `StatId::read(&view)`.
        let stats_obj = json["stats"]
            .as_object()
            .expect("stats sub-object must be present");
        for (key, value) in stats_obj {
            let sid = StatId::from_cli_key(key)
                .unwrap_or_else(|| panic!("KEEL-B1: emitted key `{key}` does not parse"));
            let expected = sid.read(&view);
            match (expected, value) {
                (Some(x), v) => {
                    let got = v
                        .as_f64()
                        .unwrap_or_else(|| panic!("`{key}` must be number"));
                    assert!(
                        (x - got).abs() < 1e-9,
                        "value mismatch for `{key}`: expected={x} got={got}"
                    );
                }
                (None, Value::Null) => {}
                (e, v) => panic!("shape mismatch for `{key}`: expected={e:?} got={v:?}"),
            }
        }
        // Core stats round-trip cleanly (stored as f64 since catalog
        // reads return Option<f64>).
        assert_eq!(stats_obj["games"].as_f64(), Some(70.0));
        assert_eq!(stats_obj["goals"].as_f64(), Some(30.0));
        assert_eq!(stats_obj["assists"].as_f64(), Some(80.0));
        assert_eq!(stats_obj["points"].as_f64(), Some(110.0));
        // C4 (L.5b expansion) — broader stat slice present.
        assert!(
            stats_obj.contains_key("hits"),
            "C4: hits key present in expanded slice"
        );
        assert!(
            stats_obj.contains_key("blocked-shots"),
            "C4: blocked-shots key present"
        );
        assert!(
            stats_obj.contains_key("plus-minus"),
            "C4: plus-minus key present"
        );
        assert!(
            stats_obj.contains_key("faceoff-win-pct"),
            "C4: faceoff-win-pct key present"
        );
    }

    /// L.5.6 (gap-fill) — `build_roster_player_json` for a player
    /// WITHOUT a matched view (cold-start: name in roster but no view
    /// in pool) emits sentinel values: pos="—", empty stats map.
    /// Top-level `gp` is gone post-WIRE-2.
    #[test]
    fn l0_lindsay_l5_build_roster_player_json_no_view() {
        let json = build_roster_player_json("Phantom Player", 0.0, None);
        assert_eq!(json["name"], "Phantom Player");
        assert_eq!(json["pos"], "—");
        assert_eq!(json["score"], 0.0);
        assert!(json.get("gp").is_none(), "WIRE-2: top-level `gp` removed");
        // Empty stats map — no entries since no view was matched.
        let stats_obj = json["stats"]
            .as_object()
            .expect("stats sub-object always present, even when empty");
        assert!(
            stats_obj.is_empty(),
            "no view → no catalog reads → empty stats map"
        );
    }

    /// L.5.6 — emitted stats values match `StatId::read(view)` for the
    /// same view. Tests the read-side contract for the round-trip:
    /// consumers can re-derive the value via the catalog and get the
    /// same number we emitted.
    #[test]
    fn l0_lindsay_l5_roster_stats_values_match_stat_id_read() {
        use icelines_core::model::{Position, Season, TeamAbbr};
        use icelines_core::season_stats::SeasonType;
        use icelines_core::season_stats::{SeasonStatsBuilder, TeamStint};
        use icelines_core::stats_catalog::StatId;
        use icelines_core::stats_repository::PlayerView;

        let identity = fixtures::identity(8478402).build();
        let stats = SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            Position::Center,
        )
        .add_team_stint(TeamStint {
            team: TeamAbbr("EDM".into()),
            started: Some("2024-10-09".into()),
            ended: None,
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            goalie: None,
        })
        .with_totals(icelines_core::season_stats::StatTotals {
            gp: 70,
            goals: 30,
            assists: 80,
            points: 110,
            ..Default::default()
        })
        .build();
        let view = PlayerView {
            identity: &identity,
            stats: &stats,
            contract: None,
        };

        // Emitted JSON value for `StatId::Games` should equal Some(70.0).
        assert_eq!(StatId::Games.read(&view), Some(70.0));
        assert_eq!(StatId::Goals.read(&view), Some(30.0));
        assert_eq!(StatId::Assists.read(&view), Some(80.0));
        assert_eq!(StatId::Points.read(&view), Some(110.0));
    }

    #[test]
    fn weekly_simulation_counts_only_players_who_fit_active_slots() {
        let better = DraftPoolPlayer {
            key: "better".to_owned(),
            player: "Better Center".to_owned(),
            team: "NYR".to_owned(),
            positions: vec![Position::Center],
            quality: 82.0,
            games_played: 82,
        };
        let bench = DraftPoolPlayer {
            key: "bench".to_owned(),
            player: "Bench Center".to_owned(),
            team: "BOS".to_owned(),
            positions: vec![Position::Center],
            quality: 41.0,
            games_played: 82,
        };
        let pool = HashMap::from([(better.key.clone(), &better), (bench.key.clone(), &bench)]);
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let schedules = HashMap::from([
            ("NYR".to_owned(), BTreeSet::from([date])),
            ("BOS".to_owned(), BTreeSet::from([date])),
        ]);
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(icelines_core::FantasyActiveSlotKind::Center, 1)]);
        rules.bench_slots = 1;

        let result = simulate_weekly_roster(
            &[better.key.clone(), bench.key.clone()],
            &pool,
            &schedules,
            &[date],
            &rules,
        )
        .unwrap();

        assert_eq!(result.usable_starts, 1);
        assert_eq!(result.player_values.get("better"), Some(&1.0));
        assert!(!result.player_values.contains_key("bench"));
    }

    #[test]
    fn previous_season_id_preserves_nhl_season_shape() {
        assert_eq!(previous_season_id("20252026").unwrap(), "20242025");
        assert!(previous_season_id("20252027").is_err());
        assert!(previous_season_id("2025").is_err());
    }

    #[test]
    fn comparison_matrix_uses_baseline_then_parity_then_first_as_reference() {
        assert_eq!(
            matrix_reference_index(&["clean", "baseline", "chaos"]),
            Some(1)
        );
        assert_eq!(
            matrix_reference_index(&["parity", "edge-15", "edge-30"]),
            Some(0)
        );
        assert_eq!(
            matrix_reference_index(&["strict", "adaptive", "all-in"]),
            Some(2)
        );
        assert_eq!(matrix_reference_index(&["custom"]), Some(0));
        assert_eq!(matrix_reference_index(&[]), None);
    }

    #[test]
    fn season_event_messages_replace_internal_user_team_placeholder() {
        assert_eq!(
            rewrite_simulated_team_name(
                "traded Player A to Gio Simulation for Player B",
                "Dexter's Dawgs"
            ),
            "traded Player A to Dexter's Dawgs for Player B"
        );
        assert_eq!(
            rewrite_simulated_team_name("added Player C", "Dexter's Dawgs"),
            "added Player C"
        );
    }

    #[test]
    fn rink_brand_headers_keep_literal_report_purpose() {
        assert_eq!(
            CREASE_STARTER_EVIDENCE_HEADER,
            "THE CREASE — STARTER EVIDENCE"
        );
        assert_eq!(CREASE_GOALIE_PLAN_HEADER, "THE CREASE — WHO GETS THE NET?");
        assert_eq!(
            PENALTY_BOX_AVAILABILITY_HEADER,
            "THE PENALTY BOX — AVAILABILITY REPORT"
        );
        assert_eq!(INSIDER_MORNING_SKATE_HEADER, "THE INSIDER — MORNING SKATE");
        assert_eq!(
            SCOREBOARD_FANTASY_STANDINGS_HEADER,
            "THE SCOREBOARD — FANTASY STANDINGS"
        );
        assert_eq!(
            BENCH_SCHEDULE_EDGE_HEADER,
            "THE BENCH — THE GAUNTLET — FANTASY SCHEDULE EDGE"
        );
        assert_eq!(
            FACEOFF_MATCHUP_HEADER,
            "THE FACEOFF CIRCLE — TALE OF THE TAPE — MATCHUP PLAN"
        );
        assert_eq!(
            FACEOFF_CATEGORY_MATCHUP_HEADER,
            "THE FACEOFF CIRCLE — TALE OF THE TAPE — CATEGORY MATCHUP PLAN"
        );
        assert_eq!(
            BENCH_WAIVER_WIRE_HEADER,
            "THE BENCH — WAIVER WIRE — WEEKLY PICKUPS"
        );
        assert_eq!(
            BENCH_CALL_UP_BOARD_HEADER,
            "THE BENCH — CALL-UP BOARD — SLEEPERS"
        );
        assert_eq!(
            BENCH_WAR_ROOM_DRAFT_HEADER,
            "THE BENCH — WAR ROOM — DRAFT BOARD"
        );
        assert_eq!(
            BOARDS_TRADE_READINESS_HEADER,
            "THE BOARDS — TRADE READINESS"
        );
        assert_eq!(
            BOARDS_TRADE_ANALYSIS_HEADER,
            "THE BOARDS — TRADE DESK — ANALYSIS"
        );
        assert_eq!(BOARDS_TRADE_HISTORY_HEADER, "THE BOARDS — TRADE HISTORY");
        assert_eq!(BOARDS_TRADE_OFFERS_HEADER, "THE BOARDS — TRADE OFFERS");
        assert_eq!(
            BOARDS_TRADE_FINDER_HEADER,
            "THE BOARDS — HOT STOVE — TRADE FINDER"
        );
    }
}
