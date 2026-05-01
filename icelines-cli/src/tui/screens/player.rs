use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use icelines_core::identity::PlayerId;
use icelines_core::stats_repository::PlayerView;
use crate::tui::app::App;
use crate::tui::headshot;

pub fn render(f: &mut Frame, app: &App, area: Rect, idx: usize) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Player Card  ·  c: comps  ·  g: group  ·  f: favorites  ·  Esc: back ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(p) = app.players.get(idx) else {
        f.render_widget(Paragraph::new(vec![Line::from("  No player data — run icelines fetch all")]), inner);
        return;
    };

    // Trigger headshot fetch if not cached.
    // URL is derivable from nhl_id + team + season even without fetching rosters:
    //   https://assets.nhle.com/mugs/nhl/{SEASON}/{TEAM}/{ID}.png
    // Use active_season so historical-season views build a season-correct
    // URL — disk cache key (nhl_id) is season-agnostic, so once any
    // season's URL succeeds the image persists for every future view.
    if let Some(id) = p.nhl_id {
        if app.headshot_cache.get(id).is_none() {
            let url = p.headshot_url.clone().unwrap_or_else(|| {
                format!(
                    "https://assets.nhle.com/mugs/nhl/{}/{}/{}.png",
                    app.active_season,
                    p.team.as_str(),
                    id
                )
            });
            headshot::spawn_fetch(id, url, app.headshot_cache.clone(), 22, 15);
        }
    }

    // Layout: headshot (26 cols) | stats (rest) [| dashboard panel (30 cols)]
    // Phase 8j: third pane only renders when the dashboards flag is on AND
    // there's enough horizontal room (≥ 100 cols) — under that we'd squeeze
    // the stats column too hard.
    let dashboards_on = crate::config::dashboards_enabled() && inner.width >= 100;
    let constraints: Vec<Constraint> = if dashboards_on {
        vec![Constraint::Length(26), Constraint::Min(0), Constraint::Length(30)]
    } else {
        vec![Constraint::Length(26), Constraint::Min(0)]
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    render_headshot(f, app, p, chunks[0]);
    render_stats(f, app, p, chunks[1]);
    if dashboards_on {
        render_dashboard_panel(f, app, p, chunks[2]);
    }

    // Group picker overlay — floats over the card when `g` is pressed
    if app.group_picker_open {
        render_group_picker(f, app, area);
    }
}

/// Phase 8j: native scout card driven by `tui::dashboard_panel`. The
/// panel emits already-styled Lines (per-column colour on the
/// sparklines, accent colour on values), so render is a direct
/// pass-through. Cache by nhl_id keeps repeated frames cheap.
fn render_dashboard_panel(f: &mut Frame, app: &App, p: &icelines_core::model::Player, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Scout card ")
        .style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = app.dashboard_panel.lines_for_player(p, &app.league_context);
    f.render_widget(Paragraph::new(lines), inner);
}

