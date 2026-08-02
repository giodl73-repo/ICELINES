//! Layout-driven official NHL club-publication adapters.
//!
//! Club articles generally identify people by displayed name rather than a
//! stable NHL player ID. The adapter therefore emits reviewable identity
//! proposals and staged participation rows. It cannot emit canonical player
//! facts before an identity review decision exists.

use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, Utc};
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, EffectivePrecision, EffectiveTime, FactAuthority, FreshnessClass,
    OrganizationId, ParticipationAuthority, ParticipationKind, PlayerParticipationFact, ProposalId,
    ProviderId, ProviderIdentityProposal, ProviderPersonLocator, SourceEvidence, SourceFact,
    SourceId, SourceUrl, StagedAssertionId, StagedPlayerAssertion,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubPublicationParticipant {
    pub proposal_id: ProposalId,
    pub organization: OrganizationId,
    pub season: Season,
    pub kind: ParticipationKind,
    pub authority: ParticipationAuthority,
    pub position_group: PublishedPositionGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishedPositionGroup {
    Forward,
    Defense,
    Goalie,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClubPublicationOutput {
    pub identity_proposals: Vec<ProviderIdentityProposal>,
    pub participants: Vec<ClubPublicationParticipant>,
    pub staged_assertions: Vec<StagedPlayerAssertion>,
}

#[derive(Debug, Clone)]
pub struct NhlArticleNamedSectionsCampAdapter {
    organization: OrganizationId,
    season: Season,
    kind: ParticipationKind,
    captured_at: DateTime<Utc>,
    source_url: SourceUrl,
}

/// Parses the reviewed NHL.com camp-table layout whose `Acquired` column
/// distinguishes contract attendees from free-agent invites.
#[derive(Debug, Clone)]
pub struct NhlArticleAcquiredTableCampAdapter {
    organization: OrganizationId,
    season: Season,
    kind: ParticipationKind,
    captured_at: DateTime<Utc>,
    source_url: SourceUrl,
    table_title: String,
}

impl NhlArticleAcquiredTableCampAdapter {
    pub fn new(
        organization: &str,
        season: Season,
        kind: ParticipationKind,
        captured_at: DateTime<Utc>,
        source_url: &str,
        table_title: &str,
    ) -> Result<Self, String> {
        let organization = organization.trim().to_ascii_uppercase();
        if !(2..=4).contains(&organization.len())
            || !organization.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err("organization must be a 2-4 letter NHL abbreviation".to_owned());
        }
        let table_title = table_title.trim();
        if table_title.is_empty() {
            return Err("table title must not be empty".to_owned());
        }
        Ok(Self {
            organization: OrganizationId::try_new(organization)
                .map_err(|error| error.to_string())?,
            season,
            kind,
            captured_at,
            source_url: SourceUrl::try_new(source_url).map_err(|error| error.to_string())?,
            table_title: table_title.to_owned(),
        })
    }

    fn kind_key(&self) -> &'static str {
        match self.kind {
            ParticipationKind::DevelopmentCamp => "development-camp",
            ParticipationKind::RookieCamp => "rookie-camp",
            ParticipationKind::TrainingCamp => "training-camp",
            ParticipationKind::ProspectTournament => "prospect-tournament",
        }
    }

    fn error(
        &self,
        input: &SourceInput<'_>,
        descriptor: &SourceDescriptor,
        category: AdapterErrorCategory,
        message: String,
    ) -> AdapterError {
        AdapterError {
            source_id: input.source_id().clone(),
            adapter_id: descriptor.adapter_id.clone(),
            input_hash: input.content_hash().clone(),
            category,
            disposition: AdapterDisposition::FatalSource,
            message,
        }
    }
}

