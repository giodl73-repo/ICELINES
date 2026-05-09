use std::path::PathBuf;

use anyhow::{bail, Context};
use serde::Serialize;

use icelines_core::{
    model::{Position, Season, TeamAbbr},
    name::normalize_name,
    view_model::{
        default_watch_rules_view, poach_report_from_board, weekly_poach_report_from_board,
        AvailabilityState, DeploymentSignal, PoachBoardView, PoachQuery, PoachReportView,
        SourceKind, SourceState, ViewContext, ViewWindow, WatchRule, WatchRuleTrigger,
        WatchRulesView,
    },
};

use crate::cli::QuerySeasonType;
use crate::commands::players::{load_repo_for_season, validate_bundled_season};
use crate::config::Config;
use crate::db::{GroupDb, MemberKind};

pub struct PoachArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub scheme: String,
    pub categories: Vec<String>,
    pub teams: Vec<String>,
    pub positions: Vec<String>,
    pub top: u16,
    pub json: bool,
}

pub struct PoachReportArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub scheme: String,
    pub categories: Vec<String>,
    pub teams: Vec<String>,
    pub positions: Vec<String>,
    pub top: u16,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub struct WeeklyReportArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub scheme: String,
    pub league: String,
    pub categories: Vec<String>,
    pub teams: Vec<String>,
    pub positions: Vec<String>,
    pub top: u16,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub struct WatchRulesArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub json: bool,
}

pub struct WatchListArgs {
    pub json: bool,
}

pub struct WatchNoteArgs {
    pub player: String,
    pub reason: String,
    pub json: bool,
}

pub struct WatchPlayerArgs {
    pub player: String,
    pub when: String,
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub json: bool,
}

pub struct WatchDeploymentArgs {
    pub team: Option<String>,
    pub line_change: bool,
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
    pub json: bool,
}

pub async fn run(args: PoachArgs) -> anyhow::Result<()> {
    let json = args.json;
    let view = build_board(args)?;
    emit_board(&view, json)
}

pub async fn run_report_poach(args: PoachReportArgs) -> anyhow::Result<()> {
    let board = build_board(PoachArgs {
        season: args.season,
        season_type: args.season_type,
        scheme: args.scheme,
        categories: args.categories,
        teams: args.teams,
        positions: args.positions,
        top: args.top,
        json: false,
    })?;
    let report = poach_report_from_board(board);
    let body = if args.json {
        serde_json::to_string_pretty(&report).context("serializing poach report")?
    } else {
        render_report_markdown(&report)
    };
    write_or_print(args.out.as_ref(), &body)
}

pub async fn run_report_weekly(args: WeeklyReportArgs) -> anyhow::Result<()> {
    let board = build_board(PoachArgs {
        season: args.season,
        season_type: args.season_type,
        scheme: args.scheme,
        categories: args.categories,
        teams: args.teams,
        positions: args.positions,
        top: args.top,
        json: false,
    })?;
    let report = weekly_poach_report_from_board(board, &args.league, args.top);
    let body = if args.json {
        serde_json::to_string_pretty(&report).context("serializing weekly poach report")?
    } else {
        render_report_markdown(&report)
    };
    write_or_print(args.out.as_ref(), &body)
}

pub async fn run_watch_rules(args: WatchRulesArgs) -> anyhow::Result<()> {
    let context = watch_context(args.season.as_deref(), args.season_type)?;
    emit_watch_view(&default_watch_rules_view(context), args.json)
}

pub async fn run_watch_list(args: WatchListArgs) -> anyhow::Result<()> {
    let db = GroupDb::open()?;
    let rows = watchlist_rows(&db)?;
    emit_watchlist_rows(&rows, args.json)
}

