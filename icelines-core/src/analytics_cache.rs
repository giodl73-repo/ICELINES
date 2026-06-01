use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::Season;
use crate::season_stats::SeasonType;
use crate::view_model::{
    Completeness, MetricCell, SourceKind, SourceProvenance, SourceState, StatKey, ViewWarning,
    ViewWindow,
};

pub const ANALYTICS_CACHE_SCHEMA_VERSION: u16 = 1;
pub const ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AnalyticsCacheError {
    #[error("analytics cache schema version {found} is not supported; expected {expected}")]
    UnsupportedSchema { found: u16, expected: u16 },
    #[error("analytics cache json is incompatible: {0}")]
    Json(String),
    #[error("analytics cache key is empty")]
    EmptyCacheKey,
    #[error("analytics cache scope kind is empty")]
    EmptyScopeKind,
    #[error("analytics cache has no source window")]
    EmptySourceWindow,
    #[error("analytics cache has no source state")]
    MissingSources,
    #[error("analytics cache source {source_kind:?} came from a live fetch read path")]
    LiveFetchSource { source_kind: SourceKind },
    #[error("analytics cache has no metrics")]
    MissingMetrics,
    #[error("analytics cache metric {key} is not in the supported metric set")]
    UnsupportedMetric { key: String },
    #[error("analytics cache has no disclosure or non-claim text")]
    MissingDisclosure,
    #[error(
        "analytics cache consumer contract version {found} is not supported; expected {expected}"
    )]
    UnsupportedConsumerContract { found: u16, expected: u16 },
    #[error("analytics cache record does not support consumer {consumer:?}")]
    UnsupportedConsumer {
        consumer: AnalyticsCacheConsumerKind,
    },
}

