use crate::config::Config;
use crate::render::terminal::render_team_card;
use anyhow::bail;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::{DepthChartBuilder, TeamAbbr};
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::load_into_repo;

pub async fn run(team: String, _scheme: Option<String>, no_color: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let team_abbr = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{team}' is not a valid NHL team abbreviation"))?;

    // Hart.5b2g: load via load_into_repo, filter to team via repo's
    // built-in roster index (last-stint), build depth chart from views.
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(Season(season_u32), SeasonType::Regular, &store)
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))?;

    // team_roster includes goalies; DepthChartBuilder expects skaters
    // only (goalies fall to its `unplaced` bucket otherwise).
    let team_views: Vec<_> = outcome
        .repo
        .team_roster(&team_abbr, Season(season_u32), SeasonType::Regular)
        .into_iter()
        .filter(|v| !v.is_goalie())
        .collect();
    if team_views.is_empty() {
        bail!("no skaters found for {} in data", team_abbr);
    }

    let chart = DepthChartBuilder::build_views(team_abbr, Season(season_u32), &team_views);
    render_team_card(&chart, no_color);
    Ok(())
}
