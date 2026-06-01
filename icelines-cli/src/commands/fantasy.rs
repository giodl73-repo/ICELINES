//! Fantasy league commands — leagues, teams, scoring, trades, HTTP server.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json,
};
use chrono::NaiveDate;
use icelines_core::timeframe::Timeframe;
use icelines_core::view_model::{
    FantasyDailyDeltaView, FantasyDailyPlayerStatus, FantasyDailyTeamRow, FantasyImportRowStatus,
    FantasyImportView, FantasyMatchupOutcome, FantasyMatchupSideRow, FantasyMatchupWeekView,
};
use icelines_core::{
    build_fantasy_simulation_view,
    model::{Position, Season},
    name::normalize_name,
    resolve_fantasy_scenario_roster_details,
    scheme::Scheme,
    score_fantasy_roster,
    season_stats::SeasonType,
    stats_repository::{PlayerView, StatsRepository},
    FantasyLeagueInput, FantasyLeagueTeamInput, FantasyLeagueView, FantasyRosterGapInput,
    FantasyRosterGapView, FantasySimulationBuildInput, FantasySimulationConfidence,
    FantasySimulationHorizon, FantasySimulationRosterTeamInput,
    FantasySimulationScenarioRosterInput, FantasySimulationView, RosterShape, RosterShapeStatus,
    RosterShapeValidationView, ViewContext, ViewWindow, CURRENT_SEASON,
};
use icelines_fetch::datastore::DataStore;
use icelines_fetch::fantasy_daily::build_fantasy_daily_delta_view;
use icelines_fetch::fantasy_import::{import_yahoo_roster_csv, FantasyRosterImportOptions};
use icelines_fetch::fantasy_matchup::build_fantasy_matchup_week_view;
use icelines_fetch::nhl_api::NhlApiClient;
use icelines_fetch::schedule_remaining::remaining_games_by_team_from_cache;
use icelines_fetch::stats_loader::LoadOutcome;
use serde_json::{json, Value};

use crate::fantasy_db::{resolve_roster_shape, FantasyDb, LeagueRow, TeamRow};

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
pub async fn run_team_show(name: String, league_override: Option<String>) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let team = require_team(&db, &league.id, &name)?;
    let scheme = resolve_scheme(&league.scheme)?;
    let (outcome, season) = load_pools()?;
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
        let v = all_skaters
            .iter()
            .find(|v| v.identity.name_normalized.contains(norm_q.as_str()));

        let (team_abbr, pos, gp_str, pts_str) = match v {
            Some(view) => (
                view.team_display().to_owned(),
                view.position().abbreviation().to_owned(),
                view.gp().to_string(),
                view.stats.totals.points.to_string(),
            ),
            None => (
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
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let team = require_team(&db, &league.id, &team_name)?;
    let (outcome, season) = load_pools()?;
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

    println!("\nSTANDINGS — {} ({})", league.name, scheme_name);
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

    let view = import_yahoo_roster_csv(&db, &file, options)
        .with_context(|| format!("import Yahoo roster CSV from {}", file.display()))?;

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

/// `icelines fantasy trade <player1> --to-team <team2> --for-player <player2> [--execute] [--league <league>]`
pub async fn run_trade(
    player1_q: String,
    to_team_name: String,
    player2_q: String,
    execute: bool,
    league_override: Option<String>,
) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    let league = require_league(&db, &league_override)?;
    let scheme = resolve_scheme(&league.scheme)?;
    let (outcome, season) = load_pools()?;
    let (all_skaters, all_goalies) = pools_views(&outcome.repo, season);

    // Resolve players from queries.
    let p1 = fuzzy_find_skater(&all_skaters, &player1_q)?;
    let p2 = fuzzy_find_skater(&all_skaters, &player2_q)?;
    let p1_norm = p1.identity.name_normalized.clone();
    let p2_norm = p2.identity.name_normalized.clone();
    let p1_name = p1.identity.full_name.clone();
    let p2_name = p2.identity.full_name.clone();

    // Find which team has player1.
    let team1_name = db
        .is_on_any_team(&league.id, &p1_norm)?
        .with_context(|| format!("'{}' is not on any team in this league", p1_name))?;
    let team1 = require_team(&db, &league.id, &team1_name)?;

    // team2 is named explicitly.
    let team2 = require_team(&db, &league.id, &to_team_name)?;

    // Verify player2 is on team2.
    let team2_has_p2 = db
        .is_on_any_team(&league.id, &p2_norm)?
        .map(|t| t == to_team_name)
        .unwrap_or(false);
    if !team2_has_p2 {
        bail!("'{}' is not on team '{to_team_name}'", p2_name);
    }

    // Compute BEFORE scores.
    let roster1_before = db.list_roster(&team1.id)?;
    let roster2_before = db.list_roster(&team2.id)?;
    let score1_before: f32 = score_team(&roster1_before, &all_skaters, &all_goalies, &scheme)
        .iter()
        .map(|(_, s)| s)
        .sum();
    let score2_before: f32 = score_team(&roster2_before, &all_skaters, &all_goalies, &scheme)
        .iter()
        .map(|(_, s)| s)
        .sum();

    // Simulate AFTER scores (swap player norms in rosters).
    let roster1_after: Vec<String> = roster1_before
        .iter()
        .map(|n| {
            if n == &p1_norm {
                p2_norm.clone()
            } else {
                n.clone()
            }
        })
        .collect();
    let roster2_after: Vec<String> = roster2_before
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

    println!("\nTRADE ANALYSIS — {}", league.name);
    println!("{}", "─".repeat(60));
    println!("  {p1_name} ({team1_name})  <->  {p2_name} ({to_team_name})");
    println!("{}", "─".repeat(60));
    println!("  {:<22}  BEFORE    AFTER     DELTA", "Team");
    println!("{}", "─".repeat(60));
    println!(
        "  {:<22}  {:<9.1} {:<9.1} {:+.1}",
        team1_name,
        score1_before,
        score1_after,
        score1_after - score1_before
    );
    println!(
        "  {:<22}  {:<9.1} {:<9.1} {:+.1}",
        to_team_name,
        score2_before,
        score2_after,
        score2_after - score2_before
    );

    if execute {
        // Actually execute the trade.
        db.drop_player(&team1.id, &p1_norm)?;
        db.add_player(&team1.id, &p2_norm)?;
        db.drop_player(&team2.id, &p2_norm)?;
        db.add_player(&team2.id, &p1_norm)?;
        println!("\nTrade executed.");
    } else {
        println!("\n(use --execute to commit this trade)");
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
}
