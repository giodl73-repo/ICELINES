use anyhow::Context;
use icelines_fetch::nhl_api::NhlApiClient;

/// Convert a UTC time string "HH:MM" to "H:MM AM/PM ET" (assumes EDT = UTC-4, April–Oct).
/// Falls back to showing UTC if parsing fails.
fn format_time_et(utc_hhmm: &str) -> String {
    let parts: Vec<&str> = utc_hhmm.splitn(2, ':').collect();
    if parts.len() == 2 {
        if let (Ok(h), Ok(m)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            // EDT = UTC-4 (daylight saving, Oct–Mar use EST = UTC-5; we use -4 year-round as approximation)
            let et_h = (h + 24 - 4) % 24;
            let period = if et_h < 12 { "AM" } else { "PM" };
            let display_h = match et_h % 12 {
                0 => 12,
                n => n,
            };
            return format!("{display_h}:{m:02} {period} ET");
        }
    }
    format!("{utc_hhmm} UTC")
}

pub async fn run(team_filter: Option<String>) -> anyhow::Result<()> {
    let client = NhlApiClient::production();
    let all_games = client
        .fetch_today_schedule()
        .await
        .context("fetching today's schedule")?;

    // Filter to today only (first date in the gameWeek)
    let today = all_games.first().map(|g| g.date.as_str()).unwrap_or("");
    let schedule: Vec<_> = all_games
        .iter()
        .filter(|g| g.date.is_empty() || g.date == today)
        .collect();

    if schedule.is_empty() {
        println!("No games scheduled today.");
        return Ok(());
    }

    let team_up = team_filter.as_deref().map(str::to_uppercase);
    let date_label = if today.is_empty() {
        "today".to_owned()
    } else {
        today.to_owned()
    };

    // Apply team filter up front so the header count matches what's displayed
    let filtered: Vec<_> = schedule
        .iter()
        .filter(|g| match &team_up {
            Some(t) => &g.away_abbrev == t || &g.home_abbrev == t,
            None => true,
        })
        .collect();

    let team_label = team_up
        .as_deref()
        .map(|t| format!(" · {t}"))
        .unwrap_or_default();
    println!(
        "TONIGHT'S GAMES — {}{} ({} game(s))",
        date_label,
        team_label,
        filtered.len()
    );
    println!("{}", "─".repeat(60usize));

    if filtered.is_empty() {
        println!("No games tonight{}.", team_label);
        return Ok(());
    }

    for game in &filtered {
        let utc = game.start_time_utc.get(11..16).unwrap_or("?");
        let et = format_time_et(utc);
        println!(
            "{} {} @ {} {}  {}",
            game.away_abbrev, game.away_name, game.home_abbrev, game.home_name, et
        );
    }
    Ok(())
}

/// LP.1 — `ScheduleRow` is the projection used for table / JSON / CSV
/// output. Mirrors the data the TUI Schedule tab + web /schedule
/// surface already render. Pure data — no formatting decisions baked
/// in beyond the time-zone conversion (UTC → ET).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleRow {
    pub date: String,
    pub away: String,
    pub home: String,
    pub time_et: String,
    pub status: String,
}

/// Filter + project NHL API games into ScheduleRow values. Pure —
/// unit-testable without a live network. The filter step also caps
/// the date range so `--days 7` includes exactly 7 distinct calendar
/// days regardless of how many games span them.
pub(crate) fn project_schedule_rows(
    games: &[icelines_fetch::nhl_api::ScheduledGame],
    team_up: Option<&str>,
    days: u32,
) -> Vec<ScheduleRow> {
    let mut current_date = String::new();
    let mut days_shown = 0u32;
    let mut out = Vec::new();
    for game in games {
        if game.date != current_date {
            if days_shown >= days {
                break;
            }
            current_date = game.date.clone();
            days_shown += 1;
        }
        if let Some(t) = team_up {
            if game.away_abbrev != t && game.home_abbrev != t {
                continue;
            }
        }
        let utc = game.start_time_utc.get(11..16).unwrap_or("?");
        out.push(ScheduleRow {
            date: game.date.clone(),
            away: game.away_abbrev.clone(),
            home: game.home_abbrev.clone(),
            time_et: format_time_et(utc),
            // game_state is "FUT"/"PRE"/"LIVE"/"FINAL"/"OFF" — lowercase
            // for the user-facing table.
            status: game
                .game_state
                .as_deref()
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_else(|| "?".into()),
        });
    }
    out
}

