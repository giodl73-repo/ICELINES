//! Phase Foster +26 — windowed leaderboard from per-game boxscore data.
//!
//! `query leaders --week` / `--month` aggregates skater stats across
//! every persisted boxscore inside the timeframe window, grouped by
//! PlayerId. Reads from the manifest's `Boxscore` shard so only the
//! games already on disk count — caller can run
//! `icelines fetch boxscore --date <D>` for each day in the window
//! to populate, or `icelines fetch sync` once boxscores get TTLs.
//!
//! Output is a leader table sorted by `--sort` (g / a / p / sog),
//! defaulting to points. Same JSON envelope shape as `query leaders`.

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::Utc;
use icelines_core::timeframe::Timeframe;
use icelines_fetch::manifest::{DataKey, DataKind};

#[derive(Debug, Default, Clone)]
struct PlayerWindowTotals {
    player_id: u32,
    games: u32,
    goals: u32,
    assists: u32,
    points: u32,
    plus_minus: i32,
    sog: u32,
    hits: u32,
    blocks: u32,
    pim: u32,
    toi_seconds: u32,
}

pub async fn run_windowed_leaders(
    timeframe: Timeframe,
    top: usize,
    sort: String,
    json: bool,
) -> Result<()> {
    let today = Utc::now().date_naive();
    let (start, end) = timeframe.range(today);

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let data_root = home.join(".icelines").join("data");
    let store = icelines_fetch::datastore::DataStore::open(&data_root)
        .context("open DataStore")?;

    // Walk every persisted boxscore. The manifest entry's path
    // includes the date (e.g. data/boxscores/2026-05-06/<id>.json);
    // we filter to dates inside the window via the path component.
    let mut totals: HashMap<u32, PlayerWindowTotals> = HashMap::new();
    let mut games_seen = 0usize;
    for entry in store.manifest().list(DataKind::Boxscore) {
        let DataKey::Game(game_id) = entry.key else { continue };
        let date_str = match entry
            .path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
        {
            Some(s) => s,
            None => continue,
        };
        let date = match chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        if date < start || date > end {
            continue;
        }
        let raw = match store.load_boxscore_raw(DataKey::Game(game_id)) {
            Some(r) => r,
            None => continue,
        };
        let parsed = icelines_fetch::nhl_api::parse_boxscore(&raw, game_id.0);
        for skater in parsed.away_skaters.iter().chain(parsed.home_skaters.iter()) {
            let t = totals.entry(skater.player_id).or_insert_with(|| {
                PlayerWindowTotals {
                    player_id: skater.player_id,
                    ..Default::default()
                }
            });
            t.games += 1;
            t.goals += skater.goals;
            t.assists += skater.assists;
            t.points += skater.goals + skater.assists;
            t.plus_minus += skater.plus_minus;
            t.sog += skater.sog;
            t.hits += skater.hits;
            t.blocks += skater.blocked_shots;
            t.pim += skater.pim;
            t.toi_seconds += skater.toi_seconds;
        }
        games_seen += 1;
    }

    let mut rows: Vec<PlayerWindowTotals> = totals.into_values().collect();
    sort_rows(&mut rows, &sort);
    rows.truncate(top);

    if json {
        emit_json(&rows, &sort, timeframe, start, end, games_seen)?;
    } else {
        emit_text(&rows, &sort, timeframe, start, end, games_seen);
    }
    Ok(())
}

fn sort_rows(rows: &mut [PlayerWindowTotals], sort: &str) {
    match sort {
        "g" | "goals" => rows.sort_by(|a, b| b.goals.cmp(&a.goals)),
        "a" | "assists" => rows.sort_by(|a, b| b.assists.cmp(&a.assists)),
        "sog" | "shots" => rows.sort_by(|a, b| b.sog.cmp(&a.sog)),
        "hits" => rows.sort_by(|a, b| b.hits.cmp(&a.hits)),
        "blocks" => rows.sort_by(|a, b| b.blocks.cmp(&a.blocks)),
        "plus-minus" | "+/-" => rows.sort_by(|a, b| b.plus_minus.cmp(&a.plus_minus)),
        // Default: points (also matches "p" / "pts" / explicit "points").
        _ => rows.sort_by(|a, b| b.points.cmp(&a.points)),
    }
}

