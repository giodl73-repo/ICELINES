//! Rinkcraft evidence profile: native HTML and JSON use the shared core view.
use crate::{WebError, WebState};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse, Response},
};
use icelines_core::view_model::rinkcraft_profile::{
    build_rinkcraft_stat_profile, ProfileMetric, RinkcraftProfileRequest, RinkcraftStatProfile,
};
use icelines_core::{identity::PlayerId, model::Season, season_stats::SeasonType};
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct Params {
    #[serde(default, deserialize_with = "optional_season")]
    first_season: Option<u32>,
    #[serde(default, deserialize_with = "optional_season")]
    last_season: Option<u32>,
    #[serde(rename = "type")]
    season_type: Option<String>,
    #[serde(default, deserialize_with = "optional_season")]
    season: Option<u32>,
}
fn optional_season<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    let s = String::deserialize(d)?;
    if s.is_empty() {
        Ok(None)
    } else {
        s.parse().map(Some).map_err(serde::de::Error::custom)
    }
}

#[derive(Template)]
#[template(path = "rinkcraft_atlas.html")]
struct AtlasTemplate {
    active_label: String,
    title: String,
    notes: String,
    cards: Vec<AtlasCard>,
}
struct AtlasCard {
    id: u32,
    name: String,
    class_id: String,
    class_name: String,
    specialty: String,
    summary: String,
    evidence: String,
    featured: bool,
}
#[derive(Template)]
#[template(path = "rinkcraft_numbers.html")]
struct NumbersTemplate {
    active_label: String,
    period: String,
    credit: String,
    sources: Vec<icelines_core::view_model::rinkcraft_numbers::Source>,
    limits: Vec<String>,
    cards: Vec<NumberCard>,
    comparison: Option<ComparisonView>,
    choices: Vec<NumberChoice>,
    right_choices: Vec<NumberChoice>,
    left_id: u32,
}
#[derive(Default, Deserialize)]
pub struct NumberParams {
    left: Option<u32>,
    right: Option<u32>,
}
struct NumberChoice {
    id: u32,
    name: String,
    selected: bool,
}
struct ComparisonView {
    left: String,
    right: String,
    scope: String,
    rows: Vec<ComparisonViewRow>,
}
struct ComparisonViewRow {
    label: String,
    left: String,
    right: String,
    gap: String,
    point_gap: String,
    left_p: String,
    right_p: String,
    left_fill: f64,
    right_fill: f64,
    ranked: bool,
}
struct NumberCard {
    id: u32,
    name: String,
    scope: String,
    metrics: Vec<NumberRow>,
}
struct NumberRow {
    label: String,
    domain: String,
    value: String,
    percentile: String,
    peers: usize,
    meaning: String,
}
fn numerical_catalog(
) -> Result<icelines_core::view_model::rinkcraft_numbers::NumericalCatalog, WebError> {
    let path =
        std::env::var_os("ICELINES_RINKCRAFT_NUMBERS").ok_or_else(|| WebError::NotFound {
            what: "No numerical assessment dataset has been loaded".into(),
            hint: None,
        })?;
    icelines_fetch::rinkcraft_numbers::load_catalog(std::path::Path::new(&path))
        .map_err(WebError::Internal)
}
pub async fn get_numbers_json() -> Result<Response, WebError> {
    Ok(crate::api::json_data_meta(
        "rinkcraft-numbers",
        numerical_catalog()?,
        serde_json::json!({"source_state":"explicit_local_measurement_catalog","live_fetch":false}),
    ))
}
pub async fn get_numerical_comparison(
    Query(params): Query<NumberParams>,
) -> Result<Response, WebError> {
    let bad = |message: &str| WebError::BadParam {
        param: "comparison".into(),
        message: message.into(),
        hint: None,
    };
    let left = params
        .left
        .ok_or_else(|| bad("left player id is required"))?;
    let right = params
        .right
        .ok_or_else(|| bad("right player id is required"))?;
    let comparison = numerical_catalog()?
        .compare(left, right)
        .map_err(|message| bad(&message))?;
    Ok(crate::api::json_data_meta(
        "rinkcraft-numerical-comparison",
        comparison,
        serde_json::json!({"live_fetch":false}),
    ))
}
pub async fn get_numbers(
    State(state): State<WebState>,
    Query(params): Query<NumberParams>,
) -> Result<Response, WebError> {
    let catalog = numerical_catalog()?;
    if [params.left, params.right]
        .into_iter()
        .flatten()
        .any(|id| !catalog.profiles.iter().any(|p| p.player_id == id))
    {
        return Err(WebError::BadParam {
            param: "comparison".into(),
            message: "player has no numerical snapshot".into(),
            hint: None,
        });
    }
    let left = catalog
        .profiles
        .iter()
        .find(|p| Some(p.player_id) == params.left)
        .or_else(|| catalog.profiles.first());
    let right = left.and_then(|left| {
        catalog
            .profiles
            .iter()
            .find(|p| Some(p.player_id) == params.right)
            .or_else(|| {
                catalog
                    .profiles
                    .iter()
                    .find(|p| p.group == left.group && p.player_id != left.player_id)
            })
    });
    let choices = catalog
        .profiles
        .iter()
        .map(|p| NumberChoice {
            id: p.player_id,
            name: format!("{} · {}", p.name, p.group),
            selected: left.is_some_and(|l| l.player_id == p.player_id),
        })
        .collect();
    let right_choices = catalog
        .profiles
        .iter()
        .filter(|p| left.is_some_and(|l| p.group == l.group && p.player_id != l.player_id))
        .map(|p| NumberChoice {
            id: p.player_id,
            name: p.name.clone(),
            selected: right.is_some_and(|r| r.player_id == p.player_id),
        })
        .collect();
    let comparison = left
        .zip(right)
        .map(|(l, r)| catalog.compare(l.player_id, r.player_id))
        .transpose()
        .map_err(|message| WebError::BadParam {
            param: "comparison".into(),
            message,
            hint: None,
        })?
        .map(|c| {
            let rows = c
                .rows
                .iter()
                .filter(|r| c.left.facets.iter().any(|f| f.measurement.key == r.key))
                .map(|r| ComparisonViewRow {
                    label: r.label.clone(),
                    left: comparison_value(&r.unit, r.left, false),
                    right: comparison_value(&r.unit, r.right, false),
                    gap: comparison_value(&r.unit, r.difference, true),
                    point_gap: r
                        .percentile_difference
                        .map(|v| format!("{v:+.1} pp"))
                        .unwrap_or_else(|| "—".into()),
                    left_p: r
                        .left_percentile
                        .map(percentile_label)
                        .unwrap_or_else(|| "Unranked".into()),
                    right_p: r
                        .right_percentile
                        .map(percentile_label)
                        .unwrap_or_else(|| "Unranked".into()),
                    left_fill: r.left_percentile.unwrap_or_default(),
                    right_fill: r.right_percentile.unwrap_or_default(),
                    ranked: r.left_percentile.is_some() && r.right_percentile.is_some(),
                })
                .collect();
            ComparisonView {
                scope: format!(
                    "{} · {} · {}",
                    season_label(c.season),
                    c.left.situation,
                    c.left.group
                ),
                left: c.left.name,
                right: c.right.name,
                rows,
            }
        });
    let cards = catalog
        .profiles
        .iter()
        .map(|p| NumberCard {
            id: p.player_id,
            name: p.name.clone(),
            scope: format!(
                "{} · {} · {} GP · {:.1} minutes · {} eligible peers",
                p.group, p.situation, p.games, p.minutes, p.eligible_peers
            ),
            metrics: p
                .metrics
                .iter()
                .map(|m| NumberRow {
                    label: m.label.clone(),
                    domain: m.domain.clone(),
                    value: m
                        .value
                        .map(|v| {
                            if m.unit == "Pct" {
                                format!("{:.2}%", v * 100.)
                            } else {
                                format!("{v:.2}")
                            }
                        })
                        .unwrap_or_else(|| "—".into()),
                    percentile: m
                        .percentile
                        .map(percentile_label)
                        .unwrap_or_else(|| "Unranked".into()),
                    peers: m.measured_peers,
                    meaning: m.interpretation.clone(),
                })
                .collect(),
        })
        .collect();
    NumbersTemplate {
        left_id: left.map(|p| p.player_id).unwrap_or_default(),
        comparison,
        choices,
        right_choices,
        active_label: state.config.read().await.active_label.clone(),
        period: season_label(catalog.season),
        credit: catalog.source_credit,
        sources: catalog.sources,
        limits: catalog.limitations,
        cards,
    }
    .render()
    .map(|s| Html(s).into_response())
    .map_err(|e| WebError::Internal(e.into()))
}
fn comparison_value(unit: &str, value: Option<f64>, signed: bool) -> String {
    value
        .map(|v| {
            let v = if unit == "Pct" { v * 100. } else { v };
            format!(
                "{}{}",
                if signed {
                    format!("{v:+.2}")
                } else {
                    format!("{v:.2}")
                },
                if unit == "Pct" {
                    if signed {
                        " pp"
                    } else {
                        "%"
                    }
                } else {
                    ""
                }
            )
        })
        .unwrap_or_else(|| "—".into())
}
pub async fn get_atlas(State(state): State<WebState>) -> Result<Response, WebError> {
    let mut cards = Vec::new();
    let mut title = "Rinkcraft player atlas".to_string();
    let mut notes = "No character catalog configured.".to_string();
    if let Some(path) = std::env::var_os("ICELINES_RINKCRAFT_SHEETS") {
        let catalog = icelines_fetch::rinkcraft_character::load_character_catalog(
            std::path::Path::new(&path),
        )
        .map_err(|e| WebError::Internal(anyhow::anyhow!(e)))?;
        if !catalog.showcase_title.is_empty() {
            title = catalog.showcase_title.clone();
        }
        notes = catalog.showcase_notes.clone();
        for a in &catalog.assessments {
            if let Some(s) = catalog
                .sheet(a.player_id, Season(icelines_core::CURRENT_SEASON))
                .map_err(|e| WebError::Internal(anyhow::anyhow!(e)))?
            {
                cards.push(AtlasCard {
                    id: a.player_id.0,
                    name: a.player_name.clone(),
                    class_id: s.primary.id,
                    class_name: s.primary.name,
                    specialty: s.specialty,
                    summary: s.summary,
                    evidence: s.evidence_through,
                    featured: a.featured,
                });
            }
        }
        cards.sort_by(|a, b| b.featured.cmp(&a.featured).then(a.name.cmp(&b.name)));
    }
    AtlasTemplate {
        active_label: state.config.read().await.active_label.clone(),
        title,
        notes,
        cards,
    }
    .render()
    .map(|s| Html(s).into_response())
    .map_err(|e| WebError::Internal(e.into()))
}

