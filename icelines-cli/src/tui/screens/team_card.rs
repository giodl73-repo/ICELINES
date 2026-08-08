//! TUI projection of the UI-neutral `card_document.v1` contract.
//!
//! This module deliberately imports document/section types only. It does not
//! score players, run simulations, or rebuild scenario data.

use std::{collections::BTreeMap, sync::OnceLock};

#[cfg(test)]
use chrono::TimeZone;
use chrono::{DateTime, Utc};
use icelines_core::{
    build_prospect_arrival_card, load_organization_window_profile_inventory, parse_card_document,
    project_organization_window_card, season_stats::SeasonType, validate_organization_window_board,
    validate_prospect_authority_closure_board, CardDocumentView, CardSectionView,
    OrganizationWindowBoardView, ProspectArrivalBoardView, ProspectArrivalCardInput,
    ProspectArrivalLeagueCalibrationView, ProspectAuthorityClosureBoardView,
    ProspectAuthorityClosureDisposition, ProspectAuthorityProgressChangeKind,
    ProspectAuthorityProgressView, ProspectCensusAuthorityGapState,
    ProspectCensusReadinessBoardView, Season, ViewContext, ViewWindow, CANONICAL_TEAMS,
};
use icelines_fetch::{CardPublicationStore, CardPublicationStoreError};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::App;

const NYR_CARD_JSON: &str =
    include_str!("../../../../examples/team-prognosis-card-nyr-2026-27.json");
const SEA_CARD_JSON: &str =
    include_str!("../../../../examples/team-prognosis-card-sea-2026-27.json");
const NYR_SEASON_SIMULATION_CARD_JSON: &str =
    include_str!("../../../../examples/season-simulation-card-nyr-2026-27.json");
const SEA_SEASON_SIMULATION_CARD_JSON: &str =
    include_str!("../../../../examples/season-simulation-card-sea-2026-27.json");
const NYR_2024_REPLAY_CARD_JSON: &str =
    include_str!("../../../../examples/season-simulation-card-nyr-2024-25.json");
const SEA_2024_REPLAY_CARD_JSON: &str =
    include_str!("../../../../examples/season-simulation-card-sea-2024-25.json");
const NYR_2024_MOVEMENT_CARD_JSON: &str =
    include_str!("../../../../examples/forecast-movement-card-nyr-2024-25.json");
const SEA_2024_MOVEMENT_CARD_JSON: &str =
    include_str!("../../../../examples/forecast-movement-card-sea-2024-25.json");
const NYR_2024_HISTORY_CARD_JSON: &str =
    include_str!("../../../../examples/forecast-history-card-nyr-2024-25.json");
const SEA_2024_HISTORY_CARD_JSON: &str =
    include_str!("../../../../examples/forecast-history-card-sea-2024-25.json");
const NYR_VS_SEA_PLAYER_LINE_MATCHUP_CARD_JSON: &str =
    include_str!("../../../../examples/player-line-matchup-card-nyr-vs-sea-2026-27.json");
#[cfg(test)]
const NYR_PROSPECT_ARRIVAL_CARD_JSON: &str =
    include_str!("../../../../examples/prospect-arrival-card-nyr-2026-27.json");
#[cfg(test)]
const SEA_PROSPECT_ARRIVAL_CARD_JSON: &str =
    include_str!("../../../../examples/prospect-arrival-card-sea-2026-27.json");
const PROSPECT_ARRIVAL_LEAGUE_JSON: &str =
    include_str!("../../../../examples/icecast-prospect-arrival-league-2026-27.json");
const PROSPECT_ARRIVAL_BOARD_JSON: &str =
    include_str!("../../../../examples/prospect-arrival-board-2026-27.json");
const PROSPECT_CENSUS_READINESS_BOARD_JSON: &str =
    include_str!("../../../../examples/prospect-census-readiness-2026-27.json");
const PROSPECT_AUTHORITY_CLOSURE_BOARD_JSON: &str =
    include_str!("../../../../examples/prospect-authority-closure-2026-27.json");
const PROSPECT_AUTHORITY_PROGRESS_JSON: &str =
    include_str!("../../../../examples/prospect-authority-progress-2026-27-ahl.json");
const ORGANIZATION_WINDOW_BOARD_JSON: &str =
    include_str!("../../../../examples/organization-window-board-partial-2026-07-28.json");
const FANTASY_CARD_JSON: &str =
    include_str!("../../../../examples/fantasy-roster-card-dexters-dawgs-2026-10-05.json");
const FANTASY_DRAFT_CARD_JSON: &str =
    include_str!("../../../../examples/fantasy-draft-card-dexters-dawgs-pick-7.json");
const FANTASY_MORNING_CARD_JSON: &str =
    include_str!("../../../../examples/fantasy-morning-card-dexters-dawgs-2026-10-08.json");
const FANTASY_TRADE_CARD_JSON: &str =
    include_str!("../../../../examples/fantasy-trade-card-dexters-dawgs-fox-rantanen.json");

fn card(team: &str) -> &'static CardDocumentView {
    let upper = team.to_ascii_uppercase();
    if let Some(arrival_team) = upper.strip_prefix("ARRIVAL-") {
        return prospect_arrival_cards()
            .get(arrival_team)
            .expect("canonical prospect arrival team");
    }
    if let Some(window_team) = upper.strip_prefix("WINDOW-") {
        return organization_window_cards()
            .get(window_team)
            .expect("canonical organization Window team");
    }
    static NYR: OnceLock<CardDocumentView> = OnceLock::new();
    static SEA: OnceLock<CardDocumentView> = OnceLock::new();
    static FANTASY: OnceLock<CardDocumentView> = OnceLock::new();
    static FANTASY_DRAFT: OnceLock<CardDocumentView> = OnceLock::new();
    static FANTASY_MORNING: OnceLock<CardDocumentView> = OnceLock::new();
    static FANTASY_TRADE: OnceLock<CardDocumentView> = OnceLock::new();
    static NYR_SEASON_SIMULATION: OnceLock<CardDocumentView> = OnceLock::new();
    static SEA_SEASON_SIMULATION: OnceLock<CardDocumentView> = OnceLock::new();
    static NYR_2024_REPLAY: OnceLock<CardDocumentView> = OnceLock::new();
    static SEA_2024_REPLAY: OnceLock<CardDocumentView> = OnceLock::new();
    static NYR_2024_MOVEMENT: OnceLock<CardDocumentView> = OnceLock::new();
    static SEA_2024_MOVEMENT: OnceLock<CardDocumentView> = OnceLock::new();
    static NYR_2024_HISTORY: OnceLock<CardDocumentView> = OnceLock::new();
    static SEA_2024_HISTORY: OnceLock<CardDocumentView> = OnceLock::new();
    static NYR_VS_SEA_PLAYER_LINE_MATCHUP: OnceLock<CardDocumentView> = OnceLock::new();
    match upper.as_str() {
        "SIM-NYR" => NYR_SEASON_SIMULATION.get_or_init(|| {
            parse_card_document(NYR_SEASON_SIMULATION_CARD_JSON)
                .expect("sealed NYR season simulation card")
        }),
        "SIM-SEA" => SEA_SEASON_SIMULATION.get_or_init(|| {
            parse_card_document(SEA_SEASON_SIMULATION_CARD_JSON)
                .expect("sealed SEA season simulation card")
        }),
        "REPLAY-NYR" => NYR_2024_REPLAY.get_or_init(|| {
            parse_card_document(NYR_2024_REPLAY_CARD_JSON).expect("sealed NYR 2024-25 replay card")
        }),
        "REPLAY-SEA" => SEA_2024_REPLAY.get_or_init(|| {
            parse_card_document(SEA_2024_REPLAY_CARD_JSON).expect("sealed SEA 2024-25 replay card")
        }),
        "MOVE-NYR" => NYR_2024_MOVEMENT.get_or_init(|| {
            parse_card_document(NYR_2024_MOVEMENT_CARD_JSON)
                .expect("sealed NYR 2024-25 movement card")
        }),
        "MOVE-SEA" => SEA_2024_MOVEMENT.get_or_init(|| {
            parse_card_document(SEA_2024_MOVEMENT_CARD_JSON)
                .expect("sealed SEA 2024-25 movement card")
        }),
        "HISTORY-NYR" => NYR_2024_HISTORY.get_or_init(|| {
            parse_card_document(NYR_2024_HISTORY_CARD_JSON)
                .expect("sealed NYR 2024-25 forecast history card")
        }),
        "HISTORY-SEA" => SEA_2024_HISTORY.get_or_init(|| {
            parse_card_document(SEA_2024_HISTORY_CARD_JSON)
                .expect("sealed SEA 2024-25 forecast history card")
        }),
        "MATCHUP-NYR" => NYR_VS_SEA_PLAYER_LINE_MATCHUP.get_or_init(|| {
            parse_card_document(NYR_VS_SEA_PLAYER_LINE_MATCHUP_CARD_JSON)
                .expect("sealed NYR vs SEA player-line matchup card")
        }),
        "SEA" => SEA.get_or_init(|| parse_card_document(SEA_CARD_JSON).expect("sealed SEA card")),
        "DEX" | "DEXTERS-DAWGS" | "FANTASY" => FANTASY.get_or_init(|| {
            parse_card_document(FANTASY_CARD_JSON).expect("sealed Dexter's Dawgs fantasy card")
        }),
        "DRAFT" | "DEX-DRAFT" => FANTASY_DRAFT.get_or_init(|| {
            parse_card_document(FANTASY_DRAFT_CARD_JSON)
                .expect("sealed Dexter's Dawgs fantasy draft card")
        }),
        "MORNING" | "DEX-MORNING" => FANTASY_MORNING.get_or_init(|| {
            parse_card_document(FANTASY_MORNING_CARD_JSON)
                .expect("sealed Dexter's Dawgs fantasy morning card")
        }),
        "TRADE" | "DEX-TRADE" => FANTASY_TRADE.get_or_init(|| {
            parse_card_document(FANTASY_TRADE_CARD_JSON)
                .expect("sealed Dexter's Dawgs fantasy trade card")
        }),
        _ => NYR.get_or_init(|| parse_card_document(NYR_CARD_JSON).expect("sealed NYR card")),
    }
}