fn emit_text(
    rows: &[PlayerWindowTotals],
    sort: &str,
    timeframe: Timeframe,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    games_seen: usize,
) {
    println!(
        "WINDOWED LEADERS — {:?} ({} → {}) · {games_seen} game(s) on disk · sort: {sort}",
        timeframe, start, end,
    );
    println!("{}", "─".repeat(86));
    println!(
        "{:>4} {:>10} {:>3} {:>3} {:>3} {:>4} {:>4} {:>5} {:>5} {:>5} {:>5}",
        "Rank", "PID", "GP", "G", "A", "P", "+/-", "SOG", "Hits", "Blk", "PIM"
    );
    println!("{}", "─".repeat(86));
    if rows.is_empty() {
        println!("(no boxscores on disk inside window — try `icelines fetch boxscore`)");
        return;
    }
    for (i, r) in rows.iter().enumerate() {
        println!(
            "{:>4} {:>10} {:>3} {:>3} {:>3} {:>4} {:>+4} {:>5} {:>5} {:>5} {:>5}",
            i + 1,
            r.player_id,
            r.games,
            r.goals,
            r.assists,
            r.points,
            r.plus_minus,
            r.sog,
            r.hits,
            r.blocks,
            r.pim,
        );
    }
}

fn emit_json(
    rows: &[PlayerWindowTotals],
    sort: &str,
    timeframe: Timeframe,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    games_seen: usize,
) -> Result<()> {
    let env = serde_json::json!({
        "schema_version": 1,
        "route": "leaders.windowed",
        "data": rows.iter().map(|r| serde_json::json!({
            "player_id": r.player_id,
            "games": r.games,
            "goals": r.goals,
            "assists": r.assists,
            "points": r.points,
            "plus_minus": r.plus_minus,
            "sog": r.sog,
            "hits": r.hits,
            "blocks": r.blocks,
            "pim": r.pim,
            "toi_seconds": r.toi_seconds,
        })).collect::<Vec<_>>(),
        "meta": {
            "timeframe": format!("{:?}", timeframe).to_lowercase(),
            "range_start": start.format("%Y-%m-%d").to_string(),
            "range_end": end.format("%Y-%m-%d").to_string(),
            "games_aggregated": games_seen,
            "sort": sort,
            "rows": rows.len(),
        },
    });
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

// ── Phase Conn Smythe C.2 — Cup-run leaderboard ──────────────────────────────

pub async fn run_playoff_leaders(top: usize, sort: String, json: bool) -> Result<()> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let data_root = home.join(".icelines").join("data");
    let store = icelines_fetch::datastore::DataStore::open(&data_root)
        .context("open DataStore")?;

    // Walk every persisted boxscore; filter to gameType=3 (playoffs).
    // No date-window filter — the playoff round naturally bounds the
    // games that exist in the manifest.
    let mut totals: HashMap<u32, PlayerWindowTotals> = HashMap::new();
    let mut games_seen = 0usize;
    for entry in store.manifest().list(DataKind::Boxscore) {
        let DataKey::Game(game_id) = entry.key else { continue };
        let raw = match store.load_boxscore_raw(DataKey::Game(game_id)) {
            Some(r) => r,
            None => continue,
        };
        // Filter on gameType — playoff games carry 3.
        if raw.get("gameType").and_then(|v| v.as_u64()) != Some(3) {
            continue;
        }
        let parsed = icelines_fetch::nhl_api::parse_boxscore(&raw, game_id.0);
        for skater in parsed.away_skaters.iter().chain(parsed.home_skaters.iter()) {
            let t = totals.entry(skater.player_id).or_insert_with(|| {
                PlayerWindowTotals {
                    player_id: skater.player_id,
                    ..Default::default()
                }
            });
            t.games += 1;
            t.goals += skater.goals;
            t.assists += skater.assists;
            t.points += skater.goals + skater.assists;
            t.plus_minus += skater.plus_minus;
            t.sog += skater.sog;
            t.hits += skater.hits;
            t.blocks += skater.blocked_shots;
            t.pim += skater.pim;
            t.toi_seconds += skater.toi_seconds;
        }
        games_seen += 1;
    }

    let mut rows: Vec<PlayerWindowTotals> = totals.into_values().collect();
    sort_rows(&mut rows, &sort);
    rows.truncate(top);

    if json {
        emit_playoff_json(&rows, &sort, games_seen)?;
    } else {
        emit_playoff_text(&rows, &sort, games_seen);
    }
    Ok(())
}

