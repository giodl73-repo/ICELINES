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
            let display_h = match et_h % 12 { 0 => 12, n => n };
            return format!("{display_h}:{m:02} {period} ET");
        }
    }
    format!("{utc_hhmm} UTC")
}

pub async fn run(team_filter: Option<String>) -> anyhow::Result<()> {
    let client = NhlApiClient::production();
    let all_games = client.fetch_today_schedule().await
        .context("fetching today's schedule")?;

    // Filter to today only (first date in the gameWeek)
    let today = all_games.first().map(|g| g.date.as_str()).unwrap_or("");
    let schedule: Vec<_> = all_games.iter()
        .filter(|g| g.date.is_empty() || g.date == today)
        .collect();

    if schedule.is_empty() {
        println!("No games scheduled today.");
        return Ok(());
    }

    let team_up = team_filter.as_deref().map(str::to_uppercase);
    let date_label = if today.is_empty() { "today".to_owned() } else { today.to_owned() };

    // Apply team filter up front so the header count matches what's displayed
    let filtered: Vec<_> = schedule.iter().filter(|g| {
        match &team_up {
            Some(t) => &g.away_abbrev == t || &g.home_abbrev == t,
            None    => true,
        }
    }).collect();

    let team_label = team_up.as_deref().map(|t| format!(" · {t}")).unwrap_or_default();
    println!("TONIGHT'S GAMES — {}{} ({} game(s))", date_label, team_label, filtered.len());
    println!("{}", "─".repeat(60usize));

    if filtered.is_empty() {
        println!("No games tonight{}.", team_label);
        return Ok(());
    }

    for game in &filtered {
        let utc = game.start_time_utc.get(11..16).unwrap_or("?");
        let et  = format_time_et(utc);
        println!("{} {} @ {} {}  {}",
            game.away_abbrev, game.away_name,
            game.home_abbrev, game.home_name,
            et);
    }
    Ok(())
}

