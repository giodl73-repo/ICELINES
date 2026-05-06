//! Phase Foster.2 — `icelines favorites` CLI command.
//!
//! Reads the user's Favorites group (entity_refs) from the SQLite
//! db, resolves the day's slate via the schedule fetch path, and
//! renders a per-night summary. Heavy data orchestration (boxscore
//! reads, EventStream insertion) is deferred to Foster.3; this
//! command ships the viewer + the empty-state UX so users can
//! validate the surface end-to-end.

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use icelines_core::entity::EntityRef;
use icelines_core::favorites::{AggregateView, FavoritesView};
use icelines_core::timeframe::Timeframe;

use crate::commands::tonight::parse_iso_date;
use crate::db::GroupDb;

pub async fn run(
    date: Option<String>,
    range: Option<String>,
    group: String,
    json: bool,
) -> Result<()> {
    let anchor = match date.as_deref() {
        Some(d) => {
            let iso = parse_iso_date(d)?;
            NaiveDate::parse_from_str(&iso, "%Y-%m-%d")
                .context("parse anchor date")?
        }
        None => Utc::now().date_naive(),
    };
    let range = parse_range(range.as_deref())?;

    let db = GroupDb::open().context("open group db")?;
    let members = match db.list_members_with_kind(&group) {
        Ok(m) => m,
        Err(_) => Vec::new(),
    };

    if members.is_empty() {
        if json {
            print_empty_json(&group, anchor, range);
        } else {
            print_empty_text(&group);
        }
        return Ok(());
    }

    // Map db members to entity_refs (the kind discriminator picks
    // the prefix). Players whose key is a normalized name surface
    // as `player:<name>` — Foster.3 swaps these for `player:<pid>`
    // once the resolver is wired into the add path.
    let entity_refs: Vec<EntityRef> = members
        .iter()
        .filter_map(|(key, kind)| {
            let s = format!("{}:{}", kind.as_str(), key);
            // Strict EntityRef parse will reject player:<name>
            // (alphanumeric-only) — that's expected. Non-strict
            // entries skip resolution but stay visible in the
            // empty-state count below.
            s.parse().ok()
        })
        .collect();

    let view = FavoritesView {
        date: anchor,
        range,
        players: Vec::new(),
        teams: Vec::new(),
        events: Vec::new(),
        aggregate: empty_aggregate(anchor, range),
    };

    if json {
        print_json(&view, &group, members.len(), entity_refs.len())?;
    } else {
        print_text(&view, &group, &members);
    }
    Ok(())
}

fn parse_range(s: Option<&str>) -> Result<Timeframe> {
    Ok(match s {
        None | Some("day") => Timeframe::Day,
        Some("week") => Timeframe::Week,
        Some("month") => Timeframe::Month,
        Some("season") => Timeframe::Season,
        Some(other) => anyhow::bail!(
            "unknown --range '{other}' — expected one of: day, week, month, season"
        ),
    })
}

fn empty_aggregate(date: NaiveDate, range: Timeframe) -> AggregateView {
    let (start, end) = range.range(date);
    AggregateView {
        range_start: start,
        range_end: end,
        player_rollups: Vec::new(),
        team_rollups: Vec::new(),
    }
}

fn print_empty_text(group: &str) {
    println!("FAVORITES — group '{group}' is empty");
    println!();
    println!("  No favorites yet. Add players or teams via:");
    println!();
    println!("    icelines group add {group} \"Connor McDavid\"");
    println!("    icelines group add {group} EDM");
    println!();
    println!("  Then re-run `icelines favorites` to see tonight's lines.");
}

fn print_empty_json(group: &str, date: NaiveDate, range: Timeframe) {
    let env = serde_json::json!({
        "schema_version": 1,
        "route": "favorites",
        "data": {
            "players": [],
            "teams": [],
            "events": [],
        },
        "meta": {
            "date": date.format("%Y-%m-%d").to_string(),
            "range": format!("{range:?}").to_lowercase(),
            "group_name": group,
            "counts": { "players": 0, "teams": 0, "events": 0 },
        },
    });
    println!("{}", serde_json::to_string_pretty(&env).unwrap());
}

fn print_text(view: &FavoritesView, _group: &str, members: &[(String, crate::db::MemberKind)]) {
    let player_count = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Player))
        .count();
    let team_count = members
        .iter()
        .filter(|(_, k)| matches!(k, crate::db::MemberKind::Team))
        .count();
    println!(
        "FAVORITES — {} ({} player(s), {} team(s))",
        view.date.format("%Y-%m-%d (%a)"),
        player_count,
        team_count,
    );
    println!("{}", "─".repeat(64));
    if !members.is_empty() {
        println!("Watching:");
        for (key, kind) in members {
            let label = match kind {
                crate::db::MemberKind::Player => format!("  · player {key}"),
                crate::db::MemberKind::Team => format!("  · team {key}"),
            };
            println!("{label}");
        }
        println!();
    }
    if view.players.is_empty() && view.teams.is_empty() {
        println!("Per-night stat lines + team scores ship in Foster.3 (boxscore fetcher).");
        println!("Use `icelines tonight --date {}` for the live slate today.", view.date);
    } else {
        // Future Foster.3 path: render rows.
        for row in &view.players {
            println!("  {row:?}");
        }
        for row in &view.teams {
            println!("  {row:?}");
        }
    }
}

fn print_json(
    view: &FavoritesView,
    group: &str,
    members_count: usize,
    resolved: usize,
) -> Result<()> {
    let env = serde_json::json!({
        "schema_version": 1,
        "route": "favorites",
        "data": {
            "players": view.players,
            "teams": view.teams,
            "events": view.events,
        },
        "meta": {
            "date": view.date.format("%Y-%m-%d").to_string(),
            "range": format!("{:?}", view.range).to_lowercase(),
            "group_name": group,
            "counts": {
                "players": view.players.len(),
                "teams": view.teams.len(),
                "events": view.events.len(),
            },
            "members_total": members_count,
            "members_resolved": resolved,
        },
    });
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_foster2_parse_range_defaults_to_day() {
        assert_eq!(parse_range(None).unwrap(), Timeframe::Day);
        assert_eq!(parse_range(Some("day")).unwrap(), Timeframe::Day);
    }

    #[test]
    fn l0_foster2_parse_range_known_values() {
        assert_eq!(parse_range(Some("week")).unwrap(), Timeframe::Week);
        assert_eq!(parse_range(Some("month")).unwrap(), Timeframe::Month);
        assert_eq!(parse_range(Some("season")).unwrap(), Timeframe::Season);
    }

    #[test]
    fn l0_foster2_parse_range_rejects_garbage() {
        let err = parse_range(Some("forever")).unwrap_err();
        assert!(err.to_string().contains("unknown --range"));
    }

    #[test]
    fn l0_foster2_empty_aggregate_uses_timeframe_range() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let agg = empty_aggregate(date, Timeframe::Week);
        // Week of Thu Jan 15 → Mon Jan 12 ..= Sun Jan 18
        assert_eq!(agg.range_start, NaiveDate::from_ymd_opt(2026, 1, 12).unwrap());
        assert_eq!(agg.range_end, NaiveDate::from_ymd_opt(2026, 1, 18).unwrap());
        assert!(agg.player_rollups.is_empty());
        assert!(agg.team_rollups.is_empty());
    }
}
