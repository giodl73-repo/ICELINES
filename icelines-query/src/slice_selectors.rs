//! SLICE helpers for prepared ICELINES rows.
//!
//! This module intentionally works only after ICELINES has already projected
//! domain data into simple player/stat rows. Hockey query parsing, stat aliases,
//! windows, ranking, aggregation, and requirements remain owned by ICELINES.

use crate::plan::{
    BioField, Constraint, MemberOp, PatternOp, Predicate, QueryPlan, ScalarOp, ScalarValue,
    SeasonAxis,
};
use serde_json::Value;
use slice_core::{CompiledExpr, FieldCatalog, FoldCatalog, FoldPlan, SliceError, ValueType};

pub fn select_prepared_player_rows<'a>(
    rows: &'a [Value],
    expr: &str,
) -> Result<Vec<&'a Value>, SliceError> {
    let selector = compile_prepared_player_selector(expr)?;
    Ok(rows.iter().filter(|row| selector.matches(row)).collect())
}

pub fn compile_prepared_player_selector(expr: &str) -> Result<CompiledExpr, SliceError> {
    slice_core::compile(expr, &prepared_player_selector_catalog())
}

pub fn plan_prepared_player_sqlite_selector(expr: &str) -> Result<FoldPlan, SliceError> {
    slice_core::parse(expr)?.plan_sqlite(&prepared_player_sqlite_fold_catalog())
}

pub fn prepared_player_slice_expr_for_query_plan(plan: &QueryPlan) -> Option<String> {
    constraint_to_slice_expr(&plan.root)
}

pub fn plan_prepared_player_query_sqlite_selector(
    plan: &QueryPlan,
) -> Result<Option<FoldPlan>, SliceError> {
    let Some(expr) = prepared_player_slice_expr_for_query_plan(plan) else {
        return Ok(None);
    };
    plan_prepared_player_sqlite_selector(&expr).map(Some)
}

pub fn prepared_player_selector_catalog() -> FieldCatalog {
    let mut catalog = FieldCatalog::new();
    catalog
        .insert("player.id", ValueType::String)
        .insert("player.position", ValueType::String)
        .insert("player.nationality", ValueType::String)
        .insert("player.shoots", ValueType::String)
        .insert("player.age", ValueType::Number)
        .insert("player.draft_year", ValueType::Number)
        .insert("player.height", ValueType::Number)
        .insert("player.weight", ValueType::Number)
        .insert("stats.games", ValueType::Number)
        .insert("stats.goals", ValueType::Number)
        .insert("stats.assists", ValueType::Number)
        .insert("stats.points", ValueType::Number)
        .insert("stats.ppg", ValueType::Number)
        .insert("stats.wins", ValueType::Number)
        .insert("stats.saves", ValueType::Number)
        .insert("stats.save_pct", ValueType::Number);
    catalog
}

fn constraint_to_slice_expr(constraint: &Constraint) -> Option<String> {
    match constraint {
        Constraint::Bio(bio) => {
            let path = bio_field_to_slice_path(bio.field)?;
            predicate_to_slice_expr(path, &bio.predicate)
        }
        Constraint::SeasonStat(stat) => {
            if stat.axis != SeasonAxis::Regular {
                return None;
            }
            let path = season_stat_to_slice_path(stat.stat.cli_key())?;
            predicate_to_slice_expr(path, &stat.predicate)
        }
        Constraint::All(children) => join_slice_children("and", children),
        Constraint::Any(children) => join_slice_children("or", children),
        Constraint::Not(inner) => {
            constraint_to_slice_expr(inner).map(|expr| format!("not ({expr})"))
        }
        Constraint::SlidingWindow(_)
        | Constraint::CareerAggregate(_)
        | Constraint::CareerLeague(_) => None,
    }
}

fn join_slice_children(operator: &str, children: &[Constraint]) -> Option<String> {
    let expressions = children
        .iter()
        .map(constraint_to_slice_expr)
        .collect::<Option<Vec<_>>>()?;
    Some(
        expressions
            .into_iter()
            .map(|expr| format!("({expr})"))
            .collect::<Vec<_>>()
            .join(&format!(" {operator} ")),
    )
}

