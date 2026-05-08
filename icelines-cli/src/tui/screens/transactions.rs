// Phase Norris.3 — `TransactionsState` repeats the module name in
// the type identifier. Same canonical pattern as Norris.1/2.
#![allow(clippy::module_name_repetitions)]

//! Transactions tab — Phase T.5.
//!
//! GLASS contract:
//! - Column order: TEAM (identity) → KIND (glyph + bold-color) → DATE → DESCRIPTION (ellipsis-truncated, never wrapped).
//! - Glyph on every kind so color is supplementary (deuteranopia + WCAG 1.4.1).
//! - Color the kind token only; everything else default fg.
//! - Title bar carries provenance ("ESPN · as of …"); footer is keybindings.
//! - `[STALE]` red prefix when the meta flag is set OR fetched_at > 7d.
//! - Empty / pre-coverage state renders a centered card with the glyph legend.
//! - `T` / `k` / `d` / `/` for team / kind / date / search filtering.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use icelines_core::transactions::description_matches_query;
use icelines_core::{Transaction, TransactionKind};

use crate::tui::app::App;

// ── Phase Norris.3 — per-screen state struct ─────────────────────────────────

/// Phase Norris.3 — owns every piece of state that belongs to the
/// Transactions tab. Replaces the 8 fields previously scattered
/// across `App` (`transactions`, `transactions_fetched_at`,
/// `transactions_stale`, `tx_selected`, `tx_team_filter`,
/// `tx_kind_filter`, `tx_search_query`, `tx_search_mode`).
///
/// **Naming asymmetry**: App holds this as `app.txs` (not
/// `app.txs.rows`) because the previous Vec field was already
/// named `transactions`, and naming the new struct field
/// `transactions` would create substring overlap during the rename
/// (`app.txs.fetched_at` and `app.txs.rows` would both
/// match a bare `app.txs.rows` regex). `txs` matches the
/// existing `tx_*` field-name heritage.
#[derive(Debug)]
pub struct TransactionsState {
    /// Loaded transactions envelope. Empty until the loader picks
    /// up the snapshot.
    pub rows: Vec<Transaction>,
    /// Wall-clock string ("YYYY-MM-DDThh:mm:ss-04:00") from the
    /// snapshot envelope; surfaced in the title bar.
    pub fetched_at: String,
    /// True when the most recent fetch failed (read from
    /// `SnapshotMetaFlags::transactions_stale`). Drives the red
    /// `[STALE]` prefix in the title bar.
    pub stale: bool,
    /// Selected row index on the Transactions tab.
    pub selected: usize,
    /// Filter to a single team abbrev (None = all). Cycles via `T`.
    pub team_filter: Option<String>,
    /// Filter to a single kind (None = all). Cycles via `k`.
    pub kind_filter: Option<TransactionKind>,
    /// Substring filter against the description (case-insensitive).
    /// Live-applied as the user types in search mode.
    pub search_query: String,
    /// True while the `/` search bar is open.
    pub search_mode: bool,
}

impl Default for TransactionsState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            fetched_at: String::new(),
            stale: false,
            selected: 0,
            team_filter: None,
            kind_filter: None,
            search_query: String::new(),
            search_mode: false,
        }
    }
}

// ── Public glyph + color tables ───────────────────────────────────────────────
//
// `pub` because the L1 colorblind test reads these to assert that every
// rendered kind row carries the right glyph in its plain-text projection.

/// One-char glyph for each kind. Drives the colorblind-safe contract:
/// even with all color stripped, every row is unambiguous.
pub fn glyph_for(k: TransactionKind) -> &'static str {
    match k {
        TransactionKind::Trade => "⇄",
        TransactionKind::Signing => "$",
        TransactionKind::Recall => "↑",
        TransactionKind::Reassignment => "↓",
        TransactionKind::WaiverPlacement => "⊘",
        TransactionKind::WaiverClear => "↻",
        TransactionKind::WaiverClaim => "+",
        TransactionKind::InjuryReserve => "✚",
        TransactionKind::Other => "◇",
    }
}

