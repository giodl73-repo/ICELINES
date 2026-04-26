use anyhow::Context;
use icelines_core::{
    compute_cross_team_metrics, compute_projection, name::normalize_name,
    model::MIN_GP, ProjectionMode,
};
use crate::commands::players::load_all_players;

pub async fn run(player_name: String, format: String) -> anyhow::Result<()> {
    let players = load_all_players()?;
    let norm    = normalize_name(&player_name);
    let player  = players.iter()
        .find(|p| p.name_normalized.contains(&norm))
        .with_context(|| format!("player '{player_name}' not found"))?;

    let md = format.to_lowercase() == "markdown";
    let sep: String = if md { "---".to_owned() } else { "─".repeat(60usize) };

    // ── Section 1: Bio ────────────────────────────────────────────────────────
    println!("{}", if md { format!("# Scouting Report — {}", player.full_name) }
             else { format!("SCOUTING REPORT — {}", player.full_name) });
    println!("{sep}");
    println!();
    println!("## 1. Bio");
    let age = player.birth_date.as_deref()
        .and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string()).unwrap_or_else(|| "—".to_owned());
    let draft = match (player.draft_year, player.draft_round, player.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} · Round {r} · Pick #{o}"),
        (Some(y), _, _)             => y.to_string(),
        _                           => "Undrafted".to_owned(),
    };
    println!("  Team:         {} ({})", player.team.as_str(),
        if md { format!("*{}*", player.team.as_str()) } else { player.team.as_str().to_owned() });
    println!("  Position:     {:?}", player.position);
    println!("  Age:          {}", age);
    println!("  Nationality:  {}", player.nationality_code.as_deref().unwrap_or("—"));
    println!("  Draft:        {}", draft);
    println!("  Handedness:   {}", player.shoots_catches.as_deref().unwrap_or("—"));

    // ── Section 2: Current season stats ──────────────────────────────────────
    println!();
    println!("## 2. Current Season");
    if let Some(s) = player.pace_score {
        let ppg  = s.pace_82 / 82.0;
        let gpg  = s.goals_per_82 / 82.0;
        println!("  GP:           {}", s.gp);
        println!("  G:            {}  →  {:.0}/82", player.season_goals, s.goals_per_82);
        println!("  A:            {}  →  {:.0}/82", player.season_assists, s.pace_82 - s.goals_per_82);
        println!("  PPG:          {ppg:.3} pts/gp");
        println!("  G/gp:         {gpg:.3}");
        println!("  Proj/82g:     {:.1}", s.pace_82);
    } else {
        println!("  < {MIN_GP} games played — not enough data");
    }

    // ── Section 3: Career trajectory ─────────────────────────────────────────
    println!();
    println!("## 3. Career Trajectory");
    println!("  Current season only (multi-season history: `icelines fetch history` Phase 4)");

    // ── Section 4: Peer group rank ────────────────────────────────────────────
    println!();
    println!("## 4. Peer Group Rank");
    let draft_year = player.draft_year.unwrap_or(0);
    if draft_year > 0 {
        let peers: Vec<_> = players.iter()
            .filter(|p| {
                p.position == player.position &&
                p.draft_year.map(|y| (y as i32 - draft_year as i32).abs() <= 1).unwrap_or(false) &&
                p.pace_score.is_some()
            })
            .collect();
        let rank = peers.iter()
            .filter(|p| p.pace_score.map(|s| s.pace_82).unwrap_or(0.0) >
                player.pace_score.map(|s| s.pace_82).unwrap_or(0.0))
            .count() + 1;
        println!("  Draft class:  {} ± 1 year, {:?}", draft_year, player.position);
        println!("  Peer count:   {}", peers.len());
        println!("  Peer rank:    #{rank} of {}", peers.len());
        let pct = if peers.len() > 1 {
            100 - (rank * 100 / peers.len())
        } else { 100 };
        println!("  Percentile:   {}th", pct);
    } else {
        println!("  Draft data not available");
    }

    // ── Section 5: Linemates ──────────────────────────────────────────────────
    println!();
    println!("## 5. Linemates");
    println!("  Run `icelines fetch shifts` then `icelines mates {}` for shift-based linemate data.",
        player.full_name.split_whitespace().last().unwrap_or(&player.full_name));
    let teammates: Vec<_> = players.iter()
        .filter(|p| p.team == player.team && p.position == player.position
            && p.name_normalized != player.name_normalized && p.pace_score.is_some())
        .take(3).collect();
    println!("  Same-team same-position players:");
    for t in &teammates {
        let ppg = t.pace_score.map(|s| format!("{:.2}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
        println!("    {} ({} pts/gp)", t.full_name, ppg);
    }

    // ── Section 6: Depth chart position ──────────────────────────────────────
    println!();
    println!("## 6. Depth Chart Position");
    let same_pos: Vec<_> = players.iter()
        .filter(|p| p.team == player.team && p.position == player.position && p.pace_score.is_some())
        .collect();
    let rank_on_team = same_pos.iter()
        .filter(|p| p.pace_score.map(|s| s.pace_82).unwrap_or(0.0) >
            player.pace_score.map(|s| s.pace_82).unwrap_or(0.0))
        .count() + 1;
    println!("  Line {} {:?} on {} (#{rank_on_team} of {} {:?}s)",
        rank_on_team, player.position, player.team.as_str(),
        same_pos.len(), player.position);

    // ── Section 7: Cross-team value ───────────────────────────────────────────
    println!();
    println!("## 7. Cross-Team Value");
    let metrics = compute_cross_team_metrics(&players);
    if let Some(m) = metrics.iter().find(|m| m.player_nhl_id == player.nhl_id) {
        println!("  Own line:      #{}", m.own_line);
        println!("  Avg elsewhere: L{:.2}", m.avg_other_line);
        println!("  Delta:         {:+.2}", m.delta);
        let cls = m.web_fit_class();
        println!("  Fit class:     {} {}", cls.label(), match cls {
            icelines_core::WebFitClass::Elite   => "elite — plays above their line on most teams",
            icelines_core::WebFitClass::Solid   => "solid — fits their role well",
            icelines_core::WebFitClass::Buried  => "buried — underused, worth more elsewhere",
            icelines_core::WebFitClass::Stretch => "stretch — overextended in current role",
        });
    } else {
        println!("  Cross-team metrics unavailable (GP < {MIN_GP})");
    }

    // ── Section 8: Fit interpretation ────────────────────────────────────────
    println!();
    println!("## 8. Fit Interpretation");
    if let Some(s) = player.pace_score {
        let age_n: u8 = age.parse().unwrap_or(27);
        let proj = compute_projection(s.pace_82/82.0, None, s.gp, age_n, 20, ProjectionMode::Regressed);
        println!("  Regressed projection (next 20 games): {:.1} pts", proj.projected_points);
        println!("  Confidence band: {:.1} – {:.1}", proj.low_band, proj.high_band);
        if s.pace_82 > 80.0 {
            println!("  Assessment: elite-tier producer — franchise-caliber");
        } else if s.pace_82 > 50.0 {
            println!("  Assessment: top-6 contributor — strong fantasy asset");
        } else if s.pace_82 > 30.0 {
            println!("  Assessment: depth/third-line player");
        } else {
            println!("  Assessment: fourth-line / below average production");
        }
    }
    println!();
    Ok(())
}