fn emit_playoff_text(rows: &[PlayerWindowTotals], sort: &str, games_seen: usize) {
    println!(
        "PLAYOFF LEADERS — {games_seen} game(s) on disk · sort: {sort}"
    );
    println!("{}", "─".repeat(86));
    println!(
        "{:>4} {:>10} {:>3} {:>3} {:>3} {:>4} {:>4} {:>5} {:>5} {:>5} {:>5}",
        "Rank", "PID", "GP", "G", "A", "P", "+/-", "SOG", "Hits", "Blk", "PIM"
    );
    println!("{}", "─".repeat(86));
    if rows.is_empty() {
        println!("(no playoff boxscores on disk — try `icelines fetch boxscore`)");
        return;
    }
    for (i, r) in rows.iter().enumerate() {
        println!(
            "{:>4} {:>10} {:>3} {:>3} {:>3} {:>4} {:>+4} {:>5} {:>5} {:>5} {:>5}",
            i + 1,
            r.player_id,
            r.games,
            r.goals,
            r.assists,
            r.points,
            r.plus_minus,
            r.sog,
            r.hits,
            r.blocks,
            r.pim,
        );
    }
}

fn emit_playoff_json(rows: &[PlayerWindowTotals], sort: &str, games_seen: usize) -> Result<()> {
    let env = serde_json::json!({
        "schema_version": 1,
        "route": "leaders.playoff",
        "data": rows.iter().map(|r| serde_json::json!({
            "player_id": r.player_id,
            "games": r.games,
            "goals": r.goals,
            "assists": r.assists,
            "points": r.points,
            "plus_minus": r.plus_minus,
            "sog": r.sog,
            "hits": r.hits,
            "blocks": r.blocks,
            "pim": r.pim,
            "toi_seconds": r.toi_seconds,
        })).collect::<Vec<_>>(),
        "meta": {
            "kind": "playoff_run",
            "games_aggregated": games_seen,
            "sort": sort,
            "rows": rows.len(),
        },
    });
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pwt(pid: u32, g: u32, a: u32, sog: u32) -> PlayerWindowTotals {
        PlayerWindowTotals {
            player_id: pid,
            games: 1,
            goals: g,
            assists: a,
            points: g + a,
            sog,
            ..Default::default()
        }
    }

    #[test]
    fn l0_foster_plus26_sort_default_points() {
        let mut rows = vec![pwt(1, 0, 5, 3), pwt(2, 3, 0, 8), pwt(3, 1, 1, 5)];
        sort_rows(&mut rows, "");
        assert_eq!(rows[0].player_id, 1, "5P first");
        assert_eq!(rows[1].player_id, 2, "3P second");
    }

    #[test]
    fn l0_foster_plus26_sort_goals() {
        let mut rows = vec![pwt(1, 0, 5, 3), pwt(2, 3, 0, 8), pwt(3, 5, 1, 5)];
        sort_rows(&mut rows, "g");
        assert_eq!(rows[0].player_id, 3);
        assert_eq!(rows[1].player_id, 2);
    }

    #[test]
    fn l0_foster_plus26_sort_sog() {
        let mut rows = vec![pwt(1, 0, 5, 3), pwt(2, 3, 0, 8), pwt(3, 5, 1, 5)];
        sort_rows(&mut rows, "sog");
        assert_eq!(rows[0].sog, 8);
    }
}
