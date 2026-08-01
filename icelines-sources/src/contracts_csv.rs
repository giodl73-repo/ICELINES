//! Deterministic parsing for provenance-preserving contract CSV overlays.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContractCsvRecord {
    pub source_row: usize,
    pub nhl_id: u32,
    pub player: String,
    pub team: String,
    pub season: String,
    pub cap_hit: Option<u64>,
    pub aav: Option<u64>,
    pub salary: Option<u64>,
    pub expiry_year: Option<u16>,
    pub expiry_type: Option<String>,
    pub source_url: String,
    pub checked_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ContractCsvParseError {
    #[error("reading contract CSV row {row}: {source}")]
    Row { row: usize, source: csv::Error },
    #[error("contract CSV row {row}: invalid season '{season}' (expected consecutive years like 20262027)")]
    InvalidSeason { row: usize, season: String },
    #[error("contract CSV row {row}: duplicate NHL player id {player_id} for season {season}")]
    DuplicatePlayer {
        row: usize,
        player_id: u32,
        season: String,
    },
    #[error("contract CSV row {row}: cap_hit, aav, and salary are all empty")]
    MissingValue { row: usize },
    #[error("contract CSV row {row}: source_url must be an absolute http(s) URL")]
    InvalidSourceUrl { row: usize },
    #[error("contract CSV row {row}: checked_at must be an RFC 3339 timestamp")]
    InvalidCheckedAt { row: usize },
    #[error("contract CSV row {row}: player and team must not be empty")]
    MissingAuditLabel { row: usize },
    #[error("contract CSV contains no rows for valuation season {season}")]
    NoRowsForSeason { season: String },
}

#[derive(Debug, Deserialize)]
struct ContractCsvRow {
    #[serde(alias = "player_id")]
    nhl_id: u32,
    player: String,
    team: String,
    season: String,
    #[serde(default)]
    cap_hit: Option<u64>,
    #[serde(default)]
    aav: Option<u64>,
    #[serde(default)]
    salary: Option<u64>,
    #[serde(default)]
    expiry_year: Option<u16>,
    #[serde(default)]
    expiry_type: Option<String>,
    source_url: String,
    checked_at: String,
}

/// Parse one requested valuation season from caller-supplied CSV bytes.
/// Other valid seasons are ignored so a reviewed file can carry multiple
/// seasons. Identity joins remain the caller's responsibility.
pub fn parse_contracts_csv(
    bytes: &[u8],
    valuation_season: &str,
) -> Result<Vec<ContractCsvRecord>, ContractCsvParseError> {
    let mut seen = HashSet::new();
    let mut contracts = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(bytes);

    for (index, result) in reader.deserialize::<ContractCsvRow>().enumerate() {
        let row_number = index + 2;
        let row = result.map_err(|source| ContractCsvParseError::Row {
            row: row_number,
            source,
        })?;
        if !valid_season(&row.season) {
            return Err(ContractCsvParseError::InvalidSeason {
                row: row_number,
                season: row.season,
            });
        }
        if row.season != valuation_season {
            continue;
        }
        if !seen.insert(row.nhl_id) {
            return Err(ContractCsvParseError::DuplicatePlayer {
                row: row_number,
                player_id: row.nhl_id,
                season: valuation_season.to_owned(),
            });
        }
        if row.cap_hit.is_none() && row.aav.is_none() && row.salary.is_none() {
            return Err(ContractCsvParseError::MissingValue { row: row_number });
        }
        let source_url = url::Url::parse(&row.source_url)
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https"))
            .ok_or(ContractCsvParseError::InvalidSourceUrl { row: row_number })?;
        if chrono::DateTime::parse_from_rfc3339(&row.checked_at).is_err() {
            return Err(ContractCsvParseError::InvalidCheckedAt { row: row_number });
        }
        if row.player.is_empty() || row.team.is_empty() {
            return Err(ContractCsvParseError::MissingAuditLabel { row: row_number });
        }

        contracts.push(ContractCsvRecord {
            source_row: row_number,
            nhl_id: row.nhl_id,
            player: row.player,
            team: row.team,
            season: row.season,
            cap_hit: row.cap_hit,
            aav: row.aav,
            salary: row.salary,
            expiry_year: row.expiry_year,
            expiry_type: row.expiry_type.filter(|value| !value.is_empty()),
            source_url: source_url.to_string(),
            checked_at: row.checked_at,
        });
    }

    if contracts.is_empty() {
        return Err(ContractCsvParseError::NoRowsForSeason {
            season: valuation_season.to_owned(),
        });
    }
    contracts.sort_by_key(|contract| contract.nhl_id);
    Ok(contracts)
}

fn valid_season(season: &str) -> bool {
    if season.len() != 8 || !season.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Ok(start) = season[..4].parse::<u16>() else {
        return false;
    };
    let Ok(end) = season[4..].parse::<u16>() else {
        return false;
    };
    end == start + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSV: &[u8] = concat!(
        "nhl_id,player,team,season,cap_hit,aav,salary,expiry_year,expiry_type,source_url,checked_at\n",
        "2,Second Player,SEA,20262027,2000000,2000000,,2028,RFA,https://example.com/2,2026-07-14T12:00:00Z\n",
        "1,First Player,NYR,20252026,975000,975000,,2026,RFA,https://example.com/old,2026-07-14T00:00:00Z\n",
        "1,First Player,NYR,20262027,18000000,18000000,18000000,2031,UFA,https://example.com/1,2026-07-14T12:00:00Z\n"
    )
    .as_bytes();

    #[test]
    fn parses_requested_season_with_provenance_and_stable_order() {
        let rows = parse_contracts_csv(CSV, "20262027").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].nhl_id, 1);
        assert_eq!(rows[0].cap_hit, Some(18_000_000));
        assert_eq!(rows[0].source_url, "https://example.com/1");
        assert_eq!(rows[1].nhl_id, 2);
    }

    #[test]
    fn rejects_non_http_provenance() {
        let bytes = b"nhl_id,player,team,season,cap_hit,source_url,checked_at\n1,Player,NYR,20262027,1,file:///tmp/source,2026-07-14T00:00:00Z\n";
        assert!(matches!(
            parse_contracts_csv(bytes, "20262027"),
            Err(ContractCsvParseError::InvalidSourceUrl { row: 2 })
        ));
    }
}
