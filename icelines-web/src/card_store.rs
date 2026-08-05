//! Read-only web provider for sealed UI-neutral card documents.

use std::{collections::BTreeMap, sync::OnceLock};

use chrono::{DateTime, Utc};
use icelines_core::{
    build_prospect_arrival_card, load_organization_window_profile_inventory, parse_card_document,
    project_organization_window_card, season_stats::SeasonType, validate_organization_window_board,
    CardDocumentView, OrganizationWindowBoardView, OrganizationWindowCardError,
    ProspectArrivalCardInput, ProspectArrivalLeagueCalibrationView, Season, ViewContext,
    ViewWindow, CANONICAL_TEAMS,
};
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
#[cfg(test)]
const NYR_PROSPECT_ARRIVAL: &str =
    include_str!("../../examples/prospect-arrival-card-nyr-2026-27.json");
#[cfg(test)]
const SEA_PROSPECT_ARRIVAL: &str =
    include_str!("../../examples/prospect-arrival-card-sea-2026-27.json");
const PROSPECT_ARRIVAL_LEAGUE: &str =
    include_str!("../../examples/icecast-prospect-arrival-league-2026-27.json");
const BALANCED_ORGANIZATION_WINDOW: &str =
    include_str!("../../examples/organization-window-board-partial-2026-07-28.json");
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
    #[error("prospect arrival card is not available for team {0}")]
    UnsupportedProspectArrivalTeam(String),
    #[error("organization Window card is not available for team {0}")]
    UnsupportedOrganizationWindowTeam(String),
    #[error("organization Window frame is not available: {0}")]
    UnsupportedOrganizationWindowFrame(String),
    #[error("organization Window card projection failed: {0}")]
    InvalidOrganizationWindowCard(String),
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

pub fn prospect_arrival_card(season: u32, team: &str) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_uppercase();
    if season != 20262027 {
        return Err(CardStoreError::UnsupportedSeason(season));
    }
    prospect_arrival_cards()
        .get(&team)
        .cloned()
        .ok_or(CardStoreError::UnsupportedProspectArrivalTeam(team))
}

pub fn organization_window_card(
    season: u32,
    team: &str,
) -> Result<CardDocumentView, CardStoreError> {
    let team = team.trim().to_ascii_uppercase();
    let board = organization_window_board("balanced.v1", season)?;
    project_organization_window_card(board, &team, None, None).map_err(|error| match error {
        OrganizationWindowCardError::InvalidTeam(_)
        | OrganizationWindowCardError::MissingTeam(_) => {
            CardStoreError::UnsupportedOrganizationWindowTeam(team)
        }
        error => CardStoreError::InvalidOrganizationWindowCard(error.to_string()),
    })
}

pub fn organization_window_board(
    frame: &str,
    season: u32,
) -> Result<OrganizationWindowBoardView, CardStoreError> {
    if frame != "balanced.v1" {
        return Err(CardStoreError::UnsupportedOrganizationWindowFrame(
            frame.to_owned(),
        ));
    }
    if season != 20262027 {
        return Err(CardStoreError::UnsupportedSeason(season));
    }
    static BOARD: OnceLock<OrganizationWindowBoardView> = OnceLock::new();
    Ok(BOARD
        .get_or_init(|| {
            let board: OrganizationWindowBoardView =
                serde_json::from_str(BALANCED_ORGANIZATION_WINDOW)
                    .expect("sealed balanced organization Window board");
            let inventory = load_organization_window_profile_inventory()
                .expect("embedded organization Window profile inventory");
            validate_organization_window_board(&board, &inventory)
                .expect("embedded Window board must remain canonical and sealed");
            board
        })
        .clone())
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

fn prospect_arrival_cards() -> &'static BTreeMap<String, CardDocumentView> {
    static CARDS: OnceLock<BTreeMap<String, CardDocumentView>> = OnceLock::new();
    CARDS.get_or_init(|| {
        let arrival: ProspectArrivalLeagueCalibrationView =
            serde_json::from_str(PROSPECT_ARRIVAL_LEAGUE)
                .expect("sealed prospect arrival league calibration");
        let evidence_at = DateTime::parse_from_rfc3339("2026-09-15T12:00:00Z")
            .expect("fixed prospect arrival evidence time")
            .with_timezone(&Utc);
        CANONICAL_TEAMS
            .iter()
            .map(|(team, team_name)| {
                let mut view = ViewContext::new(ViewWindow::new(
                    Season(arrival.forecast_season),
                    SeasonType::Regular,
                ));
                view.generated_at = Some(evidence_at);
                let card = build_prospect_arrival_card(ProspectArrivalCardInput {
                    arrival: arrival.clone(),
                    focus_team: (*team).to_owned(),
                    team_name: (*team_name).to_owned(),
                    view,
                    evidence_at: Some(evidence_at),
                })
                .expect("canonical prospect arrival card projection");
                ((*team).to_owned(), card)
            })
            .collect()
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
        let nyr_arrival = prospect_arrival_card(20262027, "NYR").unwrap();
        let sea_arrival = prospect_arrival_card(20262027, "SEA").unwrap();
        assert_eq!(
            nyr_arrival.context.simulation.parameter_fingerprint,
            sea_arrival.context.simulation.parameter_fingerprint
        );
        assert_eq!(nyr_arrival.provenance, sea_arrival.provenance);
        assert_eq!(prospect_arrival_cards().len(), CANONICAL_TEAMS.len());
        for (team, team_name) in CANONICAL_TEAMS {
            let card = prospect_arrival_card(20262027, team).unwrap();
            assert_eq!(card.context.joins.team_ids, [*team]);
            assert_eq!(card.title, format!("{team_name} prospect arrivals"));
        }
        let sealed_nyr = parse_card_document(NYR_PROSPECT_ARRIVAL)
            .expect("sealed NYR prospect arrival card fixture");
        let sealed_sea = parse_card_document(SEA_PROSPECT_ARRIVAL)
            .expect("sealed SEA prospect arrival card fixture");
        assert_eq!(nyr_arrival, sealed_nyr);
        assert_eq!(sea_arrival, sealed_sea);
        assert!(matches!(
            prospect_arrival_card(20262027, "XYZ"),
            Err(CardStoreError::UnsupportedProspectArrivalTeam(_))
        ));
        for (team, _) in icelines_core::CANONICAL_TEAMS {
            let card = organization_window_card(20262027, team).unwrap();
            assert_eq!(card.context.joins.team_ids, [*team]);
            assert!(card
                .subtitle
                .as_deref()
                .is_some_and(|subtitle| subtitle.starts_with("Under review · NR")));
        }
        assert!(matches!(
            organization_window_card(20262027, "XYZ"),
            Err(CardStoreError::UnsupportedOrganizationWindowTeam(_))
        ));
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