impl SourceAdapter for NhlArticleAcquiredTableCampAdapter {
    type Output = ClubPublicationOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!(
                "nhl-club-publication:{}:{}:{}:acquired-table",
                self.organization.as_str(),
                self.season.0,
                self.kind_key()
            ))
            .expect("validated publication fields produce a valid source id"),
            provider: ProviderId::try_new("official_nhl_club_publication")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.article.acquired_table.camp")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_club_publication",
            supported_layouts: &["nhl_article_html.acquired_table.v1"],
            required_identity_keys: &["table_title", "position", "displayed_name", "acquired"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Roster,
            historical_availability: HistoricalAvailability::ProviderArchive,
            absence_semantics: AbsenceSemantics::NotEvidence,
            output_fact_families: &["identity_proposal", "staged_player_participation"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let html = std::str::from_utf8(input.bytes()).map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("club publication is not UTF-8: {error}"),
            )
        })?;
        let rows = parse_acquired_table(html, &self.table_title).map_err(|message| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::MalformedRecord,
                message,
            )
        })?;
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            self.source_url.clone(),
            descriptor.provider.clone(),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let mut identity_proposals = Vec::with_capacity(rows.len());
        let mut participants = Vec::with_capacity(rows.len());
        let mut staged_assertions = Vec::with_capacity(rows.len());
        for (index, row) in rows.into_iter().enumerate() {
            let row_key = format!("{}:{:03}", row.position_group.key(), index + 1);
            let proposal_id = ProposalId::try_new(format!(
                "club-camp:{}:{}:{}:acquired-table:{}",
                self.organization.as_str(),
                self.season.0,
                self.kind_key(),
                row_key
            ))
            .expect("validated publication fields produce a valid proposal id");
            identity_proposals.push(
                ProviderIdentityProposal::new(
                    proposal_id.clone(),
                    ProviderPersonLocator::SourceRow {
                        source_id: input.source_id().clone(),
                        row_key,
                    },
                    row.displayed_name,
                    None,
                    None,
                    vec![evidence.clone()],
                )
                .map_err(|error| {
                    self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
            participants.push(ClubPublicationParticipant {
                proposal_id: proposal_id.clone(),
                organization: self.organization.clone(),
                season: self.season,
                kind: self.kind,
                authority: row.authority,
                position_group: row.position_group,
            });
            staged_assertions.push(
                stage_participation(
                    proposal_id,
                    &self.organization,
                    self.season,
                    self.kind,
                    row.authority,
                    self.captured_at,
                    &evidence,
                )
                .map_err(|error| {
                    self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
        }
        Ok(ClubPublicationOutput {
            identity_proposals,
            participants,
            staged_assertions,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AcquiredTableRow {
    displayed_name: String,
    position_group: PublishedPositionGroup,
    authority: ParticipationAuthority,
}

/// Parses the official NHL.com league-wide list of players explicitly in
/// training camp on professional tryout contracts.
#[derive(Debug, Clone)]
pub struct NhlArticlePtoCampListAdapter {
    season: Season,
    captured_at: DateTime<Utc>,
    source_url: SourceUrl,
    organizations: BTreeMap<String, OrganizationId>,
}

impl NhlArticlePtoCampListAdapter {
    pub fn new(
        season: Season,
        captured_at: DateTime<Utc>,
        source_url: &str,
        organizations: BTreeMap<String, OrganizationId>,
    ) -> Result<Self, String> {
        if organizations.is_empty() {
            return Err("organization registry must not be empty".to_owned());
        }
        let mut normalized = BTreeMap::new();
        for (name, organization) in organizations {
            let key = normalize_words(&name);
            if key.is_empty() || normalized.insert(key.clone(), organization).is_some() {
                return Err(format!("invalid or duplicate organization name {name:?}"));
            }
        }
        Ok(Self {
            season,
            captured_at,
            source_url: SourceUrl::try_new(source_url).map_err(|error| error.to_string())?,
            organizations: normalized,
        })
    }

    fn error(
        &self,
        input: &SourceInput<'_>,
        descriptor: &SourceDescriptor,
        category: AdapterErrorCategory,
        message: String,
    ) -> AdapterError {
        AdapterError {
            source_id: input.source_id().clone(),
            adapter_id: descriptor.adapter_id.clone(),
            input_hash: input.content_hash().clone(),
            category,
            disposition: AdapterDisposition::FatalSource,
            message,
        }
    }
}

impl SourceAdapter for NhlArticlePtoCampListAdapter {
    type Output = ClubPublicationOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!("nhl-pto-camp-list:{}", self.season.0))
                .expect("validated season produces a source id"),
            provider: ProviderId::try_new("official_nhl_publication")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.article.pto_camp_list")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_pto_camp_list",
            supported_layouts: &["nhl_article_jsonld.pto_heading_list.v1"],
            required_identity_keys: &["articleBody", "displayed_name", "position", "team"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Transactional,
            historical_availability: HistoricalAvailability::ProviderArchive,
            absence_semantics: AbsenceSemantics::NotEvidence,
            output_fact_families: &["identity_proposal", "staged_player_participation"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let document = std::str::from_utf8(input.bytes()).map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("PTO publication is not UTF-8: {error}"),
            )
        })?;
        let article = extract_json_string_property(document, "articleBody").map_err(|message| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                message,
            )
        })?;
        if !article.contains("professional tryout contract (PTO)")
            || !article.contains("in an NHL camp on a PTO")
        {
            return Err(self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                "article lacks the reviewed league-wide PTO statement".to_owned(),
            ));
        }
        let headings = article
            .lines()
            .filter_map(|line| line.trim().strip_prefix("## "))
            .map(|line| line.replace("**", ""))
            .collect::<Vec<_>>();
        if headings.is_empty() {
            return Err(self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                "PTO article contains no reviewed player headings".to_owned(),
            ));
        }
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            self.source_url.clone(),
            descriptor.provider.clone(),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let mut identity_proposals = Vec::with_capacity(headings.len());
        let mut participants = Vec::with_capacity(headings.len());
        let mut staged_assertions = Vec::with_capacity(headings.len());
        for (index, heading) in headings.into_iter().enumerate() {
            let fields = heading.split(',').map(str::trim).collect::<Vec<_>>();
            if fields.len() != 3 || fields.iter().any(|field| field.is_empty()) {
                return Err(self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    format!("PTO heading {} is not name, position, team", index + 1),
                ));
            }
            let position_group = match fields[1].to_ascii_uppercase().as_str() {
                "F" | "C" | "LW" | "RW" => PublishedPositionGroup::Forward,
                "D" => PublishedPositionGroup::Defense,
                "G" => PublishedPositionGroup::Goalie,
                position => {
                    return Err(self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::MalformedRecord,
                        format!("PTO heading {} has position {position:?}", index + 1),
                    ))
                }
            };
            let organization = self
                .organizations
                .get(&normalize_words(fields[2]))
                .cloned()
                .ok_or_else(|| {
                    self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        format!("unknown PTO organization {:?}", fields[2]),
                    )
                })?;
            let row_key = format!("pto:{:03}", index + 1);
            let proposal_id = ProposalId::try_new(format!(
                "club-camp:{}:{}:training-camp:{}",
                organization.as_str(),
                self.season.0,
                row_key
            ))
            .expect("validated PTO fields produce a proposal id");
            identity_proposals.push(
                ProviderIdentityProposal::new(
                    proposal_id.clone(),
                    ProviderPersonLocator::SourceRow {
                        source_id: input.source_id().clone(),
                        row_key,
                    },
                    fields[0],
                    None,
                    None,
                    vec![evidence.clone()],
                )
                .map_err(|error| {
                    self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
            participants.push(ClubPublicationParticipant {
                proposal_id: proposal_id.clone(),
                organization: organization.clone(),
                season: self.season,
                kind: ParticipationKind::TrainingCamp,
                authority: ParticipationAuthority::Tryout,
                position_group,
            });
            staged_assertions.push(
                stage_participation(
                    proposal_id,
                    &organization,
                    self.season,
                    ParticipationKind::TrainingCamp,
                    ParticipationAuthority::Tryout,
                    self.captured_at,
                    &evidence,
                )
                .map_err(|error| {
                    self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
        }
        Ok(ClubPublicationOutput {
            identity_proposals,
            participants,
            staged_assertions,
        })
    }
}

impl NhlArticleNamedSectionsCampAdapter {
    pub fn new(
        organization: &str,
        season: Season,
        kind: ParticipationKind,
        captured_at: DateTime<Utc>,
        source_url: &str,
    ) -> Result<Self, String> {
        let organization = organization.trim().to_ascii_uppercase();
        if !(2..=4).contains(&organization.len())
            || !organization.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err("organization must be a 2-4 letter NHL abbreviation".to_owned());
        }
        Ok(Self {
            organization: OrganizationId::try_new(organization)
                .map_err(|error| error.to_string())?,
            season,
            kind,
            captured_at,
            source_url: SourceUrl::try_new(source_url).map_err(|error| error.to_string())?,
        })
    }

    fn kind_key(&self) -> &'static str {
        match self.kind {
            ParticipationKind::DevelopmentCamp => "development-camp",
            ParticipationKind::RookieCamp => "rookie-camp",
            ParticipationKind::TrainingCamp => "training-camp",
            ParticipationKind::ProspectTournament => "prospect-tournament",
        }
    }
}

impl SourceAdapter for NhlArticleNamedSectionsCampAdapter {
    type Output = ClubPublicationOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!(
                "nhl-club-publication:{}:{}:{}",
                self.organization.as_str(),
                self.season.0,
                self.kind_key()
            ))
            .expect("validated publication fields produce a valid source id"),
            provider: ProviderId::try_new("official_nhl_club_publication")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.article.named_sections.camp")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_club_publication",
            supported_layouts: &["nhl_article_jsonld.named_position_sections.v1"],
            required_identity_keys: &["articleBody", "displayed_name", "source_row"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Roster,
            historical_availability: HistoricalAvailability::ProviderArchive,
            absence_semantics: AbsenceSemantics::NotEvidence,
            output_fact_families: &["identity_proposal", "staged_player_participation"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let html = std::str::from_utf8(input.bytes()).map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("club publication is not UTF-8: {error}"),
            )
        })?;
        let article_body =
            extract_json_string_property(html, "articleBody").map_err(|message| {
                self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::UnsupportedLayout,
                    message,
                )
            })?;
        let mut rows = Vec::new();
        for (label, group) in [
            ("Forwards", PublishedPositionGroup::Forward),
            ("Defensemen", PublishedPositionGroup::Defense),
            ("Goaltenders", PublishedPositionGroup::Goalie),
        ] {
            let names = parse_named_section(&article_body, label).map_err(|message| {
                self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    message,
                )
            })?;
            rows.extend(names.into_iter().map(|name| (group, name)));
        }
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            self.source_url.clone(),
            descriptor.provider.clone(),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let mut identity_proposals = Vec::with_capacity(rows.len());
        let mut participants = Vec::with_capacity(rows.len());
        let mut staged_assertions = Vec::with_capacity(rows.len());
        for (index, (position_group, displayed_name)) in rows.into_iter().enumerate() {
            let row_key = format!("{}:{:03}", position_group.key(), index + 1);
            let proposal_id = ProposalId::try_new(format!(
                "club-camp:{}:{}:{}:{}",
                self.organization.as_str(),
                self.season.0,
                self.kind_key(),
                row_key
            ))
            .expect("validated publication fields produce a valid proposal id");
            identity_proposals.push(
                ProviderIdentityProposal::new(
                    proposal_id.clone(),
                    ProviderPersonLocator::SourceRow {
                        source_id: input.source_id().clone(),
                        row_key,
                    },
                    displayed_name,
                    None,
                    None,
                    vec![evidence.clone()],
                )
                .map_err(|error| {
                    self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
            participants.push(ClubPublicationParticipant {
                proposal_id: proposal_id.clone(),
                organization: self.organization.clone(),
                season: self.season,
                kind: self.kind,
                authority: ParticipationAuthority::Unknown,
                position_group,
            });
            staged_assertions.push(
                stage_participation(
                    proposal_id,
                    &self.organization,
                    self.season,
                    self.kind,
                    ParticipationAuthority::Unknown,
                    self.captured_at,
                    &evidence,
                )
                .map_err(|error| {
                    self.error(
                        &input,
                        &descriptor,
                        AdapterErrorCategory::SemanticValidation,
                        error.to_string(),
                    )
                })?,
            );
        }
        Ok(ClubPublicationOutput {
            identity_proposals,
            participants,
            staged_assertions,
        })
    }
}

impl NhlArticleNamedSectionsCampAdapter {
    fn error(
        &self,
        input: &SourceInput<'_>,
        descriptor: &SourceDescriptor,
        category: AdapterErrorCategory,
        message: String,
    ) -> AdapterError {
        AdapterError {
            source_id: input.source_id().clone(),
            adapter_id: descriptor.adapter_id.clone(),
            input_hash: input.content_hash().clone(),
            category,
            disposition: AdapterDisposition::FatalSource,
            message,
        }
    }
}

impl PublishedPositionGroup {
    fn key(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Defense => "defense",
            Self::Goalie => "goalie",
        }
    }
}

