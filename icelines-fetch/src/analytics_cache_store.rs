use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use icelines_core::{
    analytics_cache_consumer_envelope, analytics_cache_read_disposition,
    parse_analytics_cache_record_json, AnalyticsCacheConsumerEnvelope, AnalyticsCacheConsumerKind,
    AnalyticsCacheError, AnalyticsCacheReadDisposition, AnalyticsCacheRecord, StatKey,
    ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
};
use thiserror::Error;

use crate::atomic_write::write_bytes_atomic;

#[derive(Debug, Error)]
pub enum AnalyticsCacheStoreError {
    #[error("analytics cache key is empty")]
    EmptyCacheKey,
    #[error("analytics cache invalidation key is empty")]
    EmptyInvalidationKey,
    #[error("analytics cache entry is missing: {cache_key}")]
    MissingCache { cache_key: String },
    #[error("analytics cache store IO at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Contract(#[from] AnalyticsCacheError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyticsCacheRead {
    pub record: AnalyticsCacheRecord,
    pub disposition: AnalyticsCacheReadDisposition,
}

#[derive(Debug, Clone)]
pub struct AnalyticsCacheStore {
    root: PathBuf,
}

impl AnalyticsCacheStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn under_data_root(data_root: impl AsRef<Path>) -> Self {
        Self::new(data_root.as_ref().join("analytics_cache"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn record_path(&self, cache_key: &str) -> Result<PathBuf, AnalyticsCacheStoreError> {
        Ok(self
            .root
            .join(format!("{}.json", encode_cache_key(cache_key)?)))
    }

    pub fn write_record(
        &self,
        record: &AnalyticsCacheRecord,
        supported_metric_keys: &[StatKey],
    ) -> Result<(), AnalyticsCacheStoreError> {
        let json = serde_json::to_vec_pretty(record).map_err(AnalyticsCacheError::from)?;
        parse_analytics_cache_record_json(&json, supported_metric_keys)?;
        let path = self.record_path(&record.cache_key)?;
        write_bytes_atomic(&path, &json)
            .map_err(|source| AnalyticsCacheStoreError::Io { path, source })
    }

    pub fn read_record(
        &self,
        cache_key: &str,
        supported_metric_keys: &[StatKey],
        now: DateTime<Utc>,
    ) -> Result<AnalyticsCacheRead, AnalyticsCacheStoreError> {
        let path = self.record_path(cache_key)?;
        if !path.exists() {
            return Err(AnalyticsCacheStoreError::MissingCache {
                cache_key: cache_key.to_string(),
            });
        }
        let bytes = std::fs::read(&path).map_err(|source| AnalyticsCacheStoreError::Io {
            path: path.clone(),
            source,
        })?;
        let record = parse_analytics_cache_record_json(&bytes, supported_metric_keys)?;
        let disposition = analytics_cache_read_disposition(&record, now);
        Ok(AnalyticsCacheRead {
            record,
            disposition,
        })
    }

    pub fn read_consumer_envelope(
        &self,
        cache_key: &str,
        supported_metric_keys: &[StatKey],
        consumer: AnalyticsCacheConsumerKind,
        now: DateTime<Utc>,
    ) -> Result<AnalyticsCacheConsumerEnvelope, AnalyticsCacheStoreError> {
        let read = self.read_record(cache_key, supported_metric_keys, now)?;
        Ok(analytics_cache_consumer_envelope(
            &read.record,
            consumer,
            ANALYTICS_CACHE_CONSUMER_CONTRACT_VERSION,
        )?)
    }

    pub fn invalidate_cache_key(&self, cache_key: &str) -> Result<bool, AnalyticsCacheStoreError> {
        let path = self.record_path(cache_key)?;
        if !path.exists() {
            return Ok(false);
        }
        std::fs::remove_file(&path)
            .map_err(|source| AnalyticsCacheStoreError::Io { path, source })?;
        Ok(true)
    }

    pub fn invalidate_records_matching(
        &self,
        invalidation_key: &str,
        supported_metric_keys: &[StatKey],
    ) -> Result<usize, AnalyticsCacheStoreError> {
        if invalidation_key.trim().is_empty() {
            return Err(AnalyticsCacheStoreError::EmptyInvalidationKey);
        }
        if !self.root.exists() {
            return Ok(0);
        }

        let mut removed = 0;
        for entry in
            std::fs::read_dir(&self.root).map_err(|source| AnalyticsCacheStoreError::Io {
                path: self.root.clone(),
                source,
            })?
        {
            let entry = entry.map_err(|source| AnalyticsCacheStoreError::Io {
                path: self.root.clone(),
                source,
            })?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let bytes = std::fs::read(&path).map_err(|source| AnalyticsCacheStoreError::Io {
                path: path.clone(),
                source,
            })?;
            let record = parse_analytics_cache_record_json(&bytes, supported_metric_keys)?;
            if record
                .invalidation
                .keys
                .iter()
                .any(|key| key == invalidation_key)
            {
                std::fs::remove_file(&path)
                    .map_err(|source| AnalyticsCacheStoreError::Io { path, source })?;
                removed += 1;
            }
        }
        Ok(removed)
    }
}

fn encode_cache_key(cache_key: &str) -> Result<String, AnalyticsCacheStoreError> {
    if cache_key.trim().is_empty() {
        return Err(AnalyticsCacheStoreError::EmptyCacheKey);
    }

    let mut encoded = String::new();
    for byte in cache_key.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => {
                encoded.push(char::from(*byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use icelines_core::season_stats::SeasonType;
    use icelines_core::{
        AnalyticsCacheBuildInput, AnalyticsCacheInvalidation, AnalyticsCacheMetric,
        AnalyticsCacheQuality, AnalyticsCacheScope, AnalyticsCacheSourceWindow, Completeness,
        MetricCell, MetricUnit, MetricValue, Season, SemanticToken, SourceKind, SourceProvenance,
        SourceState, ValuePrecision, ViewWarning, ViewWindow, WarningKind,
    };
    use tempfile::TempDir;

    use super::*;

    fn t() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 1, 12, 0, 0).unwrap()
    }

    fn supported_metric_keys() -> Vec<StatKey> {
        vec![StatKey::from("expected_goals_share")]
    }

    fn source_state(state: Completeness) -> SourceState {
        SourceState {
            source: SourceKind::Snapshot,
            state,
            provenance: Some(SourceProvenance::Snapshot {
                id: "stats-2026-06-01".to_string(),
            }),
            fetched_at: Some(t()),
            stale_reason: (state == Completeness::Stale).then(|| "snapshot expired".to_string()),
            message: Some("local snapshot source".to_string()),
        }
    }

    fn sample_record(cache_key: &str) -> AnalyticsCacheRecord {
        let source = source_state(Completeness::Complete);
        let metric = AnalyticsCacheMetric::new(
            MetricCell {
                key: StatKey::from("expected_goals_share"),
                label: "xG Share".to_string(),
                value: MetricValue::Decimal(55.1),
                unit: MetricUnit::Percentage,
                precision: ValuePrecision::OneDecimal,
                token: Some(SemanticToken::DecisionHighlight),
            },
            vec![source.clone()],
        );

        icelines_core::build_analytics_cache_record(AnalyticsCacheBuildInput {
            cache_key: cache_key.to_string(),
            scope: AnalyticsCacheScope::new(
                "coach_dashboard",
                Season(20252026),
                SeasonType::Regular,
            ),
            built_at: t(),
            source_window: AnalyticsCacheSourceWindow::season(
                ViewWindow::new(Season(20252026), SeasonType::Regular),
                "2025-26 regular season through 2026-06-01",
            ),
            sources: vec![source],
            quality: AnalyticsCacheQuality {
                completeness: Completeness::Complete,
                sample_size: Some(82),
                warnings: Vec::new(),
                limitations: vec!["Does not prove line chemistry causality".to_string()],
            },
            invalidation: AnalyticsCacheInvalidation::keys(vec![
                "snapshot:stats-2026-06-01".to_string(),
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
    fn l1_wp009_store_writes_and_reads_record_without_live_fetch() {
        let dir = TempDir::new().unwrap();
        let store = AnalyticsCacheStore::under_data_root(dir.path());
        let record = sample_record("coach_dashboard:20252026:regular");

        store
            .write_record(&record, &supported_metric_keys())
            .unwrap();
        let read = store
            .read_record(&record.cache_key, &supported_metric_keys(), t())
            .unwrap();

        assert_eq!(read.disposition, AnalyticsCacheReadDisposition::Fresh);
        assert_eq!(read.record.cache_key, record.cache_key);
        assert_eq!(read.record.sources, record.sources);
        assert!(!store
            .record_path(&record.cache_key)
            .unwrap()
            .with_extension("json.tmp")
            .exists());
    }

    #[test]
    fn l1_wp009_store_missing_read_refuses_without_creating_cache_root() {
        let dir = TempDir::new().unwrap();
        let store = AnalyticsCacheStore::under_data_root(dir.path());
        let err = store
            .read_record("missing:cache", &supported_metric_keys(), t())
            .expect_err("missing cache entries must not be synthesized");

        assert!(matches!(err, AnalyticsCacheStoreError::MissingCache { .. }));
        assert!(
            !store.root().exists(),
            "read miss must not create cache storage"
        );
    }

    #[test]
    fn l1_wp009_store_preserves_stale_partial_and_missing_source_state() {
        let dir = TempDir::new().unwrap();
        let store = AnalyticsCacheStore::under_data_root(dir.path());
        let mut record = sample_record("coach_dashboard:stale-partial");
        record.quality.completeness = Completeness::Partial;
        record.quality.warnings = vec![ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::Snapshot),
            message: "shot-quality inputs are partial".to_string(),
            recovery: Vec::new(),
        }];
        record.sources = vec![SourceState::missing(SourceKind::PlayByPlay)];
        record.metrics[0].source_state = vec![source_state(Completeness::Stale)];
        record.invalidation.stale_after = Some(Utc.with_ymd_and_hms(2026, 6, 1, 11, 0, 0).unwrap());

        store
            .write_record(&record, &supported_metric_keys())
            .unwrap();
        let read = store
            .read_record(&record.cache_key, &supported_metric_keys(), t())
            .unwrap();

        assert_eq!(read.disposition, AnalyticsCacheReadDisposition::Stale);
        assert_eq!(read.record.quality.completeness, Completeness::Partial);
        assert_eq!(read.record.sources[0].state, Completeness::Unavailable);
        assert_eq!(
            read.record.metrics[0].source_state[0].state,
            Completeness::Stale
        );
    }

    #[test]
    fn l1_wp009_store_refuses_newer_schema_and_unsupported_metric_on_read() {
        let dir = TempDir::new().unwrap();
        let store = AnalyticsCacheStore::under_data_root(dir.path());
        let record = sample_record("coach_dashboard:schema-refusal");
        store
            .write_record(&record, &supported_metric_keys())
            .unwrap();

        let path = store.record_path(&record.cache_key).unwrap();
        let mut value = serde_json::to_value(record).unwrap();
        value["schema_version"] = serde_json::json!(99);
        write_bytes_atomic(&path, serde_json::to_vec_pretty(&value).unwrap().as_slice()).unwrap();
        let err = store
            .read_record(
                "coach_dashboard:schema-refusal",
                &supported_metric_keys(),
                t(),
            )
            .expect_err("newer schema must refuse before projection");
        assert!(matches!(
            err,
            AnalyticsCacheStoreError::Contract(AnalyticsCacheError::UnsupportedSchema { .. })
        ));

        let record = sample_record("coach_dashboard:metric-refusal");
        store
            .write_record(&record, &supported_metric_keys())
            .unwrap();
        let err = store
            .read_record(
                "coach_dashboard:metric-refusal",
                &[StatKey::from("goals_for")],
                t(),
            )
            .expect_err("unsupported metric must refuse");
        assert!(matches!(
            err,
            AnalyticsCacheStoreError::Contract(AnalyticsCacheError::UnsupportedMetric { .. })
        ));
    }

    #[test]
    fn l1_wp009_store_invalidates_matching_records_only() {
        let dir = TempDir::new().unwrap();
        let store = AnalyticsCacheStore::under_data_root(dir.path());
        let matching = sample_record("coach_dashboard:invalidate-me");
        let mut retained = sample_record("coach_dashboard:retain-me");
        retained.invalidation.keys = vec!["snapshot:other".to_string()];

        store
            .write_record(&matching, &supported_metric_keys())
            .unwrap();
        store
            .write_record(&retained, &supported_metric_keys())
            .unwrap();
        let removed = store
            .invalidate_records_matching("snapshot:stats-2026-06-01", &supported_metric_keys())
            .unwrap();

        assert_eq!(removed, 1);
        assert!(matches!(
            store.read_record(&matching.cache_key, &supported_metric_keys(), t()),
            Err(AnalyticsCacheStoreError::MissingCache { .. })
        ));
        assert!(store
            .read_record(&retained.cache_key, &supported_metric_keys(), t())
            .is_ok());
    }

    #[test]
    fn l1_wp009_store_failed_rebuild_does_not_replace_existing_record() {
        let dir = TempDir::new().unwrap();
        let store = AnalyticsCacheStore::under_data_root(dir.path());
        let record = sample_record("coach_dashboard:rollback");
        store
            .write_record(&record, &supported_metric_keys())
            .unwrap();

        let mut invalid = record.clone();
        invalid.metrics.clear();
        let err = store
            .write_record(&invalid, &supported_metric_keys())
            .expect_err("invalid rebuild candidate must not be written");
        assert!(matches!(
            err,
            AnalyticsCacheStoreError::Contract(AnalyticsCacheError::MissingMetrics)
        ));

        let read = store
            .read_record(&record.cache_key, &supported_metric_keys(), t())
            .unwrap();
        assert_eq!(read.record.metrics.len(), 1);
        assert_eq!(read.record.built_at, record.built_at);
    }

    #[test]
    fn l2_wp009_store_consumer_envelope_preserves_stored_contract() {
        let dir = TempDir::new().unwrap();
        let store = AnalyticsCacheStore::under_data_root(dir.path());
        let record = sample_record("coach_dashboard:consumer");
        store
            .write_record(&record, &supported_metric_keys())
            .unwrap();

        let envelope = store
            .read_consumer_envelope(
                &record.cache_key,
                &supported_metric_keys(),
                AnalyticsCacheConsumerKind::CoachDashboard,
                t(),
            )
            .unwrap();

        assert_eq!(envelope.cache_key, record.cache_key);
        assert_eq!(envelope.sources, record.sources);
        assert_eq!(envelope.metrics, record.metrics);
        assert_eq!(envelope.non_claims, record.non_claims);
    }
}
