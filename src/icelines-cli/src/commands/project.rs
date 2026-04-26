use anyhow::Context;
use icelines_core::{
    compute_projection, model::MIN_GP, name::normalize_name, ProjectionMode,
};
use crate::commands::players::load_all_players;

const DEFAULT_REMAINING: u32 = 20; // fallback when schedule not available

pub async fn run(
    target: Option<String>,
    team:   Option<String>,
    mode:   String,
    games:  Option<u32>,
) -> anyhow::Result<()> {
    let mode: ProjectionMode = mode.parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    let players = load_all_players()?;
    let remaining = games.unwrap_or(DEFAULT_REMAINING);

    if let Some(name) = target {
        // Single player
        let norm  = normalize_name(&name);
        let player = players.iter()
            .find(|p| p.name_normalized.contains(&norm))
            .with_context(|| format!("player '{name}' not found — try a partial name"))?;

        let Some(score) = player.pace_score else {
            anyhow::bail!("'{}' has fewer than {MIN_GP} games — not enough data to project", player.full_name);
        };

        let current_ppg = score.pace_82 / 82.0;
        let age: u8 = player.birth_date.as_deref()
            .and_then(|d| d.get(..4))
            .and_then(|y| y.parse::<u16>().ok())
            .map(|y| (2026u16.saturating_sub(y)).min(99) as u8)
            .unwrap_or(27);

        let result = compute_projection(
            current_ppg, None, score.gp, age, remaining, mode
        );

        println!("PROJECTION — {} ({} · {:?} · {} remaining games)",
            player.full_name, player.team.as_str(), mode, remaining);
        println!("{}", "─".repeat(56usize));
        println!("  Current PPG:        {:.3}", result.current_ppg);
        println!("  α (blend weight):   {:.2}", result.alpha);
        println!("  Age factor:         {:.2}", result.age_factor);
        println!("  Projected pts:      {:.1}", result.projected_points);
        println!("  Confidence band:    {:.1} – {:.1}  (±{:.1})",
            result.low_band, result.high_band,
            result.confidence_band_width() / 2.0);
        println!();
        println!("  Note: career data requires `icelines fetch history` (Phase 4).");

    } else if let Some(team_abbr) = team {
        // Team-wide projection
        let team_upper = team_abbr.to_uppercase();
        let team_players: Vec<_> = players.iter()
            .filter(|p| p.team.as_str() == team_upper && p.pace_score.is_some())
            .collect();

        if team_players.is_empty() {
            anyhow::bail!("no rankable players found for {} — run `icelines fetch`", team_upper);
        }

        println!("PROJECTIONS — {} ({:?} mode · {} remaining)", team_upper, mode, remaining);
        println!("{}", "─".repeat(64usize));
        println!("{:<24} {:<4} {:<8} {:<8} {:<12}",
            "Player", "Pos", "Curr PPG", "Proj Pts", "Band");
        println!("{}", "─".repeat(64usize));

        for p in &team_players {
            let score = p.pace_score.unwrap();
            let current_ppg = score.pace_82 / 82.0;
            let age: u8 = p.birth_date.as_deref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<u16>().ok())
                .map(|y| (2026u16.saturating_sub(y)).min(99) as u8)
                .unwrap_or(27);
            let r = compute_projection(current_ppg, None, score.gp, age, remaining, mode);
            println!("{:<24} {:<4} {:<8.2} {:<8.1} {:.1}–{:.1}",
                p.full_name, p.position.abbreviation(),
                r.current_ppg, r.projected_points,
                r.low_band, r.high_band);
        }
    } else {
        anyhow::bail!("specify a player name or --team ABBREV");
    }

    Ok(())
}
