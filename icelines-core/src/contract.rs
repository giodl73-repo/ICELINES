//! Phase Hart.3 — `PlayerContract`.
//!
//! Per-player contract facts. Lives in `icelines-core` so `StatsRepository`
//! and `PlayerView` can carry it without depending on `icelines-fetch`.
//! The legacy `icelines_fetch::schema::PlayerContract` stays during
//! parallel-run; Hart.5 deletes it.

use serde::{Deserialize, Serialize};

/// Per-player contract data. Keyed by `PlayerId` in `StatsRepository`,
/// so this struct does not carry the player_id field — the legacy
/// `schema::PlayerContract` does.
///
/// As of 2026-04-30 the public NHL landing API does not expose contract
/// fields; contract values are typically `None`. Future API versions or a
/// third-party source may populate them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerContract {
    /// Season whose monetary values are represented (for example, `20262027`).
    #[serde(default)]
    pub valuation_season: Option<String>,
    /// Year the contract expires (e.g. 2027).
    #[serde(default)]
    pub expiry_year: Option<u16>,
    /// Contract type: "UFA", "RFA", "ELC", etc.
    #[serde(default)]
    pub expiry_type: Option<String>,
    /// Current-season cap hit / salary in dollars.
    #[serde(default)]
    pub salary: Option<u64>,
    /// Current-season cap hit in dollars.
    #[serde(default)]
    pub cap_hit: Option<u64>,
    /// Contract average annual value in dollars.
    #[serde(default)]
    pub aav: Option<u64>,
    /// Provenance identifier for the value source.
    #[serde(default)]
    pub source: Option<String>,
    /// Direct URL supporting the imported value, when available.
    #[serde(default)]
    pub source_url: Option<String>,
    /// Timestamp reported by the source or observed during the fetch.
    #[serde(default)]
    pub source_checked_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_hart3_contract_serde_round_trip() {
        let c = PlayerContract {
            valuation_season: Some("20262027".into()),
            expiry_year: Some(2027),
            expiry_type: Some("UFA".into()),
            salary: Some(12_500_000),
            cap_hit: Some(12_500_000),
            aav: Some(12_500_000),
            source: Some("capwages".into()),
            source_url: Some("https://capwages.com".into()),
            source_checked_at: Some("2026-07-14T00:00:00Z".into()),
        };
        let s = serde_json::to_string(&c).unwrap();
        let back: PlayerContract = serde_json::from_str(&s).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn l0_hart3_contract_serde_default_on_missing() {
        // Pre-Hart bundle wouldn't have any of these fields.
        let c: PlayerContract = serde_json::from_str("{}").unwrap();
        assert_eq!(c, PlayerContract::default());
    }
}
