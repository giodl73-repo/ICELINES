use icelines_query::{
    compile_prepared_player_selector, plan_prepared_player_sqlite_selector,
    select_prepared_player_rows,
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