/// Color contract per GLASS. Magenta dropped (collapses with Cyan under
/// protanopia) — WaiverClaim uses Blue+Bold instead.
pub fn color_for(k: TransactionKind) -> Color {
    match k {
        TransactionKind::Trade => Color::Cyan,
        TransactionKind::Signing => Color::Yellow,
        TransactionKind::Recall => Color::Green,
        TransactionKind::Reassignment => Color::DarkGray,
        TransactionKind::WaiverPlacement => Color::Blue,
        TransactionKind::WaiverClear => Color::Blue,
        TransactionKind::WaiverClaim => Color::Blue,
        TransactionKind::InjuryReserve => Color::Red,
        TransactionKind::Other => Color::White,
    }
}

/// Capitalized kind label used in the KIND column (`Trade`, `Signing`, …).
pub fn kind_display(k: TransactionKind) -> &'static str {
    match k {
        TransactionKind::Trade => "Trade",
        TransactionKind::Signing => "Signing",
        TransactionKind::Recall => "Recall",
        TransactionKind::Reassignment => "Reassign",
        TransactionKind::WaiverPlacement => "WaiverPlace",
        TransactionKind::WaiverClear => "WaiverClear",
        TransactionKind::WaiverClaim => "WaiverClaim",
        TransactionKind::InjuryReserve => "IR",
        TransactionKind::Other => "Other",
    }
}

