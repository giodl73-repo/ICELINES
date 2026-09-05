//! Active fantasy roster-gap and simulation TUI boards.

use icelines_core::{
    build_fantasy_simulation_view, resolve_fantasy_scenario_roster_details,
    view_model::{
        FantasyRosterGapInput, FantasyRosterGapView, FantasySimulationBuildInput,
        FantasySimulationConfidence, FantasySimulationHorizon, FantasySimulationRosterTeamInput,
        FantasySimulationScenarioRosterInput, FantasySimulationView, FantasyTodayState,
        FantasyTodayView,
    },
    FantasyTodayV2View, Scheme,
};
use icelines_fetch::{
    fantasy_today_service::{assemble_fantasy_today, FantasyTodayAssemblyRequest},
    schedule_remaining::remaining_games_by_team_from_cache,
};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame,
};

use crate::tui::app::App;
use crate::visual::{
    tui_error_style, tui_header_style, tui_meta_style, tui_panel_block, tui_selected_style,
    tui_title_style, tui_warning_style,
};

pub fn chrome() -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    ScreenChrome {
        title: "The Bench - Coach's Clipboard - roster gaps".to_string(),
        keybinds: vec![
            KeyHint::new("up/down", "select"),
            KeyHint::new("Enter", "player card"),
            KeyHint::new("g", "gap filters"),
            KeyHint::new(":", "command"),
        ],
    }
}

pub fn simulation_chrome() -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    ScreenChrome {
        title: "The Bench - The Line Blender - league simulation".to_string(),
        keybinds: vec![
            KeyHint::new("a", "add/drop scenario"),
            KeyHint::new(":", "command"),
        ],
    }
}

pub fn today_chrome() -> crate::tui::chrome::ScreenChrome {
    use crate::tui::chrome::{KeyHint, ScreenChrome};
    ScreenChrome {
        title: "The Bench - Fantasy Today".to_string(),
        keybinds: vec![
            KeyHint::new("g", "roster gaps"),
            KeyHint::new("r", "refresh"),
            KeyHint::new(":", "command"),
        ],
    }
}

pub fn render_today(f: &mut Frame, app: &App, area: Rect) {
    let block = tui_panel_block(" The Bench - Fantasy Today ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    match (&app.fantasy_today.view, &app.fantasy_today.error) {
        (Some(view), _) => f.render_widget(Paragraph::new(today_v2_lines(view)), inner),
        (None, Some(message)) => f.render_widget(
            Paragraph::new(format!(
                "Fantasy cockpit unavailable\n\n{message}\n\nRun `icelines fantasy today` for recovery guidance."
            ))
            .style(tui_error_style()),
            inner,
        ),
        (None, None) => f.render_widget(
            Paragraph::new(
                "Fantasy cockpit unavailable: it has not loaded yet.\n\nPress r to refresh, or run `icelines fantasy today` for recovery guidance.",
            )
            .style(tui_warning_style()),
            inner,
        ),
    }
}

#[derive(Debug, Clone, Default)]
pub struct FantasyTodayScreenState {
    pub view: Option<FantasyTodayV2View>,
    pub error: Option<String>,
    pub loaded_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn load_today_contract(stats_season: &str) -> Result<FantasyTodayV2View, String> {
    let request = FantasyTodayAssemblyRequest::from_default_paths(
        None,
        None,
        stats_season.to_owned(),
        icelines_core::CURRENT_SEASON,
        chrono::Utc::now(),
    )
    .map_err(|error| error.to_string())?;
    assemble_fantasy_today(request).map_err(|error| error.to_string())
}

fn today_v2_lines(view: &FantasyTodayV2View) -> Vec<Line<'static>> {
    let summary = view.surface_decision();
    let mut projected = view.today.clone();
    if let Some(primary) = &view.decisions.primary_decision {
        let mut action = primary.action.clone();
        action.message = summary.primary_display_message();
        projected.primary_decision = Some(action);
    }
    projected.alternatives = view
        .decisions
        .alternatives
        .iter()
        .map(|row| row.action.clone())
        .collect();
    projected.next_decision_deadline_utc = summary.deadline_utc;
    let mut lines = today_lines(&projected);
    lines.push(Line::styled(
        format!("Decision {}", summary.material_fingerprint),
        tui_meta_style(),
    ));
    lines
}

