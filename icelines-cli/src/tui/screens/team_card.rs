//! TUI projection of the UI-neutral `card_document.v1` contract.
//!
//! This module deliberately imports document/section types only. It does not
//! score players, run simulations, or rebuild scenario data.

use std::{collections::BTreeMap, sync::OnceLock};

use icelines_core::{
    load_organization_window_profile_inventory, parse_card_document,
    project_organization_window_card, validate_organization_window_board, CardDocumentView,
    CardSectionView, OrganizationWindowBoardView, CANONICAL_TEAMS,
};
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
const ORGANIZATION_WINDOW_BOARD_JSON: &str =
    include_str!("../../../../examples/organization-window-board-evaluation-2026-27.json");
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
    let page = app.selected.min(1);
    if team.eq_ignore_ascii_case("WINDOW-BOARD") {
        render_organization_window_board(f, area, page);
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
        render_document(f, area, card(team), page);
    }
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
    let mut rows = board.organizations.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.overall
            .rank
            .unwrap_or(usize::MAX)
            .cmp(&right.overall.rank.unwrap_or(usize::MAX))
            .then_with(|| {
                right
                    .overall
                    .score
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&left.overall.score.unwrap_or(f64::NEG_INFINITY))
            })
            .then_with(|| left.organization.cmp(&right.organization))
    });
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
        let classification = if row.overall.rank.is_some() {
            format!("{:?}", row.overall.classification)
        } else {
            "Under review".to_owned()
        };
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
                lines.push(format!(
                    "-- {}: {} --",
                    section.title,
                    section.provenance_ids.join(", ")
                ));
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
        assert!(insider.contains("earlier-run, later-run"));
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
        assert!(insider.contains("Profiles and evidence"));
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

    #[tokio::test]
    async fn l2_organization_window_golden_parity_across_cli_tui_and_web() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let board_path =
            manifest_dir.join("../examples/organization-window-board-evaluation-2026-27.json");
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
        assert_eq!(cli_card, expected_card);

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
        assert_eq!(web_card, expected_card);

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
            "Score: 42.4",
            "League rank: NR",
            "Confidence: 9%",
            "Coverage: 16%",
        ] {
            assert!(tui_card.contains(expected), "TUI missing {expected}");
            let value = expected.split_once(": ").unwrap().1;
            assert!(html.contains(value), "Web HTML missing {value}");
        }
        let tui_board = organization_window_board_lines(&expected_board, 1, 80);
        let nyr_row = tui_board
            .iter()
            .find(|line| line.split_whitespace().any(|cell| cell == "NYR"))
            .unwrap();
        let cells = nyr_row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(&cells[..5], &["NR", "NYR", "42.4", "9%", "16%"]);
    }
}
