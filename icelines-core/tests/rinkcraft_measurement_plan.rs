use icelines_core::view_model::rinkcraft_measurement_plan::{build, hash};
use serde_json::{json, Value};
fn inputs() -> (String, Value, Value) {
    let r=json!({"schema":"rinkcraft.attribute-contracts.v1","production_grades_released":false,"attributes":[{"id":"CAP-TEST","name":"Test","branch":"goalie","components":[{"id":"TEST-C"}]}]}).to_string();
    let q = json!({"schema":"rinkcraft.profile-task-measurement-queue.v0.1","authored":"2026-01-01","status":"Authored proposal","parent_attribute_registry":"unopened-path","parent_registry_sha256":hash(&r),"scope":"Separate game states","selection":"Freeze selection","counting":"Unique opportunities","grade_gate":"No grades","reference_policy":"Full cohort","next_step":"Coverage audit","tasks":[{"task_id":"test-task","attribute_id":"CAP-TEST","component_id":"TEST-C","opportunity":"Observed opportunity","start":"Start","end":"End","candidate_measure":"Resolved outcomes / tasks","unit":"probability","denominator_scope":"Tasks, not all shifts","primary_outcomes":["success","technical_failure","choice_failure","opponent_wins","teammate_failure","joint_or_ambiguous","untested","censored","out_of_scope"],"missingness_rule":"Censor incomplete observations","current_gap":"No task denominator","task_envelope_status":"Unresolved","calibration_status":"not fitted","grade_release":false}]});
    let p = json!({"schema":"puck.profile-measurement-priorities.v0.1","date":"2026-01-02","research_queue":"unopened-queue","research_queue_sha256":hash(&q.to_string()),"purpose":"Authored priorities","players":[{"player_id":1,"name":"Synthetic goalie","task_ids":["test-task"]}]});
    (r, q, p)
}
fn run(
    r: &str,
    q: &Value,
    p: &Value,
) -> Result<icelines_core::view_model::rinkcraft_measurement_plan::Catalog, String> {
    let mut p = p.clone();
    p["research_queue_sha256"] = hash(&q.to_string()).into();
    build(r, &q.to_string(), &p.to_string())
}
#[test]
fn copies_authored_order_contract_and_branch_without_scoring() {
    let (r, q, p) = inputs();
    let c = run(&r, &q, &p).unwrap();
    assert_eq!(c.players[0].tasks[0].branch, "goalie");
    assert_eq!(
        c.players[0].tasks[0].definition.denominator_scope,
        "Tasks, not all shifts"
    );
    assert!(!c.players[0].tasks[0].definition.grade_release);
    assert_eq!(c.registry_sha256, hash(&r));
    assert!(!serde_json::to_value(c)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("scores"));
}
#[test]
fn fingerprint_failure_has_no_fallback() {
    let (r, q, mut p) = inputs();
    p["research_queue_sha256"] = "0".repeat(64).into();
    assert!(build(&r, &q.to_string(), &p.to_string()).is_err());
    let mut q = q;
    q["parent_registry_sha256"] = "0".repeat(64).into();
    assert!(run(&r, &q, &p).is_err());
}
#[test]
fn rejects_unknown_task_duplicate_selection_and_wrong_component() {
    let (r, q, p) = inputs();
    let mut bad = p.clone();
    bad["players"][0]["task_ids"] = json!(["unknown"]);
    assert!(run(&r, &q, &bad).is_err());
    bad["players"][0]["task_ids"] = json!(["test-task", "test-task"]);
    assert!(run(&r, &q, &bad).is_err());
    let mut q = q;
    q["tasks"][0]["component_id"] = "elsewhere".into();
    assert!(run(&r, &q, &p).is_err());
}
#[test]
fn rejects_fit_grade_promotion_missing_denominators_and_outcome_relabeling() {
    let (r, q, p) = inputs();
    for (field, value) in [
        ("grade_release", json!(true)),
        ("calibration_status", json!("fitted")),
        ("denominator_scope", json!("")),
        ("primary_outcomes", json!(["success", "failure"])),
    ] {
        let mut bad = q.clone();
        bad["tasks"][0][field] = value;
        assert!(run(&r, &bad, &p).is_err(), "{field}");
    }
}
#[test]
fn rejects_duplicate_players_and_unknown_input_fields() {
    let (r, q, p) = inputs();
    let mut bad = p.clone();
    let player = bad["players"][0].clone();
    bad["players"].as_array_mut().unwrap().push(player);
    assert!(run(&r, &q, &bad).is_err());
    let mut bad = q;
    bad["tasks"][0]["ability_score"] = json!(18);
    assert!(run(&r, &bad, &p).is_err());
}

fn draft_inputs() -> (String, Value, Value) {
    let (r, mut q, mut p) = inputs();
    let mut r: Value = serde_json::from_str(&r).unwrap();
    r["version"] = "0.1.0-draft".into();
    r["observation_policy"] = json!({"failure_states":q["tasks"][0]["primary_outcomes"].clone(),"denominator_accounting":"Eligible includes censored; no invented failures."});
    r["evidence_policy"] = json!({"required_record":["player_id","attribute_id","component_id","definition_version","task_id","opportunity_id","result","visibility","source_lineage_id"]});
    let r = r.to_string();
    q["parent_registry_sha256"] = hash(&r).into();
    p["research_queue_sha256"] = hash(&q.to_string()).into();
    (r, q, p)
}
#[test]
fn worksheet_is_blank_and_preserves_all_required_fields_and_policies() {
    let (r, q, p) = draft_inputs();
    let d = icelines_core::view_model::rinkcraft_measurement_plan::worksheet(
        &r,
        &q.to_string(),
        &p.to_string(),
        1,
    )
    .unwrap();
    assert!(d.observations.is_empty());
    assert!(d.protocol_to_define.values().all(Value::is_null));
    let t = &d.tasks[0].record_template;
    assert_eq!(t.len(), d.required_record_fields.len());
    assert_eq!(t["player_id"], json!(1));
    assert_eq!(t["task_id"], json!("test-task"));
    for key in [
        "opportunity_id",
        "result",
        "visibility",
        "source_lineage_id",
    ] {
        assert!(t[key].is_null());
    }
    assert_eq!(
        d.observation_policy,
        serde_json::from_str::<Value>(&r).unwrap()["observation_policy"]
    );
    assert_eq!(d.definition_version, "0.1.0-draft");
}
#[test]
fn worksheet_never_substitutes_player_or_missing_parent_contract() {
    let (r, q, p) = draft_inputs();
    assert!(
        icelines_core::view_model::rinkcraft_measurement_plan::worksheet(
            &r,
            &q.to_string(),
            &p.to_string(),
            2
        )
        .is_err()
    );
    let mut r: Value = serde_json::from_str(&r).unwrap();
    r["evidence_policy"]["required_record"] = json!(["player_id"]);
    let r = r.to_string();
    let mut q = q;
    q["parent_registry_sha256"] = hash(&r).into();
    let mut p = p;
    p["research_queue_sha256"] = hash(&q.to_string()).into();
    assert!(
        icelines_core::view_model::rinkcraft_measurement_plan::worksheet(
            &r,
            &q.to_string(),
            &p.to_string(),
            1
        )
        .is_err()
    );
}
