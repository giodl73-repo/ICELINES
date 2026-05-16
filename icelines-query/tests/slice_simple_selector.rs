use icelines_query::{
    compile_prepared_player_selector, parse_query, plan_prepared_player_query_sqlite_selector,
    plan_prepared_player_sqlite_selector, prepared_player_slice_expr_for_query_plan,
    select_prepared_player_rows, FilterInput,
};
use serde_json::{json, Value};

#[test]
fn slice_selects_simple_player_bio_and_stat_rows() {
    let rows = vec![
        player_row("847-mock-swe-c", "C", "SWE", 0.82),
        player_row("847-mock-can-d", "D", "CAN", 0.71),
        player_row("847-mock-fin-w", "W", "FIN", 0.91),
    ];

    let selected = select_prepared_player_rows(
        &rows,
        "player.position eq 'C' and player.nationality eq 'SWE' and stats.ppg ge 0.8",
    )
    .unwrap();
    let selected = selected
        .iter()
        .map(|row| row["player"]["id"].as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(selected, ["847-mock-swe-c"]);
}

#[test]
fn slice_requirements_stay_low_level_for_icelines_rows() {
    let selector =
        compile_prepared_player_selector("player.position eq 'W' and stats.goals ge 30").unwrap();

    let required = selector
        .requirements()
        .fields
        .iter()
        .map(|field| field.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(required, ["player.position", "stats.goals"]);
}

#[test]
fn slice_plans_sqlite_predicates_for_icelines_owned_joins() {
    let plan = plan_prepared_player_sqlite_selector(
        "player.position eq 'C' and stats.ppg ge 0.8 and stats.goals between 20 and 40",
    )
    .unwrap();

    assert_eq!(plan.backend, "sqlite");
    assert_eq!(plan.sources[0].source, "players");
    assert_eq!(
        plan.sources[0].predicate.text,
        "((\"players\".\"position\" = ?))"
    );
    assert_eq!(plan.sources[1].source, "stats");
    assert_eq!(
        plan.sources[1].predicate.text,
        "((\"stats\".\"ppg\" >= ?) AND (\"stats\".\"goals\" BETWEEN ? AND ?))"
    );
    assert!(plan.residual.is_none());
}

#[test]
fn slice_can_plan_simple_icelines_query_ir_for_prepared_rows() {
    let query = parse_query(FilterInput::Cli(
        "pos=C AND nationality=SWE AND ppg>=0.8".to_string(),
    ))
    .unwrap();
    let expr = prepared_player_slice_expr_for_query_plan(&query).unwrap();
    let plan = plan_prepared_player_query_sqlite_selector(&query)
        .unwrap()
        .unwrap();

    assert_eq!(
        expr,
        "(player.position eq 'c') and (player.nationality eq 'swe') and (stats.ppg ge 0.8)"
    );
    assert_eq!(plan.source_count, 2);
    assert_eq!(
        plan.sources[0].predicate.text,
        "((\"players\".\"position\" = ?) AND (\"players\".\"nationality\" = ?))"
    );
}

fn player_row(id: &str, position: &str, nationality: &str, ppg: f64) -> Value {
    json!({
        "player": {
            "id": id,
            "position": position,
            "nationality": nationality,
        },
        "stats": {
            "ppg": ppg,
        },
    })
}