fn prospect_arrival_cards() -> &'static BTreeMap<String, CardDocumentView> {
    static CARDS: OnceLock<BTreeMap<String, CardDocumentView>> = OnceLock::new();
    CARDS.get_or_init(|| {
        let arrival: ProspectArrivalLeagueCalibrationView =
            serde_json::from_str(PROSPECT_ARRIVAL_LEAGUE_JSON)
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

pub(crate) fn page_count(team: &str) -> usize {
    if team.eq_ignore_ascii_case("AUTHORITY-CLOSURE-BOARD") {
        4
    } else {
        2
    }
}

pub fn chrome(team: &str, page: usize, compare: bool) -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    if team.eq_ignore_ascii_case("WINDOW-BOARD") {
        return ScreenChrome {
            title: format!("The Window - 32-team board - page {}", page + 1),
            keybinds: vec![
                KeyHint::new("p", "teams 1-16/17-32"),
                KeyHint::new(":", "command"),
            ],
        };
    }
    if team.eq_ignore_ascii_case("ARRIVAL-BOARD") {
        return ScreenChrome {
            title: format!("Prospect Arrival Board - page {}", page + 1),
            keybinds: vec![
                KeyHint::new("p", "teams 1-16/17-32"),
                KeyHint::new(":", "command"),
            ],
        };
    }
    if team.eq_ignore_ascii_case("CENSUS-BOARD") {
        return ScreenChrome {
            title: format!("Prospect Census Readiness - page {}", page + 1),
            keybinds: vec![
                KeyHint::new("p", "teams 1-16/17-32"),
                KeyHint::new(":", "command"),
            ],
        };
    }
    if team.eq_ignore_ascii_case("AUTHORITY-PROGRESS-BOARD") {
        return ScreenChrome {
            title: format!("Prospect Authority Progress - page {}", page + 1),
            keybinds: vec![
                KeyHint::new("p", "teams 1-16/17-32"),
                KeyHint::new(":", "command"),
            ],
        };
    }
    if team.eq_ignore_ascii_case("AUTHORITY-CLOSURE-BOARD") {
        return ScreenChrome {
            title: format!("Prospect Authority Closure - page {} of 4", page + 1),
            keybinds: vec![
                KeyHint::new("p", "next 16 cells"),
                KeyHint::new(":", "command"),
            ],
        };
    }
    let fantasy = matches!(
        team.to_ascii_uppercase().as_str(),
        "DEX" | "DRAFT" | "MORNING" | "TRADE"
    );
    let mode = if fantasy {
        if team.eq_ignore_ascii_case("DRAFT") {
            "Dexter's Dawgs Draft"
        } else if team.eq_ignore_ascii_case("MORNING") {
            "Dexter's Dawgs Morning"
        } else if team.eq_ignore_ascii_case("TRADE") {
            "Dexter's Dawgs Trade"
        } else {
            "Dexter's Dawgs"
        }
    } else if compare {
        if team.to_ascii_uppercase().starts_with("REPLAY-") {
            "NYR vs SEA 2024-25 Replay"
        } else if team.to_ascii_uppercase().starts_with("MOVE-") {
            "NYR vs SEA Forecast Movement"
        } else if team.to_ascii_uppercase().starts_with("HISTORY-") {
            "NYR vs SEA Forecast History"
        } else if team.to_ascii_uppercase().starts_with("SIM-") {
            "NYR vs SEA Season Simulation"
        } else if team.to_ascii_uppercase().starts_with("WINDOW-") {
            "NYR vs SEA Organization Window"
        } else if team.to_ascii_uppercase().starts_with("ARRIVAL-") {
            "NYR vs SEA Prospect Arrivals"
        } else {
            "NYR vs SEA"
        }
    } else {
        team
    };
    let mut keybinds = vec![KeyHint::new("p", "page 1/2")];
    if !fantasy {
        keybinds.extend([KeyHint::new("t", "NYR/SEA"), KeyHint::new("c", "compare")]);
    }
    keybinds.push(KeyHint::new(":", "command"));
    let brand = if fantasy { "The Bench" } else { "IceCast" };
    ScreenChrome {
        title: format!("{brand} - {mode} - page {}", page + 1),
        keybinds,
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect, team: &str, compare: bool) {
    let page = app.selected.min(page_count(team) - 1);
    if team.eq_ignore_ascii_case("WINDOW-BOARD") {
        render_organization_window_board(f, area, page);
        return;
    }
    if team.eq_ignore_ascii_case("ARRIVAL-BOARD") {
        render_prospect_arrival_board(f, area, page);
        return;
    }
    if team.eq_ignore_ascii_case("CENSUS-BOARD") {
        render_prospect_census_readiness_board(f, area, page);
        return;
    }
    if team.eq_ignore_ascii_case("AUTHORITY-PROGRESS-BOARD") {
        render_prospect_authority_progress(f, area, page);
        return;
    }
    if team.eq_ignore_ascii_case("AUTHORITY-CLOSURE-BOARD") {
        render_prospect_authority_closure(f, area, page);
        return;
    }
    if compare
        && !matches!(
            team.to_ascii_uppercase().as_str(),
            "DEX" | "DRAFT" | "MORNING" | "TRADE"
        )
    {
        let upper = team.to_ascii_uppercase();
        let left = if upper.starts_with("REPLAY-") {
            "REPLAY-NYR"
        } else if upper.starts_with("MOVE-") {
            "MOVE-NYR"
        } else if upper.starts_with("HISTORY-") {
            "HISTORY-NYR"
        } else if upper.starts_with("SIM-") {
            "SIM-NYR"
        } else if upper.starts_with("WINDOW-") {
            "WINDOW-NYR"
        } else if upper.starts_with("ARRIVAL-") {
            "ARRIVAL-NYR"
        } else {
            "NYR"
        };
        let right = if upper.starts_with("REPLAY-") {
            "REPLAY-SEA"
        } else if upper.starts_with("MOVE-") {
            "MOVE-SEA"
        } else if upper.starts_with("HISTORY-") {
            "HISTORY-SEA"
        } else if upper.starts_with("SIM-") {
            "SIM-SEA"
        } else if upper.starts_with("WINDOW-") {
            "WINDOW-SEA"
        } else if upper.starts_with("ARRIVAL-") {
            "ARRIVAL-SEA"
        } else {
            "SEA"
        };
        let direction = if area.width >= 120 {
            Direction::Horizontal
        } else {
            Direction::Vertical
        };
        let panes = Layout::default()
            .direction(direction)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        render_document(f, panes[0], card(left), page);
        render_document(f, panes[1], card(right), page);
    } else {
        match published_matchup_card(team, None) {
            Ok(Some(document)) => render_document(f, area, &document, page),
            Ok(None) => render_document(f, area, card(team), page),
            Err(error) => render_card_error(f, area, &error),
        }
    }
}

fn published_matchup_card(
    token: &str,
    data_root: Option<&std::path::Path>,
) -> Result<Option<CardDocumentView>, String> {
    let upper = token.to_ascii_uppercase();
    let (season, game_id, team, explicit) = if upper == "MATCHUP-NYR" {
        (20262027, 2026020001, "NYR".to_owned(), false)
    } else if let Some(value) = upper.strip_prefix("MATCHUP:") {
        let parts = value.split(':').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err("published Matchup key is invalid".to_owned());
        }
        (
            parts[0]
                .parse::<u32>()
                .map_err(|_| "published Matchup season is invalid".to_owned())?,
            parts[1]
                .parse::<u64>()
                .map_err(|_| "published Matchup game ID is invalid".to_owned())?,
            parts[2].to_owned(),
            true,
        )
    } else {
        return Ok(None);
    };
    let data_root = data_root
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(CardPublicationStore::default_root);
    match CardPublicationStore::under_data_root(data_root)
        .load_player_line_matchup(season, game_id, &team)
    {
        Ok(card) => Ok(Some(card)),
        Err(
            CardPublicationStoreError::MissingCatalog
            | CardPublicationStoreError::MissingEntry { .. },
        ) if !explicit => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn render_card_error(f: &mut Frame, area: Rect, error: &str) {
    f.render_widget(
        Paragraph::new(format!("Matchup card unavailable\n\n{error}"))
            .block(Block::default().title("The Matchup").borders(Borders::ALL))
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn organization_window_board() -> &'static OrganizationWindowBoardView {
    static BOARD: OnceLock<OrganizationWindowBoardView> = OnceLock::new();
    BOARD.get_or_init(|| {
        let board = serde_json::from_str(ORGANIZATION_WINDOW_BOARD_JSON)
            .expect("sealed 32-team organization Window board");
        let inventory = load_organization_window_profile_inventory()
            .expect("embedded organization Window profile inventory");
        validate_organization_window_board(&board, &inventory)
            .expect("embedded Window board must remain canonical and sealed");
        board
    })
}

fn organization_window_cards() -> &'static BTreeMap<String, CardDocumentView> {
    static CARDS: OnceLock<BTreeMap<String, CardDocumentView>> = OnceLock::new();
    CARDS.get_or_init(|| {
        CANONICAL_TEAMS
            .iter()
            .map(|(team, _)| {
                let card = project_organization_window_card(
                    organization_window_board().clone(),
                    team,
                    None,
                    None,
                )
                .expect("canonical organization Window card");
                ((*team).to_owned(), card)
            })
            .collect()
    })
}

fn prospect_arrival_board() -> &'static ProspectArrivalBoardView {
    static BOARD: OnceLock<ProspectArrivalBoardView> = OnceLock::new();
    BOARD.get_or_init(|| {
        serde_json::from_str(PROSPECT_ARRIVAL_BOARD_JSON)
            .expect("sealed 32-team prospect arrival board")
    })
}

fn prospect_census_readiness_board() -> &'static ProspectCensusReadinessBoardView {
    static BOARD: OnceLock<ProspectCensusReadinessBoardView> = OnceLock::new();
    BOARD.get_or_init(|| {
        let board: ProspectCensusReadinessBoardView =
            serde_json::from_str(PROSPECT_CENSUS_READINESS_BOARD_JSON)
                .expect("sealed 32-team prospect census readiness board");
        assert_eq!(
            board
                .calculate_fingerprint()
                .expect("readiness fingerprint"),
            board.fingerprint,
            "sealed prospect census readiness board must remain canonical"
        );
        board
    })
}

fn prospect_authority_progress() -> &'static ProspectAuthorityProgressView {
    static PROGRESS: OnceLock<ProspectAuthorityProgressView> = OnceLock::new();
    PROGRESS.get_or_init(|| {
        let progress: ProspectAuthorityProgressView =
            serde_json::from_str(PROSPECT_AUTHORITY_PROGRESS_JSON)
                .expect("sealed prospect authority progress");
        assert_eq!(
            progress
                .calculate_fingerprint()
                .expect("authority progress fingerprint"),
            progress.fingerprint,
            "sealed prospect authority progress must remain canonical"
        );
        progress
    })
}

