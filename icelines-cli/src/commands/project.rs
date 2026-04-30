use anyhow::Context;
use icelines_core::{
    compute_projection, model::MIN_GP, name::normalize_name, ProjectionMode,
};
use icelines_fetch::{career::load_career, snapshot::SnapshotStore};
use crate::commands::output::Format;
use crate::{commands::players::load_all_players, config::Config};

const DEFAULT_REMAINING: u32 = 20; // fallback when schedule not available

pub async fn run(
    target: Option<String>,
    team:   Option<String>,
    mode:   String,
    games:  Option<u32>,
    json:   bool,
    csv:    bool,
    out:    Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let mode: ProjectionMode = mode.parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;

    let players = load_all_players()?;
    let remaining = games.unwrap_or(DEFAULT_REMAINING);
    let format = Format::resolve(csv, json)?;

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

        // Use real career PPG from bundled historical data for regressed mode
        let cfg   = Config::load()?;
        let store = SnapshotStore::new(cfg.snapshot_dir());
        let career_ppg = load_career(&player.full_name, 5, &store)
            .map(|c| c.career_ppg as f64);

        let result = compute_projection(
            current_ppg, career_ppg, score.gp, age, remaining, mode
        );

        if format == Format::Table && out.is_none() {
            println!("PROJECTION — {} ({} · {:?} · {} remaining games)",
                player.full_name, player.team.as_str(), mode, remaining);
            println!("{}", "─".repeat(56usize));
            println!("  Current PPG:        {:.3}", result.current_ppg);
            if let Some(cp) = career_ppg {
                println!("  Career PPG:         {:.3}  (5-season avg)", cp);
            }
            println!("  α (blend weight):   {:.2}", result.alpha);
            println!("  Age factor:         {:.2}", result.age_factor);
            println!("  Projected pts:      {:.1}", result.projected_points);
            println!("  Confidence band:    {:.1} – {:.1}  (±{:.1})",
                result.low_band, result.high_band,
                result.confidence_band_width() / 2.0);
            return Ok(());
        }

        // Single-player CSV/JSON: long-form rows (one stat per row).
        let headers = &["stat", "value"];
        let career_str = career_ppg.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "—".to_owned());
        let rows: Vec<Vec<String>> = vec![
            vec!["player".to_owned(),         player.full_name.clone()],
            vec!["team".to_owned(),           player.team.as_str().to_owned()],
            vec!["mode".to_owned(),           format!("{mode:?}")],
            vec!["remaining_games".to_owned(),remaining.to_string()],
            vec!["current_ppg".to_owned(),    format!("{:.3}", result.current_ppg)],
            vec!["career_ppg".to_owned(),     career_str],
            vec!["alpha".to_owned(),          format!("{:.2}", result.alpha)],
            vec!["age_factor".to_owned(),     format!("{:.2}", result.age_factor)],
            vec!["projected_points".to_owned(), format!("{:.1}", result.projected_points)],
            vec!["band_low".to_owned(),       format!("{:.1}", result.low_band)],
            vec!["band_high".to_owned(),      format!("{:.1}", result.high_band)],
        ];
        format.emit_to(headers, &rows, out.as_deref())?;

    } else if let Some(team_abbr) = team {
        // Team-wide projection
        let team_upper = team_abbr.to_uppercase();
        let team_players: Vec<_> = players.iter()
            .filter(|p| p.team.as_str() == team_upper && p.pace_score.is_some())
            .collect();

        if team_players.is_empty() {
            anyhow::bail!("no rankable players found for {} — run `icelines fetch`", team_upper);
        }

        let headers = &["player", "pos", "current_ppg", "projected_points", "band_low", "band_high"];
        let rows: Vec<Vec<String>> = team_players.iter().map(|p| {
            let score = p.pace_score.unwrap();
            let current_ppg = score.pace_82 / 82.0;
            let age: u8 = p.birth_date.as_deref()
                .and_then(|d| d.get(..4))
                .and_then(|y| y.parse::<u16>().ok())
                .map(|y| (2026u16.saturating_sub(y)).min(99) as u8)
                .unwrap_or(27);
            let r = compute_projection(current_ppg, None, score.gp, age, remaining, mode);
            vec![
                p.full_name.clone(),
                p.position.abbreviation().to_owned(),
                format!("{:.2}", r.current_ppg),
                format!("{:.1}", r.projected_points),
                format!("{:.1}", r.low_band),
                format!("{:.1}", r.high_band),
            ]
        }).collect();

        if format == Format::Table && out.is_none() {
            println!("PROJECTIONS — {} ({:?} mode · {} remaining)", team_upper, mode, remaining);
        }
        format.emit_to(headers, &rows, out.as_deref())?;
    } else {
        anyhow::bail!("specify a player name or --team ABBREV");
    }

    Ok(())
}