fn request(id: u32, p: &Params) -> Result<RinkcraftProfileRequest, WebError> {
    let bad = |name: &str, message: &str| WebError::BadParam {
        param: name.into(),
        message: message.into(),
        hint: None,
    };
    let valid = |s: u32| (1900..=2200).contains(&(s / 10000)) && s % 10000 == s / 10000 + 1;
    for s in [p.first_season, p.last_season, p.season]
        .into_iter()
        .flatten()
    {
        if !valid(s) {
            return Err(bad("season", "use a YYYYZZZZ season id"));
        }
    }
    if p.first_season
        .zip(p.last_season)
        .is_some_and(|(a, b)| a > b)
    {
        return Err(bad("first_season", "must not follow last_season"));
    }
    let ty = match p.season_type.as_deref().unwrap_or("regular") {
        "regular" => SeasonType::Regular,
        "playoff" => SeasonType::Playoff,
        _ => return Err(bad("type", "use regular or playoff")),
    };
    Ok(RinkcraftProfileRequest {
        player_id: PlayerId(id),
        season_type: ty,
        first_season: p.first_season.map(Season),
        last_season: p.last_season.map(Season),
    })
}

async fn profile(
    state: &WebState,
    req: RinkcraftProfileRequest,
    character_season: Option<Season>,
) -> Result<RinkcraftStatProfile, WebError> {
    let root = state.snapshots_root.as_ref().clone();
    // Preserve the active (possibly newer than bundled) window, as the CLI does.
    let mut repo = icelines_core::stats_repository::StatsRepository::with_lru_cap(80);
    {
        let shared = state.repo.read().await;
        if let Some(identity) = shared.identity(req.player_id) {
            repo.upsert_identity(identity.clone())
                .map_err(|e| WebError::Internal(e.into()))?;
            for row in shared.iter_stats().filter(|s| s.player_id == req.player_id) {
                repo.upsert_stats(row.clone())
                    .map_err(|e| WebError::Internal(e.into()))?;
            }
        }
    }
    tokio::task::spawn_blocking(move || -> anyhow::Result<RinkcraftStatProfile> {
        let root = root.unwrap_or_else(icelines_fetch::snapshot::SnapshotStore::default_root);
        let store = icelines_fetch::snapshot::SnapshotStore::new(root);
        icelines_fetch::stats_loader::load_player_career_into_repo_with_store(
            &mut repo,
            req.player_id,
            &store,
        )?;
        let sources = icelines_fetch::rinkcraft_profile::hydrate_profile_reports(
            &mut repo,
            req.player_id,
            req.season_type,
            req.first_season,
            req.last_season,
            &store,
        )?;
        let mut profile = build_rinkcraft_stat_profile(&repo, req)?;
        profile.source_windows = sources;
        icelines_fetch::rinkcraft_profile::attach_attribute_seasons(&mut profile, &store)?;
        if req.season_type == SeasonType::Regular {
            if let Some(path) = std::env::var_os("ICELINES_RINKCRAFT_NUMBERS") {
                let catalog =
                    icelines_fetch::rinkcraft_numbers::load_catalog(std::path::Path::new(&path))?;
                if profile
                    .included_seasons
                    .iter()
                    .any(|s| s.0 == catalog.season)
                    && character_season.is_none_or(|s| s.0 == catalog.season)
                {
                    profile.numerical_evidence = catalog
                        .card(req.player_id.0, req.last_season.map(|s| s.0))
                        .map_err(anyhow::Error::msg)?;
                }
            }
        }
        if let Some(path) = std::env::var_os("ICELINES_RINKCRAFT_SHEETS") {
            let catalog = icelines_fetch::rinkcraft_character::load_character_catalog(
                std::path::Path::new(&path),
            )
            .map_err(anyhow::Error::msg)?;
            let season = character_season
                .or(req.last_season)
                .or(profile.latest_available.as_ref().map(|s| s.season))
                .unwrap_or(Season(icelines_core::CURRENT_SEASON));
            profile.character_sheet = catalog
                .sheet(req.player_id, season)
                .map_err(anyhow::Error::msg)?;
        }
        Ok(profile)
    })
    .await
    .map_err(|e| WebError::Internal(e.into()))?
    .map_err(|e| {
        if matches!(
            e.downcast_ref(),
            Some(
                icelines_core::view_model::rinkcraft_profile::RinkcraftProfileError::UnknownPlayer(
                    _
                )
            )
        ) {
            WebError::UnknownPlayer {
                query: req.player_id.to_string(),
                hint: None,
            }
        } else {
            WebError::Internal(e)
        }
    })
}

