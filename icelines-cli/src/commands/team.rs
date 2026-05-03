use crate::config::Config;
use crate::render::terminal::render_team_card;
use anyhow::bail;
use icelines_core::{model::Season, season_stats::SeasonType, DepthChartBuilder, TeamAbbr};
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
    let views = outcome
        .repo
        .team_roster(&team_abbr, season, SeasonType::Regular);

    if views.is_empty() {
        bail!("no skaters found for {} in data", team_abbr);
    }

    let chart = DepthChartBuilder::build_views(team_abbr, season, &views);
    render_team_card(&chart, no_color);
    Ok(())
}
