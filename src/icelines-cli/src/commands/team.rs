use crate::config::Config;
use crate::render::terminal::render_team_card;
use anyhow::bail;
use icelines_core::{model::Season, DepthChartBuilder, TeamAbbr};
use icelines_fetch::{snapshot::SnapshotStore, PlayerRepository};

pub async fn run(team: String, _scheme: Option<String>, no_color: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let team_abbr = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{team}' is not a valid NHL team abbreviation"))?;

    let repo = PlayerRepository::new(
        SnapshotStore::new(cfg.snapshot_dir()),
        cfg.season_str(),
    );

    let players = repo.load_team(team_abbr.as_str())
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))?;

    if players.is_empty() {
        bail!("no skaters found for {} in data", team_abbr);
    }

    let season_u32: u32 = cfg.season_str().parse().unwrap_or(icelines_core::CURRENT_SEASON);
    let chart = DepthChartBuilder::build(team_abbr, Season(season_u32), players);
    render_team_card(&chart, no_color);
    Ok(())
}
