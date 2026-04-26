use crate::config::Config;
use crate::render::terminal::render_rank_table;
use anyhow::Context;
use icelines_core::{
    model::Season, position::PositionResolver, scoring::sort_by_pace, Position, TeamAbbr,
};
use icelines_fetch::{
    cache::{ttl, Cache},
    player_builder::{build_players, index_bios, index_stats},
    schema::{RosterResponse, SkaterBio, SkaterStats},
};

const ALL_TEAMS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

pub async fn run(top: usize, pos: Option<String>, _scheme: Option<String>) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let cache = Cache::new(&cfg.cache_dir);
    let season = cfg.season_str();

    // Parse optional position filter
    let pos_filter: Option<Position> = pos
        .as_deref()
        .and_then(|p| PositionResolver::parse(p).ok().map(|(primary, _)| primary));

    // Load global stats (bios + summary)
    let bios: Vec<SkaterBio> = cache
        .get(&format!("stats/{season}/bios.json"), ttl::STATS)
        .with_context(|| "no cached stats — run `icelines fetch stats` first")?;
    let stats: Vec<SkaterStats> = cache
        .get(&format!("stats/{season}/stats.json"), ttl::STATS)
        .unwrap_or_default();

    let bio_idx = index_bios(&bios);
    let stats_idx = index_stats(&stats);
    let season_u32: u32 = season.parse().unwrap_or(20252026);

    // Collect all skaters across all 32 teams
    let mut all_players = Vec::new();
    for team_str in ALL_TEAMS {
        let roster_key = format!("rosters/{season}/{team_str}.json");
        let roster: Option<RosterResponse> = cache.get(&roster_key, ttl::ROSTER);
        if let Some(r) = roster {
            let team = TeamAbbr(team_str.to_string());
            let fwds = build_players(&r.forwards, &bio_idx, &stats_idx, Season(season_u32), &team);
            let defs = build_players(
                &r.defensemen,
                &bio_idx,
                &stats_idx,
                Season(season_u32),
                &team,
            );
            all_players.extend(fwds);
            all_players.extend(defs);
        }
    }

    sort_by_pace(&mut all_players);

    // Filter by position if requested
    if let Some(p) = pos_filter {
        all_players.retain(|player| player.position == p);
    }

    // Only show rankable (pace_score is Some)
    all_players.retain(|p| p.is_rankable());

    render_rank_table(&all_players, top, pos_filter, false);
    Ok(())
}
