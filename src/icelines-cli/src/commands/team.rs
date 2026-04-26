use crate::config::Config;
use crate::render::terminal::render_team_card;
use anyhow::{bail, Context};
use icelines_core::{model::Season, DepthChartBuilder, TeamAbbr};
use icelines_fetch::{
    cache::{ttl, Cache},
    player_builder::{build_players, index_bios, index_stats},
    schema::{RosterResponse, SkaterBio, SkaterStats},
};

pub async fn run(team: String, _scheme: Option<String>, no_color: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let cache = Cache::new(&cfg.cache_dir);
    let season = cfg.season_str();

    let team_abbr = TeamAbbr::parse(&team)
        .with_context(|| format!("'{team}' is not a valid NHL team abbreviation"))?;

    // Load roster from cache
    let roster_key = format!("rosters/{season}/{}.json", team_abbr.as_str());
    let roster: RosterResponse = cache.get(&roster_key, ttl::ROSTER).with_context(|| {
        format!(
            "no cached roster for {} — run `icelines fetch rosters` first",
            team_abbr
        )
    })?;

    // Load stats from cache
    let bios: Vec<SkaterBio> = cache
        .get(&format!("stats/{season}/bios.json"), ttl::STATS)
        .with_context(|| "no cached stats — run `icelines fetch stats` first")?;
    let stats: Vec<SkaterStats> = cache
        .get(&format!("stats/{season}/stats.json"), ttl::STATS)
        .unwrap_or_default();

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