pub async fn run_watch_note(args: WatchNoteArgs) -> anyhow::Result<()> {
    let db = GroupDb::open()?;
    ensure_watchlist(&db)?;
    let key = normalize_name(&args.player);
    db.add_member_kind("Watchlist", &key, MemberKind::Player)?;
    db.upsert_watch_note(MemberKind::Player, &key, &args.reason, "manual")?;
    let row = watchlist_row_for(&db, MemberKind::Player, key)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&row).context("serializing watch note")?
        );
    } else {
        println!(
            "Watching '{}': {}",
            args.player,
            row.reason.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}

pub async fn run_watch_player(args: WatchPlayerArgs) -> anyhow::Result<()> {
    let context = watch_context(args.season.as_deref(), args.season_type)?;
    let view = WatchRulesView {
        context,
        rules: vec![player_watch_rule(&args.player, &args.when)],
        warnings: Vec::new(),
    };
    emit_watch_view(&view, args.json)
}

pub async fn run_watch_deployment(args: WatchDeploymentArgs) -> anyhow::Result<()> {
    let context = watch_context(args.season.as_deref(), args.season_type)?;
    let team = args.team.map(|team| TeamAbbr(team.trim().to_uppercase()));
    let label = if args.line_change {
        "Deployment line-change watch"
    } else {
        "Deployment promotion watch"
    };
    let view = WatchRulesView {
        context,
        rules: vec![WatchRule {
            id: "deployment-preview".to_string(),
            label: match &team {
                Some(team) => format!("{label} for {}", team.as_str()),
                None => label.to_string(),
            },
            enabled: true,
            trigger: WatchRuleTrigger::PlayerPromoted {
                player_id: None,
                evidence: DeploymentSignal::Unknown,
            },
            last_fired: None,
            unsupported_sources: vec![SourceKind::Shifts],
        }],
        warnings: Vec::new(),
    };
    emit_watch_view(&view, args.json)
}

fn build_board(args: PoachArgs) -> anyhow::Result<PoachBoardView> {
    let (outcome, season, season_type) =
        load_repo_for_season(args.season.as_deref(), Some(args.season_type.to_core()))?;

    let mut query = PoachQuery::new(season, season_type, args.scheme);
    query.categories = normalize_categories(args.categories);
    query.teams = args
        .teams
        .into_iter()
        .map(|team| TeamAbbr(team.trim().to_uppercase()))
        .collect();
    query.positions = parse_positions(args.positions)?;
    query.limit = Some(args.top);
    query.sort = Some("poach_score".to_string());

    Ok(PoachBoardView::from_repository(&outcome.repo, query))
}

fn emit_board(view: &PoachBoardView, json: bool) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing poach board")?
        );
    } else {
        print_table(view);
    }
    Ok(())
}

fn render_report_markdown(report: &PoachReportView) -> String {
    let mut out = String::new();
    out.push_str("# Fantasy Poacher\n\n");
    out.push_str(&format!(
        "- Season: {}\n- Type: {:?}\n- Scheme: {}\n- Window: {:?}\n\n",
        report.context.view_context.window.season,
        report.context.view_context.window.season_type,
        report.scoring_scheme,
        report.window
    ));

    if !report.omissions.is_empty() {
        out.push_str("## Source Omissions\n\n");
        for omission in &report.omissions {
            out.push_str(&format!("- {omission}\n"));
        }
        out.push('\n');
    }

    for section in &report.sections {
        out.push_str(&format!("## {}\n\n", section.title));
        if section.rows.is_empty() {
            out.push_str("No candidates matched this report.\n\n");
            continue;
        }
        out.push_str("| Rank | Player | Team | Pos | Score | Confidence | Why |\n");
        out.push_str("|---:|---|---|---|---:|---|---|\n");
        for (idx, row) in section.rows.iter().enumerate() {
            let why = row
                .explanations
                .first()
                .map(|explanation| explanation.message.as_str())
                .unwrap_or("No explanation");
            out.push_str(&format!(
                "| {} | {} | {} | {} | {:.1} | {:?} | {} |\n",
                idx + 1,
                markdown_cell(&row.display_name),
                row.team.as_str(),
                row.position.abbreviation(),
                row.score.final_score,
                row.confidence,
                markdown_cell(why)
            ));
        }
        out.push('\n');
    }
    out
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

fn write_or_print(out: Option<&PathBuf>, body: &str) -> anyhow::Result<()> {
    match out {
        Some(path) if path.as_os_str() != "-" => {
            std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))?;
        }
        _ => {
            print!("{body}");
        }
    }
    Ok(())
}

