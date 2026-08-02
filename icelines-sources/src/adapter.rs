pub use icelines_core::source_facts::{AdapterId, AdapterVersion, ContentHash, SourceId};
use icelines_core::source_facts::{FreshnessClass, ProviderId};
pub type ValidationError = icelines_core::source_facts::SourceContractError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInput<'a> {
    bytes: &'a [u8],
    source_id: SourceId,
    content_hash: ContentHash,
}

impl<'a> SourceInput<'a> {
    pub fn new(bytes: &'a [u8], source_id: SourceId, content_hash: ContentHash) -> Self {
        Self {
            bytes,
            source_id,
            content_hash,
        }
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDescriptor {
    pub source_id: SourceId,
    pub provider: ProviderId,
    pub adapter_id: AdapterId,
    pub adapter_version: AdapterVersion,
    pub payload_family: &'static str,
    pub supported_layouts: &'static [&'static str],
    pub required_identity_keys: &'static [&'static str],
    pub additive_field_policy: AdditiveFieldPolicy,
    pub freshness_class: FreshnessClass,
    pub historical_availability: HistoricalAvailability,
    pub absence_semantics: AbsenceSemantics,
    pub output_fact_families: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdditiveFieldPolicy {
    Reject,
    IgnoreReviewed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalAvailability {
    PointInTimeOnly,
    ProviderArchive,
    CallerSuppliedArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsenceSemantics {
    AuthoritativeEmpty,
    NotEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterErrorCategory {
    UnsupportedLayout,
    MalformedRecord,
    SemanticValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterDisposition {
    FatalSource,
    QuarantinedRecord,
}

#[derive(Debug, thiserror::Error)]
#[error("{adapter_id} failed for {source_id} ({category:?}, {disposition:?}): {message}")]
pub struct AdapterError {
    pub source_id: SourceId,
    pub adapter_id: AdapterId,
    pub input_hash: ContentHash,
    pub category: AdapterErrorCategory,
    pub disposition: AdapterDisposition,
    pub message: String,
}

pub trait SourceAdapter {
    type Output;

    fn descriptor(&self) -> SourceDescriptor;
    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError>;
}

#[cfg(test)]
mod tests {
    use super::{AdapterId, AdapterVersion, ContentHash, SourceId, ValidationError};

    #[test]
    fn identifiers_reject_empty_values() {
        assert_eq!(
            SourceId::try_new(" ").unwrap_err(),
            ValidationError::Empty("source_id")
        );
        assert!(AdapterId::try_new("nhl.player_landing").is_ok());
        assert!(AdapterVersion::try_new("v1").is_ok());
    }

    #[test]
    fn content_hash_requires_canonical_sha256_hex() {
        assert!(ContentHash::try_new("0".repeat(64)).is_ok());
        assert_eq!(
            ContentHash::try_new("ABC").unwrap_err(),
            ValidationError::InvalidContentHash
        );
    }
}
