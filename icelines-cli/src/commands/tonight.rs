use anyhow::Context;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::timeframe::Timeframe;
use icelines_core::{ScheduleView, ScheduledGameInput, ScoresView, ViewContext, ViewWindow};
use icelines_fetch::nhl_api::NhlApiClient;

/// Phase Foster.1 — strict YYYY-MM-DD parser. Returns the canonical
/// string back so callers can hand it straight to NHL API URLs (which
/// also expect YYYY-MM-DD). Rejects any value chrono cannot resolve
/// to a real calendar date — catches "2026-13-01" etc.
pub(crate) fn parse_iso_date(s: &str) -> anyhow::Result<String> {
    let parsed = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| anyhow::anyhow!("invalid date '{s}' — expected YYYY-MM-DD ({e})"))?;
    Ok(parsed.format("%Y-%m-%d").to_string())
}

/// LP review fix #1 — true US DST detection for ET conversion.
///
/// US DST since 2007: starts second Sunday of March, ends first Sunday
/// of November. Returns true when the given UTC date is during DST in
/// the America/New_York zone (i.e. EDT = UTC-4). Returns false for EST
/// = UTC-5. This is a deliberate small implementation rather than
/// adding `chrono-tz` (which compiles in a multi-MB IANA db) — we only
/// need US/Eastern.
///
/// The boundary is approximate at the transition seconds (DST flips at
/// 2 AM local), but for NHL game start-times printed to the minute,
/// rounding to a whole-day boundary loses at most a handful of seconds
/// of precision twice a year.
fn is_us_eastern_dst(date: chrono::NaiveDate) -> bool {
    use chrono::Datelike;
    let year = date.year();
    let dst_start = nth_sunday_of(year, 3, 2); // 2nd Sunday of March
    let dst_end = nth_sunday_of(year, 11, 1); // 1st Sunday of November
    date >= dst_start && date < dst_end
}

/// Returns the date of the Nth (1-indexed) Sunday of a given month.
fn nth_sunday_of(year: i32, month: u32, n: u32) -> chrono::NaiveDate {
    use chrono::{Datelike, Weekday};
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1)
        .expect("month/year combo should always produce day 1");
    let first_dow_offset = (Weekday::Sun.num_days_from_monday() as i64
        - first.weekday().num_days_from_monday() as i64)
        .rem_euclid(7);
    let first_sunday_day = 1 + first_dow_offset as u32;
    let nth_sunday_day = first_sunday_day + (n - 1) * 7;
    chrono::NaiveDate::from_ymd_opt(year, month, nth_sunday_day)
        .expect("nth Sunday should land within the month for n=1..=2 in March/November")
}

