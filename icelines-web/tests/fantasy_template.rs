use askama::Template;
use icelines_web::templates::{FantasySimulationScenarioRow, FantasyTemplate};

#[test]
fn fantasy_template_renders_simulation_scenarios_and_preserves_inputs() {
    let template = FantasyTemplate {
        active_label: "2025-26 Regular".to_string(),
        league: "Main".to_string(),
        team: "Mine".to_string(),
        scoring_scheme: "yahoo-standard".to_string(),
        categories: "hits,blocks".to_string(),
        add_player: "Connor McDavid".to_string(),
        drop_player: "Bench Forward".to_string(),
        rows: Vec::new(),
        simulation_rows: Vec::new(),
        simulation_scenarios: vec![FantasySimulationScenarioRow {
            action: "improve".to_string(),
            label: "Web add/drop scenario".to_string(),
            add_player: "Connor McDavid".to_string(),
            drop_player: "Bench Forward".to_string(),
            projected_score_delta: "+12.5".to_string(),
            projected_games_delta: 2,
            confidence: "low".to_string(),
            explanation: "Connor McDavid for Bench Forward improves projected score by 12.5."
                .to_string(),
        }],
        simulation_assumptions: Vec::new(),
        simulation_warnings: vec!["schedule unavailable".to_string()],
        warnings: Vec::new(),
        empty_title: "No roster gaps found".to_string(),
        empty_detail: String::new(),
    };

    let html = template.render().expect("fantasy template renders");

    assert!(html.contains("Web add/drop scenario"));
    assert!(html.contains("+12.5"));
    assert!(html.contains("value=\"Connor McDavid\""));
    assert!(html.contains("value=\"Bench Forward\""));
    assert!(html.contains("class=\"state-warning\""));
    assert!(html.contains("class=\"state-warning-line\""));
    assert!(!html.contains("color: #8a5a00"));
}
