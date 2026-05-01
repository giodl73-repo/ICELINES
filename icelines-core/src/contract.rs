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
/// fields; all three are typically `None`. Future API versions or a
/// third-party source may populate them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerContract {
    /// Year the contract expires (e.g. 2027).
    #[serde(default)]
    pub expiry_year: Option<u16>,
    /// Contract type: "UFA", "RFA", "ELC", etc.
    #[serde(default)]
    pub expiry_type: Option<String>,
    /// Current-season cap hit / salary in dollars.
    #[serde(default)]
    pub salary: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_hart3_contract_serde_round_trip() {
        let c = PlayerContract {
            expiry_year: Some(2027),
            expiry_type: Some("UFA".into()),
            salary: Some(12_500_000),
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