fn prospect_authority_closure_board() -> &'static ProspectAuthorityClosureBoardView {
    static BOARD: OnceLock<ProspectAuthorityClosureBoardView> = OnceLock::new();
    BOARD.get_or_init(|| {
        let board: ProspectAuthorityClosureBoardView =
            serde_json::from_str(PROSPECT_AUTHORITY_CLOSURE_BOARD_JSON)
                .expect("sealed prospect authority closure board");
        validate_prospect_authority_closure_board(&board)
            .expect("sealed prospect authority closure board must remain canonical");
        board
    })
}

fn render_prospect_authority_closure(f: &mut Frame, area: Rect, page: usize) {
    let board = prospect_authority_closure_board();
    let title = format!(
        " PROSPECT AUTHORITY CLOSURE | {} | {} | {}/4 ",
        board.evaluation_season,
        &board.fingerprint[..8],
        page + 1
    );
    let lines = prospect_authority_closure_lines(board, page, area.width.saturating_sub(2));
    let paragraph = Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

pub(crate) fn prospect_authority_closure_lines(
    board: &ProspectAuthorityClosureBoardView,
    page: usize,
    width: u16,
) -> Vec<String> {
    let start = page.min(3) * 16;
    let mut lines = vec![format!(
        "{} blockers · {} population · {} control · {}/{} teams affected",
        board.cells,
        board.population_blocking_cells,
        board.control_blocking_cells,
        board.affected_organizations,
        board.organizations,
    )];
    if width >= 72 {
        lines.extend([
            "CAMP · camp_participation_ledger.v1 · --camp-participation-ledger".to_owned(),
            "CONTRACT · contract_control_ledger.v1 · --contract-control-ledger".to_owned(),
        ]);
    } else {
        lines.extend([
            "CAMP · camp_participation_ledger.v1".to_owned(),
            "CONTRACT · contract_control_ledger.v1".to_owned(),
        ]);
    }
    lines.push("TEAM SOURCE FAMILY               GATE       STATE      ACTION".to_owned());
    for row in board.closure_cells.iter().skip(start).take(16) {
        lines.push(format!(
            "{:<4} {:<27} {:<10} {:<10} {}",
            row.organization,
            fit_authority_cell(&row.source_family, 27),
            authority_gate_label(row.gate),
            authority_gap_state_label(row.state),
            authority_disposition_label(row.disposition),
        ));
    }
    lines
}

fn render_prospect_authority_progress(f: &mut Frame, area: Rect, page: usize) {
    let progress = prospect_authority_progress();
    let title = format!(
        " PROSPECT AUTHORITY PROGRESS | {} | {} ",
        progress.evaluation_season,
        &progress.fingerprint[..8]
    );
    let lines = prospect_authority_progress_lines(progress, page, area.width.saturating_sub(2));
    let paragraph = Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

pub(crate) fn prospect_authority_progress_lines(
    progress: &ProspectAuthorityProgressView,
    page: usize,
    width: u16,
) -> Vec<String> {
    let start = page.min(1) * 16;
    let mut lines = vec![format!(
        "{}→{} blockers · {} closed · {} opened · {:.2}% retired",
        progress.prior_cells,
        progress.current_cells,
        progress.closed_cells,
        progress.opened_cells,
        f64::from(progress.closure_basis_points) / 100.0,
    )];
    if width >= 72 {
        lines.push(
            "TEAM SOURCE FAMILY            GATE       CHANGE        PRIOR      → CURRENT"
                .to_owned(),
        );
    } else {
        lines.push("TEAM SOURCE FAMILY          CHANGE        PRIOR      → CURRENT".to_owned());
    }
    for row in progress.changes.iter().skip(start).take(16) {
        let family = fit_authority_cell(&row.source_family, 22);
        let prior = row
            .prior_state
            .map(authority_gap_state_label)
            .unwrap_or("absent");
        let current = row
            .current_state
            .map(authority_gap_state_label)
            .unwrap_or("resolved");
        if width >= 72 {
            lines.push(format!(
                "{:<4} {:<22} {:<10} {:<13} {:<10} → {}",
                row.organization,
                family,
                authority_gate_label(row.gate),
                authority_change_label(row.kind),
                prior,
                current,
            ));
        } else {
            lines.push(format!(
                "{:<4} {:<22} {:<13} {:<10} → {}",
                row.organization,
                family,
                authority_change_label(row.kind),
                prior,
                current,
            ));
        }
    }
    lines
}

fn fit_authority_cell(value: &str, width: usize) -> String {
    value.chars().take(width).collect()
}

fn authority_change_label(kind: ProspectAuthorityProgressChangeKind) -> &'static str {
    match kind {
        ProspectAuthorityProgressChangeKind::Closed => "closed",
        ProspectAuthorityProgressChangeKind::Opened => "opened",
        ProspectAuthorityProgressChangeKind::StateChanged => "state changed",
    }
}

fn authority_disposition_label(disposition: ProspectAuthorityClosureDisposition) -> &'static str {
    match disposition {
        ProspectAuthorityClosureDisposition::Acquire => "acquire",
        ProspectAuthorityClosureDisposition::ResolveQuarantine => "resolve quarantine",
        ProspectAuthorityClosureDisposition::CompletePagination => "complete pagination",
    }
}

fn authority_gap_state_label(state: ProspectCensusAuthorityGapState) -> &'static str {
    match state {
        ProspectCensusAuthorityGapState::Failed => "failed",
        ProspectCensusAuthorityGapState::Quarantined => "quarantine",
        ProspectCensusAuthorityGapState::IncompletePagination => "pagination",
    }
}

fn authority_gate_label(gate: icelines_core::ProspectAuthorityClosureGate) -> &'static str {
    match gate {
        icelines_core::ProspectAuthorityClosureGate::PopulationAuthority => "population",
        icelines_core::ProspectAuthorityClosureGate::OrganizationalControl => "control",
    }
}

