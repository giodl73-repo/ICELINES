use crate::commands::players::load_all_players;
use crate::config::Config;
use crate::render::terminal::render_team_card;
use anyhow::bail;
use icelines_core::{model::Season, DepthChartBuilder, TeamAbbr};

pub async fn run(team: String, _scheme: Option<String>, no_color: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let team_abbr = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{team}' is not a valid NHL team abbreviation"))?;

    // Hart.5a: filter the centralized full-league load by team abbrev.
    // Legacy `repo.load_team()` had a snapshot-aware fast path; the
    // full-league load is what every other consumer uses, and the
    // bundled-only path collapses to the same result. Hart.5b will
    // refactor this to `repo.team_roster(team, season, type)` directly.
    let all_players = load_all_players()?;
    let players: Vec<_> = all_players
        .into_iter()
        .filter(|p| p.team == team_abbr)
        .collect();

    if players.is_empty() {
        bail!("no skaters found for {} in data", team_abbr);
    }

    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    let chart = DepthChartBuilder::build(team_abbr, Season(season_u32), players);
    render_team_card(&chart, no_color);
    Ok(())
}
