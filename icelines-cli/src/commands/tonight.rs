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

pub async fn run_schedule(team: Option<String>, days: u32) -> anyhow::Result<()> {
    let client = NhlApiClient::production();
    let all_games = client
        .fetch_today_schedule()
        .await
        .context("fetching schedule")?;

    if all_games.is_empty() {
        println!("No upcoming games found.");
        return Ok(());
    }

    let team_up = team.as_deref().map(str::to_uppercase);

    // Group by date, show up to `days` distinct dates
    let mut current_date = String::new();
    let mut days_shown = 0u32;

    println!(
        "SCHEDULE — next {days} day(s){}",
        team_up
            .as_deref()
            .map(|t| format!(" · {t}"))
            .unwrap_or_default()
    );
    println!("{}", "─".repeat(60usize));

    for game in &all_games {
        if game.date != current_date {
            if days_shown >= days {
                break;
            }
            current_date = game.date.clone();
            days_shown += 1;
            println!("\n{}", current_date);
        }
        if let Some(ref t) = team_up {
            if &game.away_abbrev != t && &game.home_abbrev != t {
                continue;
            }
        }
        let utc = game.start_time_utc.get(11..16).unwrap_or("?");
        let et = format_time_et(utc);
        println!("  {} @ {}  {}", game.away_abbrev, game.home_abbrev, et);
    }
    Ok(())
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

    let store = SnapshotStore::new(&cfg.snapshot_dir());
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
