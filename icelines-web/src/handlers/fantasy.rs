use crate::state::WebState;
use crate::templates::{
    FantasyGapRow, FantasySimulationRow, FantasySimulationScenarioRow, FantasyTemplate,
};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::timeframe::Timeframe;
use icelines_core::view_model::{
    Completeness, FantasyDailyDeltaInput, FantasyDailyPlayerInput, FantasyDailyTeamInput,
    FantasyMatchupScheduleInput, FantasyMatchupTeamTotalInput, FantasyMatchupWeekInput, SourceKind,
    SourceState,
};
use icelines_core::{
    build_fantasy_simulation_view, resolve_fantasy_scenario_roster_details, FantasyReadinessView,
    FantasyReadinessWorkflow, FantasyRosterGapInput, FantasyRosterGapView,
    FantasySimulationBuildInput, FantasySimulationConfidence, FantasySimulationHorizon,
    FantasySimulationRosterTeamInput, FantasySimulationScenarioRosterInput, FantasySimulationView,
    FantasyTodayState, FantasyTodayV2View, Scheme,
};
use icelines_fetch::datastore::DataStore;
use icelines_fetch::fantasy_daily::build_fantasy_daily_delta_view;
use icelines_fetch::fantasy_db::FantasyDb;
use icelines_fetch::fantasy_decision_review_service::assemble_fantasy_decision_review;
use icelines_fetch::fantasy_matchup::build_fantasy_matchup_week_view;
use icelines_fetch::fantasy_readiness_service::{
    assemble_fantasy_readiness, FantasyReadinessAssemblyRequest,
};
use icelines_fetch::fantasy_today_service::{assemble_fantasy_today, FantasyTodayAssemblyRequest};
use icelines_fetch::schedule_remaining::remaining_games_by_team_from_cache;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Default)]
pub struct FantasyWebQuery {
    #[serde(default)]
    pub league: Option<String>,
    #[serde(default)]
    pub scheme: Option<String>,
    #[serde(default, rename = "category")]
    pub categories: Option<String>,
    #[serde(default)]
    pub top: Option<usize>,
    #[serde(default)]
    pub weeks: Option<u8>,
    #[serde(default)]
    pub add_player: Option<String>,
    #[serde(default)]
    pub drop_player: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub week: Option<String>,
    #[serde(default)]
    pub season: Option<String>,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub workflow: Option<String>,
    #[serde(default)]
    pub stats_season: Option<String>,
}

pub async fn get_fantasy(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    let active_label = state.config.read().await.active_label.clone();
    let result = match build_fantasy_gaps(&state, &q).await {
        Ok(result) => result,
        Err(message) => {
            let tmpl = FantasyTemplate {
                active_label,
                league: q.league.unwrap_or_default(),
                team: String::new(),
                scoring_scheme: q.scheme.unwrap_or_else(|| "yahoo-standard".to_string()),
                categories: q.categories.unwrap_or_default(),
                add_player: q.add_player.unwrap_or_default(),
                drop_player: q.drop_player.unwrap_or_default(),
                rows: Vec::new(),
                simulation_rows: Vec::new(),
                simulation_scenarios: Vec::new(),
                simulation_assumptions: Vec::new(),
                simulation_warnings: Vec::new(),
                warnings: Vec::new(),
                empty_title: "Fantasy roster gaps unavailable".to_string(),
                empty_detail: message,
            };
            return render_template(tmpl);
        }
    };

    let simulation = build_fantasy_simulation(&state, &q).await;
    let simulation_error = simulation.as_ref().err().cloned();
    render_template(project_template(
        active_label,
        result,
        simulation.ok(),
        simulation_error,
        q.categories.unwrap_or_default(),
        q.add_player.unwrap_or_default(),
        q.drop_player.unwrap_or_default(),
    ))
}

pub async fn get_fantasy_gaps_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match build_fantasy_gaps(&state, &q).await {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": message })),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_simulation_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match build_fantasy_simulation(&state, &q).await {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": message })),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_daily_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match build_fantasy_daily(&state, &q).await {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": message })),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_matchup_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match build_fantasy_matchup(&state, &q).await {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": message })),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_roster_shape_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match build_fantasy_roster_shape(&state, &q).await {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "error": message })),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_today_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match load_fantasy_today_contract(&state, &q).await {
        Ok(view) => axum::Json(view.v1_projection()).into_response(),
        Err(message) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "schema": "fantasy_today.v1",
                "state": "blocked",
                "error": message,
                "recovery_command": "icelines fantasy today"
            })),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_today_v2_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match load_fantasy_today_contract(&state, &q).await {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "schema": "fantasy_today.v2",
                "state": "blocked",
                "error": message,
                "recovery_command": "icelines fantasy today"
            })),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_readiness_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    no_store(match load_fantasy_readiness_contract(&state, &q).await {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "schema": icelines_core::FANTASY_READINESS_SCHEMA,
                "state": "invalid_request",
                "error": message,
                "recovery_command": "icelines fantasy readiness"
            })),
        )
            .into_response(),
    })
}

pub async fn get_fantasy_readiness(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    no_store(match load_fantasy_readiness_contract(&state, &q).await {
        Ok(view) => Html(render_fantasy_readiness_html(&view)).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            Html(format!(
                "<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><main><h1>Fantasy readiness unavailable</h1><p role=\"alert\">{}</p><p>Run <code>icelines fantasy readiness</code>.</p></main></html>",
                escape_html(&message)
            )),
        )
            .into_response(),
    })
}