pub async fn run_schedule(team: Option<String>, days: u32) -> anyhow::Result<()> {
    let client = NhlApiClient::production();
    let all_games = client.fetch_today_schedule().await
        .context("fetching schedule")?;

    if all_games.is_empty() {
        println!("No upcoming games found.");
        return Ok(());
    }

    let team_up = team.as_deref().map(str::to_uppercase);

    // Group by date, show up to `days` distinct dates
    let mut current_date = String::new();
    let mut days_shown = 0u32;

    println!("SCHEDULE — next {days} day(s){}",
        team_up.as_deref().map(|t| format!(" · {t}")).unwrap_or_default());
    println!("{}", "─".repeat(60usize));

    for game in &all_games {
        if game.date != current_date {
            if days_shown >= days { break; }
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
        let et  = format_time_et(utc);
        println!("  {} @ {}  {}", game.away_abbrev, game.home_abbrev, et);
    }
    Ok(())
}

pub async fn run_trade(player_out: String, player_in: String, team: Option<String>) -> anyhow::Result<()> {
    use crate::config::Config;
    use icelines_core::model::Season;
    use icelines_core::name::normalize_name;
    use icelines_core::season_stats::SeasonType;
    use icelines_core::{DepthChartBuilder, TeamAbbr};
    use icelines_fetch::snapshot::SnapshotStore;
    use icelines_fetch::stats_loader::load_into_repo;

    // Hart.5b2g: load via load_into_repo, find p_out / p_in among
    // skaters as PlayerView. Build BEFORE/AFTER charts via build_views.
    let cfg = Config::load()?;
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(Season(season_u32), SeasonType::Regular, &store)
        .with_context(|| format!("loading season {season_u32} for trade analysis"))?;

    let norm_out = normalize_name(&player_out);
    let norm_in = normalize_name(&player_in);

    let v_out = outcome
        .repo
        .skaters(Season(season_u32), SeasonType::Regular)
        .find(|v| v.name_normalized().contains(&norm_out))
        .with_context(|| format!("player out '{player_out}' not found"))?;
    let v_in = outcome
        .repo
        .skaters(Season(season_u32), SeasonType::Regular)
        .find(|v| v.name_normalized().contains(&norm_in))
        .with_context(|| format!("player in '{player_in}' not found"))?;

    let team_abbr = team
        .as_deref()
        .unwrap_or(v_out.team_display())
        .to_uppercase();

    println!("TRADE ANALYSIS — {} perspective", team_abbr);
    println!(
        "  OUT: {} ({:.2} pts/gp)",
        v_out.full_name(),
        v_out.pace_score().map(|s| s.pace_82 / 82.0).unwrap_or(0.0)
    );
    println!(
        "  IN:  {} ({:.2} pts/gp)",
        v_in.full_name(),
        v_in.pace_score().map(|s| s.pace_82 / 82.0).unwrap_or(0.0)
    );
    println!();

    // BEFORE chart: skaters whose last-stint team is this team.
    let team_views_before: Vec<_> = outcome
        .repo
        .team_roster(&TeamAbbr(team_abbr.clone()), Season(season_u32), SeasonType::Regular)
        .into_iter()
        .filter(|v| !v.is_goalie())
        .collect();

    // AFTER chart: BEFORE minus v_out, plus v_in. v_in's view points
    // at his current-team stats; for the chart we still need a Player
    // with team = team_abbr. Convert + clone to legacy Player here so
    // we can mutate the team field (the only legitimate use of the
    // mutable Player struct in the trade-hypothetical context).
    let v_out_norm = v_out.name_normalized().to_owned();
    let mut player_in_adjusted = icelines_core::stats_repository::player_from_view(&v_in);
    player_in_adjusted.team = TeamAbbr(team_abbr.clone());
    let mut team_players_after: Vec<_> = team_views_before
        .iter()
        .filter(|v| v.name_normalized() != v_out_norm)
        .map(icelines_core::stats_repository::player_from_view)
        .collect();
    team_players_after.push(player_in_adjusted);

    let chart_before = DepthChartBuilder::build_views(
        TeamAbbr(team_abbr.clone()),
        Season(season_u32),
        &team_views_before,
    );
    let chart_after = DepthChartBuilder::build(
        TeamAbbr(team_abbr.clone()),
        Season(season_u32),
        team_players_after,
    );

    let fmt3 = |row: Option<&[Option<icelines_core::model::Player>; 3]>| {
        row.map(|r| r.iter()
            .map(|s| s.as_ref().map(|p| p.full_name.chars().take(12).collect::<String>())
                .unwrap_or_else(|| "—".repeat(12)))
            .collect::<Vec<_>>()
            .join(" | ")
        ).unwrap_or_else(|| "—".to_owned())
    };
    let fmt2 = |row: Option<&[Option<icelines_core::model::Player>; 2]>| {
        row.map(|r| r.iter()
            .map(|s| s.as_ref().map(|p| p.full_name.chars().take(16).collect::<String>())
                .unwrap_or_else(|| "—".repeat(16)))
            .collect::<Vec<_>>()
            .join(" | ")
        ).unwrap_or_else(|| "—".to_owned())
    };

    // Compare top 3 forward lines
    println!("FORWARD LINES — BEFORE vs AFTER");
    println!("{}", "─".repeat(72usize));
    for line in 0..3 {
        let b_row = chart_before.forward_lines.get(line);
        let a_row = chart_after.forward_lines.get(line);
        println!("  Line {} BEFORE: {}", line+1, fmt3(b_row));
        println!("  Line {} AFTER:  {}", line+1, fmt3(a_row));
        println!();
    }

    // Compare top 3 defense pairs
    println!("DEFENSE PAIRS — BEFORE vs AFTER");
    println!("{}", "─".repeat(72usize));
    for pair in 0..3 {
        let b_row = chart_before.defense_pairs.get(pair);
        let a_row = chart_after.defense_pairs.get(pair);
        println!("  Pair {} BEFORE: {}", pair+1, fmt2(b_row));
        println!("  Pair {} AFTER:  {}", pair+1, fmt2(a_row));
        println!();
    }

    // Score delta — read pace_82 directly off the views.
    let score_view =
        |v: &icelines_core::stats_repository::PlayerView<'_>| v.pace_82().unwrap_or(0.0);
    let delta = score_view(&v_in) - score_view(&v_out);
    if delta > 5.0 {
        println!("  Result: UPGRADE (+{:.1} projected pts/82)", delta);
    } else if delta < -5.0 {
        println!("  Result: DOWNGRADE ({:.1} projected pts/82)", delta);
    } else {
        println!("  Result: roughly even ({:+.1} projected pts/82)", delta);
    }
    Ok(())
}
