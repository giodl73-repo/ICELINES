//! Official AHL transaction-stream acquisition.
//!
//! Provider player/team IDs remain provider-local. This document is source
//! evidence; later ledgers may interpret explicit ADD/DEL sequences, but an
//! absent transaction is never an assignment or organization-status fact.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ahl::{AhlFeedClient, AhlFeedError, AHL_PROVIDER};

pub const AHL_TRANSACTION_SNAPSHOT_SCHEMA: &str = "ahl_transaction_snapshot.v1";
pub const AHL_TRANSACTION_SOURCE_URL: &str = "https://theahl.com/stats/transactions";
const TRANSACTION_PAGE_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlTransactionKind {
    Add,
    Delete,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionRow {
    pub transaction_date: String,
    pub provider_player_id: String,
    pub display_name: String,
    #[serde(default)]
    pub position: Option<String>,
    pub provider_team_id: String,
    pub team_display_name: String,
    pub kind: AhlTransactionKind,
    pub raw_type: String,
    pub description: String,
    pub source_page: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionPageEvidence {
    pub page: usize,
    pub first: usize,
    pub limit: usize,
    pub dataset_id: String,
    pub fetched_at: String,
    pub feed_url: String,
    pub rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionTeamIdentity {
    pub provider_team_id: String,
    pub team_code: String,
    pub team_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionSnapshot {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub provider_season_id: String,
    pub provider_season_name: String,
    pub source_url: String,
    pub total_results: usize,
    pub teams: Vec<AhlTransactionTeamIdentity>,
    pub pages: Vec<AhlTransactionPageEvidence>,
    pub transactions: Vec<AhlTransactionRow>,
    pub disclosures: Vec<String>,
}

impl AhlTransactionSnapshot {
    pub fn validate(&self) -> Result<(), AhlFeedError> {
        if self.schema != AHL_TRANSACTION_SNAPSHOT_SCHEMA
            || self.provider != AHL_PROVIDER
            || self.provider_season_id.trim().is_empty()
            || self.provider_season_name.trim().is_empty()
            || self.source_url != AHL_TRANSACTION_SOURCE_URL
            || self.total_results != self.transactions.len()
            || self.teams.is_empty()
            || self.pages.is_empty()
        {
            return Err(AhlFeedError::Validation(
                "AHL transaction snapshot envelope is incomplete or inconsistent".into(),
            ));
        }
        let mut provider_team_ids = BTreeSet::new();
        let mut page_numbers = BTreeSet::new();
        if self.teams.iter().any(|team| {
            team.provider_team_id.trim().is_empty()
                || team.team_code.trim().is_empty()
                || team.team_name.trim().is_empty()
                || !provider_team_ids.insert(team.provider_team_id.as_str())
        }) || self.pages.iter().any(|page| {
            page.page == 0
                || page.limit == 0
                || page.dataset_id.trim().is_empty()
                || page.feed_url.trim().is_empty()
                || chrono::DateTime::parse_from_rfc3339(&page.fetched_at).is_err()
                || !page_numbers.insert(page.page)
        }) || self.transactions.iter().any(|row| {
            NaiveDate::parse_from_str(&row.transaction_date, "%Y-%m-%d").is_err()
                || row.provider_player_id.trim().is_empty()
                || row.display_name.trim().is_empty()
                || row.provider_team_id.trim().is_empty()
                || !provider_team_ids.contains(row.provider_team_id.as_str())
                || row.team_display_name.trim().is_empty()
                || row.raw_type.trim().is_empty()
                || row.description.trim().is_empty()
                || !page_numbers.contains(&row.source_page)
        }) {
            return Err(AhlFeedError::Validation(
                "AHL transaction snapshot contains invalid pages or rows".into(),
            ));
        }
        let page_rows = self.pages.iter().map(|page| page.rows).sum::<usize>();
        if page_rows != self.total_results {
            return Err(AhlFeedError::Validation(
                "AHL transaction page counts do not reconcile".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ParsedTransactionPage {
    total_results: usize,
    rows: Vec<AhlTransactionRow>,
}

pub async fn fetch_ahl_transactions(
    client: &AhlFeedClient,
    season: u32,
) -> Result<AhlTransactionSnapshot, AhlFeedError> {
    let (provider_season_id, provider_season_name) = client.regular_season_identity(season).await?;
    let mut teams = client
        .official_team_identities(season, &provider_season_id)
        .await?
        .into_iter()
        .map(
            |(provider_team_id, team_code, team_name)| AhlTransactionTeamIdentity {
                provider_team_id,
                team_code,
                team_name,
            },
        )
        .collect::<Vec<_>>();
    teams.sort_by(|left, right| left.team_name.cmp(&right.team_name));
    let bootstrap_dataset = format!("icelines.ahl.{season}.transactions.page.1");
    let bootstrap_first = "0";
    let bootstrap_limit = TRANSACTION_PAGE_LIMIT.to_string();
    let bootstrap_params = [
        ("view", "transactions"),
        ("team_id", "-1"),
        ("season_id", provider_season_id.as_str()),
        ("site_id", "3"),
        ("league_id", "4"),
        ("lang", "en"),
        ("first", bootstrap_first),
        ("limit", bootstrap_limit.as_str()),
    ];
    let bootstrap = client
        .official_feed_value(&bootstrap_dataset, &bootstrap_params)
        .await?;
    let expected_total = parse_transaction_page(&bootstrap, 1)?.total_results;
    let page_count = expected_total.div_ceil(TRANSACTION_PAGE_LIMIT).max(1);
    let mut requests = Vec::with_capacity(page_count);
    let mut request_metadata = std::collections::BTreeMap::new();
    for page_number in 1..=page_count {
        let first = (page_number - 1) * TRANSACTION_PAGE_LIMIT;
        let first_string = first.to_string();
        let limit_string = TRANSACTION_PAGE_LIMIT.to_string();
        let params = [
            ("view", "transactions"),
            ("team_id", "-1"),
            ("season_id", provider_season_id.as_str()),
            ("site_id", "3"),
            ("league_id", "4"),
            ("lang", "en"),
            ("first", first_string.as_str()),
            ("limit", limit_string.as_str()),
        ];
        let dataset_id = format!("icelines.ahl.{season}.transactions.page.{page_number}");
        let feed_url = client.official_feed_url(&params);
        request_metadata.insert(dataset_id.clone(), (page_number, first, feed_url.clone()));
        requests.push((dataset_id, feed_url));
    }
    let values = client.official_feed_batch_values(requests, 6).await?;
    if values.len() != page_count {
        return Err(AhlFeedError::Schema(
            "AHL transaction batch did not return every requested page".into(),
        ));
    }
    let mut transactions = Vec::with_capacity(expected_total);
    let mut pages = Vec::with_capacity(page_count);
    for (dataset_id, value, fetched_at) in values {
        let (page_number, first, feed_url) = request_metadata
            .get(&dataset_id)
            .cloned()
            .ok_or_else(|| AhlFeedError::Schema("unexpected AHL transaction page".into()))?;
        let parsed = parse_transaction_page(&value, page_number)?;
        if parsed.total_results != expected_total {
            return Err(AhlFeedError::Schema(
                "AHL transaction total changed during pagination".into(),
            ));
        }
        let rows = parsed.rows.len();
        transactions.extend(parsed.rows);
        pages.push(AhlTransactionPageEvidence {
            page: page_number,
            first,
            limit: TRANSACTION_PAGE_LIMIT,
            dataset_id,
            fetched_at,
            feed_url,
            rows,
        });
    }
    pages.sort_by_key(|page| page.page);
    let snapshot = AhlTransactionSnapshot {
        schema: AHL_TRANSACTION_SNAPSHOT_SCHEMA.into(),
        season,
        provider: AHL_PROVIDER.into(),
        provider_season_id,
        provider_season_name,
        source_url: AHL_TRANSACTION_SOURCE_URL.into(),
        total_results: expected_total,
        teams,
        pages,
        transactions,
        disclosures: vec![
            "Provider player and team IDs are AHL HockeyTech identities and are not canonical NHL IDs.".into(),
            "ADD and DEL are preserved as dated source events. Their descriptions must be interpreted by a separate versioned state ledger.".into(),
            "No transaction absence is treated as roster assignment, departure, contract status, waiver passage, or another-league evidence.".into(),
        ],
    };
    snapshot.validate()?;
    Ok(snapshot)
}

fn parse_transaction_page(
    value: &Value,
    source_page: usize,
) -> Result<ParsedTransactionPage, AhlFeedError> {
    let sections = value
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("sections"))
        .and_then(Value::as_array)
        .ok_or_else(|| AhlFeedError::Schema("transaction sections missing".into()))?;
    let result_section = sections
        .iter()
        .find(|section| section.get("title").and_then(Value::as_str) == Some("transaction_results"))
        .ok_or_else(|| AhlFeedError::Schema("transaction_results section missing".into()))?;
    let data = result_section
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| AhlFeedError::Schema("transaction result data missing".into()))?;
    let mut rows = Vec::with_capacity(data.len());
    for item in data {
        let row = item
            .get("row")
            .ok_or_else(|| AhlFeedError::Schema("transaction row missing".into()))?;
        let props = item
            .get("prop")
            .ok_or_else(|| AhlFeedError::Schema("transaction properties missing".into()))?;
        let raw_display_name = required_string(row, "player_name")?;
        let display_name = props
            .pointer("/player_name/seoName")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(raw_display_name)
            .trim()
            .to_owned();
        let raw_type = required_string(row, "transaction_type")?
            .trim()
            .to_ascii_uppercase();
        let kind = match raw_type.as_str() {
            "ADD" => AhlTransactionKind::Add,
            "DEL" => AhlTransactionKind::Delete,
            _ => AhlTransactionKind::Other,
        };
        rows.push(AhlTransactionRow {
            transaction_date: required_string(row, "transaction_date")?.trim().to_owned(),
            provider_player_id: required_pointer_string(props, "/player_name/playerLink")?
                .trim()
                .to_owned(),
            display_name,
            position: position_suffix(raw_display_name),
            provider_team_id: required_pointer_string(props, "/team_city/teamLink")?
                .trim()
                .to_owned(),
            team_display_name: required_string(row, "team_city")?.trim().to_owned(),
            kind,
            raw_type,
            description: required_string(row, "transaction")?.trim().to_owned(),
            source_page,
        });
    }
    let total_results = sections
        .iter()
        .find(|section| section.get("title").and_then(Value::as_str) == Some("num_results"))
        .and_then(|section| section.get("data"))
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .and_then(|row| row.pointer("/row/num_results"))
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        })
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| AhlFeedError::Schema("transaction total missing".into()))?;
    Ok(ParsedTransactionPage {
        total_results,
        rows,
    })
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, AhlFeedError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AhlFeedError::Schema(format!("transaction {key} missing")))
}

fn required_pointer_string<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, AhlFeedError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AhlFeedError::Schema(format!("transaction {pointer} missing")))
}

fn position_suffix(value: &str) -> Option<String> {
    let open = value.rfind(" (")?;
    let suffix = value.get(open + 2..)?.strip_suffix(')')?;
    (!suffix.is_empty()
        && suffix.len() <= 3
        && suffix.bytes().all(|byte| byte.is_ascii_uppercase()))
    .then(|| suffix.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Value {
        serde_json::json!([{"sections":[
            {"title":"transaction_results","data":[
                {"prop":{"player_name":{"playerLink":"10467","seoName":"Ruslan Khazheyev"},"team_city":{"teamLink":"330"}},"row":{"transaction_date":"2026-04-24","player_name":"Ruslan Khazheyev (G)","team_city":"Chicago","transaction_type":"ADD","transaction":"Returned on loan from Carolina (NHL)"}},
                {"prop":{"player_name":{"playerLink":"10973","seoName":"Austin Brimmer"},"team_city":{"teamLink":"440"}},"row":{"transaction_date":"2026-04-20","player_name":"Austin Brimmer (RW)","team_city":"Abbotsford","transaction_type":"DEL","transaction":"Released from PTO"}}
            ]},
            {"title":"num_results","data":[{"row":{"num_results":2}}]}
        ]}])
    }

    #[test]
    fn parses_provider_ids_types_and_positions() {
        let parsed = parse_transaction_page(&page(), 1).unwrap();
        assert_eq!(parsed.total_results, 2);
        assert_eq!(parsed.rows[0].provider_player_id, "10467");
        assert_eq!(parsed.rows[0].position.as_deref(), Some("G"));
        assert_eq!(parsed.rows[0].kind, AhlTransactionKind::Add);
        assert_eq!(parsed.rows[1].kind, AhlTransactionKind::Delete);
    }

    #[test]
    fn rejects_missing_provider_identity() {
        let mut value = page();
        value[0]["sections"][0]["data"][0]["prop"]["player_name"] = Value::Null;
        assert!(parse_transaction_page(&value, 1).is_err());
    }

    #[test]
    fn validates_exact_page_and_event_reconciliation() {
        let parsed = parse_transaction_page(&page(), 1).unwrap();
        let snapshot = AhlTransactionSnapshot {
            schema: AHL_TRANSACTION_SNAPSHOT_SCHEMA.into(),
            season: 20252026,
            provider: AHL_PROVIDER.into(),
            provider_season_id: "90".into(),
            provider_season_name: "2025-26 Regular Season".into(),
            source_url: AHL_TRANSACTION_SOURCE_URL.into(),
            total_results: 2,
            teams: vec![
                AhlTransactionTeamIdentity {
                    provider_team_id: "330".into(),
                    team_code: "CHI".into(),
                    team_name: "Chicago Wolves".into(),
                },
                AhlTransactionTeamIdentity {
                    provider_team_id: "440".into(),
                    team_code: "ABB".into(),
                    team_name: "Abbotsford Canucks".into(),
                },
            ],
            pages: vec![AhlTransactionPageEvidence {
                page: 1,
                first: 0,
                limit: 200,
                dataset_id: "test".into(),
                fetched_at: "2026-04-24T12:00:00Z".into(),
                feed_url: "https://example.test/feed".into(),
                rows: 2,
            }],
            transactions: parsed.rows,
            disclosures: Vec::new(),
        };
        snapshot.validate().unwrap();
        let mut bad = snapshot;
        bad.transactions[0].provider_team_id = "unknown".into();
        assert!(bad.validate().is_err());
    }
}
