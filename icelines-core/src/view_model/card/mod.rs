//! UI-neutral card document primitives.
//!
//! This module owns presentation-independent document structure and validation.
//! Domain builders add hockey meaning in later layers; renderers only project
//! these serialized values.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
    Completeness, EmptyState, EvidenceLabel, MetricCell, MetricValue, SemanticToken, SourceKind,
    ViewContext, ViewWarning,
};

mod team_prognosis;
pub use team_prognosis::*;
mod fantasy_roster;
pub use fantasy_roster::*;
mod fantasy_draft;
pub use fantasy_draft::*;
mod fantasy_morning;
pub use fantasy_morning::*;
mod fantasy_trade;
pub use fantasy_trade::*;
mod season_simulation;
pub use season_simulation::*;
mod forecast_movement;
pub use forecast_movement::*;
mod forecast_history;
pub use forecast_history::*;
mod organization_window;
pub use organization_window::*;
mod organization_profile_history;
pub use organization_profile_history::*;

pub const CARD_DOCUMENT_SCHEMA: &str = "card_document.v1";
pub const CARD_DOCUMENT_JSON_SCHEMA: &str =
    include_str!("../../../../design/schemas/card_document.v1.schema.json");

pub fn parse_card_document(json: &str) -> Result<CardDocumentView, CardDocumentError> {
    let document: CardDocumentView = serde_json::from_str(json)
        .map_err(|error| CardDocumentError::Deserialization(error.to_string()))?;
    document.validate()?;
    Ok(document)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    TeamPrognosis,
    SeasonSimulation,
    ForecastMovement,
    ForecastHistory,
    FantasyRoster,
    FantasyDraft,
    FantasyMorning,
    FantasyTrade,
    OrganizationWindow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CardIdentityJoinsView {
    pub league_id: Option<String>,
    pub roster_snapshot_id: Option<String>,
    pub calendar_fingerprint: Option<String>,
    pub scoring_scheme_id: Option<String>,
    pub scenario_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario_comparison_key: Option<String>,
    pub team_ids: Vec<String>,
    pub player_ids: Vec<String>,
    pub game_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CardSimulationContextView {
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub parameter_fingerprint: Option<String>,
    pub seed: Option<u64>,
    pub trials: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardContextView {
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
    pub evidence_label: EvidenceLabel,
    pub builder_version: String,
    pub methodology_versions: BTreeMap<String, String>,
    pub joins: CardIdentityJoinsView,
    pub simulation: CardSimulationContextView,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardThemeView {
    pub theme_key: String,
    pub primary: Option<String>,
    pub secondary: Option<String>,
    pub accent: Option<String>,
    pub surface: Option<String>,
    pub text: Option<String>,
    pub team_abbreviation: Option<String>,
    pub ascii_identity: String,
    pub minimum_text_contrast_x100: u16,
}

/// Build the canonical UI-neutral card theme for an NHL team.
///
/// Card builders own these semantic colors so every renderer and card family
/// receives the same identity. Unknown abbreviations deliberately retain a
/// neutral fallback rather than borrowing another team's colors.
pub fn nhl_team_card_theme(team: &str) -> CardThemeView {
    let team = team.trim().to_ascii_uppercase();
    let (primary, secondary, accent) = match team.as_str() {
        "ANA" => ("#F47A38", "#B9975B", "#B9975B"),
        "BOS" => ("#FFB81C", "#000000", "#000000"),
        "BUF" => ("#002654", "#FCB514", "#FCB514"),
        "CAR" => ("#CC0000", "#000000", "#000000"),
        "CBJ" => ("#002654", "#CE1126", "#CE1126"),
        "CGY" => ("#C8102E", "#F1BE48", "#F1BE48"),
        "CHI" => ("#CF0A2C", "#000000", "#000000"),
        "COL" => ("#6F263D", "#236192", "#236192"),
        "DAL" => ("#006847", "#8F8F8C", "#8F8F8C"),
        "DET" => ("#CE1126", "#FFFFFF", "#FFFFFF"),
        "EDM" => ("#041E42", "#FF4C00", "#FF4C00"),
        "FLA" => ("#041E42", "#C8102E", "#C8102E"),
        "LAK" => ("#111111", "#A2AAAD", "#A2AAAD"),
        "MIN" => ("#154734", "#A6192E", "#A6192E"),
        "MTL" => ("#AF1E2D", "#192168", "#192168"),
        "NJD" => ("#CE1126", "#000000", "#000000"),
        "NSH" => ("#FFB81C", "#041E42", "#041E42"),
        "NYI" => ("#00539B", "#F47D30", "#F47D30"),
        // Preserve the released NYR and SEA document values exactly.
        "NYR" => ("#0038A8", "#CE1126", "#FFFFFF"),
        "OTT" => ("#C52032", "#C2912C", "#C2912C"),
        "PHI" => ("#F74902", "#000000", "#000000"),
        "PIT" => ("#FCB514", "#000000", "#000000"),
        "SEA" => ("#001628", "#99D9D9", "#E9072B"),
        "SJS" => ("#006D75", "#EA7200", "#EA7200"),
        "STL" => ("#002F87", "#FCB514", "#FCB514"),
        "TBL" => ("#002868", "#FFFFFF", "#FFFFFF"),
        "TOR" => ("#00205B", "#FFFFFF", "#FFFFFF"),
        "UTA" => ("#71AFE5", "#000000", "#000000"),
        "VAN" => ("#00205B", "#00843D", "#00843D"),
        "VGK" => ("#B4975A", "#333F42", "#333F42"),
        "WPG" => ("#041E42", "#AC162C", "#AC162C"),
        "WSH" => ("#041E42", "#C8102E", "#C8102E"),
        _ => ("#14213D", "#E5E5E5", "#FCA311"),
    };
    CardThemeView {
        theme_key: format!("team_{}", team.to_ascii_lowercase()),
        primary: Some(primary.to_owned()),
        secondary: Some(secondary.to_owned()),
        accent: Some(accent.to_owned()),
        surface: Some("#FFFFFF".to_owned()),
        text: Some("#111111".to_owned()),
        team_abbreviation: Some(team.clone()),
        ascii_identity: team,
        minimum_text_contrast_x100: 450,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardProvenanceView {
    pub id: String,
    pub source: SourceKind,
    pub label: String,
    pub state: Completeness,
    pub observed_at: Option<DateTime<Utc>>,
    pub fingerprint: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRendererCapability {
    RichColor,
    RasterAssets,
    Tables,
    Timelines,
    InteractiveActions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAssetKind {
    Headshot,
    TeamMark,
    GeneratedChart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAssetState {
    Available,
    Missing,
    Stale,
    Blocked,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reference_type", content = "value", rename_all = "snake_case")]
pub enum CardAssetReference {
    ExternalUrl(String),
    LocalContent(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "fallback_type", content = "value", rename_all = "snake_case")]
pub enum CardAssetFallback {
    Initials(String),
    Abbreviation(String),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardAssetView {
    pub id: String,
    pub subject_id: String,
    pub kind: CardAssetKind,
    pub reference: Option<CardAssetReference>,
    pub source: SourceKind,
    pub observed_at: Option<DateTime<Utc>>,
    pub integrity_sha256: Option<String>,
    pub alt: String,
    pub state: CardAssetState,
    pub fallback: CardAssetFallback,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardPageView {
    pub id: String,
    pub literal_label: String,
    pub display_label: Option<String>,
    pub order: u16,
    pub accessible_summary: String,
    pub sections: Vec<CardSectionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "section_type", rename_all = "snake_case")]
pub enum CardSectionView {
    IdentityHeader(IdentityHeaderSectionView),
    Lineup(LineupSectionView),
    MetricStrip(MetricStripSectionView),
    PlayerList(PlayerListSectionView),
    ScenarioBridge(ScenarioBridgeSectionView),
    ProbabilityRange(ProbabilityRangeSectionView),
    Decision(DecisionSectionView),
    Timeline(TimelineSectionView),
    StateNotice(StateNoticeSectionView),
    Methodology(MethodologySectionView),
    Provenance(ProvenanceSectionView),
}

impl CardSectionView {
    pub fn id(&self) -> &str {
        match self {
            Self::IdentityHeader(section) => &section.id,
            Self::Lineup(section) => &section.id,
            Self::MetricStrip(section) => &section.id,
            Self::PlayerList(section) => &section.id,
            Self::ScenarioBridge(section) => &section.id,
            Self::ProbabilityRange(section) => &section.id,
            Self::Decision(section) => &section.id,
            Self::Timeline(section) => &section.id,
            Self::StateNotice(section) => &section.id,
            Self::Methodology(section) => &section.id,
            Self::Provenance(section) => &section.id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardIdentityKind {
    Team,
    Player,
    League,
    Matchup,
    Scenario,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardIdentityView {
    pub kind: CardIdentityKind,
    pub subject_id: String,
    pub label: String,
    pub asset_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityHeaderSectionView {
    pub id: String,
    pub eyebrow: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub identities: Vec<CardIdentityView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardLineupGroupKind {
    ForwardLine,
    DefensePair,
    Goalies,
    ActiveSlots,
    Bench,
    InjuredReserve,
    Extras,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardLineupSlotView {
    pub id: String,
    pub label: String,
    pub subject_id: Option<String>,
    pub subject_label: Option<String>,
    pub asset_id: Option<String>,
    pub metrics: Vec<CardMetricView>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardLineupGroupView {
    pub id: String,
    pub label: String,
    pub kind: CardLineupGroupKind,
    pub slots: Vec<CardLineupSlotView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineupSectionView {
    pub id: String,
    pub title: String,
    pub groups: Vec<CardLineupGroupView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardMetricComparisonView {
    pub label: String,
    pub baseline: MetricValue,
    pub delta: MetricValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardMetricView {
    pub metric: MetricCell,
    pub display_text: String,
    pub accessible_text: String,
    pub comparison: Option<CardMetricComparisonView>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricStripSectionView {
    pub id: String,
    pub title: Option<String>,
    pub metrics: Vec<CardMetricView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardPlayerRowView {
    pub player_id: String,
    pub name: String,
    pub role: Option<String>,
    pub asset_id: Option<String>,
    pub metrics: Vec<CardMetricView>,
    pub tokens: Vec<SemanticToken>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerListSectionView {
    pub id: String,
    pub title: String,
    pub rows: Vec<CardPlayerRowView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioBridgeSectionView {
    pub id: String,
    pub title: String,
    pub from_label: String,
    pub to_label: String,
    pub metrics: Vec<CardMetricView>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardProbabilityRangeView {
    pub key: String,
    pub label: String,
    pub low: MetricCell,
    pub median: MetricCell,
    pub high: MetricCell,
    pub display_text: String,
    pub accessible_text: String,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProbabilityRangeSectionView {
    pub id: String,
    pub title: String,
    pub ranges: Vec<CardProbabilityRangeView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDecisionAlternativeView {
    pub id: String,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionSectionView {
    pub id: String,
    pub title: String,
    pub recommendation: String,
    pub rationale: Vec<String>,
    pub alternatives: Vec<CardDecisionAlternativeView>,
    pub action_id: Option<String>,
    pub token: SemanticToken,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardTimelineItemView {
    pub id: String,
    pub effective_at: DateTime<Utc>,
    pub observed_at: Option<DateTime<Utc>>,
    pub label: String,
    pub detail: Option<String>,
    pub evidence_label: EvidenceLabel,
    pub token: SemanticToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineSectionView {
    pub id: String,
    pub title: String,
    pub items: Vec<CardTimelineItemView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateNoticeSectionView {
    pub id: String,
    pub title: String,
    pub detail: Option<String>,
    pub empty_state: Option<EmptyState>,
    pub warnings: Vec<ViewWarning>,
    pub token: SemanticToken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardMethodologyItemView {
    pub key: String,
    pub label: String,
    pub version: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodologySectionView {
    pub id: String,
    pub title: String,
    pub methods: Vec<CardMethodologyItemView>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceSectionView {
    pub id: String,
    pub title: String,
    pub provenance_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardDocumentView {
    pub schema: String,
    pub card_kind: CardKind,
    pub document_id: String,
    pub fingerprint: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub context: CardContextView,
    pub theme: CardThemeView,
    pub required_capabilities: Vec<CardRendererCapability>,
    pub pages: Vec<CardPageView>,
    pub assets: Vec<CardAssetView>,
    pub provenance: Vec<CardProvenanceView>,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl CardDocumentView {
    pub fn seal(mut self) -> Result<Self, CardDocumentError> {
        self.fingerprint = self.calculate_fingerprint()?;
        self.validate()?;
        Ok(self)
    }

    pub fn refresh_fingerprint(&mut self) -> Result<(), CardDocumentError> {
        self.fingerprint = self.calculate_fingerprint()?;
        Ok(())
    }

    pub fn calculate_fingerprint(&self) -> Result<String, CardDocumentError> {
        let mut material = self.clone();
        material.fingerprint.clear();
        // Seal the wire-normalized representation. Some deeply derived f64
        // values can be shortened by JSON serialization; hashing the in-memory
        // pre-wire value makes an otherwise unchanged document fail validation
        // after it is read back by another renderer.
        let wire = serde_json::to_vec(&material)
            .map_err(|error| CardDocumentError::Serialization(error.to_string()))?;
        let normalized: Self = serde_json::from_slice(&wire)
            .map_err(|error| CardDocumentError::Deserialization(error.to_string()))?;
        let bytes = serde_json::to_vec(&normalized)
            .map_err(|error| CardDocumentError::Serialization(error.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub fn validate_renderer_capabilities(
        &self,
        supported: &[CardRendererCapability],
    ) -> Result<(), CardDocumentError> {
        for required in &self.required_capabilities {
            if !supported.contains(required) {
                return Err(CardDocumentError::MissingRendererCapability(*required));
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), CardDocumentError> {
        if self.schema != CARD_DOCUMENT_SCHEMA {
            return Err(CardDocumentError::UnsupportedSchema(self.schema.clone()));
        }
        require_text("document_id", &self.document_id)?;
        require_text("title", &self.title)?;
        require_text("context.builder_version", &self.context.builder_version)?;
        require_text("theme.theme_key", &self.theme.theme_key)?;
        require_text("theme.ascii_identity", &self.theme.ascii_identity)?;
        validate_theme(&self.theme)?;

        let mut capabilities = HashSet::new();
        for capability in &self.required_capabilities {
            if !capabilities.insert(*capability) {
                return Err(CardDocumentError::DuplicateRendererCapability(*capability));
            }
        }

        let mut asset_ids = HashSet::new();
        for asset in &self.assets {
            require_text("asset.id", &asset.id)?;
            require_text("asset.subject_id", &asset.subject_id)?;
            require_text("asset.alt", &asset.alt)?;
            if !asset_ids.insert(asset.id.as_str()) {
                return Err(CardDocumentError::DuplicateAssetId(asset.id.clone()));
            }
            if asset.state == CardAssetState::Available && asset.reference.is_none() {
                return Err(CardDocumentError::AvailableAssetMissingReference(
                    asset.id.clone(),
                ));
            }
            if let Some(CardAssetReference::ExternalUrl(url)) = &asset.reference {
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    return Err(CardDocumentError::InvalidAssetReference(asset.id.clone()));
                }
            }
            if let Some(CardAssetReference::LocalContent(reference)) = &asset.reference {
                if reference.trim().is_empty() {
                    return Err(CardDocumentError::InvalidAssetReference(asset.id.clone()));
                }
            }
            if let Some(integrity) = &asset.integrity_sha256 {
                if !is_sha256(integrity) {
                    return Err(CardDocumentError::InvalidAssetIntegrity(asset.id.clone()));
                }
            }
            match &asset.fallback {
                CardAssetFallback::Initials(value) | CardAssetFallback::Abbreviation(value) => {
                    if value.trim().is_empty() {
                        return Err(CardDocumentError::InvalidAssetFallback(asset.id.clone()));
                    }
                }
                CardAssetFallback::None => {}
            }
        }

        let mut provenance_ids = HashSet::new();
        for provenance in &self.provenance {
            require_text("provenance.id", &provenance.id)?;
            require_text("provenance.label", &provenance.label)?;
            if !provenance_ids.insert(provenance.id.as_str()) {
                return Err(CardDocumentError::DuplicateProvenanceId(
                    provenance.id.clone(),
                ));
            }
            if let Some(fingerprint) = &provenance.fingerprint {
                if !is_sha256(fingerprint) {
                    return Err(CardDocumentError::InvalidProvenanceFingerprint(
                        provenance.id.clone(),
                    ));
                }
            }
        }

        if let (Some(evidence_at), Some(generated_at)) =
            (self.context.evidence_at, self.context.view.generated_at)
        {
            if evidence_at > generated_at {
                return Err(CardDocumentError::EvidenceAfterGeneration);
            }
        }

        if self.context.simulation.seed.is_some() != self.context.simulation.trials.is_some() {
            return Err(CardDocumentError::IncompleteSimulationContext);
        }
        if self.context.simulation.trials == Some(0) {
            return Err(CardDocumentError::ZeroSimulationTrials);
        }

        if self.pages.is_empty() {
            return Err(CardDocumentError::NoPages);
        }
        let mut page_ids = HashSet::new();
        let mut page_orders = HashSet::new();
        let mut section_ids = HashSet::new();
        for page in &self.pages {
            require_text("page.id", &page.id)?;
            require_text("page.literal_label", &page.literal_label)?;
            require_text("page.accessible_summary", &page.accessible_summary)?;
            if !page_ids.insert(page.id.as_str()) {
                return Err(CardDocumentError::DuplicatePageId(page.id.clone()));
            }
            if !page_orders.insert(page.order) {
                return Err(CardDocumentError::DuplicatePageOrder(page.order));
            }
            if page.sections.is_empty() {
                return Err(CardDocumentError::NoSections(page.id.clone()));
            }
            for section in &page.sections {
                require_text("section.id", section.id())?;
                if !section_ids.insert(section.id()) {
                    return Err(CardDocumentError::DuplicateSectionId(
                        section.id().to_string(),
                    ));
                }
                match section {
                    CardSectionView::IdentityHeader(header) => {
                        require_text("identity_header.title", &header.title)?;
                        if header.identities.is_empty() {
                            return Err(CardDocumentError::EmptySection(header.id.clone()));
                        }
                        let mut subjects = HashSet::new();
                        for identity in &header.identities {
                            require_text("identity.subject_id", &identity.subject_id)?;
                            require_text("identity.label", &identity.label)?;
                            if !subjects.insert(identity.subject_id.as_str()) {
                                return Err(CardDocumentError::DuplicateSubjectId(
                                    identity.subject_id.clone(),
                                ));
                            }
                            validate_asset_reference(
                                identity.asset_id.as_deref(),
                                &asset_ids,
                                &header.id,
                            )?;
                        }
                    }
                    CardSectionView::Lineup(lineup) => {
                        require_text("lineup.title", &lineup.title)?;
                        if lineup.groups.is_empty() {
                            return Err(CardDocumentError::EmptySection(lineup.id.clone()));
                        }
                        let mut group_ids = HashSet::new();
                        let mut slot_ids = HashSet::new();
                        let mut assigned_subjects = HashSet::new();
                        for group in &lineup.groups {
                            require_text("lineup.group.id", &group.id)?;
                            require_text("lineup.group.label", &group.label)?;
                            if !group_ids.insert(group.id.as_str()) {
                                return Err(CardDocumentError::DuplicateGroupId(group.id.clone()));
                            }
                            if group.slots.is_empty() {
                                return Err(CardDocumentError::EmptyLineupGroup(group.id.clone()));
                            }
                            for slot in &group.slots {
                                require_text("lineup.slot.id", &slot.id)?;
                                require_text("lineup.slot.label", &slot.label)?;
                                if !slot_ids.insert(slot.id.as_str()) {
                                    return Err(CardDocumentError::DuplicateSlotId(
                                        slot.id.clone(),
                                    ));
                                }
                                if slot.subject_id.is_some() != slot.subject_label.is_some() {
                                    return Err(CardDocumentError::IncompleteLineupSubject(
                                        slot.id.clone(),
                                    ));
                                }
                                if let Some(subject_id) = &slot.subject_id {
                                    require_text("lineup.slot.subject_id", subject_id)?;
                                    require_text(
                                        "lineup.slot.subject_label",
                                        slot.subject_label.as_deref().unwrap_or_default(),
                                    )?;
                                    if !assigned_subjects.insert(subject_id.as_str()) {
                                        return Err(CardDocumentError::DuplicateLineupSubject(
                                            subject_id.clone(),
                                        ));
                                    }
                                }
                                validate_asset_reference(
                                    slot.asset_id.as_deref(),
                                    &asset_ids,
                                    &slot.id,
                                )?;
                                validate_card_metrics(&slot.metrics, &slot.id)?;
                            }
                        }
                    }
                    CardSectionView::MetricStrip(strip) => {
                        if strip.metrics.is_empty() {
                            return Err(CardDocumentError::EmptyMetricStrip(strip.id.clone()));
                        }
                        validate_card_metrics(&strip.metrics, &strip.id)?;
                    }
                    CardSectionView::PlayerList(list) => {
                        require_text("player_list.title", &list.title)?;
                        if list.rows.is_empty() {
                            return Err(CardDocumentError::EmptySection(list.id.clone()));
                        }
                        let mut player_ids = HashSet::new();
                        for row in &list.rows {
                            require_text("player_list.player_id", &row.player_id)?;
                            require_text("player_list.name", &row.name)?;
                            if !player_ids.insert(row.player_id.as_str()) {
                                return Err(CardDocumentError::DuplicateSubjectId(
                                    row.player_id.clone(),
                                ));
                            }
                            validate_asset_reference(
                                row.asset_id.as_deref(),
                                &asset_ids,
                                &list.id,
                            )?;
                            validate_card_metrics(&row.metrics, &row.player_id)?;
                        }
                    }
                    CardSectionView::ScenarioBridge(bridge) => {
                        require_text("scenario_bridge.title", &bridge.title)?;
                        require_text("scenario_bridge.from_label", &bridge.from_label)?;
                        require_text("scenario_bridge.to_label", &bridge.to_label)?;
                        if bridge.metrics.is_empty() {
                            return Err(CardDocumentError::EmptySection(bridge.id.clone()));
                        }
                        validate_card_metrics(&bridge.metrics, &bridge.id)?;
                        if bridge
                            .metrics
                            .iter()
                            .any(|metric| metric.comparison.is_none())
                        {
                            return Err(CardDocumentError::ScenarioBridgeMissingComparison(
                                bridge.id.clone(),
                            ));
                        }
                    }
                    CardSectionView::ProbabilityRange(section) => {
                        require_text("probability_range.title", &section.title)?;
                        if section.ranges.is_empty() {
                            return Err(CardDocumentError::EmptySection(section.id.clone()));
                        }
                        let mut keys = HashSet::new();
                        for range in &section.ranges {
                            require_text("probability_range.key", &range.key)?;
                            require_text("probability_range.label", &range.label)?;
                            require_text("probability_range.display_text", &range.display_text)?;
                            require_text(
                                "probability_range.accessible_text",
                                &range.accessible_text,
                            )?;
                            if !keys.insert(range.key.as_str()) {
                                return Err(CardDocumentError::DuplicateMetricKey {
                                    section_id: section.id.clone(),
                                    metric_key: range.key.clone(),
                                });
                            }
                            validate_metric_cell(&range.low)?;
                            validate_metric_cell(&range.median)?;
                            validate_metric_cell(&range.high)?;
                            validate_probability_range(range)?;
                        }
                    }
                    CardSectionView::Decision(decision) => {
                        require_text("decision.title", &decision.title)?;
                        require_text("decision.recommendation", &decision.recommendation)?;
                        if decision.rationale.is_empty()
                            || decision.rationale.iter().any(|item| item.trim().is_empty())
                        {
                            return Err(CardDocumentError::DecisionMissingRationale(
                                decision.id.clone(),
                            ));
                        }
                        let mut alternative_ids = HashSet::new();
                        for alternative in &decision.alternatives {
                            require_text("decision.alternative.id", &alternative.id)?;
                            require_text("decision.alternative.label", &alternative.label)?;
                            if !alternative_ids.insert(alternative.id.as_str()) {
                                return Err(CardDocumentError::DuplicateAlternativeId(
                                    alternative.id.clone(),
                                ));
                            }
                        }
                    }
                    CardSectionView::Timeline(timeline) => {
                        require_text("timeline.title", &timeline.title)?;
                        if timeline.items.is_empty() {
                            return Err(CardDocumentError::EmptySection(timeline.id.clone()));
                        }
                        let mut item_ids = HashSet::new();
                        for item in &timeline.items {
                            require_text("timeline.item.id", &item.id)?;
                            require_text("timeline.item.label", &item.label)?;
                            if !item_ids.insert(item.id.as_str()) {
                                return Err(CardDocumentError::DuplicateTimelineItemId(
                                    item.id.clone(),
                                ));
                            }
                        }
                    }
                    CardSectionView::StateNotice(notice) => {
                        require_text("state_notice.title", &notice.title)?;
                        if notice
                            .detail
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .is_none()
                            && notice.empty_state.is_none()
                            && notice.warnings.is_empty()
                        {
                            return Err(CardDocumentError::EmptyStateNotice(notice.id.clone()));
                        }
                    }
                    CardSectionView::Methodology(methodology) => {
                        require_text("methodology.title", &methodology.title)?;
                        if methodology.methods.is_empty() {
                            return Err(CardDocumentError::EmptySection(methodology.id.clone()));
                        }
                        let mut method_keys = HashSet::new();
                        for method in &methodology.methods {
                            require_text("methodology.key", &method.key)?;
                            require_text("methodology.label", &method.label)?;
                            require_text("methodology.version", &method.version)?;
                            require_text("methodology.summary", &method.summary)?;
                            if !method_keys.insert(method.key.as_str()) {
                                return Err(CardDocumentError::DuplicateMethodKey(
                                    method.key.clone(),
                                ));
                            }
                        }
                        if methodology
                            .limitations
                            .iter()
                            .any(|limitation| limitation.trim().is_empty())
                        {
                            return Err(CardDocumentError::EmptyLimitation(methodology.id.clone()));
                        }
                    }
                    CardSectionView::Provenance(section) => {
                        require_text("provenance_section.title", &section.title)?;
                        if section.provenance_ids.is_empty() {
                            return Err(CardDocumentError::EmptySection(section.id.clone()));
                        }
                        let mut references = HashSet::new();
                        for provenance_id in &section.provenance_ids {
                            require_text("provenance_section.provenance_id", provenance_id)?;
                            if !references.insert(provenance_id.as_str()) {
                                return Err(CardDocumentError::DuplicateProvenanceReference(
                                    provenance_id.clone(),
                                ));
                            }
                            if !provenance_ids.contains(provenance_id.as_str()) {
                                return Err(CardDocumentError::UnknownProvenanceReference(
                                    provenance_id.clone(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        if !is_sha256(&self.fingerprint) {
            return Err(CardDocumentError::InvalidFingerprint);
        }
        if self.fingerprint != self.calculate_fingerprint()? {
            return Err(CardDocumentError::FingerprintMismatch);
        }
        Ok(())
    }
}

fn require_text(field: &'static str, value: &str) -> Result<(), CardDocumentError> {
    if value.trim().is_empty() {
        Err(CardDocumentError::MissingText(field))
    } else {
        Ok(())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_metric_value(value: &MetricValue, key: &str) -> Result<(), CardDocumentError> {
    if let MetricValue::Decimal(value) = value {
        if !value.is_finite() {
            return Err(CardDocumentError::NonFiniteMetric(key.to_string()));
        }
    }
    Ok(())
}

fn validate_metric_cell(metric: &MetricCell) -> Result<(), CardDocumentError> {
    require_text("metric.key", &metric.key.0)?;
    require_text("metric.label", &metric.label)?;
    validate_metric_value(&metric.value, &metric.key.0)
}

fn validate_card_metrics(
    metrics: &[CardMetricView],
    section_id: &str,
) -> Result<(), CardDocumentError> {
    let mut metric_keys = HashSet::new();
    for metric in metrics {
        validate_metric_cell(&metric.metric)?;
        require_text("metric.display_text", &metric.display_text)?;
        require_text("metric.accessible_text", &metric.accessible_text)?;
        if !metric_keys.insert(metric.metric.key.0.as_str()) {
            return Err(CardDocumentError::DuplicateMetricKey {
                section_id: section_id.to_string(),
                metric_key: metric.metric.key.0.clone(),
            });
        }
        if let Some(comparison) = &metric.comparison {
            require_text("metric.comparison.label", &comparison.label)?;
            validate_metric_value(&comparison.baseline, &metric.metric.key.0)?;
            validate_metric_value(&comparison.delta, &metric.metric.key.0)?;
        }
    }
    Ok(())
}

fn validate_asset_reference(
    asset_id: Option<&str>,
    asset_ids: &HashSet<&str>,
    owner_id: &str,
) -> Result<(), CardDocumentError> {
    if let Some(asset_id) = asset_id {
        require_text("asset_reference", asset_id)?;
        if !asset_ids.contains(asset_id) {
            return Err(CardDocumentError::UnknownAssetReference {
                owner_id: owner_id.to_string(),
                asset_id: asset_id.to_string(),
            });
        }
    }
    Ok(())
}

fn validate_probability_range(range: &CardProbabilityRangeView) -> Result<(), CardDocumentError> {
    if range.low.unit != range.median.unit || range.median.unit != range.high.unit {
        return Err(CardDocumentError::ProbabilityRangeUnitMismatch(
            range.key.clone(),
        ));
    }
    let numeric = |value: &MetricValue| match value {
        MetricValue::Integer(value) => Some(*value as f64),
        MetricValue::Decimal(value) if value.is_finite() => Some(*value),
        _ => None,
    };
    let (Some(low), Some(median), Some(high)) = (
        numeric(&range.low.value),
        numeric(&range.median.value),
        numeric(&range.high.value),
    ) else {
        return Err(CardDocumentError::ProbabilityRangeNotNumeric(
            range.key.clone(),
        ));
    };
    if low > median || median > high {
        return Err(CardDocumentError::ProbabilityRangeOutOfOrder(
            range.key.clone(),
        ));
    }
    Ok(())
}

fn validate_theme(theme: &CardThemeView) -> Result<(), CardDocumentError> {
    for (role, color) in [
        ("primary", theme.primary.as_deref()),
        ("secondary", theme.secondary.as_deref()),
        ("accent", theme.accent.as_deref()),
        ("surface", theme.surface.as_deref()),
        ("text", theme.text.as_deref()),
    ] {
        if let Some(color) = color {
            parse_hex_color(color)
                .ok_or_else(|| CardDocumentError::InvalidThemeColor(role.to_string()))?;
        }
    }
    if theme.minimum_text_contrast_x100 < 300 {
        return Err(CardDocumentError::InvalidThemeContrastMinimum(
            theme.minimum_text_contrast_x100,
        ));
    }
    if let (Some(surface), Some(text)) = (&theme.surface, &theme.text) {
        let ratio_x100 = (contrast_ratio(
            parse_hex_color(surface).expect("surface validated above"),
            parse_hex_color(text).expect("text validated above"),
        ) * 100.0)
            .round() as u16;
        if ratio_x100 < theme.minimum_text_contrast_x100 {
            return Err(CardDocumentError::ThemeContrastTooLow {
                actual_x100: ratio_x100,
                minimum_x100: theme.minimum_text_contrast_x100,
            });
        }
    }
    Ok(())
}

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    if value.len() != 7 || !value.starts_with('#') {
        return None;
    }
    Some([
        u8::from_str_radix(&value[1..3], 16).ok()?,
        u8::from_str_radix(&value[3..5], 16).ok()?,
        u8::from_str_radix(&value[5..7], 16).ok()?,
    ])
}

fn contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f64 {
    fn luminance(color: [u8; 3]) -> f64 {
        let channel = |value: u8| {
            let value = f64::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
    }
    let first = luminance(first);
    let second = luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CardDocumentError {
    #[error("unsupported card schema: {0}")]
    UnsupportedSchema(String),
    #[error("required card text is empty: {0}")]
    MissingText(&'static str),
    #[error("card document has no pages")]
    NoPages,
    #[error("card page has no sections: {0}")]
    NoSections(String),
    #[error("duplicate card page id: {0}")]
    DuplicatePageId(String),
    #[error("duplicate card page order: {0}")]
    DuplicatePageOrder(u16),
    #[error("duplicate card section id: {0}")]
    DuplicateSectionId(String),
    #[error("duplicate card asset id: {0}")]
    DuplicateAssetId(String),
    #[error("duplicate card provenance id: {0}")]
    DuplicateProvenanceId(String),
    #[error("card provenance has an invalid SHA-256 fingerprint: {0}")]
    InvalidProvenanceFingerprint(String),
    #[error("available card asset has no reference: {0}")]
    AvailableAssetMissingReference(String),
    #[error("card asset has an invalid reference: {0}")]
    InvalidAssetReference(String),
    #[error("card asset has an invalid SHA-256 integrity value: {0}")]
    InvalidAssetIntegrity(String),
    #[error("card asset has an empty deterministic fallback: {0}")]
    InvalidAssetFallback(String),
    #[error("duplicate required renderer capability: {0:?}")]
    DuplicateRendererCapability(CardRendererCapability),
    #[error("renderer does not support required card capability: {0:?}")]
    MissingRendererCapability(CardRendererCapability),
    #[error("metric strip has no metrics: {0}")]
    EmptyMetricStrip(String),
    #[error("duplicate metric key {metric_key} in section {section_id}")]
    DuplicateMetricKey {
        section_id: String,
        metric_key: String,
    },
    #[error("metric contains a non-finite decimal: {0}")]
    NonFiniteMetric(String),
    #[error("card section has no rows: {0}")]
    EmptySection(String),
    #[error("duplicate card subject id: {0}")]
    DuplicateSubjectId(String),
    #[error("duplicate card group id: {0}")]
    DuplicateGroupId(String),
    #[error("lineup group has no slots: {0}")]
    EmptyLineupGroup(String),
    #[error("duplicate card lineup slot id: {0}")]
    DuplicateSlotId(String),
    #[error("lineup slot subject id and label must either both be present or both absent: {0}")]
    IncompleteLineupSubject(String),
    #[error("subject appears in more than one lineup slot: {0}")]
    DuplicateLineupSubject(String),
    #[error("card owner {owner_id} references unknown asset {asset_id}")]
    UnknownAssetReference { owner_id: String, asset_id: String },
    #[error("scenario bridge metric is missing its baseline comparison: {0}")]
    ScenarioBridgeMissingComparison(String),
    #[error("probability range uses different units: {0}")]
    ProbabilityRangeUnitMismatch(String),
    #[error("probability range is not numeric: {0}")]
    ProbabilityRangeNotNumeric(String),
    #[error("probability range is not ordered low <= median <= high: {0}")]
    ProbabilityRangeOutOfOrder(String),
    #[error("decision has no non-empty rationale: {0}")]
    DecisionMissingRationale(String),
    #[error("duplicate decision alternative id: {0}")]
    DuplicateAlternativeId(String),
    #[error("duplicate timeline item id: {0}")]
    DuplicateTimelineItemId(String),
    #[error("duplicate methodology key: {0}")]
    DuplicateMethodKey(String),
    #[error("methodology contains an empty limitation: {0}")]
    EmptyLimitation(String),
    #[error("duplicate provenance reference: {0}")]
    DuplicateProvenanceReference(String),
    #[error("unknown provenance reference: {0}")]
    UnknownProvenanceReference(String),
    #[error("state notice has no detail, empty state, or warning: {0}")]
    EmptyStateNotice(String),
    #[error("card evidence time is later than its generation time")]
    EvidenceAfterGeneration,
    #[error("card theme color is not #RRGGBB: {0}")]
    InvalidThemeColor(String),
    #[error("card theme minimum contrast must be at least 3.00, got {0}")]
    InvalidThemeContrastMinimum(u16),
    #[error("card theme text contrast {actual_x100} is below required {minimum_x100}")]
    ThemeContrastTooLow { actual_x100: u16, minimum_x100: u16 },
    #[error("simulation seed and trials must either both be present or both be absent")]
    IncompleteSimulationContext,
    #[error("simulation trials must be greater than zero")]
    ZeroSimulationTrials,
    #[error("card document fingerprint is not a lowercase SHA-256 value")]
    InvalidFingerprint,
    #[error("card document fingerprint does not match its content")]
    FingerprintMismatch,
    #[error("card document serialization failed: {0}")]
    Serialization(String),
    #[error("card document deserialization failed: {0}")]
    Deserialization(String),
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::{
        model::Season,
        season_stats::SeasonType,
        teams::CANONICAL_TEAMS,
        view_model::{EmptyKind, MetricUnit, StatKey, ValuePrecision, ViewWindow},
    };

    use super::*;

    #[test]
    fn canonical_nhl_teams_have_non_fallback_card_themes() {
        let fallback = nhl_team_card_theme("UNKNOWN");

        for (abbreviation, _) in CANONICAL_TEAMS {
            let theme = nhl_team_card_theme(abbreviation);
            assert_eq!(
                theme.theme_key,
                format!("team_{}", abbreviation.to_ascii_lowercase())
            );
            assert_eq!(theme.team_abbreviation.as_deref(), Some(*abbreviation));
            assert_eq!(theme.ascii_identity, *abbreviation);
            assert_ne!(
                (&theme.primary, &theme.secondary, &theme.accent),
                (&fallback.primary, &fallback.secondary, &fallback.accent),
                "{abbreviation} must not receive the unknown-team fallback"
            );
        }
    }

    #[test]
    fn team_theme_normalizes_input_and_preserves_released_card_colors() {
        assert_eq!(nhl_team_card_theme(" nyr "), nhl_team_card_theme("NYR"));

        let rangers = nhl_team_card_theme("NYR");
        assert_eq!(rangers.primary.as_deref(), Some("#0038A8"));
        assert_eq!(rangers.secondary.as_deref(), Some("#CE1126"));
        assert_eq!(rangers.accent.as_deref(), Some("#FFFFFF"));

        let kraken = nhl_team_card_theme("SEA");
        assert_eq!(kraken.primary.as_deref(), Some("#001628"));
        assert_eq!(kraken.secondary.as_deref(), Some("#99D9D9"));
        assert_eq!(kraken.accent.as_deref(), Some("#E9072B"));
    }

    fn sample_document() -> CardDocumentView {
        let generated_at = Utc.with_ymd_and_hms(2026, 7, 21, 18, 0, 0).unwrap();
        let evidence_at = Utc.with_ymd_and_hms(2026, 7, 21, 17, 0, 0).unwrap();
        let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
        view.generated_at = Some(generated_at);
        view.data_generation = Some("roster-generation-1".to_string());

        CardDocumentView {
            schema: CARD_DOCUMENT_SCHEMA.to_string(),
            card_kind: CardKind::TeamPrognosis,
            document_id: "team-prognosis:20262027:NYR:baseline".to_string(),
            fingerprint: String::new(),
            title: "New York Rangers".to_string(),
            subtitle: Some("2026-27 prognosis".to_string()),
            context: CardContextView {
                view,
                evidence_at: Some(evidence_at),
                evidence_label: EvidenceLabel::Estimated,
                builder_version: "team-prognosis.v1".to_string(),
                methodology_versions: BTreeMap::from([(
                    "forecast".to_string(),
                    "icecast.v1".to_string(),
                )]),
                joins: CardIdentityJoinsView {
                    league_id: Some("nhl".to_string()),
                    roster_snapshot_id: Some("20262027-2026-07-21-rosters".to_string()),
                    calendar_fingerprint: Some("calendar-sha".to_string()),
                    scenario_id: Some("baseline".to_string()),
                    team_ids: vec!["NYR".to_string()],
                    ..CardIdentityJoinsView::default()
                },
                simulation: CardSimulationContextView {
                    model_id: Some("icecast".to_string()),
                    model_version: Some("v1".to_string()),
                    parameter_fingerprint: Some("parameters-sha".to_string()),
                    seed: Some(73),
                    trials: Some(10_000),
                },
            },
            theme: CardThemeView {
                theme_key: "team_nyr".to_string(),
                primary: Some("#0038A8".to_string()),
                secondary: Some("#CE1126".to_string()),
                accent: None,
                surface: Some("#FFFFFF".to_string()),
                text: Some("#111111".to_string()),
                team_abbreviation: Some("NYR".to_string()),
                ascii_identity: "NYR".to_string(),
                minimum_text_contrast_x100: 450,
            },
            required_capabilities: vec![CardRendererCapability::RasterAssets],
            pages: vec![CardPageView {
                id: "depth_chart".to_string(),
                literal_label: "Projected team lineup and IceLines player scores".to_string(),
                display_label: Some("The Depth Chart".to_string()),
                order: 1,
                accessible_summary: "Projected Rangers lineup is not available yet.".to_string(),
                sections: vec![CardSectionView::StateNotice(StateNoticeSectionView {
                    id: "lineup_unavailable".to_string(),
                    title: "Projected lineup unavailable".to_string(),
                    detail: None,
                    empty_state: Some(EmptyState {
                        kind: EmptyKind::MissingSource,
                        title: "No cross-season lineup".to_string(),
                        detail: Some(
                            "The roster and stats windows cannot yet be joined.".to_string(),
                        ),
                        recovery: Vec::new(),
                    }),
                    warnings: Vec::new(),
                    token: SemanticToken::SourceUnavailable,
                })],
            }],
            assets: vec![CardAssetView {
                id: "player:8481789:headshot".to_string(),
                subject_id: "player:8481789".to_string(),
                kind: CardAssetKind::Headshot,
                reference: Some(CardAssetReference::ExternalUrl(
                    "https://assets.nhle.com/mugs/nhl/20252026/NYR/8481789.png".to_string(),
                )),
                source: SourceKind::Roster,
                observed_at: Some(evidence_at),
                integrity_sha256: None,
                alt: "Tye Kartye headshot".to_string(),
                state: CardAssetState::Available,
                fallback: CardAssetFallback::Initials("TK".to_string()),
            }],
            provenance: vec![CardProvenanceView {
                id: "official-roster".to_string(),
                source: SourceKind::Roster,
                label: "Official NHL roster snapshot".to_string(),
                state: Completeness::Complete,
                observed_at: Some(evidence_at),
                fingerprint: Some(
                    "b3bacf1f6cf450833d3dc5f3936135be5bbc82b580dede55727537d2a8fab1cc".to_string(),
                ),
                note: None,
            }],
            warnings: Vec::new(),
            empty_state: None,
        }
    }

    fn metric_cell(key: &str, label: &str, value: f64) -> MetricCell {
        MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Decimal(value),
            unit: MetricUnit::Points,
            precision: ValuePrecision::OneDecimal,
            token: None,
        }
    }

    fn card_metric(key: &str, label: &str, value: f64) -> CardMetricView {
        CardMetricView {
            metric: metric_cell(key, label, value),
            display_text: format!("{value:.1}"),
            accessible_text: format!("{label} {value:.1}"),
            comparison: None,
            evidence_label: EvidenceLabel::Simulated,
        }
    }

    #[test]
    fn card_document_round_trips_with_stable_field_names() {
        let document = sample_document().seal().unwrap();
        let json = serde_json::to_string_pretty(&document).unwrap();
        assert!(json.contains("\"schema\": \"card_document.v1\""));
        assert!(json.contains("\"section_type\": \"state_notice\""));
        assert!(json.contains("\"roster_snapshot_id\""));
        assert!(json.contains("\"reference_type\": \"external_url\""));
        let decoded: CardDocumentView = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, document);
        decoded.validate().unwrap();
    }

    #[test]
    fn card_document_fingerprint_is_deterministic_and_content_sensitive() {
        let first = sample_document().seal().unwrap();
        let second = sample_document().seal().unwrap();
        assert_eq!(first.fingerprint, second.fingerprint);

        let mut changed = second;
        changed.title = "Changed".to_string();
        changed.refresh_fingerprint().unwrap();
        assert_ne!(first.fingerprint, changed.fingerprint);
        changed.validate().unwrap();
    }

    #[test]
    fn card_document_rejects_duplicate_page_and_section_ids() {
        let mut duplicate_page = sample_document();
        duplicate_page.pages.push(duplicate_page.pages[0].clone());
        duplicate_page.refresh_fingerprint().unwrap();
        assert!(matches!(
            duplicate_page.validate(),
            Err(CardDocumentError::DuplicatePageId(_))
        ));

        let mut duplicate_section = sample_document();
        let section = duplicate_section.pages[0].sections[0].clone();
        duplicate_section.pages[0].sections.push(section);
        duplicate_section.refresh_fingerprint().unwrap();
        assert!(matches!(
            duplicate_section.validate(),
            Err(CardDocumentError::DuplicateSectionId(_))
        ));
    }

    #[test]
    fn card_document_rejects_unsupported_schema_and_tampering() {
        let mut unsupported = sample_document();
        unsupported.schema = "card_document.v2".to_string();
        unsupported.refresh_fingerprint().unwrap();
        assert_eq!(
            unsupported.validate(),
            Err(CardDocumentError::UnsupportedSchema(
                "card_document.v2".to_string()
            ))
        );

        let mut tampered = sample_document().seal().unwrap();
        tampered.title = "Tampered".to_string();
        assert_eq!(
            tampered.validate(),
            Err(CardDocumentError::FingerprintMismatch)
        );
    }

    #[test]
    fn card_document_requires_deterministic_notice_content_and_simulation_pair() {
        let mut empty_notice = sample_document();
        let CardSectionView::StateNotice(notice) = &mut empty_notice.pages[0].sections[0] else {
            panic!("expected state notice");
        };
        notice.empty_state = None;
        empty_notice.refresh_fingerprint().unwrap();
        assert!(matches!(
            empty_notice.validate(),
            Err(CardDocumentError::EmptyStateNotice(_))
        ));

        let mut incomplete_simulation = sample_document();
        incomplete_simulation.context.simulation.trials = None;
        incomplete_simulation.refresh_fingerprint().unwrap();
        assert_eq!(
            incomplete_simulation.validate(),
            Err(CardDocumentError::IncompleteSimulationContext)
        );
    }

    #[test]
    fn metric_strip_reuses_metric_contract_and_rejects_bad_values() {
        let mut document = sample_document();
        document.pages[0].sections = vec![CardSectionView::MetricStrip(MetricStripSectionView {
            id: "headline".to_string(),
            title: Some("Season outlook".to_string()),
            metrics: vec![CardMetricView {
                metric: MetricCell {
                    key: StatKey("playoff_probability".to_string()),
                    label: "Playoffs".to_string(),
                    value: MetricValue::Decimal(65.25),
                    unit: MetricUnit::Percentage,
                    precision: ValuePrecision::PercentOneDecimal,
                    token: Some(SemanticToken::DecisionHighlight),
                },
                display_text: "65.3%".to_string(),
                accessible_text: "65.3 percent playoff probability".to_string(),
                comparison: Some(CardMetricComparisonView {
                    label: "versus prior season".to_string(),
                    baseline: MetricValue::Decimal(63.0),
                    delta: MetricValue::Decimal(2.25),
                }),
                evidence_label: EvidenceLabel::Simulated,
            }],
        })];
        document = document.seal().unwrap();
        assert!(serde_json::to_string(&document)
            .unwrap()
            .contains("\"section_type\":\"metric_strip\""));

        let CardSectionView::MetricStrip(strip) = &mut document.pages[0].sections[0] else {
            panic!("expected metric strip");
        };
        strip.metrics[0].metric.value = MetricValue::Decimal(f64::NAN);
        assert!(matches!(
            document.validate(),
            Err(CardDocumentError::NonFiniteMetric(_))
        ));
    }

    #[test]
    fn assets_require_authority_reference_and_deterministic_identity() {
        let mut missing_reference = sample_document();
        missing_reference.assets[0].reference = None;
        missing_reference.refresh_fingerprint().unwrap();
        assert!(matches!(
            missing_reference.validate(),
            Err(CardDocumentError::AvailableAssetMissingReference(_))
        ));

        let mut duplicate = sample_document();
        duplicate.assets.push(duplicate.assets[0].clone());
        duplicate.refresh_fingerprint().unwrap();
        assert!(matches!(
            duplicate.validate(),
            Err(CardDocumentError::DuplicateAssetId(_))
        ));
    }

    #[test]
    fn renderer_capabilities_and_theme_contrast_are_enforced() {
        let document = sample_document().seal().unwrap();
        assert_eq!(
            document.validate_renderer_capabilities(&[]),
            Err(CardDocumentError::MissingRendererCapability(
                CardRendererCapability::RasterAssets
            ))
        );
        document
            .validate_renderer_capabilities(&[CardRendererCapability::RasterAssets])
            .unwrap();

        let mut low_contrast = sample_document();
        low_contrast.theme.surface = Some("#FFFFFF".to_string());
        low_contrast.theme.text = Some("#F8F8F8".to_string());
        low_contrast.refresh_fingerprint().unwrap();
        assert!(matches!(
            low_contrast.validate(),
            Err(CardDocumentError::ThemeContrastTooLow { .. })
        ));
    }

    #[test]
    fn complete_section_grammar_round_trips_through_checked_schema_contract() {
        let timestamp = Utc.with_ymd_and_hms(2026, 7, 21, 17, 0, 0).unwrap();
        let mut bridge_metric = card_metric("ceiling_points", "Ceiling points", 103.8);
        bridge_metric.comparison = Some(CardMetricComparisonView {
            label: "baseline".to_string(),
            baseline: MetricValue::Decimal(98.9),
            delta: MetricValue::Decimal(4.9),
        });
        let mut document = sample_document();
        document.pages[0].sections = vec![
            CardSectionView::IdentityHeader(IdentityHeaderSectionView {
                id: "identity".to_string(),
                eyebrow: Some("2026-27".to_string()),
                title: "New York Rangers".to_string(),
                subtitle: None,
                identities: vec![CardIdentityView {
                    kind: CardIdentityKind::Team,
                    subject_id: "team:NYR".to_string(),
                    label: "New York Rangers".to_string(),
                    asset_id: None,
                }],
            }),
            CardSectionView::Lineup(LineupSectionView {
                id: "lineup".to_string(),
                title: "Projected lineup".to_string(),
                groups: vec![CardLineupGroupView {
                    id: "line-1".to_string(),
                    label: "Line 1".to_string(),
                    kind: CardLineupGroupKind::ForwardLine,
                    slots: vec![CardLineupSlotView {
                        id: "line-1-lw".to_string(),
                        label: "LW".to_string(),
                        subject_id: Some("player:8481789".to_string()),
                        subject_label: Some("Tye Kartye".to_string()),
                        asset_id: Some("player:8481789:headshot".to_string()),
                        metrics: vec![card_metric("player_score", "IceLines score", 72.0)],
                        evidence_label: EvidenceLabel::Estimated,
                    }],
                }],
            }),
            CardSectionView::MetricStrip(MetricStripSectionView {
                id: "headlines".to_string(),
                title: None,
                metrics: vec![card_metric("points", "Projected points", 98.9)],
            }),
            CardSectionView::PlayerList(PlayerListSectionView {
                id: "breakouts".to_string(),
                title: "Breakout candidates".to_string(),
                rows: vec![CardPlayerRowView {
                    player_id: "player:8481789".to_string(),
                    name: "Tye Kartye".to_string(),
                    role: Some("Top six".to_string()),
                    asset_id: Some("player:8481789:headshot".to_string()),
                    metrics: vec![card_metric("impact", "Team points impact", 1.2)],
                    tokens: vec![SemanticToken::Rising],
                    evidence_label: EvidenceLabel::Simulated,
                }],
            }),
            CardSectionView::ScenarioBridge(ScenarioBridgeSectionView {
                id: "ceiling-bridge".to_string(),
                title: "Baseline to ceiling".to_string(),
                from_label: "Baseline".to_string(),
                to_label: "All-five ceiling".to_string(),
                metrics: vec![bridge_metric],
                evidence_label: EvidenceLabel::Simulated,
            }),
            CardSectionView::ProbabilityRange(ProbabilityRangeSectionView {
                id: "range".to_string(),
                title: "Points range".to_string(),
                ranges: vec![CardProbabilityRangeView {
                    key: "points_distribution".to_string(),
                    label: "Projected points".to_string(),
                    low: metric_cell("p10", "P10", 88.0),
                    median: metric_cell("p50", "P50", 99.0),
                    high: metric_cell("p90", "P90", 110.0),
                    display_text: "88 / 99 / 110".to_string(),
                    accessible_text: "10th, 50th, and 90th percentile points".to_string(),
                    evidence_label: EvidenceLabel::Simulated,
                }],
            }),
            CardSectionView::Decision(DecisionSectionView {
                id: "decision".to_string(),
                title: "Prognosis".to_string(),
                recommendation: "Treat as a playoff team with conditional upside".to_string(),
                rationale: vec!["Median outcome clears the playoff bubble.".to_string()],
                alternatives: vec![CardDecisionAlternativeView {
                    id: "downside".to_string(),
                    label: "Downside case".to_string(),
                    detail: None,
                }],
                action_id: None,
                token: SemanticToken::PrimaryAction,
                evidence_label: EvidenceLabel::Simulated,
            }),
            CardSectionView::Timeline(TimelineSectionView {
                id: "timeline".to_string(),
                title: "Scenario events".to_string(),
                items: vec![CardTimelineItemView {
                    id: "event-1".to_string(),
                    effective_at: timestamp,
                    observed_at: Some(timestamp),
                    label: "Roster snapshot sealed".to_string(),
                    detail: None,
                    evidence_label: EvidenceLabel::Confirmed,
                    token: SemanticToken::Info,
                }],
            }),
            sample_document().pages.remove(0).sections.remove(0),
            CardSectionView::Methodology(MethodologySectionView {
                id: "methodology".to_string(),
                title: "Methodology".to_string(),
                methods: vec![CardMethodologyItemView {
                    key: "forecast".to_string(),
                    label: "IceCast".to_string(),
                    version: "v1".to_string(),
                    summary: "Seeded league simulation.".to_string(),
                }],
                limitations: vec!["Projected lineup is estimated.".to_string()],
            }),
            CardSectionView::Provenance(ProvenanceSectionView {
                id: "sources".to_string(),
                title: "Sources".to_string(),
                provenance_ids: vec!["official-roster".to_string()],
            }),
        ];

        let document = document.seal().unwrap();
        let json = serde_json::to_string(&document).unwrap();
        assert_eq!(parse_card_document(&json).unwrap(), document);

        let schema: serde_json::Value = serde_json::from_str(CARD_DOCUMENT_JSON_SCHEMA).unwrap();
        assert_eq!(
            schema["$id"],
            "https://icelines.app/schemas/card_document.v1.schema.json"
        );
        for section_type in [
            "identity_header",
            "lineup",
            "metric_strip",
            "player_list",
            "scenario_bridge",
            "probability_range",
            "decision",
            "timeline",
            "state_notice",
            "methodology",
            "provenance",
        ] {
            assert!(json.contains(&format!("\"section_type\":\"{section_type}\"")));
            assert!(CARD_DOCUMENT_JSON_SCHEMA.contains(&format!("\"{section_type}\"")));
        }
    }

    #[test]
    fn section_cross_references_and_lineup_assignments_are_validated_in_core() {
        let mut unknown_asset = sample_document();
        unknown_asset.pages[0].sections =
            vec![CardSectionView::IdentityHeader(IdentityHeaderSectionView {
                id: "identity".to_string(),
                eyebrow: None,
                title: "Rangers".to_string(),
                subtitle: None,
                identities: vec![CardIdentityView {
                    kind: CardIdentityKind::Team,
                    subject_id: "team:NYR".to_string(),
                    label: "Rangers".to_string(),
                    asset_id: Some("missing-asset".to_string()),
                }],
            })];
        unknown_asset.refresh_fingerprint().unwrap();
        assert!(matches!(
            unknown_asset.validate(),
            Err(CardDocumentError::UnknownAssetReference { .. })
        ));

        let mut unknown_provenance = sample_document();
        unknown_provenance.pages[0].sections =
            vec![CardSectionView::Provenance(ProvenanceSectionView {
                id: "sources".to_string(),
                title: "Sources".to_string(),
                provenance_ids: vec!["not-recorded".to_string()],
            })];
        unknown_provenance.refresh_fingerprint().unwrap();
        assert_eq!(
            unknown_provenance.validate(),
            Err(CardDocumentError::UnknownProvenanceReference(
                "not-recorded".to_string()
            ))
        );
    }
}
