//! Phase Foster.2 + Foster +21 — `icelines favorites` CLI command.
//!
//! Reads the user's Favorites group (entity_refs) from the SQLite
//! db, fetches the day's slate, parses the per-game boxscore JSON
//! that Foster +3 persisted, and renders a per-night summary with
//! G/A/P/SOG/+/-/TOI on each row. Goalies render SV/SA/GAA. DNP
//! rows surface the reason (TeamBye / Scratched / DataPending).

use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use icelines_core::favorites::{
    AggregateView, FavoritesView, GameResult, GameState, GoalieNightLine, HomeAway,
    PlayerNightRow, SkaterNightLine, TeamNightRow,
};
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

    // Phase Foster +21 — fetch the slate + walk persisted boxscore
    // bodies via the favorites_view builder so each row gets real
    // G/A/P/+/- numbers. Slate fetch is best-effort: when offline,
    // every favorited entity surfaces as DataPending.
    let client = icelines_fetch::nhl_api::NhlApiClient::production();
    let anchor_str = anchor.format("%Y-%m-%d").to_string();
    let slate = client
        .fetch_schedule_for_date(&anchor_str)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|g| g.date == anchor_str)
        .collect::<Vec<_>>();

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let data_root = home.join(".icelines").join("data");
    let view = crate::favorites_view::compute_favorites_view(
        &db, &group, anchor, range, &slate, &data_root,
    )
    .unwrap_or_else(|_| FavoritesView {
        date: anchor,
        range,
        players: Vec::new(),
        teams: Vec::new(),
        events: Vec::new(),
        aggregate: empty_aggregate(anchor, range),
    });

    if json {
        print_json(&view, &group, members.len(), view.players.len() + view.teams.len())?;
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
    println!("{}", "─".repeat(72));

    if !view.players.is_empty() {
        println!("Players");
        for row in &view.players {
            println!("  {}", format_player_row(row));
        }
    }
    if !view.teams.is_empty() {
        if !view.players.is_empty() {
            println!();
        }
        println!("Teams");
        for row in &view.teams {
            println!("  {}", format_team_row(row));
        }
    }

    if view.players.is_empty() && view.teams.is_empty() {
        println!();
        println!("Tip: run `icelines fetch boxscore` to populate per-night stat lines.");
    }
}

fn format_player_row(row: &PlayerNightRow) -> String {
    match row {
        PlayerNightRow::Skater(s) => format_skater_line(s),
        PlayerNightRow::Goalie(g) => format_goalie_line(g),
        PlayerNightRow::DidNotPlay { player, reason } => {
            format!("{player}  — DNP ({reason:?})")
        }
    }
}

fn format_skater_line(s: &SkaterNightLine) -> String {
    // Compact one-liner: PID + matchup + line + game state
    let matchup = match s.home_or_away {
        HomeAway::Home => format!("{} vs {}", s.team.0, s.opponent.0),
        HomeAway::Away => format!("{} @ {}", s.team.0, s.opponent.0),
    };
    let result = match s.result {
        GameResult::Win => "W",
        GameResult::Loss => "L",
        GameResult::OtLoss => "OTL",
        GameResult::InProgress => "—",
    };
    let toi = s
        .toi_seconds
        .map(|sec| format!("{}:{:02}", sec / 60, sec % 60))
        .unwrap_or_else(|| "—".to_string());
    let hits = s.hits.map(|n| n.to_string()).unwrap_or_else(|| "—".into());
    let blocks = s.blocks.map(|n| n.to_string()).unwrap_or_else(|| "—".into());
    let sog = s.shots.map(|n| n.to_string()).unwrap_or_else(|| "—".into());
    format!(
        "{}  {} {}-{} {} · {}G {}A {}P · {:+} · TOI {}  · {} SOG · {} hits · {} blk{}",
        s.player,
        matchup,
        s.team_score,
        s.opponent_score,
        result,
        s.goals,
        s.assists,
        s.points,
        s.plus_minus,
        toi,
        sog,
        hits,
        blocks,
        if matches!(s.game_state, GameState::Live) {
            " · LIVE"
        } else {
            ""
        }
    )
}

fn format_goalie_line(g: &GoalieNightLine) -> String {
    let matchup = match g.home_or_away {
        HomeAway::Home => format!("{} vs {}", g.team.0, g.opponent.0),
        HomeAway::Away => format!("{} @ {}", g.team.0, g.opponent.0),
    };
    let dec = match g.decision {
        Some(icelines_core::favorites::Decision::Win) => "W",
        Some(icelines_core::favorites::Decision::Loss) => "L",
        Some(icelines_core::favorites::Decision::OtLoss) => "OTL",
        None => "—",
    };
    format!(
        "{}  {} {}-{} {} · {}/{} SV · SV%.{:.0} · GAA {:.2}{}",
        g.player,
        matchup,
        g.team_score,
        g.opponent_score,
        dec,
        g.saves,
        g.shots_against,
        g.save_pct * 1000.0, // 0.971 → "971" → renders as ".971"
        g.gaa,
        if g.shutout { " · SHUTOUT" } else { "" },
    )
}

fn format_team_row(t: &TeamNightRow) -> String {
    if t.on_bye {
        return format!("{}  — bye", t.team_abbr.0);
    }
    let opp = t
        .opponent
        .as_ref()
        .map(|o| o.0.as_str())
        .unwrap_or("—");
    let result = match t.result {
        Some(GameResult::Win) => "W",
        Some(GameResult::Loss) => "L",
        Some(GameResult::OtLoss) => "OTL",
        Some(GameResult::InProgress) => "LIVE",
        None => "—",
    };
    format!(
        "{}  {} {} vs {}",
        t.team_abbr.0,
        if t.score.is_empty() { "—" } else { t.score.as_str() },
        result,
        opp,
    )
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
