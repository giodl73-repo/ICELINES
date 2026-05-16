//! `icelines mates <PLAYER> [--top N]`
//!
//! Displays the top linemates for a player.  If a ShiftProfile has been
//! pre-computed and stored in the Positions snapshot tier, it is displayed
//! directly.  Otherwise the command falls back to a roster-derived placeholder
//! showing forwards on the same team.

use anyhow::Context as _;
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::name::normalize_name;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::PlayerView;
use icelines_fetch::snapshot::{SnapshotStore, SnapshotTier};
use icelines_fetch::stats_loader::load_into_repo;
use icelines_fetch::ShiftProfile;

use crate::config::Config;

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(
    player_name: String,
    top: usize,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    // Hart.5b2: load via load_into_repo + collect skaters into Vec<PlayerView>.
    let cfg = Config::load()?;
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{}' is not a YYYYZZZZ id", cfg.season_str()))?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(Season(season_u32), SeasonType::Regular, &store)
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))?;
    let views: Vec<PlayerView<'_>> = outcome
        .repo
        .skaters(Season(season_u32), SeasonType::Regular)
        .collect();

    let target = find_view(&views, &player_name)?;
    let format = Format::resolve(csv, json)?;

    let player_id = target.id().0;
    let filename = format!("{player_id}.json");

    match store.read_tier::<ShiftProfile>(&SnapshotTier::Positions, &filename) {
        Ok(profile) => display_profile(&profile, &views, top, format, out.as_deref())?,
        Err(_) => display_placeholder(*target, &views, top, format, out.as_deref())?,
    }

    Ok(())
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn display_profile(
    profile: &ShiftProfile,
    views: &[PlayerView<'_>],
    top: usize,
    format: crate::commands::output::Format,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    let headers = &["rank", "partner", "shared_shifts", "co_ice_pct"];
    let rows: Vec<Vec<String>> = profile
        .top_linemates
        .iter()
        .take(top)
        .enumerate()
        .map(|(i, lm)| {
            vec![
                (i + 1).to_string(),
                find_view_name(views, lm.partner_id),
                lm.shared_shifts.to_string(),
                format!("{:.1}", lm.co_ice_pct * 100.0),
            ]
        })
        .collect();

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
    target: PlayerView<'_>,
    views: &[PlayerView<'_>],
    top: usize,
    format: crate::commands::output::Format,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    let target_team = target.team_display().to_owned();
    let target_full_name = target.full_name().to_owned();
    let teammates: Vec<&PlayerView<'_>> = views
        .iter()
        .filter(|v| {
            v.team_display() == target_team
                && v.position().is_forward()
                && v.full_name() != target_full_name
        })
        .take(top)
        .collect();

    let headers = &["rank", "player", "pos"];
    let rows: Vec<Vec<String>> = teammates
        .iter()
        .enumerate()
        .map(|(i, v)| {
            vec![
                (i + 1).to_string(),
                v.full_name().to_owned(),
                v.position().abbreviation().to_owned(),
            ]
        })
        .collect();

    if format == Format::Table && out.is_none() {
        eprintln!("No shift data found for {target_full_name}.");
        eprintln!("Shift-profile fetch/bundling is not supported yet; showing roster fallback.");
        eprintln!();
        eprintln!("PLACEHOLDER — forwards on {target_team} roster:");
    }
    format.emit_to(headers, &rows, out)?;
    Ok(())
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn find_view<'a, 'v>(
    views: &'a [PlayerView<'v>],
    name: &str,
) -> anyhow::Result<&'a PlayerView<'v>> {
    let norm = normalize_name(name);
    views
        .iter()
        .find(|v| v.name_normalized().contains(&norm))
        .with_context(|| format!("player '{name}' not found in snapshot — try a partial name"))
}

fn find_view_name(views: &[PlayerView<'_>], player_id: u32) -> String {
    views
        .iter()
        .find(|v| v.id() == PlayerId(player_id))
        .map(|v| v.full_name().to_owned())
        .unwrap_or_else(|| format!("#{player_id}"))
}