pub async fn run_schedule(
    team: Option<String>,
    days: u32,
    json: bool,
    csv: bool,
) -> anyhow::Result<()> {
    let client = NhlApiClient::production();
    let all_games = client
        .fetch_today_schedule()
        .await
        .context("fetching schedule")?;

    let team_up = team.as_deref().map(str::to_uppercase);
    let rows = project_schedule_rows(&all_games, team_up.as_deref(), days);

    if json {
        return emit_schedule_json(&rows, team_up.as_deref(), days);
    }
    if csv {
        return emit_schedule_csv(&rows);
    }

    // Default: comfy-table output.
    if rows.is_empty() {
        println!(
            "No games found in next {days} day(s){}.",
            team_up
                .as_deref()
                .map(|t| format!(" for {t}"))
                .unwrap_or_default()
        );
        return Ok(());
    }

    println!(
        "SCHEDULE — next {days} day(s){} ({} game(s))",
        team_up
            .as_deref()
            .map(|t| format!(" · {t}"))
            .unwrap_or_default(),
        rows.len()
    );

    use comfy_table::{ContentArrangement, Table};
    let mut table = Table::new();
    table
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["Date", "Away", "Home", "Time", "Status"]);
    for r in &rows {
        table.add_row(vec![
            r.date.as_str(),
            r.away.as_str(),
            r.home.as_str(),
            r.time_et.as_str(),
            r.status.as_str(),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn emit_schedule_json(rows: &[ScheduleRow], team: Option<&str>, days: u32) -> anyhow::Result<()> {
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        schema_version: u32,
        team: Option<&'a str>,
        days: u32,
        count: usize,
        games: &'a [ScheduleRow],
    }
    let env = Envelope {
        schema_version: 1,
        team,
        days,
        count: rows.len(),
        games: rows,
    };
    println!("{}", serde_json::to_string_pretty(&env)?);
    Ok(())
}

fn emit_schedule_csv(rows: &[ScheduleRow]) -> anyhow::Result<()> {
    let mut wtr = csv::Writer::from_writer(std::io::stdout());
    wtr.write_record(["date", "away", "home", "time_et", "status"])?;
    for r in rows {
        wtr.write_record([&r.date, &r.away, &r.home, &r.time_et, &r.status])?;
    }
    wtr.flush()?;
    Ok(())
}

#[cfg(test)]
mod schedule_tests {
    use super::*;
    use icelines_fetch::nhl_api::ScheduledGame;

    fn mk_game(date: &str, away: &str, home: &str, utc: &str, state: &str) -> ScheduledGame {
        ScheduledGame {
            date: date.into(),
            game_id: 0,
            game_type: 2,
            away_abbrev: away.into(),
            away_name: away.into(),
            home_abbrev: home.into(),
            home_name: home.into(),
            start_time_utc: format!("2026-05-05T{utc}:00Z"),
            away_score: None,
            home_score: None,
            game_state: Some(state.into()),
            last_period: None,
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    /// LP.1 / l0_project_schedule_rows_team_filter
    /// — Only games involving the filter team appear in the output.
    #[test]
    fn l0_project_schedule_rows_team_filter() {
        let games = vec![
            mk_game("2026-05-06", "EDM", "TOR", "23:00", "pre"),
            mk_game("2026-05-06", "BOS", "NYR", "23:30", "pre"),
        ];
        let rows = project_schedule_rows(&games, Some("EDM"), 7);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].away, "EDM");
    }

    /// LP.1 / l0_project_schedule_rows_days_cap
    /// — The cap is per distinct calendar day, not per game count.
    ///   3 games on day 1 + 2 on day 2 with `days=1` returns only the
    ///   day-1 games.
    #[test]
    fn l0_project_schedule_rows_days_cap() {
        let games = vec![
            mk_game("2026-05-06", "EDM", "TOR", "23:00", "pre"),
            mk_game("2026-05-06", "BOS", "NYR", "23:30", "pre"),
            mk_game("2026-05-06", "VGK", "LAK", "01:00", "pre"),
            mk_game("2026-05-07", "MTL", "OTT", "23:00", "pre"),
        ];
        let rows = project_schedule_rows(&games, None, 1);
        assert_eq!(rows.len(), 3);
        for r in &rows {
            assert_eq!(r.date, "2026-05-06");
        }
    }

    /// LP.1 / l0_project_schedule_rows_empty_no_team_match
    #[test]
    fn l0_project_schedule_rows_empty_no_team_match() {
        let games = vec![mk_game("2026-05-06", "EDM", "TOR", "23:00", "pre")];
        let rows = project_schedule_rows(&games, Some("BOS"), 7);
        assert!(rows.is_empty());
    }

    /// LP.1 / l0_schedule_row_serialize_to_csv
    /// — CSV emission produces the documented column order.
    #[test]
    fn l0_schedule_row_serialize_to_csv() {
        let row = ScheduleRow {
            date: "2026-05-06".into(),
            away: "EDM".into(),
            home: "TOR".into(),
            time_et: "7:00 PM ET".into(),
            status: "pre".into(),
        };
        let mut wtr = csv::Writer::from_writer(vec![]);
        wtr.write_record(["date", "away", "home", "time_et", "status"])
            .unwrap();
        wtr.write_record([&row.date, &row.away, &row.home, &row.time_et, &row.status])
            .unwrap();
        let bytes = wtr.into_inner().unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.starts_with("date,away,home,time_et,status\n"));
        assert!(s.contains("2026-05-06,EDM,TOR,7:00 PM ET,pre"));
    }
}

pub async fn run_trade(
    player_out: String,
    player_in: String,
    team: Option<String>,
) -> anyhow::Result<()> {
    use crate::config::Config;
    use icelines_core::{
        model::{DepthChartSlot, Season},
        name::normalize_name,
        season_stats::SeasonType,
        DepthChartBuilder, TeamAbbr,
    };
    use icelines_fetch::{snapshot::SnapshotStore, stats_loader::load_into_repo};

    let cfg = Config::load()?;
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    let season = Season(season_u32);
    let stype = SeasonType::Regular;

    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome =
        load_into_repo(season, stype, &store).map_err(|e| anyhow::anyhow!("loading repo: {e}"))?;
    let repo = &outcome.repo;

    let norm_out = normalize_name(&player_out);
    let norm_in = normalize_name(&player_in);

    let view_out = repo
        .skaters(season, stype)
        .find(|v| v.identity.name_normalized.contains(&norm_out))
        .with_context(|| format!("player out '{player_out}' not found"))?;
    let view_in = repo
        .skaters(season, stype)
        .find(|v| v.identity.name_normalized.contains(&norm_in))
        .with_context(|| format!("player in '{player_in}' not found"))?;

    let team_abbr = team
        .as_deref()
        .map(|t| t.to_uppercase())
        .unwrap_or_else(|| view_out.team_display().to_string());
    let team_abbr_t = TeamAbbr(team_abbr.clone());

    println!("TRADE ANALYSIS — {team_abbr} perspective");
    println!(
        "  OUT: {} ({:.2} pts/gp)",
        view_out.identity.full_name,
        view_out.pace_82().map(|p| p / 82.0).unwrap_or(0.0)
    );
    println!(
        "  IN:  {} ({:.2} pts/gp)",
        view_in.identity.full_name,
        view_in.pace_82().map(|p| p / 82.0).unwrap_or(0.0)
    );
    println!();

    let team_views = repo.team_roster(&team_abbr_t, season, stype);

    let chart_before = DepthChartBuilder::build_views(team_abbr_t.clone(), season, &team_views);
    let chart_after = DepthChartBuilder::build_views_with_swap(
        team_abbr_t,
        season,
        &team_views,
        view_in,
        view_out.identity.id,
    );

    let fmt3 = |row: Option<&[Option<DepthChartSlot>; 3]>| {
        row.map(|r| {
            r.iter()
                .map(|s| {
                    s.as_ref()
                        .map(|slot| slot.full_name.chars().take(12).collect::<String>())
                        .unwrap_or_else(|| "—".repeat(12))
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .unwrap_or_else(|| "—".to_owned())
    };
    let fmt2 = |row: Option<&[Option<DepthChartSlot>; 2]>| {
        row.map(|r| {
            r.iter()
                .map(|s| {
                    s.as_ref()
                        .map(|slot| slot.full_name.chars().take(16).collect::<String>())
                        .unwrap_or_else(|| "—".repeat(16))
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .unwrap_or_else(|| "—".to_owned())
    };

    println!("FORWARD LINES — BEFORE vs AFTER");
    println!("{}", "─".repeat(72usize));
    for line in 0..3 {
        let b_row = chart_before.forward_lines.get(line);
        let a_row = chart_after.forward_lines.get(line);
        println!("  Line {} BEFORE: {}", line + 1, fmt3(b_row));
        println!("  Line {} AFTER:  {}", line + 1, fmt3(a_row));
        println!();
    }

    println!("DEFENSE PAIRS — BEFORE vs AFTER");
    println!("{}", "─".repeat(72usize));
    for pair in 0..3 {
        let b_row = chart_before.defense_pairs.get(pair);
        let a_row = chart_after.defense_pairs.get(pair);
        println!("  Pair {} BEFORE: {}", pair + 1, fmt2(b_row));
        println!("  Pair {} AFTER:  {}", pair + 1, fmt2(a_row));
        println!();
    }

    let delta = view_in.pace_82().unwrap_or(0.0) - view_out.pace_82().unwrap_or(0.0);
    if delta > 5.0 {
        println!("  Result: UPGRADE (+{delta:.1} projected pts/82)");
    } else if delta < -5.0 {
        println!("  Result: DOWNGRADE ({delta:.1} projected pts/82)");
    } else {
        println!("  Result: roughly even ({delta:+.1} projected pts/82)");
    }
    Ok(())
}
