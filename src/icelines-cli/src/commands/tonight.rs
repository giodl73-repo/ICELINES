use anyhow::Context;
use icelines_fetch::nhl_api::NhlApiClient;

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

    println!("TONIGHT'S GAMES — {} ({} game(s))", date_label, schedule.len());
    println!("{}", "─".repeat(60usize));

    for game in &schedule {
        if let Some(ref t) = team_up {
            if &game.away_abbrev != t && &game.home_abbrev != t {
                continue;
            }
        }
        let time = game.start_time_utc.get(11..16).unwrap_or("?");
        println!("{} {} @ {} {}  UTC {}",
            game.away_abbrev, game.away_name,
            game.home_abbrev, game.home_name,
            time);
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
        let time = game.start_time_utc.get(11..16).unwrap_or("?");
        println!("  {} @ {}  UTC {}", game.away_abbrev, game.home_abbrev, time);
    }
    Ok(())
}

pub async fn run_trade(player_out: String, player_in: String, team: Option<String>) -> anyhow::Result<()> {
    use crate::commands::players::load_all_players;
    use icelines_core::{name::normalize_name, model::Season, DepthChartBuilder, TeamAbbr};

    let players = load_all_players()?;

    let norm_out = normalize_name(&player_out);
    let norm_in  = normalize_name(&player_in);

    let p_out = players.iter().find(|p| p.name_normalized.contains(&norm_out))
        .with_context(|| format!("player out '{player_out}' not found"))?
        .clone();
    let p_in = players.iter().find(|p| p.name_normalized.contains(&norm_in))
        .with_context(|| format!("player in '{player_in}' not found"))?
        .clone();

    let team_abbr = team.as_deref().unwrap_or(p_out.team.as_str()).to_uppercase();

    println!("TRADE ANALYSIS — {} perspective", team_abbr);
    println!("  OUT: {} ({:.2} pts/gp)", p_out.full_name,
        p_out.pace_score.map(|s| s.pace_82/82.0).unwrap_or(0.0));
    println!("  IN:  {} ({:.2} pts/gp)", p_in.full_name,
        p_in.pace_score.map(|s| s.pace_82/82.0).unwrap_or(0.0));
    println!();

    // Build BEFORE depth chart
    let team_players_before: Vec<_> = players.iter()
        .filter(|p| p.team.as_str() == team_abbr)
        .cloned().collect();

    // Build AFTER: remove p_out, add p_in (with team set to this team)
    let mut p_in_adjusted = p_in.clone();
    p_in_adjusted.team = TeamAbbr(team_abbr.clone());
    let team_players_after: Vec<_> = players.iter()
        .filter(|p| p.team.as_str() == team_abbr && p.name_normalized != norm_out)
        .cloned()
        .chain(std::iter::once(p_in_adjusted))
        .collect();

    let chart_before = DepthChartBuilder::build(
        TeamAbbr(team_abbr.clone()), Season(icelines_core::CURRENT_SEASON), team_players_before
    );
    let chart_after = DepthChartBuilder::build(
        TeamAbbr(team_abbr.clone()), Season(icelines_core::CURRENT_SEASON), team_players_after
    );

    // Compare top 3 lines
    println!("FORWARD LINES — BEFORE vs AFTER");
    println!("{}", "─".repeat(72usize));
    for line in 0..3 {
        let b_row = chart_before.forward_lines.get(line);
        let a_row = chart_after.forward_lines.get(line);
        let fmt = |row: Option<&[Option<icelines_core::model::Player>; 3]>| {
            row.map(|r| r.iter()
                .map(|s| s.as_ref().map(|p| p.full_name.chars().take(12).collect::<String>())
                    .unwrap_or_else(|| "—".repeat(12)))
                .collect::<Vec<_>>()
                .join(" | ")
            ).unwrap_or_else(|| "—".to_owned())
        };
        println!("  Line {} BEFORE: {}", line+1, fmt(b_row));
        println!("  Line {} AFTER:  {}", line+1, fmt(a_row));
        println!();
    }

    // Score delta
    let score = |p: &icelines_core::model::Player| p.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
    let delta = score(&p_in) - score(&p_out);
    if delta > 5.0 {
        println!("  Result: UPGRADE (+{:.1} projected pts/82)", delta);
    } else if delta < -5.0 {
        println!("  Result: DOWNGRADE ({:.1} projected pts/82)", delta);
    } else {
        println!("  Result: roughly even ({:+.1} projected pts/82)", delta);
    }
    Ok(())
}
