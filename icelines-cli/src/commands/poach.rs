use std::path::PathBuf;

use anyhow::{bail, Context};

use icelines_core::{
    model::{Position, Season, TeamAbbr},
    view_model::{
        poach_report_context, AvailabilityState, DeploymentSignal, PoachBoardView, PoachQuery,
        PoachReportSection, PoachReportView, SourceKind, SourceState, ViewContext, ViewWindow,
        WatchRule, WatchRuleTrigger, WatchRulesView,
    },
};

use crate::cli::QuerySeasonType;
use crate::commands::players::{load_repo_for_season, validate_bundled_season};
use crate::config::Config;

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

pub struct WatchRulesArgs {
    pub season: Option<String>,
    pub season_type: QuerySeasonType,
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
    let report = report_from_board(board);
    let body = if args.json {
        serde_json::to_string_pretty(&report).context("serializing poach report")?
    } else {
        render_report_markdown(&report)
    };
    write_or_print(args.out.as_ref(), &body)
}

pub async fn run_watch_rules(args: WatchRulesArgs) -> anyhow::Result<()> {
    let context = watch_context(args.season.as_deref(), args.season_type)?;
    emit_watch_view(&default_watch_rules_view(context), args.json)
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

fn report_from_board(board: PoachBoardView) -> PoachReportView {
    let omissions = board
        .source_state
        .iter()
        .filter(|state| state.state != icelines_core::Completeness::Complete)
        .map(|state| format!("{:?}: {:?}", state.source, state.state).to_ascii_lowercase())
        .collect();

    PoachReportView {
        context: poach_report_context(board.context.clone(), "poach-report"),
        scoring_scheme: board.scoring_scheme,
        window: board.window,
        source_state: board.source_state,
        warnings: board.warnings,
        omissions,
        sections: vec![PoachReportSection {
            id: "top_adds".to_string(),
            title: "Top Adds".to_string(),
            rows: board.rows,
        }],
    }
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

fn default_watch_rules_view(context: ViewContext) -> WatchRulesView {
    WatchRulesView {
        context,
        rules: vec![
            WatchRule {
                id: "category-hits-pace".to_string(),
                label: "Category specialist crosses hits threshold".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::CategoryThreshold {
                    category: "hits".to_string(),
                    threshold: 200.0,
                },
                last_fired: None,
                unsupported_sources: Vec::new(),
            },
            WatchRule {
                id: "category-blocks-pace".to_string(),
                label: "Category specialist crosses blocks threshold".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::CategoryThreshold {
                    category: "blocks".to_string(),
                    threshold: 120.0,
                },
                last_fired: None,
                unsupported_sources: Vec::new(),
            },
            WatchRule {
                id: "deployment-promotion".to_string(),
                label: "Player promotion from deployment signal".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::PlayerPromoted {
                    player_id: None,
                    evidence: DeploymentSignal::Unknown,
                },
                last_fired: None,
                unsupported_sources: vec![SourceKind::Shifts],
            },
            WatchRule {
                id: "goalie-back-to-back".to_string(),
                label: "Goalie back-to-back start candidate".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::GoalieBackToBackStart { team: None },
                last_fired: None,
                unsupported_sources: vec![SourceKind::Schedule],
            },
            WatchRule {
                id: "availability-change".to_string(),
                label: "Watched player becomes available".to_string(),
                enabled: true,
                trigger: WatchRuleTrigger::AvailabilityChanged {
                    player_id: None,
                    state: AvailabilityState::Unknown,
                },
                last_fired: None,
                unsupported_sources: vec![SourceKind::FantasyImport],
            },
        ],
        warnings: Vec::new(),
    }
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
            context: poach_report_context(context, "test-report"),
            scoring_scheme: "yahoo-standard".to_string(),
            window: icelines_core::view_model::PoachWindow::Days14,
            source_state: Vec::new(),
            warnings: Vec::new(),
            omissions: vec!["schedule: unavailable".to_string()],
            sections: vec![PoachReportSection {
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
}