fn watch_context(
    season: Option<&str>,
    season_type: QuerySeasonType,
) -> anyhow::Result<ViewContext> {
    let cfg = Config::load()?;
    let season_str = match season {
        Some(season) => {
            validate_bundled_season(season)?;
            season.to_string()
        }
        None => cfg.season_str(),
    };
    let season_u32 = season_str
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{season_str}' is not a YYYYZZZZ id"))?;
    let mut context = ViewContext::new(ViewWindow::new(Season(season_u32), season_type.to_core()));
    context.completeness = icelines_core::Completeness::Partial;
    context.source_state = vec![
        SourceState::complete(SourceKind::Roster),
        SourceState::missing(SourceKind::Shifts),
        SourceState::missing(SourceKind::Schedule),
        SourceState::missing(SourceKind::FantasyImport),
    ];
    Ok(context)
}

fn player_watch_rule(player: &str, trigger: &str) -> WatchRule {
    let normalized_trigger = trigger.trim().to_ascii_lowercase();
    let (rule_trigger, unsupported_sources) = match normalized_trigger.as_str() {
        "available" | "availability" => (
            WatchRuleTrigger::AvailabilityChanged {
                player_id: None,
                state: AvailabilityState::Unknown,
            },
            vec![SourceKind::FantasyImport],
        ),
        "pp1" | "pp2" | "top-six" | "promotion" | "line-change" => (
            WatchRuleTrigger::PlayerPromoted {
                player_id: None,
                evidence: DeploymentSignal::Unknown,
            },
            vec![SourceKind::Shifts],
        ),
        _ => (
            WatchRuleTrigger::PlayerPromoted {
                player_id: None,
                evidence: DeploymentSignal::Unknown,
            },
            vec![SourceKind::Shifts],
        ),
    };

    WatchRule {
        id: format!("player-{}", slug(player)),
        label: format!("Watch {player} when {normalized_trigger}"),
        enabled: true,
        trigger: rule_trigger,
        last_fired: None,
        unsupported_sources,
    }
}

fn emit_watch_view(view: &WatchRulesView, json: bool) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(view).context("serializing watch rules")?
        );
        return Ok(());
    }

    println!("{:<28} {:<7} Unsupported", "Rule", "Enabled");
    for rule in &view.rules {
        let unsupported = if rule.unsupported_sources.is_empty() {
            "-".to_string()
        } else {
            rule.unsupported_sources
                .iter()
                .map(|source| format!("{source:?}").to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "{:<28} {:<7} {}",
            truncate(&rule.label, 28),
            if rule.enabled { "yes" } else { "no" },
            unsupported
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct WatchlistRow {
    kind: String,
    key: String,
    reason: Option<String>,
    source: Option<String>,
    updated_at: Option<String>,
}

fn ensure_watchlist(db: &GroupDb) -> anyhow::Result<()> {
    let exists = db
        .list_groups()?
        .iter()
        .any(|group| group.name == "Watchlist");
    if !exists {
        db.create_group("Watchlist", "Fantasy poacher watchlist")?;
    }
    Ok(())
}

fn watchlist_rows(db: &GroupDb) -> anyhow::Result<Vec<WatchlistRow>> {
    match db.list_members_with_kind("Watchlist") {
        Ok(members) => members
            .into_iter()
            .map(|(key, kind)| watchlist_row_for(db, kind, key))
            .collect(),
        Err(err) if err.to_string().contains("group 'Watchlist' not found") => Ok(Vec::new()),
        Err(err) => Err(err),
    }
}

fn watchlist_row_for(db: &GroupDb, kind: MemberKind, key: String) -> anyhow::Result<WatchlistRow> {
    let note = db.watch_note(kind, &key)?;
    Ok(WatchlistRow {
        kind: kind.as_str().to_string(),
        key,
        reason: note.as_ref().map(|note| note.reason.clone()),
        source: note.as_ref().map(|note| note.source.clone()),
        updated_at: note.map(|note| note.updated_at),
    })
}

fn emit_watchlist_rows(rows: &[WatchlistRow], json: bool) -> anyhow::Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(rows).context("serializing watchlist")?
        );
        return Ok(());
    }

    if rows.is_empty() {
        println!("Watchlist is empty. Add from TUI Poach with `w` or run `icelines watch note <player> <reason>`.");
        return Ok(());
    }

    println!("{:<8} {:<28} Reason", "Kind", "Key");
    for row in rows {
        println!(
            "{:<8} {:<28} {}",
            row.kind,
            truncate(&row.key, 28),
            truncate(row.reason.as_deref().unwrap_or("-"), 80)
        );
    }
    Ok(())
}

