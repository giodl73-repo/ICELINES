use crate::config::Config;
use crate::render::terminal::render_team_depth_view;
use anyhow::bail;
use icelines_core::{model::Season, season_stats::SeasonType, TeamAbbr, TeamDepthView};
use icelines_fetch::{snapshot::SnapshotStore, stats_loader::load_into_repo};

pub async fn run(team: String, _scheme: Option<String>, no_color: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let team_abbr = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{team}' is not a valid NHL team abbreviation"))?;

    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    let season = Season(season_u32);

    // Hart.5c.1: load directly into a StatsRepository, take the team's
    // roster as PlayerView slice, build the depth chart from views.
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(season, SeasonType::Regular, &store)
        .map_err(|e| anyhow::anyhow!("loading repo: {e}"))?;
    let view = TeamDepthView::from_repository(
        &outcome.repo,
        team_abbr.clone(),
        season,
        SeasonType::Regular,
    );
    if view.is_empty() {
        bail!("no skaters found for {} in data", team_abbr);
    }

    render_team_depth_view(&view, no_color);
    Ok(())
}