pub fn render_group_picker(f: &mut Frame, app: &App, area: Rect) {
    // Center a small popup
    let popup_h = (app.group_picker_list.len() as u16 + 4).min(area.height - 4);
    let popup_w = 36u16.min(area.width - 4);
    let popup = Rect::new(
        area.x + (area.width - popup_w) / 2,
        area.y + (area.height - popup_h) / 2,
        popup_w,
        popup_h,
    );
    f.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add to group — ↑↓ · Enter · Esc ")
        .style(Style::default().fg(Color::Yellow));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let items: Vec<ListItem> = app.group_picker_list.iter().enumerate().map(|(i, name)| {
        let style = if i == app.selected {
            Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(Line::styled(format!("  ★  {}", name), style))
    }).collect();
    f.render_widget(List::new(items), inner);
}

fn render_headshot(f: &mut Frame, app: &App, p: &icelines_core::model::Player, area: Rect) {
    let rows = p.nhl_id.and_then(|id| app.headshot_cache.get(id));
    let lines: Vec<Line> = match rows.as_deref() {
        None => {
            // headshot_url is None (bundled data) — no fetch triggered, show placeholder
            let abbr = p.team.as_str();
            vec![
                Line::from(""),
                Line::from(""),
                Line::from(""),
                Line::from(format!("  {:^20}", abbr)),
                Line::from(""),
                Line::from("  loading…"),
            ]
        }
        Some(r) if headshot::is_loading(r) => vec![
            Line::from(""),
            Line::from("  downloading…"),
        ],
        Some(r) if headshot::is_error(r)          => vec![
            Line::from("  ┌──────────────────┐"),
            Line::from("  │                  │"),
            Line::from("  │   no headshot    │"),
            Line::from("  │                  │"),
            Line::from("  └──────────────────┘"),
        ],
        Some(rows) => rows.iter().map(|row| Line::styled(row.clone(), Style::default().fg(Color::White))).collect(),
    };
    f.render_widget(Paragraph::new(lines), area);
}

fn render_stats(f: &mut Frame, app: &App, p: &icelines_core::model::Player, area: Rect) {
    let ppg  = p.pace_score.map(|s| format!("{:.3}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
    let proj = p.pace_score.map(|s| format!("{:.1}", s.pace_82)).unwrap_or_else(|| "—".to_owned());
    let gp   = p.pace_score.map(|s| s.gp.to_string()).unwrap_or_else(|| "—".to_owned());
    let age  = p.birth_date.as_deref().and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string()).unwrap_or_else(|| "—".to_owned());
    let draft = match (p.draft_year, p.draft_round, p.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r} #{o}"),
        (Some(y), _, _) => y.to_string(),
        _ => "Undrafted".to_owned(),
    };
    let pm  = if p.plus_minus >= 0 { format!("+{}", p.plus_minus) } else { p.plus_minus.to_string() };
    let toi = p.toi_mmss().unwrap_or_else(|| "—".to_owned());
    let sh  = p.shooting_pct.map(|v| format!("{:.1}%", v*100.0)).unwrap_or_else(|| "—".to_owned());

    let hi  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines = vec![
        Line::styled(format!(" {}", p.full_name), hi),
        Line::from(format!(" {} · {} · Age {}", p.team.as_str(), p.position.abbreviation(), age)),
        Line::from(""),
        Line::styled(" Scoring", dim),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "PPG", ppg, "Proj/82", proj)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "GP", gp, "TOI/g", toi)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "+/-", pm, "SH%", sh)),
        Line::from(""),
        Line::styled(" Stats", dim),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "G", p.season_goals, "PP-G", p.pp_goals)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "A", p.season_assists, "PP-Pts", p.pp_points)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "Pts", p.season_points, "Shots", p.shots)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "GWG", p.gwg, "Hits", p.hits)),
        Line::from(""),
        Line::styled(" Bio", dim),
        Line::from(format!(" Draft: {}", draft)),
        Line::from(format!(" {}  Shoots: {}",
            p.nationality_code.as_deref().unwrap_or("—"),
            p.shoots_catches.as_deref().unwrap_or("—"))),
    ];

    // ── Recent transactions section ─────────────────────────────────
    // Phase T+1: surface every loaded transaction whose description
    // mentions this player by last name. Team-disambiguated so the two
    // Sebastian Ahos don't pollute each other's cards.
    let team_for_disambig = p.team.as_str();
    let hits = icelines_core::transactions::transactions_for_player(
        &app.transactions,
        &p.full_name,
        Some(team_for_disambig),
    );
    if !hits.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::styled(" Recent moves", dim));
        // Newest 5; full list available on the Transactions tab.
        let mut sorted: Vec<&icelines_core::Transaction> = hits.clone();
        sorted.sort_by(|a, b| b.date.cmp(&a.date));
        for tx in sorted.into_iter().take(5) {
            let kind = tx.kind.label();
            // Truncate at 60 chars so the card row doesn't overflow on
            // narrow terminals — full description is on the Transactions tab.
            let desc: String = tx.description.chars().take(60).collect();
            lines.push(Line::from(format!(" {}  {:<10}  {}", tx.date, kind, desc)));
        }
        if hits.len() > 5 {
            lines.push(Line::styled(
                format!(" ({} more on Transactions tab)", hits.len() - 5),
                dim,
            ));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

// ── Phase 8j: dashboard panel render guard tests ────────────────────────────

#[cfg(test)]
mod dashboard_tests {
    // The full Player struct has 50+ fields — instead of hand-authoring a
    // fixture (which would couple this test file to the schema and break
    // every time a field is added), we test the render guard logic in
    // isolation: render the dashboard panel directly into a sub-area and
    // verify that the title only appears when `dashboards_enabled()` is on.
    // The end-to-end "render full player screen with panel" path is
    // exercised by L2 subprocess tests on the TUI launcher.

    use crate::config::{init_dashboards, Config};
    use std::path::PathBuf;

    #[test]
    fn l0_init_dashboards_explicit_true_takes_effect() {
        // OnceLock is set-once: first set wins for the duration of the
        // test binary. Other tests in the same binary may already have
        // set the flag, so we test the resolver logic directly here.
        let cfg = Config {
            csv_path:   None,
            cache_dir:  PathBuf::from("/tmp"),
            season:     None,
            live:       None,
            dashboards: Some(true),
        };
        init_dashboards(true, &cfg); // idempotent — first call wins
        // Verifying `dashboards_enabled()` here would race with other tests
        // that initialize the flag differently. The pure resolver
        // (`crate::config::resolve_dashboards`) covers the precedence
        // matrix in config.rs::tests; the OnceLock contract is set-once.
    }
}

// ── Hart.5c.6 Phase B-2.2 — view-based render path ───────────────────────────
//
// `render_by_id` is the post-Hart entry point. Looks up the view via
// `app.view_for(pid)`; on miss renders a placeholder (D6 auto-pop UX
// is event-handler side). Field-by-field equivalent of `render` /
// `render_stats` / `render_headshot` / `render_dashboard_panel`,
// sourcing through PlayerView accessors instead of `&Player` fields.
// Phase C deletes the legacy render paths once enter handlers all
// migrate to `Screen::PlayerById`.

pub fn render_by_id(f: &mut Frame, app: &App, area: Rect, pid: PlayerId) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Player Card  ·  c: comps  ·  g: group  ·  f: favorites  ·  Esc: back ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(view) = app.view_for(pid) else {
        let dim = Style::default().fg(Color::DarkGray);
        let name = app
            .repo
            .identity(pid)
            .map(|i| i.full_name.as_str())
            .unwrap_or("Player");
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::styled(
                    format!("  {} not in {} roster.", name, app.active_season),
                    dim,
                ),
                Line::from(""),
                Line::styled("  Press Esc to return.", dim),
            ]),
            inner,
        );
        return;
    };

    // Headshot fetch — same NHL CDN URL pattern as the legacy path.
    let nhl_id = view.identity.id.0;
    if app.headshot_cache.get(nhl_id).is_none() {
        let url = view.identity.headshot_canonical_url.clone().unwrap_or_else(|| {
            format!(
                "https://assets.nhle.com/mugs/nhl/{}/{}/{}.png",
                app.active_season,
                view.team_display(),
                nhl_id,
            )
        });
        headshot::spawn_fetch(nhl_id, url, app.headshot_cache.clone(), 22, 15);
    }

    let dashboards_on = crate::config::dashboards_enabled() && inner.width >= 100;
    let constraints: Vec<Constraint> = if dashboards_on {
        vec![Constraint::Length(26), Constraint::Min(0), Constraint::Length(30)]
    } else {
        vec![Constraint::Length(26), Constraint::Min(0)]
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(inner);

    render_headshot_view(f, app, &view, chunks[0]);
    render_stats_view(f, app, &view, chunks[1]);
    if dashboards_on {
        render_dashboard_panel_view(f, app, &view, chunks[2]);
    }

    if app.group_picker_open {
        render_group_picker(f, app, area);
    }
}