async fn load_fantasy_readiness_contract(
    state: &WebState,
    q: &FantasyWebQuery,
) -> Result<FantasyReadinessView, String> {
    let stats_season = match q.stats_season.as_deref() {
        Some(value) if value.len() == 8 && value.chars().all(|ch| ch.is_ascii_digit()) => {
            value.to_owned()
        }
        Some(value) => {
            return Err(format!(
                "stats season '{value}' must be an 8-digit NHL season ID (for example 20262027)"
            ));
        }
        None => state.config.read().await.active_season.clone(),
    };
    let workflow = q
        .workflow
        .as_deref()
        .map(parse_readiness_workflow)
        .transpose()?;
    let mut today = FantasyTodayAssemblyRequest::from_default_paths(
        q.league.clone(),
        q.team.clone(),
        stats_season,
        icelines_core::CURRENT_SEASON,
        Utc::now(),
    )
    .map_err(|error| error.to_string())?;
    today.local_date = q
        .date
        .as_deref()
        .map(|value| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|error| format!("invalid date '{value}': {error}"))
        })
        .transpose()?;
    tokio::task::spawn_blocking(move || {
        assemble_fantasy_readiness(FantasyReadinessAssemblyRequest { today, workflow })
    })
    .await
    .map_err(|error| format!("join local readiness assembler: {error}"))?
}

fn parse_readiness_workflow(value: &str) -> Result<FantasyReadinessWorkflow, String> {
    match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "draft" => Ok(FantasyReadinessWorkflow::Draft),
        "today" => Ok(FantasyReadinessWorkflow::Today),
        "matchup" => Ok(FantasyReadinessWorkflow::Matchup),
        "week_plan" => Ok(FantasyReadinessWorkflow::WeekPlan),
        "goalie" => Ok(FantasyReadinessWorkflow::Goalie),
        "trade" => Ok(FantasyReadinessWorkflow::Trade),
        "decision_review" => Ok(FantasyReadinessWorkflow::DecisionReview),
        _ => Err(format!(
            "unknown workflow '{value}'; expected draft, today, matchup, week-plan, goalie, trade, or decision-review"
        )),
    }
}

fn render_fantasy_readiness_html(view: &FantasyReadinessView) -> String {
    let workflows = view
        .workflows
        .iter()
        .map(|workflow| {
            let checks = workflow
                .checks
                .iter()
                .filter(|check| check.state != FantasyTodayState::Ready)
                .map(|check| {
                    format!(
                        "<li><strong>{}</strong> — {:?}: {}{}</li>",
                        escape_html(&check.check_id),
                        check.state,
                        escape_html(&check.message),
                        check.recovery_command.as_ref().map(|command| format!(
                            " <code>{}</code>", escape_html(command)
                        )).unwrap_or_default()
                    )
                })
                .collect::<String>();
            format!(
                "<section><h2>{}</h2><p><strong>{:?}</strong> · {}/{} checks ready</p><ul>{}</ul></section>",
                escape_html(workflow.workflow.as_str()),
                workflow.state,
                workflow.ready_checks,
                workflow.total_checks,
                if checks.is_empty() { "<li>No recovery needed.</li>".to_owned() } else { checks }
            )
        })
        .collect::<String>();
    format!(
        "<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Fantasy data readiness</title><style>body{{font:16px system-ui;max-width:72rem;margin:auto;padding:1rem}}section{{border:1px solid #888;border-radius:.5rem;padding:1rem;margin-block:1rem}}code{{overflow-wrap:anywhere}}</style><main><h1>Fantasy data readiness</h1><p aria-label=\"Overall readiness\"><strong>{:?}</strong> · {} ready · {} provisional · {} blocked</p><p>{} · evaluated <time>{}</time></p>{}<p><small>Snapshot <code>{}</code></small></p></main></html>",
        view.state,
        view.ready_workflows,
        view.provisional_workflows,
        view.blocked_workflows,
        escape_html(&view.stats_season),
        view.evaluated_at.to_rfc3339(),
        workflows,
        escape_html(&view.material_fingerprint)
    )
}

pub async fn get_fantasy_week_plan_json(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    if let Err((message, canonical_week)) = validate_week_shape(&q) {
        return no_store(
            (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "schema": "fantasy_pickup_sequence.v1",
                    "state": "invalid_request",
                    "error": message,
                    "recovery_url": format!("/api/v1/fantasy/week-plan?week={canonical_week}")
                })),
            )
                .into_response(),
        );
    }
    no_store(match load_fantasy_today_contract(&state, &q).await {
        Ok(view) => match validate_requested_week(&q, &view) {
            Ok(()) => axum::Json(view.week_plan).into_response(),
            Err(message) => (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "schema": "fantasy_pickup_sequence.v1",
                    "state": "invalid_request",
                    "error": message,
                    "recovery_url": format!("/api/v1/fantasy/week-plan?week={}", view.today.context.week_start)
                })),
            )
                .into_response(),
        },
        Err(message) => (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "schema": "fantasy_pickup_sequence.v1",
                "state": "blocked",
                "error": message,
                "recovery_command": "icelines fantasy week-plan"
            })),
        )
            .into_response(),
    })
}

pub async fn get_fantasy_week_plan(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    if let Err((message, canonical_week)) = validate_week_shape(&q) {
        return no_store(
            (
                StatusCode::BAD_REQUEST,
                Html(format!(
                    "<!doctype html><html lang=\"en\"><main><h1>Invalid fantasy week</h1><p>{}</p><p><a href=\"/fantasy/week-plan?week={canonical_week}\">Open the containing week</a></p></main></html>",
                    escape_html(&message)
                )),
            )
                .into_response(),
        );
    }
    no_store(match load_fantasy_today_contract(&state, &q).await {
        Ok(view) => match validate_requested_week(&q, &view) {
            Ok(()) => Html(render_fantasy_week_plan_html(
                view.week_plan.as_ref().expect("assembled week plan"),
            ))
            .into_response(),
            Err(message) => (
                StatusCode::BAD_REQUEST,
                Html(format!(
                    "<!doctype html><html lang=\"en\"><main><h1>Invalid fantasy week</h1><p>{}</p><p><a href=\"/fantasy/week-plan?week={}\">Open the active week</a></p></main></html>",
                    escape_html(&message),
                    view.today.context.week_start
                )),
            )
                .into_response(),
        },
        Err(message) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(format!(
                "<!doctype html><html lang=\"en\"><main><h1>Week plan unavailable</h1><p>{}</p><p>Run <code>icelines fantasy week-plan</code>.</p></main></html>",
                escape_html(&message)
            )),
        )
            .into_response(),
    })
}

