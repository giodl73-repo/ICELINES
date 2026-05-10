use crate::state::WebState;
use crate::templates::{GoalieRow, LeaderRow, TeamTemplate};
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::{
    DepthGoalieSlot, DepthLine, DepthPair, DepthPlayerSlot, TeamDepthView,
};
use icelines_core::{MetricCell, MetricValue};

pub async fn get_team(State(state): State<WebState>, Path(abbrev_raw): Path<String>) -> Response {
    let team = match parse_team(&abbrev_raw) {
        Ok(team) => team,
        Err((_abbrev_upper, message)) => {
            return (
                StatusCode::NOT_FOUND,
                Html(format!(
                    "<!doctype html><html><body><h1>Unknown team</h1>\
                     <p>{message}</p>\
                     <p><a href=\"/leaders\">back to leaders</a></p>\
                     </body></html>"
                )),
            )
                .into_response();
        }
    };

    let (season_str, season_type, active_label) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st, cfg.active_label.clone())
    };
    let season = match parse_season(&season_str) {
        Ok(season) => season,
        Err(()) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(
                    "<!doctype html><html><body><h1>500</h1>\
                     <p>Active season '{season_str}' is not a YYYYZZZZ id</p>\
                     </body></html>"
                )),
            )
                .into_response();
        }
    };

    let (skaters, goalies) = {
        let repo = state.repo.read().await;
        let view = TeamDepthView::from_repository(&repo, team.clone(), season, season_type);

        let mut skaters: Vec<LeaderRow> = skater_slots(&view)
            .into_iter()
            .map(|slot| leader_row_from_depth_slot(slot, season))
            .collect();
        skaters.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goals.cmp(&a.goals))
                .then(a.name.cmp(&b.name))
        });

        let mut goalies: Vec<GoalieRow> = view
            .goalies
            .iter()
            .map(|slot| goalie_row_from_depth_slot(slot, season))
            .collect();
        goalies.sort_by(|a, b| b.wins.cmp(&a.wins).then(a.name.cmp(&b.name)));

        (skaters, goalies)
    };

    let tmpl = TeamTemplate {
        active_label,
        team_abbrev: team.0.to_string(),
        skaters,
        goalies,
    };
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {e}")),
        )
            .into_response(),
    }
}

#[derive(Debug, serde::Serialize)]
pub struct TeamData {
    pub team_abbrev: String,
    pub skaters: Vec<TeamSkaterRow>,
    pub goalies: Vec<TeamGoalieRow>,
}

#[derive(Debug, serde::Serialize)]
pub struct TeamSkaterRow {
    pub nhl_id: u32,
    pub name: String,
    pub position: String,
    pub games: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct TeamGoalieRow {
    pub nhl_id: u32,
    pub name: String,
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub shutouts: u32,
    pub save_pct: Option<f64>,
    pub goals_against_average: Option<f64>,
}

#[derive(Debug, serde::Serialize)]
pub struct TeamMeta {
    pub team_abbrev: String,
    pub season: String,
    pub season_type: String,
    pub skater_count: usize,
    pub goalie_count: usize,
}

pub async fn get_team_json(
    State(state): State<WebState>,
    Path(abbrev_raw): Path<String>,
) -> Response {
    let team = match parse_team(&abbrev_raw) {
        Ok(team) => team,
        Err((abbrev_upper, message)) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": "unknown_team",
                    "message": message,
                    "team_abbrev": abbrev_upper,
                })),
            )
                .into_response();
        }
    };

    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        let st = SeasonType::parse_lossy(&cfg.active_season_type);
        (cfg.active_season.clone(), st)
    };
    let season = match parse_season(&season_str) {
        Ok(season) => season,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "bad_active_season",
                    "message": format!("Season '{season_str}' is not a valid YYYYZZZZ id"),
                })),
            )
                .into_response();
        }
    };

    let (skaters, goalies) = {
        let repo = state.repo.read().await;
        let view = TeamDepthView::from_repository(&repo, team.clone(), season, season_type);

        let mut skaters: Vec<TeamSkaterRow> = skater_slots(&view)
            .into_iter()
            .map(team_skater_row_from_depth_slot)
            .collect();
        skaters.sort_by(|a, b| {
            b.points
                .cmp(&a.points)
                .then(b.goals.cmp(&a.goals))
                .then(a.name.cmp(&b.name))
        });

        let mut goalies: Vec<TeamGoalieRow> = view
            .goalies
            .iter()
            .map(team_goalie_row_from_depth_slot)
            .collect();
        goalies.sort_by(|a, b| b.wins.cmp(&a.wins).then(a.name.cmp(&b.name)));

        (skaters, goalies)
    };

    let meta = TeamMeta {
        team_abbrev: team.0.to_string(),
        season: season_str,
        season_type: match season_type {
            SeasonType::Regular => "regular".to_owned(),
            SeasonType::Playoff => "playoff".to_owned(),
        },
        skater_count: skaters.len(),
        goalie_count: goalies.len(),
    };
    let data = TeamData {
        team_abbrev: team.0.to_string(),
        skaters,
        goalies,
    };
    crate::api::json_data_meta("team", data, meta)
}

