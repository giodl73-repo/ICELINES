use anyhow::{bail, Context};

use icelines_core::{
    model::{Position, TeamAbbr},
    view_model::{PoachBoardView, PoachQuery},
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

pub async fn run(args: PoachArgs) -> anyhow::Result<()> {
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

    let view = PoachBoardView::from_repository(&outcome.repo, query);
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&view).context("serializing poach board")?
        );
    } else {
        print_table(&view);
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
}
