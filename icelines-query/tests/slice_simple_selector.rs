use serde_json::{json, Value};
use slice_core::{FieldCatalog, ValueType};

#[test]
fn slice_selects_simple_player_bio_and_stat_rows() {
    let rows = vec![
        player_row("847-mock-swe-c", "C", "SWE", 0.82),
        player_row("847-mock-can-d", "D", "CAN", 0.71),
        player_row("847-mock-fin-w", "W", "FIN", 0.91),
    ];

    let selected = select_player_ids(
        &rows,
        "player.position eq 'C' and player.nationality eq 'SWE' and stats.ppg ge 0.8",
    );

    assert_eq!(selected, ["847-mock-swe-c"]);
}

#[test]
fn slice_requirements_stay_low_level_for_icelines_rows() {
    let mut catalog = player_catalog();
    catalog.insert("stats.goals", ValueType::Number);
    let selector =
        slice_core::compile("player.position eq 'W' and stats.goals ge 30", &catalog).unwrap();

    let required = selector
        .requirements()
        .fields
        .iter()
        .map(|field| field.path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(required, ["player.position", "stats.goals"]);
}

fn select_player_ids(rows: &[Value], expr: &str) -> Vec<String> {
    let catalog = player_catalog();
    let selector = slice_core::compile(expr, &catalog).unwrap();
    rows.iter()
        .filter(|row| selector.matches(row))
        .map(|row| row["player"]["id"].as_str().unwrap().to_string())
        .collect()
}

fn player_catalog() -> FieldCatalog {
    let mut catalog = FieldCatalog::new();
    catalog
        .insert("player.position", ValueType::String)
        .insert("player.nationality", ValueType::String)
        .insert("stats.ppg", ValueType::Number);
    catalog
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
