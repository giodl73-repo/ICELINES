//! Reviewed future draft-pick ownership CSV with row-level provenance.

use std::collections::HashSet;

use icelines_core::{TradeDraftPickOwnershipInput, TradeDraftPickOwnershipStatus};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum DraftPickOwnershipCsvError {
    #[error("reading draft-pick ownership CSV row {row}: {source}")]
    Row { row: usize, source: csv::Error },
    #[error("draft-pick ownership CSV row {row}: duplicate asset id {asset_id}")]
    DuplicateAsset { row: usize, asset_id: String },
    #[error("draft-pick ownership CSV row {row}: owner, original_team, and asset_id are required")]
    MissingIdentity { row: usize },
    #[error("draft-pick ownership CSV row {row}: draft year must be at least {minimum_year} and round must be 1-7")]
    InvalidDraftCoordinate { row: usize, minimum_year: u16 },
    #[error("draft-pick ownership CSV row {row}: source_url must be absolute http(s)")]
    InvalidSourceUrl { row: usize },
    #[error("draft-pick ownership CSV row {row}: checked_at must be RFC 3339")]
    InvalidCheckedAt { row: usize },
    #[error("draft-pick ownership CSV row {row}: conditional or encumbered rights require conditions, while unconditional rights cannot carry them")]
    InvalidConditions { row: usize },
    #[error("draft-pick ownership CSV contains no rows")]
    NoRows,
}

#[derive(Debug, Deserialize)]
struct DraftPickOwnershipCsvRow {
    asset_id: String,
    owner: String,
    original_team: String,
    draft_year: u16,
    round: u8,
    status: TradeDraftPickOwnershipStatus,
    #[serde(default)]
    conditions: Option<String>,
    source_url: String,
    checked_at: String,
}

pub fn parse_draft_pick_ownership_csv(
    bytes: &[u8],
    minimum_year: u16,
) -> Result<Vec<TradeDraftPickOwnershipInput>, DraftPickOwnershipCsvError> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(bytes);
    let mut seen = HashSet::new();
    let mut rows = Vec::new();
    for (index, result) in reader.deserialize::<DraftPickOwnershipCsvRow>().enumerate() {
        let row_number = index + 2;
        let mut row = result.map_err(|source| DraftPickOwnershipCsvError::Row {
            row: row_number,
            source,
        })?;
        if row.asset_id.is_empty() || row.owner.is_empty() || row.original_team.is_empty() {
            return Err(DraftPickOwnershipCsvError::MissingIdentity { row: row_number });
        }
        if !seen.insert(row.asset_id.clone()) {
            return Err(DraftPickOwnershipCsvError::DuplicateAsset {
                row: row_number,
                asset_id: row.asset_id,
            });
        }
        if row.draft_year < minimum_year || !(1..=7).contains(&row.round) {
            return Err(DraftPickOwnershipCsvError::InvalidDraftCoordinate {
                row: row_number,
                minimum_year,
            });
        }
        let valid_url = url::Url::parse(&row.source_url)
            .ok()
            .is_some_and(|url| matches!(url.scheme(), "http" | "https"));
        if !valid_url {
            return Err(DraftPickOwnershipCsvError::InvalidSourceUrl { row: row_number });
        }
        if chrono::DateTime::parse_from_rfc3339(&row.checked_at).is_err() {
            return Err(DraftPickOwnershipCsvError::InvalidCheckedAt { row: row_number });
        }
        row.conditions = row.conditions.filter(|value| !value.trim().is_empty());
        let resolved = row.status == TradeDraftPickOwnershipStatus::ConfirmedUnconditional;
        if (resolved && row.conditions.is_some()) || (!resolved && row.conditions.is_none()) {
            return Err(DraftPickOwnershipCsvError::InvalidConditions { row: row_number });
        }
        rows.push(TradeDraftPickOwnershipInput {
            asset_id: row.asset_id,
            owner: row.owner,
            original_team: row.original_team,
            draft_year: row.draft_year,
            round: row.round,
            status: row.status,
            conditions: row.conditions,
            source_url: row.source_url,
            observed_at: row.checked_at,
        });
    }
    if rows.is_empty() {
        return Err(DraftPickOwnershipCsvError::NoRows);
    }
    rows.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| left.draft_year.cmp(&right.draft_year))
            .then_with(|| left.round.cmp(&right.round))
            .then_with(|| left.asset_id.cmp(&right.asset_id))
    });
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_reviewed_ownership_and_preserves_conditions() {
        let bytes = b"asset_id,owner,original_team,draft_year,round,status,conditions,source_url,checked_at\nTB-2027-1,SEA,TBL,2027,1,confirmed_unconditional,,https://example.test/trade,2026-08-03T12:00:00Z\nCBJ-WPG-2027-2,SEA,CBJ,2027,2,conditional,SEA retains the lower of CBJ or WPG,https://example.test/condition,2026-08-03T12:00:00Z\n";
        let rows = parse_draft_pick_ownership_csv(bytes, 2027).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].asset_id, "TB-2027-1");
        assert_eq!(rows[1].asset_id, "CBJ-WPG-2027-2");
        assert_eq!(rows[1].status, TradeDraftPickOwnershipStatus::Conditional);
    }

    #[test]
    fn rejects_unexplained_conditional_rights() {
        let bytes = b"asset_id,owner,original_team,draft_year,round,status,conditions,source_url,checked_at\nconditional,SEA,SEA,2027,1,conditional,,https://example.test/trade,2026-08-03T12:00:00Z\n";
        assert!(matches!(
            parse_draft_pick_ownership_csv(bytes, 2027),
            Err(DraftPickOwnershipCsvError::InvalidConditions { row: 2 })
        ));
    }
}