pub async fn get_json(
    State(state): State<WebState>,
    Path(id): Path<u32>,
    Query(p): Query<Params>,
) -> Result<Response, WebError> {
    let view = profile(&state, request(id, &p)?, p.season.map(Season)).await?;
    Ok(crate::api::json_data_meta(
        "rinkcraft-profile",
        view,
        serde_json::json!({"source_state":"local_loaded_windows","live_fetch":false}),
    ))
}

#[derive(Template)]
#[template(path = "rinkcraft.html")]
struct ProfileTemplate {
    numerical: Option<EvidenceView>,
    stat_line: Vec<StatCell>,
    feature_cards: Vec<FeatureCard>,
    card_stats_context: String,
    attribute_cards: Vec<AttributeCard>,
    attribute_context: String,
    active_label: String,
    profile: RinkcraftStatProfile,
    id: u32,
    season_type: String,
    first: String,
    last: String,
    selected: String,
    label: String,
    rows: Vec<MetricRow>,
    latest: String,
    sources_json: String,
    changes: Vec<ChangeSection>,
    absent_json: String,
}
struct EvidenceView {
    scope: String,
    leading: String,
    facets: Vec<FacetView>,
    metrics: Vec<NumberRow>,
    credit: String,
}
struct FacetView {
    label: String,
    key: String,
    value: String,
    rank: String,
    fill: f64,
    ranked: bool,
    peers: usize,
    meaning: String,
}
fn measurement_value(m: &icelines_core::view_model::rinkcraft_numbers::Measurement) -> String {
    m.value
        .map(|v| {
            if m.unit == "Pct" {
                format!("{:.2}%", v * 100.)
            } else {
                format!("{v:.2}")
            }
        })
        .unwrap_or_else(|| "—".into())
}
fn evidence_view(c: &icelines_core::view_model::rinkcraft_numbers::EvidenceCard) -> EvidenceView {
    EvidenceView {
        scope: format!(
            "{} · {} · {} GP · {:.1} minutes · {} peers",
            season_label(c.season),
            c.situation,
            c.games,
            c.minutes,
            c.eligible_peers
        ),
        leading: c
            .leading
            .iter()
            .filter_map(|key| {
                c.facets
                    .iter()
                    .find(|f| f.measurement.key == *key)
                    .map(|f| f.label.clone())
            })
            .collect::<Vec<_>>()
            .join(" · "),
        facets: c
            .facets
            .iter()
            .map(|f| FacetView {
                label: f.label.clone(),
                key: f.measurement.key.clone(),
                value: format!(
                    "{}{}",
                    measurement_value(&f.measurement),
                    if f.measurement.unit == "Per60" {
                        " /60"
                    } else {
                        ""
                    }
                ),
                rank: f
                    .measurement
                    .percentile
                    .map(percentile_label)
                    .unwrap_or_else(|| "Unranked".into()),
                fill: f.measurement.percentile.unwrap_or_default(),
                ranked: f.measurement.percentile.is_some(),
                peers: f.measurement.measured_peers,
                meaning: f.measurement.interpretation.clone(),
            })
            .collect(),
        metrics: c
            .metrics
            .iter()
            .map(|m| NumberRow {
                label: m.label.clone(),
                domain: m.domain.clone(),
                value: measurement_value(m),
                percentile: m
                    .percentile
                    .map(percentile_label)
                    .unwrap_or_else(|| "Unranked".into()),
                peers: m.measured_peers,
                meaning: m.interpretation.clone(),
            })
            .collect(),
        credit: c.source_credit.clone(),
    }
}
struct AttributeCard {
    percentile_short: String,
    peer_short: String,
    percentile_fill: f64,
    ranked: bool,
    label: String,
    layer: String,
    value: String,
    level: String,
    percentile: String,
    evidence: String,
    explanation: String,
    steps: Vec<bool>,
    path: String,
    points: Vec<ChartPoint>,
    first: String,
    last: String,
    history: Vec<String>,
}
struct ChartPoint {
    x: f64,
    y: f64,
    title: String,
}
fn attribute_cards(
    view: &RinkcraftStatProfile,
    selected: Option<u32>,
) -> (Vec<AttributeCard>, String) {
    let current = selected
        .and_then(|s| view.attribute_seasons.iter().find(|r| r.season.0 == s))
        .or_else(|| view.attribute_seasons.last());
    let Some(current) = current else {
        return (Vec::new(), "Attribute assessment unavailable".into());
    };
    let context=format!("{} · {} · {} GP · {} · Same-season, same-competition peers; ≥ {} GP. Descriptive bands, not skill grades.",
        season_label(current.season.0),current.group,current.games,current.strength_scope,current.minimum_games);
    let first = view.attribute_seasons.first().unwrap().season.0 / 10000;
    let last = view.attribute_seasons.last().unwrap().season.0 / 10000;
    let cards=current.attributes.iter().map(|a|{
        let mut path=String::new();let mut points=Vec::new();let mut history=Vec::new();let mut previous=None;
        for s in &view.attribute_seasons {
            let Some(at)=s.attributes.iter().find(|m|m.key==a.key) else{previous=None;continue;};
            let raw=attribute_value(&at.unit,at.value);
            history.push(format!("{}: {} · band {} · {} GP · {} measured / {} eligible {}",season_label(s.season.0),raw,at.band.map(|v|v.to_string()).unwrap_or_else(||"unknown".into()),s.games,at.measured_peers,s.eligible_peers,s.group));
            if let Some(band)=at.band {
                let year=s.season.0/10000;let x=24.0+252.0*f64::from(year-first)/f64::from((last-first).max(1));let y=94.0-f64::from(band-1)*18.0;
                if previous.is_some_and(|p|p+1==year){path.push_str(&format!(" H {x:.2} V {y:.2}"));}else{path.push_str(&format!(" M {x:.2} {y:.2}"));}
                points.push(ChartPoint{x,y,title:format!("{} · band {} · {}",season_label(s.season.0),band,raw)});previous=Some(year);
            }else{previous=None;}
        }
        AttributeCard{percentile_short:a.percentile.map(percentile_label).unwrap_or_else(||"—".into()),peer_short:format!("n={}/{}",a.measured_peers,current.eligible_peers),percentile_fill:a.percentile.unwrap_or(0.0),ranked:a.percentile.is_some(),label:a.label.clone(),layer:a.layer.clone(),value:attribute_value(&a.unit,a.value),
            level:a.band.map(|v|format!("{v} / 5")).unwrap_or_else(||"Unranked".into()),
            percentile:a.percentile.map(|p|format!("{p:.1} percentile")).unwrap_or_else(||a.state.replace('_'," ")),
            evidence:format!("{} measured / {} eligible {} · {} GP",a.measured_peers,current.eligible_peers,current.group,current.games),
            explanation:format!("{} Formula: {}. Reference minimum: {} measured peers. Reference fingerprint: {}. Protocol: {}.",a.interpretation,a.formula,current.minimum_peers,a.reference_sha256,current.protocol),
            steps:(1..=5).map(|n|a.band.is_some_and(|b|n<=b)).collect(),path,points,first:first.to_string(),last:last.to_string(),history}
    }).collect();
    (cards, context)
}
fn attribute_value(unit: &str, value: Option<f64>) -> String {
    match value {
        None => "Unknown".into(),
        Some(v) => match unit {
            "Pct" => format!("{:.2}%", v * 100.0),
            "Seconds" => format!("{:.1} min / game", v / 60.0),
            "Count" => format!("{v:.0} starts"),
            _ => format!("{v:.2} / game"),
        },
    }
}
struct StatCell {
    key: String,
    label: String,
    value: String,
}
struct FeatureCard {
    name: String,
    detail: String,
    evidence: String,
    stats: Vec<FeatureReadout>,
    gaps: Vec<String>,
}
struct FeatureReadout {
    key: String,
    label: String,
    value: String,
    percentile: String,
    peers: String,
    relation: String,
}
fn stat_line(view: &RinkcraftStatProfile, selected: Option<u32>) -> Vec<StatCell> {
    let row = selected
        .and_then(|s| view.timeline.iter().find(|r| r.season.0 == s))
        .or(view.latest_available.as_ref());
    let Some(row) = row else { return Vec::new() };
    let goalie = row.position == icelines_core::model::Position::Goalie;
    let slots = if goalie {
        vec![
            ("goalie-starts", "GS"),
            ("wins", "W"),
            ("save-pct", "SV%"),
            ("gaa", "GAA"),
            ("shots-against", "SA"),
            ("saves", "SV"),
            ("goals-against", "GA"),
            ("shutouts", "SO"),
        ]
    } else {
        vec![
            ("goals", "G"),
            ("assists", "A"),
            ("points", "PTS"),
            ("shots", "SOG"),
            ("hits", "HIT"),
            ("blocked-shots", "BLK"),
            ("pp-points", "PP PTS"),
            ("total-toi-per-game", "TOI/G"),
        ]
    };
    let mut cells = vec![StatCell {
        key: "games".into(),
        label: "GP".into(),
        value: row.games.to_string(),
    }];
    cells.extend(slots.into_iter().map(|(key, label)| {
        StatCell {
            key: key.into(),
            label: label.into(),
            value: row
                .metrics
                .iter()
                .find(|m| m.key == key)
                .map(compact_metric)
                .unwrap_or_else(|| "—".into()),
        }
    }));
    cells
}
fn percentile_label(p: f64) -> String {
    format!("P{:.1}", (p * 10.0).round() / 10.0)
}
fn compact_metric(m: &ProfileMetric) -> String {
    let Some(v) = m.value else {
        return "—".into();
    };
    match m.unit.as_str() {
        "Count" => format!("{v:.0}"),
        "Pct" => format!("{:.2}%", v * 100.0),
        "Seconds" => {
            let s = v.round() as u64;
            format!("{}:{:02}", s / 60, s % 60)
        }
        _ => format!("{v:.2}"),
    }
}
fn feature_cards(view: &RinkcraftStatProfile, selected: Option<u32>) -> Vec<FeatureCard> {
    let Some(sheet) = &view.character_sheet else {
        return Vec::new();
    };
    let assessment = selected
        .and_then(|s| view.attribute_seasons.iter().find(|a| a.season.0 == s))
        .or_else(|| view.attribute_seasons.last());
    sheet
        .features
        .iter()
        .map(|f| FeatureCard {
            name: f.name.clone(),
            detail: f.detail.clone(),
            evidence: format!("{} · {}", f.evidence_type, f.source_ids.join(", ")),
            gaps: f.unmeasured_stats.clone(),
            stats: f
                .stat_evidence
                .iter()
                .map(|link| {
                    let attribute = assessment
                        .and_then(|s| s.attributes.iter().find(|a| a.key == link.attribute_key));
                    FeatureReadout {
                        key: link.attribute_key.clone(),
                        label: attribute
                            .map(|a| a.label.clone())
                            .unwrap_or_else(|| link.attribute_key.replace('-', " ")),
                        value: attribute
                            .map(|a| attribute_value(&a.unit, a.value))
                            .unwrap_or_else(|| "—".into()),
                        percentile: attribute
                            .and_then(|a| a.percentile)
                            .map(percentile_label)
                            .unwrap_or_else(|| "Unranked".into()),
                        peers: attribute
                            .map(|a| {
                                format!(
                                    "n={}/{}",
                                    a.measured_peers,
                                    assessment.map(|s| s.eligible_peers).unwrap_or_default()
                                )
                            })
                            .unwrap_or_else(|| "No reference".into()),
                        relation: link.relation.label().into(),
                    }
                })
                .collect(),
        })
        .collect()
}
struct MetricRow {
    label: String,
    key: String,
    value: String,
    status: String,
    evidence: String,
}
struct ChangeSection {
    label: String,
    games: String,
    rows: Vec<ChangeRow>,
}
struct ChangeRow {
    label: String,
    previous: String,
    latest: String,
    difference: String,
}
fn format_metric(m: &ProfileMetric) -> String {
    let Some(v) = m.value else {
        return "—".into();
    };
    if m.unit == "Pct" {
        format!("{:.2}%", v * 100.0)
    } else if m.unit == "Count" {
        format!("{v:.0}")
    } else if m.unit == "Seconds" {
        format!("{v:.1} s")
    } else {
        format!("{v:.3}")
    }
}
fn season_label(s: u32) -> String {
    format!("{}–{}", s / 10000, s % 100)
}