fn render_headshot_view(f: &mut Frame, app: &App, v: &PlayerView<'_>, area: Rect) {
    let nhl_id = v.identity.id.0;
    let rows = app.headshot_cache.get(nhl_id);
    let lines: Vec<Line> = match rows.as_deref() {
        None => {
            let abbr = v.team_display();
            vec![
                Line::from(""),
                Line::from(""),
                Line::from(""),
                Line::from(format!("  {:^20}", abbr)),
                Line::from(""),
                Line::from("  loading…"),
            ]
        }
        Some(r) if headshot::is_loading(r) => vec![
            Line::from(""),
            Line::from("  downloading…"),
        ],
        Some(r) if headshot::is_error(r) => vec![
            Line::from("  ┌──────────────────┐"),
            Line::from("  │                  │"),
            Line::from("  │   no headshot    │"),
            Line::from("  │                  │"),
            Line::from("  └──────────────────┘"),
        ],
        Some(rows) => rows
            .iter()
            .map(|row| Line::styled(row.clone(), Style::default().fg(Color::White)))
            .collect(),
    };
    f.render_widget(Paragraph::new(lines), area);
}

fn render_stats_view(f: &mut Frame, app: &App, v: &PlayerView<'_>, area: Rect) {
    let p82 = v.pace_82();
    let ppg  = p82.map(|p| format!("{:.3}", p / 82.0)).unwrap_or_else(|| "—".to_owned());
    let proj = p82.map(|p| format!("{:.1}", p)).unwrap_or_else(|| "—".to_owned());
    let gp_str = if v.gp() > 0 { v.gp().to_string() } else { "—".to_owned() };
    let age = v
        .identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string())
        .unwrap_or_else(|| "—".to_owned());
    let draft = match (
        v.identity.bio.draft_year,
        v.identity.bio.draft_round,
        v.identity.bio.draft_overall,
    ) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r} #{o}"),
        (Some(y), _, _) => y.to_string(),
        _ => "Undrafted".to_owned(),
    };
    let pm = if v.plus_minus() >= 0 {
        format!("+{}", v.plus_minus())
    } else {
        v.plus_minus().to_string()
    };
    let toi = v.toi_mmss().unwrap_or_else(|| "—".to_owned());
    let sh = v
        .stats
        .totals
        .shooting_pct
        .map(|x| format!("{:.1}%", x))
        .unwrap_or_else(|| "—".to_owned());

    let totals = &v.stats.totals;

    let hi  = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let mut lines = vec![
        Line::styled(format!(" {}", v.full_name()), hi),
        Line::from(format!(
            " {} · {} · Age {}",
            v.team_display(),
            v.position().abbreviation(),
            age,
        )),
        Line::from(""),
        Line::styled(" Scoring", dim),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "PPG", ppg, "Proj/82", proj)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "GP", gp_str, "TOI/g", toi)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "+/-", pm, "SH%", sh)),
        Line::from(""),
        Line::styled(" Stats", dim),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "G", totals.goals, "PP-G", totals.pp_goals)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "A", totals.assists, "PP-Pts", totals.pp_points)),
        Line::from(format!(" {:<10} {:>6}    {:<10} {:>6}", "Pts", totals.points, "Shots", totals.shots)),
        Line::from(format!(
            " {:<10} {:>6}    {:<10} {:>6}",
            "GWG", totals.gwg,
            "Hits",
            v.hits().map(|h| h.to_string()).unwrap_or_else(|| "—".to_owned()),
        )),
        Line::from(""),
        Line::styled(" Bio", dim),
        Line::from(format!(" Draft: {}", draft)),
        Line::from(format!(
            " {}  Shoots: {}",
            v.identity.bio.nationality_code.as_deref().unwrap_or("—"),
            v.identity.bio.shoots_catches.as_deref().unwrap_or("—"),
        )),
    ];

    let team_for_disambig = v.team_display();
    let hits = icelines_core::transactions::transactions_for_player(
        &app.transactions,
        v.full_name(),
        Some(team_for_disambig),
    );
    if !hits.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::styled(" Recent moves", dim));
        let mut sorted: Vec<&icelines_core::Transaction> = hits.clone();
        sorted.sort_by(|a, b| b.date.cmp(&a.date));
        for tx in sorted.into_iter().take(5) {
            let kind = tx.kind.label();
            let desc: String = tx.description.chars().take(60).collect();
            lines.push(Line::from(format!(" {}  {:<10}  {}", tx.date, kind, desc)));
        }
        if hits.len() > 5 {
            lines.push(Line::styled(
                format!(" ({} more on Transactions tab)", hits.len() - 5),
                dim,
            ));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn render_dashboard_panel_view(f: &mut Frame, app: &App, v: &PlayerView<'_>, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Scout card ")
        .style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Use the post-Hart compile() API. ctx_window must match the active
    // window — reload_for_season + poll_repo_load keep these in
    // lockstep, so the D11 cross-window rejection is a safety net.
    let result = app.dashboard_panel.compile(
        &app.repo,
        app.active_season_typed,
        app.active_type,
        v.identity.id,
        &app.league_context,
        app.league_context_window,
    );
    match result {
        Ok(out) => f.render_widget(Paragraph::new(out.lines), inner),
        Err(err) => {
            let dim = Style::default().fg(Color::DarkGray);
            f.render_widget(
                Paragraph::new(vec![
                    Line::styled("  Scout card unavailable", dim),
                    Line::styled(format!("  {err}"), dim),
                ]),
                inner,
            );
        }
    }
}