fn slug(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() || ch == '-' || ch == '_' {
                Some('-')
            } else {
                None
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn normalize_categories(categories: Vec<String>) -> Vec<String> {
    categories
        .into_iter()
        .map(|category| category.trim().to_ascii_lowercase())
        .filter(|category| !category.is_empty())
        .collect()
}

fn parse_positions(values: Vec<String>) -> anyhow::Result<Vec<Position>> {
    values
        .into_iter()
        .map(|value| parse_position(&value))
        .collect()
}

fn parse_position(value: &str) -> anyhow::Result<Position> {
    match value.trim().to_ascii_uppercase().as_str() {
        "C" => Ok(Position::Center),
        "LW" | "L" => Ok(Position::LeftWing),
        "RW" | "R" => Ok(Position::RightWing),
        "D" => Ok(Position::Defense),
        "G" => Ok(Position::Goalie),
        other => bail!("unknown position '{other}' - valid: C, LW, RW, D, G"),
    }
}

fn print_table(view: &PoachBoardView) {
    if let Some(empty) = &view.empty_state {
        println!("{}", empty.title);
        if let Some(detail) = &empty.detail {
            println!("{detail}");
        }
        return;
    }

    println!(
        "{:<4} {:<26} {:<4} {:<3} {:>6} {:<12} {:<28} Risk",
        "Rank", "Player", "Team", "Pos", "Score", "Confidence", "Why"
    );
    for (idx, row) in view.rows.iter().enumerate() {
        let why = row
            .explanations
            .first()
            .map(|explanation| explanation.message.as_str())
            .unwrap_or("No explanation");
        let risk = row.risk_summary.as_deref().unwrap_or("-");
        println!(
            "{:<4} {:<26} {:<4} {:<3} {:>6.1} {:<12} {:<28} {}",
            idx + 1,
            truncate(&row.display_name, 26),
            row.team.as_str(),
            row.position.abbreviation(),
            row.score.final_score,
            format!("{:?}", row.confidence).to_ascii_lowercase(),
            truncate(why, 28),
            truncate(risk, 24)
        );
    }

    if view.context.completeness != icelines_core::Completeness::Complete {
        println!(
            "\nSource state: {:?}. Missing schedule/import/shift data is disclosed, not scored as negative evidence.",
            view.context.completeness
        );
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let len = value.chars().count();
    if len <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }

    let mut out = String::new();
    for ch in value.chars().take(max_chars - 3) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_poach_normalizes_categories() {
        assert_eq!(
            normalize_categories(vec![
                " Hits".to_string(),
                "BLOCKS".to_string(),
                "".to_string()
            ]),
            vec!["hits".to_string(), "blocks".to_string()]
        );
    }

    #[test]
    fn l0_poach_parses_positions() {
        assert_eq!(
            parse_positions(vec!["C".to_string(), "lw".to_string()]).unwrap(),
            vec![Position::Center, Position::LeftWing]
        );
    }

    #[test]
    fn l0_poach_rejects_bad_position() {
        let err = parse_position("bad").unwrap_err().to_string();
        assert!(err.contains("valid: C, LW, RW, D, G"));
    }

    #[test]
    fn l0_poach_truncates_long_text() {
        assert_eq!(truncate("abcdef", 4), "a...");
        assert_eq!(truncate("abc", 4), "abc");
        assert_eq!(truncate("abcdef", 3), "...");
    }

    #[test]
    fn l0_poach_markdown_escapes_table_cells() {
        assert_eq!(markdown_cell("A|B"), "A\\|B");
    }

    #[test]
    fn l0_poach_report_markdown_includes_empty_section() {
        let context = icelines_core::ViewContext::new(icelines_core::ViewWindow::new(
            icelines_core::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        ));
        let report = PoachReportView {
            context: icelines_core::view_model::poach_report_context(context, "test-report"),
            scoring_scheme: "yahoo-standard".to_string(),
            window: icelines_core::view_model::PoachWindow::Days14,
            source_state: Vec::new(),
            warnings: Vec::new(),
            omissions: vec!["schedule: unavailable".to_string()],
            sections: vec![icelines_core::view_model::PoachReportSection {
                id: "top_adds".to_string(),
                title: "Top Adds".to_string(),
                rows: Vec::new(),
            }],
        };

        let md = render_report_markdown(&report);

        assert!(md.contains("# Fantasy Poacher"));
        assert!(md.contains("schedule: unavailable"));
        assert!(md.contains("No candidates matched this report."));
    }

    #[test]
    fn l0_poach_weekly_report_names_expected_sections() {
        let season = icelines_core::Season(20252026);
        let season_type = icelines_core::season_stats::SeasonType::Regular;
        let query = PoachQuery::new(season, season_type, "yahoo-standard");
        let board = PoachBoardView::new(
            icelines_core::ViewContext::new(icelines_core::ViewWindow::new(season, season_type)),
            query,
            "yahoo-standard",
        );

        let report = weekly_poach_report_from_board(board, "Main League", 20);
        let section_ids: Vec<_> = report
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect();

        assert_eq!(report.context.report_id, "weekly-main-league");
        assert_eq!(report.context.title, "Weekly Fantasy Prep");
        assert_eq!(report.context.sections.len(), report.sections.len());
        assert_eq!(
            section_ids,
            vec![
                "top_adds",
                "category_specialists",
                "deployment_risers",
                "risk_discounts",
                "watched_player_alerts"
            ]
        );
    }

    #[test]
    fn l0_watch_default_rules_disclose_unsupported_sources() {
        let context = icelines_core::ViewContext::new(icelines_core::ViewWindow::new(
            icelines_core::Season(20252026),
            icelines_core::season_stats::SeasonType::Regular,
        ));
        let view = default_watch_rules_view(context);

        assert_eq!(view.rules.len(), 5);
        assert!(view
            .rules
            .iter()
            .any(|rule| rule.unsupported_sources.contains(&SourceKind::Shifts)));
        assert!(view.rules.iter().any(|rule| rule
            .unsupported_sources
            .contains(&SourceKind::FantasyImport)));
    }

    #[test]
    fn l0_watch_player_rule_names_player_and_trigger() {
        let rule = player_watch_rule("Matthew Knies", "pp1");

        assert_eq!(rule.id, "player-matthew-knies");
        assert!(rule.label.contains("Matthew Knies"));
        assert!(rule.unsupported_sources.contains(&SourceKind::Shifts));
    }

    #[test]
    fn l0_watch_slug_strips_punctuation() {
        assert_eq!(slug("A. Player Jr."), "a-player-jr");
    }

    #[test]
    fn l0_watchlist_rows_include_notes() {
        let db = GroupDb::open_in_memory().expect("open db");
        db.create_group("Watchlist", "").expect("create watchlist");
        db.add_member_kind("Watchlist", "matthew knies", MemberKind::Player)
            .expect("add player");
        db.upsert_watch_note(
            MemberKind::Player,
            "matthew knies",
            "Poach score 72.0; PP1 promotion",
            "manual",
        )
        .expect("note");

        let rows = watchlist_rows(&db).expect("watchlist rows");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "player");
        assert_eq!(rows[0].key, "matthew knies");
        assert_eq!(
            rows[0].reason.as_deref(),
            Some("Poach score 72.0; PP1 promotion")
        );
        assert_eq!(rows[0].source.as_deref(), Some("manual"));
    }

    #[test]
    fn l0_watchlist_rows_missing_group_is_empty() {
        let db = GroupDb::open_in_memory().expect("open db");

        let rows = watchlist_rows(&db).expect("watchlist rows");

        assert!(rows.is_empty());
    }
}
