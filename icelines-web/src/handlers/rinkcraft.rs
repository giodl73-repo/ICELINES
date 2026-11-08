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
    let view = profile(&state, request(id, &p)?).await?;
    Ok(crate::api::json_data_meta(
        "rinkcraft-profile",
        view,
        serde_json::json!({"source_state":"local_loaded_windows","live_fetch":false}),
    ))
}

#[derive(Template)]
#[template(path = "rinkcraft.html")]
struct ProfileTemplate {
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
    let view = profile(&state, req).await?;
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
    let template = ProfileTemplate {
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
        let mut m:ProfileMetric=serde_json::from_value(serde_json::json!({"key":"save-pct","label":"SV%","unit":"Pct","value":0.915,"status":"available","aggregation":"catalog_read","observed_seasons":1,"missing_seasons":[],"report_source":null,"ratio":null})).unwrap();
        assert_eq!(format_metric(&m), "91.50%");
        m.value = None;
        assert_eq!(format_metric(&m), "—");
    }
}