fn stage_participation(
    proposal_id: ProposalId,
    organization: &OrganizationId,
    season: Season,
    kind: ParticipationKind,
    authority: ParticipationAuthority,
    observed_by: DateTime<Utc>,
    evidence: &SourceEvidence,
) -> Result<StagedPlayerAssertion, icelines_core::source_facts::SourceContractError> {
    StagedPlayerAssertion::new(
        StagedAssertionId::try_new(format!("staged:{proposal_id}:participation"))?,
        format!("proposal:{proposal_id}:participation"),
        proposal_id,
        // Some camp publications provide no exact participation day. The
        // capture time is the observed-by bound and precision remains unknown.
        EffectiveTime::new(observed_by, None, EffectivePrecision::Unknown)?,
        FactAuthority::Attendance,
        SourceFact::PlayerParticipation(PlayerParticipationFact {
            organization: organization.clone(),
            season,
            kind,
            authority,
        }),
        vec![evidence.clone()],
    )
}

pub(crate) fn extract_json_string_property(document: &str, key: &str) -> Result<String, String> {
    let needle = format!("\"{key}\"");
    let start = document
        .find(&needle)
        .ok_or_else(|| format!("required JSON property {key} is absent"))?;
    let after_key = &document[start + needle.len()..];
    let colon = after_key
        .find(':')
        .ok_or_else(|| format!("required JSON property {key} has no value"))?;
    let mut deserializer = serde_json::Deserializer::from_str(&after_key[colon + 1..]);
    String::deserialize(&mut deserializer)
        .map_err(|error| format!("required JSON property {key} is not a string: {error}"))
}

