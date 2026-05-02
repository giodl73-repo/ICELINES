//! Fantasy league commands — leagues, teams, scoring, trades, HTTP server.

use std::sync::Arc;

use anyhow::{bail, Context};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json,
};
use icelines_core::{
    model::Season,
    name::normalize_name,
    scheme::{self, compute_fantasy_score, compute_goalie_fantasy_score, GoalieScoreStats, Scheme},
    season_stats::SeasonType,
    stats_repository::{PlayerView, StatsRepository},
};
use icelines_fetch::stats_loader::LoadOutcome;
use serde_json::{json, Value};

use crate::fantasy_db::{FantasyDb, LeagueRow, TeamRow};

// ── PlayerView → SkaterStats bridge ───────────────────────────────────────────
//
// Hart.5c.4: cold-start mapping (per spec D5). Realtime fields
// (hits / blocked_shots / takeaways / giveaways) are Option<u32> in the
// new model — `unwrap_or(0)` preserves legacy behavior where the cold-
// start case scored those categories as zero. The cold-start parity
// test pins this so a future Option-aware fantasy rewrite knows what
// it's changing.

fn to_scheme_stats_view(v: &PlayerView<'_>) -> scheme::SkaterStats {
    let totals = &v.stats.totals;
    scheme::SkaterStats {
        goals:         totals.goals,
        assists:       totals.assists,
        pp_goals:      totals.pp_goals,
        pp_assists:    totals.pp_points.saturating_sub(totals.pp_goals),
        sh_goals:      totals.sh_goals,
        sh_assists:    totals.sh_points.saturating_sub(totals.sh_goals),
        gwg:           totals.gwg,
        ot_goals:      totals.ot_goals,
        hits:          v.hits().unwrap_or(0),
        blocks:        v.blocked_shots().unwrap_or(0),
        shots_on_goal: v.shots(),
        plus_minus:    v.plus_minus(),
        takeaways:     v.takeaways().unwrap_or(0),
        giveaways:     v.giveaways().unwrap_or(0),
        faceoff_wins:  0,
    }
}

/// Bridge a goalie `PlayerView` → fantasy `GoalieScoreStats`.
/// Returns None when `stats.goalie` is unset (call-ups not yet on ice).
fn to_goalie_scheme_stats_view(v: &PlayerView<'_>) -> Option<GoalieScoreStats> {
    let g = v.stats.goalie.as_ref()?;
    Some(GoalieScoreStats {
        games_played:  g.games_started,
        wins:          g.wins,
        losses:        g.losses,
        saves:         g.saves,
        goals_against: g.goals_against,
        shutouts:      g.shutouts,
        save_pct:      g.save_pct.unwrap_or(0.0),
    })
}

/// Find a view by partial normalized name in the given slice.
fn fuzzy_find_view_in<'a, 'r>(
    views: &'a [PlayerView<'r>],
    query: &str,
) -> Option<&'a PlayerView<'r>> {
    let norm = normalize_name(query);
    views.iter().find(|v| v.identity.name_normalized.contains(norm.as_str()))
}

