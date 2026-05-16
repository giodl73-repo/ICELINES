//! SLICE helpers for prepared ICELINES rows.
//!
//! This module intentionally works only after ICELINES has already projected
//! domain data into simple player/stat rows. Hockey query parsing, stat aliases,
//! windows, ranking, aggregation, and requirements remain owned by ICELINES.

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