fn today_lines(view: &FantasyTodayView) -> Vec<Line<'static>> {
    let state_style = match view.state {
        FantasyTodayState::Ready => tui_header_style(),
        FantasyTodayState::Provisional => tui_warning_style(),
        FantasyTodayState::Blocked => tui_error_style(),
    };
    let mut lines = vec![
        Line::styled(
            format!(
                "{} / {} / {}",
                view.context.league_name, view.context.fantasy_team_name, view.context.date
            ),
            tui_title_style(),
        ),
        Line::styled(
            format!("{:?}", view.state).to_ascii_uppercase(),
            state_style,
        ),
        Line::raw(""),
        Line::styled("DO NOW", tui_header_style()),
        Line::raw(
            view.primary_decision
                .as_ref()
                .map(|action| action.message.clone())
                .unwrap_or_else(|| "No action recommended.".to_owned()),
        ),
    ];
    if let Some(deadline) = view.next_decision_deadline_utc {
        lines.push(Line::styled(
            format!("Deadline: {}", deadline.to_rfc3339()),
            tui_warning_style(),
        ));
    }
    lines.extend([
        Line::raw(""),
        Line::raw(format!(
            "Lineup  {} starts | {} open | {} bench games",
            view.lineup.usable_starts,
            view.lineup.open_active_slots,
            view.lineup.bench_players_with_games
        )),
        Line::raw(format!(
            "Adds    {}/{} used | {} proactive",
            view.acquisitions.used, view.acquisitions.limit, view.acquisitions.proactive_remaining
        )),
    ]);
    if let Some(matchup) = &view.matchup {
        lines.push(Line::raw(format!(
            "Matchup {} | {} | {}",
            matchup.opponent, matchup.matchup_state, matchup.recommendation
        )));
    }
    if let Some(quiet) = &view.quiet_nights {
        lines.push(Line::raw(format!(
            "Quiet   {} usable starts | best {} ({})",
            quiet.usable_substitute_starts,
            quiet.best_substitute.as_deref().unwrap_or("-"),
            quiet.best_substitute_team.as_deref().unwrap_or("-")
        )));
    }
    for row in view
        .readiness
        .iter()
        .filter(|row| row.state != FantasyTodayState::Ready)
    {
        lines.push(Line::styled(
            format!("{}: {}", row.workflow, row.message),
            tui_warning_style(),
        ));
    }
    lines
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasySimulationScreenState {
    pub weeks: u8,
    pub add_player: Option<String>,
    pub drop_player: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyGapsScreenState {
    pub categories: Vec<String>,
    pub limit: usize,
}

impl Default for FantasyGapsScreenState {
    fn default() -> Self {
        Self {
            categories: Vec::new(),
            limit: 16,
        }
    }
}

impl FantasyGapsScreenState {
    pub fn context_label(&self) -> String {
        let categories = if self.categories.is_empty() {
            "scheme-cats".to_string()
        } else {
            self.categories.join(",")
        };
        format!("{categories} | top {}", self.limit)
    }
}

impl Default for FantasySimulationScreenState {
    fn default() -> Self {
        Self {
            weeks: 4,
            add_player: None,
            drop_player: None,
        }
    }
}

impl FantasySimulationScreenState {
    pub fn has_scenario(&self) -> bool {
        self.add_player.is_some() || self.drop_player.is_some()
    }

    pub fn scenario_label(&self) -> String {
        match (&self.add_player, &self.drop_player) {
            (Some(add), Some(drop)) => format!("Add {add} / drop {drop}"),
            (Some(add), None) => format!("Add {add}"),
            (None, Some(drop)) => format!("Drop {drop}"),
            (None, None) => "No add/drop scenario".to_string(),
        }
    }
}

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = tui_panel_block(" The Bench - Coach's Clipboard - roster gaps ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let view = match build_view(app) {
        Ok(view) => view,
        Err(message) => {
            f.render_widget(
                Paragraph::new(format!(
                    "Fantasy roster gaps unavailable\n\n{message}\n\nRun `icelines fantasy team-use <name>` after importing or creating a league."
                ))
                .style(tui_error_style()),
                inner,
            );
            return;
        }
    };

    let mut items = Vec::new();
    items.push(ListItem::new(Line::styled(
        format!(
            "  {} / {} / {}",
            view.league, view.team, view.scoring_scheme
        ),
        tui_title_style(),
    )));
    items.push(ListItem::new(Line::styled(
        format!("  {}", app.fantasy_gaps.context_label()),
        tui_meta_style(),
    )));
    for warning in &view.warnings {
        items.push(ListItem::new(Line::styled(
            format!("  warning: {warning}"),
            tui_warning_style(),
        )));
    }
    items.push(ListItem::new(Line::styled(
        format!(
            "  {:<3} {:<9} {:<12} {:>8} {:>5} {:<22} {:<4} {:>7} {:>7} {:<22} {}",
            "Rk",
            "Action",
            "Category",
            "Roster",
            "Wgt",
            "Best Available",
            "Pos",
            "Raw",
            "Delta",
            "Drop",
            "Why"
        ),
        tui_meta_style(),
    )));
    items.push(ListItem::new(Line::styled(
        format!("  {}", "-".repeat(112)),
        tui_meta_style(),
    )));

    for (idx, row) in view.rows.iter().enumerate() {
        let selected = idx == app.selected.min(view.rows.len().saturating_sub(1));
        let candidate = row.best_available.as_ref();
        let target = row.replacement_target.as_ref();
        let weighted_delta = target
            .map(|target| target.weighted_delta)
            .unwrap_or(row.weighted_gap_score);
        let style = if selected {
            tui_selected_style()
        } else if weighted_delta > 0.0 {
            tui_header_style()
        } else {
            tui_meta_style()
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(
                "  {:<3} {:<9} {:<12} {:>8.1} {:>5.2} {:<22} {:<4} {:>7.1} {:>7.1} {:<22} {}",
                idx + 1,
                format!("{:?}", row.action).to_ascii_lowercase(),
                truncate(&row.category, 12),
                row.user_total,
                row.weight,
                truncate(
                    candidate
                        .map(|candidate| candidate.display_name.as_str())
                        .unwrap_or("-"),
                    22
                ),
                candidate
                    .map(|candidate| candidate.position.as_str())
                    .unwrap_or("-"),
                candidate.map(|candidate| candidate.value).unwrap_or(0.0),
                weighted_delta,
                truncate(
                    target
                        .map(|target| target.display_name.as_str())
                        .unwrap_or("-"),
                    22
                ),
                truncate(&row.recommendation, 34),
            ),
            style,
        )])));
    }

    f.render_widget(List::new(items), inner);
}

