//! AHL HockeyTech transaction DTOs, page parsing, and snapshot validation.

use super::roster_stats::AHL_PROVIDER;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const AHL_TRANSACTION_SNAPSHOT_SCHEMA: &str = "ahl_transaction_snapshot.v1";
pub const AHL_TRANSACTION_SOURCE_URL: &str = "https://theahl.com/stats/transactions";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AhlTransactionError {
    #[error("AHL transaction schema error: {0}")]
    Schema(String),
    #[error("AHL transaction validation error: {0}")]
    Validation(String),
}

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
    pub fn validate(&self) -> Result<(), AhlTransactionError> {
        if self.schema != AHL_TRANSACTION_SNAPSHOT_SCHEMA
            || self.provider != AHL_PROVIDER
            || self.provider_season_id.trim().is_empty()
            || self.provider_season_name.trim().is_empty()
            || self.source_url != AHL_TRANSACTION_SOURCE_URL
            || self.total_results != self.transactions.len()
            || self.teams.is_empty()
            || self.pages.is_empty()
        {
            return Err(AhlTransactionError::Validation(
                "AHL transaction snapshot envelope is incomplete or inconsistent".to_owned(),
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
            return Err(AhlTransactionError::Validation(
                "AHL transaction snapshot contains invalid pages or rows".to_owned(),
            ));
        }
        if self.pages.iter().map(|page| page.rows).sum::<usize>() != self.total_results {
            return Err(AhlTransactionError::Validation(
                "AHL transaction page counts do not reconcile".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTransactionPage {
    pub total_results: usize,
    pub rows: Vec<AhlTransactionRow>,
}

pub fn parse_transaction_page(
    value: &Value,
    source_page: usize,
) -> Result<ParsedTransactionPage, AhlTransactionError> {
    let sections = value
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("sections"))
        .and_then(Value::as_array)
        .ok_or_else(|| AhlTransactionError::Schema("transaction sections missing".to_owned()))?;
    let result_section = sections
        .iter()
        .find(|section| section.get("title").and_then(Value::as_str) == Some("transaction_results"))
        .ok_or_else(|| {
            AhlTransactionError::Schema("transaction_results section missing".to_owned())
        })?;
    let data = result_section
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| AhlTransactionError::Schema("transaction result data missing".to_owned()))?;
    let mut rows = Vec::with_capacity(data.len());
    for item in data {
        let row = item
            .get("row")
            .ok_or_else(|| AhlTransactionError::Schema("transaction row missing".to_owned()))?;
        let properties = item.get("prop").ok_or_else(|| {
            AhlTransactionError::Schema("transaction properties missing".to_owned())
        })?;
        let raw_display_name = required_string(row, "player_name")?;
        let display_name = properties
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
            provider_player_id: required_pointer_string(properties, "/player_name/playerLink")?
                .trim()
                .to_owned(),
            display_name,
            position: position_suffix(raw_display_name),
            provider_team_id: required_pointer_string(properties, "/team_city/teamLink")?
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
        .ok_or_else(|| AhlTransactionError::Schema("transaction total missing".to_owned()))?;
    Ok(ParsedTransactionPage {
        total_results,
        rows,
    })
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, AhlTransactionError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AhlTransactionError::Schema(format!("transaction {key} missing")))
}

fn required_pointer_string<'a>(
    value: &'a Value,
    pointer: &str,
) -> Result<&'a str, AhlTransactionError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AhlTransactionError::Schema(format!("transaction {pointer} missing")))
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
}
