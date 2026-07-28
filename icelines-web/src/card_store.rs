//! Read-only web provider for sealed UI-neutral card documents.

use std::sync::OnceLock;

use icelines_core::{parse_card_document, CardDocumentView};
use thiserror::Error;

const NYR: &str = include_str!("../../examples/team-prognosis-card-nyr-2026-27.json");
const SEA: &str = include_str!("../../examples/team-prognosis-card-sea-2026-27.json");
const NYR_SEASON_SIMULATION: &str =
    include_str!("../../examples/season-simulation-card-nyr-2026-27.json");
const SEA_SEASON_SIMULATION: &str =
    include_str!("../../examples/season-simulation-card-sea-2026-27.json");
const NYR_2024_REPLAY: &str =
    include_str!("../../examples/season-simulation-card-nyr-2024-25.json");
const SEA_2024_REPLAY: &str =
    include_str!("../../examples/season-simulation-card-sea-2024-25.json");
const NYR_2024_MOVEMENT: &str =
    include_str!("../../examples/forecast-movement-card-nyr-2024-25.json");
const SEA_2024_MOVEMENT: &str =
    include_str!("../../examples/forecast-movement-card-sea-2024-25.json");
const NYR_2024_HISTORY: &str =
    include_str!("../../examples/forecast-history-card-nyr-2024-25.json");
const SEA_2024_HISTORY: &str =
    include_str!("../../examples/forecast-history-card-sea-2024-25.json");
const DEXTERS_DAWGS: &str =
    include_str!("../../examples/fantasy-roster-card-dexters-dawgs-2026-10-05.json");
const DEXTERS_DAWGS_DRAFT: &str =
    include_str!("../../examples/fantasy-draft-card-dexters-dawgs-pick-7.json");
const DEXTERS_DAWGS_MORNING: &str =
    include_str!("../../examples/fantasy-morning-card-dexters-dawgs-2026-10-08.json");
const DEXTERS_DAWGS_TRADE: &str =
    include_str!("../../examples/fantasy-trade-card-dexters-dawgs-fox-rantanen.json");

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CardStoreError {
    #[error("team prognosis cards are not available for season {0}")]
    UnsupportedSeason(u32),
    #[error("team prognosis card is not available for team {0}")]
    UnsupportedTeam(String),
    #[error("season simulation card is not available for team {0}")]
    UnsupportedSeasonSimulationTeam(String),
    #[error("forecast movement card is not available for team {0}")]
    UnsupportedForecastMovementTeam(String),
    #[error("forecast history card is not available for team {0}")]
    UnsupportedForecastHistoryTeam(String),
    #[error("fantasy roster card is not available for team {0}")]
    UnsupportedFantasyTeam(String),
    #[error("fantasy draft card is not available for team {0}")]
    UnsupportedFantasyDraftTeam(String),
    #[error("fantasy morning card is not available for team {0}")]
    UnsupportedFantasyMorningTeam(String),
    #[error("fantasy trade card is not available for team {0}")]
    UnsupportedFantasyTradeTeam(String),
    #[error("scenario '{requested}' is not available for {team}; expected '{expected}'")]
    UnsupportedScenario {
        team: String,
        requested: String,
        expected: String,
    },
}

pub fn season_simulation_card(season: u32, team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_uppercase();
    match (season, team.as_str()) {
        (20262027, "NYR") => Ok(nyr_season_simulation_card().clone()),
        (20262027, "SEA") => Ok(sea_season_simulation_card().clone()),
        (20242025, "NYR") => Ok(nyr_2024_replay_card().clone()),
        (20242025, "SEA") => Ok(sea_2024_replay_card().clone()),
        (20262027 | 20242025, _) => Err(CardStoreError::UnsupportedSeasonSimulationTeam(team)),
        _ => Err(CardStoreError::UnsupportedSeason(season)),
    }
}

