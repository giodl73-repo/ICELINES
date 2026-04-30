//! `icelines mates <PLAYER> [--top N]`
//!
//! Displays the top linemates for a player.  If a ShiftProfile has been
//! pre-computed and stored in the Positions snapshot tier, it is displayed
//! directly.  Otherwise the command falls back to a roster-derived placeholder
//! showing forwards on the same team.

use anyhow::Context as _;
use icelines_core::{model::Player, name::normalize_name};
use icelines_fetch::{
    snapshot::{SnapshotStore, SnapshotTier},
    ShiftProfile,
};

use crate::commands::players::load_all_players;
use crate::config::Config;

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(
    player_name: String,
    top: usize,
    json: bool,
    csv:  bool,
    out:  Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;
    let players = load_all_players()?;
    let target = find_player(&players, &player_name)?;
    let format = Format::resolve(csv, json)?;

    let player_id = match target.nhl_id {
        Some(id) => id,
        None => {
            println!(
                "Player '{}' has no NHL ID in snapshot — cannot look up linemate data.",
                target.full_name
            );
            return Ok(());
        }
    };

    // Try to load a pre-computed ShiftProfile from the Positions snapshot tier.
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let filename = format!("{player_id}.json");

    match store.read_tier::<ShiftProfile>(&SnapshotTier::Positions, &filename) {
        Ok(profile) => display_profile(&profile, &players, top, format, out.as_deref())?,
        Err(_)      => display_placeholder(target, &players, top, format, out.as_deref())?,
    }

    Ok(())
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn display_profile(
    profile: &ShiftProfile,
    players: &[Player],
    top: usize,
    format: crate::commands::output::Format,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    let headers = &["rank", "partner", "shared_shifts", "co_ice_pct"];
    let rows: Vec<Vec<String>> = profile.top_linemates.iter().take(top).enumerate().map(|(i, lm)| {
        vec![
            (i + 1).to_string(),
            find_player_name(players, lm.partner_id),
            lm.shared_shifts.to_string(),
            format!("{:.1}", lm.co_ice_pct * 100.0),
        ]
    }).collect();

    if format == Format::Table && out.is_none() {
        let toi_mins = profile.avg_ev_toi_seconds_per_game / 60;
        let toi_secs = profile.avg_ev_toi_seconds_per_game % 60;
        println!(
            "LINEMATES — player {} ({} games analyzed)",
            profile.player_id, profile.games_analyzed
        );
        println!("Avg EV TOI/game: {}:{:02}", toi_mins, toi_secs);
    }
    format.emit_to(headers, &rows, out)?;
    Ok(())
}

fn display_placeholder(
    target: &Player,
    players: &[Player],
    top: usize,
    format: crate::commands::output::Format,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    let teammates: Vec<&Player> = players.iter()
        .filter(|p| {
            p.team.as_str() == target.team.as_str()
                && p.position.is_forward()
                && p.full_name != target.full_name
        })
        .take(top)
        .collect();

    let headers = &["rank", "player", "pos"];
    let rows: Vec<Vec<String>> = teammates.iter().enumerate().map(|(i, p)| vec![
        (i + 1).to_string(),
        p.full_name.clone(),
        p.position.abbreviation().to_owned(),
    ]).collect();

    if format == Format::Table && out.is_none() {
        eprintln!("No shift data found for {}.", target.full_name);
        eprintln!("Run `icelines fetch shifts` to compute linemate data.");
        eprintln!();
        eprintln!("PLACEHOLDER — forwards on {} roster:", target.team.as_str());
    }
    format.emit_to(headers, &rows, out)?;
    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn find_player<'a>(players: &'a [Player], name: &str) -> anyhow::Result<&'a Player> {
    let norm = normalize_name(name);
    players
        .iter()
        .find(|p| p.name_normalized.contains(&norm))
        .with_context(|| format!("player '{name}' not found in snapshot — try a partial name"))
}

fn find_player_name(players: &[Player], player_id: u32) -> String {
    players
        .iter()
        .find(|p| p.nhl_id == Some(player_id))
        .map(|p| p.full_name.clone())
        .unwrap_or_else(|| format!("#{player_id}"))
}
