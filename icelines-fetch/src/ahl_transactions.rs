//! Official AHL transaction-stream acquisition.
//!
//! Provider player/team IDs remain provider-local. This document is source
//! evidence; later ledgers may interpret explicit ADD/DEL sequences, but an
//! absent transaction is never an assignment or organization-status fact.

use crate::ahl::{AhlFeedClient, AhlFeedError, AHL_PROVIDER};
pub use icelines_sources::ahl::transactions::{
    parse_transaction_page, AhlTransactionError, AhlTransactionKind, AhlTransactionPageEvidence,
    AhlTransactionRow, AhlTransactionSnapshot, AhlTransactionTeamIdentity, ParsedTransactionPage,
    AHL_TRANSACTION_SNAPSHOT_SCHEMA, AHL_TRANSACTION_SOURCE_URL,
};
const TRANSACTION_PAGE_LIMIT: usize = 200;

impl From<AhlTransactionError> for AhlFeedError {
    fn from(error: AhlTransactionError) -> Self {
        match error {
            AhlTransactionError::Schema(detail) => Self::Schema(detail),
            AhlTransactionError::Validation(detail) => Self::Validation(detail),
        }
    }
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
    let expected_total = parse_transaction_page(&bootstrap, 1)
        .map_err(map_transaction_error)?
        .total_results;
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
        let parsed = parse_transaction_page(&value, page_number).map_err(map_transaction_error)?;
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
    snapshot.validate().map_err(map_transaction_error)?;
    Ok(snapshot)
}

fn map_transaction_error(error: AhlTransactionError) -> AhlFeedError {
    error.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

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