fn parse_named_section(document: &str, label: &str) -> Result<Vec<String>, String> {
    let prefix = format!("**{label} (");
    let line = document
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))
        .ok_or_else(|| format!("required {label} section is absent"))?
        .replace('\u{a0}', " ");
    let count_start = line
        .find(&prefix)
        .map(|index| index + prefix.len())
        .expect("line was selected using prefix");
    let count_end = line[count_start..]
        .find(')')
        .map(|index| count_start + index)
        .ok_or_else(|| format!("{label} section has no closing count"))?;
    let expected: usize = line[count_start..count_end]
        .parse()
        .map_err(|_| format!("{label} section count is invalid"))?;
    let names_start = line[count_end..]
        .find(":**")
        .map(|index| count_end + index + 3)
        .ok_or_else(|| format!("{label} section delimiter is invalid"))?;
    let names: Vec<_> = line[names_start..]
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    if names.len() != expected {
        return Err(format!(
            "{label} section declared {expected} players but parsed {}",
            names.len()
        ));
    }
    Ok(names)
}

fn parse_acquired_table(
    document: &str,
    table_title: &str,
) -> Result<Vec<AcquiredTableRow>, String> {
    let title_start = document
        .find(table_title)
        .ok_or_else(|| format!("required table title {table_title:?} is absent"))?;
    let after_title = &document[title_start + table_title.len()..];
    let table_start = after_title
        .find("<table")
        .ok_or_else(|| "required camp table is absent after its title".to_owned())?;
    let table_open_end = after_title[table_start..]
        .find('>')
        .map(|offset| table_start + offset + 1)
        .ok_or_else(|| "camp table opening tag is incomplete".to_owned())?;
    let table_end = after_title[table_open_end..]
        .find("</table>")
        .map(|offset| table_open_end + offset)
        .ok_or_else(|| "camp table closing tag is absent".to_owned())?;
    let table = &after_title[table_open_end..table_end];
    let mut raw_rows = Vec::new();
    let mut remainder = table;
    while let Some(row_start) = remainder.find("<tr") {
        let after_start = &remainder[row_start..];
        let row_open_end = after_start
            .find('>')
            .ok_or_else(|| "camp table row opening tag is incomplete".to_owned())?
            + 1;
        let row_end = after_start[row_open_end..]
            .find("</tr>")
            .map(|offset| row_open_end + offset)
            .ok_or_else(|| "camp table row closing tag is absent".to_owned())?;
        raw_rows.push(extract_html_cells(&after_start[row_open_end..row_end])?);
        remainder = &after_start[row_end + "</tr>".len()..];
    }
    let mut non_empty = raw_rows.into_iter().filter(|row| !row.is_empty());
    let header = non_empty
        .next()
        .ok_or_else(|| "camp table contains no rows".to_owned())?;
    if header.len() != 5
        || !header[0].eq_ignore_ascii_case("pos")
        || !header[1].eq_ignore_ascii_case("no")
        || !header[2].eq_ignore_ascii_case("name")
        || !header[4].eq_ignore_ascii_case("acquired")
    {
        return Err(format!(
            "camp table header does not match the reviewed five-column layout: {header:?}"
        ));
    }
    let mut rows = Vec::new();
    for (index, cells) in non_empty.enumerate() {
        if cells.len() != 5 {
            return Err(format!(
                "camp table player row {} has {} cells instead of 5",
                index + 1,
                cells.len()
            ));
        }
        let position_group = match cells[0].trim().to_ascii_uppercase().as_str() {
            "F" | "C" | "LW" | "RW" => PublishedPositionGroup::Forward,
            "D" => PublishedPositionGroup::Defense,
            "G" => PublishedPositionGroup::Goalie,
            position => {
                return Err(format!(
                    "camp table player row {} has unsupported position {position:?}",
                    index + 1
                ))
            }
        };
        if cells[2].trim().is_empty() {
            return Err(format!(
                "camp table player row {} has an empty name",
                index + 1
            ));
        }
        let acquired = cells[4].trim();
        let authority = if acquired.eq_ignore_ascii_case("FA Invite") {
            ParticipationAuthority::FreeAgentInvite
        } else if acquired
            .split_whitespace()
            .last()
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case("ELC"))
        {
            ParticipationAuthority::ControlledPlayer
        } else if acquired
            .split_whitespace()
            .last()
            .is_some_and(|suffix| suffix.eq_ignore_ascii_case("draft"))
        {
            // Draft provenance is historical and cannot establish current rights.
            ParticipationAuthority::Unknown
        } else {
            return Err(format!(
                "camp table player row {} has unsupported acquired label {acquired:?}",
                index + 1
            ));
        };
        rows.push(AcquiredTableRow {
            displayed_name: cells[2].trim().to_owned(),
            position_group,
            authority,
        });
    }
    if rows.is_empty() {
        return Err("camp table contains no player rows".to_owned());
    }
    Ok(rows)
}

