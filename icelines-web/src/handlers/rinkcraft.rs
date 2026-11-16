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
    seasons: Vec<NumberChoice>,
    season: u32,
    credit: String,
    sources: Vec<icelines_core::view_model::rinkcraft_numbers::Source>,
    limits: Vec<String>,
    cards: Vec<NumberCard>,
    comparison: Option<ComparisonView>,
    choices: Vec<NumberChoice>,
    right_choices: Vec<NumberChoice>,
    left_id: u32,
    right_id: u32,
}
#[derive(Default, Deserialize)]
pub struct NumberParams {
    #[serde(default, deserialize_with = "optional_season")]
    season: Option<u32>,
    left: Option<u32>,
    right: Option<u32>,
}
struct NumberChoice {
    id: u32,
    name: String,
    selected: bool,
}
struct ComparisonView {
    context: Option<ContextComparisonView>,
    identities: Vec<ComparisonIdentityView>,
    identity_note: String,
    contrast: Option<ContrastView>,
    style: Option<StyleComparisonView>,
    left: String,
    right: String,
    scope: String,
    rows: Vec<ComparisonViewRow>,
}
struct ContextView {
    title: String,
    note: String,
    coverage: String,
    groups: Vec<ContextGroupView>,
}
struct ContextGroupView {
    label: String,
    note: String,
    rows: Vec<ContextRowView>,
}
struct ContextRowView {
    key: String,
    label: String,
    value: String,
    rank: String,
    peers: String,
    question: String,
    limit: String,
    formula: String,
    components: String,
    reference: String,
    state: String,
}
struct ContextComparisonView {
    title: String,
    note: String,
    coverage: String,
    groups: Vec<ContextComparisonGroupView>,
}
struct ContextComparisonGroupView {
    label: String,
    note: String,
    rows: Vec<ContextComparisonRowView>,
}
struct ContextComparisonRowView {
    key: String,
    label: String,
    left: String,
    right: String,
    left_p: String,
    right_p: String,
    gap: String,
    point_gap: String,
    limit: String,
}
fn context_value(m: Option<&icelines_core::view_model::rinkcraft_numbers::Measurement>) -> String {
    m.map(|m| {
        format!(
            "{}{}",
            measurement_value(m),
            match m.unit.as_str() {
                "Per60" => " /60",
                "Goals" => " goals",
                _ => "",
            }
        )
    })
    .unwrap_or_else(|| "Unavailable".into())
}
fn context_rank(m: Option<&icelines_core::view_model::rinkcraft_numbers::Measurement>) -> String {
    m.filter(|m| m.state == "ranked" && m.value.is_some())
        .and_then(|m| m.percentile)
        .map(percentile_label)
        .unwrap_or_else(|| "Unranked".into())
}
fn context_view(c: &icelines_core::view_model::rinkcraft_context::ContextEvidence) -> ContextView {
    ContextView {title:c.title.clone(),note:c.note.clone(),coverage:format!("{}/{} values available · {}/{} ranked. Higher means a larger recorded value, not always better.",c.measured_facets,c.facet_count,c.ranked_facets,c.facet_count),
        groups:c.groups.iter().map(|g|ContextGroupView {label:g.label.clone(),note:g.note.clone(),rows:g.facets.iter().map(|f| {let m=f.measurement.as_ref();ContextRowView {
            key:f.key.clone(),label:f.label.clone(),value:context_value(m),rank:context_rank(m),peers:m.map(|m|format!("n={}",m.measured_peers)).unwrap_or_else(||"No reference".into()),
            question:f.explanation.question.clone(),limit:f.explanation.limit.clone(),formula:m.map(|m|m.formula.clone()).unwrap_or_else(||"Unavailable".into()),
            components:m.map(|m|format!("{} / {} × {}",m.numerator.map(|v|v.to_string()).unwrap_or_else(||"unavailable".into()),m.denominator.map(|v|v.to_string()).unwrap_or_else(||"unavailable".into()),m.scale)).unwrap_or_else(||"Unavailable".into()),
            reference:m.map(|m|m.reference_sha256.clone()).unwrap_or_else(||"Unavailable".into()),state:m.map(|m|m.state.clone()).unwrap_or_else(||"missing_measurement".into()),
        }}).collect()}).collect() }
}
fn context_comparison_view(
    c: &icelines_core::view_model::rinkcraft_numbers::NumericalComparison,
) -> Option<ContextComparisonView> {
    let (a, b) = c
        .left
        .context_evidence
        .as_ref()
        .zip(c.right.context_evidence.as_ref())?;
    Some(ContextComparisonView {
        title: a.title.clone(),
        note: a.note.clone(),
        coverage: format!(
            "Left {}/{} ranked · right {}/{} ranked. Raw-value ranks are not inverted or combined.",
            a.ranked_facets, a.facet_count, b.ranked_facets, b.facet_count
        ),
        groups: a
            .groups
            .iter()
            .map(|g| ContextComparisonGroupView {
                label: g.label.clone(),
                note: g.note.clone(),
                rows: g
                    .facets
                    .iter()
                    .map(|f| {
                        let left = f.measurement.as_ref();
                        let right = c.right.metrics.iter().find(|m| m.key == f.key);
                        let row = c.rows.iter().find(|r| r.key == f.key);
                        let unit = left.or(right).map(|m| m.unit.as_str()).unwrap_or("");
                        ContextComparisonRowView {
                            key: f.key.clone(),
                            label: f.label.clone(),
                            left: context_value(left),
                            right: context_value(right),
                            left_p: context_rank(left),
                            right_p: context_rank(right),
                            gap: style_comparison_value(unit, row.and_then(|r| r.difference), true),
                            point_gap: row
                                .filter(|_| {
                                    left.zip(right).is_some_and(|(a, b)| {
                                        a.state == "ranked"
                                            && b.state == "ranked"
                                            && a.value.is_some()
                                            && b.value.is_some()
                                    })
                                })
                                .and_then(|r| r.percentile_difference)
                                .map(|v| format!("{v:+.1} pp"))
                                .unwrap_or_else(|| "—".into()),
                            limit: f.explanation.limit.clone(),
                        }
                    })
                    .collect(),
            })
            .collect(),
    })
}
struct ComparisonIdentityView {
    name: String,
    sheet: Option<icelines_core::view_model::rinkcraft_character::CharacterSheet>,
    unavailable_label: String,
}
fn comparison_character_catalog() -> (
    Option<icelines_core::view_model::rinkcraft_character::CharacterCatalog>,
    String,
) {
    let Some(path) = std::env::var_os("ICELINES_RINKCRAFT_SHEETS") else {
        return (
            None,
            "No documentary class catalog loaded. Statistics do not assign classes.".into(),
        );
    };
    match icelines_fetch::rinkcraft_character::load_character_catalog(std::path::Path::new(&path)) {
        Ok(c) => (Some(c), "Documentary class hypotheses are selected for this source season. Numerical differences do not establish class fit.".into()),
        Err(_) => (None, "Documentary class catalog unavailable. The numerical comparison remains available.".into()),
    }
}
struct ContrastView {
    facet_count: usize,
    ranked_facets: usize,
    equal_rank_facets: usize,
    highlights: Vec<ContrastHighlightView>,
    omitted: Vec<icelines_core::view_model::rinkcraft_contrast::ContrastOmission>,
    selection_note: String,
}
struct ContrastHighlightView {
    key: String,
    label: String,
    gap: String,
    raw_gap: String,
    left: String,
    right: String,
    left_p: String,
    right_p: String,
    limit: String,
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
struct StyleComparisonView {
    rows: Vec<StyleComparisonRowView>,
    gaps: Vec<icelines_core::view_model::rinkcraft_style::StyleGap>,
}
struct StyleComparisonRowView {
    key: String,
    label: String,
    left: String,
    right: String,
    left_scope: String,
    right_scope: String,
    difference: String,
    status: String,
    reason: String,
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
fn numerical_source(
) -> Result<icelines_core::view_model::rinkcraft_history::NumericalHistory, WebError> {
    let path = std::env::var_os("ICELINES_RINKCRAFT_HISTORY")
        .or_else(|| std::env::var_os("ICELINES_RINKCRAFT_NUMBERS"))
        .ok_or_else(|| WebError::NotFound {
            what: "No numerical evidence source loaded".into(),
            hint: None,
        })?;
    icelines_fetch::rinkcraft_history::load(std::path::Path::new(&path)).map_err(WebError::Internal)
}
fn numerical_catalog(
    season: Option<u32>,
) -> Result<icelines_core::view_model::rinkcraft_history::CatalogView, WebError> {
    numerical_source()?
        .catalog_view(season)
        .map_err(|message| WebError::BadParam {
            param: "season".into(),
            message,
            hint: None,
        })
}
pub async fn get_numbers_json(Query(params): Query<NumberParams>) -> Result<Response, WebError> {
    let view = numerical_catalog(params.season)?;
    Ok(crate::api::json_data_meta(
        "rinkcraft-numbers",
        view.catalog,
        serde_json::json!({"source_state":"explicit_local_measurement_catalog","live_fetch":false,"available_seasons":view.seasons}),
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
    let comparison = numerical_catalog(params.season)?
        .catalog
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
    let view = numerical_catalog(params.season)?;
    let catalog = view.catalog;
    let (characters, identity_note) = comparison_character_catalog();
    if [params.left, params.right]
        .into_iter()
        .flatten()
        .any(|id| !catalog.profiles.iter().any(|p| p.player_id == id))
    {
        return Err(WebError::BadParam {
            param: "comparison".into(),
            message: format!("A requested player has no numerical snapshot in {}. Choose a different source season or player; no fallback was used.", season_label(catalog.season)),
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
                context: context_comparison_view(&c),
                identities: [
                    (c.left.player_id, &c.left.name),
                    (c.right.player_id, &c.right.name),
                ]
                .into_iter()
                .map(|(id, name)| ComparisonIdentityView {
                    name: name.clone(),
                    sheet: characters.as_ref().and_then(|catalog| {
                        catalog.sheet(PlayerId(id), Season(c.season)).ok().flatten()
                    }),
                    unavailable_label: if characters.is_some() {
                        "Unscouted for this source season"
                    } else {
                        "Class dossier unavailable"
                    }
                    .into(),
                })
                .collect(),
                identity_note: identity_note.clone(),
                contrast: c.contrast.as_ref().map(|s| ContrastView {
                    facet_count: s.facet_count,
                    ranked_facets: s.ranked_facets,
                    equal_rank_facets: s.equal_rank_facets,
                    selection_note: s.selection_note.clone(),
                    omitted: s.omitted.clone(),
                    highlights: s
                        .highlights
                        .iter()
                        .map(|h| {
                            let r = &h.measurement;
                            ContrastHighlightView {
                                key: r.key.clone(),
                                label: h.label.clone(),
                                gap: r
                                    .percentile_difference
                                    .map(|v| format!("{v:+.1} pp"))
                                    .unwrap_or_else(|| "—".into()),
                                raw_gap: style_comparison_value(&r.unit, r.difference, true),
                                left: style_comparison_value(&r.unit, r.left, false),
                                right: style_comparison_value(&r.unit, r.right, false),
                                left_p: r
                                    .left_percentile
                                    .map(percentile_label)
                                    .unwrap_or_else(|| "Unranked".into()),
                                right_p: r
                                    .right_percentile
                                    .map(percentile_label)
                                    .unwrap_or_else(|| "Unranked".into()),
                                limit: h.limit.clone(),
                            }
                        })
                        .collect(),
                }),
                style: c.style.as_ref().map(|s| StyleComparisonView {
                    rows: s
                        .rows
                        .iter()
                        .map(|r| StyleComparisonRowView {
                            key: r.key.clone(),
                            label: r.label.clone(),
                            left: r
                                .left
                                .as_ref()
                                .map(|m| style_comparison_value(&m.unit, Some(m.value), false))
                                .unwrap_or_else(|| "Unmeasured".into()),
                            right: r
                                .right
                                .as_ref()
                                .map(|m| style_comparison_value(&m.unit, Some(m.value), false))
                                .unwrap_or_else(|| "Unmeasured".into()),
                            left_scope: r
                                .left
                                .as_ref()
                                .map(|m| m.scope.clone())
                                .unwrap_or_default(),
                            right_scope: r
                                .right
                                .as_ref()
                                .map(|m| m.scope.clone())
                                .unwrap_or_default(),
                            difference: style_comparison_value(&r.unit, r.difference, true),
                            status: match r.status.as_str() {
                                "matched" => "Matched definition",
                                "left_only" => "Left player only",
                                "right_only" => "Right player only",
                                _ => "Different context",
                            }
                            .into(),
                            reason: r.reason.clone(),
                        })
                        .collect(),
                    gaps: s.shared_gaps.clone(),
                }),
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
        season: catalog.season,
        seasons: view
            .seasons
            .into_iter()
            .map(|id| NumberChoice {
                id,
                name: season_label(id),
                selected: id == catalog.season,
            })
            .collect(),
        left_id: left.map(|p| p.player_id).unwrap_or_default(),
        right_id: right.map(|p| p.player_id).unwrap_or_default(),
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
fn style_comparison_value(unit: &str, value: Option<f64>, signed: bool) -> String {
    if value.is_none() {
        return "—".into();
    }
    let suffix = match unit {
        "Per60" => " /60",
        "Mph" => " mph",
        "PerGame" => " /game",
        "MilesPerGame" => " mi/game",
        _ => "",
    };
    format!("{}{}", comparison_value(unit, value, signed), suffix)
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
            if let Some(path) = std::env::var_os("ICELINES_RINKCRAFT_HISTORY")
                .or_else(|| std::env::var_os("ICELINES_RINKCRAFT_NUMBERS"))
            {
                let history = icelines_fetch::rinkcraft_history::load(std::path::Path::new(&path))?;
                let ceiling = character_season
                    .map(|s| s.0)
                    .or(req.last_season.map(|s| s.0))
                    .unwrap_or(icelines_core::CURRENT_SEASON);
                let ceiling = ceiling.min(req.last_season.map(|s| s.0).unwrap_or(ceiling));
                profile.numerical_history = history
                    .card(
                        req.player_id.0,
                        req.first_season.map(|s| s.0),
                        Some(ceiling),
                    )
                    .map_err(anyhow::Error::msg)?;
                let selected = history
                    .snapshot(
                        req.player_id.0,
                        req.first_season.map(|s| s.0),
                        Some(ceiling),
                        character_season.map(|s| s.0),
                    )
                    .map_err(anyhow::Error::msg)?;
                profile.numerical_evidence = selected.card;
                profile.numerical_snapshot = Some(selected.selection);
            }
        }
        if let Some(path) = std::env::var_os("ICELINES_RINKCRAFT_SHEETS") {
            if let Ok(catalog) = icelines_fetch::rinkcraft_character::load_character_catalog(
                std::path::Path::new(&path),
            ) {
                let season = character_season
                    .or(req.last_season)
                    .or(profile.latest_available.as_ref().map(|s| s.season))
                    .unwrap_or(Season(icelines_core::CURRENT_SEASON));
                profile.character_sheet = catalog
                    .sheet(req.player_id, season)
                    .map_err(anyhow::Error::msg)?;
            }
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
    front_position: String,
    documentary_catalog_available: bool,
    comparison_choices: Vec<NumberChoice>,
    history: Option<HistoryView>,
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
    context: Option<ContextView>,
    sources: Vec<icelines_core::view_model::rinkcraft_numbers::Source>,
    season: u32,
    style: Option<StyleView>,
    scope: String,
    leading: String,
    facets: Vec<FacetView>,
    metrics: Vec<NumberRow>,
    credit: String,
}
struct HistoryView {
    assessment: Option<AssessmentView>,
    sources: Vec<icelines_core::view_model::rinkcraft_numbers::Source>,
    scope: String,
    seasons: Vec<String>,
    rows: Vec<HistoryRowView>,
    coverage: Vec<String>,
    limits: Vec<String>,
}
struct HistoryRowView {
    key: String,
    label: String,
    values: Vec<String>,
    repeat: String,
    reason: String,
}
struct AssessmentView {
    scope: String,
    repeated: Vec<RepeatedView>,
    repeated_reason: String,
    recent: Option<RecentView>,
    recent_reason: String,
    shortlist: String,
}
struct RepeatedView {
    key: String,
    label: String,
    count: String,
    range: String,
    latest: String,
}
struct RecentView {
    key: String,
    label: String,
    period: String,
    delta: String,
    ranks: String,
    raw: String,
}
fn assessment_view(
    h: &icelines_core::view_model::rinkcraft_history::HistoryCard,
) -> Option<AssessmentView> {
    h.assessment.as_ref().map(|a| AssessmentView {
        scope: format!(
            "MoneyPuck.com · {} → {} · {}/{} supplied seasons reported · Last reported: {}",
            season_label(a.first_season),
            season_label(a.last_season),
            a.reported_seasons,
            a.supplied_seasons,
            h.seasons
                .iter()
                .rev()
                .find(|s| s.group.is_some())
                .map(|s| format!(
                    "{}: {} · {}",
                    season_label(s.season),
                    s.group.as_deref().unwrap_or("Unavailable"),
                    if s.situation.as_deref() == Some("all") {
                        "all strengths"
                    } else {
                        s.situation.as_deref().unwrap_or("unavailable")
                    }
                ))
                .unwrap_or_else(|| "Unavailable".into())
        ),
        repeated: a
            .repeated
            .iter()
            .map(|s| RepeatedView {
                key: s.key.clone(),
                label: s.label.clone(),
                count: format!("{} / {} seasons ≥ P80", s.seasons, a.supplied_seasons),
                range: format!(
                    "{}–{}",
                    percentile_label(s.minimum_percentile),
                    percentile_label(s.maximum_percentile)
                ),
                latest: format!(
                    "{}: {} · {}",
                    season_label(a.last_season),
                    style_comparison_value(&s.unit, Some(s.latest_value), false),
                    percentile_label(s.latest_percentile)
                ),
            })
            .collect(),
        repeated_reason: a.repeated_reason.clone(),
        recent_reason: a.recent_reason.clone(),
        recent: a.recent.as_ref().map(|c| RecentView {
            key: c.key.clone(),
            label: c.label.clone(),
            period: format!(
                "{} → {}",
                season_label(c.from_season),
                season_label(c.to_season)
            ),
            delta: format!("{:+.1} pp", (c.percentile_difference * 10.).round() / 10.),
            ranks: format!(
                "{} → {}",
                percentile_label(c.previous_percentile),
                percentile_label(c.latest_percentile)
            ),
            raw: format!(
                "{} → {}",
                style_comparison_value(&c.unit, Some(c.previous_value), false),
                style_comparison_value(&c.unit, Some(c.latest_value), false)
            ),
        }),
        shortlist: a
            .shortlist
            .iter()
            .map(|key| {
                h.rows
                    .iter()
                    .find(|r| r.key == *key)
                    .map(|r| r.label.clone())
                    .unwrap_or_else(|| key.clone())
            })
            .collect::<Vec<_>>()
            .join(" · "),
    })
}
fn history_view(h: &icelines_core::view_model::rinkcraft_history::HistoryCard) -> HistoryView {
    HistoryView {
        assessment: assessment_view(h),
        sources: h.seasons.iter().flat_map(|s| s.sources.clone()).collect(),
        scope: format!(
            "{} season snapshots · {} reported for this player",
            h.seasons.len(),
            h.seasons.iter().filter(|s| s.state == "available").count()
        ),
        seasons: h.seasons.iter().map(|s| season_label(s.season)).collect(),
        rows: h
            .rows
            .iter()
            .map(|r| HistoryRowView {
                key: r.key.clone(),
                label: r.label.clone(),
                values: r
                    .points
                    .iter()
                    .map(|p| {
                        p.measurement
                            .as_ref()
                            .map(|m| {
                                format!(
                                    "{}{} · {} · n={}",
                                    measurement_value(m),
                                    if m.unit == "Per60" { " /60" } else { "" },
                                    m.percentile
                                        .map(percentile_label)
                                        .unwrap_or_else(|| m.state.replace('_', " ")),
                                    m.measured_peers
                                )
                            })
                            .unwrap_or_else(|| p.state.replace('_', " "))
                    })
                    .collect(),
                repeat: if r.ranked_seasons == 0 {
                    "Unranked".into()
                } else {
                    r.upper_quintile_seasons
                        .map(|n| format!("{n} / {}", r.ranked_seasons))
                        .unwrap_or_else(|| "Not comparable".into())
                },
                reason: r.comparability.clone(),
            })
            .collect(),
        coverage: h
            .seasons
            .iter()
            .map(|s| {
                format!(
                    "{} · {} · {} GP · {} · {}",
                    season_label(s.season),
                    s.group.as_deref().unwrap_or("Missing player"),
                    s.games.map(|g| g.to_string()).unwrap_or_else(|| "—".into()),
                    s.situation.as_deref().unwrap_or("unavailable"),
                    s.minutes
                        .map(|m| format!("{m:.1} minutes"))
                        .unwrap_or_else(|| "unavailable".into())
                )
            })
            .collect(),
        limits: h.limitations.clone(),
    }
}
struct SupportView {
    label: String,
    value: String,
    rank: String,
    formula: String,
    components: String,
    reference: String,
}
struct FacetView {
    explanation:
        Option<icelines_core::view_model::rinkcraft_attribute_evidence::AttributeExplanation>,
    support: Vec<SupportView>,
    label: String,
    key: String,
    value: String,
    rank: String,
    fill: f64,
    ranked: bool,
    peers: usize,
    meaning: String,
}
struct StyleView {
    season: String,
    measured: Vec<StyleMeasureView>,
    gaps: Vec<icelines_core::view_model::rinkcraft_style::StyleGap>,
    limitations: Vec<String>,
}
struct StyleMeasureView {
    key: String,
    label: String,
    value: String,
    scope: String,
    meaning: String,
    formula: String,
    sources: Vec<icelines_core::view_model::rinkcraft_numbers::Source>,
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
        context: c.context_evidence.as_ref().map(context_view),
        sources: c.sources.clone(),
        season: c.season,
        style: c.style_evidence.as_ref().map(|s| StyleView {
            season: season_label(s.season),
            gaps: s.gaps.clone(),
            limitations: s.limitations.clone(),
            measured: s
                .measured
                .iter()
                .map(|m| StyleMeasureView {
                    key: m.key.clone(),
                    label: m.label.clone(),
                    value: format!(
                        "{:.2}{}",
                        m.value * if m.unit == "Pct" { 100. } else { 1. },
                        match m.unit.as_str() {
                            "Pct" => "%",
                            "Mph" => " mph",
                            "Per60" => " /60",
                            "PerGame" => " /game",
                            "MilesPerGame" => " mi/game",
                            _ => "",
                        }
                    ),
                    scope: m.scope.clone(),
                    meaning: m.meaning.clone(),
                    formula: format!("{} / {} = {}", m.numerator, m.denominator, m.value),
                    sources: m.sources.clone(),
                })
                .collect(),
        }),
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
                explanation: f.explanation.clone(),
                support: f
                    .explanation
                    .as_ref()
                    .map(|e| {
                        e.related_keys
                            .iter()
                            .filter_map(|key| c.metrics.iter().find(|m| m.key == *key))
                            .map(|m| SupportView {
                                label: m.label.clone(),
                                value: format!(
                                    "{}{}",
                                    measurement_value(m),
                                    match m.unit.as_str() {
                                        "Per60" => " /60",
                                        "Goals" => " goals",
                                        _ => "",
                                    }
                                ),
                                rank: m
                                    .percentile
                                    .map(percentile_label)
                                    .unwrap_or_else(|| "Unranked".into()),
                                formula: m.formula.clone(),
                                components: format!(
                                    "{} / {} × {}",
                                    m.numerator
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "unavailable".into()),
                                    m.denominator
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "unavailable".into()),
                                    m.scale
                                ),
                                reference: m.reference_sha256.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
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
    context_note: String,
    name: String,
    detail: String,
    evidence: String,
    stats: Vec<FeatureReadout>,
    gaps: Vec<String>,
    sources: Vec<icelines_core::view_model::rinkcraft_character::CharacterSource>,
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
            context_note: view
                .numerical_evidence
                .as_ref()
                .map(|c| {
                    let notes = f
                        .stat_evidence
                        .iter()
                        .filter_map(|link| {
                            c.feature_context
                                .iter()
                                .find(|v| v.attribute_key == link.attribute_key)
                        })
                        .map(|v| v.note.as_str())
                        .collect::<Vec<_>>();
                    if notes.is_empty() {
                        "Documented method; no matched numerical measure in this snapshot.".into()
                    } else {
                        format!(
                            "{} · {}. {}",
                            season_label(c.season),
                            c.situation,
                            notes.join(" ")
                        )
                    }
                })
                .unwrap_or_default(),
            name: f.name.clone(),
            detail: f.detail.clone(),
            evidence: format!("{} · {}", f.evidence_type, f.source_ids.join(", ")),
            gaps: f.unmeasured_stats.clone(),
            sources: f
                .source_ids
                .iter()
                .filter_map(|id| sheet.sources.iter().find(|s| s.id == *id).cloned())
                .collect(),
            stats: if let Some(c) = &view.numerical_evidence {
                f.stat_evidence
                    .iter()
                    .filter_map(|link| {
                        c.feature_context
                            .iter()
                            .find(|v| v.attribute_key == link.attribute_key)
                    })
                    .flat_map(|v| {
                        v.measurements.iter().map(move |m| FeatureReadout {
                            key: m.key.clone(),
                            label: m.label.clone(),
                            value: format!(
                                "{}{}",
                                measurement_value(m),
                                match m.unit.as_str() {
                                    "Per60" => " /60",
                                    "Goals" => " goals",
                                    _ => "",
                                }
                            ),
                            percentile: m
                                .percentile
                                .map(percentile_label)
                                .unwrap_or_else(|| "Unranked".into()),
                            peers: format!("n={}/{}", m.measured_peers, c.eligible_peers),
                            relation: v.relation.clone(),
                        })
                    })
                    .collect()
            } else {
                f.stat_evidence
                    .iter()
                    .map(|link| {
                        let attribute = assessment.and_then(|s| {
                            s.attributes.iter().find(|a| a.key == link.attribute_key)
                        });
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
                    .collect()
            },
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
    let comparison_choices = if let Some(n) = &view.numerical_evidence {
        numerical_catalog(Some(n.season))?
            .catalog
            .profiles
            .into_iter()
            .filter(|p| p.group == n.group && p.player_id != n.player_id)
            .map(|p| NumberChoice {
                id: p.player_id,
                name: p.name,
                selected: false,
            })
            .collect()
    } else {
        Vec::new()
    };
    let template = ProfileTemplate {
        front_position: view.latest_available.as_ref()
            .map(|s| s.position.abbreviation().to_owned())
            .or_else(|| view.numerical_evidence.as_ref().map(|n| n.group.clone()))
            .unwrap_or_else(|| "Unknown".into()),
        documentary_catalog_available: comparison_character_catalog().0.is_some(),
        comparison_choices,
        history: view.numerical_history.as_ref().map(history_view),
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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameBuildPayload { rules: serde_json::Value, request: serde_json::Value }
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameImportPayload { rules: serde_json::Value, character: serde_json::Value }
fn game_response(value: Result<String, String>) -> Result<Response, WebError> {
    let raw = value.map_err(|message| WebError::BadParam { param: "game_character".into(), message, hint: Some("Game builds are fictional; use the supplied versioned rules.".into()) })?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| WebError::Internal(e.into()))?;
    Ok(crate::api::json_data_meta("rinkcraft-game-character", parsed, serde_json::json!({"profile_kind":"fictional_game_character","nhl_assessment":false})))
}
pub async fn post_game_character(axum::Json(payload): axum::Json<GameBuildPayload>) -> Result<Response, WebError> {
    game_response(icelines_core::view_model::rinkcraft_game::build_from_json(&payload.rules.to_string(), &payload.request.to_string()))
}
pub async fn post_import_game_character(axum::Json(payload): axum::Json<GameImportPayload>) -> Result<Response, WebError> {
    game_response(icelines_core::view_model::rinkcraft_game::import_from_json(&payload.rules.to_string(), &payload.character.to_string()))
}

#[derive(Template)]
#[template(path = "rinkcraft_game.html")]
struct GamePageTemplate { active_label: String }
pub async fn get_game_page() -> Result<Html<String>, WebError> {
    GamePageTemplate { active_label: "Game character creation".into() }.render().map(Html).map_err(|e| WebError::Internal(e.into()))
}
pub async fn get_game_rules() -> Result<Response, WebError> {
    let path = std::env::var_os("ICELINES_RINKCRAFT_GAME_RULES").ok_or_else(|| WebError::NotFound { what: "Game rules not configured".into(), hint: Some("Configure the explicit Rinkcraft playtest rules file.".into()) })?;
    let metadata = std::fs::metadata(&path).map_err(|e| WebError::Internal(e.into()))?;
    if metadata.len() > 64 * 1024 { return Err(WebError::BadParam { param: "game_rules".into(), message: "Game rules exceed 64 KiB".into(), hint: None }); }
    let raw = std::fs::read_to_string(path).map_err(|e| WebError::Internal(e.into()))?;
    let rules: icelines_core::view_model::rinkcraft_game::GameRules = serde_json::from_str(&raw).map_err(|e| WebError::BadParam { param: "game_rules".into(), message: e.to_string(), hint: None })?;
    rules.validate().map_err(|message| WebError::BadParam { param: "game_rules".into(), message, hint: None })?;
    Ok(crate::api::json_data_meta("rinkcraft-game-rules", rules, serde_json::json!({"game_only":true})))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PossessionPayload { character_rules: serde_json::Value, rules: serde_json::Value, request: serde_json::Value }
pub async fn post_possession(axum::Json(p): axum::Json<PossessionPayload>) -> Result<Response, WebError> {
    let raw=icelines_core::view_model::rinkcraft_possession::from_json(&p.character_rules.to_string(),&p.rules.to_string(),&p.request.to_string()).map_err(|message|WebError::BadParam {param:"possession".into(),message,hint:None})?;
    let result:serde_json::Value=serde_json::from_str(&raw).map_err(|e|WebError::Internal(e.into()))?;
    Ok(crate::api::json_data_meta("rinkcraft-possession",result,serde_json::json!({"fictional":true,"nhl_probability":false})))
}
pub async fn get_possession_rules() -> Result<Response, WebError> {
    let path=std::env::var_os("ICELINES_RINKCRAFT_ENCOUNTER_RULES").ok_or_else(||WebError::NotFound {what:"Possession playtest rules not configured".into(),hint:None})?;
    if std::fs::metadata(&path).map_err(|e|WebError::Internal(e.into()))?.len()>8192 {return Err(WebError::BadParam {param:"possession_rules".into(),message:"Rules exceed 8 KiB".into(),hint:None});}
    let raw=std::fs::read_to_string(path).map_err(|e|WebError::Internal(e.into()))?;
    if raw.len()>8192 {return Err(WebError::BadParam {param:"possession_rules".into(),message:"Rules exceed 8 KiB".into(),hint:None});}
    let rules:icelines_core::view_model::rinkcraft_possession::Rules=serde_json::from_str(&raw).map_err(|e|WebError::BadParam {param:"possession_rules".into(),message:e.to_string(),hint:None})?;
    rules.validate().map_err(|message|WebError::BadParam {param:"possession_rules".into(),message,hint:None})?;
    Ok(crate::api::json_data_meta("rinkcraft-possession-rules",rules,serde_json::json!({"fictional":true})))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PossessionReplayPayload { replay: serde_json::Value }
fn possession_replay_response(result: Result<String,String>) -> Result<Response,WebError> {
    let raw=result.map_err(|message|WebError::BadParam{param:"possession_replay".into(),message,hint:None})?;
    let value:serde_json::Value=serde_json::from_str(&raw).map_err(|e|WebError::Internal(e.into()))?;
    Ok(crate::api::json_data_meta("rinkcraft-possession-replay",value,serde_json::json!({"fictional":true,"validation":"recomputed_internal_consistency"})))
}
pub async fn post_export_possession(axum::Json(p):axum::Json<PossessionPayload>) -> Result<Response,WebError> {
    possession_replay_response(icelines_core::view_model::rinkcraft_possession::export_replay_from_json(&p.character_rules.to_string(),&p.rules.to_string(),&p.request.to_string()))
}
pub async fn post_import_possession(axum::Json(p):axum::Json<PossessionReplayPayload>) -> Result<Response,WebError> {
    possession_replay_response(icelines_core::view_model::rinkcraft_possession::import_replay_from_json(&p.replay.to_string()))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PossessionInspectPayload { replay:serde_json::Value,step:usize }
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PossessionBranchPayload { replay:serde_json::Value,fork:serde_json::Value }
pub async fn post_inspect_possession(axum::Json(p):axum::Json<PossessionInspectPayload>)->Result<Response,WebError> {
    possession_replay_response(icelines_core::view_model::rinkcraft_possession::inspect_replay_from_json(&p.replay.to_string(),p.step))
}
pub async fn post_branch_possession(axum::Json(p):axum::Json<PossessionBranchPayload>)->Result<Response,WebError> {
    possession_replay_response(icelines_core::view_model::rinkcraft_possession::branch_replay_from_json(&p.replay.to_string(),&p.fork.to_string()))
}
pub async fn post_compare_possession(axum::Json(p):axum::Json<PossessionReplayPayload>)->Result<Response,WebError> {
    possession_replay_response(icelines_core::view_model::rinkcraft_plan_compare::compare_replay_from_json(&p.replay.to_string()))
}
pub async fn post_unit_studio(axum::Json(p):axum::Json<serde_json::Value>)->Result<Response,WebError> {
    let raw=icelines_core::view_model::rinkcraft_unit_studio::from_json(&p.to_string()).map_err(|message|WebError::BadParam{param:"unit_scene".into(),message,hint:None})?;
    let value:serde_json::Value=serde_json::from_str(&raw).map_err(|e|WebError::Internal(e.into()))?;
    Ok(crate::api::json_data_meta("rinkcraft-unit-studio",value,serde_json::json!({"fictional":true,"validation":"recomputed_internal_consistency"})))
}
fn read_unit_asset(env:&str,max:usize)->Result<String,WebError> {
    let path=std::env::var_os(env).ok_or_else(||WebError::NotFound{what:"Authored unit setup not configured".into(),hint:None})?;
    if std::fs::metadata(&path).map_err(|e|WebError::Internal(e.into()))?.len()>max as u64 {return Err(WebError::BadParam{param:"unit_setup".into(),message:"Unit setup file exceeds bounds".into(),hint:None})}
    let raw=std::fs::read_to_string(path).map_err(|e|WebError::Internal(e.into()))?;
    if raw.len()>max {return Err(WebError::BadParam{param:"unit_setup".into(),message:"Unit setup file exceeds bounds".into(),hint:None})}Ok(raw)
}
pub async fn get_unit_blueprint()->Result<Response,WebError> {
    let raw=read_unit_asset("ICELINES_RINKCRAFT_UNIT_BLUEPRINT",256*1024)?;
    let value:icelines_core::view_model::rinkcraft_units::Blueprint=serde_json::from_str(&raw).map_err(|e|WebError::BadParam{param:"unit_blueprint".into(),message:e.to_string(),hint:None})?;
    value.validate_shape().map_err(|message|WebError::BadParam{param:"unit_blueprint".into(),message,hint:None})?;
    Ok(crate::api::json_data_meta("rinkcraft-unit-blueprint",value,serde_json::json!({"fictional":true})))
}
pub async fn get_unit_rules()->Result<Response,WebError> {
    let raw=read_unit_asset("ICELINES_RINKCRAFT_UNIT_RULES",8192)?;
    let value:icelines_core::view_model::rinkcraft_units::Rules=serde_json::from_str(&raw).map_err(|e|WebError::BadParam{param:"unit_rules".into(),message:e.to_string(),hint:None})?;
    value.validate().map_err(|message|WebError::BadParam{param:"unit_rules".into(),message,hint:None})?;
    Ok(crate::api::json_data_meta("rinkcraft-unit-rules",value,serde_json::json!({"fictional":true})))
}