impl From<serde_json::Error> for AnalyticsCacheError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsCacheConsumerKind {
    CoachDashboard,
    OpponentScoutReport,
    PlayerEvidenceCard,
    LineCombinationExplorer,
    GoalieReadiness,
    PracticeFocusReport,
    PostgameReviewReport,
    AgentEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsCacheScope {
    pub kind: String,
    pub window: ViewWindow,
    pub entity: Option<AnalyticsCacheEntity>,
    pub filters: Vec<AnalyticsCacheFilter>,
}

impl AnalyticsCacheScope {
    pub fn new(kind: impl Into<String>, season: Season, season_type: SeasonType) -> Self {
        Self {
            kind: kind.into(),
            window: ViewWindow::new(season, season_type),
            entity: None,
            filters: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsCacheEntity {
    pub kind: String,
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsCacheFilter {
    pub key: String,
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsCacheSourceWindow {
    pub window: ViewWindow,
    pub from_date: Option<NaiveDate>,
    pub through_date: Option<NaiveDate>,
    pub source_window_label: String,
}

impl AnalyticsCacheSourceWindow {
    pub fn season(window: ViewWindow, label: impl Into<String>) -> Self {
        Self {
            window,
            from_date: None,
            through_date: None,
            source_window_label: label.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsCacheMetric {
    pub cell: MetricCell,
    pub source_state: Vec<SourceState>,
    pub methodology_note: Option<String>,
}

impl AnalyticsCacheMetric {
    pub fn new(cell: MetricCell, source_state: Vec<SourceState>) -> Self {
        Self {
            cell,
            source_state,
            methodology_note: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsCacheQuality {
    pub completeness: Completeness,
    pub sample_size: Option<u32>,
    pub warnings: Vec<ViewWarning>,
    pub limitations: Vec<String>,
}

impl AnalyticsCacheQuality {
    pub fn complete(sample_size: Option<u32>) -> Self {
        Self {
            completeness: Completeness::Complete,
            sample_size,
            warnings: Vec::new(),
            limitations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsCacheInvalidation {
    pub keys: Vec<String>,
    pub stale_after: Option<DateTime<Utc>>,
    pub rebuild_after: Option<DateTime<Utc>>,
}

impl AnalyticsCacheInvalidation {
    pub fn keys(keys: Vec<String>) -> Self {
        Self {
            keys,
            stale_after: None,
            rebuild_after: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalyticsCacheBuildInput {
    pub cache_key: String,
    pub scope: AnalyticsCacheScope,
    pub built_at: DateTime<Utc>,
    pub source_window: AnalyticsCacheSourceWindow,
    pub sources: Vec<SourceState>,
    pub quality: AnalyticsCacheQuality,
    pub invalidation: AnalyticsCacheInvalidation,
    pub methodology_version: String,
    pub metrics: Vec<AnalyticsCacheMetric>,
    pub disclosures: Vec<String>,
    pub non_claims: Vec<String>,
    pub supported_metric_keys: Vec<StatKey>,
    pub supported_consumers: Vec<AnalyticsCacheConsumerKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsCacheRecord {
    pub schema_version: u16,
    pub cache_key: String,
    pub scope: AnalyticsCacheScope,
    pub built_at: DateTime<Utc>,
    pub source_window: AnalyticsCacheSourceWindow,
    pub sources: Vec<SourceState>,
    pub quality: AnalyticsCacheQuality,
    pub invalidation: AnalyticsCacheInvalidation,
    pub methodology_version: String,
    pub metrics: Vec<AnalyticsCacheMetric>,
    pub disclosures: Vec<String>,
    pub non_claims: Vec<String>,
    pub supported_consumers: Vec<AnalyticsCacheConsumerKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsCacheConsumerEnvelope {
    pub schema_version: u16,
    pub consumer_contract_version: u16,
    pub consumer: AnalyticsCacheConsumerKind,
    pub cache_key: String,
    pub scope: AnalyticsCacheScope,
    pub built_at: DateTime<Utc>,
    pub source_window: AnalyticsCacheSourceWindow,
    pub sources: Vec<SourceState>,
    pub quality: AnalyticsCacheQuality,
    pub invalidation: AnalyticsCacheInvalidation,
    pub methodology_version: String,
    pub metrics: Vec<AnalyticsCacheMetric>,
    pub disclosures: Vec<String>,
    pub non_claims: Vec<String>,
}

#[derive(Deserialize)]
struct SchemaProbe {
    schema_version: u16,
}

pub fn build_analytics_cache_record(
    input: AnalyticsCacheBuildInput,
) -> Result<AnalyticsCacheRecord, AnalyticsCacheError> {
    let record = AnalyticsCacheRecord {
        schema_version: ANALYTICS_CACHE_SCHEMA_VERSION,
        cache_key: input.cache_key,
        scope: input.scope,
        built_at: input.built_at,
        source_window: input.source_window,
        sources: input.sources,
        quality: input.quality,
        invalidation: input.invalidation,
        methodology_version: input.methodology_version,
        metrics: input.metrics,
        disclosures: input.disclosures,
        non_claims: input.non_claims,
        supported_consumers: input.supported_consumers,
    };
    validate_record(&record, &input.supported_metric_keys)?;
    Ok(record)
}

pub fn parse_analytics_cache_record_json(
    bytes: &[u8],
    supported_metric_keys: &[StatKey],
) -> Result<AnalyticsCacheRecord, AnalyticsCacheError> {
    let probe: SchemaProbe = serde_json::from_slice(bytes)?;
    if probe.schema_version != ANALYTICS_CACHE_SCHEMA_VERSION {
        return Err(AnalyticsCacheError::UnsupportedSchema {
            found: probe.schema_version,
            expected: ANALYTICS_CACHE_SCHEMA_VERSION,
        });
    }

    let record: AnalyticsCacheRecord = serde_json::from_slice(bytes)?;
    validate_record(&record, supported_metric_keys)?;
    Ok(record)
}

pub fn analytics_cache_consumer_envelope(
    record: &AnalyticsCacheRecord,
    consumer: AnalyticsCacheConsumerKind,
    consumer_contract_version: u16,
) -> Result<AnalyticsCacheConsumerEnvelope, AnalyticsCacheError> {
    if consumer_contract_version != ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION {
        return Err(AnalyticsCacheError::UnsupportedConsumerContract {
            found: consumer_contract_version,
            expected: ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
        });
    }
    if !record.supported_consumers.contains(&consumer) {
        return Err(AnalyticsCacheError::UnsupportedConsumer { consumer });
    }

    Ok(AnalyticsCacheConsumerEnvelope {
        schema_version: record.schema_version,
        consumer_contract_version,
        consumer,
        cache_key: record.cache_key.clone(),
        scope: record.scope.clone(),
        built_at: record.built_at,
        source_window: record.source_window.clone(),
        sources: record.sources.clone(),
        quality: record.quality.clone(),
        invalidation: record.invalidation.clone(),
        methodology_version: record.methodology_version.clone(),
        metrics: record.metrics.clone(),
        disclosures: record.disclosures.clone(),
        non_claims: record.non_claims.clone(),
    })
}

fn validate_record(
    record: &AnalyticsCacheRecord,
    supported_metric_keys: &[StatKey],
) -> Result<(), AnalyticsCacheError> {
    if record.schema_version != ANALYTICS_CACHE_SCHEMA_VERSION {
        return Err(AnalyticsCacheError::UnsupportedSchema {
            found: record.schema_version,
            expected: ANALYTICS_CACHE_SCHEMA_VERSION,
        });
    }
    if record.cache_key.trim().is_empty() {
        return Err(AnalyticsCacheError::EmptyCacheKey);
    }
    if record.scope.kind.trim().is_empty() {
        return Err(AnalyticsCacheError::EmptyScopeKind);
    }
    if record.source_window.source_window_label.trim().is_empty() {
        return Err(AnalyticsCacheError::EmptySourceWindow);
    }
    if record.sources.is_empty() {
        return Err(AnalyticsCacheError::MissingSources);
    }
    validate_no_live_sources(&record.sources)?;
    if record.metrics.is_empty() {
        return Err(AnalyticsCacheError::MissingMetrics);
    }
    for metric in &record.metrics {
        if metric.source_state.is_empty() {
            return Err(AnalyticsCacheError::MissingSources);
        }
        validate_no_live_sources(&metric.source_state)?;
        if !supported_metric_keys.contains(&metric.cell.key) {
            return Err(AnalyticsCacheError::UnsupportedMetric {
                key: metric.cell.key.0.clone(),
            });
        }
    }
    if record.disclosures.is_empty() || record.non_claims.is_empty() {
        return Err(AnalyticsCacheError::MissingDisclosure);
    }

    Ok(())
}

fn validate_no_live_sources(sources: &[SourceState]) -> Result<(), AnalyticsCacheError> {
    for source in sources {
        if matches!(source.provenance, Some(SourceProvenance::LiveFetch { .. })) {
            return Err(AnalyticsCacheError::LiveFetchSource {
                source_kind: source.source,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::view_model::{MetricUnit, MetricValue, SemanticToken, ValuePrecision, WarningKind};

    fn t() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 31, 20, 0, 0).unwrap()
    }

    fn source_state() -> SourceState {
        SourceState {
            source: SourceKind::Snapshot,
            state: Completeness::Complete,
            provenance: Some(SourceProvenance::Snapshot {
                id: "stats-2026-05-31".to_string(),
            }),
            fetched_at: Some(t()),
            stale_reason: None,
            message: Some("local snapshot source".to_string()),
        }
    }

    fn supported_metric_keys() -> Vec<StatKey> {
        vec![StatKey::from("expected_goals_share")]
    }

    fn sample_record() -> AnalyticsCacheRecord {
        let metric = AnalyticsCacheMetric::new(
            MetricCell {
                key: StatKey::from("expected_goals_share"),
                label: "xG Share".to_string(),
                value: MetricValue::Decimal(54.2),
                unit: MetricUnit::Percentage,
                precision: ValuePrecision::OneDecimal,
                token: Some(SemanticToken::DecisionHighlight),
            },
            vec![source_state()],
        );
        build_analytics_cache_record(AnalyticsCacheBuildInput {
            cache_key: "coach_dashboard:20252026:regular".to_string(),
            scope: AnalyticsCacheScope::new(
                "coach_dashboard",
                Season(20252026),
                SeasonType::Regular,
            ),
            built_at: t(),
            source_window: AnalyticsCacheSourceWindow::season(
                ViewWindow::new(Season(20252026), SeasonType::Regular),
                "2025-26 regular season through 2026-05-31",
            ),
            sources: vec![source_state()],
            quality: AnalyticsCacheQuality {
                completeness: Completeness::Partial,
                sample_size: Some(82),
                warnings: vec![ViewWarning {
                    kind: WarningKind::PartialSource,
                    source: Some(SourceKind::Snapshot),
                    message: "shift-level details are not included".to_string(),
                    recovery: Vec::new(),
                }],
                limitations: vec!["Does not prove line chemistry causality".to_string()],
            },
            invalidation: AnalyticsCacheInvalidation::keys(vec![
                "snapshot:stats-2026-05-31".to_string(),
                "methodology:cache-foundation-v1".to_string(),
            ]),
            methodology_version: "cache-foundation-v1".to_string(),
            metrics: vec![metric],
            disclosures: vec![
                "Prepared from local snapshot evidence; stale or partial state is explicit."
                    .to_string(),
            ],
            non_claims: vec![
                "Not a prediction, betting, injury, or autonomous coaching claim.".to_string(),
            ],
            supported_metric_keys: supported_metric_keys(),
            supported_consumers: vec![
                AnalyticsCacheConsumerKind::CoachDashboard,
                AnalyticsCacheConsumerKind::PlayerEvidenceCard,
            ],
        })
        .expect("sample cache record")
    }

    #[test]
    fn l0_wp009_cache_record_serde_round_trip_preserves_evidence() {
        let record = sample_record();
        let json = serde_json::to_vec_pretty(&record).expect("serialize cache record");
        let parsed = parse_analytics_cache_record_json(&json, &supported_metric_keys())
            .expect("parse cache record");

        assert_eq!(parsed.schema_version, ANALYTICS_CACHE_SCHEMA_VERSION);
        assert_eq!(parsed.sources[0].state, Completeness::Complete);
        assert_eq!(
            parsed.metrics[0].source_state[0].source,
            SourceKind::Snapshot
        );
        assert_eq!(
            parsed.invalidation.keys[1],
            "methodology:cache-foundation-v1"
        );
        assert!(parsed.disclosures[0].contains("local snapshot evidence"));
        assert!(parsed.non_claims[0].contains("Not a prediction"));
    }

    #[test]
    fn l0_wp009_cache_refuses_live_fetch_source_state() {
        let mut source = source_state();
        source.source = SourceKind::Scores;
        source.provenance = Some(SourceProvenance::LiveFetch {
            path: "/api/live/scores".to_string(),
        });

        let mut input_record = sample_record();
        input_record.sources = vec![source];

        let err = validate_record(&input_record, &supported_metric_keys())
            .expect_err("live fetch sources must not validate");
        assert_eq!(
            err,
            AnalyticsCacheError::LiveFetchSource {
                source_kind: SourceKind::Scores
            }
        );
    }

    #[test]
    fn l0_wp009_cache_refuses_metric_level_live_fetch_source_state() {
        let mut record = sample_record();
        record.metrics[0].source_state[0].provenance = Some(SourceProvenance::LiveFetch {
            path: "/api/live/metric".to_string(),
        });

        let err = validate_record(&record, &supported_metric_keys())
            .expect_err("metric-level live fetch sources must not validate");
        assert_eq!(
            err,
            AnalyticsCacheError::LiveFetchSource {
                source_kind: SourceKind::Snapshot
            }
        );
    }

    #[test]
    fn l0_wp009_cache_refuses_newer_schema_before_projection() {
        let mut value = serde_json::to_value(sample_record()).expect("record json value");
        value["schema_version"] = serde_json::json!(99);
        let json = serde_json::to_vec(&value).expect("mutated json");

        let err = parse_analytics_cache_record_json(&json, &supported_metric_keys())
            .expect_err("newer schema must refuse");
        assert_eq!(
            err,
            AnalyticsCacheError::UnsupportedSchema {
                found: 99,
                expected: ANALYTICS_CACHE_SCHEMA_VERSION
            }
        );
    }

    #[test]
    fn l0_wp009_cache_refuses_unsupported_metric_key() {
        let record = sample_record();
        let json = serde_json::to_vec(&record).expect("record json");

        let err = parse_analytics_cache_record_json(&json, &[StatKey::from("goals_for")])
            .expect_err("unsupported metric must refuse");
        assert_eq!(
            err,
            AnalyticsCacheError::UnsupportedMetric {
                key: "expected_goals_share".to_string()
            }
        );
    }

    #[test]
    fn l1_wp009_consumer_envelope_preserves_cache_contract() {
        let record = sample_record();
        let envelope = analytics_cache_consumer_envelope(
            &record,
            AnalyticsCacheConsumerKind::CoachDashboard,
            ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
        )
        .expect("consumer envelope");

        assert_eq!(envelope.cache_key, record.cache_key);
        assert_eq!(envelope.metrics, record.metrics);
        assert_eq!(envelope.sources, record.sources);
        assert_eq!(envelope.quality.warnings, record.quality.warnings);
        assert_eq!(envelope.disclosures, record.disclosures);
        assert_eq!(envelope.non_claims, record.non_claims);
    }

    #[test]
    fn l1_wp009_consumer_envelope_rejects_contract_mismatch() {
        let record = sample_record();
        let err = analytics_cache_consumer_envelope(
            &record,
            AnalyticsCacheConsumerKind::CoachDashboard,
            99,
        )
        .expect_err("contract mismatch must refuse");

        assert_eq!(
            err,
            AnalyticsCacheError::UnsupportedConsumerContract {
                found: 99,
                expected: ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION
            }
        );
    }

    #[test]
    fn l1_wp009_consumer_envelope_rejects_unsupported_surface() {
        let record = sample_record();
        let err = analytics_cache_consumer_envelope(
            &record,
            AnalyticsCacheConsumerKind::OpponentScoutReport,
            ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
        )
        .expect_err("unsupported consumer must refuse");

        assert_eq!(
            err,
            AnalyticsCacheError::UnsupportedConsumer {
                consumer: AnalyticsCacheConsumerKind::OpponentScoutReport
            }
        );
    }
}