fn extract_html_cells(row: &str) -> Result<Vec<String>, String> {
    let mut cells = Vec::new();
    let mut remainder = row;
    loop {
        let next_td = remainder.find("<td").map(|offset| (offset, "td"));
        let next_th = remainder.find("<th").map(|offset| (offset, "th"));
        let Some((start, tag)) = [next_td, next_th]
            .into_iter()
            .flatten()
            .min_by_key(|item| item.0)
        else {
            break;
        };
        let after_start = &remainder[start..];
        let content_start = after_start
            .find('>')
            .ok_or_else(|| "camp table cell opening tag is incomplete".to_owned())?
            + 1;
        let closing = format!("</{tag}>");
        let content_end = after_start[content_start..]
            .find(&closing)
            .map(|offset| content_start + offset)
            .ok_or_else(|| format!("camp table {tag} cell closing tag is absent"))?;
        cells.push(html_text(&after_start[content_start..content_end]));
        remainder = &after_start[content_end + closing.len()..];
    }
    Ok(cells)
}

fn html_text(fragment: &str) -> String {
    let mut text = String::with_capacity(fragment.len());
    let mut inside_tag = false;
    for character in fragment.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => inside_tag = false,
            _ if !inside_tag => text.push(character),
            _ => {}
        }
    }
    html_escape::decode_html_entities(&text)
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_words(value: &str) -> String {
    value
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}