pub fn render_simulation(f: &mut Frame, app: &App, area: Rect) {
    let block = tui_panel_block(" The Bench - The Line Blender - league simulation ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let view = match build_simulation_view(app) {
        Ok(view) => view,
        Err(message) => {
            f.render_widget(
                Paragraph::new(format!(
                    "Fantasy simulation unavailable\n\n{message}\n\nRun `icelines fantasy team-use <name>` after importing or creating a league."
                ))
                .style(tui_error_style()),
                inner,
            );
            return;
        }
    };

    let mut items = Vec::new();
    items.push(ListItem::new(Line::styled(
        format!(
            "  {} / {} / {:?}",
            view.league, view.scoring_scheme, view.horizon
        ),
        tui_title_style(),
    )));
    if app.fantasy_sim.has_scenario() {
        items.push(ListItem::new(Line::styled(
            format!("  scenario: {}", app.fantasy_sim.scenario_label()),
            tui_meta_style(),
        )));
    }
    for warning in &view.warnings {
        items.push(ListItem::new(Line::styled(
            format!("  warning: {warning}"),
            tui_warning_style(),
        )));
    }
    for assumption in &view.assumptions {
        items.push(ListItem::new(Line::styled(
            format!("  assumption: {assumption}"),
            tui_meta_style(),
        )));
    }
    items.push(ListItem::new(Line::styled(
        format!(
            "  {:<4} {:<28} {:<18} {:>12} {:>9} {:>8} {:>8}",
            "Rank", "Team", "Owner", "Projected", "Gap", "Games", "Players"
        ),
        tui_meta_style(),
    )));
    items.push(ListItem::new(Line::styled(
        format!("  {}", "-".repeat(96)),
        tui_meta_style(),
    )));

    for row in &view.rows {
        let selected = row.is_user_team;
        let style = if selected {
            tui_selected_style()
        } else {
            tui_meta_style()
        };
        items.push(ListItem::new(Line::from(vec![Span::styled(
            format!(
                "  {:<4} {:<28} {:<18} {:>12.1} {:>9.1} {:>8} {:>8}",
                row.rank,
                truncate(
                    if row.is_user_team {
                        format!("{} (mine)", row.team)
                    } else {
                        row.team.clone()
                    }
                    .as_str(),
                    28
                ),
                truncate(&row.owner, 18),
                row.projected_score,
                row.score_gap_to_leader,
                row.games_remaining,
                row.rostered_players,
            ),
            style,
        )])));
    }

    if !view.scenarios.is_empty() {
        items.push(ListItem::new(Line::from("")));
        items.push(ListItem::new(Line::styled(
            "  Scenarios",
            tui_title_style(),
        )));
        items.push(ListItem::new(Line::styled(
            format!(
                "  {:<9} {:<22} {:>9} {:>6} {:<10} {}",
                "Action", "Scenario", "Delta", "Games", "Confidence", "Why"
            ),
            tui_meta_style(),
        )));
        items.push(ListItem::new(Line::styled(
            format!("  {}", "-".repeat(106)),
            tui_meta_style(),
        )));
        for (scenario, row_text) in view.scenarios.iter().zip(simulation_scenario_rows(&view)) {
            let style = match scenario.action {
                icelines_core::FantasySimulationAction::Improve => tui_header_style(),
                icelines_core::FantasySimulationAction::Watch => tui_warning_style(),
                icelines_core::FantasySimulationAction::Avoid => tui_meta_style(),
            };
            items.push(ListItem::new(Line::from(vec![Span::styled(
                row_text, style,
            )])));
        }
    }

    f.render_widget(List::new(items), inner);
}

