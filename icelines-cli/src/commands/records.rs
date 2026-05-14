use anyhow::{bail, Context};
use icelines_core::{
    model::Season, season_stats::SeasonType, PlayerRecordsView, TeamAbbr, TeamRecordsView,
    ViewContext, ViewWindow, CURRENT_SEASON,
};
use icelines_fetch::records_provider::{
    load_fight_record_inputs_from_default_store, load_goal_record_inputs_from_default_store,
    load_play_by_play_goal_record_inputs_from_default_store,
};

use crate::cli::RecordsMetric;
use crate::commands::output::Format;

pub async fn run_player(
    player: String,
    metric: RecordsMetric,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let player_id = icelines_fetch::stats_loader::resolve_player_id_by_name(&player)
        .with_context(|| format!("could not resolve player `{player}` from bundled bios"))?;
    let view = match metric {
        RecordsMetric::TeamsScoredAgainst => {
            let goals = load_goal_record_inputs_from_default_store()?;
            PlayerRecordsView::teams_scored_against(context(), player_id, player, &goals)
        }
        RecordsMetric::GoaliesScoredAgainst => {
            let goals = load_play_by_play_goal_record_inputs_from_default_store()?;
            PlayerRecordsView::goalies_scored_against(context(), player_id, player, &goals)
        }
        RecordsMetric::FightOpponents => {
            let fights = load_fight_record_inputs_from_default_store()?;
            PlayerRecordsView::fight_opponents(context(), player_id, player, &fights)
        }
        RecordsMetric::PlayersScoredAgainstTeam
        | RecordsMetric::GoaliesBeatenByTeam
        | RecordsMetric::FightOpponentsByTeam => {
            bail!(
                "player records do not support team-only metric `{}`",
                metric.as_str()
            )
        }
    };
    emit_player_view(&view, json, csv, out.as_deref())
}

pub async fn run_team(
    team: String,
    metric: RecordsMetric,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let team = TeamAbbr::parse(&team)
        .map_err(|_| anyhow::anyhow!("'{}' is not a valid NHL team abbreviation", team))?;
    let view = match metric {
        RecordsMetric::PlayersScoredAgainstTeam => {
            let goals = load_goal_record_inputs_from_default_store()?;
            TeamRecordsView::players_scored_against_team(context(), team.0, &goals)
        }
        RecordsMetric::GoaliesBeatenByTeam => {
            let goals = load_play_by_play_goal_record_inputs_from_default_store()?;
            TeamRecordsView::goalies_beaten_by_team(context(), team.0, &goals)
        }
        RecordsMetric::FightOpponentsByTeam => {
            let fights = load_fight_record_inputs_from_default_store()?;
            TeamRecordsView::fight_opponents_by_team(context(), team.0, &fights)
        }
        RecordsMetric::TeamsScoredAgainst
        | RecordsMetric::GoaliesScoredAgainst
        | RecordsMetric::FightOpponents => {
            bail!(
                "team records do not support player-only metric `{}`",
                metric.as_str()
            )
        }
    };
    emit_team_view(&view, json, csv, out.as_deref())
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
        view.subject_key_header(),
        view.subject_label_header(),
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
        view.subject_key_header(),
        view.subject_label_header(),
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
        let source_hint = if headers.iter().any(|header| header.contains("goalie")) {
            "play-by-play files. Run `icelines fetch play-by-play --date YYYY-MM-DD`"
        } else {
            "boxscores. Run `icelines fetch boxscore --date YYYY-MM-DD`"
        };
        eprintln!("No matching records found. Run {source_hint} to populate persisted records.");
    }
    if incomplete_goal_rows > 0 && out.is_none() {
        eprintln!("{incomplete_goal_rows} goal rows lacked required ids and were excluded.");
    }
    Ok(())
}

trait RecordsMetricHeaders {
    fn subject_key_header(&self) -> &'static str;
    fn subject_label_header(&self) -> &'static str;
}

impl RecordsMetricHeaders for PlayerRecordsView {
    fn subject_key_header(&self) -> &'static str {
        match self.metric.as_str() {
            "goalies-scored-against" => "goalie_id",
            "fight-opponents" => "opponent_id",
            _ => "opponent_team",
        }
    }

    fn subject_label_header(&self) -> &'static str {
        match self.metric.as_str() {
            "goalies-scored-against" => "goalie",
            "fight-opponents" => "opponent",
            _ => "opponent_team",
        }
    }
}

impl RecordsMetricHeaders for TeamRecordsView {
    fn subject_key_header(&self) -> &'static str {
        match self.metric.as_str() {
            "goalies-beaten-by-team" => "goalie_id",
            "fight-opponents-by-team" => "opponent_id",
            _ => "player_id",
        }
    }

    fn subject_label_header(&self) -> &'static str {
        match self.metric.as_str() {
            "goalies-beaten-by-team" => "goalie",
            "fight-opponents-by-team" => "opponent",
            _ => "player",
        }
    }
}

impl RecordsMetric {
    fn as_str(self) -> &'static str {
        match self {
            Self::TeamsScoredAgainst => "teams-scored-against",
            Self::GoaliesScoredAgainst => "goalies-scored-against",
            Self::FightOpponents => "fight-opponents",
            Self::PlayersScoredAgainstTeam => "players-scored-against-team",
            Self::GoaliesBeatenByTeam => "goalies-beaten-by-team",
            Self::FightOpponentsByTeam => "fight-opponents-by-team",
        }
    }
}
