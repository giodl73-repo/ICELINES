//! Official NHL trade-tracker adapter.
//!
//! The tracker is authoritative transaction evidence but identifies players
//! by displayed name. Every player leg is staged behind identity review; draft
//! picks and other non-player assets remain explicit ignored assets.

use super::club_publication::extract_json_string_property;
use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, EffectivePrecision, EffectiveTime, FactAuthority, FreshnessClass,
    OrganizationId, PlayerOrganizationEvent, ProposalId, ProviderId, ProviderIdentityProposal,
    ProviderPersonLocator, SourceEvidence, SourceFact, SourceId, SourceUrl, StagedAssertionId,
    StagedPlayerAssertion,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedRightsTransfer {
    pub proposal_id: ProposalId,
    pub from: OrganizationId,
    pub to: OrganizationId,
    pub occurred_at: EffectiveTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredTradeAsset {
    pub transaction_row: usize,
    pub from: OrganizationId,
    pub to: OrganizationId,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeTrackerOutput {
    pub evidence: SourceEvidence,
    pub identity_proposals: Vec<ProviderIdentityProposal>,
    pub transfers: Vec<StagedRightsTransfer>,
    pub staged_assertions: Vec<StagedPlayerAssertion>,
    pub ignored_assets: Vec<IgnoredTradeAsset>,
}

#[derive(Debug, Clone)]
pub struct NhlTradeTrackerAdapter {
    season_start_year: i32,
    captured_at: DateTime<Utc>,
    source_url: SourceUrl,
    organizations: BTreeMap<String, OrganizationId>,
}

impl NhlTradeTrackerAdapter {
    pub fn new(
        season_start_year: i32,
        captured_at: DateTime<Utc>,
        source_url: &str,
        organizations: BTreeMap<String, OrganizationId>,
    ) -> Result<Self, String> {
        if !(1917..=2200).contains(&season_start_year) {
            return Err("season_start_year is outside the supported NHL range".to_owned());
        }
        if organizations.is_empty() {
            return Err("organization registry must not be empty".to_owned());
        }
        let mut normalized = BTreeMap::new();
        for (display_name, organization) in organizations {
            let key = normalize_team_name(&display_name);
            if key.is_empty() {
                return Err("organization display name must not be empty".to_owned());
            }
            if normalized.insert(key.clone(), organization).is_some() {
                return Err(format!("duplicate organization display name {key}"));
            }
        }
        Ok(Self {
            season_start_year,
            captured_at,
            source_url: SourceUrl::try_new(source_url).map_err(|error| error.to_string())?,
            organizations: normalized,
        })
    }

    fn organization(&self, display_name: &str) -> Option<OrganizationId> {
        self.organizations
            .get(&normalize_team_name(display_name))
            .cloned()
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

impl SourceAdapter for NhlTradeTrackerAdapter {
    type Output = TradeTrackerOutput;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: SourceId::try_new(format!("nhl-trade-tracker:{}", self.season_start_year))
                .expect("validated year produces a source id"),
            provider: ProviderId::try_new("official_nhl_publication")
                .expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.article.trade_tracker")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_trade_tracker",
            supported_layouts: &["nhl_article_jsonld.trade_tracker_acquire.v1"],
            required_identity_keys: &["articleBody", "transaction_row", "displayed_name"],
            additive_field_policy: AdditiveFieldPolicy::Reject,
            freshness_class: FreshnessClass::Transactional,
            historical_availability: HistoricalAvailability::ProviderArchive,
            absence_semantics: AbsenceSemantics::NotEvidence,
            output_fact_families: &["identity_proposal", "staged_rights_transfer"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let document = std::str::from_utf8(input.bytes()).map_err(|error| {
            self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                format!("trade tracker is not UTF-8: {error}"),
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
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            self.source_url.clone(),
            descriptor.provider.clone(),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version.clone(),
        );
        let normalized_article = normalize_tracker_article(&article);
        let trade_lines = normalized_article
            .lines()
            .map(str::trim)
            .filter(|line| line.contains(": ") && line.contains(" acquire"))
            .collect::<Vec<_>>();
        if trade_lines.is_empty() {
            return Err(self.error(
                &input,
                &descriptor,
                AdapterErrorCategory::UnsupportedLayout,
                "trade tracker contains no reviewed acquire rows".to_owned(),
            ));
        }
        let mut output = TradeTrackerOutput {
            evidence: evidence.clone(),
            identity_proposals: Vec::new(),
            transfers: Vec::new(),
            staged_assertions: Vec::new(),
            ignored_assets: Vec::new(),
        };
        let mut proposal_ids = BTreeSet::new();
        for (row_index, line) in trade_lines.into_iter().enumerate() {
            let parsed = self.parse_line(line).map_err(|message| {
                self.error(
                    &input,
                    &descriptor,
                    AdapterErrorCategory::MalformedRecord,
                    format!("transaction row {}: {message}", row_index + 1),
                )
            })?;
            let legs = [
                (&parsed.acquired, &parsed.from, &parsed.to),
                (&parsed.returned, &parsed.to, &parsed.from),
            ];
            for (assets, from, to) in legs {
                for (asset_index, displayed_name) in assets.players.iter().enumerate() {
                    let proposal_id = ProposalId::try_new(format!(
                        "trade:{}:{}:{}:{}:{}",
                        parsed.date.format("%Y-%m-%d"),
                        from.as_str(),
                        to.as_str(),
                        normalized_key(displayed_name),
                        asset_index + 1
                    ))
                    .expect("validated trade fields produce a proposal id");
                    if !proposal_ids.insert(proposal_id.clone()) {
                        return Err(self.error(
                            &input,
                            &descriptor,
                            AdapterErrorCategory::SemanticValidation,
                            format!("duplicate staged player leg {proposal_id}"),
                        ));
                    }
                    output.identity_proposals.push(
                        ProviderIdentityProposal::new(
                            proposal_id.clone(),
                            ProviderPersonLocator::SourceRow {
                                source_id: input.source_id().clone(),
                                row_key: format!(
                                    "trade:{:03}:{}:{:03}",
                                    row_index + 1,
                                    from.as_str(),
                                    asset_index + 1
                                ),
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
                    let occurred_at = EffectiveTime::new(
                        Utc.from_utc_datetime(
                            &parsed.date.and_hms_opt(0, 0, 0).expect("midnight is valid"),
                        ),
                        None,
                        EffectivePrecision::Day,
                    )
                    .expect("single-ended effective time is valid");
                    output.transfers.push(StagedRightsTransfer {
                        proposal_id: proposal_id.clone(),
                        from: from.clone(),
                        to: to.clone(),
                        occurred_at: occurred_at.clone(),
                    });
                    output.staged_assertions.push(
                        StagedPlayerAssertion::new(
                            StagedAssertionId::try_new(format!(
                                "staged:{proposal_id}:rights-transfer"
                            ))
                            .expect("validated proposal id produces a staged assertion id"),
                            format!("proposal:{proposal_id}:rights-transfer"),
                            proposal_id,
                            occurred_at,
                            FactAuthority::LegalControl,
                            SourceFact::PlayerOrganization(
                                PlayerOrganizationEvent::RightsTransferred {
                                    from: from.clone(),
                                    to: to.clone(),
                                },
                            ),
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
                }
                for description in &assets.ignored {
                    output.ignored_assets.push(IgnoredTradeAsset {
                        transaction_row: row_index + 1,
                        from: from.clone(),
                        to: to.clone(),
                        description: description.clone(),
                    });
                }
            }
        }
        Ok(output)
    }
}

fn normalize_tracker_article(article: &str) -> String {
    const MONTHS: [&str; 12] = [
        "JANUARY",
        "FEBRUARY",
        "MARCH",
        "APRIL",
        "MAY",
        "JUNE",
        "JULY",
        "AUGUST",
        "SEPTEMBER",
        "OCTOBER",
        "NOVEMBER",
        "DECEMBER",
    ];
    let mut normalized = article.replace("**", "");
    for month in MONTHS {
        normalized = normalized.replace(&format!("{month} "), &format!("\n{month} "));
    }
    normalized
}

struct ParsedTradeLine {
    date: NaiveDate,
    from: OrganizationId,
    to: OrganizationId,
    acquired: ParsedAssets,
    returned: ParsedAssets,
}

#[derive(Default)]
struct ParsedAssets {
    players: Vec<String>,
    ignored: Vec<String>,
}

impl NhlTradeTrackerAdapter {
    fn parse_line(&self, line: &str) -> Result<ParsedTradeLine, String> {
        let line = line.split('|').next().unwrap_or(line).trim();
        let (date_label, transaction) = line
            .split_once(": ")
            .ok_or_else(|| "missing date delimiter".to_owned())?;
        let date = parse_tracker_date(date_label, self.season_start_year)?;
        let (destination_name, remainder) = transaction
            .split_once(" acquire")
            .ok_or_else(|| "missing acquire verb".to_owned())?;
        let to = self
            .organization(destination_name)
            .ok_or_else(|| format!("unknown destination organization `{destination_name}`"))?;
        let remainder = remainder
            .strip_prefix('s')
            .unwrap_or(remainder)
            .trim_start();
        let (acquired_clause, source_and_return) = split_from_clause(remainder)
            .ok_or_else(|| "missing source organization clause".to_owned())?;
        let (source_name, returned_clause) = source_and_return
            .split_once(" for ")
            .ok_or_else(|| "missing return-assets clause".to_owned())?;
        let from = self
            .organization(source_name.trim_start_matches("the "))
            .ok_or_else(|| format!("unknown source organization `{source_name}`"))?;
        let returned_clause = returned_clause.trim_end_matches('.').trim();
        Ok(ParsedTradeLine {
            date,
            from,
            to,
            acquired: parse_assets(acquired_clause),
            returned: parse_assets(returned_clause),
        })
    }
}

fn split_from_clause(value: &str) -> Option<(&str, &str)> {
    value
        .split_once(" from the ")
        .or_else(|| value.split_once(" from "))
}

fn parse_assets(value: &str) -> ParsedAssets {
    const MARKERS: [&str; 10] = [
        "goaltenders ",
        "goaltender ",
        "defensemen ",
        "defenseman ",
        "forwards ",
        "forward ",
        "goalies ",
        "goalie ",
        "centers ",
        "center ",
    ];
    let lower = value.to_ascii_lowercase();
    let mut positions = Vec::new();
    for marker in MARKERS {
        let mut offset = 0usize;
        while let Some(relative) = lower[offset..].find(marker) {
            let start = offset + relative;
            let boundary = start == 0 || !lower.as_bytes()[start - 1].is_ascii_alphanumeric();
            if boundary {
                positions.push((start, marker.len()));
            }
            offset = start + marker.len();
        }
    }
    positions.sort_unstable();
    positions.dedup_by_key(|position| position.0);
    let mut parsed = ParsedAssets::default();
    if positions.is_empty() {
        let value = value.trim();
        if !value.is_empty() {
            parsed.ignored.push(value.to_owned());
        }
        return parsed;
    }
    for (index, (start, marker_len)) in positions.iter().copied().enumerate() {
        let end = positions
            .get(index + 1)
            .map(|position| position.0)
            .unwrap_or(value.len());
        let fragment = value[start + marker_len..end]
            .trim()
            .trim_start_matches("and ")
            .trim_end_matches(" and")
            .trim_matches(|character: char| character == ',' || character.is_whitespace());
        for candidate in split_people(fragment) {
            if looks_like_non_player_asset(&candidate) {
                parsed.ignored.push(candidate);
            } else if !candidate.is_empty() {
                parsed.players.push(candidate);
            }
        }
    }
    parsed.ignored.sort();
    parsed.ignored.dedup();
    parsed
}

fn split_people(value: &str) -> Vec<String> {
    value
        .replace(", and ", ",")
        .replace(" and a ", ",a ")
        .replace(" and an ", ",an ")
        .replace(" and ", ",")
        .split(',')
        .map(str::trim)
        .map(|part| part.trim_end_matches('.').trim())
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect()
}

fn looks_like_non_player_asset(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("pick")
        || value.contains("consideration")
        || value.starts_with("a conditional")
        || value.starts_with("conditional")
        || value.starts_with("future ")
}

fn parse_tracker_date(value: &str, season_start_year: i32) -> Result<NaiveDate, String> {
    let mut fields = value.split_whitespace();
    let month = match fields
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase()
        .as_str()
    {
        "JANUARY" => 1,
        "FEBRUARY" => 2,
        "MARCH" => 3,
        "APRIL" => 4,
        "MAY" => 5,
        "JUNE" => 6,
        "JULY" => 7,
        "AUGUST" => 8,
        "SEPTEMBER" => 9,
        "OCTOBER" => 10,
        "NOVEMBER" => 11,
        "DECEMBER" => 12,
        _ => return Err(format!("unsupported transaction month `{value}`")),
    };
    let day = fields
        .next()
        .and_then(|day| day.parse::<u32>().ok())
        .ok_or_else(|| format!("invalid transaction day `{value}`"))?;
    if fields.next().is_some() {
        return Err(format!("unexpected transaction date fields `{value}`"));
    }
    let year = if month >= 7 {
        season_start_year
    } else {
        season_start_year + 1
    };
    NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| format!("invalid transaction date `{value}`"))
}

fn normalize_team_name(value: &str) -> String {
    value
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalized_key(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
