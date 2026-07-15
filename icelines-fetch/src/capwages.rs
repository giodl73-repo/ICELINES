//! Opt-in CapWages salary and contract adapter.
//!
//! CapWages is a licensed third-party source. Callers must supply their own
//! API key; ICELINES never persists the key in snapshots or configuration.

use crate::schema::{PlayerContract, SkaterBio};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_BASE_URL: &str = "https://capwages.com/api/gateway/v1";

#[derive(Debug, thiserror::Error)]
pub enum CapWagesError {
    #[error("CAPWAGES_API_KEY is not set")]
    MissingApiKey,
    #[error("invalid CapWages API key header")]
    InvalidApiKey,
    #[error("CapWages request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("CapWages worker failed: {0}")]
    Worker(String),
}

#[derive(Debug, Clone)]
pub struct CapWagesClient {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
    #[serde(default)]
    meta: ApiMeta,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMeta {
    last_updated: Option<String>,
    #[serde(default)]
    pagination: Pagination,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pagination {
    total_pages: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PlayerIndex {
    slug: String,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerDetail {
    nhl_id: Option<u32>,
    contracts: Vec<ContractDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractDetail {
    expiry_status: Option<String>,
    seasons: Vec<ContractSeason>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractSeason {
    season: String,
    cap_hit: Option<u64>,
    aav: Option<u64>,
    total_salary: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TeamCapSummary {
    pub team: String,
    pub roster_players: u32,
    pub valued_players: u32,
    pub total_cap_hit: u64,
    pub upper_limit: u64,
    pub cap_share_pct: f64,
}

impl CapWagesClient {
    pub fn from_env() -> Result<Self, CapWagesError> {
        let key = std::env::var("CAPWAGES_API_KEY").map_err(|_| CapWagesError::MissingApiKey)?;
        let base_url = std::env::var("ICELINES_CAPWAGES_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
        Self::new(&key, base_url)
    }

    pub fn new(key: &str, base_url: impl Into<String>) -> Result<Self, CapWagesError> {
        let mut headers = HeaderMap::new();
        let value = HeaderValue::from_str(&format!("ApiKey {key}"))
            .map_err(|_| CapWagesError::InvalidApiKey)?;
        headers.insert(AUTHORIZATION, value);
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        Ok(Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
        })
    }

    /// Fetch contract values for the supplied NHL bios. Players that cannot
    /// be matched or have no row for the requested season remain explicitly
    /// absent rather than being represented as zero-valued contracts.
    pub async fn fetch_contracts(
        &self,
        bios: &[SkaterBio],
        season: &str,
    ) -> Result<Vec<PlayerContract>, CapWagesError> {
        let mut wanted: HashMap<String, Vec<u32>> = HashMap::new();
        for bio in bios {
            wanted
                .entry(normalize_name(&bio.skater_full_name))
                .or_default()
                .push(bio.player_id);
        }
        let mut matched = Vec::new();
        let mut page = 1;
        loop {
            let response: ApiResponse<Vec<PlayerIndex>> = self
                .client
                .get(format!("{}/players", self.base_url))
                .query(&[("page", page), ("limit", 100)])
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            for player in response.data {
                if let Some(player_ids) = wanted.get(&normalize_name(&player.name)) {
                    matched.push((player.slug, player_ids.clone()));
                }
            }
            if page >= response.meta.pagination.total_pages.unwrap_or(1) {
                break;
            }
            page += 1;
        }

        let checked_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let season_label = season_label(season);
        let mut contracts = Vec::new();
        // Keep bounded concurrency: this is much faster than serial detail
        // calls without turning an opt-in run into an API traffic spike.
        for batch in matched.chunks(12) {
            let mut tasks = tokio::task::JoinSet::new();
            for (slug, expected_ids) in batch {
                let client = self.clone();
                let slug = slug.clone();
                let expected_ids = expected_ids.clone();
                let season_label = season_label.clone();
                let checked_at = checked_at.clone();
                tasks.spawn(async move {
                    client
                        .fetch_player_contract(&slug, &expected_ids, &season_label, &checked_at)
                        .await
                });
            }
            while let Some(result) = tasks.join_next().await {
                if let Ok(Ok(Some(contract))) = result {
                    contracts.push(contract);
                } else if let Ok(Err(error)) = result {
                    return Err(error);
                } else if let Err(error) = result {
                    return Err(CapWagesError::Worker(error.to_string()));
                }
            }
        }
        contracts.sort_by_key(|contract| contract.player_id);
        Ok(contracts)
    }

    async fn fetch_player_contract(
        &self,
        slug: &str,
        expected_ids: &[u32],
        season: &str,
        checked_at: &str,
    ) -> Result<Option<PlayerContract>, CapWagesError> {
        let response: ApiResponse<PlayerDetail> = self
            .client
            .get(format!("{}/players/{slug}", self.base_url))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let player_id = match response.data.nhl_id {
            Some(id) if expected_ids.contains(&id) => id,
            None if expected_ids.len() == 1 => expected_ids[0],
            _ => return Ok(None),
        };
        Ok(contract_for_season(
            player_id,
            &response.data.contracts,
            season,
            response.meta.last_updated.as_deref().unwrap_or(checked_at),
        ))
    }
}

pub fn summarize_team_caps(
    roster_players: &[(String, u32)],
    contracts: &[PlayerContract],
    upper_limit: u64,
) -> Vec<TeamCapSummary> {
    let by_player: HashMap<u32, u64> = contracts
        .iter()
        .filter_map(|contract| contract.cap_hit.map(|value| (contract.player_id, value)))
        .collect();
    let mut teams: HashMap<String, (u32, u32, u64)> = HashMap::new();
    for (team, player_id) in roster_players {
        let entry = teams.entry(team.clone()).or_default();
        entry.0 += 1;
        if let Some(cap_hit) = by_player.get(player_id) {
            entry.1 += 1;
            entry.2 += cap_hit;
        }
    }
    let mut rows: Vec<_> = teams
        .into_iter()
        .map(
            |(team, (roster_players, valued_players, total_cap_hit))| TeamCapSummary {
                team,
                roster_players,
                valued_players,
                total_cap_hit,
                upper_limit,
                cap_share_pct: if upper_limit == 0 {
                    0.0
                } else {
                    total_cap_hit as f64 * 100.0 / upper_limit as f64
                },
            },
        )
        .collect();
    rows.sort_by(|a, b| a.team.cmp(&b.team));
    rows
}

fn contract_for_season(
    player_id: u32,
    contracts: &[ContractDetail],
    season: &str,
    checked_at: &str,
) -> Option<PlayerContract> {
    contracts.iter().find_map(|contract| {
        let row = contract.seasons.iter().find(|row| row.season == season)?;
        let expiry_year = contract
            .seasons
            .iter()
            .filter_map(|row| row.season.split('-').nth(1)?.parse::<u16>().ok())
            .max()
            .map(|year| 2000 + year);
        Some(PlayerContract {
            player_id,
            valuation_season: None,
            expiry_year,
            expiry_type: contract.expiry_status.clone(),
            salary: row.total_salary,
            cap_hit: row.cap_hit,
            aav: row.aav,
            source: Some("capwages".to_owned()),
            source_url: None,
            source_checked_at: Some(checked_at.to_owned()),
        })
    })
}

fn season_label(season: &str) -> String {
    if season.len() == 8 {
        format!("{}-{}", &season[..4], &season[6..])
    } else {
        season.to_owned()
    }
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_contract_conversion_preserves_values_and_provenance() {
        let contracts = vec![ContractDetail {
            expiry_status: Some("RFA".into()),
            seasons: vec![
                ContractSeason {
                    season: "2025-26".into(),
                    cap_hit: Some(975_000),
                    aav: Some(975_000),
                    total_salary: Some(1_050_000),
                },
                ContractSeason {
                    season: "2026-27".into(),
                    cap_hit: Some(18_000_000),
                    aav: Some(18_000_000),
                    total_salary: Some(18_000_000),
                },
            ],
        }];
        let row = contract_for_season(1, &contracts, "2026-27", "2026-07-14T00:00:00Z")
            .expect("contract");
        assert_eq!(row.expiry_year, Some(2027));
        assert_eq!(row.cap_hit, Some(18_000_000));
        assert_eq!(row.source.as_deref(), Some("capwages"));
    }

    #[test]
    fn l0_season_label_converts_nhl_id() {
        assert_eq!(season_label("20252026"), "2025-26");
    }

    #[test]
    fn l0_team_summary_counts_missing_values_without_zero_filling() {
        let roster = vec![("ANA".to_owned(), 1), ("ANA".to_owned(), 2)];
        let contracts = vec![PlayerContract {
            player_id: 1,
            cap_hit: Some(18_000_000),
            ..PlayerContract::default()
        }];
        let rows = summarize_team_caps(&roster, &contracts, 104_000_000);
        assert_eq!(rows[0].roster_players, 2);
        assert_eq!(rows[0].valued_players, 1);
        assert_eq!(rows[0].total_cap_hit, 18_000_000);
        assert!((rows[0].cap_share_pct - 17.3077).abs() < 0.001);
    }
}
