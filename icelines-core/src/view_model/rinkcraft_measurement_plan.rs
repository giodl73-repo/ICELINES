//! Source-bound authored measurement plans; no observations, estimates or grades.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct Registry {
    schema: String,
    production_grades_released: bool,
    attributes: Vec<Attribute>,
}
#[derive(Deserialize)]
struct Attribute {
    id: String,
    name: String,
    branch: String,
    components: Vec<Component>,
}
#[derive(Deserialize)]
struct Component {
    id: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Queue {
    schema: String,
    authored: String,
    status: String,
    parent_attribute_registry: String,
    parent_registry_sha256: String,
    scope: String,
    selection: String,
    counting: String,
    grade_gate: String,
    reference_policy: String,
    next_step: String,
    tasks: Vec<Task>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub task_id: String,
    pub attribute_id: String,
    pub component_id: String,
    pub opportunity: String,
    pub start: String,
    pub end: String,
    pub candidate_measure: String,
    pub unit: String,
    pub denominator_scope: String,
    pub primary_outcomes: Vec<String>,
    pub missingness_rule: String,
    pub current_gap: String,
    pub task_envelope_status: String,
    pub calibration_status: String,
    pub grade_release: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Priorities {
    schema: String,
    date: String,
    research_queue: String,
    research_queue_sha256: String,
    purpose: String,
    players: Vec<Priority>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Priority {
    player_id: u32,
    name: String,
    task_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedTask {
    pub attribute_name: String,
    pub branch: String,
    pub definition: Task,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerPlan {
    pub player_id: u32,
    pub name: String,
    pub tasks: Vec<PlannedTask>,
}

#[derive(Debug, Serialize)]
pub struct WorksheetTask {
    pub planned_task: PlannedTask,
    pub record_template: BTreeMap<String, Value>,
}
#[derive(Debug, Serialize)]
pub struct WorksheetDraft {
    pub schema: String,
    pub status: String,
    pub player_id: u32,
    pub player_name: String,
    pub registry_sha256: String,
    pub queue_sha256: String,
    pub priorities_sha256: String,
    pub definition_version: String,
    pub required_record_fields: Vec<String>,
    pub protocol_to_define: BTreeMap<String, Value>,
    pub observation_policy: Value,
    pub evidence_policy: Value,
    pub tasks: Vec<WorksheetTask>,
    pub observations: [Value; 0],
}

/// Blank research scaffold only: this function neither accepts nor validates observations.
pub fn worksheet(
    registry: &str,
    queue: &str,
    priorities: &str,
    player_id: u32,
) -> Result<WorksheetDraft, String> {
    let catalog = build(registry, queue, priorities)?;
    let player = catalog
        .players
        .iter()
        .find(|p| p.player_id == player_id)
        .ok_or("No authored plan for requested player; no fallback")?;
    let r: Value = serde_json::from_str(registry).map_err(|e| e.to_string())?;
    let version = r["version"]
        .as_str()
        .filter(|v| !v.trim().is_empty())
        .ok_or("Missing definition version")?;
    let fields: Vec<String> =
        serde_json::from_value(r["evidence_policy"]["required_record"].clone())
            .map_err(|e| e.to_string())?;
    let unique = fields.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if fields.is_empty()
        || unique.len() != fields.len()
        || fields.iter().any(|s| s.trim().is_empty())
        || ![
            "player_id",
            "attribute_id",
            "component_id",
            "definition_version",
            "task_id",
            "opportunity_id",
            "result",
        ]
        .iter()
        .all(|s| unique.contains(s))
    {
        return Err("Incomplete or duplicated required-record contract".into());
    }
    let outcomes: Vec<String> =
        serde_json::from_value(r["observation_policy"]["failure_states"].clone())
            .map_err(|e| e.to_string())?;
    let expected = player.tasks[0]
        .definition
        .primary_outcomes
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if outcomes.len() != expected.len()
        || outcomes.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected
        || !r["observation_policy"]["denominator_accounting"]
            .as_str()
            .is_some_and(|s| !s.trim().is_empty())
    {
        return Err("Missing or inconsistent parent outcome/accounting policy".into());
    }
    let tasks = player
        .tasks
        .iter()
        .map(|t| {
            let mut record_template = fields
                .iter()
                .map(|f| (f.clone(), Value::Null))
                .collect::<BTreeMap<_, _>>();
            for (key, value) in [
                ("player_id", Value::from(player_id)),
                (
                    "attribute_id",
                    Value::from(t.definition.attribute_id.clone()),
                ),
                (
                    "component_id",
                    Value::from(t.definition.component_id.clone()),
                ),
                ("definition_version", Value::from(version)),
                ("task_id", Value::from(t.definition.task_id.clone())),
            ] {
                record_template.insert(key.into(), value);
            }
            WorksheetTask {
                planned_task: t.clone(),
                record_template,
            }
        })
        .collect();
    Ok(WorksheetDraft {
        schema: "icelines.measurement-worksheet-draft.v1".into(),
        status: "Unfilled research draft; not observations or an approved sampling protocol".into(),
        player_id,
        player_name: player.name.clone(),
        registry_sha256: catalog.registry_sha256,
        queue_sha256: catalog.queue_sha256,
        priorities_sha256: catalog.priorities_sha256,
        definition_version: version.into(),
        required_record_fields: fields,
        protocol_to_define: [
            "sampling_protocol_id",
            "selection_rule",
            "planned_window",
            "game_ids",
            "source_access_terms",
            "assessor_id",
            "coding_manual_version",
            "task_envelope_version",
            "review_status",
        ]
        .into_iter()
        .map(|s| (s.into(), Value::Null))
        .collect(),
        observation_policy: r["observation_policy"].clone(),
        evidence_policy: r["evidence_policy"].clone(),
        tasks,
        observations: [],
    })
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub schema: String,
    pub queue_authored: String,
    pub priorities_authored: String,
    pub status: String,
    pub purpose: String,
    pub scope: String,
    pub selection: String,
    pub counting: String,
    pub reference_policy: String,
    pub grade_gate: String,
    pub next_step: String,
    pub registry_sha256: String,
    pub queue_sha256: String,
    pub priorities_sha256: String,
    pub declared_registry: String,
    pub declared_queue: String,
    pub players: Vec<PlayerPlan>,
}
pub fn hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}
fn text(s: &str) -> bool {
    !s.trim().is_empty()
}

pub fn build(registry: &str, queue: &str, priorities: &str) -> Result<Catalog, String> {
    if [registry, queue, priorities]
        .iter()
        .any(|s| s.len() > 1024 * 1024)
    {
        return Err("Plan input exceeds1MiB".into());
    }
    let r: Registry = serde_json::from_str(registry).map_err(|e| e.to_string())?;
    let q: Queue = serde_json::from_str(queue).map_err(|e| e.to_string())?;
    let p: Priorities = serde_json::from_str(priorities).map_err(|e| e.to_string())?;
    if r.schema != "rinkcraft.attribute-contracts.v1"
        || r.production_grades_released
        || q.schema != "rinkcraft.profile-task-measurement-queue.v0.1"
        || p.schema != "puck.profile-measurement-priorities.v0.1"
    {
        return Err("Unsupported planning contract or released-grade registry".into());
    }
    let registry_sha256 = hash(registry);
    let queue_sha256 = hash(queue);
    if q.parent_registry_sha256 != registry_sha256 || p.research_queue_sha256 != queue_sha256 {
        return Err("Plan input fingerprint mismatch; no fallback or string-based matching".into());
    }
    if q.tasks.is_empty()
        || q.tasks.len() > 200
        || p.players.is_empty()
        || p.players.len() > 5000
        || ![
            &q.authored,
            &q.status,
            &q.scope,
            &q.selection,
            &q.counting,
            &q.grade_gate,
            &q.reference_policy,
            &q.next_step,
            &p.date,
            &p.purpose,
        ]
        .into_iter()
        .all(|s| text(s))
    {
        return Err("Missing planning scope, policy or bounded tasks/players".into());
    }
    let mut attributes = BTreeMap::new();
    let mut components = BTreeSet::new();
    for a in &r.attributes {
        if !text(&a.id)
            || !text(&a.name)
            || !matches!(a.branch.as_str(), "skater" | "goalie")
            || attributes.insert(a.id.as_str(), a).is_some()
        {
            return Err("Invalid or duplicate registry attribute".into());
        }
        for c in &a.components {
            if !text(&c.id) || !components.insert(c.id.as_str()) {
                return Err("Invalid or duplicate component".into());
            }
        }
    }
    let mut tasks = BTreeMap::new();
    let required = [
        "success",
        "technical_failure",
        "choice_failure",
        "opponent_wins",
        "teammate_failure",
        "joint_or_ambiguous",
        "untested",
        "censored",
        "out_of_scope",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    for t in &q.tasks {
        let Some(a) = attributes.get(t.attribute_id.as_str()) else {
            return Err("Unknown task attribute".into());
        };
        if !a.components.iter().any(|c| c.id == t.component_id) {
            return Err("Component does not belong to task attribute".into());
        }
        if t.grade_release
            || t.calibration_status != "not fitted"
            || ![
                &t.task_id,
                &t.opportunity,
                &t.start,
                &t.end,
                &t.candidate_measure,
                &t.unit,
                &t.denominator_scope,
                &t.missingness_rule,
                &t.current_gap,
                &t.task_envelope_status,
            ]
            .into_iter()
            .all(|s| text(s))
        {
            return Err("Task is missing a contract or attempts fitted/grade promotion".into());
        }
        if t.primary_outcomes.len() != required.len()
            || t.primary_outcomes
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
                != required
        {
            return Err(
                "Task outcome taxonomy differs; censored/ambiguous/untested cannot become failures"
                    .into(),
            );
        }
        if tasks.insert(t.task_id.as_str(), t).is_some() {
            return Err("Duplicate task ID".into());
        }
    }
    let mut ids = BTreeSet::new();
    let mut players = Vec::new();
    for player in &p.players {
        if player.player_id == 0
            || !text(&player.name)
            || !ids.insert(player.player_id)
            || player.task_ids.is_empty()
        {
            return Err("Invalid/duplicate player or empty authored task selection".into());
        }
        let mut seen = BTreeSet::new();
        let mut selected = Vec::new();
        for id in &player.task_ids {
            let Some(t) = tasks.get(id.as_str()) else {
                return Err(format!("Unknown authored task: {id}"));
            };
            if !seen.insert(id) {
                return Err("Duplicate player task".into());
            }
            let a = attributes[t.attribute_id.as_str()];
            selected.push(PlannedTask {
                attribute_name: a.name.clone(),
                branch: a.branch.clone(),
                definition: (**t).clone(),
            });
        }
        players.push(PlayerPlan {
            player_id: player.player_id,
            name: player.name.clone(),
            tasks: selected,
        });
    }
    Ok(Catalog {
        schema: "icelines.measurement-plan-catalog.v1".into(),
        queue_authored: q.authored,
        priorities_authored: p.date,
        status: q.status,
        purpose: p.purpose,
        scope: q.scope,
        selection: q.selection,
        counting: q.counting,
        reference_policy: q.reference_policy,
        grade_gate: q.grade_gate,
        next_step: q.next_step,
        registry_sha256,
        queue_sha256,
        priorities_sha256: hash(priorities),
        declared_registry: q.parent_attribute_registry,
        declared_queue: p.research_queue,
        players,
    })
}