pub async fn get_html(
    State(state): State<WebState>,
    Path(id): Path<u32>,
    Query(p): Query<Params>,
) -> Result<Response, WebError> {
    let req = request(id, &p)?;
    let view = profile(&state, req, p.season.map(Season)).await?;
    let metrics = match p.season {
        Some(s) => {
            &view
                .timeline
                .iter()
                .find(|r| r.season.0 == s)
                .ok_or_else(|| WebError::NotFound {
                    what: "season absent from selected player window".into(),
                    hint: None,
                })?
                .metrics
        }
        None => &view.summary,
    };
    let rows = metrics
        .iter()
        .map(|m| MetricRow {
            label: m.label.clone(),
            key: m.key.clone(),
            value: format_metric(m),
            status: format!("{:?}", m.status),
            evidence: format!(
                "{} · {} · {} observed seasons · missing: {:?}{}",
                m.unit,
                m.aggregation,
                m.observed_seasons,
                m.missing_seasons,
                m.ratio
                    .as_ref()
                    .map(|r| format!(
                        " · {} ({}) / {} ({}) × {}",
                        r.numerator_key, r.numerator, r.denominator_key, r.denominator, r.scale
                    ))
                    .unwrap_or_default()
            ),
        })
        .collect();
    let (attribute_cards, attribute_context) = attribute_cards(&view, p.season);
    let card_stats_context = view
        .attribute_seasons
        .iter()
        .find(|s| Some(s.season.0) == p.season)
        .or_else(|| view.attribute_seasons.last())
        .map(|s| {
            format!(
                "{} · {} GP · {} · all strengths",
                season_label(s.season.0),
                s.games,
                s.group
            )
        })
        .unwrap_or_else(|| "Statistical snapshot unavailable".into());
    let template = ProfileTemplate {
        numerical: view.numerical_evidence.as_ref().map(evidence_view),
        stat_line: stat_line(&view, p.season),
        feature_cards: feature_cards(&view, p.season),
        card_stats_context,
        attribute_cards,
        attribute_context,
        active_label: state.config.read().await.active_label.clone(),
        id,
        season_type: req.season_type.to_string(),
        sources_json: serde_json::to_string_pretty(&view.source_windows).unwrap_or_default(),
        changes: view
            .changes
            .iter()
            .map(|c| ChangeSection {
                label: format!("{} → {}", season_label(c.from.0), season_label(c.to.0)),
                games: format!(
                    "{} → {} games; consecutive seasons: {}",
                    c.previous_games, c.latest_games, c.consecutive
                ),
                rows: c
                    .metrics
                    .iter()
                    .filter(|m| m.difference.is_some())
                    .filter_map(|m| {
                        let base = view.summary.iter().find(|s| s.key == m.key)?;
                        let display = |value| {
                            format_metric(&ProfileMetric {
                                value,
                                ..base.clone()
                            })
                        };
                        Some(ChangeRow {
                            label: base.label.clone(),
                            previous: display(m.previous),
                            latest: display(m.latest),
                            difference: display(m.difference),
                        })
                    })
                    .collect(),
            })
            .collect(),
        absent_json: serde_json::to_string(&view.absent_seasons_in_requested_range)
            .unwrap_or_default(),
        first: p.first_season.map(|v| v.to_string()).unwrap_or_default(),
        last: p.last_season.map(|v| v.to_string()).unwrap_or_default(),
        selected: p.season.map(|v| v.to_string()).unwrap_or_default(),
        label: p
            .season
            .map(season_label)
            .unwrap_or_else(|| "Selected window summary".into()),
        latest: view
            .latest_available
            .as_ref()
            .map(|v| season_label(v.season.0))
            .unwrap_or_else(|| "Unavailable".into()),
        profile: view,
        rows,
    };
    template
        .render()
        .map(|s| Html(s).into_response())
        .map_err(|e| WebError::Internal(e.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rinkcraft_rejects_invalid_windows_and_season_types() {
        assert!(request(
            1,
            &Params {
                season_type: Some("typo".into()),
                ..Default::default()
            }
        )
        .is_err());
        assert!(request(
            1,
            &Params {
                first_season: Some(20252026),
                last_season: Some(20232024),
                ..Default::default()
            }
        )
        .is_err());
        assert!(request(
            1,
            &Params {
                season: Some(20250000),
                ..Default::default()
            }
        )
        .is_err());
    }
    #[test]
    fn rinkcraft_formats_null_and_fractional_percentages() {
        assert_eq!(percentile_label(56.25), "P56.3");
        let mut m:ProfileMetric=serde_json::from_value(serde_json::json!({"key":"save-pct","label":"SV%","unit":"Pct","value":0.915,"status":"available","aggregation":"catalog_read","observed_seasons":1,"missing_seasons":[],"report_source":null,"ratio":null})).unwrap();
        assert_eq!(format_metric(&m), "91.50%");
        m.value = None;
        assert_eq!(format_metric(&m), "—");
    }
}