pub async fn get_fantasy_decision_review_json(
    State(_state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    no_store(match build_fantasy_decision_review_web(&q) {
        Ok(view) => axum::Json(view).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "schema": icelines_core::FANTASY_DECISION_REVIEW_SCHEMA,
                "state": "invalid_request",
                "error": message,
                "recovery_command": "icelines fantasy decision-review"
            })),
        )
            .into_response(),
    })
}

pub async fn get_fantasy_decision_review(
    State(_state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    no_store(match build_fantasy_decision_review_web(&q) {
        Ok(view) => Html(render_fantasy_decision_review_html(&view)).into_response(),
        Err(message) => (
            StatusCode::BAD_REQUEST,
            Html(format!(
                "<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><main><h1>Decision review unavailable</h1><p>{}</p><p>Run <code>icelines fantasy decision-review</code>.</p></main></html>",
                escape_html(&message)
            )),
        )
            .into_response(),
    })
}

fn build_fantasy_decision_review_web(
    q: &FantasyWebQuery,
) -> Result<icelines_core::FantasyDecisionReviewView, String> {
    if q.week.is_some() && q.season.is_some() {
        return Err("week and season filters cannot be combined".to_owned());
    }
    let week = q
        .week
        .as_deref()
        .map(|value| {
            let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|_| format!("week '{value}' must use YYYY-MM-DD"))?;
            if date.weekday() != chrono::Weekday::Mon {
                return Err(format!("week '{value}' must be a Monday"));
            }
            Ok(date)
        })
        .transpose()?;
    let db = open_existing_fantasy_db()?;
    let league = if let Some(name) = q.league.as_deref() {
        db.list_leagues()
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|league| league.name == name)
            .ok_or_else(|| format!("fantasy league '{name}' not found"))?
    } else {
        db.get_active_league()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "no active fantasy league found".to_owned())?
    };
    assemble_fantasy_decision_review(
        &db,
        &league,
        q.top.unwrap_or(20).clamp(1, 500),
        week,
        q.season.clone(),
        false,
    )
    .map_err(|error| error.to_string())
}