fn parse_team(abbrev_raw: &str) -> Result<TeamAbbr, (String, String)> {
    let abbrev_upper = abbrev_raw.to_ascii_uppercase();
    TeamAbbr::parse(&abbrev_upper).map_err(|e| {
        let message = format!("'{abbrev_upper}' is not a recognized NHL team abbrev: {e}");
        (abbrev_upper, message)
    })
}

fn parse_season(season_str: &str) -> Result<Season, ()> {
    season_str.parse::<u32>().map(Season).map_err(|_| ())
}

fn skater_slots(view: &TeamDepthView) -> Vec<&DepthPlayerSlot> {
    let mut slots = Vec::new();
    for line in &view.forward_lines {
        push_line_slots(&mut slots, line);
    }
    for pair in &view.defense_pairs {
        push_pair_slots(&mut slots, pair);
    }
    slots.extend(view.extras.iter());
    slots
}

fn push_line_slots<'a>(out: &mut Vec<&'a DepthPlayerSlot>, line: &'a DepthLine) {
    out.extend(
        [
            line.left.as_ref(),
            line.center.as_ref(),
            line.right.as_ref(),
        ]
        .into_iter()
        .flatten(),
    );
}

fn push_pair_slots<'a>(out: &mut Vec<&'a DepthPlayerSlot>, pair: &'a DepthPair) {
    out.extend(
        [pair.left.as_ref(), pair.right.as_ref()]
            .into_iter()
            .flatten(),
    );
}

fn leader_row_from_depth_slot(slot: &DepthPlayerSlot, season: Season) -> LeaderRow {
    let gp = metric_u32(&slot.metrics, "gp").unwrap_or(0);
    let goals = metric_u32(&slot.metrics, "goals").unwrap_or(0);
    let assists = metric_u32(&slot.metrics, "assists").unwrap_or(0);
    let points = metric_u32(&slot.metrics, "points").unwrap_or(0);
    let team = slot.team.0.clone();
    let ppg_str = if gp > 0 {
        format!("{:.2}", points as f64 / gp as f64)
    } else {
        String::new()
    };

    LeaderRow {
        nhl_id: slot.player_id.0,
        name: slot.display_name.clone(),
        position: slot.position.abbreviation().to_owned(),
        team: team.clone(),
        gp,
        goals,
        assists,
        points,
        ppg_str,
        headshot_url: super::shared::build_headshot_url_for_display(
            season.0,
            &team,
            slot.player_id.0,
        ),
        headshot_fallback_url: format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            slot.player_id.0
        ),
        plus_minus_str: "-".to_owned(),
        pim: 0,
        shots: 0,
        shooting_pct_str: "-".to_owned(),
        hits_str: "-".to_owned(),
        blocks_str: "-".to_owned(),
        faceoff_pct_str: "-".to_owned(),
        pp_points: 0,
        plus_minus: 0,
        shooting_pct: None,
        hits: None,
        blocks: None,
        faceoff_pct: None,
        points_per_60_str: "-".to_owned(),
        goals_per_60_str: "-".to_owned(),
        assists_per_60_str: "-".to_owned(),
        hits_per_60_str: "-".to_owned(),
        blocks_per_60_str: "-".to_owned(),
        points_per_60: None,
        goals_per_60: None,
        assists_per_60: None,
        hits_per_60: None,
        blocks_per_60: None,
        points_delta: None,
        points_delta_str: String::new(),
        points_delta_class: String::new(),
    }
}