fn simulation_scenario_rows(view: &FantasySimulationView) -> Vec<String> {
    view.scenarios
        .iter()
        .map(|scenario| {
            format!(
                "  {:<9} {:<22} {:>+9.1} {:>+6} {:<10} {}",
                format!("{:?}", scenario.action).to_ascii_lowercase(),
                truncate(&scenario.label, 22),
                scenario.projected_score_delta,
                scenario.projected_games_delta,
                format!("{:?}", scenario.confidence).to_ascii_lowercase(),
                truncate(&scenario.explanation, 42),
            )
        })
        .collect()
}

pub fn selected_player_id(app: &App) -> Option<icelines_core::identity::PlayerId> {
    let view = build_view(app).ok()?;
    view.rows
        .get(app.selected.min(view.rows.len().saturating_sub(1)))
        .and_then(|row| row.best_available.as_ref())
        .map(|candidate| icelines_core::identity::PlayerId(candidate.player_id))
}

fn build_simulation_view(app: &App) -> anyhow::Result<FantasySimulationView> {
    let db = crate::fantasy_db::FantasyDb::open()?;
    let league = db
        .get_active_league()?
        .ok_or_else(|| anyhow::anyhow!("no active fantasy league found"))?;
    let user_team = db.get_user_team(&league.id)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no user team marked in '{}'; run `icelines fantasy team-use <name>`",
            league.name
        )
    })?;
    let scheme = Scheme::builtin_named(&league.scheme)
        .ok_or_else(|| anyhow::anyhow!("unknown scoring scheme '{}'", league.scheme))?;
    let skaters = app
        .repo
        .skaters(app.active_season_typed, app.active_type)
        .collect::<Vec<_>>();
    let goalies = app
        .repo
        .goalies(app.active_season_typed, app.active_type)
        .collect::<Vec<_>>();
    let schedule_cache = remaining_games_by_team_from_cache(app.active_season_typed);
    let schedule_available = !schedule_cache.remaining_by_team.is_empty();
    let snapshot = db.league_snapshot(None)?;
    let mut scenario_rosters = Vec::new();
    let mut warnings = if schedule_available {
        Vec::new()
    } else {
        vec![
            "schedule unavailable in TUI; projection falls back to current fantasy score"
                .to_string(),
        ]
    };
    if app.fantasy_sim.has_scenario() {
        let baseline = snapshot
            .teams
            .iter()
            .find(|team| team.name == snapshot.user_team)
            .map(|team| team.roster.clone())
            .unwrap_or_default();
        match resolve_fantasy_scenario_roster_details(
            &baseline,
            app.fantasy_sim.add_player.as_deref(),
            app.fantasy_sim.drop_player.as_deref(),
            &skaters,
            &goalies,
        ) {
            Ok(resolution) => {
                scenario_rosters.push(FantasySimulationScenarioRosterInput {
                    id: "tui-add-drop".to_string(),
                    label: app.fantasy_sim.scenario_label(),
                    add_player: resolution
                        .resolved_add_player
                        .or_else(|| app.fantasy_sim.add_player.clone()),
                    drop_player: resolution
                        .resolved_drop_player
                        .or_else(|| app.fantasy_sim.drop_player.clone()),
                    baseline_roster: baseline,
                    scenario_roster: resolution.roster,
                    confidence: FantasySimulationConfidence::Low,
                });
            }
            Err(message) => warnings.push(format!("scenario error: {message}")),
        }
    }

    Ok(build_fantasy_simulation_view(
        FantasySimulationBuildInput {
            season: app.active_season_typed,
            season_type: app.active_type,
            league: snapshot.league,
            scoring_scheme: league.scheme.clone(),
            horizon: FantasySimulationHorizon::Weeks(app.fantasy_sim.weeks.max(1)),
            user_team: user_team.name,
            teams: snapshot
                .teams
                .into_iter()
                .map(|team| FantasySimulationRosterTeamInput {
                    team: team.name,
                    owner: team.owner,
                    roster: team.roster,
                })
                .collect(),
            remaining_by_team: schedule_cache.remaining_by_team,
            scenarios: Vec::new(),
            scenario_rosters,
            assumptions: vec![
                "projects each roster from season-to-date fantasy points per played game"
                    .to_string(),
                "games remaining use the local schedule cache when available".to_string(),
            ],
            warnings,
            schedule_available,
        },
        &skaters,
        &goalies,
        &scheme,
    ))
}

