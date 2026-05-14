use anyhow::{bail, Context};
use icelines_core::{
    model::Season, season_stats::SeasonType, PlayerRecordsView, TeamAbbr, TeamRecordsView,
    ViewContext, ViewWindow, CURRENT_SEASON,
};
use icelines_fetch::{datastore::DataStore, records_provider::load_goal_record_inputs};

use crate::cli::RecordsMetric;
use crate::commands::output::Format;

pub async fn run_player(
    player: String,
    metric: RecordsMetric,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    if metric != RecordsMetric::TeamsScoredAgainst {
        bail!("player records currently support --metric teams-scored-against");
    }
    let player_id = icelines_fetch::stats_loader::resolve_player_id_by_name(&player)
        .with_context(|| format!("could not resolve player `{player}` from bundled bios"))?;
    let store = open_store()?;
    let goals = load_goal_record_inputs(&store)?;
    let view = PlayerRecordsView::teams_scored_against(context(), player_id, player, &goals);
    emit_player_view(&view, json, csv, out.as_deref())
}

pub async fn run_team(
    team: String,
    metric: RecordsMetric,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    if metric != RecordsMetric::PlayersScoredAgainstTeam {
        bail!("team records currently support --metric players-scored-against-team");
    }
    let team = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{}' is not a valid NHL team abbreviation", team))?;
    let store = open_store()?;
    let goals = load_goal_record_inputs(&store)?;
    let view = TeamRecordsView::players_scored_against_team(context(), team.0, &goals);
    emit_team_view(&view, json, csv, out.as_deref())
}

fn open_store() -> anyhow::Result<DataStore> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let data_root = home.join(".icelines").join("data");
    DataStore::open(&data_root).context("open DataStore")
}

fn context() -> ViewContext {
    ViewContext::new(ViewWindow::new(Season(CURRENT_SEASON), SeasonType::Regular))
}

fn emit_player_view(
    view: &PlayerRecordsView,
    json: bool,
    csv: bool,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    let headers = [
        "player_id",
        "player",
        "metric",
        "opponent_team",
        "count",
        "first_game_id",
        "first_date",
        "last_game_id",
        "last_date",
    ];
    let rows = view
        .rows
        .iter()
        .map(|row| {
            vec![
                view.player_id.to_string(),
                view.player_name.clone(),
                view.metric.clone(),
                row.label.clone(),
                row.count.to_string(),
                row.first_game_id.to_string(),
                row.first_date.clone().unwrap_or_default(),
                row.last_game_id.to_string(),
                row.last_date.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    emit_records_rows(&headers, &rows, view.incomplete_goal_rows, json, csv, out)
}

fn emit_team_view(
    view: &TeamRecordsView,
    json: bool,
    csv: bool,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    let headers = [
        "team",
        "metric",
        "player_id",
        "player",
        "count",
        "first_game_id",
        "first_date",
        "last_game_id",
        "last_date",
    ];
    let rows = view
        .rows
        .iter()
        .map(|row| {
            vec![
                view.team.clone(),
                view.metric.clone(),
                row.key.clone(),
                row.label.clone(),
                row.count.to_string(),
                row.first_game_id.to_string(),
                row.first_date.clone().unwrap_or_default(),
                row.last_game_id.to_string(),
                row.last_date.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    emit_records_rows(&headers, &rows, view.incomplete_goal_rows, json, csv, out)
}

fn emit_records_rows(
    headers: &[&str],
    rows: &[Vec<String>],
    incomplete_goal_rows: u32,
    json: bool,
    csv: bool,
    out: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    let format = Format::resolve(csv, json)?;
    format.emit_to(headers, rows, out)?;
    if rows.is_empty() && out.is_none() && !json && !csv {
        eprintln!(
            "No matching records found. Run `icelines fetch boxscore --date YYYY-MM-DD` to populate persisted boxscores."
        );
    }
    if incomplete_goal_rows > 0 && out.is_none() {
        eprintln!("{incomplete_goal_rows} goal rows lacked scorer ids and were excluded.");
    }
    Ok(())
}