fn goalie_row_from_depth_slot(slot: &DepthGoalieSlot, season: Season) -> GoalieRow {
    let save_pct_str = metric_f64(&slot.metrics, "save_pct")
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "-".to_owned());
    let gaa_str = metric_f64(&slot.metrics, "gaa")
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "-".to_owned());
    let team = slot.team.0.clone();

    GoalieRow {
        nhl_id: slot.player_id.0,
        name: slot.display_name.clone(),
        team: team.clone(),
        gp: metric_u32(&slot.metrics, "gp").unwrap_or(0),
        wins: metric_u32(&slot.metrics, "wins").unwrap_or(0),
        losses: metric_u32(&slot.metrics, "losses").unwrap_or(0),
        shutouts: metric_u32(&slot.metrics, "shutouts").unwrap_or(0),
        save_pct_str,
        gaa_str,
        headshot_url: super::shared::build_headshot_url_for_display(
            season.0,
            &team,
            slot.player_id.0,
        ),
        headshot_fallback_url: format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            slot.player_id.0
        ),
    }
}

fn team_skater_row_from_depth_slot(slot: &DepthPlayerSlot) -> TeamSkaterRow {
    let games = metric_u32(&slot.metrics, "gp").unwrap_or(0);
    let goals = metric_u32(&slot.metrics, "goals").unwrap_or(0);
    let assists = metric_u32(&slot.metrics, "assists").unwrap_or(0);
    let points = metric_u32(&slot.metrics, "points").unwrap_or(0);
    let points_per_game = if games > 0 {
        Some(points as f64 / games as f64)
    } else {
        None
    };

    TeamSkaterRow {
        nhl_id: slot.player_id.0,
        name: slot.display_name.clone(),
        position: slot.position.abbreviation().to_owned(),
        games,
        goals,
        assists,
        points,
        points_per_game,
    }
}

fn team_goalie_row_from_depth_slot(slot: &DepthGoalieSlot) -> TeamGoalieRow {
    TeamGoalieRow {
        nhl_id: slot.player_id.0,
        name: slot.display_name.clone(),
        games: metric_u32(&slot.metrics, "gp").unwrap_or(0),
        wins: metric_u32(&slot.metrics, "wins").unwrap_or(0),
        losses: metric_u32(&slot.metrics, "losses").unwrap_or(0),
        shutouts: metric_u32(&slot.metrics, "shutouts").unwrap_or(0),
        save_pct: metric_f64(&slot.metrics, "save_pct"),
        goals_against_average: metric_f64(&slot.metrics, "gaa"),
    }
}

fn metric_u32(metrics: &[MetricCell], key: &str) -> Option<u32> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Integer(value) => u32::try_from(value).ok(),
            _ => None,
        })
}

fn metric_f64(metrics: &[MetricCell], key: &str) -> Option<f64> {
    metrics
        .iter()
        .find(|metric| metric.key.0 == key)
        .and_then(|metric| match metric.value {
            MetricValue::Decimal(value) => Some(value),
            _ => None,
        })
}