fn bio_field_to_slice_path(field: BioField) -> Option<&'static str> {
    match field {
        BioField::Age => Some("player.age"),
        BioField::DraftYear => Some("player.draft_year"),
        BioField::Height => Some("player.height"),
        BioField::Weight => Some("player.weight"),
        BioField::Nationality => Some("player.nationality"),
        BioField::Shoots => Some("player.shoots"),
        BioField::Position => Some("player.position"),
        _ => None,
    }
}

fn season_stat_to_slice_path(stat_key: &str) -> Option<&'static str> {
    match stat_key {
        "games" | "gp" => Some("stats.games"),
        "goals" | "g" => Some("stats.goals"),
        "assists" | "a" => Some("stats.assists"),
        "points" | "p" => Some("stats.points"),
        "points-per-game" | "ppg" => Some("stats.ppg"),
        "wins" | "w" => Some("stats.wins"),
        "saves" | "sv" => Some("stats.saves"),
        "save-pct" | "save_pct" | "svpct" => Some("stats.save_pct"),
        _ => None,
    }
}

fn predicate_to_slice_expr(path: &str, predicate: &Predicate) -> Option<String> {
    match predicate {
        Predicate::Scalar(operator, value) => Some(format!(
            "{path} {} {}",
            scalar_op_to_slice(*operator),
            scalar_value_to_slice(value)
        )),
        Predicate::Member(operator, values) => Some(format!(
            "{path} {} [{}]",
            member_op_to_slice(*operator),
            values
                .iter()
                .map(scalar_value_to_slice)
                .collect::<Vec<_>>()
                .join(", ")
        )),
        Predicate::Pattern(operator, pattern) => {
            pattern_to_slice_expr(path, *operator, pattern.raw.as_str())
        }
        Predicate::Range(range) => Some(format!("{path} between {} and {}", range.min, range.max)),
    }
}

fn scalar_op_to_slice(operator: ScalarOp) -> &'static str {
    match operator {
        ScalarOp::Eq => "eq",
        ScalarOp::Ne => "ne",
        ScalarOp::Lt => "lt",
        ScalarOp::Le => "le",
        ScalarOp::Gt => "gt",
        ScalarOp::Ge => "ge",
    }
}

fn member_op_to_slice(operator: MemberOp) -> &'static str {
    match operator {
        MemberOp::In => "in",
        MemberOp::NotIn => "not in",
    }
}

fn scalar_value_to_slice(value: &ScalarValue) -> String {
    match value {
        ScalarValue::Number(value) => value.to_string(),
        ScalarValue::Text(value) => quote_slice_string(value),
    }
}

fn pattern_to_slice_expr(path: &str, operator: PatternOp, raw: &str) -> Option<String> {
    let (slice_operator, literal) = simple_glob_to_slice_operator(raw)?;
    let expression = format!("{path} {slice_operator} {}", quote_slice_string(literal));
    match operator {
        PatternOp::Like | PatternOp::Contains => Some(expression),
        PatternOp::NotLike | PatternOp::NotContains => Some(format!("not ({expression})")),
    }
}

fn simple_glob_to_slice_operator(raw: &str) -> Option<(&'static str, &str)> {
    let starts = raw.starts_with('*');
    let ends = raw.ends_with('*');
    let inner = raw.trim_matches('*');
    if inner.is_empty() || inner.contains('*') {
        return None;
    }
    match (starts, ends) {
        (true, true) => Some(("contains", inner)),
        (true, false) => Some(("ends_with", inner)),
        (false, true) => Some(("starts_with", inner)),
        (false, false) => Some(("eq", inner)),
    }
}

