use crate::config::Config;
use crate::render::terminal::render_team_card;
use anyhow::{bail, Context};
use icelines_core::{model::Season, DepthChartBuilder, TeamAbbr};
use icelines_fetch::{
    player_builder::{build_players, index_bios, index_stats},
    schema::{RosterResponse, SkaterBio, SkaterStats},
    snapshot::{SnapshotStore, SnapshotTier},
};

pub async fn run(team: String, _scheme: Option<String>, no_color: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());

    let team_abbr = TeamAbbr::parse(&team)
        .with_context(|| format!("'{team}' is not a valid NHL team abbreviation"))?;

    // Read roster from snapshot chain (may be in a Rosters parent snapshot)
    let roster: RosterResponse = store
        .read_tier(
            &SnapshotTier::Rosters,
            &format!("{}.json", team_abbr.as_str()),
        )
        .with_context(|| {
            format!(
                "no roster for {} — run `icelines fetch rosters` first",
                team_abbr
            )
        })?;

    // Read stats from snapshot chain
    let bios: Vec<SkaterBio> = store
        .read_tier(&SnapshotTier::Stats, "bios.json")
        .with_context(|| "no stats found — run `icelines fetch stats` first")?;
    let stats: Vec<SkaterStats> = store
        .read_tier(&SnapshotTier::Stats, "stats.json")
        .unwrap_or_default();

    let season = cfg.season_str();

    let bio_idx = index_bios(&bios);
    let stats_idx = index_stats(&stats);

    let season_u32: u32 = season.parse().unwrap_or(20252026);

    // Build players for just the forwards and defensemen
    let fwd_players = build_players(
        &roster.forwards,
        &bio_idx,
        &stats_idx,
        Season(season_u32),
        &team_abbr,
    );
    let def_players = build_players(
        &roster.defensemen,
        &bio_idx,
        &stats_idx,
        Season(season_u32),
        &team_abbr,
    );

    let all_players: Vec<_> = fwd_players.into_iter().chain(def_players).collect();

    if all_players.is_empty() {
        bail!("no skaters found for {} in cache", team_abbr);
    }

    let chart = DepthChartBuilder::build(team_abbr, Season(season_u32), all_players);
    render_team_card(&chart, no_color);
    Ok(())
}
