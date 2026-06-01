use crate::state::WebState;
use crate::templates::{
    FantasyGapRow, FantasySimulationRow, FantasySimulationScenarioRow, FantasyTemplate,
};
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use chrono::NaiveDate;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::timeframe::Timeframe;
use icelines_core::view_model::{
    Completeness, FantasyDailyDeltaInput, FantasyDailyPlayerInput, FantasyDailyTeamInput,
    FantasyMatchupScheduleInput, FantasyMatchupTeamTotalInput, FantasyMatchupWeekInput, SourceKind,
    SourceState,
};
use icelines_core::{
    build_fantasy_simulation_view, resolve_fantasy_scenario_roster_details, FantasyRosterGapInput,
    FantasyRosterGapView, FantasySimulationBuildInput, FantasySimulationConfidence,
    FantasySimulationHorizon, FantasySimulationRosterTeamInput,
    FantasySimulationScenarioRosterInput, FantasySimulationView, Scheme,
};
use icelines_fetch::datastore::DataStore;
use icelines_fetch::fantasy_daily::build_fantasy_daily_delta_view;
use icelines_fetch::fantasy_db::FantasyDb;
use icelines_fetch::fantasy_matchup::build_fantasy_matchup_week_view;
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
    pub team: Option<String>,
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
