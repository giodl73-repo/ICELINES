use serde::{Deserialize, Serialize};

use crate::analytics_cache::{
    AnalyticsCacheConsumerEnvelope, AnalyticsCacheConsumerKind, AnalyticsCacheInvalidation,
    AnalyticsCacheQuality, AnalyticsCacheReadDisposition, AnalyticsCacheScope,
    AnalyticsCacheSourceWindow,
};
use crate::view_model::context::{SourceState, ViewWarning};
use crate::view_model::tokens::MetricCell;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsCacheConsumerView {
    pub consumer: AnalyticsCacheConsumerKind,
    pub consumer_contract_version: u16,
    pub cache_key: String,
    pub title: String,
    pub scope: AnalyticsCacheScope,
    pub source_window: AnalyticsCacheSourceWindow,
    pub disposition: AnalyticsCacheReadDisposition,
    pub sources: Vec<SourceState>,
    pub quality: AnalyticsCacheQuality,
    pub invalidation: AnalyticsCacheInvalidation,
    pub methodology_version: String,
    pub metrics: Vec<AnalyticsCacheConsumerMetricRow>,
    pub warnings: Vec<ViewWarning>,
    pub disclosures: Vec<String>,
    pub non_claims: Vec<String>,
}

impl AnalyticsCacheConsumerView {
    pub fn from_envelope(
        envelope: &AnalyticsCacheConsumerEnvelope,
        disposition: AnalyticsCacheReadDisposition,
    ) -> Self {
        Self {
            consumer: envelope.consumer.clone(),
            consumer_contract_version: envelope.consumer_contract_version,
            cache_key: envelope.cache_key.clone(),
            title: analytics_cache_consumer_title(&envelope.consumer).to_string(),
            scope: envelope.scope.clone(),
            source_window: envelope.source_window.clone(),
            disposition,
            sources: envelope.sources.clone(),
            quality: envelope.quality.clone(),
            invalidation: envelope.invalidation.clone(),
            methodology_version: envelope.methodology_version.clone(),
            metrics: envelope
                .metrics
                .iter()
                .map(AnalyticsCacheConsumerMetricRow::from_metric)
                .collect(),
            warnings: envelope.quality.warnings.clone(),
            disclosures: envelope.disclosures.clone(),
            non_claims: envelope.non_claims.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsCacheConsumerMetricRow {
    pub cell: MetricCell,
    pub source_state: Vec<SourceState>,
    pub methodology_note: Option<String>,
}

impl AnalyticsCacheConsumerMetricRow {
    fn from_metric(metric: &crate::analytics_cache::AnalyticsCacheMetric) -> Self {
        Self {
            cell: metric.cell.clone(),
            source_state: metric.source_state.clone(),
            methodology_note: metric.methodology_note.clone(),
        }
    }
}

pub fn analytics_cache_consumer_title(consumer: &AnalyticsCacheConsumerKind) -> &'static str {
    match consumer {
        AnalyticsCacheConsumerKind::CoachDashboard => "Coach Game-Day Dashboard",
        AnalyticsCacheConsumerKind::OpponentScoutReport => "Opponent Scout Report",
        AnalyticsCacheConsumerKind::PlayerEvidenceCard => "Player Evidence Card",
        AnalyticsCacheConsumerKind::LineCombinationExplorer => "Line Combination Explorer",
        AnalyticsCacheConsumerKind::GoalieReadiness => "Goalie Readiness & Workload View",
        AnalyticsCacheConsumerKind::PracticeFocusReport => "Practice Focus Report",
        AnalyticsCacheConsumerKind::PostgameReviewReport => "Postgame Review Report",
        AnalyticsCacheConsumerKind::AgentEvidence => "Agent Evidence Summary",
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::analytics_cache::{
        analytics_cache_consumer_envelope, build_analytics_cache_record, AnalyticsCacheBuildInput,
        AnalyticsCacheConsumerKind, AnalyticsCacheInvalidation, AnalyticsCacheMetric,
        AnalyticsCacheQuality, AnalyticsCacheScope, AnalyticsCacheSourceWindow,
        ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
    };
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::context::{
        Completeness, SourceKind, SourceProvenance, SourceState, ViewWindow,
    };
    use crate::view_model::tokens::{
        MetricCell, MetricUnit, MetricValue, SemanticToken, StatKey, ValuePrecision,
    };

    use super::*;

    #[test]
    fn l2_wp009_consumer_view_preserves_cache_envelope_without_recomputing() {
        let built_at = Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap();
        let source = SourceState {
            source: SourceKind::Snapshot,
            state: Completeness::Partial,
            provenance: Some(SourceProvenance::Snapshot {
                id: "stats-2026-06-01".to_string(),
            }),
            fetched_at: Some(built_at),
            stale_reason: None,
            message: Some("local snapshot source".to_string()),
        };
        let metric = AnalyticsCacheMetric {
            cell: MetricCell {
                key: StatKey::from("expected_goals_share"),
                label: "xG Share".to_string(),
                value: MetricValue::Decimal(55.1),
                unit: MetricUnit::Percentage,
                precision: ValuePrecision::OneDecimal,
                token: Some(SemanticToken::DecisionHighlight),
            },
            source_state: vec![source.clone()],
            methodology_note: Some("prepared by cache methodology".to_string()),
        };
        let record = build_analytics_cache_record(AnalyticsCacheBuildInput {
            cache_key: "coach_dashboard:20252026:regular".to_string(),
            scope: AnalyticsCacheScope::new(
                "coach_dashboard",
                Season(20252026),
                SeasonType::Regular,
            ),
            built_at,
            source_window: AnalyticsCacheSourceWindow::season(
                ViewWindow::new(Season(20252026), SeasonType::Regular),
                "2025-26 regular season through 2026-06-01",
            ),
            sources: vec![source.clone()],
            quality: AnalyticsCacheQuality {
                completeness: Completeness::Partial,
                sample_size: Some(82),
                warnings: Vec::new(),
                limitations: vec!["Does not prove line chemistry causality".to_string()],
            },
            invalidation: AnalyticsCacheInvalidation::keys(vec![
                "snapshot:stats-2026-06-01".to_string()
            ]),
            methodology_version: "cache-foundation-v1".to_string(),
            metrics: vec![metric.clone()],
            disclosures: vec!["Prepared from local snapshot evidence.".to_string()],
            non_claims: vec!["Not a prediction or autonomous coaching claim.".to_string()],
            supported_metric_keys: vec![StatKey::from("expected_goals_share")],
            supported_consumers: vec![AnalyticsCacheConsumerKind::CoachDashboard],
        })
        .expect("cache record");
        let envelope = analytics_cache_consumer_envelope(
            &record,
            AnalyticsCacheConsumerKind::CoachDashboard,
            ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
        )
        .expect("consumer envelope");

        let view = AnalyticsCacheConsumerView::from_envelope(
            &envelope,
            AnalyticsCacheReadDisposition::Stale,
        );

        assert_eq!(view.title, "Coach Game-Day Dashboard");
        assert_eq!(view.cache_key, envelope.cache_key);
        assert_eq!(view.disposition, AnalyticsCacheReadDisposition::Stale);
        assert_eq!(view.sources, envelope.sources);
        assert_eq!(view.quality, envelope.quality);
        assert_eq!(view.methodology_version, envelope.methodology_version);
        assert_eq!(view.metrics[0].cell, metric.cell);
        assert_eq!(view.metrics[0].source_state, metric.source_state);
        assert_eq!(view.disclosures, envelope.disclosures);
        assert_eq!(view.non_claims, envelope.non_claims);
    }
}
