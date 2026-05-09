use std::path::PathBuf;

use anyhow::{bail, Context};

use icelines_core::{
    model::{Position, TeamAbbr},
    view_model::{
        poach_report_context, PoachBoardView, PoachQuery, PoachReportSection, PoachReportView,
    },
};

use crate::cli::QuerySeasonType;
use crate::commands::players::load_repo_for_season;

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
}
