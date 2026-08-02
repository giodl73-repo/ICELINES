use serde::Deserialize;

use crate::schema::PlayerContract;

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(default)]
    pub meta: ApiMeta,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMeta {
    pub last_updated: Option<String>,
    #[serde(default)]
    pub pagination: Pagination,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub total_pages: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct PlayerIndex {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetail {
    pub nhl_id: Option<u32>,
    pub contracts: Vec<ContractDetail>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractDetail {
    pub expiry_status: Option<String>,
    pub seasons: Vec<ContractSeason>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractSeason {
    pub season: String,
    pub cap_hit: Option<u64>,
    pub aav: Option<u64>,
    pub total_salary: Option<u64>,
}

pub fn contract_for_season(
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

pub fn season_label(season: &str) -> String {
    if season.len() == 8 {
        format!("{}-{}", &season[..4], &season[6..])
    } else {
        season.to_owned()
    }
}

pub fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{contract_for_season, season_label, ContractDetail, ContractSeason};

    #[test]
    fn converts_contract_values_and_provenance() {
        let contracts = vec![ContractDetail {
            expiry_status: Some("RFA".into()),
            seasons: vec![ContractSeason {
                season: "2026-27".into(),
                cap_hit: Some(18_000_000),
                aav: Some(18_000_000),
                total_salary: Some(18_000_000),
            }],
        }];
        let row = contract_for_season(1, &contracts, "2026-27", "2026-07-14T00:00:00Z")
            .expect("contract");
        assert_eq!(row.expiry_year, Some(2027));
        assert_eq!(row.cap_hit, Some(18_000_000));
        assert_eq!(row.source.as_deref(), Some("capwages"));
    }

    #[test]
    fn converts_nhl_season_id_to_provider_label() {
        assert_eq!(season_label("20252026"), "2025-26");
    }
}