/// Convert a UTC time on a given date to "H:MM AM/PM ET". Uses the
/// game's date to pick EDT (UTC-4) vs EST (UTC-5). Falls back to UTC
/// if either parse fails.
fn format_time_et(utc_hhmm: &str, game_date: &str) -> String {
    let date = chrono::NaiveDate::parse_from_str(game_date, "%Y-%m-%d").ok();
    let parts: Vec<&str> = utc_hhmm.splitn(2, ':').collect();
    if parts.len() == 2 {
        if let (Ok(h), Ok(m)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            let offset_hours = match date {
                Some(d) if is_us_eastern_dst(d) => 4,
                _ => 5,
            };
            let et_h = (h + 24 - offset_hours) % 24;
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

pub async fn run(
    team_filter: Option<String>,
    date: Option<String>,
    widen_to_week: bool,
) -> anyhow::Result<()> {
    let client = NhlApiClient::production();
    // Phase Foster.1 — anchor on `--date` if supplied; otherwise the
    // existing today path.
    let anchor = match date.as_deref() {
        Some(d) => Some(parse_iso_date(d)?),
        None => None,
    };
    let all_games = match anchor.as_deref() {
        Some(d) => client
            .fetch_schedule_for_date(d)
            .await
            .with_context(|| format!("fetching schedule for {d}"))?,
        None => client
            .fetch_today_schedule()
            .await
            .context("fetching today's schedule")?,
    };

    // Phase Foster +7 — `--week` / `--month` keeps the full 7-day
    // gameWeek; default narrows to the anchor day.
    let day = anchor
        .as_deref()
        .or_else(|| all_games.first().map(|g| g.date.as_str()))
        .unwrap_or("");
    let schedule: Vec<_> = if widen_to_week {
        all_games.iter().collect()
    } else {
        all_games
            .iter()
            .filter(|g| g.date.is_empty() || g.date == day)
            .collect()
    };
    let today = day;

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
    let scores_view = ScoresView::from_games(
        ViewContext::new(ViewWindow::new(
            Season(icelines_core::CURRENT_SEASON),
            SeasonType::Regular,
        )),
        chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Local::now().date_naive()),
        chrono::Local::now().date_naive(),
        if widen_to_week {
            // The NHL API `gameWeek` payload is a rolling date window,
            // not necessarily an ISO Monday-Sunday week. Preserve the
            // existing CLI behavior by keeping every pre-filtered row.
            Timeframe::Season
        } else {
            Timeframe::Day
        },
        filtered
            .iter()
            .map(|game| scheduled_game_input((**game).clone()))
            .collect(),
    );

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

    for day in &scores_view.days {
        for game in &day.rows {
            let utc = game.start_time_utc.get(11..16).unwrap_or("?");
            let et = format_time_et(utc, &day.date);
            println!(
                "{} {} @ {} {}  {}",
                game.away_abbrev, game.away_name, game.home_abbrev, game.home_name, et
            );
        }
    }
    Ok(())
}

fn scheduled_game_input(game: icelines_fetch::nhl_api::ScheduledGame) -> ScheduledGameInput {
    ScheduledGameInput {
        game_id: game.game_id,
        date: game.date,
        game_type: game.game_type,
        away_abbrev: game.away_abbrev,
        away_name: game.away_name,
        home_abbrev: game.home_abbrev,
        home_name: game.home_name,
        start_time_utc: game.start_time_utc,
        away_score: game.away_score,
        home_score: game.home_score,
        game_state: game.game_state,
        last_period: game.last_period,
        series_game: game.series_game,
        away_wins: game.away_wins,
        home_wins: game.home_wins,
    }
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
    let mut selected_games = Vec::new();
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
        selected_games.push(game.clone());
    }

    let active_date = selected_games.first().map(|game| game.date.clone());
    let view = ScheduleView::from_games(
        ViewContext::new(ViewWindow::new(
            Season(icelines_core::CURRENT_SEASON),
            SeasonType::Regular,
        )),
        icelines_core::CURRENT_SEASON.to_string(),
        team_up.unwrap_or_default().to_string(),
        active_date,
        &[],
        selected_games
            .into_iter()
            .map(scheduled_game_input)
            .collect(),
    );

    view.rows
        .into_iter()
        .map(|row| {
            let utc = row.start_time_utc.get(11..16).unwrap_or("?");
            let time_et = format_time_et(utc, &row.date);
            ScheduleRow {
                date: row.date,
                away: row.away_abbrev,
                home: row.home_abbrev,
                time_et,
                status: row.state_label.to_ascii_lowercase(),
            }
        })
        .collect()
}

pub async fn run_schedule(
    team: Option<String>,
    days: u32,
    json: bool,
    csv: bool,
    date: Option<String>,
) -> anyhow::Result<()> {
    let client = NhlApiClient::production();
    // Phase Foster.1 — anchor on `--date` if supplied; otherwise today.
    let anchor = match date.as_deref() {
        Some(d) => Some(parse_iso_date(d)?),
        None => None,
    };
    let all_games = match anchor.as_deref() {
        Some(d) => client
            .fetch_schedule_for_date(d)
            .await
            .with_context(|| format!("fetching schedule for {d}"))?,
        None => client
            .fetch_today_schedule()
            .await
            .context("fetching schedule")?,
    };

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
    // Post-LP review fix #5 — envelope shape matches the King.2.4 web
    // convention `{schema_version, route, data, meta}`. Query echo
    // (team / days / count) lives under `meta` so consumers can read
    // `env.data` uniformly across all icelines JSON outputs.
    #[derive(serde::Serialize)]
    struct Meta<'a> {
        team: Option<&'a str>,
        days: u32,
        count: usize,
    }
    #[derive(serde::Serialize)]
    struct Envelope<'a> {
        schema_version: u32,
        route: &'static str,
        data: &'a [ScheduleRow],
        meta: Meta<'a>,
    }
    let env = Envelope {
        schema_version: 1,
        route: "schedule",
        data: rows,
        meta: Meta {
            team,
            days,
            count: rows.len(),
        },
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
#[allow(clippy::items_after_test_module)]
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

    /// Post-LP review / l0_format_time_et_uses_dst_in_summer
    /// — Mid-July: 23:30 UTC = 7:30 PM EDT (UTC-4).
    #[test]
    fn l0_format_time_et_uses_dst_in_summer() {
        assert_eq!(format_time_et("23:30", "2026-07-15"), "7:30 PM ET");
    }

    /// Post-LP review / l0_format_time_et_uses_est_in_winter
    /// — Mid-January: 23:30 UTC = 6:30 PM EST (UTC-5).
    ///   Pre-fix this rendered as 7:30 PM (off by one hour).
    #[test]
    fn l0_format_time_et_uses_est_in_winter() {
        assert_eq!(format_time_et("23:30", "2026-01-15"), "6:30 PM ET");
    }

    /// Post-LP review / l0_format_time_et_dst_boundary_march
    /// — DST 2026 starts Sunday March 8 (2nd Sunday of March).
    ///   March 7 is EST (UTC-5); March 8 is EDT (UTC-4).
    #[test]
    fn l0_format_time_et_dst_boundary_march() {
        assert_eq!(format_time_et("18:00", "2026-03-07"), "1:00 PM ET");
        assert_eq!(format_time_et("18:00", "2026-03-08"), "2:00 PM ET");
    }

    /// Post-LP review / l0_format_time_et_dst_boundary_november
    /// — DST 2026 ends Sunday November 1 (1st Sunday).
    ///   October 31 is EDT (UTC-4); November 1 is EST (UTC-5).
    #[test]
    fn l0_format_time_et_dst_boundary_november() {
        assert_eq!(format_time_et("18:00", "2026-10-31"), "2:00 PM ET");
        assert_eq!(format_time_et("18:00", "2026-11-01"), "1:00 PM ET");
    }

    /// Post-LP review / l0_format_time_et_garbage_date_falls_back_to_est
    /// — Unparseable date defaults to EST (UTC-5), the more common offset
    ///   during regular-season hockey (Nov–Mar).
    #[test]
    fn l0_format_time_et_garbage_date_falls_back_to_est() {
        // Garbage date → EST = UTC-5; 23:30 → 6:30 PM.
        assert_eq!(format_time_et("23:30", "not-a-date"), "6:30 PM ET");
    }

    // ── Phase Foster.1 — date parser tests ───────────────────────────────

    #[test]
    fn l0_foster1_parse_iso_date_accepts_canonical() {
        assert_eq!(parse_iso_date("2026-05-06").unwrap(), "2026-05-06");
        assert_eq!(parse_iso_date("2014-10-08").unwrap(), "2014-10-08");
    }

    #[test]
    fn l0_foster1_parse_iso_date_rejects_garbage() {
        assert!(parse_iso_date("not-a-date").is_err());
        assert!(parse_iso_date("2026/05/06").is_err());
        assert!(parse_iso_date("2026-13-01").is_err(), "month 13 invalid");
        assert!(parse_iso_date("2026-02-30").is_err(), "Feb 30 invalid");
    }

    #[test]
    fn l0_foster1_parse_iso_date_far_past() {
        // 2014-01-01 — verified the NHL API serves dates this far back.
        assert_eq!(parse_iso_date("2014-01-01").unwrap(), "2014-01-01");
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