pub fn forecast_movement_card(season: u32, team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_uppercase();
    match (season, team.as_str()) {
        (20242025, "NYR") => Ok(nyr_2024_movement_card().clone()),
        (20242025, "SEA") => Ok(sea_2024_movement_card().clone()),
        (20242025, _) => Err(CardStoreError::UnsupportedForecastMovementTeam(team)),
        _ => Err(CardStoreError::UnsupportedSeason(season)),
    }
}

pub fn forecast_history_card(season: u32, team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_uppercase();
    match (season, team.as_str()) {
        (20242025, "NYR") => Ok(nyr_2024_history_card().clone()),
        (20242025, "SEA") => Ok(sea_2024_history_card().clone()),
        (20242025, _) => Err(CardStoreError::UnsupportedForecastHistoryTeam(team)),
        _ => Err(CardStoreError::UnsupportedSeason(season)),
    }
}

pub fn fantasy_draft_card(team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_lowercase();
    if !matches!(team.as_str(), "dexters-dawgs" | "dexter's-dawgs" | "dex") {
        return Err(CardStoreError::UnsupportedFantasyDraftTeam(team));
    }
    Ok(dexters_dawgs_draft_card().clone())
}

pub fn fantasy_morning_card(team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_lowercase();
    if !matches!(team.as_str(), "dexters-dawgs" | "dexter's-dawgs" | "dex") {
        return Err(CardStoreError::UnsupportedFantasyMorningTeam(team));
    }
    Ok(dexters_dawgs_morning_card().clone())
}

pub fn fantasy_trade_card(team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_lowercase();
    if !matches!(team.as_str(), "dexters-dawgs" | "dexter's-dawgs" | "dex") {
        return Err(CardStoreError::UnsupportedFantasyTradeTeam(team));
    }
    Ok(dexters_dawgs_trade_card().clone())
}

pub fn fantasy_roster_card(team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_lowercase();
    if !matches!(team.as_str(), "dexters-dawgs" | "dexter's-dawgs" | "dex") {
        return Err(CardStoreError::UnsupportedFantasyTeam(team));
    }
    Ok(dexters_dawgs_card().clone())
}

pub fn default_scenario(team: &str) -> Option<&'static str> {
    match team.trim().to_ascii_uppercase().as_str() {
        "NYR" => Some("nyr-development-variance"),
        "SEA" => Some("sea-development-variance"),
        _ => None,
    }
}

pub fn team_prognosis_card(
    season: u32,
    team: &str,
    scenario: Option<&str>,
) -> Result<CardDocumentView, CardStoreError> {
    if season != 20262027 {
        return Err(CardStoreError::UnsupportedSeason(season));
    }
    let team = team.trim().to_ascii_uppercase();
    let expected =
        default_scenario(&team).ok_or_else(|| CardStoreError::UnsupportedTeam(team.clone()))?;
    let requested = scenario.unwrap_or(expected);
    if requested != expected {
        return Err(CardStoreError::UnsupportedScenario {
            team,
            requested: requested.to_string(),
            expected: expected.to_string(),
        });
    }
    let card = match team.as_str() {
        "NYR" => nyr_card(),
        "SEA" => sea_card(),
        _ => unreachable!("default_scenario accepted unsupported team"),
    };
    Ok(card.clone())
}

fn nyr_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| parse_card_document(NYR).expect("sealed NYR card fixture"))
}

fn sea_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| parse_card_document(SEA).expect("sealed SEA card fixture"))
}

fn nyr_season_simulation_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(NYR_SEASON_SIMULATION).expect("sealed NYR season simulation card")
    })
}

fn sea_season_simulation_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(SEA_SEASON_SIMULATION).expect("sealed SEA season simulation card")
    })
}

fn nyr_2024_replay_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(NYR_2024_REPLAY).expect("sealed NYR 2024-25 replay card")
    })
}

fn sea_2024_replay_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(SEA_2024_REPLAY).expect("sealed SEA 2024-25 replay card")
    })
}

fn nyr_2024_movement_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(NYR_2024_MOVEMENT).expect("sealed NYR 2024-25 movement card")
    })
}

