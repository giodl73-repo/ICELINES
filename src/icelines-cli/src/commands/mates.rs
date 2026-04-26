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

pub async fn run(player_name: String, top: usize) -> anyhow::Result<()> {
    let players = load_all_players()?;
    let target = find_player(&players, &player_name)?;

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
        Ok(profile) => display_profile(&profile, &players, top),
        Err(_) => display_placeholder(target, &players, top),
    }

    Ok(())
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn display_profile(profile: &ShiftProfile, players: &[Player], top: usize) {
    let toi_mins = profile.avg_ev_toi_seconds_per_game / 60;
    let toi_secs = profile.avg_ev_toi_seconds_per_game % 60;

    println!(
        "LINEMATES — player {} ({} games analyzed)",
        profile.player_id, profile.games_analyzed
    );
    println!(
        "Avg EV TOI/game: {}:{:02}",
        toi_mins, toi_secs
    );
    println!("{}", "─".repeat(62usize));
    println!(
        "{:<5} {:<24} {:>12} {:>10}",
        "Rank", "Partner", "Shared Games", "Co-Ice%"
    );
    println!("{}", "─".repeat(62usize));

    let displayed = profile.top_linemates.iter().take(top);
    for (i, lm) in displayed.enumerate() {
        let name = find_player_name(players, lm.partner_id);
        println!(
            "{:<5} {:<24} {:>12} {:>9.1}%",
            i + 1,
            name,
            lm.shared_shifts,
            lm.co_ice_pct * 100.0,
        );
    }

    let shown = profile.top_linemates.len().min(top);
    println!("\nShowing {shown} of {} linemates.", profile.top_linemates.len());
}

fn display_placeholder(target: &Player, players: &[Player], top: usize) {
    println!(
        "No shift data found for {}.",
        target.full_name
    );
    println!("Run `icelines fetch shifts` to compute linemate data.");
    println!();
    println!(
        "PLACEHOLDER — forwards on {} roster:",
        target.team.as_str()
    );
    println!("{}", "─".repeat(50usize));
    println!("{:<5} {:<24} {:<5}", "Rank", "Player", "Pos");
    println!("{}", "─".repeat(50usize));

    let teammates: Vec<&Player> = players
        .iter()
        .filter(|p| {
            p.team.as_str() == target.team.as_str()
                && p.position.is_forward()
                && p.full_name != target.full_name
        })
        .take(top)
        .collect();

    for (i, p) in teammates.iter().enumerate() {
        println!(
            "{:<5} {:<24} {:<5}",
            i + 1,
            p.full_name,
            p.position.abbreviation()
        );
    }

    if teammates.is_empty() {
        println!("  (no forwards found on roster)");
    }
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
