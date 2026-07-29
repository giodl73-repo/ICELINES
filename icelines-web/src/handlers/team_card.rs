use std::collections::BTreeMap;

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use icelines_core::{
    CardAssetFallback, CardAssetReference, CardDocumentView, CardKind, CardMetricComparisonView,
    CardMetricView, CardSectionView, MetricUnit, MetricValue,
};

use crate::card_store::{
    default_scenario, fantasy_draft_card, fantasy_morning_card, fantasy_roster_card,
    fantasy_trade_card, forecast_history_card, forecast_movement_card, organization_window_card,
    season_simulation_card, team_prognosis_card, CardStoreError,
};
use crate::state::WebState;
use crate::templates::{
    FantasyRosterCardTemplate, GenericCardAlternative, GenericCardDecision, GenericCardMetricGroup,
    GenericCardPlayerGroup, GenericCardTimelineGroup, GenericCardTimelineItem, TeamCardLineupGroup,
    TeamCardLineupSlot, TeamCardMetric, TeamCardPlayer, TeamCardTemplate,
};

#[derive(Debug, Default, Deserialize)]
pub struct TeamCardQuery {
    pub scenario: Option<String>,
    pub page: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct FantasyRosterCardQuery {
    pub page: Option<String>,
}

pub async fn get_season_simulation_card_json(
    Path((season, team)): Path<(u32, String)>,
    headers: HeaderMap,
) -> Response {
    match season_simulation_card(season, &team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_forecast_movement_card_json(
    Path((season, team)): Path<(u32, String)>,
    headers: HeaderMap,
) -> Response {
    match forecast_movement_card(season, &team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_forecast_history_card_json(
    Path((season, team)): Path<(u32, String)>,
    headers: HeaderMap,
) -> Response {
    match forecast_history_card(season, &team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_organization_window_card_json(
    Path((season, team)): Path<(u32, String)>,
    headers: HeaderMap,
) -> Response {
    match organization_window_card(season, &team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_organization_window_card(
    State(state): State<WebState>,
    Path((season, team)): Path<(u32, String)>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match organization_window_card(season, &team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_first = !matches!(query.page.as_deref(), Some("insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_first);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_forecast_history_card(
    State(state): State<WebState>,
    Path((season, team)): Path<(u32, String)>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match forecast_history_card(season, &team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_first = !matches!(query.page.as_deref(), Some("insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_first);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_forecast_movement_card(
    State(state): State<WebState>,
    Path((season, team)): Path<(u32, String)>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match forecast_movement_card(season, &team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_first = !matches!(query.page.as_deref(), Some("insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_first);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_season_simulation_card(
    State(state): State<WebState>,
    Path((season, team)): Path<(u32, String)>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match season_simulation_card(season, &team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_first = !matches!(query.page.as_deref(), Some("insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_first);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_trade_card_json(Path(team): Path<String>, headers: HeaderMap) -> Response {
    match fantasy_trade_card(&team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_fantasy_trade_card(
    State(state): State<WebState>,
    Path(team): Path<String>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match fantasy_trade_card(&team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_first = !matches!(query.page.as_deref(), Some("insider" | "trade-insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_first);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_morning_card_json(
    Path(team): Path<String>,
    headers: HeaderMap,
) -> Response {
    match fantasy_morning_card(&team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_fantasy_morning_card(
    State(state): State<WebState>,
    Path(team): Path<String>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match fantasy_morning_card(&team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_first = !matches!(query.page.as_deref(), Some("insider" | "morning-insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_first);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_draft_card_json(Path(team): Path<String>, headers: HeaderMap) -> Response {
    match fantasy_draft_card(&team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_fantasy_draft_card(
    State(state): State<WebState>,
    Path(team): Path<String>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match fantasy_draft_card(&team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_first = !matches!(query.page.as_deref(), Some("insider" | "draft-insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_first);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_fantasy_roster_card_json(
    Path(team): Path<String>,
    headers: HeaderMap,
) -> Response {
    match fantasy_roster_card(&team) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_fantasy_roster_card(
    State(state): State<WebState>,
    Path(team): Path<String>,
    Query(query): Query<FantasyRosterCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match fantasy_roster_card(&team) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let show_roster = !matches!(query.page.as_deref(), Some("insider" | "roster-insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_fantasy_template(active_label, &card, show_roster);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub async fn get_team_card_json(
    Path((season, team)): Path<(u32, String)>,
    Query(query): Query<TeamCardQuery>,
    headers: HeaderMap,
) -> Response {
    match team_prognosis_card(season, &team, query.scenario.as_deref()) {
        Ok(card) => {
            let fingerprint = card.fingerprint.clone();
            cached_response(axum::Json(card).into_response(), &fingerprint, &headers)
        }
        Err(error) => card_error(error, false),
    }
}

pub async fn get_team_card(
    State(state): State<WebState>,
    Path((season, team)): Path<(u32, String)>,
    Query(query): Query<TeamCardQuery>,
    headers: HeaderMap,
) -> Response {
    let card = match team_prognosis_card(season, &team, query.scenario.as_deref()) {
        Ok(card) => card,
        Err(error) => return card_error(error, true),
    };
    let active_label = state.config.read().await.active_label.clone();
    let scenario = query
        .scenario
        .or_else(|| default_scenario(&team).map(str::to_string))
        .expect("card store accepted team without default scenario");
    let show_depth = !matches!(query.page.as_deref(), Some("insider"));
    let fingerprint = card.fingerprint.clone();
    let template = project_template(active_label, season, scenario, show_depth, &card);
    match template.render() {
        Ok(html) => cached_response(Html(html).into_response(), &fingerprint, &headers),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("template render failed: {error}")),
        )
            .into_response(),
    }
}

pub(crate) fn cached_response(
    mut response: Response,
    fingerprint: &str,
    request_headers: &HeaderMap,
) -> Response {
    let etag = format!("\"{fingerprint}\"");
    if request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.split(',').any(|value| value.trim() == etag))
    {
        let mut not_modified = StatusCode::NOT_MODIFIED.into_response();
        not_modified.headers_mut().insert(
            header::ETAG,
            HeaderValue::from_str(&etag).expect("SHA-256 ETag header"),
        );
        return not_modified;
    }
    response.headers_mut().insert(
        header::ETAG,
        HeaderValue::from_str(&etag).expect("SHA-256 ETag header"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300, must-revalidate"),
    );
    response
}

fn project_template(
    active_label: String,
    season: u32,
    scenario: String,
    show_depth: bool,
    card: &CardDocumentView,
) -> TeamCardTemplate {
    let team = card
        .context
        .joins
        .team_ids
        .first()
        .cloned()
        .unwrap_or_default();
    let assets = card
        .assets
        .iter()
        .map(|asset| {
            let url = match &asset.reference {
                Some(CardAssetReference::ExternalUrl(url)) => url.clone(),
                _ => String::new(),
            };
            let fallback = match &asset.fallback {
                CardAssetFallback::Initials(value) | CardAssetFallback::Abbreviation(value) => {
                    value.clone()
                }
                CardAssetFallback::None => String::new(),
            };
            (asset.id.as_str(), (url, fallback))
        })
        .collect::<BTreeMap<_, _>>();
    let mut lineup_groups = Vec::new();
    let mut baseline_metrics = Vec::new();
    let mut points_range = String::new();
    let mut bridge_title = String::new();
    let mut bridge_metrics = Vec::new();
    let mut breakouts = Vec::new();
    let mut downturns = Vec::new();
    let mut warnings = card
        .warnings
        .iter()
        .map(|warning| warning.message.clone())
        .collect::<Vec<_>>();
    let mut limitations = Vec::new();

    for section in card.pages.iter().flat_map(|page| &page.sections) {
        match section {
            CardSectionView::Lineup(lineup) => {
                lineup_groups = lineup
                    .groups
                    .iter()
                    .map(|group| TeamCardLineupGroup {
                        label: group.label.clone(),
                        slots: group
                            .slots
                            .iter()
                            .map(|slot| {
                                let (asset_url, fallback) = slot
                                    .asset_id
                                    .as_deref()
                                    .and_then(|id| assets.get(id))
                                    .cloned()
                                    .unwrap_or_default();
                                TeamCardLineupSlot {
                                    label: slot.label.clone(),
                                    name: slot
                                        .subject_label
                                        .clone()
                                        .unwrap_or_else(|| "Open".to_string()),
                                    score: slot
                                        .metrics
                                        .first()
                                        .map(|metric| metric.display_text.clone())
                                        .unwrap_or_else(|| "—".to_string()),
                                    asset_url,
                                    fallback,
                                }
                            })
                            .collect(),
                    })
                    .collect();
            }
            CardSectionView::MetricStrip(strip) if strip.id == "baseline-headlines" => {
                baseline_metrics = strip.metrics.iter().map(project_metric).collect();
            }
            CardSectionView::ProbabilityRange(range) if range.id == "points-range" => {
                points_range = range
                    .ranges
                    .first()
                    .map(|range| range.display_text.clone())
                    .unwrap_or_default();
            }
            CardSectionView::ScenarioBridge(bridge) => {
                bridge_title = bridge.title.clone();
                bridge_metrics = bridge.metrics.iter().map(project_metric).collect();
            }
            CardSectionView::PlayerList(players) if players.id == "breakout-upside" => {
                breakouts = project_players(&players.rows, &assets);
            }
            CardSectionView::PlayerList(players) if players.id == "downside-risks" => {
                downturns = project_players(&players.rows, &assets);
            }
            CardSectionView::StateNotice(notice) => {
                warnings.extend(
                    notice
                        .warnings
                        .iter()
                        .map(|warning| warning.message.clone()),
                );
            }
            CardSectionView::Methodology(methodology) => {
                limitations = methodology.limitations.clone();
            }
            _ => {}
        }
    }
    warnings.sort();
    warnings.dedup();

    let base = format!("/icecast/{season}/{team}/card?scenario={scenario}");
    TeamCardTemplate {
        active_label,
        title: card.title.clone(),
        subtitle: card.subtitle.clone().unwrap_or_default(),
        team: team.clone(),
        season,
        scenario: scenario.clone(),
        show_depth,
        primary: card.theme.primary.clone().unwrap_or_default(),
        secondary: card.theme.secondary.clone().unwrap_or_default(),
        accent: card.theme.accent.clone().unwrap_or_default(),
        json_href: format!("/api/v1/cards/team-prognosis/{season}/{team}?scenario={scenario}"),
        depth_href: format!("{base}&page=depth-chart"),
        insider_href: format!("{base}&page=insider"),
        lineup_groups,
        baseline_metrics,
        points_range,
        bridge_title,
        bridge_metrics,
        breakouts,
        downturns,
        warnings,
        limitations,
    }
}

fn project_fantasy_template(
    active_label: String,
    card: &CardDocumentView,
    show_roster: bool,
) -> FantasyRosterCardTemplate {
    let page = if show_roster {
        &card.pages[0]
    } else {
        &card.pages[1]
    };
    let mut lineup_groups = Vec::new();
    let mut metric_groups = Vec::new();
    let mut decisions = Vec::new();
    let mut player_groups = Vec::new();
    let mut timeline_groups = Vec::new();
    let mut warnings = card
        .warnings
        .iter()
        .map(|warning| warning.message.clone())
        .collect::<Vec<_>>();
    let mut methodology_title = String::new();
    let mut limitations = Vec::new();
    for section in &page.sections {
        match section {
            CardSectionView::Lineup(lineup) => {
                lineup_groups = lineup
                    .groups
                    .iter()
                    .map(|group| TeamCardLineupGroup {
                        label: group.label.clone(),
                        slots: group
                            .slots
                            .iter()
                            .map(|slot| {
                                let name = slot
                                    .subject_label
                                    .clone()
                                    .unwrap_or_else(|| "Open".to_string());
                                TeamCardLineupSlot {
                                    label: slot.label.clone(),
                                    fallback: initials(&name),
                                    name,
                                    score: slot
                                        .metrics
                                        .first()
                                        .map(|metric| metric.display_text.clone())
                                        .unwrap_or_else(|| "NR".to_string()),
                                    asset_url: String::new(),
                                }
                            })
                            .collect(),
                    })
                    .collect();
            }
            CardSectionView::MetricStrip(strip) => {
                metric_groups.push(GenericCardMetricGroup {
                    title: strip.title.clone().unwrap_or_else(|| "Metrics".to_string()),
                    metrics: strip.metrics.iter().map(project_metric).collect(),
                });
            }
            CardSectionView::ProbabilityRange(range) => {
                metric_groups.push(GenericCardMetricGroup {
                    title: range.title.clone(),
                    metrics: range
                        .ranges
                        .iter()
                        .map(|item| TeamCardMetric {
                            label: item.label.clone(),
                            value: item.display_text.clone(),
                            comparison: String::new(),
                        })
                        .collect(),
                });
            }
            CardSectionView::ScenarioBridge(bridge) => {
                metric_groups.push(GenericCardMetricGroup {
                    title: bridge.title.clone(),
                    metrics: bridge.metrics.iter().map(project_metric).collect(),
                });
            }
            CardSectionView::Decision(decision) => {
                decisions.push(GenericCardDecision {
                    title: decision.title.clone(),
                    recommendation: decision.recommendation.clone(),
                    rationale: decision.rationale.clone(),
                    alternatives: decision
                        .alternatives
                        .iter()
                        .map(|alternative| GenericCardAlternative {
                            label: alternative.label.clone(),
                            detail: alternative.detail.clone().unwrap_or_default(),
                        })
                        .collect(),
                });
            }
            CardSectionView::PlayerList(players) => {
                player_groups.push(GenericCardPlayerGroup {
                    title: players.title.clone(),
                    players: players
                        .rows
                        .iter()
                        .map(|row| TeamCardPlayer {
                            name: row.name.clone(),
                            role: row.role.clone().unwrap_or_default(),
                            asset_url: String::new(),
                            fallback: initials(&row.name),
                            metrics: row.metrics.iter().map(project_metric).collect(),
                        })
                        .collect(),
                });
            }
            CardSectionView::Timeline(timeline) => {
                timeline_groups.push(GenericCardTimelineGroup {
                    title: timeline.title.clone(),
                    items: timeline
                        .items
                        .iter()
                        .map(|item| GenericCardTimelineItem {
                            label: item.label.clone(),
                            effective_at: item.effective_at.to_rfc3339(),
                            detail: item.detail.clone().unwrap_or_default(),
                        })
                        .collect(),
                });
            }
            CardSectionView::StateNotice(notice) => {
                warnings.extend(
                    notice
                        .warnings
                        .iter()
                        .map(|warning| warning.message.clone()),
                );
                if let Some(detail) = &notice.detail {
                    warnings.push(detail.clone());
                }
            }
            CardSectionView::Methodology(methodology) => {
                methodology_title = methodology.title.clone();
                limitations.extend(methodology.methods.iter().map(|method| {
                    format!("{} {} — {}", method.label, method.version, method.summary)
                }));
                limitations.extend(methodology.limitations.iter().cloned());
            }
            _ => {}
        }
    }
    warnings.sort();
    warnings.dedup();
    let team = card
        .context
        .joins
        .team_ids
        .first()
        .map(String::as_str)
        .unwrap_or("unknown");
    let (kicker, nav_label, json_href, first_href, insider_href, first_label) = match card.card_kind
    {
        CardKind::SeasonSimulation => (
            "ICECAST · SEASON SIMULATION",
            "Season simulation card pages",
            format!(
                "/api/v1/cards/season-simulation/{}/{team}",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/simulation?page=scoreboard",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/simulation?page=insider",
                card.context.view.window.season.0
            ),
            "The Scoreboard",
        ),
        CardKind::ForecastMovement => (
            "ICECAST · FORECAST MOVEMENT",
            "Forecast movement card pages",
            format!(
                "/api/v1/cards/forecast-movement/{}/{team}",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/movement?page=shift",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/movement?page=insider",
                card.context.view.window.season.0
            ),
            "The Shift",
        ),
        CardKind::ForecastHistory => (
            "ICECAST · FORECAST HISTORY",
            "Forecast history card pages",
            format!(
                "/api/v1/cards/forecast-history/{}/{team}",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/history?page=tape",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/history?page=insider",
                card.context.view.window.season.0
            ),
            "The Tape",
        ),
        CardKind::OrganizationWindow => (
            "THE RINK · ORGANIZATION WINDOW",
            "Organization Window card pages",
            format!(
                "/api/v1/cards/organization-window/{}/{team}",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/window?page=window",
                card.context.view.window.season.0
            ),
            format!(
                "/icecast/{}/{team}/window?page=insider",
                card.context.view.window.season.0
            ),
            "The Window",
        ),
        CardKind::FantasyDraft => (
            "THE BENCH · FANTASY DRAFT",
            "Fantasy draft card pages",
            format!("/api/v1/cards/fantasy-draft/{team}"),
            format!("/fantasy/cards/draft/{team}?page=draft-board"),
            format!("/fantasy/cards/draft/{team}?page=insider"),
            "The Draft Board",
        ),
        CardKind::FantasyMorning => (
            "THE INSIDER · MORNING SKATE",
            "Fantasy morning card pages",
            format!("/api/v1/cards/fantasy-morning/{team}"),
            format!("/fantasy/cards/morning/{team}?page=morning-skate"),
            format!("/fantasy/cards/morning/{team}?page=insider"),
            "The Morning Skate",
        ),
        CardKind::FantasyTrade => (
            "THE BOARDS · TRADE ANALYSIS",
            "Fantasy trade card pages",
            format!("/api/v1/cards/fantasy-trade/{team}"),
            format!("/fantasy/cards/trade/{team}?page=trade-board"),
            format!("/fantasy/cards/trade/{team}?page=insider"),
            "The Trade Board",
        ),
        _ => (
            "THE BENCH · FANTASY ROSTER",
            "Fantasy roster card pages",
            format!("/api/v1/cards/fantasy-roster/{team}"),
            format!("/fantasy/cards/roster/{team}?page=roster"),
            format!("/fantasy/cards/roster/{team}?page=insider"),
            "The Lineup",
        ),
    };
    FantasyRosterCardTemplate {
        active_label,
        title: card.title.clone(),
        subtitle: card.subtitle.clone().unwrap_or_default(),
        fingerprint: card.fingerprint.clone(),
        kicker: kicker.to_string(),
        nav_label: nav_label.to_string(),
        json_href,
        first_href,
        insider_href,
        first_label: first_label.to_string(),
        show_first: show_roster,
        page_title: page
            .display_label
            .clone()
            .unwrap_or_else(|| page.literal_label.clone()),
        page_summary: page.accessible_summary.clone(),
        primary: card.theme.primary.clone().unwrap_or_default(),
        secondary: card.theme.secondary.clone().unwrap_or_default(),
        accent: card.theme.accent.clone().unwrap_or_default(),
        lineup_groups,
        metric_groups,
        decisions,
        player_groups,
        timeline_groups,
        warnings,
        methodology_title,
        limitations,
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|part| part.chars().next())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn project_players(
    rows: &[icelines_core::CardPlayerRowView],
    assets: &BTreeMap<&str, (String, String)>,
) -> Vec<TeamCardPlayer> {
    rows.iter()
        .map(|row| {
            let (asset_url, fallback) = row
                .asset_id
                .as_deref()
                .and_then(|id| assets.get(id))
                .cloned()
                .unwrap_or_default();
            TeamCardPlayer {
                name: row.name.clone(),
                role: row.role.clone().unwrap_or_default(),
                asset_url,
                fallback,
                metrics: row.metrics.iter().map(project_metric).collect(),
            }
        })
        .collect()
}

fn project_metric(metric: &CardMetricView) -> TeamCardMetric {
    TeamCardMetric {
        label: metric.metric.label.clone(),
        value: metric.display_text.clone(),
        comparison: metric
            .comparison
            .as_ref()
            .map(|comparison| format_comparison(comparison, metric.metric.unit))
            .unwrap_or_default(),
    }
}

fn format_comparison(comparison: &CardMetricComparisonView, unit: MetricUnit) -> String {
    let value = match &comparison.delta {
        MetricValue::Decimal(value) => *value,
        MetricValue::Integer(value) => *value as f64,
        _ => return String::new(),
    };
    match unit {
        MetricUnit::Percentage => format!("{value:+.1} pp"),
        MetricUnit::Points => format!("{value:+.1} pts"),
        _ => format!("{value:+.2}"),
    }
}

fn card_error(error: CardStoreError, html: bool) -> Response {
    let status = match error {
        CardStoreError::UnsupportedScenario { .. } => StatusCode::BAD_REQUEST,
        CardStoreError::UnsupportedSeason(_)
        | CardStoreError::UnsupportedTeam(_)
        | CardStoreError::UnsupportedSeasonSimulationTeam(_)
        | CardStoreError::UnsupportedForecastMovementTeam(_)
        | CardStoreError::UnsupportedForecastHistoryTeam(_)
        | CardStoreError::UnsupportedOrganizationWindowTeam(_)
        | CardStoreError::UnsupportedOrganizationWindowFrame(_)
        | CardStoreError::UnsupportedFantasyTeam(_)
        | CardStoreError::UnsupportedFantasyDraftTeam(_)
        | CardStoreError::UnsupportedFantasyMorningTeam(_)
        | CardStoreError::UnsupportedFantasyTradeTeam(_) => StatusCode::NOT_FOUND,
        CardStoreError::InvalidOrganizationWindowCard(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    if html {
        (
            status,
            Html(format!("<h1>Team card unavailable</h1><p>{error}</p>")),
        )
            .into_response()
    } else {
        (
            status,
            axum::Json(serde_json::json!({
                "schema": "card_error.v1",
                "error": error.to_string()
            })),
        )
            .into_response()
    }
}