fn render_prospect_census_readiness_board(f: &mut Frame, area: Rect, page: usize) {
    let board = prospect_census_readiness_board();
    let title = format!(
        " PROSPECT CENSUS READINESS | {} | {} ",
        board.evaluation_season,
        &board.fingerprint[..8]
    );
    let lines = prospect_census_readiness_board_lines(board, page, area.width.saturating_sub(2));
    let paragraph = Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

pub(crate) fn prospect_census_readiness_board_lines(
    board: &ProspectCensusReadinessBoardView,
    page: usize,
    width: u16,
) -> Vec<String> {
    let start = page.min(1) * 16;
    let mut lines = vec![format!(
        "{}/32 population · {}/32 depth · {}/32 published · {} authority gaps",
        board.population_complete_organizations,
        board.depth_complete_organizations,
        board.published_organizations,
        board
            .authority_gap_summary
            .iter()
            .map(|row| row.organizations)
            .sum::<usize>()
    )];
    if width >= 72 {
        lines.push("TEAM AUTH       PUBLICATION           CAN CTRL RANK/TGT GAPS".to_owned());
    } else {
        lines.push("TEAM AUTH       CAN CTRL RANK/TGT GAPS".to_owned());
    }
    for row in board.teams.iter().skip(start).take(16) {
        if width >= 72 {
            lines.push(format!(
                "{:<4} {:<10?} {:<21?} {:>3} {:>4} {:>4}/{:<3} {:>4}",
                row.organization,
                row.population_authority_status,
                row.publication_status,
                row.counts.canonical_identity,
                row.counts.controlled_relationship,
                row.counts.ranked,
                row.requested_ranking_depth,
                row.authority_gaps.len(),
            ));
        } else {
            lines.push(format!(
                "{:<4} {:<10?} {:>3} {:>4} {:>4}/{:<3} {:>4}",
                row.organization,
                row.population_authority_status,
                row.counts.canonical_identity,
                row.counts.controlled_relationship,
                row.counts.ranked,
                row.requested_ranking_depth,
                row.authority_gaps.len(),
            ));
        }
    }
    lines
}

fn render_prospect_arrival_board(f: &mut Frame, area: Rect, page: usize) {
    let board = prospect_arrival_board();
    let title = format!(
        " PROSPECT ARRIVALS | {} | {:?} | {} ",
        board.forecast_season,
        board.rank_state,
        &board.fingerprint[..8]
    );
    let lines = prospect_arrival_board_lines(board, page, area.width.saturating_sub(2));
    let paragraph = Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

pub(crate) fn prospect_arrival_board_lines(
    board: &ProspectArrivalBoardView,
    page: usize,
    width: u16,
) -> Vec<String> {
    let rows = board.teams_in_display_order();
    let start = page.min(1) * 16;
    let mut lines = vec![format!(
        "32 teams · {}/{} eligible · {} rerouted · {} blocked · ranks {:?}",
        board.calibrated_skaters,
        board.eligible_skaters,
        board.routed_established_skaters,
        board.blocking_exclusions,
        board.rank_state
    )];
    if width >= 72 {
        lines.push("RK TEAM CAL/ELG COV  EXP ARR EXP ROLE TOP".to_owned());
    } else {
        lines.push("RK TEAM CAL/ELG COV  EXP ARR TOP".to_owned());
    }
    for row in rows.into_iter().skip(start).take(16) {
        let rank = row
            .rank
            .map(|rank| format!("{rank:>2}"))
            .unwrap_or_else(|| "NR".to_owned());
        let top = row
            .top_arrival_probability
            .map(|value| format!("{:>4.0}%", value * 100.0))
            .unwrap_or_else(|| "  NR".to_owned());
        if width >= 72 {
            lines.push(format!(
                "{rank:<2} {:<4} {:>2}/{:<2}    {:>3.0}%  {:>7.2} {:>8.2} {top}",
                row.organization,
                row.calibrated_skaters,
                row.eligible_skaters,
                row.calibration_coverage * 100.0,
                row.expected_arrivals,
                row.expected_established_roles,
            ));
        } else {
            lines.push(format!(
                "{rank:<2} {:<4} {:>2}/{:<2}    {:>3.0}%  {:>7.2} {top}",
                row.organization,
                row.calibrated_skaters,
                row.eligible_skaters,
                row.calibration_coverage * 100.0,
                row.expected_arrivals,
            ));
        }
    }
    lines
}

fn render_organization_window_board(f: &mut Frame, area: Rect, page: usize) {
    let board = organization_window_board();
    let title = format!(
        " THE WINDOW | {} | {} | {} ",
        board.as_of,
        board.manifest.manifest_id,
        &board.fingerprint[..8]
    );
    let lines = organization_window_board_lines(board, page, area.width.saturating_sub(2));
    let paragraph = Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

/// Pure compact projection of one half of the sealed 32-team board.
pub(crate) fn organization_window_board_lines(
    board: &OrganizationWindowBoardView,
    page: usize,
    width: u16,
) -> Vec<String> {
    let rows = board.organizations_in_display_order();
    let start = page.min(1) * 16;
    let mut lines = vec![format!(
        "32 teams · coverage {:.0}% · ranks may be withheld when evidence gates fail",
        board.league_coverage * 100.0
    )];
    if width >= 72 {
        lines.push("RK  TEAM  SCORE  CONF  COV   STATE        CLASS".to_owned());
    } else {
        lines.push("RK  TEAM  SCORE  CONF  COV   STATE".to_owned());
    }
    for row in rows.into_iter().skip(start).take(16) {
        let rank = row
            .overall
            .rank
            .map(|rank| format!("{rank:>2}"))
            .unwrap_or_else(|| "NR".to_owned());
        let score = row
            .overall
            .score
            .map(|score| format!("{score:>5.1}"))
            .unwrap_or_else(|| "   NR".to_owned());
        let state = format!("{:?}", row.overall.rank_status.state);
        let classification = row
            .overall
            .published_classification()
            .map(|classification| format!("{classification:?}"))
            .unwrap_or_else(|| "Under review".to_owned());
        if width >= 72 {
            lines.push(format!(
                "{rank:<2}  {:<4}  {score}  {:>3.0}%  {:>3.0}%  {:<11}  {classification}",
                row.organization,
                row.overall.confidence * 100.0,
                row.overall.coverage * 100.0,
                state
            ));
        } else {
            lines.push(format!(
                "{rank:<2}  {:<4}  {score}  {:>3.0}%  {:>3.0}%  {}",
                row.organization,
                row.overall.confidence * 100.0,
                row.overall.coverage * 100.0,
                state
            ));
        }
    }
    lines
}

fn render_document(f: &mut Frame, area: Rect, document: &CardDocumentView, page: usize) {
    let team = document
        .theme
        .team_abbreviation
        .as_deref()
        .unwrap_or(&document.theme.ascii_identity);
    let page_label = document.pages[page]
        .display_label
        .as_deref()
        .unwrap_or(&document.pages[page].literal_label);
    let title = format!(" {team} | {page_label} ");
    let lines = document_lines(document, page, area.width.saturating_sub(2));
    let theme_style = card_theme_style(document);
    let paragraph = Paragraph::new(lines.into_iter().map(Line::from).collect::<Vec<_>>())
        .block(
            Block::default()
                .title(title)
                .title_style(theme_style.add_modifier(Modifier::BOLD))
                .border_style(theme_style)
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn card_theme_style(document: &CardDocumentView) -> Style {
    document
        .theme
        .primary
        .as_deref()
        .and_then(parse_hex_color)
        .map_or_else(Style::default, |color| Style::default().fg(color))
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(red, green, blue))
}

/// Pure text projection used by the terminal renderer and density tests.
pub(crate) fn document_lines(document: &CardDocumentView, page: usize, width: u16) -> Vec<String> {
    let page = &document.pages[page.min(document.pages.len().saturating_sub(1))];
    let mut lines = vec![document.title.clone()];
    if let Some(subtitle) = &document.subtitle {
        lines.push(subtitle.clone());
    }
    lines.push(format!("Page {}: {}", page.order, page.accessible_summary));

    // Warnings are deliberately early so they remain visible in short,
    // narrow terminals instead of being clipped below methodology details.
    for warning in &document.warnings {
        lines.push(format!("WARNING: {}", warning.message));
    }

    for section in &page.sections {
        match section {
            CardSectionView::IdentityHeader(section) => {
                lines.push(section.title.clone());
                if let Some(subtitle) = &section.subtitle {
                    lines.push(subtitle.clone());
                }
            }
            CardSectionView::Lineup(section) => {
                lines.push(format!("-- {} --", section.title));
                for group in &section.groups {
                    lines.push(format!("{}:", group.label));
                    let cells = group
                        .slots
                        .iter()
                        .map(|slot| {
                            let name = slot.subject_label.as_deref().unwrap_or("Open");
                            let score = slot
                                .metrics
                                .first()
                                .map(|metric| metric.display_text.as_str())
                                .unwrap_or("NR");
                            format!("{} {} [{}]", slot.label, name, score)
                        })
                        .collect::<Vec<_>>();
                    if width >= 150 {
                        lines.push(format!("  {}", cells.join(" | ")));
                    } else {
                        lines.extend(cells.into_iter().map(|cell| format!("  {cell}")));
                    }
                }
            }
            CardSectionView::MetricStrip(section) => {
                lines.push(format!(
                    "-- {} --",
                    section.title.as_deref().unwrap_or("Metrics")
                ));
                for metric in &section.metrics {
                    lines.push(format!("{}: {}", metric.metric.label, metric.display_text));
                }
            }
            CardSectionView::ProbabilityRange(section) => {
                lines.push(format!("-- {} --", section.title));
                for range in &section.ranges {
                    lines.push(format!("{}: {}", range.label, range.display_text));
                }
            }
            CardSectionView::ScenarioBridge(section) => {
                lines.push(format!(
                    "-- {}: {} -> {} --",
                    section.title, section.from_label, section.to_label
                ));
                for metric in &section.metrics {
                    lines.push(format!("{}: {}", metric.metric.label, metric.display_text));
                }
            }
            CardSectionView::PlayerList(section) => {
                lines.push(format!("-- {} --", section.title));
                for row in &section.rows {
                    let metrics = row
                        .metrics
                        .iter()
                        .map(|metric| format!("{} {}", metric.metric.label, metric.display_text))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    let role = row
                        .role
                        .as_deref()
                        .map(|role| format!(" [{role}]"))
                        .unwrap_or_default();
                    lines.push(format!("{}{}: {}", row.name, role, metrics));
                }
            }
            CardSectionView::StateNotice(section) => {
                lines.push(format!("-- {} --", section.title));
                if let Some(detail) = &section.detail {
                    lines.push(detail.clone());
                }
                for warning in &section.warnings {
                    lines.push(format!("WARNING: {}", warning.message));
                }
            }
            CardSectionView::Methodology(section) => {
                lines.push(format!("-- {} --", section.title));
                for method in &section.methods {
                    lines.push(format!(
                        "{} {}: {}",
                        method.label, method.version, method.summary
                    ));
                }
                for limitation in &section.limitations {
                    lines.push(format!("LIMIT: {limitation}"));
                }
            }
            CardSectionView::Provenance(section) => {
                lines.push(format!("-- {} --", section.title));
                for provenance_id in &section.provenance_ids {
                    if let Some(provenance) = document
                        .provenance
                        .iter()
                        .find(|item| item.id == *provenance_id)
                    {
                        lines.push(format!(
                            "{}: {:?} · {:?}",
                            provenance.label, provenance.source, provenance.state
                        ));
                        if let Some(observed_at) = provenance.observed_at {
                            lines.push(format!("Observed: {}", observed_at.to_rfc3339()));
                        }
                        if let Some(fingerprint) = &provenance.fingerprint {
                            lines.push(format!("Fingerprint: {fingerprint}"));
                        }
                        if let Some(note) = &provenance.note {
                            lines.push(note.clone());
                        }
                    }
                }
            }
            CardSectionView::Decision(section) => {
                lines.push(format!("-- {} --", section.title));
                lines.push(section.recommendation.clone());
                lines.extend(section.rationale.iter().cloned());
                for alternative in &section.alternatives {
                    lines.push(format!("- {}", alternative.label));
                    if let Some(detail) = &alternative.detail {
                        lines.push(format!("  {detail}"));
                    }
                }
            }
            CardSectionView::Timeline(section) => {
                lines.push(format!("-- {} --", section.title));
                for item in &section.items {
                    lines.push(format!(
                        "{}: {}",
                        item.effective_at.date_naive(),
                        item.label
                    ));
                }
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::{
        app::{App, Screen},
        command::{execute_command, parse_command, Command},
        event::Action,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use ratatui::{backend::TestBackend, Terminal};
    use tower::ServiceExt;

    fn render_app(app: &App, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| crate::tui::screens::render(frame, app))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut output = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                output.push_str(buffer[(x, y)].symbol());
            }
            output.push('\n');
        }
        output
    }

    #[test]
    fn l0_one_sealed_fixture_drives_all_density_projections() {
        let document = card("NYR");
        let fingerprint = document.fingerprint.clone();
        for width in [80, 120, 160] {
            let page_one = document_lines(document, 0, width);
            let page_two = document_lines(document, 1, width);
            assert!(page_one.iter().any(|line| line.contains("Tye Kartye")));
            assert!(page_two
                .iter()
                .any(|line| line.contains("Projected points")));
            assert!(page_two.iter().any(|line| line.contains("WARNING:")));
            assert_eq!(document.fingerprint, fingerprint);
        }
    }

    #[test]
    fn l0_page_projection_preserves_order_values_and_nr() {
        let document = card("NYR");
        let lines = document_lines(document, 0, 80);
        let text = lines.join("\n");
        assert!(text.contains("Line 1:"));
        assert!(text.contains("Goalies:"));
        assert!(text.contains("[NR]"));
        assert!(text.find("Line 1:") < text.find("Goalies:"));
    }

    #[test]
    fn l0_long_names_and_nr_remain_explicit() {
        let mut document = card("NYR").clone();
        let CardSectionView::Lineup(lineup) = &mut document.pages[0].sections[1] else {
            panic!("fixture lineup section");
        };
        lineup.groups[0].slots[0].subject_label =
            Some("A-Very-Long-Hyphenated-Player-Name".to_string());
        lineup.groups[0].slots[0].metrics[0].display_text = "NR".to_string();
        let text = document_lines(&document, 0, 80).join("\n");
        assert!(text.contains("A-Very-Long-Hyphenated-Player-Name [NR]"));
    }

    #[test]
    fn l1_launch_grammar_and_page_keys_preserve_document() {
        assert_eq!(
            parse_command("team-card sea").unwrap(),
            Command::TeamCard {
                team: "SEA".to_string()
            }
        );
        assert!(parse_command("team-card BOS").is_err());

        let fingerprint = card("SEA").fingerprint.clone();
        let mut app = App::new(true);
        execute_command(parse_command("team-card SEA").unwrap(), &mut app);
        assert!(matches!(
            app.screen,
            Screen::TeamCard {
                ref team,
                compare: false
            } if team == "SEA"
        ));
        app.handle(Action::Char('p'));
        assert_eq!(app.selected, 1);
        app.handle(Action::Char('c'));
        assert!(matches!(app.screen, Screen::TeamCard { compare: true, .. }));
        assert_eq!(card("SEA").fingerprint, fingerprint);
    }

    #[test]
    fn l1_matchup_card_command_opens_the_shared_sealed_document() {
        assert_eq!(
            parse_command("matchup-card").unwrap(),
            Command::TeamCard {
                team: "MATCHUP-NYR".to_string()
            }
        );
        assert!(parse_command("matchup-card SEA").is_err());
        assert_eq!(
            parse_command("matchup-card 20262027 2026020001 NYR").unwrap(),
            Command::TeamCard {
                team: "MATCHUP:20262027:2026020001:NYR".to_owned()
            }
        );

        let document = card("MATCHUP-NYR");
        assert_eq!(
            document.card_kind,
            icelines_core::CardKind::PlayerLineMatchup
        );
        let matchup = document_lines(document, 0, 100).join("\n");
        let insider = document_lines(document, 1, 100).join("\n");
        assert!(matchup.contains("SEA at NYR"));
        assert!(matchup.contains("Game probability"));
        assert!(matchup.contains("NYR Player 2"));
        assert!(insider.contains("NYR player profile evidence"));
    }

    #[test]
    fn l1_matchup_card_loads_the_published_catalog_without_recomputation() {
        let temp = tempfile::tempdir().unwrap();
        let mut published = card("MATCHUP-NYR").clone();
        published.title = "Published Matchup".to_owned();
        published = published.seal().unwrap();
        CardPublicationStore::under_data_root(temp.path())
            .publish(
                &published,
                Utc.with_ymd_and_hms(2026, 10, 10, 18, 0, 0).unwrap(),
            )
            .unwrap();

        let loaded = published_matchup_card("MATCHUP:20262027:2026020001:NYR", Some(temp.path()))
            .unwrap()
            .unwrap();

        assert_eq!(loaded, published);
    }

    #[tokio::test]
    async fn l2_matchup_web_and_tui_use_the_identical_document() {
        let expected = card("MATCHUP-NYR").clone();
        let app = icelines_web::router(icelines_web::WebState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/cards/player-line-matchup/20262027/2026020001/NYR")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["etag"].to_str().unwrap(),
            format!("\"{}\"", expected.fingerprint)
        );
        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let web_card: CardDocumentView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_card, expected);

        let matchup = document_lines(&expected, 0, 100).join("\n");
        let insider = document_lines(&expected, 1, 100).join("\n");
        assert!(matchup.contains("Game probability"));
        assert!(matchup.contains("NYR dressed units"));
        assert!(insider.contains("Sealed matchup source"));
    }

    #[test]
    fn l1_80_120_160_terminal_snapshots_preserve_meaning() {
        for width in [80, 120, 160] {
            let mut app = App::new(true);
            app.screen = Screen::TeamCard {
                team: "NYR".to_string(),
                compare: false,
            };
            let depth = render_app(&app, width, 60);
            assert!(depth.contains("Tye Kartye"), "width {width}: {depth}");
            assert!(depth.contains("NR"), "width {width}: {depth}");

            app.selected = 1;
            let insider = render_app(&app, width, 60);
            assert!(insider.contains("WARNING:"), "width {width}: {insider}");
            assert!(insider.contains("breakout"), "width {width}: {insider}");
        }
    }

    #[test]
    fn l1_comparison_stacks_narrow_and_splits_wide() {
        for width in [80, 120, 160] {
            let mut app = App::new(true);
            app.screen = Screen::TeamCard {
                team: "NYR".to_string(),
                compare: true,
            };
            let text = render_app(&app, width, 80);
            assert!(text.contains("NYR | The Depth Chart"));
            assert!(text.contains("SEA | The Depth Chart"));
        }
    }

    #[test]
    fn l1_fantasy_fixture_projects_roster_rules_and_schedule_classes() {
        let document = card("DEX");
        let roster = document_lines(document, 0, 80).join("\n");
        assert!(roster.contains("Nathan MacKinnon"));
        assert!(roster.contains("BN4"));
        assert!(roster.contains("IR+2"));

        let insider = document_lines(document, 1, 80).join("\n");
        assert!(insider.contains("Free-agent activation: Same day"));
        assert!(insider.contains("Best calendar complement: WSH (Class 8)"));
        assert!(insider.contains("Class 1: BOS, COL, DET, NYR"));
        assert!(insider.contains("Historical roster names"));

        assert_eq!(
            parse_command("team-card dex").unwrap(),
            Command::TeamCard {
                team: "DEX".to_string()
            }
        );

        let mut app = App::new(true);
        execute_command(parse_command("team-card DEX").unwrap(), &mut app);
        app.handle(Action::Char('c'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard {
                ref team,
                compare: false
            } if team == "DEX"
        ));
        app.handle(Action::Char('t'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard { ref team, .. } if team == "DEX"
        ));
    }

    #[test]
    fn l1_fantasy_draft_fixture_projects_pick_fallback_and_components() {
        let document = card("DRAFT");
        let board = document_lines(document, 0, 80).join("\n");
        assert!(board.contains("Draft Jason Robertson"));
        assert!(board.contains("Fallback: William Nylander"));
        assert!(board.contains("LW/RW"));
        assert!(board.contains("Priority slots: LW1, RW1, D1, G1"));

        let insider = document_lines(document, 1, 80).join("\n");
        assert!(insider.contains("Schedule diversity"));
        assert!(insider.contains("Taken matched: 2"));
        assert!(insider.contains("not current claims"));

        assert_eq!(
            parse_command("draft-card").unwrap(),
            Command::TeamCard {
                team: "DRAFT".to_string()
            }
        );
    }

    #[test]
    fn l1_fantasy_morning_fixture_projects_actions_goalies_pickups_and_timeline() {
        let document = card("MORNING");
        let morning = document_lines(document, 0, 80).join("\n");
        assert!(morning.contains("Move Justin Brazeau to IR+1"));
        assert!(morning.contains("Nathan MacKinnon"));

        let insider = document_lines(document, 1, 80).join("\n");
        assert!(insider.contains("Darren Raddysh"));
        assert!(insider.contains("Goalie start evidence"));
        assert!(insider.contains("Today's goalie checkpoints"));
        assert!(insider.contains("Final goalie safety check"));
        assert!(insider.contains("Safe proactive adds: 0"));

        assert_eq!(
            parse_command("morning-card").unwrap(),
            Command::TeamCard {
                team: "MORNING".to_string()
            }
        );
    }

    #[test]
    fn l1_fantasy_trade_fixture_projects_packages_fairness_and_team_impacts() {
        let document = card("TRADE");
        let board = document_lines(document, 0, 80).join("\n");
        assert!(board.contains("Adam Fox"));
        assert!(board.contains("Mikko Rantanen"));
        assert!(board.contains("Reasonable offer range"));
        assert!(board.contains("Value gap percent"));

        let insider = document_lines(document, 1, 80).join("\n");
        assert!(insider.contains("Before and after"));
        assert!(insider.contains("Roster 16/16 · Legal"));
        assert!(insider.contains("Open slots after"));
        assert!(insider.contains("not current trade advice"));

        assert_eq!(
            parse_command("trade-card").unwrap(),
            Command::TeamCard {
                team: "TRADE".to_string()
            }
        );
    }

    #[test]
    fn l1_season_simulation_fixture_projects_scoreboard_and_insider() {
        let nyr = card("SIM-NYR");
        let sea = card("SIM-SEA");
        assert_eq!(
            nyr.context.simulation.parameter_fingerprint,
            sea.context.simulation.parameter_fingerprint
        );
        let scoreboard = document_lines(nyr, 0, 100).join("\n");
        assert!(scoreboard.contains("Stanley Cup"));
        assert!(scoreboard.contains("Points distribution"));
        assert!(scoreboard.contains("Scenario delta from baseline"));

        let insider = document_lines(nyr, 1, 100).join("\n");
        assert!(insider.contains("Schedule pressure"));
        assert!(insider.contains("Pivotal games"));
        assert!(insider.contains("How to read this forecast"));

        assert_eq!(
            parse_command("season-card SEA").unwrap(),
            Command::TeamCard {
                team: "SIM-SEA".to_string()
            }
        );
    }

    #[test]
    fn l1_completed_replay_projects_actuals_and_calibration() {
        let replay = card("REPLAY-NYR");
        let insider = document_lines(replay, 1, 100).join("\n");
        assert!(insider.contains("Actual team result"));
        assert!(insider.contains("Wins: 39"));
        assert!(insider.contains("Points: 85"));
        assert!(insider.contains("Completed-season calibration"));
        assert!(insider.contains("League picks correct: 56.6%"));
        assert!(insider.contains("Calibration intercept · ideal 0: -0.378"));
        assert!(insider.contains("Calibration slope · ideal 1: 4.417"));
        assert!(insider.contains("Calibration slope 95% lower: 2.380"));
        assert!(insider.contains("Calibration slope 95% upper: 6.454"));
        assert!(insider.contains("Best chronological Elo blend"));
        assert_eq!(
            parse_command("replay-card SEA").unwrap(),
            Command::TeamCard {
                team: "REPLAY-SEA".to_string()
            }
        );
    }

    #[test]
    fn l1_forecast_movement_projects_shift_and_preserves_card_family_toggle() {
        let nyr = card("MOVE-NYR");
        let sea = card("MOVE-SEA");
        assert_eq!(
            nyr.context.simulation.parameter_fingerprint,
            sea.context.simulation.parameter_fingerprint
        );
        let shift = document_lines(nyr, 0, 100).join("\n");
        assert!(shift.contains("What changed: 2025-01-31 -> 2025-02-28"));
        assert!(shift.contains("Projected points: +0.06"));
        assert!(shift.contains("Observed standings points: +10.00"));
        let insider = document_lines(nyr, 1, 100).join("\n");
        assert!(insider.contains("Sealed checkpoint delta"));
        assert!(insider.contains("Earlier sealed IceCast league run"));
        assert!(insider.contains("Later sealed IceCast league run"));
        assert!(
            insider.contains("11cdcfcf9c10338a0384454803ca0aa67abdcb1ddce371c7f0b71c7faf320fef")
        );
        assert!(
            insider.contains("5f14d35e417c6ca1897e9ebccfbe773fb92452b1b23282b9cc868460db3ac20a")
        );
        assert_eq!(
            parse_command("movement-card SEA").unwrap(),
            Command::TeamCard {
                team: "MOVE-SEA".to_string()
            }
        );

        let mut app = App::new(true);
        execute_command(parse_command("movement-card SEA").unwrap(), &mut app);
        app.handle(Action::Char('t'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard { ref team, .. } if team == "MOVE-NYR"
        ));

        execute_command(parse_command("replay-card SEA").unwrap(), &mut app);
        app.handle(Action::Char('t'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard { ref team, .. } if team == "REPLAY-NYR"
        ));
    }

    #[test]
    fn l1_forecast_history_projects_tape_and_preserves_card_family_toggle() {
        let nyr = card("HISTORY-NYR");
        let sea = card("HISTORY-SEA");
        assert_eq!(
            nyr.context.simulation.parameter_fingerprint,
            sea.context.simulation.parameter_fingerprint
        );
        assert_eq!(nyr.provenance, sea.provenance);

        let tape = document_lines(nyr, 0, 120).join("\n");
        assert!(tape.contains("Through 2025-01-31"));
        assert!(tape.contains("Through 2025-02-28"));
        assert!(tape.contains("Through 2025-03-31"));
        assert!(tape.contains("Projected points"));
        assert!(tape.contains("Change in projected points"));
        assert!(tape.contains("Net change in projected points"));
        assert!(tape.contains("League movement rank: 19 of 32"));
        assert!(tape.contains("Trajectory: mixed"));
        assert!(tape.contains("Largest checkpoint swing"));
        assert!(tape.contains("Projected points P10 / P50 / P90"));
        assert!(tape.contains("Movement materiality: moderate"));
        assert!(tape.contains("Confirmed standings points gained: +25.00"));
        assert!(tape.contains("Change in expected remaining points: -26.75"));
        assert!(tape.contains("Movement bridge"));
        assert!(tape.contains("Prior expected points for 24 newly completed games: +26.59"));
        assert!(tape.contains("Realized points versus prior expected pace: -1.59"));
        assert!(tape.contains("Still-unplayed outlook revaluation: -0.16"));
        assert!(tape.contains("Pace-normalized attribution"));
        assert!(tape.contains("Realized points versus prior checkpoint pace: +0.03"));
        assert!(tape.contains("Still-unplayed outlook revaluation from prior checkpoint: -0.17"));
        let insider = document_lines(nyr, 1, 120).join("\n");
        assert!(insider.contains("Chronological checkpoint history"));

        assert_eq!(
            parse_command("history-card SEA").unwrap(),
            Command::TeamCard {
                team: "HISTORY-SEA".to_string()
            }
        );
        let mut app = App::new(true);
        execute_command(parse_command("history-card SEA").unwrap(), &mut app);
        app.handle(Action::Char('t'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard { ref team, .. } if team == "HISTORY-NYR"
        ));
    }

    #[test]
    fn l1_prospect_arrival_projects_depth_chart_insider_and_card_controls() {
        let nyr = card("ARRIVAL-NYR");
        let sea = card("ARRIVAL-SEA");
        assert_eq!(nyr.card_kind, icelines_core::CardKind::ProspectArrival);
        assert_eq!(sea.card_kind, icelines_core::CardKind::ProspectArrival);
        assert_eq!(
            nyr.context.simulation.parameter_fingerprint,
            sea.context.simulation.parameter_fingerprint
        );
        assert_eq!(nyr.provenance[0].fingerprint, sea.provenance[0].fingerprint);
        assert_ne!(nyr.fingerprint, sea.fingerprint);
        assert_eq!(prospect_arrival_cards().len(), CANONICAL_TEAMS.len());
        assert_eq!(
            *nyr,
            parse_card_document(NYR_PROSPECT_ARRIVAL_CARD_JSON)
                .expect("sealed NYR prospect arrival fixture")
        );
        assert_eq!(
            *sea,
            parse_card_document(SEA_PROSPECT_ARRIVAL_CARD_JSON)
                .expect("sealed SEA prospect arrival fixture")
        );
        for (team, team_name) in CANONICAL_TEAMS {
            let card = card(&format!("ARRIVAL-{team}"));
            assert_eq!(card.context.joins.team_ids, [*team]);
            assert_eq!(card.title, format!("{team_name} prospect arrivals"));
        }

        for width in [80, 120, 160] {
            let depth_chart = document_lines(nyr, 0, width).join("\n");
            assert!(depth_chart.contains("Cole Beaudoin"));
            assert!(depth_chart.contains("Calibrated arrival outlook"));
            assert!(depth_chart.contains("Arrival"));

            let insider = document_lines(nyr, 1, width).join("\n");
            assert!(insider.contains("Coverage draft"));
            assert!(insider.contains("Exclusion ledger"));
            assert!(insider.contains("Source authority"));
            assert!(insider.contains("No sealed prospect population source package"));
        }

        assert_eq!(
            parse_command("prospect-arrival-card SEA").unwrap(),
            Command::TeamCard {
                team: "ARRIVAL-SEA".to_owned()
            }
        );
        assert_eq!(
            parse_command("prospect-arrival-card BOS").unwrap(),
            Command::TeamCard {
                team: "ARRIVAL-BOS".to_owned()
            }
        );
        assert!(parse_command("prospect-arrival-card XYZ").is_err());

        let mut app = App::new(true);
        execute_command(
            parse_command("prospect-arrival-card SEA").unwrap(),
            &mut app,
        );
        app.handle(Action::Char('p'));
        assert_eq!(app.selected, 1);
        app.handle(Action::Char('t'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard { ref team, .. } if team == "ARRIVAL-NYR"
        ));
        app.handle(Action::Char('c'));
        assert!(matches!(app.screen, Screen::TeamCard { compare: true, .. }));
        assert!(chrome("ARRIVAL-NYR", 0, true)
            .title
            .contains("NYR vs SEA Prospect Arrivals"));
    }

    #[test]
    fn l1_organization_window_projects_shared_values_and_toggles() {
        let nyr = card("WINDOW-NYR");
        let sea = card("WINDOW-SEA");
        assert_eq!(nyr.card_kind, icelines_core::CardKind::OrganizationWindow);
        assert_eq!(sea.card_kind, icelines_core::CardKind::OrganizationWindow);
        assert_eq!(nyr.provenance[0].fingerprint, sea.provenance[0].fingerprint);
        let window = document_lines(nyr, 0, 80).join("\n");
        assert!(window.contains("Organization Window"));
        assert!(window.contains("League rank"));
        assert!(window.contains("Confidence"));
        assert!(window.contains("Coverage"));
        let insider = document_lines(nyr, 1, 80).join("\n");
        assert!(insider.contains("Leading available panes"));
        assert!(insider.contains("Lowest available panes"));
        assert!(insider.contains("Profiles and evidence"));
        assert!(insider.contains("Sealed board"));
        assert!(insider.contains("Balanced organization Window · 2026-07-28"));
        assert!(
            insider.contains("30083b67d6ac6a8bd185cc8a21f074c6cc00301e415cd60b954dcf68ff79a920")
        );
        assert!(nyr
            .subtitle
            .as_deref()
            .unwrap()
            .starts_with("Under review · NR"));

        for (team, _) in CANONICAL_TEAMS {
            let card = card(&format!("WINDOW-{team}"));
            assert_eq!(card.context.joins.team_ids, [*team]);
            assert!(
                card_theme_style(card).fg.is_some(),
                "{team} card must project its core theme into the TUI"
            );
        }

        assert_eq!(
            card_theme_style(card("WINDOW-BOS")).fg,
            Some(Color::Rgb(255, 184, 28))
        );
        assert_eq!(parse_hex_color("not-a-color"), None);

        assert_eq!(
            parse_command("window-card SEA").unwrap(),
            Command::TeamCard {
                team: "WINDOW-SEA".to_owned()
            }
        );
        assert_eq!(
            parse_command("window-card BOS").unwrap(),
            Command::TeamCard {
                team: "WINDOW-BOS".to_owned()
            }
        );
        assert!(parse_command("window-card XYZ").is_err());
        let mut app = App::new(true);
        execute_command(parse_command("window-card SEA").unwrap(), &mut app);
        app.handle(Action::Char('t'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard { ref team, .. } if team == "WINDOW-NYR"
        ));
    }

    #[test]
    fn l1_organization_window_board_pages_cover_all_32_teams_at_80_columns() {
        let board = organization_window_board();
        let first = organization_window_board_lines(board, 0, 80);
        let second = organization_window_board_lines(board, 1, 80);
        assert_eq!(first.len(), 18);
        assert_eq!(second.len(), 18);
        assert!(first.iter().chain(&second).all(|line| line.len() <= 80));
        assert_eq!(first[2].split_whitespace().nth(1), Some("ANA"));
        assert_eq!(second[2].split_whitespace().nth(1), Some("NSH"));
        let teams = first
            .iter()
            .skip(2)
            .chain(second.iter().skip(2))
            .filter_map(|line| line.split_whitespace().nth(1))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(teams.len(), 32);
        assert_eq!(
            parse_command("window-board").unwrap(),
            Command::TeamCard {
                team: "WINDOW-BOARD".to_owned()
            }
        );
        let mut app = App::new(true);
        execute_command(parse_command("window-board").unwrap(), &mut app);
        app.handle(Action::Char('t'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard { ref team, .. } if team == "WINDOW-BOARD"
        ));
        app.handle(Action::Char('p'));
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn l1_prospect_arrival_board_pages_cover_all_32_without_shadow_ranks() {
        let board = prospect_arrival_board();
        let first = prospect_arrival_board_lines(board, 0, 80);
        let second = prospect_arrival_board_lines(board, 1, 80);
        assert_eq!(first.len(), 18);
        assert_eq!(second.len(), 18);
        assert!(first.iter().chain(&second).all(|line| line.len() <= 80));
        assert!(first[0].contains("8/10 eligible"));
        assert!(first[0].contains("157 rerouted"));
        assert!(first[0].contains("2 blocked"));
        assert!(first[0].contains("ranks Withheld"));
        assert!(first.iter().skip(2).all(|line| line.starts_with("NR")));
        assert!(second.iter().skip(2).all(|line| line.starts_with("NR")));
        let teams = first
            .iter()
            .skip(2)
            .chain(second.iter().skip(2))
            .filter_map(|line| line.split_whitespace().nth(1))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(teams.len(), CANONICAL_TEAMS.len());
        assert!(teams.contains("NYR"));
        assert!(teams.contains("SEA"));
        assert_eq!(
            parse_command("prospect-arrival-board").unwrap(),
            Command::TeamCard {
                team: "ARRIVAL-BOARD".to_owned()
            }
        );

        let mut app = App::new(true);
        execute_command(parse_command("prospect-arrival-board").unwrap(), &mut app);
        app.handle(Action::Char('t'));
        app.handle(Action::Char('c'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard {
                ref team,
                compare: false
            } if team == "ARRIVAL-BOARD"
        ));
        app.handle(Action::Char('p'));
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn l1_prospect_census_readiness_pages_cover_all_32_without_shadow_rankings() {
        let board = prospect_census_readiness_board();
        let first = prospect_census_readiness_board_lines(board, 0, 80);
        let second = prospect_census_readiness_board_lines(board, 1, 80);
        assert_eq!(first.len(), 18);
        assert_eq!(second.len(), 18);
        assert!(first.iter().chain(&second).all(|line| line.len() <= 80));
        assert!(first[0].contains("0/32 population"));
        assert!(first[0].contains("0/32 depth"));
        assert!(first[0].contains("0/32 published"));
        assert!(first[0].contains("64 authority gaps"));
        let teams = first
            .iter()
            .skip(2)
            .chain(second.iter().skip(2))
            .filter_map(|line| line.split_whitespace().next())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(teams.len(), CANONICAL_TEAMS.len());
        assert!(teams.contains("NYR"));
        assert!(teams.contains("SEA"));
        assert_eq!(
            parse_command("prospect-census-readiness").unwrap(),
            Command::TeamCard {
                team: "CENSUS-BOARD".to_owned()
            }
        );

        let mut app = App::new(true);
        execute_command(
            parse_command("prospect-census-readiness").unwrap(),
            &mut app,
        );
        app.handle(Action::Char('t'));
        app.handle(Action::Char('c'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard {
                ref team,
                compare: false
            } if team == "CENSUS-BOARD"
        ));
        app.handle(Action::Char('p'));
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn l1_prospect_authority_progress_pages_cover_every_exact_change() {
        let progress = prospect_authority_progress();
        let first = prospect_authority_progress_lines(progress, 0, 80);
        let second = prospect_authority_progress_lines(progress, 1, 80);
        assert_eq!(first.len(), 18);
        assert_eq!(second.len(), 18);
        assert!(first.iter().chain(&second).all(|line| line.len() <= 80));
        assert!(first[0].contains("96→64 blockers"));
        assert!(first[0].contains("32 closed"));
        assert!(first[0].contains("0 opened"));
        assert!(first[0].contains("33.33% retired"));
        let teams = first
            .iter()
            .skip(2)
            .chain(second.iter().skip(2))
            .filter_map(|line| line.split_whitespace().next())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(teams.len(), CANONICAL_TEAMS.len());
        assert!(teams.contains("NYR"));
        assert!(teams.contains("SEA"));
        assert!(first
            .iter()
            .chain(&second)
            .all(|line| !line.contains("Some(") && !line.contains("None")));
        assert_eq!(
            parse_command("prospect-authority-progress").unwrap(),
            Command::TeamCard {
                team: "AUTHORITY-PROGRESS-BOARD".to_owned()
            }
        );

        let mut app = App::new(true);
        execute_command(
            parse_command("prospect-authority-progress").unwrap(),
            &mut app,
        );
        app.handle(Action::Char('t'));
        app.handle(Action::Char('c'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard {
                ref team,
                compare: false
            } if team == "AUTHORITY-PROGRESS-BOARD"
        ));
        let rendered = render_app(&app, 80, 30);
        assert!(rendered.contains("PROSPECT AUTHORITY PROGRESS"));
        assert!(rendered.contains("96→64 blockers"));
        assert!(rendered.contains("ANA"));
        assert!(!rendered.contains("SJS"));
        app.handle(Action::Char('p'));
        assert_eq!(app.selected, 1);
        let rendered = render_app(&app, 80, 30);
        assert!(rendered.contains("SJS"));
        assert!(!rendered.contains("ANA  "));
    }

    #[test]
    fn l1_prospect_authority_closure_pages_preserve_all_64_exact_cells() {
        let board = prospect_authority_closure_board();
        let pages = (0..4)
            .map(|page| prospect_authority_closure_lines(board, page, 80))
            .collect::<Vec<_>>();
        assert!(pages.iter().all(|page| page.len() == 20));
        assert!(pages
            .iter()
            .flat_map(|page| page.iter())
            .all(|line| line.len() <= 80));
        assert!(pages[0][0].contains("64 blockers"));
        assert!(pages[0][0].contains("32 population"));
        assert!(pages[0][0].contains("32 control"));
        assert!(pages[0][1].contains("camp_participation_ledger.v1"));
        assert!(pages[0][1].contains("--camp-participation-ledger"));
        assert!(pages[0][2].contains("contract_control_ledger.v1"));
        assert!(pages[0][2].contains("--contract-control-ledger"));
        let keys = pages
            .iter()
            .flat_map(|page| page.iter().skip(4))
            .filter_map(|line| {
                let mut cells = line.split_whitespace();
                Some(format!("{}:{}", cells.next()?, cells.next()?))
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), board.closure_cells.len());
        assert!(keys.contains("NYR:nhl_club_camp_publication"));
        assert!(keys.contains("NYR:nhl_contract_publication"));
        assert!(keys.contains("SEA:nhl_club_camp_publication"));
        assert!(keys.contains("SEA:nhl_contract_publication"));
        assert_eq!(page_count("AUTHORITY-CLOSURE-BOARD"), 4);
        assert_eq!(page_count("NYR"), 2);
        assert_eq!(
            parse_command("prospect-authority-closure").unwrap(),
            Command::TeamCard {
                team: "AUTHORITY-CLOSURE-BOARD".to_owned()
            }
        );

        let mut app = App::new(true);
        execute_command(
            parse_command("prospect-authority-closure").unwrap(),
            &mut app,
        );
        app.handle(Action::Char('t'));
        app.handle(Action::Char('c'));
        assert!(matches!(
            app.screen,
            Screen::TeamCard {
                ref team,
                compare: false
            } if team == "AUTHORITY-CLOSURE-BOARD"
        ));
        let first = render_app(&app, 80, 30);
        assert!(first.contains("PROSPECT AUTHORITY CLOSURE"));
        assert!(first.contains("ANA"));
        assert!(!first.contains("WSH"));
        for expected_page in 1..=3 {
            app.handle(Action::Char('p'));
            assert_eq!(app.selected, expected_page);
        }
        let fourth = render_app(&app, 80, 30);
        assert!(fourth.contains("WSH"));
        assert!(!fourth.contains("ANA  "));
        app.handle(Action::Char('p'));
        assert_eq!(app.selected, 0);
    }

    #[tokio::test]
    async fn l2_organization_window_golden_parity_across_cli_tui_and_web() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let board_path =
            manifest_dir.join("../examples/organization-window-board-partial-2026-07-28.json");
        let expected_board = organization_window_board().clone();
        let expected_card = card("WINDOW-NYR").clone();
        let temp = tempfile::tempdir().unwrap();

        let cli_board_path = temp.path().join("window.json");
        crate::commands::icecast::run_window(
            board_path.clone(),
            None,
            true,
            false,
            Some(cli_board_path.clone()),
        )
        .unwrap();
        let cli_board: OrganizationWindowBoardView =
            serde_json::from_slice(&std::fs::read(cli_board_path).unwrap()).unwrap();
        assert_eq!(cli_board, expected_board);

        let cli_card_path = temp.path().join("window-card.json");
        crate::commands::icecast::run_window_card(
            board_path,
            "NYR".to_owned(),
            Some("New York Rangers".to_owned()),
            None,
            Some(cli_card_path.clone()),
        )
        .unwrap();
        let cli_card: CardDocumentView =
            serde_json::from_slice(&std::fs::read(cli_card_path).unwrap()).unwrap();
        assert_eq!(cli_card.document_id, expected_card.document_id);
        assert_eq!(cli_card.fingerprint, expected_card.fingerprint);

        let app = icelines_web::router(icelines_web::WebState::new());
        let board_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/window/balanced.v1/20262027")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(board_response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(board_response.into_body(), 8 * 1024 * 1024)
            .await
            .unwrap();
        let web_board: OrganizationWindowBoardView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_board, expected_board);

        let card_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/cards/organization-window/20262027/NYR")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(card_response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(card_response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let web_card: CardDocumentView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_card.document_id, expected_card.document_id);
        assert_eq!(web_card.fingerprint, expected_card.fingerprint);

        let html_response = app
            .oneshot(
                Request::builder()
                    .uri("/window/balanced.v1/20262027?team=NYR")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(html_response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(html_response.into_body(), 2 * 1024 * 1024)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();

        let tui_card = document_lines(&expected_card, 0, 80).join("\n");
        for expected in [
            "Score: 59.7",
            "League rank: NR",
            "Confidence: 64%",
            "Coverage: 85%",
        ] {
            assert!(tui_card.contains(expected), "TUI missing {expected}");
            let value = expected.split_once(": ").unwrap().1;
            assert!(html.contains(value), "Web HTML missing {value}");
        }
        let first_page = organization_window_board_lines(&expected_board, 0, 80);
        let second_page = organization_window_board_lines(&expected_board, 1, 80);
        let nyr_row = first_page
            .iter()
            .chain(&second_page)
            .find(|line| line.split_whitespace().any(|cell| cell == "NYR"))
            .unwrap();
        let cells = nyr_row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(&cells[..5], &["NR", "NYR", "59.7", "64%", "85%"]);
    }

    #[tokio::test]
    async fn l2_prospect_arrival_golden_parity_across_cli_tui_and_web() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let input = manifest_dir.join("../examples/icecast-prospect-arrival-league-2026-27.json");
        let expected = card("ARRIVAL-NYR").clone();
        let temp = tempfile::tempdir().unwrap();
        let cli_path = temp.path().join("prospect-arrival-card.json");

        crate::commands::icecast::run_prospect_arrival_card(
            input,
            "NYR".to_owned(),
            Some("New York Rangers".to_owned()),
            Some("2026-09-15T12:00:00Z".to_owned()),
            Some(cli_path.clone()),
        )
        .unwrap();
        let cli_card: CardDocumentView =
            serde_json::from_slice(&std::fs::read(cli_path).unwrap()).unwrap();
        assert_eq!(cli_card, expected);

        let app = icelines_web::router(icelines_web::WebState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/cards/prospect-arrival/20262027/NYR")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let web_card: CardDocumentView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_card, expected);

        let depth_chart = document_lines(&expected, 0, 80).join("\n");
        let insider = document_lines(&expected, 1, 80).join("\n");
        assert!(depth_chart.contains("Cole Beaudoin"));
        assert!(depth_chart.contains("Arrival 29.0%"));
        assert!(insider.contains("Coverage draft"));
        assert!(insider.contains("Exclusion ledger"));
    }

    #[tokio::test]
    async fn l2_prospect_arrival_board_golden_parity_across_cli_tui_and_web() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let input = manifest_dir.join("../examples/icecast-prospect-arrival-league-2026-27.json");
        let expected = prospect_arrival_board().clone();
        let temp = tempfile::tempdir().unwrap();
        let cli_path = temp.path().join("prospect-arrival-board.json");

        crate::commands::icecast::run_prospect_arrival_board(
            input,
            "2026-09-15T12:00:00Z".to_owned(),
            true,
            Some(cli_path.clone()),
        )
        .unwrap();
        let cli_board: ProspectArrivalBoardView =
            serde_json::from_slice(&std::fs::read(cli_path).unwrap()).unwrap();
        assert_eq!(cli_board, expected);

        let app = icelines_web::router(icelines_web::WebState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/prospect-arrivals/20262027")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap();
        let web_board: ProspectArrivalBoardView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_board, expected);

        let first = prospect_arrival_board_lines(&expected, 0, 80);
        let second = prospect_arrival_board_lines(&expected, 1, 80);
        let nyr = first
            .iter()
            .chain(&second)
            .find(|line| line.split_whitespace().any(|cell| cell == "NYR"))
            .unwrap();
        assert!(nyr.starts_with("NR"));
        assert!(nyr.contains("2/2"));
        assert!(nyr.contains("100%"));
    }

    #[tokio::test]
    async fn l2_prospect_census_readiness_golden_parity_across_tui_and_web() {
        let expected = prospect_census_readiness_board().clone();
        let app = icelines_web::router(icelines_web::WebState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/prospect-census-readiness/20262027")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let web_board: ProspectCensusReadinessBoardView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_board, expected);

        let first = prospect_census_readiness_board_lines(&expected, 0, 80);
        let second = prospect_census_readiness_board_lines(&expected, 1, 80);
        let nyr = first
            .iter()
            .chain(&second)
            .find(|line| line.split_whitespace().any(|cell| cell == "NYR"))
            .unwrap();
        assert!(nyr.contains("Incomplete"));
        assert!(nyr.contains("PopulationIncomplete"));
        assert!(nyr.contains("26"));
        assert!(nyr.contains("0/10"));
    }

    #[tokio::test]
    async fn l2_prospect_authority_progress_golden_parity_across_tui_and_web() {
        let expected = prospect_authority_progress().clone();
        let app = icelines_web::router(icelines_web::WebState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/prospect-authority-progress/20262027")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["etag"].to_str().unwrap(),
            format!("\"{}\"", expected.fingerprint)
        );
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let web_progress: ProspectAuthorityProgressView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_progress, expected);

        let first = prospect_authority_progress_lines(&expected, 0, 80);
        let second = prospect_authority_progress_lines(&expected, 1, 80);
        let nyr = first
            .iter()
            .chain(&second)
            .find(|line| line.starts_with("NYR"))
            .unwrap();
        assert!(nyr.contains("ahl_current_assignment"));
        assert!(nyr.contains("population"));
        assert!(nyr.contains("closed"));
        assert!(nyr.contains("failed"));
        assert!(nyr.contains("resolved"));
    }

    #[tokio::test]
    async fn l2_prospect_authority_closure_golden_parity_across_tui_and_web() {
        let expected = prospect_authority_closure_board().clone();
        let app = icelines_web::router(icelines_web::WebState::new());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/prospect-authority-closure/20262027")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["etag"].to_str().unwrap(),
            format!("\"{}\"", expected.fingerprint)
        );
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let web_board: ProspectAuthorityClosureBoardView = serde_json::from_slice(&body).unwrap();
        assert_eq!(web_board, expected);

        let lines = (0..4)
            .flat_map(|page| prospect_authority_closure_lines(&expected, page, 80))
            .collect::<Vec<_>>();
        let nyr = lines
            .iter()
            .filter(|line| line.starts_with("NYR"))
            .collect::<Vec<_>>();
        assert_eq!(nyr.len(), 2);
        assert!(nyr
            .iter()
            .any(|line| line.contains("nhl_club_camp_publication")));
        assert!(nyr
            .iter()
            .any(|line| line.contains("nhl_contract_publication")));
        assert!(nyr.iter().all(|line| line.contains("failed")));
        assert!(nyr.iter().all(|line| line.contains("acquire")));
    }
}