fn render_fantasy_decision_review_html(view: &icelines_core::FantasyDecisionReviewView) -> String {
    let filter = view
        .week
        .map(|week| format!("Week {week}"))
        .or_else(|| {
            view.season
                .as_ref()
                .map(|season| format!("Season {season}"))
        })
        .unwrap_or_else(|| "Latest decisions".to_owned());
    let items = if view.items.is_empty() {
        "<article><h2>No decisions to review</h2><p>Run <code>icelines fantasy decision-record</code> after creating a week plan.</p></article>".to_owned()
    } else {
        view.items
            .iter()
            .map(|row| {
                let error = row
                    .active_points_error
                    .map(|value| format!("{value:+.2}"))
                    .unwrap_or_else(|| "unknown".to_owned());
                format!(
                    "<article><h2>{}</h2><p><strong>Process: {:?}</strong> · Result: {:?} · Projection: {:?}</p><p>Projected active value: {:+.2}; error: {}; outcome rows: {}.</p>{}</article>",
                    escape_html(&row.week_start.map(|date| date.to_string()).unwrap_or_else(|| "Unknown week".to_owned())),
                    row.process,
                    row.result,
                    row.projection,
                    row.projected_active_points_delta.unwrap_or_default(),
                    error,
                    row.outcome_rows,
                    if row.outcome_rows == 0 { "<p>Next: use <code>icelines fantasy decision-outcome-record</code>.</p>" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    format!(
        "<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Fantasy Decision Review</title><style>body{{font:16px system-ui;max-width:72rem;margin:auto;padding:1rem}}article{{border-block-start:1px solid #888;padding-block:1rem}}code{{overflow-wrap:anywhere}}@media(max-width:40rem){{body{{padding:.6rem}}}}</style><main><h1>Fantasy Decision Review</h1><p>{} · {} · {} decisions · {} observed.</p>{}</main></html>",
        escape_html(&view.league_name),
        escape_html(&filter),
        view.summary.decisions,
        view.summary.with_effective_outcomes,
        items
    )
}

fn validate_week_shape(q: &FantasyWebQuery) -> Result<(), (String, NaiveDate)> {
    let Some(value) = q.week.as_deref() else {
        return Ok(());
    };
    let requested = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        (
            format!("week '{value}' must use YYYY-MM-DD"),
            Utc::now().date_naive()
                - Duration::days(Utc::now().weekday().num_days_from_monday().into()),
        )
    })?;
    if requested.weekday() != chrono::Weekday::Mon {
        let canonical =
            requested - Duration::days(requested.weekday().num_days_from_monday().into());
        return Err((format!("week '{value}' is not a Monday"), canonical));
    }
    Ok(())
}

fn validate_requested_week(q: &FantasyWebQuery, view: &FantasyTodayV2View) -> Result<(), String> {
    let Some(value) = q.week.as_deref() else {
        return Ok(());
    };
    let requested = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| format!("week '{value}' must use YYYY-MM-DD"))?;
    if requested.weekday() != chrono::Weekday::Mon {
        return Err(format!(
            "week '{value}' is not a Monday; active week starts {}",
            view.today.context.week_start
        ));
    }
    if requested != view.today.context.week_start {
        return Err(format!(
            "only the active acquisition week {} is available",
            view.today.context.week_start
        ));
    }
    Ok(())
}

fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub async fn get_fantasy_today(
    State(state): State<WebState>,
    Query(q): Query<FantasyWebQuery>,
) -> Response {
    match load_fantasy_today_contract(&state, &q).await {
        Ok(view) => Html(render_fantasy_today_html(&view)).into_response(),
        Err(message) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Html(format!(
                "<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Fantasy Today unavailable</title><main><h1>Fantasy Today unavailable</h1><p role=\"status\">{}</p><p>Run <code>icelines fantasy today</code> for recovery guidance.</p></main></html>",
                escape_html(&message)
            )),
        )
            .into_response(),
    }
}

async fn load_fantasy_today_contract(
    state: &WebState,
    q: &FantasyWebQuery,
) -> Result<FantasyTodayV2View, String> {
    let stats_season = state.config.read().await.active_season.clone();
    let mut request = FantasyTodayAssemblyRequest::from_default_paths(
        q.league.clone(),
        q.team.clone(),
        stats_season,
        icelines_core::CURRENT_SEASON,
        Utc::now(),
    )
    .map_err(|error| error.to_string())?;
    request.local_date = q
        .date
        .as_deref()
        .map(|value| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map_err(|error| format!("invalid date '{value}': {error}"))
        })
        .transpose()?;
    tokio::task::spawn_blocking(move || {
        assemble_fantasy_today(request).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| format!("join local cockpit assembler: {error}"))?
}

fn render_fantasy_today_html(v2: &FantasyTodayV2View) -> String {
    let view = &v2.today;
    let summary = v2.surface_decision();
    let state = match view.state {
        FantasyTodayState::Ready => "ready",
        FantasyTodayState::Provisional => "provisional",
        FantasyTodayState::Blocked => "blocked",
    };
    let decision = summary.primary_display_message();
    let deadline = summary
        .deadline_utc
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| "No pending deadline".to_owned());
    let decision_detail = summary
        .firmness
        .map(|firmness| {
            format!(
                "{:?}; legal now: {}; evidence age: {}{}",
                firmness,
                summary.legal_at_evaluation.unwrap_or(false),
                summary
                    .evidence_age_seconds
                    .map(|seconds| format!("{seconds}s"))
                    .unwrap_or_else(|| "not timestamped".to_owned()),
                summary
                    .matchup_impact
                    .as_ref()
                    .map(|impact| format!("; {impact}"))
                    .unwrap_or_default()
            )
        })
        .unwrap_or_else(|| "No legal action is pending.".to_owned());
    let readiness = view
        .readiness
        .iter()
        .filter(|row| row.state != FantasyTodayState::Ready)
        .map(|row| {
            format!(
                "<li><strong>{}</strong>: {}{}</li>",
                escape_html(&row.workflow),
                escape_html(&row.message),
                row.recovery_command
                    .as_ref()
                    .map(|command| format!(" <code>{}</code>", escape_html(command)))
                    .unwrap_or_default()
            )
        })
        .collect::<String>();
    let alternatives = summary
        .alternative_messages
        .iter()
        .take(3)
        .map(|message| format!("<li>{}</li>", escape_html(message)))
        .collect::<String>();
    let week_plan = v2.week_plan.as_ref().map_or_else(
        || "<p>Week plan unavailable.</p>".to_owned(),
        |plan| {
            let next = plan.primary_sequence.moves.first().map_or_else(
                || "Hold the current roster.".to_owned(),
                |row| {
                    format!(
                        "{}: add {}{} ({})",
                        row.local_date,
                        escape_html(&row.add_player),
                        row.drop_player
                            .as_ref()
                            .map(|drop| format!(", drop {}", escape_html(drop)))
                            .unwrap_or_default(),
                        escape_html(&row.firmness)
                    )
                },
            );
            format!(
                "<p>{:+.2} points; {:+} starts; {} move(s); {} held.</p><p>{}</p><p><a href=\"/fantasy/week-plan?week={}\">Open full week plan</a></p>",
                plan.primary_sequence.projected_value_delta,
                plan.primary_sequence.incremental_usable_starts,
                plan.primary_sequence.moves_used,
                plan.primary_sequence.reserve_after,
                next,
                plan.context.week_start
            )
        },
    );
    format!(
        "<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Fantasy Today</title><style>body{{font:16px system-ui;max-width:72rem;margin:auto;padding:1rem}}section{{border:1px solid #888;border-radius:.5rem;padding:1rem;margin-block:1rem}}.state{{font-weight:700;text-transform:uppercase}}code{{overflow-wrap:anywhere}}@media(max-width:40rem){{body{{padding:.6rem}}}}</style><main><h1>Fantasy Today</h1><p>{} · {} · {} {} · <time>{}</time></p><p class=\"state\" aria-label=\"Cockpit readiness\">{}</p><section aria-labelledby=\"decision\"><h2 id=\"decision\">Do now</h2><p>{}</p><p>{}</p><p>Next deadline: <time>{}</time></p><h3>Next options</h3><ol>{}</ol><p><small>Decision fingerprint: <code>{}</code></small></p></section><section aria-labelledby=\"lineup\"><h2 id=\"lineup\">Lineup and budget</h2><p>{} usable starts; {} open slots; {} bench players have games.</p><p>{}/{} acquisitions used; {} safe for proactive use.</p></section><section aria-labelledby=\"week-plan\"><h2 id=\"week-plan\">Week plan</h2>{}</section><section aria-labelledby=\"readiness\"><h2 id=\"readiness\">Readiness</h2><ul>{}</ul></section></main></html>",
        escape_html(&view.context.league_name),
        escape_html(&view.context.fantasy_team_name),
        escape_html(&view.context.stats_season),
        escape_html(&view.context.season_type.to_string()),
        view.context.date,
        state,
        escape_html(&decision),
        escape_html(&decision_detail),
        escape_html(&deadline),
        alternatives,
        escape_html(&summary.material_fingerprint),
        view.lineup.usable_starts,
        view.lineup.open_active_slots,
        view.lineup.bench_players_with_games,
        view.acquisitions.used,
        view.acquisitions.limit,
        view.acquisitions.proactive_remaining,
        week_plan,
        readiness
    )
}

