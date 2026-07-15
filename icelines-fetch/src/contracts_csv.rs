//! Validated, provenance-preserving local contract overlays.

use crate::schema::{PlayerContract, SkaterBio};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ContractsCsvError {
    #[error("opening contract CSV {path}: {source}")]
    Open { path: String, source: csv::Error },
    #[error("reading contract CSV row {row}: {source}")]
    Row { row: usize, source: csv::Error },
    #[error("contract CSV row {row}: invalid season '{season}' (expected consecutive years like 20262027)")]
    InvalidSeason { row: usize, season: String },
    #[error("contract CSV row {row}: NHL player id {player_id} is not present in the selected bios snapshot")]
    UnknownPlayer { row: usize, player_id: u32 },
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

/// Load the requested valuation season from a user-maintained CSV overlay.
/// Rows for other valid seasons are ignored, allowing one file to carry
/// multiple seasons. Unknown IDs and duplicate selected-season rows fail loud.
pub fn load_contracts_csv(
    path: &Path,
    bios: &[SkaterBio],
    valuation_season: &str,
) -> Result<Vec<PlayerContract>, ContractsCsvError> {
    let known_ids: HashSet<u32> = bios.iter().map(|bio| bio.player_id).collect();
    let mut seen = HashSet::new();
    let mut contracts = Vec::new();
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(|source| ContractsCsvError::Open {
            path: path.display().to_string(),
            source,
        })?;

    for (index, result) in reader.deserialize::<ContractCsvRow>().enumerate() {
        let row_number = index + 2; // header is row 1
        let row = result.map_err(|source| ContractsCsvError::Row {
            row: row_number,
            source,
        })?;
        if !valid_season(&row.season) {
            return Err(ContractsCsvError::InvalidSeason {
                row: row_number,
                season: row.season,
            });
        }
        if row.season != valuation_season {
            continue;
        }
        if !known_ids.contains(&row.nhl_id) {
            return Err(ContractsCsvError::UnknownPlayer {
                row: row_number,
                player_id: row.nhl_id,
            });
        }
        if !seen.insert(row.nhl_id) {
            return Err(ContractsCsvError::DuplicatePlayer {
                row: row_number,
                player_id: row.nhl_id,
                season: valuation_season.to_owned(),
            });
        }
        if row.cap_hit.is_none() && row.aav.is_none() && row.salary.is_none() {
            return Err(ContractsCsvError::MissingValue { row: row_number });
        }
        let source_url = reqwest::Url::parse(&row.source_url)
            .ok()
            .filter(|url| matches!(url.scheme(), "http" | "https"))
            .ok_or(ContractsCsvError::InvalidSourceUrl { row: row_number })?;
        if chrono::DateTime::parse_from_rfc3339(&row.checked_at).is_err() {
            return Err(ContractsCsvError::InvalidCheckedAt { row: row_number });
        }

        // Player/team are human-audit columns. Requiring non-empty values keeps
        // the overlay reviewable even though NHL ID is the machine join key.
        if row.player.is_empty() || row.team.is_empty() {
            return Err(ContractsCsvError::MissingAuditLabel { row: row_number });
        }

        contracts.push(PlayerContract {
            player_id: row.nhl_id,
            valuation_season: Some(row.season),
            expiry_year: row.expiry_year,
            expiry_type: row.expiry_type.filter(|value| !value.is_empty()),
            salary: row.salary,
            cap_hit: row.cap_hit,
            aav: row.aav,
            source: Some("csv".to_owned()),
            source_url: Some(source_url.to_string()),
            source_checked_at: Some(row.checked_at),
        });
    }

    if contracts.is_empty() {
        return Err(ContractsCsvError::NoRowsForSeason {
            season: valuation_season.to_owned(),
        });
    }
    contracts.sort_by_key(|contract| contract.player_id);
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

    fn bio(id: u32) -> SkaterBio {
        serde_json::from_value(serde_json::json!({
            "playerId": id,
            "skaterFullName": "Test Player",
            "gamesPlayed": 1,
            "goals": 0,
            "assists": 0,
            "currentTeamAbbrev": "ANA",
            "positionCode": "C",
            "birthDate": null,
            "birthCountry": null,
            "nationalityCode": null,
            "shootsCatches": null,
            "draftYear": null,
            "draftRound": null,
            "draftOverall": null,
            "birthCity": null,
            "birthStateProvinceCode": null,
            "height": null,
            "weight": null,
            "firstSeasonForGameType": null,
            "isInHallOfFameYn": null,
            "lastName": "Player",
            "points": 0,
            "seasonId": 20252026
        }))
        .expect("bio fixture")
    }

    #[test]
    fn l0_csv_overlay_filters_season_and_preserves_provenance() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("contracts.csv");
        std::fs::write(
            &path,
            concat!(
                "nhl_id,player,team,season,cap_hit,aav,salary,expiry_year,expiry_type,source_url,checked_at\n",
                "1,Test Player,ANA,20252026,975000,975000,,2026,RFA,https://example.com/old,2026-07-14T00:00:00Z\n",
                "1,Test Player,ANA,20262027,18000000,18000000,18000000,2031,UFA,https://example.com/new,2026-07-14T12:00:00Z\n"
            ),
        )
        .unwrap();

        let rows = load_contracts_csv(&path, &[bio(1)], "20262027").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].cap_hit, Some(18_000_000));
        assert_eq!(rows[0].source.as_deref(), Some("csv"));
        assert_eq!(
            rows[0].source_url.as_deref(),
            Some("https://example.com/new")
        );
    }

    #[test]
    fn l0_csv_overlay_rejects_unknown_player() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("contracts.csv");
        std::fs::write(
            &path,
            concat!(
                "nhl_id,player,team,season,cap_hit,aav,source_url,checked_at\n",
                "2,Wrong Player,ANA,20262027,1000000,1000000,https://example.com,2026-07-14T00:00:00Z\n"
            ),
        )
        .unwrap();

        assert!(matches!(
            load_contracts_csv(&path, &[bio(1)], "20262027"),
            Err(ContractsCsvError::UnknownPlayer { player_id: 2, .. })
        ));
    }
}
