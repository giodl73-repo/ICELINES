//! Validated, provenance-preserving local contract overlays.

use crate::schema::{PlayerContract, SkaterBio};
use icelines_sources::contracts_csv::{
    parse_contracts_csv, ContractCsvParseError, ContractCsvRecord,
};
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

/// Load the requested valuation season from a user-maintained CSV overlay.
/// Rows for other valid seasons are ignored, allowing one file to carry
/// multiple seasons. Unknown IDs and duplicate selected-season rows fail loud.
pub fn load_contracts_csv(
    path: &Path,
    bios: &[SkaterBio],
    valuation_season: &str,
) -> Result<Vec<PlayerContract>, ContractsCsvError> {
    let known_ids: HashSet<u32> = bios.iter().map(|bio| bio.player_id).collect();
    let bytes = std::fs::read(path).map_err(|source| ContractsCsvError::Open {
        path: path.display().to_string(),
        source: source.into(),
    })?;
    let rows = parse_contracts_csv(&bytes, valuation_season).map_err(map_parse_error)?;
    rows.into_iter()
        .map(|row| contract_from_source_row(row, &known_ids))
        .collect()
}

fn contract_from_source_row(
    row: ContractCsvRecord,
    known_ids: &HashSet<u32>,
) -> Result<PlayerContract, ContractsCsvError> {
    if !known_ids.contains(&row.nhl_id) {
        return Err(ContractsCsvError::UnknownPlayer {
            row: row.source_row,
            player_id: row.nhl_id,
        });
    }
    Ok(PlayerContract {
        player_id: row.nhl_id,
        valuation_season: Some(row.season),
        expiry_year: row.expiry_year,
        expiry_type: row.expiry_type,
        salary: row.salary,
        cap_hit: row.cap_hit,
        aav: row.aav,
        source: Some("csv".to_owned()),
        source_url: Some(row.source_url),
        source_checked_at: Some(row.checked_at),
    })
}

fn map_parse_error(error: ContractCsvParseError) -> ContractsCsvError {
    match error {
        ContractCsvParseError::Row { row, source } => ContractsCsvError::Row { row, source },
        ContractCsvParseError::InvalidSeason { row, season } => {
            ContractsCsvError::InvalidSeason { row, season }
        }
        ContractCsvParseError::DuplicatePlayer {
            row,
            player_id,
            season,
        } => ContractsCsvError::DuplicatePlayer {
            row,
            player_id,
            season,
        },
        ContractCsvParseError::MissingValue { row } => ContractsCsvError::MissingValue { row },
        ContractCsvParseError::InvalidSourceUrl { row } => {
            ContractsCsvError::InvalidSourceUrl { row }
        }
        ContractCsvParseError::InvalidCheckedAt { row } => {
            ContractsCsvError::InvalidCheckedAt { row }
        }
        ContractCsvParseError::MissingAuditLabel { row } => {
            ContractsCsvError::MissingAuditLabel { row }
        }
        ContractCsvParseError::NoRowsForSeason { season } => {
            ContractsCsvError::NoRowsForSeason { season }
        }
    }
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
        assert_eq!(
            serde_json::to_value(&rows[0]).unwrap(),
            serde_json::json!({
                "player_id": 1,
                "valuation_season": "20262027",
                "expiry_year": 2031,
                "expiry_type": "UFA",
                "salary": 18_000_000,
                "cap_hit": 18_000_000,
                "aav": 18_000_000,
                "source": "csv",
                "source_url": "https://example.com/new",
                "source_checked_at": "2026-07-14T12:00:00Z"
            })
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