fn render_fantasy_week_plan_html(view: &icelines_core::FantasyPickupSequenceView) -> String {
    let moves = if view.primary_sequence.moves.is_empty() {
        "<li>Hold the current roster.</li>".to_owned()
    } else {
        view.primary_sequence
            .moves
            .iter()
            .map(|row| {
                format!(
                    "<li><time>{}</time>: add <strong>{}</strong>{} <small>{:+.2}; {}</small></li>",
                    row.local_date,
                    escape_html(&row.add_player),
                    row.drop_player
                        .as_ref()
                        .map(|drop| format!(", drop <strong>{}</strong>", escape_html(drop)))
                        .unwrap_or_default(),
                    row.marginal_active_value,
                    escape_html(&row.firmness)
                )
            })
            .collect::<String>()
    };
    let coverage = view
        .primary_sequence
        .daily_coverage
        .iter()
        .map(|row| {
            format!(
                "<tr><td><time>{}</time></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                row.date, row.usable_starts, row.benched_collisions, row.open_active_slots
            )
        })
        .collect::<String>();
    let alternatives = view
        .alternatives
        .iter()
        .map(|row| {
            format!(
                "<li>{:+.2} points; {:+} starts; {} move(s)</li>",
                row.projected_value_delta, row.incremental_usable_starts, row.moves_used
            )
        })
        .collect::<String>();
    format!(
        "<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Fantasy Week Plan</title><style>body{{font:16px system-ui;max-width:72rem;margin:auto;padding:1rem}}section{{border:1px solid #888;border-radius:.5rem;padding:1rem;margin-block:1rem}}table{{border-collapse:collapse;width:100%}}th,td{{padding:.4rem;text-align:left;border-bottom:1px solid #bbb}}code{{overflow-wrap:anywhere}}</style><main><h1>Fantasy Week Plan</h1><p>{} · {} · <time>{}</time> through <time>{}</time></p><section><h2>Primary sequence</h2><p>{:+.2} points; {:+} usable starts; {} acquisition(s) held.</p><ol>{}</ol><p>{}</p></section><section><h2>Daily coverage</h2><table><thead><tr><th>Date</th><th>Starts</th><th>Bench collisions</th><th>Open slots</th></tr></thead><tbody>{}</tbody></table></section><section><h2>Fallbacks</h2><ol>{}</ol></section><p><small>Bounded search: {} states, beam {}{} · <code>{}</code></small></p></main></html>",
        escape_html(&view.context.league_name),
        escape_html(&view.context.fantasy_team_name),
        view.context.week_start,
        view.context.week_end,
        view.primary_sequence.projected_value_delta,
        view.primary_sequence.incremental_usable_starts,
        view.primary_sequence.reserve_after,
        moves,
        escape_html(&view.holdback_recommendation),
        coverage,
        alternatives,
        view.evaluated_states,
        view.beam_width,
        if view.truncated { "; truncated" } else { "" },
        escape_html(&view.material_fingerprint)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_template(tmpl: FantasyTemplate) -> Response {
    match tmpl.render() {
        Ok(html) => Html(html).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!(
                "<!doctype html><body><h1>500</h1><p>{e}</p></body>"
            )),
        )
            .into_response(),
    }
}