fn quote_slice_string(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

pub fn prepared_player_sqlite_fold_catalog() -> FoldCatalog {
    let mut catalog = FoldCatalog::new();
    catalog
        .insert_sqlite("player.id", ValueType::String, "players", "players.id")
        .insert_sqlite(
            "player.position",
            ValueType::String,
            "players",
            "players.position",
        )
        .insert_sqlite(
            "player.nationality",
            ValueType::String,
            "players",
            "players.nationality",
        )
        .insert_sqlite(
            "player.shoots",
            ValueType::String,
            "players",
            "players.shoots",
        )
        .insert_sqlite("player.age", ValueType::Number, "players", "players.age")
        .insert_sqlite(
            "player.draft_year",
            ValueType::Number,
            "players",
            "players.draft_year",
        )
        .insert_sqlite(
            "player.height",
            ValueType::Number,
            "players",
            "players.height",
        )
        .insert_sqlite(
            "player.weight",
            ValueType::Number,
            "players",
            "players.weight",
        )
        .insert_sqlite("stats.games", ValueType::Number, "stats", "stats.games")
        .insert_sqlite("stats.goals", ValueType::Number, "stats", "stats.goals")
        .insert_sqlite("stats.assists", ValueType::Number, "stats", "stats.assists")
        .insert_sqlite("stats.points", ValueType::Number, "stats", "stats.points")
        .insert_sqlite("stats.ppg", ValueType::Number, "stats", "stats.ppg")
        .insert_sqlite("stats.wins", ValueType::Number, "stats", "stats.wins")
        .insert_sqlite("stats.saves", ValueType::Number, "stats", "stats.saves")
        .insert_sqlite(
            "stats.save_pct",
            ValueType::Number,
            "stats",
            "stats.save_pct",
        );
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_query, FilterInput};
    use serde_json::json;

    #[test]
    fn selects_prepared_player_rows_without_hockey_query_semantics() {
        let rows = vec![
            player_row("847-mock-swe-c", "C", "SWE", 0.82, 31.0),
            player_row("847-mock-can-d", "D", "CAN", 0.71, 20.0),
            player_row("847-mock-fin-w", "W", "FIN", 0.91, 34.0),
        ];

        let selected = select_prepared_player_rows(
            &rows,
            "player.position eq 'C' and player.nationality eq 'SWE' and stats.ppg ge 0.8",
        )
        .unwrap();
        let ids = selected
            .iter()
            .map(|row| row["player"]["id"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(ids, ["847-mock-swe-c"]);
    }

    #[test]
    fn reports_low_level_requirements_for_prepared_rows() {
        let selector =
            compile_prepared_player_selector("player.position eq 'W' and stats.goals ge 30")
                .unwrap();
        let required = selector
            .requirements()
            .fields
            .iter()
            .map(|field| field.path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(required, ["player.position", "stats.goals"]);
    }

    #[test]
    fn plans_sqlite_fold_for_prepared_player_rows_without_owning_joins() {
        let plan = plan_prepared_player_sqlite_selector(
            "player.position eq 'C' and player.nationality eq 'SWE' and stats.ppg ge 0.8",
        )
        .unwrap();

        assert_eq!(plan.backend, "sqlite");
        assert_eq!(plan.source_count, 2);
        assert_eq!(plan.sources[0].source, "players");
        assert_eq!(
            plan.sources[0].predicate.text,
            "((\"players\".\"position\" = ?) AND (\"players\".\"nationality\" = ?))"
        );
        assert_eq!(plan.sources[1].source, "stats");
        assert_eq!(plan.sources[1].predicate.text, "((\"stats\".\"ppg\" >= ?))");
        assert!(plan.residual.is_none());
    }

    #[test]
    fn lowers_simple_query_plan_to_slice_prepared_row_expression() {
        let query = parse_query(FilterInput::Cli(
            "pos=C AND nationality=SWE AND ppg>=0.8".to_string(),
        ))
        .unwrap();
        let expr = prepared_player_slice_expr_for_query_plan(&query).unwrap();

        assert_eq!(
            expr,
            "(player.position eq 'c') and (player.nationality eq 'swe') and (stats.ppg ge 0.8)"
        );
        let plan = plan_prepared_player_query_sqlite_selector(&query)
            .unwrap()
            .unwrap();
        assert_eq!(plan.source_count, 2);
        assert_eq!(
            plan.sources[0].predicate.text,
            "((\"players\".\"position\" = ?) AND (\"players\".\"nationality\" = ?))"
        );
        assert_eq!(plan.sources[1].predicate.text, "((\"stats\".\"ppg\" >= ?))");
    }

    #[test]
    fn leaves_domain_or_unsupported_query_plan_shapes_in_icelines() {
        let query = parse_query(FilterInput::Cli("country=CAN AND ppg>=0.8".to_string())).unwrap();

        assert!(prepared_player_slice_expr_for_query_plan(&query).is_none());
        assert!(plan_prepared_player_query_sqlite_selector(&query)
            .unwrap()
            .is_none());
    }

    fn player_row(id: &str, position: &str, nationality: &str, ppg: f64, goals: f64) -> Value {
        json!({
            "player": {
                "id": id,
                "position": position,
                "nationality": nationality,
            },
            "stats": {
                "ppg": ppg,
                "goals": goals,
            },
        })
    }
}