fn build_view(app: &App) -> anyhow::Result<FantasyRosterGapView> {
    let db = crate::fantasy_db::FantasyDb::open()?;
    let snapshot = db.league_snapshot(None)?;
    let all_rostered = snapshot.all_rostered();
    let user_rostered = snapshot.user_rostered();
    Ok(FantasyRosterGapView::from_repository(
        &app.repo,
        FantasyRosterGapInput {
            season: app.active_season_typed,
            season_type: app.active_type,
            league: &snapshot.league,
            team: &snapshot.user_team,
            scoring_scheme: &snapshot.scoring_scheme,
            categories: app.fantasy_gaps.categories.clone(),
            user_roster_keys: user_rostered,
            all_rostered_keys: all_rostered,
            limit: app.fantasy_gaps.limit,
        },
    ))
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut out = value.chars().take(max_chars - 3).collect::<String>();
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{
        model::Season,
        season_stats::SeasonType,
        view_model::{SourceKind, ViewContext, ViewWindow},
        FantasySimulationAction, FantasySimulationConfidence, FantasySimulationScenarioRow,
        FantasySimulationTeamRow, SourceState,
    };
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn l0_fantasy_gaps_tui_empty_state_names_recovery() {
        let app = App::new(true);
        let backend = TestBackend::new(120, 16);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| render(f, &app, f.area())).unwrap();
        let buf = term.backend().buffer();
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }

        assert!(out.contains("Fantasy roster gaps unavailable"));
        assert!(out.contains("fantasy team-use"));
    }

    #[test]
    fn l0_fantasy_gaps_tui_chrome_names_surface() {
        let chrome = super::chrome();

        assert!(chrome.title.contains("The Bench"));
        assert!(chrome.title.contains("roster gaps"));
        assert!(chrome.title.contains("Coach's Clipboard"));
        assert!(chrome.keybinds.iter().any(|key| key.key == "Enter"));
        assert!(chrome.keybinds.iter().any(|key| key.key == "g"));
    }

    #[test]
    fn l0_fantasy_sim_tui_chrome_names_scenario_shortcut() {
        let chrome = super::simulation_chrome();

        assert!(chrome.title.contains("The Bench"));
        assert!(chrome.title.contains("league simulation"));
        assert!(chrome.title.contains("The Line Blender"));
        assert!(chrome.keybinds.iter().any(|key| key.key == "a"));
    }

    #[test]
    fn l0_fantasy_today_tui_chrome_names_cockpit() {
        let chrome = super::today_chrome();
        assert!(chrome.title.contains("Fantasy Today"));
        assert!(chrome.keybinds.iter().any(|key| key.key == ":"));
    }

    #[test]
    fn l0_fantasy_today_tui_has_designed_degradation_at_80_and_120_columns() {
        for width in [80, 120] {
            let app = App::new(true);
            let backend = TestBackend::new(width, 18);
            let mut term = Terminal::new(backend).unwrap();
            term.draw(|f| render_today(f, &app, f.area())).unwrap();
            let buffer = term.backend().buffer();
            let mut output = String::new();
            for y in 0..buffer.area.height {
                for x in 0..buffer.area.width {
                    output.push_str(buffer[(x, y)].symbol());
                }
                output.push('\n');
            }
            assert!(output.contains("Fantasy cockpit unavailable"));
            assert!(output.contains("fantasy today"));
        }
    }

    #[test]
    fn l0_fantasy_today_tui_consumes_the_sealed_surface_projection() {
        let fixture: icelines_core::FantasyTodaySurfaceDecision =
            serde_json::from_str(include_str!(
                "../../../../icelines-core/tests/fixtures/fantasy_today_surface_decision.v1.json"
            ))
            .unwrap();

        assert!(fixture
            .primary_display_message()
            .starts_with("Start Fixture Player [firm; legal now: true"));
        assert_eq!(fixture.alternative_messages[0], "Bench Fixture Goalie");
        assert!(fixture.deadline_utc.is_some());
    }

    #[test]
    fn l0_fantasy_sim_tui_formats_scenario_rows() {
        let view = FantasySimulationView {
            context: ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular)),
            league: "Main".to_string(),
            scoring_scheme: "yahoo-standard".to_string(),
            horizon: FantasySimulationHorizon::Weeks(4),
            user_team: "Mine".to_string(),
            rows: vec![FantasySimulationTeamRow {
                rank: 1,
                team: "Mine".to_string(),
                owner: "Me".to_string(),
                is_user_team: true,
                projected_score: 100.0,
                games_remaining: 10,
                rostered_players: 1,
                score_gap_to_leader: 0.0,
            }],
            scenarios: vec![FantasySimulationScenarioRow {
                id: "swap".to_string(),
                label: "Web add/drop scenario".to_string(),
                action: FantasySimulationAction::Improve,
                add_player: Some("Connor McDavid".to_string()),
                drop_player: Some("Bench Forward".to_string()),
                projected_score_delta: 12.5,
                projected_games_delta: 1,
                confidence: FantasySimulationConfidence::Low,
                explanation: "Connor McDavid for Bench Forward improves projected score by 12.5."
                    .to_string(),
            }],
            assumptions: Vec::new(),
            warnings: Vec::new(),
            source_state: vec![SourceState::complete(SourceKind::FantasyImport)],
        };

        let rows = simulation_scenario_rows(&view);

        assert_eq!(rows.len(), 1);
        assert!(rows[0].contains("improve"));
        assert!(rows[0].contains("+12.5"));
        assert!(rows[0].contains("+1"));
        assert!(rows[0].contains("low"));
        assert!(rows[0].contains("Connor McDavid"));
    }
}