/// Skater-pool fuzzy find with anyhow context for the team-add / trade
/// flows. Mirrors the previous `fuzzy_find_player` shape.
fn fuzzy_find_skater<'a, 'r>(
    skaters: &'a [PlayerView<'r>],
    query: &str,
) -> anyhow::Result<&'a PlayerView<'r>> {
    fuzzy_find_view_in(skaters, query).with_context(|| {
        format!("no skater found matching '{query}' — try `icelines query goalies` if it's a goalie")
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
    match name {
        "yahoo-standard" => Ok(Scheme::yahoo_standard()),
        "espn-standard" => Ok(Scheme::espn_standard()),
        "simple-pts" => Ok(Scheme::simple_pts()),
        other => bail!(
            "unknown scheme '{other}'. Try: yahoo-standard, espn-standard, simple-pts"
        ),
    }
}

/// Score every name in a roster, returning `(full_name, score)` sorted
/// desc. Skater pool searched first; goalie pool as fallback.
fn score_team(
    roster_norms: &[String],
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
    scheme: &Scheme,
) -> Vec<(String, f32)> {
    let mut results: Vec<(String, f32)> = Vec::new();

    for norm in roster_norms {
        // Skater first.
        if let Some(v) = skaters.iter().find(|v| v.identity.name_normalized.contains(norm.as_str())) {
            let gp = v.gp();
            let score = compute_fantasy_score(&to_scheme_stats_view(v), &scheme.skater, gp)
                .map(|fs| fs.total)
                .unwrap_or(0.0);
            results.push((v.identity.full_name.clone(), score));
            continue;
        }
        // Then goalie.
        if let Some(v) = goalies.iter().find(|v| v.identity.name_normalized.contains(norm.as_str())) {
            let stats = match to_goalie_scheme_stats_view(v) {
                Some(s) => s,
                None    => {
                    // Goalie has no stats yet — counts as 0 score, still listed.
                    results.push((v.identity.full_name.clone(), 0.0));
                    continue;
                }
            };
            let gp = v.stats.goalie.as_ref().map(|g| g.games_started).unwrap_or(0);
            let score = compute_goalie_fantasy_score(&stats, &scheme.goalie, gp)
                .map(|fs| fs.total)
                .unwrap_or(0.0);
            results.push((v.identity.full_name.clone(), score));
            continue;
        }
        eprintln!("  [warn] roster entry '{norm}' not found in current player or goalie pool");
    }

    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    results
}

/// Require an active league, or use the given override.
fn require_league(
    db: &FantasyDb,
    league_override: &Option<String>,
) -> anyhow::Result<LeagueRow> {
    if let Some(name) = league_override {
        let leagues = db.list_leagues()?;
        leagues
            .into_iter()
            .find(|l| &l.name == name)
            .with_context(|| format!("league '{name}' not found — run `icelines fantasy league-list`"))
    } else {
        db.get_active_league()?
            .ok_or_else(|| anyhow::anyhow!(
                "no active league. Use `icelines fantasy league-use <name>` or --league."
            ))
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

    if leagues.is_empty() {
        println!("No leagues yet. Create one with `icelines fantasy league-create <name>`.");
        return Ok(());
    }

    println!("{:<28} {:<18} {:<7} Active", "Name", "Scheme", "Teams");
    println!("{}", "─".repeat(60));
    for l in &leagues {
        let active_marker = if l.is_active { "<—" } else { "" };
        println!(
            "{:<28} {:<18} {:<7} {}",
            l.name, l.scheme, l.team_count, active_marker
        );
    }
    Ok(())
}

/// `icelines fantasy league-use <name>` / `icelines fantasy league-switch <name>`
pub async fn run_league_use(name: String) -> anyhow::Result<()> {
    let db = FantasyDb::open()?;
    db.set_active_league(&name)
        .with_context(|| format!("league '{name}' not found — run `icelines fantasy league-list`"))?;
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

    if teams.is_empty() {
        println!(
            "No teams in '{}'. Add one with `icelines fantasy team-create <name>`.",
            league.name
        );
        return Ok(());
    }

    println!("League: {} ({})", league.name, league.scheme);
    println!(
        "{:<28} {:<20} {:<8}",
        "Team", "Owner", "Players"
    );
    println!("{}", "─".repeat(58));
    for t in &teams {
        println!("{:<28} {:<20} {:<8}", t.name, t.owner, t.player_count);
    }
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
        "\nRoster: {} | League: {} | Scheme: {}",
        team.name, league.name, league.scheme
    );
    println!("{}", "─".repeat(72));
    println!("  {:<4} {:<24} {:<5} {:<4} {:<5} {:<7} Fantasy", "#", "Player", "Team", "Pos", "GP", "Pts");
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
            None => ("—".to_owned(), "—".to_owned(), "—".to_owned(), "—".to_owned()),
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
        Ok(v) => (v.identity.full_name.clone(), v.identity.name_normalized.clone(), "Skater"),
        Err(skater_err) => match fuzzy_find_view_in(&all_goalies, &player_query) {
            Some(v) => (v.identity.full_name.clone(), v.identity.name_normalized.clone(), "Goalie"),
            None    => return Err(skater_err),
        }
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
        Ok(v) => (v.identity.full_name.clone(), v.identity.name_normalized.clone()),
        Err(skater_err) => match fuzzy_find_view_in(&all_goalies, &player_query) {
            Some(v) => (v.identity.full_name.clone(), v.identity.name_normalized.clone()),
            None    => return Err(skater_err),
        }
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
        "\nSTANDINGS — {} ({})",
        league.name, scheme_name
    );
    println!("{}", "─".repeat(60));
    println!("{:<5} {:<22} {:<16} {:<10} Per/G", "Rank", "Team", "Owner", "Score");
    println!("{}", "─".repeat(60));
    for (rank, (team_name, owner, total, per_g)) in standings.iter().enumerate() {
        // Avoid -0.0 display for empty teams (IEEE 754 negative zero)
        let total_display = if *total == 0.0 { 0.0f32 } else { *total };
        let perg_display  = if *per_g  == 0.0 { 0.0f32 } else { *per_g  };
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
        .map(|n| if n == &p1_norm { p2_norm.clone() } else { n.clone() })
        .collect();
    let roster2_after: Vec<String> = roster2_before
        .iter()
        .map(|n| if n == &p2_norm { p1_norm.clone() } else { n.clone() })
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
    println!(
        "  {:<22}  BEFORE    AFTER     DELTA",
        "Team"
    );
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
    let (outcome, season) = load_pools()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
    let (outcome, season) = load_pools()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
                "player_count": t.player_count,
            })
        })
        .collect();

    Ok(Json(json!(list)))
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
    let (outcome, season) = load_pools()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
            json!({
                "name": name,
                "pos": v.map(|view| view.position().abbreviation()).unwrap_or("—"),
                "gp": v.map(|view| view.gp()).unwrap_or(0),
                "score": score,
            })
        })
        .collect();

    Ok(Json(json!({
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
    let (outcome, season) = load_pools()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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

    Ok(Json(json!({ "status": "added", "player": full_name, "team": team_name })))
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
    let (outcome, season) = load_pools()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
    let (outcome, season) = load_pools()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
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
        .map(|n| if n == &p1_norm { p2_norm.clone() } else { n.clone() })
        .collect();
    let roster2_after: Vec<String> = roster2
        .iter()
        .map(|n| if n == &p2_norm { p1_norm.clone() } else { n.clone() })
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
        println!("Fantasy server running at http://0.0.0.0:{port}");
        println!("League: {league_display} | Press Ctrl-C to stop");
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
    use icelines_core::{
        fixtures,
        identity::PlayerId,
    };

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
            .view(
                PlayerId(8478402),
                Season(20242025),
                SeasonType::Regular,
            )
            .unwrap();

        let s = to_scheme_stats_view(&v);
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
            .view(
                PlayerId(8478402),
                Season(20242025),
                SeasonType::Regular,
            )
            .unwrap();

        let s = to_scheme_stats_view(&v);
        assert_eq!(s.hits, 48);
        assert_eq!(s.blocks, 22);
        assert_eq!(s.takeaways, 65);
        assert_eq!(s.giveaways, 41);
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
        let goalie_stats = fixtures::solo_goalie(
            8478406,
            20242025,
            TeamAbbr("OTT".into()),
        )
        .build();
        repo.upsert_identity(goalie_id).unwrap();
        repo.upsert_stats(goalie_stats).unwrap();

        let skaters: Vec<PlayerView<'_>> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let goalies: Vec<PlayerView<'_>> = repo
            .goalies(Season(20242025), SeasonType::Regular)
            .collect();
        assert_eq!(skaters.len(), 1, "must have exactly one skater in fixture repo");
        assert_eq!(goalies.len(), 1, "must have exactly one goalie in fixture repo");

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
        assert!(names.contains(&"McDavid"), "missing skater in results: {names:?}");
        assert!(names.contains(&"Test Goalie"), "missing goalie in results: {names:?}");
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
        let s = to_scheme_stats_view(&v);
        assert_eq!(s.goals, 30);
        assert_eq!(s.assists, 50);
        assert_eq!(s.hits, 48);
        assert_eq!(s.blocks, 22);
        assert_eq!(s.takeaways, 65);
        assert_eq!(s.giveaways, 41);
    }
}