// ── Render entry point ───────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    // ── Empty state: pre-coverage season OR no rows loaded ────────────
    if app.txs.rows.is_empty() {
        render_empty_legend_card(f, app, area);
        return;
    }

    // Outer frame — title bar carries provenance + stale marker; footer
    // carries keybindings.
    let title = title_text(app);
    let block = Block::default().borders(Borders::ALL).title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let show_search = app.txs.search_mode || !app.txs.search_query.is_empty();
    let constraints: Vec<Constraint> = if show_search {
        vec![
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Min(0), Constraint::Length(1)]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut idx = 0;
    if show_search {
        render_search_bar(f, app, chunks[idx]);
        idx += 1;
    }
    let rows = filter_rows(app);
    render_rows(f, app, chunks[idx], &rows);
    render_footer(f, chunks[idx + 1]);
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let cursor = if app.txs.search_mode { "_" } else { "" };
    let line = Line::from(vec![
        Span::styled("  /", cyan),
        Span::styled("search: ", dim),
        Span::styled(format!("{}{cursor}", app.txs.search_query), cyan),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn title_text(app: &App) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    if app.txs.stale {
        spans.push(Span::styled(
            " [STALE] ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }
    spans.push(Span::raw(" Transactions · ESPN"));
    if !app.txs.fetched_at.is_empty() {
        spans.push(Span::styled(
            format!(" · as of {} ", app.txs.fetched_at),
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(
        format!("· {} rows ", app.txs.rows.len()),
        Style::default().fg(Color::DarkGray),
    ));
    Line::from(spans)
}

pub fn filter_rows(app: &App) -> Vec<&Transaction> {
    app.txs.rows
        .iter()
        .filter(|tx| {
            if let Some(team) = app.txs.team_filter.as_deref() {
                let row_label = tx.team.as_ref().map(|t| t.0.as_str()).unwrap_or("LEAGUE");
                if !row_label.eq_ignore_ascii_case(team) {
                    return false;
                }
            }
            if let Some(k) = app.txs.kind_filter {
                if tx.kind != k {
                    return false;
                }
            }
            if !app.txs.search_query.trim().is_empty()
                && !description_matches_query(&tx.description, &app.txs.search_query)
            {
                return false;
            }
            true
        })
        .collect()
}

fn render_rows(f: &mut Frame, app: &App, area: Rect, rows: &[&Transaction]) {
    if rows.is_empty() {
        let dim = Style::default().fg(Color::DarkGray);
        let mut hint = String::from("  No rows match the current filters.");
        if app.txs.team_filter.is_some() {
            hint.push_str("  T to clear team.");
        }
        if app.txs.kind_filter.is_some() {
            hint.push_str("  k to clear kind.");
        }
        f.render_widget(Paragraph::new(Line::styled(hint, dim)), area);
        return;
    }

    // Available width for the description column = total - (team + kind + date + separators).
    // Team col 7, Kind col 13 (glyph + label), Date col 11. Separators 6.
    let inner_w = area.width as usize;
    let fixed = 7 + 13 + 11 + 6;
    let desc_w = inner_w.saturating_sub(fixed).max(20);

    let selected = app.txs.selected.min(rows.len().saturating_sub(1));
    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, tx)| {
            let team_label = tx.team.as_ref().map(|t| t.0.as_str()).unwrap_or("LEAGUE");
            let glyph = glyph_for(tx.kind);
            let kind_label = kind_display(tx.kind);
            let kind_color = color_for(tx.kind);
            let date = tx.date.as_str();
            let desc = truncate_with_ellipsis(&tx.description, desc_w);

            // GLASS contract: only the kind cell is colored (glyph + label
            // together, bold). All other cells default fg.
            let row_style = if i == selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let kind_style = if i == selected {
                // On the selected row, color is taken over by the highlight bg.
                // Bold + glyph still distinguishes the kind even when fg is
                // overridden by the selection.
                row_style
            } else {
                Style::default().fg(kind_color).add_modifier(Modifier::BOLD)
            };

            let line = Line::from(vec![
                Span::styled(format!("  {team_label:<6}"), row_style),
                Span::styled(format!(" {glyph} {kind_label:<10}"), kind_style),
                Span::styled(format!(" {date:<10}"), row_style),
                Span::styled(format!(" {desc}"), row_style),
            ]);
            ListItem::new(line)
        })
        .collect();

    f.render_widget(List::new(items), area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let dim = Style::default().fg(Color::DarkGray);
    let cyan = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let line = Line::from(vec![
        Span::styled("  /", cyan),
        Span::styled(":search  ", dim),
        Span::styled("t", cyan),
        Span::styled("/", dim),
        Span::styled("T", cyan),
        Span::styled(":team ±  ", dim),
        Span::styled("k", cyan),
        Span::styled("/", dim),
        Span::styled("K", cyan),
        Span::styled(":kind ±  ", dim),
        Span::styled("Esc", cyan),
        Span::styled(":clear filters", dim),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

// ── Empty / pre-coverage state ───────────────────────────────────────────────

fn render_empty_legend_card(f: &mut Frame, app: &App, area: Rect) {
    let title = if app.txs.stale {
        " [STALE] Transactions · ESPN "
    } else {
        " Transactions · ESPN "
    };
    let outer = Block::default().borders(Borders::ALL).title(title);
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let dim = Style::default().fg(Color::DarkGray);
    let gold = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let coverage_line = format!(
        "  Coverage begins {}.",
        format_season(icelines_core::transactions::TRANSACTIONS_EARLIEST_SEASON),
    );

    let body: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::styled("  No transactions for this season yet.", gold),
        Line::from(""),
        Line::styled("  Run `icelines fetch transactions` to populate, or", dim),
        Line::styled(coverage_line, dim),
        Line::from(""),
        Line::styled("  Kind glyphs:", gold),
        glyph_legend_line(TransactionKind::Trade),
        glyph_legend_line(TransactionKind::Signing),
        glyph_legend_line(TransactionKind::Recall),
        glyph_legend_line(TransactionKind::Reassignment),
        glyph_legend_line(TransactionKind::WaiverPlacement),
        glyph_legend_line(TransactionKind::WaiverClear),
        glyph_legend_line(TransactionKind::WaiverClaim),
        glyph_legend_line(TransactionKind::InjuryReserve),
        glyph_legend_line(TransactionKind::Other),
    ];

    let para = Paragraph::new(body)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn glyph_legend_line(k: TransactionKind) -> Line<'static> {
    let glyph = glyph_for(k);
    let label = kind_display(k);
    let color = color_for(k);
    Line::from(vec![
        Span::raw("    "),
        Span::styled(
            format!("{glyph} {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

// ── Filter ring helpers (T.5+: forward + Shift-backward) ──────────────────────

/// Sorted, deduped list of every team label that appears in the loaded
/// transactions, including the synthetic `LEAGUE` bucket for teamless rows.
pub fn transactions_teams(transactions: &[Transaction]) -> Vec<String> {
    let mut teams: Vec<String> = transactions
        .iter()
        .map(|tx| {
            tx.team
                .as_ref()
                .map(|t| t.0.clone())
                .unwrap_or_else(|| "LEAGUE".to_owned())
        })
        .collect();
    teams.sort();
    teams.dedup();
    teams
}

/// Cycle team filter forward: None → first → next → ... → last → None (wrap).
pub fn cycle_team_forward(current: Option<&str>, teams: &[String]) -> Option<String> {
    match current {
        None => teams.first().cloned(),
        Some(curr) => {
            let pos = teams.iter().position(|t| t == curr);
            match pos {
                Some(i) if i + 1 < teams.len() => Some(teams[i + 1].clone()),
                _ => None, // wrap back to "all teams"
            }
        }
    }
}

/// Cycle team filter backward: None → last → prev → ... → first → None (wrap).
pub fn cycle_team_backward(current: Option<&str>, teams: &[String]) -> Option<String> {
    if teams.is_empty() {
        return None;
    }
    match current {
        None => teams.last().cloned(),
        Some(curr) => {
            let pos = teams.iter().position(|t| t == curr);
            match pos {
                Some(0) => None, // wrap back to "all teams"
                Some(i) => Some(teams[i - 1].clone()),
                None => None, // current isn't in list — drop the filter
            }
        }
    }
}

/// Cycle kind filter forward: None → first → next → ... → last → None (wrap).
pub fn cycle_kind_forward(
    current: Option<TransactionKind>,
    cycle: &[TransactionKind],
) -> Option<TransactionKind> {
    match current {
        None => cycle.first().copied(),
        Some(curr) => {
            let pos = cycle.iter().position(|k| *k == curr);
            match pos {
                Some(i) if i + 1 < cycle.len() => Some(cycle[i + 1]),
                _ => None,
            }
        }
    }
}

/// Cycle kind filter backward: None → last → prev → ... → first → None (wrap).
pub fn cycle_kind_backward(
    current: Option<TransactionKind>,
    cycle: &[TransactionKind],
) -> Option<TransactionKind> {
    if cycle.is_empty() {
        return None;
    }
    match current {
        None => cycle.last().copied(),
        Some(curr) => {
            let pos = cycle.iter().position(|k| *k == curr);
            match pos {
                Some(0) => None,
                Some(i) => Some(cycle[i - 1]),
                None => None,
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn format_season(s: &str) -> String {
    if s.len() == 8 {
        format!("{}-{}", &s[2..4], &s[6..8])
    } else {
        s.to_owned()
    }
}

/// Truncate `s` so its char count ≤ `width`. If truncated, replaces the
/// last char with `…` so the row width is preserved.
pub fn truncate_with_ellipsis(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let count = s.chars().count();
    if count <= width {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::model::TeamAbbr;

    fn fixture_tx(kind: TransactionKind, description: &str) -> Transaction {
        Transaction {
            date: "2026-04-29".to_owned(),
            team: Some(TeamAbbr("EDM".to_owned())),
            kind,
            description: description.to_owned(),
            id: "id".to_owned(),
            trade_group_id: None,
            classifier_version: 1,
        }
    }

    #[test]
    fn l0_glyph_for_every_kind_is_unique() {
        // BENCH/GLASS-mandated colorblind safety: every kind has its own
        // glyph so the rendered text alone is sufficient to distinguish.
        use std::collections::HashSet;
        let glyphs: HashSet<&str> = TransactionKind::ALL.iter().map(|k| glyph_for(*k)).collect();
        assert_eq!(
            glyphs.len(),
            TransactionKind::ALL.len(),
            "every kind must have a unique glyph",
        );
    }

    #[test]
    fn l0_glyph_legend_card_includes_all_glyphs() {
        // Empty-state card must surface every glyph so users learn the
        // legend while waiting / browsing pre-coverage seasons.
        for k in TransactionKind::ALL {
            let line = glyph_legend_line(*k);
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert!(
                text.contains(glyph_for(*k)),
                "legend line for {:?} must include its glyph, got: {text}",
                k,
            );
        }
    }

    #[test]
    fn l0_kind_display_label_uniqueness() {
        use std::collections::HashSet;
        let labels: HashSet<&str> = TransactionKind::ALL
            .iter()
            .map(|k| kind_display(*k))
            .collect();
        assert_eq!(
            labels.len(),
            TransactionKind::ALL.len(),
            "every kind must have a unique display label"
        );
    }

    #[test]
    fn l0_truncate_with_ellipsis_short_passes_through() {
        assert_eq!(truncate_with_ellipsis("Acquired D X", 50), "Acquired D X");
    }

    #[test]
    fn l0_truncate_with_ellipsis_long_appends_dots() {
        let s =
            "Acquired D Ryan McDonagh from NSH for D Philippe Myers and a 2026 third-round pick";
        let t = truncate_with_ellipsis(s, 30);
        assert_eq!(t.chars().count(), 30, "truncated to exact width");
        assert!(t.ends_with('…'), "must end with ellipsis, got: {t}");
    }

    #[test]
    fn l0_truncate_with_ellipsis_zero_width_returns_empty() {
        assert_eq!(truncate_with_ellipsis("anything", 0), "");
    }

    #[test]
    fn l0_truncate_with_ellipsis_unicode_safe() {
        // Diacritics / multi-byte chars must not split mid-codepoint.
        let s = "Loaned G Hörnqvist to Sweden for the IIHF World Championship";
        let t = truncate_with_ellipsis(s, 20);
        assert_eq!(t.chars().count(), 20);
    }

    #[test]
    fn l0_filter_rows_no_filter_returns_all() {
        let mut app = App::new(false);
        app.txs.rows = vec![
            fixture_tx(TransactionKind::Trade, "Trade row"),
            fixture_tx(TransactionKind::Signing, "Signing row"),
        ];
        let rows = filter_rows(&app);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn l0_filter_rows_team_filter_drops_non_matching() {
        let mut app = App::new(false);
        app.txs.rows = vec![
            fixture_tx(TransactionKind::Trade, "Trade row"), // EDM
            Transaction {
                date: "2026-04-29".to_owned(),
                team: Some(TeamAbbr("CHI".to_owned())),
                kind: TransactionKind::Signing,
                description: "Chicago signing".to_owned(),
                id: "id2".to_owned(),
                trade_group_id: None,
                classifier_version: 1,
            },
        ];
        app.txs.team_filter = Some("EDM".to_owned());
        let rows = filter_rows(&app);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].team.as_ref().unwrap().0, "EDM");
    }

    #[test]
    fn l0_filter_rows_kind_filter_drops_non_matching() {
        let mut app = App::new(false);
        app.txs.rows = vec![
            fixture_tx(TransactionKind::Trade, "Trade row"),
            fixture_tx(TransactionKind::Signing, "Signing row"),
            fixture_tx(TransactionKind::Recall, "Recall row"),
        ];
        app.txs.kind_filter = Some(TransactionKind::Trade);
        let rows = filter_rows(&app);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, TransactionKind::Trade);
    }

    // ── L1: rendered-text snapshots — GLASS contract assertions ──────

    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buffer_text(buf: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn render_transactions_to_text(app: &App) -> String {
        let backend = TestBackend::new(120, 30);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let area = f.area();
            super::render(f, app, area);
        })
        .unwrap();
        buffer_text(term.backend().buffer())
    }

    fn one_row_per_kind() -> Vec<Transaction> {
        TransactionKind::ALL
            .iter()
            .enumerate()
            .map(|(i, k)| Transaction {
                date: format!("2026-04-{:02}", 10 + i),
                team: Some(TeamAbbr("EDM".to_owned())),
                kind: *k,
                description: format!("description for {}", kind_display(*k)),
                id: format!("id-{i}"),
                trade_group_id: None,
                classifier_version: 1,
            })
            .collect()
    }

    #[test]
    fn l1_tui_glyph_present_for_every_kind() {
        // BENCH/GLASS-mandated colorblind safety: render fixture with one
        // row per TransactionKind, strip everything but plain text, and
        // assert each glyph appears at least once. This fails CI before
        // shipping a render that drops the colorblind carrier.
        let mut app = App::new(false);
        app.txs.rows = one_row_per_kind();
        app.txs.fetched_at = "2026-04-30".to_owned();

        let text = render_transactions_to_text(&app);
        for k in TransactionKind::ALL {
            let g = glyph_for(*k);
            assert!(
                text.contains(g),
                "rendered output is missing glyph '{g}' for {:?}; \
                 colorblind users would not be able to distinguish kinds.\n\
                 full text:\n{text}",
                k,
            );
        }
    }

    #[test]
    fn l1_tui_kind_display_label_present_for_every_kind() {
        let mut app = App::new(false);
        app.txs.rows = one_row_per_kind();
        app.txs.fetched_at = "2026-04-30".to_owned();

        let text = render_transactions_to_text(&app);
        for k in TransactionKind::ALL {
            let label = kind_display(*k);
            assert!(
                text.contains(label),
                "rendered output is missing kind label '{label}' for {:?}",
                k,
            );
        }
    }

    #[test]
    fn l1_tui_stale_marker_renders_in_title() {
        let mut app = App::new(false);
        app.txs.rows = vec![fixture_tx(TransactionKind::Trade, "Some trade")];
        app.txs.fetched_at = "2026-04-30".to_owned();
        app.txs.stale = true;

        let text = render_transactions_to_text(&app);
        assert!(
            text.contains("[STALE]"),
            "stale flag must render '[STALE]' in title, got:\n{text}"
        );
    }

    #[test]
    fn l1_tui_empty_state_renders_legend_card_with_all_glyphs() {
        // No rows loaded → render_empty_legend_card path. Card must list
        // every glyph so users learn the legend while waiting.
        let app = App::new(false);
        let text = render_transactions_to_text(&app);
        for k in TransactionKind::ALL {
            assert!(
                text.contains(glyph_for(*k)),
                "empty-state legend missing glyph for {:?}",
                k
            );
        }
        assert!(
            text.contains("Coverage begins"),
            "empty-state must hint at coverage starting season, got:\n{text}"
        );
    }

    #[test]
    fn l1_tui_long_description_truncated_with_ellipsis() {
        let mut app = App::new(false);
        let long =
            "Acquired D Ryan McDonagh from NSH for D Philippe Myers and a 2026 third-round pick \
                    plus future considerations and a conditional 2027 second";
        app.txs.rows = vec![fixture_tx(TransactionKind::Trade, long)];
        app.txs.fetched_at = "2026-04-30".to_owned();

        let text = render_transactions_to_text(&app);
        assert!(
            text.contains("…"),
            "long description must be truncated with ellipsis, got:\n{text}"
        );
        // No newline mid-description (description must NEVER wrap).
        // Loose check: each line in the buffer is exactly the buffer width
        // chars long, so wrapping wouldn't split a single tx into two
        // visible lines. Sanity-check by counting lines starting with "  EDM"
        // — should be exactly one.
        let edm_lines = text.lines().filter(|l| l.contains("EDM")).count();
        assert_eq!(
            edm_lines, 1,
            "description must not wrap (expected 1 EDM row, got {edm_lines})"
        );
    }

    #[test]
    fn l0_filter_rows_search_query_substring_match() {
        let mut app = App::new(false);
        app.txs.rows = vec![
            fixture_tx(TransactionKind::Trade, "Acquired D Ryan McDonagh from NSH"),
            fixture_tx(
                TransactionKind::Signing,
                "Signed F Connor Bedard to a 8-year extension",
            ),
            fixture_tx(
                TransactionKind::Recall,
                "Recalled F Vasily Podkolzin from Bakersfield",
            ),
        ];
        app.txs.search_query = "bedard".to_owned();
        let rows = filter_rows(&app);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].description.contains("Bedard"));
    }

    #[test]
    fn l0_filter_rows_search_query_empty_matches_all() {
        let mut app = App::new(false);
        app.txs.rows = vec![
            fixture_tx(TransactionKind::Trade, "row a"),
            fixture_tx(TransactionKind::Signing, "row b"),
        ];
        app.txs.search_query = "".to_owned();
        assert_eq!(filter_rows(&app).len(), 2);
        app.txs.search_query = "   ".to_owned();
        assert_eq!(filter_rows(&app).len(), 2);
    }

    #[test]
    fn l0_filter_rows_search_combines_with_other_filters() {
        let mut app = App::new(false);
        app.txs.rows = vec![
            fixture_tx(TransactionKind::Trade, "Acquired D Ryan McDonagh from NSH"), // EDM
            fixture_tx(
                TransactionKind::Signing,
                "Signed F McDonagh to a 1-year deal",
            ), // EDM (hypothetical)
        ];
        app.txs.search_query = "mcdonagh".to_owned();
        app.txs.kind_filter = Some(TransactionKind::Trade);
        let rows = filter_rows(&app);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, TransactionKind::Trade);
    }

    #[test]
    fn l1_tui_search_bar_renders_when_search_mode_active() {
        let mut app = App::new(false);
        app.txs.rows = vec![fixture_tx(TransactionKind::Trade, "Some trade")];
        app.txs.fetched_at = "2026-04-30".to_owned();
        app.txs.search_mode = true;
        app.txs.search_query = "mcd".to_owned();
        let text = render_transactions_to_text(&app);
        assert!(
            text.contains("search:"),
            "search bar must render when mode is active, got:\n{text}"
        );
        assert!(
            text.contains("mcd"),
            "search bar must echo the typed query, got:\n{text}"
        );
    }

    // ── Cycle ring helpers (forward + Shift-backward) ────────────────

    fn three_teams() -> Vec<String> {
        vec!["BOS".to_owned(), "CHI".to_owned(), "EDM".to_owned()]
    }

    #[test]
    fn l0_cycle_team_forward_walks_in_order() {
        let teams = three_teams();
        assert_eq!(cycle_team_forward(None, &teams), Some("BOS".to_owned()));
        assert_eq!(
            cycle_team_forward(Some("BOS"), &teams),
            Some("CHI".to_owned())
        );
        assert_eq!(
            cycle_team_forward(Some("CHI"), &teams),
            Some("EDM".to_owned())
        );
        assert_eq!(
            cycle_team_forward(Some("EDM"), &teams),
            None,
            "wraps from last → all teams"
        );
    }

    #[test]
    fn l0_cycle_team_backward_walks_in_reverse() {
        let teams = three_teams();
        assert_eq!(
            cycle_team_backward(None, &teams),
            Some("EDM".to_owned()),
            "Shift-T from None must jump to the last team"
        );
        assert_eq!(
            cycle_team_backward(Some("EDM"), &teams),
            Some("CHI".to_owned())
        );
        assert_eq!(
            cycle_team_backward(Some("CHI"), &teams),
            Some("BOS".to_owned())
        );
        assert_eq!(
            cycle_team_backward(Some("BOS"), &teams),
            None,
            "wraps from first → all teams"
        );
    }

    #[test]
    fn l0_cycle_team_forward_then_backward_round_trips() {
        let teams = three_teams();
        // Tap team forward 4 times: None → BOS → CHI → EDM → None
        // Shift-back 4 times: None → EDM → CHI → BOS → None
        // End at the start.
        let mut state: Option<String> = None;
        for _ in 0..4 {
            state = cycle_team_forward(state.as_deref(), &teams);
        }
        assert_eq!(state, None, "4 forwards on a 3-list must wrap to None");
        for _ in 0..4 {
            state = cycle_team_backward(state.as_deref(), &teams);
        }
        assert_eq!(state, None, "4 backwards must also wrap to None");
    }

    #[test]
    fn l0_cycle_team_empty_list_safe() {
        let teams: Vec<String> = vec![];
        assert_eq!(cycle_team_forward(None, &teams), None);
        assert_eq!(cycle_team_backward(None, &teams), None);
        assert_eq!(cycle_team_forward(Some("ANY"), &teams), None);
        assert_eq!(cycle_team_backward(Some("ANY"), &teams), None);
    }

    #[test]
    fn l0_cycle_kind_forward_walks_full_ring() {
        let cycle = TransactionKind::ALL;
        let mut state: Option<TransactionKind> = None;
        let mut seen = Vec::new();
        for _ in 0..cycle.len() {
            state = cycle_kind_forward(state, cycle);
            seen.push(state);
        }
        // After ALL.len() forwards we should have seen every kind once.
        assert_eq!(
            seen.iter().filter(|s| s.is_some()).count(),
            cycle.len(),
            "every kind must appear exactly once during a full forward walk"
        );
        // One more tap wraps to None.
        state = cycle_kind_forward(state, cycle);
        assert_eq!(state, None);
    }

    #[test]
    fn l0_cycle_kind_backward_walks_in_reverse() {
        let cycle = TransactionKind::ALL;
        let last = *cycle.last().unwrap();
        assert_eq!(
            cycle_kind_backward(None, cycle),
            Some(last),
            "Shift-K from None must jump to the last kind"
        );
        let second_last = cycle[cycle.len() - 2];
        assert_eq!(cycle_kind_backward(Some(last), cycle), Some(second_last));
    }

    #[test]
    fn l0_cycle_kind_forward_and_backward_invert() {
        let cycle = TransactionKind::ALL;
        for &k in cycle {
            // forward(backward(k)) should always reach k somewhere on the path,
            // but stronger property: backward then forward returns to k.
            let back = cycle_kind_backward(Some(k), cycle);
            let fwd = cycle_kind_forward(back, cycle);
            assert_eq!(
                fwd,
                Some(k),
                "backward then forward from {:?} must round-trip; got {:?}",
                k,
                fwd
            );
        }
    }

    #[test]
    #[allow(non_snake_case)] // Test name encodes the literal "LEAGUE" sentinel value.
    fn l0_transactions_teams_dedups_and_includes_LEAGUE() {
        let txs = vec![
            fixture_tx(TransactionKind::Trade, "EDM trade"), // team = EDM
            Transaction {
                date: "x".into(),
                team: None,
                kind: TransactionKind::Other,
                description: "League-wide".into(),
                id: "i".into(),
                trade_group_id: None,
                classifier_version: 1,
            },
            fixture_tx(TransactionKind::Recall, "EDM recall"), // dup team
        ];
        let teams = transactions_teams(&txs);
        assert_eq!(teams, vec!["EDM".to_owned(), "LEAGUE".to_owned()]);
    }

    #[test]
    #[allow(non_snake_case)] // Test name encodes the literal "LEAGUE" sentinel value.
    fn l0_filter_rows_team_LEAGUE_returns_only_teamless() {
        let mut app = App::new(false);
        app.txs.rows = vec![
            fixture_tx(TransactionKind::Trade, "EDM trade"),
            Transaction {
                date: "2026-04-27".to_owned(),
                team: None,
                kind: TransactionKind::Other,
                description: "League-wide".to_owned(),
                id: "id3".to_owned(),
                trade_group_id: None,
                classifier_version: 1,
            },
        ];
        app.txs.team_filter = Some("LEAGUE".to_owned());
        let rows = filter_rows(&app);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].team.is_none());
    }
}