fn project_template(
    active_label: String,
    view: FantasyRosterGapView,
    simulation: Option<FantasySimulationView>,
    simulation_error: Option<String>,
    categories_query: String,
    add_player_query: String,
    drop_player_query: String,
) -> FantasyTemplate {
    let rows = view
        .rows
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            let candidate = row.best_available.as_ref()?;
            let replacement = row
                .replacement_target
                .as_ref()
                .map(|target| target.display_name.clone())
                .unwrap_or_else(|| "-".to_string());
            let weighted_delta = row
                .replacement_target
                .as_ref()
                .map(|target| target.weighted_delta)
                .unwrap_or(row.weighted_gap_score);
            Some(FantasyGapRow {
                rank: idx + 1,
                action: format!("{:?}", row.action).to_ascii_lowercase(),
                action_reason: row.action_reason.clone(),
                category: row.category.clone(),
                roster_total: format!("{:.1}", row.user_total),
                weight: format!("{:.2}", row.weight),
                player_id: candidate.player_id,
                best_available: candidate.display_name.clone(),
                team: candidate.team.clone(),
                position: candidate.position.clone(),
                value: format!("{:.1}", candidate.value),
                weighted_value: format!("{:.1}", candidate.weighted_value),
                weighted_delta: format!("{weighted_delta:.1}"),
                replacement,
                recommendation: row.recommendation.clone(),
            })
        })
        .collect::<Vec<_>>();
    let empty = rows.is_empty();
    let mut warnings = view.warnings;
    if let Some(message) = simulation_error {
        warnings.push(format!("Fantasy simulation unavailable: {message}"));
    }

    FantasyTemplate {
        active_label,
        league: view.league,
        team: view.team,
        scoring_scheme: view.scoring_scheme,
        categories: if categories_query.is_empty() {
            view.categories.join(",")
        } else {
            categories_query
        },
        add_player: add_player_query,
        drop_player: drop_player_query,
        rows,
        simulation_rows: simulation
            .as_ref()
            .map(|simulation| {
                simulation
                    .rows
                    .iter()
                    .map(|row| FantasySimulationRow {
                        rank: row.rank,
                        team: row.team.clone(),
                        owner: row.owner.clone(),
                        is_user_team: row.is_user_team,
                        projected_score: format!("{:.1}", row.projected_score),
                        score_gap: format!("{:.1}", row.score_gap_to_leader),
                        games_remaining: row.games_remaining,
                        rostered_players: row.rostered_players,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        simulation_scenarios: simulation
            .as_ref()
            .map(|simulation| {
                simulation
                    .scenarios
                    .iter()
                    .map(|row| FantasySimulationScenarioRow {
                        action: format!("{:?}", row.action).to_ascii_lowercase(),
                        label: row.label.clone(),
                        add_player: row.add_player.clone().unwrap_or_else(|| "-".to_string()),
                        drop_player: row.drop_player.clone().unwrap_or_else(|| "-".to_string()),
                        projected_score_delta: format!("{:+.1}", row.projected_score_delta),
                        projected_games_delta: row.projected_games_delta,
                        confidence: format!("{:?}", row.confidence).to_ascii_lowercase(),
                        explanation: row.explanation.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        simulation_assumptions: simulation
            .as_ref()
            .map(|simulation| simulation.assumptions.clone())
            .unwrap_or_default(),
        simulation_warnings: simulation
            .as_ref()
            .map(|simulation| simulation.warnings.clone())
            .unwrap_or_default(),
        warnings,
        empty_title: if empty {
            "No roster gaps found".to_string()
        } else {
            String::new()
        },
        empty_detail: if empty {
            "No available skater candidates matched the active league import.".to_string()
        } else {
            String::new()
        },
    }
}

pub(super) async fn build_fantasy_simulation(
    state: &WebState,
    q: &FantasyWebQuery,
) -> Result<FantasySimulationView, String> {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = season_str
        .parse()
        .map_err(|e| format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"))?;
    let snapshot = read_fantasy_league_snapshot(q.league.as_deref())?;
    let scheme_name = q
        .scheme
        .clone()
        .unwrap_or_else(|| snapshot.scoring_scheme.clone());
    let scheme = Scheme::builtin_named(&scheme_name)
        .ok_or_else(|| format!("unknown scoring scheme '{scheme_name}'"))?;
    let repo = state.repo.read().await;
    let skaters = repo
        .skaters(Season(season_u32), season_type)
        .collect::<Vec<_>>();
    let goalies = repo
        .goalies(Season(season_u32), season_type)
        .collect::<Vec<_>>();
    let schedule_cache = remaining_games_by_team_from_cache(Season(season_u32));
    let schedule_available = !schedule_cache.remaining_by_team.is_empty();
    let mut scenario_rosters = Vec::new();
    if q.add_player.is_some() || q.drop_player.is_some() {
        let baseline = snapshot
            .teams
            .iter()
            .find(|team| team.name == snapshot.user_team)
            .map(|team| team.roster.clone())
            .unwrap_or_default();
        let scenario = resolve_fantasy_scenario_roster_details(
            &baseline,
            q.add_player.as_deref(),
            q.drop_player.as_deref(),
            &skaters,
            &goalies,
        )?;
        scenario_rosters.push(FantasySimulationScenarioRosterInput {
            id: "web-add-drop".to_string(),
            label: "Web add/drop scenario".to_string(),
            add_player: scenario
                .resolved_add_player
                .or_else(|| q.add_player.clone()),
            drop_player: scenario
                .resolved_drop_player
                .or_else(|| q.drop_player.clone()),
            baseline_roster: baseline,
            scenario_roster: scenario.roster,
            confidence: FantasySimulationConfidence::Low,
        });
    }
    Ok(build_fantasy_simulation_view(
        FantasySimulationBuildInput {
            season: Season(season_u32),
            season_type,
            league: snapshot.league,
            scoring_scheme: scheme_name,
            horizon: FantasySimulationHorizon::Weeks(q.weeks.unwrap_or(4).max(1)),
            user_team: snapshot.user_team,
            teams: snapshot
                .teams
                .into_iter()
                .map(|team| FantasySimulationRosterTeamInput {
                    team: team.name,
                    owner: team.owner,
                    roster: team.roster,
                })
                .collect(),
            remaining_by_team: schedule_cache.remaining_by_team,
            scenarios: Vec::new(),
            scenario_rosters,
            assumptions: vec![
                "projects each roster from season-to-date fantasy points per played game"
                    .to_string(),
                "games remaining use the local schedule cache when available".to_string(),
            ],
            warnings: if schedule_available {
                Vec::new()
            } else {
                vec![
                    "schedule unavailable; projection falls back to current fantasy score"
                        .to_string(),
                ]
            },
            schedule_available,
        },
        &skaters,
        &goalies,
        &scheme,
    ))
}

pub(super) async fn build_fantasy_daily(
    state: &WebState,
    q: &FantasyWebQuery,
) -> Result<icelines_core::FantasyDailyDeltaView, String> {
    let date_raw = q
        .date
        .as_deref()
        .ok_or_else(|| "date is required; use ?date=YYYY-MM-DD".to_string())?;
    let date = NaiveDate::parse_from_str(date_raw, "%Y-%m-%d")
        .map_err(|e| format!("date '{date_raw}' is not a valid YYYY-MM-DD value: {e}"))?;
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = season_str
        .parse()
        .map_err(|e| format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"))?;
    let db = open_existing_fantasy_db()?;
    let data_root = data_root().ok_or_else(|| "cannot determine home directory".to_string())?;
    if data_root.join("manifest").is_dir() {
        let store = DataStore::open(&data_root).map_err(|e| e.to_string())?;
        build_fantasy_daily_delta_view(
            &db,
            &store,
            date,
            Season(season_u32),
            season_type,
            q.league.as_deref(),
        )
        .map_err(|e| e.to_string())
    } else {
        build_fantasy_daily_missing_cache_view(&db, date, Season(season_u32), season_type, q)
    }
}

pub(super) async fn build_fantasy_matchup(
    state: &WebState,
    q: &FantasyWebQuery,
) -> Result<icelines_core::FantasyMatchupWeekView, String> {
    let date_raw = q
        .date
        .as_deref()
        .ok_or_else(|| "date is required; use ?date=YYYY-MM-DD".to_string())?;
    let date = NaiveDate::parse_from_str(date_raw, "%Y-%m-%d")
        .map_err(|e| format!("date '{date_raw}' is not a valid YYYY-MM-DD value: {e}"))?;
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = season_str
        .parse()
        .map_err(|e| format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"))?;
    let db = open_existing_fantasy_db()?;
    let data_root = data_root().ok_or_else(|| "cannot determine home directory".to_string())?;
    if data_root.join("manifest").is_dir() {
        let store = DataStore::open(&data_root).map_err(|e| e.to_string())?;
        build_fantasy_matchup_week_view(
            &db,
            &store,
            date,
            Season(season_u32),
            season_type,
            q.league.as_deref(),
        )
        .map_err(|e| e.to_string())
    } else {
        build_fantasy_matchup_missing_cache_view(&db, date, Season(season_u32), season_type, q)
    }
}

pub(super) async fn build_fantasy_roster_shape(
    state: &WebState,
    q: &FantasyWebQuery,
) -> Result<Vec<icelines_core::RosterShapeValidationView>, String> {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = season_str
        .parse()
        .map_err(|e| format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"))?;
    let db = open_existing_fantasy_db()?;
    let league = if let Some(name) = q.league.as_deref() {
        db.list_leagues()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|league| league.name == name)
            .ok_or_else(|| format!("fantasy league '{name}' not found"))?
    } else {
        db.get_active_league()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no active fantasy league found".to_string())?
    };
    let teams = if let Some(team_name) = q.team.as_deref() {
        vec![db
            .get_team_by_name(&league.id, team_name)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("team '{team_name}' not found in '{}'", league.name))?]
    } else {
        db.list_teams(&league.id).map_err(|e| e.to_string())?
    };
    let repo = state.repo.read().await;
    let positions = repo
        .skaters(Season(season_u32), season_type)
        .chain(repo.goalies(Season(season_u32), season_type))
        .map(|view| (view.identity.name_normalized.clone(), vec![view.position()]))
        .collect::<BTreeMap<String, Vec<Position>>>();
    teams
        .iter()
        .map(|team| db.validate_team_roster_shape(&league, team, &positions))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub(super) async fn build_fantasy_gaps(
    state: &WebState,
    q: &FantasyWebQuery,
) -> Result<FantasyRosterGapView, String> {
    let (season_str, season_type) = {
        let cfg = state.config.read().await;
        (
            cfg.active_season.clone(),
            SeasonType::parse_lossy(&cfg.active_season_type),
        )
    };
    let season_u32: u32 = season_str
        .parse()
        .map_err(|e| format!("active season '{season_str}' is not a valid YYYYZZZZ id: {e}"))?;
    let snapshot = read_fantasy_league_snapshot(q.league.as_deref())?;
    let scheme = q
        .scheme
        .clone()
        .unwrap_or_else(|| snapshot.scoring_scheme.clone());
    let categories = split_csv(q.categories.as_deref())
        .into_iter()
        .map(|category| category.to_ascii_lowercase())
        .collect();
    let repo = state.repo.read().await;
    let all_rostered = snapshot.all_rostered();
    let user_rostered = snapshot.user_rostered();
    Ok(FantasyRosterGapView::from_repository(
        &repo,
        FantasyRosterGapInput {
            season: Season(season_u32),
            season_type,
            league: &snapshot.league,
            team: &snapshot.user_team,
            scoring_scheme: &scheme,
            categories,
            user_roster_keys: user_rostered,
            all_rostered_keys: all_rostered,
            limit: q.top.unwrap_or(12).clamp(1, 40),
        },
    ))
}

fn read_fantasy_league_snapshot(
    league_name: Option<&str>,
) -> Result<icelines_fetch::fantasy_db::FantasyLeagueSnapshot, String> {
    open_existing_fantasy_db()
        .and_then(|db| db.league_snapshot(league_name).map_err(|e| e.to_string()))
}

fn open_existing_fantasy_db() -> Result<FantasyDb, String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "cannot determine home directory".to_string())?;
    let db_path = home.join(".icelines").join("icelines.db");
    if !db_path.is_file() {
        return Err(format!(
            "no local fantasy league database found at {}; create or import a fantasy league first",
            db_path.display()
        ));
    }
    FantasyDb::open_existing_read_only_path(db_path).map_err(|e| e.to_string())
}

fn build_fantasy_daily_missing_cache_view(
    db: &FantasyDb,
    date: NaiveDate,
    season: Season,
    season_type: SeasonType,
    q: &FantasyWebQuery,
) -> Result<icelines_core::FantasyDailyDeltaView, String> {
    let snapshot = db
        .league_snapshot(q.league.as_deref())
        .map_err(|e| e.to_string())?;
    let scheme = Scheme::builtin_named(&snapshot.scoring_scheme).ok_or_else(|| {
        format!(
            "unknown fantasy scoring scheme '{}'",
            snapshot.scoring_scheme
        )
    })?;
    Ok(icelines_core::FantasyDailyDeltaView::from_input(
        FantasyDailyDeltaInput {
            season,
            season_type,
            date,
            league: snapshot.league,
            scoring_scheme: snapshot.scoring_scheme,
            teams: snapshot
                .teams
                .iter()
                .map(|team| FantasyDailyTeamInput {
                    team: team.name.clone(),
                    owner: team.owner.clone(),
                    is_user_team: team.name == snapshot.user_team,
                    roster: team
                        .roster
                        .iter()
                        .map(|roster_key| FantasyDailyPlayerInput {
                            display_name: display_name_from_roster_key(roster_key),
                            roster_key: roster_key.clone(),
                            position: "?".to_string(),
                            line: None,
                        })
                        .collect(),
                })
                .collect(),
            warnings: vec![format!("no cached boxscores found for {date}")],
            source_state: vec![
                SourceState::complete(SourceKind::FantasyImport),
                SourceState::missing(SourceKind::Boxscore),
            ],
        },
        &scheme,
    ))
}

fn build_fantasy_matchup_missing_cache_view(
    db: &FantasyDb,
    date: NaiveDate,
    season: Season,
    season_type: SeasonType,
    q: &FantasyWebQuery,
) -> Result<icelines_core::FantasyMatchupWeekView, String> {
    let league = if let Some(name) = q.league.as_deref() {
        db.list_leagues()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|league| league.name == name)
            .ok_or_else(|| format!("fantasy league '{name}' not found"))?
    } else {
        db.get_active_league()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no active fantasy league found".to_string())?
    };
    let snapshot = db
        .league_snapshot(Some(&league.name))
        .map_err(|e| e.to_string())?;
    let (week_start, week_end) = Timeframe::Week.range(date);
    let schedule_rows = db
        .list_matchups(&league.id, week_start)
        .map_err(|e| e.to_string())?;
    let schedule_source = if schedule_rows.is_empty() {
        SourceState::missing(SourceKind::Schedule)
    } else {
        SourceState::complete(SourceKind::Schedule)
    };
    Ok(icelines_core::FantasyMatchupWeekView::from_input(
        FantasyMatchupWeekInput {
            season,
            season_type,
            week_start,
            week_end,
            league: snapshot.league,
            scoring_scheme: snapshot.scoring_scheme,
            team_totals: snapshot
                .teams
                .iter()
                .map(|team| FantasyMatchupTeamTotalInput {
                    team: team.name.clone(),
                    owner: team.owner.clone(),
                    is_user_team: team.name == snapshot.user_team,
                    weekly_points: 0.0,
                    days_scored: 0,
                    rostered_players: team.roster.len() as u16,
                    scored_players: 0,
                })
                .collect(),
            schedule: schedule_rows
                .into_iter()
                .map(|row| FantasyMatchupScheduleInput {
                    matchup_id: Some(row.id),
                    home_team: row.home_team,
                    away_team: row.away_team,
                })
                .collect(),
            warnings: vec![format!(
                "no cached boxscores found for fantasy matchup week starting {week_start}"
            )],
            source_state: vec![
                SourceState::complete(SourceKind::FantasyImport),
                schedule_source,
                SourceState {
                    source: SourceKind::Boxscore,
                    state: Completeness::Unavailable,
                    provenance: None,
                    fetched_at: None,
                    stale_reason: None,
                    message: Some(
                        "one or more matchup-week dates are missing cached boxscores".to_string(),
                    ),
                },
            ],
        },
    ))
}

fn display_name_from_roster_key(roster_key: &str) -> String {
    roster_key
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_csv(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn data_root() -> Option<std::path::PathBuf> {
    if let Some(root) = std::env::var_os("ICELINES_DATA_ROOT") {
        return Some(std::path::PathBuf::from(root));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .map(|home| home.join(".icelines").join("data"))
}

#[cfg(test)]
mod fantasy_today_surface_tests {
    use super::*;

    #[test]
    fn web_consumes_the_sealed_surface_projection() {
        let fixture: icelines_core::FantasyTodaySurfaceDecision =
            serde_json::from_str(include_str!(
                "../../../icelines-core/tests/fixtures/fantasy_today_surface_decision.v1.json"
            ))
            .unwrap();
        let decision = fixture.primary_display_message();

        assert!(decision.contains("evidence age: 90s"));
        assert!(decision.contains("Fixture Rival"));
        assert_eq!(fixture.alternative_messages.len(), 2);
        assert_eq!(fixture.material_fingerprint.len(), 64);
    }

    #[test]
    fn decision_review_rejects_conflicting_sticky_filters_before_io() {
        let query = FantasyWebQuery {
            week: Some("2026-11-09".to_owned()),
            season: Some("20262027".to_owned()),
            ..FantasyWebQuery::default()
        };
        let error = build_fantasy_decision_review_web(&query).unwrap_err();
        assert!(error.contains("cannot be combined"));
    }

    #[test]
    fn decision_review_no_store_wrapper_covers_error_and_success_responses() {
        for status in [StatusCode::OK, StatusCode::BAD_REQUEST] {
            let response = no_store(status.into_response());
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-store"
            );
        }
    }

    #[test]
    fn readiness_workflow_parser_accepts_url_friendly_names() {
        assert_eq!(
            parse_readiness_workflow("week-plan").unwrap(),
            FantasyReadinessWorkflow::WeekPlan
        );
        assert_eq!(
            parse_readiness_workflow("decision-review").unwrap(),
            FantasyReadinessWorkflow::DecisionReview
        );
        assert!(parse_readiness_workflow("waivers").is_err());
    }
}