fn sea_2024_movement_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(SEA_2024_MOVEMENT).expect("sealed SEA 2024-25 movement card")
    })
}

fn nyr_2024_history_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(NYR_2024_HISTORY).expect("sealed NYR 2024-25 forecast history card")
    })
}

fn sea_2024_history_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(SEA_2024_HISTORY).expect("sealed SEA 2024-25 forecast history card")
    })
}

fn dexters_dawgs_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| parse_card_document(DEXTERS_DAWGS).expect("sealed fantasy roster card"))
}

fn dexters_dawgs_draft_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(DEXTERS_DAWGS_DRAFT).expect("sealed fantasy draft card")
    })
}

fn dexters_dawgs_morning_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(DEXTERS_DAWGS_MORNING).expect("sealed fantasy morning card")
    })
}

fn dexters_dawgs_trade_card() -> &'static CardDocumentView {
    static CARD: OnceLock<CardDocumentView> = OnceLock::new();
    CARD.get_or_init(|| {
        parse_card_document(DEXTERS_DAWGS_TRADE).expect("sealed fantasy trade card")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_sealed_supported_dimensions() {
        let nyr = team_prognosis_card(20262027, "nyr", None).unwrap();
        assert_eq!(nyr.context.joins.team_ids, ["NYR"]);
        assert!(matches!(
            team_prognosis_card(20252026, "NYR", None),
            Err(CardStoreError::UnsupportedSeason(20252026))
        ));
        assert!(matches!(
            team_prognosis_card(20262027, "BOS", None),
            Err(CardStoreError::UnsupportedTeam(_))
        ));
        assert!(matches!(
            team_prognosis_card(20262027, "NYR", Some("sea-development-variance")),
            Err(CardStoreError::UnsupportedScenario { .. })
        ));
        let fantasy = fantasy_roster_card("dexters-dawgs").unwrap();
        assert_eq!(fantasy.context.joins.team_ids, ["dexters-dawgs"]);
        let draft = fantasy_draft_card("dex").unwrap();
        assert_eq!(draft.context.joins.team_ids, ["dexters-dawgs"]);
        let morning = fantasy_morning_card("dex").unwrap();
        assert_eq!(morning.context.joins.team_ids, ["dexters-dawgs"]);
        let trade = fantasy_trade_card("dex").unwrap();
        assert_eq!(trade.context.joins.team_ids.len(), 2);
        let nyr_sim = season_simulation_card(20262027, "NYR").unwrap();
        let sea_sim = season_simulation_card(20262027, "SEA").unwrap();
        assert_eq!(
            nyr_sim.context.simulation.parameter_fingerprint,
            sea_sim.context.simulation.parameter_fingerprint
        );
        let nyr_movement = forecast_movement_card(20242025, "NYR").unwrap();
        let sea_movement = forecast_movement_card(20242025, "SEA").unwrap();
        assert_eq!(
            nyr_movement.context.simulation.parameter_fingerprint,
            sea_movement.context.simulation.parameter_fingerprint
        );
        let nyr_history = forecast_history_card(20242025, "NYR").unwrap();
        let sea_history = forecast_history_card(20242025, "SEA").unwrap();
        assert_eq!(
            nyr_history.context.simulation.parameter_fingerprint,
            sea_history.context.simulation.parameter_fingerprint
        );
        assert_eq!(nyr_history.provenance, sea_history.provenance);
        assert!(matches!(
            fantasy_roster_card("unknown"),
            Err(CardStoreError::UnsupportedFantasyTeam(_))
        ));
        assert!(matches!(
            fantasy_draft_card("unknown"),
            Err(CardStoreError::UnsupportedFantasyDraftTeam(_))
        ));
        assert!(matches!(
            fantasy_morning_card("unknown"),
            Err(CardStoreError::UnsupportedFantasyMorningTeam(_))
        ));
        assert!(matches!(
            fantasy_trade_card("unknown"),
            Err(CardStoreError::UnsupportedFantasyTradeTeam(_))
        ));
    }
}
