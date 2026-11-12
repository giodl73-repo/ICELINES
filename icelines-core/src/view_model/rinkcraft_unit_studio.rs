//! Shared native/WASM studio commands. Browser code never authors dice or placement legality.
use super::{rinkcraft_game as game, rinkcraft_possession::Action, rinkcraft_units as units};
use super::rinkcraft_goalie_readiness as readiness;
use super::rinkcraft_readiness_followup as followup;
use serde::{Deserialize, Serialize};
pub const MAX_COMMAND_BYTES: usize = 768 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub schema: String,
    pub command: Command,
    pub sheet_character: Option<game::GameCharacter>,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Create {
        character_rules: game::GameRules,
        rules: units::Rules,
        blueprint: units::Blueprint,
    },
    Import {
        replay: serde_json::Value,
    },
    FreezeLegacy {
        replay: serde_json::Value,
    },
    StartReceiving {
        replay: serde_json::Value,
    },
    StartShooting {
        replay: serde_json::Value,
    },
    AssessReadiness {
        replay: serde_json::Value,
        situation: readiness::Situation,
        seed: u32,
        lateral_feed_declared: bool,
    },
    ImportReadiness { report: serde_json::Value },
    StartReadinessFollowup { report: serde_json::Value, seed:u32 },
    StepReadinessFollowup { followup:serde_json::Value, action:followup::Action },
    ImportReadinessFollowup { followup:serde_json::Value },
    Reset {
        replay: serde_json::Value,
        assignments: units::Assignments,
        hero_id: String,
        seed: u32,
        commit_support: bool,
    },
    SelectHero {
        replay: serde_json::Value,
        hero_id: String,
    },
    PlaceCharacter {
        replay: serde_json::Value,
        slot: units::Slot,
    },
    Step {
        replay: serde_json::Value,
        action: Action,
    },
    Auto {
        replay: serde_json::Value,
    },
}
#[derive(Serialize)]
pub struct MemberView {
    pub id: String,
    pub slot: units::Slot,
    pub duties: Vec<String>,
}
#[derive(Serialize)]
pub struct Layout {
    pub home_members: Vec<MemberView>,
    pub skater_slots: Vec<units::Slot>,
    pub compatible_sheet_slots: Vec<units::Slot>,
    pub placement_note: String,
}
#[derive(Serialize)]
pub struct Response {
    pub schema: String,
    pub replay: units::Replay,
    pub layout: Layout,
    pub message: String,
    #[serde(skip_serializing_if="Option::is_none")]
    pub goalie_readiness: Option<readiness::Report>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub goalie_opportunities: Option<Vec<readiness::Opportunity>>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub readiness_followup:Option<followup::Replay>,
}
fn imported(value: serde_json::Value) -> Result<units::Input, String> {
    Ok(units::import(&value.to_string())?.input)
}
fn layout(input: &units::Input, sheet: Option<&game::GameCharacter>) -> Layout {
    let mut members: Vec<_> = input
        .request
        .members
        .iter()
        .filter(|m| m.side == units::Side::Home)
        .collect();
    members.sort_by_key(|m| m.slot);
    let a = &input.request.assignments;
    let home_members = members
        .into_iter()
        .map(|m| {
            let mut duties = Vec::new();
            if m.slot == units::Slot::G {
                duties.push("Goalie outlet".into())
            }
            if m.slot == a.carrier {
                duties.push("Carry / recover / pass".into())
            }
            if m.slot == a.receiver {
                duties.push(if input.rules.version == "0.4.0-playtest" {
                    "Receive / shoot / rebound pressure"
                } else { "Receive" }.into())
            }
            if m.slot == a.high_cover {
                duties.push(
                    if input.request.commit_support {
                        "Attack support"
                    } else {
                        "High cover"
                    }
                    .into(),
                )
            }
            if m.slot == a.counter_defender {
                duties.push("Counter defense".into())
            }
            MemberView {
                id: m.id.clone(),
                slot: m.slot,
                duties,
            }
        })
        .collect();
    let compatible = sheet.filter(|c| {
        game::import_from_json(
            &serde_json::to_string(&input.character_rules).unwrap_or_default(),
            &serde_json::to_string(c).unwrap_or_default(),
        )
        .is_ok()
    });
    let compatible_sheet_slots = compatible
        .map(|c| {
            units::Slot::ALL
                .into_iter()
                .filter(|slot| slot.accepts(c))
                .collect()
        })
        .unwrap_or_default();
    Layout {home_members,skater_slots:units::Slot::ALL.into_iter().filter(|s|*s!=units::Slot::G).collect(),compatible_sheet_slots,placement_note:if compatible.is_some(){"Placement starts a new scene with the sheet character; duties stay assigned to their addresses."}else{"The sheet character is unavailable or was created under different rules. The saved unit remains playable."}.into()}
}
pub fn execute(envelope: Envelope) -> Result<Response, String> {
    if envelope.schema != "rinkcraft.unit-studio-command.v1" {
        return Err("Unsupported unit studio command schema".into());
    }
    let mut goalie_readiness = None;
    let mut readiness_followup = None;
    let (input, message) = match envelope.command {
        Command::StartReadinessFollowup {report,seed} => {
            let file=followup::create(readiness::import(&report.to_string())?,seed)?;
            let input=file.input.report.input.replay.input.clone();
            goalie_readiness=Some(file.input.report.clone());readiness_followup=Some(file);
            (input,"Readiness follow-up started as a separate branch; parent scene unchanged.")
        }
        Command::StepReadinessFollowup {followup,action} => {
            let file=followup::step(followup::import(&followup.to_string())?,action)?;
            let input=file.input.report.input.replay.input.clone();
            goalie_readiness=Some(file.input.report.clone());readiness_followup=Some(file);
            (input,"Readiness follow-up action resolved in Rust; parent scene unchanged.")
        }
        Command::ImportReadinessFollowup {followup} => {
            let file=followup::import(&followup.to_string())?;
            let input=file.input.report.input.replay.input.clone();
            goalie_readiness=Some(file.input.report.clone());readiness_followup=Some(file);
            (input,"Follow-up verified; its parent unit and readiness report loaded.")
        }
        Command::AssessReadiness { replay, situation, seed, lateral_feed_declared } => {
            let input=imported(replay)?;
            goalie_readiness=Some(readiness::export(readiness::Input {
                schema:"rinkcraft.goalie-readiness-input.v1".into(),rules:readiness::Rules::default(),replay:units::export(input.clone())?,
                situation,seed,lateral_feed_declared,
            })?);
            (input,"Goalie readiness assessed; source unit scene unchanged.")
        }
        Command::ImportReadiness { report } => {
            let report=readiness::import(&report.to_string())?;
            let input=report.input.replay.input.clone();
            goalie_readiness=Some(report);
            (input,"Readiness report verified; its source unit snapshot loaded.")
        }
        Command::Create {
            character_rules,
            rules,
            blueprint,
        } => (
            units::build_input(character_rules, rules, blueprint)?,
            "Authored unit scene loaded.",
        ),
        Command::Import { replay } => (
            imported(replay)?,
            "Unit replay imported and recomputed in Rust.",
        ),
        Command::FreezeLegacy { replay } => (
            units::freeze_legacy(&replay.to_string())?,
            "Legacy replay copied into explicit units; original file unchanged.",
        ),
        Command::StartReceiving { replay } => {
            let mut input = imported(replay)?;
            input.rules.version = "0.3.0-playtest".into();
            input.request.actions.clear();
            (
                input,
                "New receiving scene started; units and duties retained.",
            )
        }
        Command::StartShooting { replay } => {
            let mut input = imported(replay)?;
            input.rules.version = "0.4.0-playtest".into();
            input.request.actions.clear();
            (input, "New shooting scene started; units and duties retained.")
        }
        Command::Reset {
            replay,
            assignments,
            hero_id,
            seed,
            commit_support,
        } => {
            let mut input = imported(replay)?;
            let q = &mut input.request;
            q.assignments = assignments;
            q.hero_id = hero_id;
            q.seed = seed;
            q.commit_support = commit_support;
            q.actions.clear();
            (input, "New unit scene started.")
        }
        Command::SelectHero { replay, hero_id } => {
            let mut input = imported(replay)?;
            input.request.hero_id = hero_id;
            (
                input,
                "Scene hero selected; duties and recorded events unchanged.",
            )
        }
        Command::PlaceCharacter { replay, slot } => {
            let mut input = imported(replay)?;
            let character = envelope
                .sheet_character
                .as_ref()
                .ok_or("No sheet character supplied")?;
            if !slot.accepts(character) {
                return Err(
                    "Sheet character has an incompatible position or branch for this address"
                        .into(),
                );
            }
            game::import_from_json(
                &serde_json::to_string(&input.character_rules).map_err(|e| e.to_string())?,
                &serde_json::to_string(character).map_err(|e| e.to_string())?,
            )?;
            let member = input
                .request
                .members
                .iter_mut()
                .find(|m| m.side == units::Side::Home && m.slot == slot)
                .ok_or("Missing placement address")?;
            member.character = character.clone();
            input.request.hero_id = member.id.clone();
            input.request.actions.clear();
            (
                input,
                "Sheet character placed; new scene started with duties unchanged.",
            )
        }
        Command::Step { replay, action } => {
            let mut input = imported(replay)?;
            input.request.actions.push(action);
            (input, "Unit action replayed in Rust.")
        }
        Command::Auto { replay } => {
            let mut input = imported(replay)?;
            units::complete(&mut input)?;
            (
                input,
                "Unit scene completed under the sole-legal-action policy.",
            )
        }
    };
    let replay = units::export(input)?;
    let goalie_opportunities=if replay.input.rules.version=="0.4.0-playtest" {
        Some(readiness::opportunities(&replay.input,units::replay(&replay.input)?.scene.state.phase))
    } else {None};
    let layout = layout(&replay.input, envelope.sheet_character.as_ref());
    Ok(Response {
        schema: "icelines.unit-studio-result.v1".into(),
        replay,
        layout,
        message: message.into(),
        goalie_readiness,
        goalie_opportunities,
        readiness_followup,
    })
}
pub fn from_json(raw: &str) -> Result<String, String> {
    if raw.len() > MAX_COMMAND_BYTES {
        return Err("Unit studio command exceeds 768 KiB".into());
    }
    let envelope =
        serde_json::from_str(raw).map_err(|e| format!("Invalid unit studio command: {e}"))?;
    serde_json::to_string(&execute(envelope)?).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn followup_commands_branch_from_verified_report_and_preserve_parent() {
        let report:serde_json::Value=serde_json::from_str(include_str!("../../tests/fixtures/rinkcraft-readiness-v01.json")).unwrap();
        let start=command(serde_json::json!({"kind":"start_readiness_followup","report":report,"seed":43}),None);
        assert_eq!(start["replay"],report["input"]["replay"]);assert_eq!(start["goalie_readiness"],report);
        let next=command(serde_json::json!({"kind":"step_readiness_followup","followup":start["readiness_followup"],"action":"shoot"}),None);
        assert_eq!(next["replay"],start["replay"]);assert_eq!(next["goalie_readiness"],report);
        assert_eq!(next["readiness_followup"]["session"]["legal_actions"],serde_json::json!(["track_save","scramble_save"]));
        let imported=command(serde_json::json!({"kind":"import_readiness_followup","followup":next["readiness_followup"]}),None);
        assert_eq!(imported["readiness_followup"],next["readiness_followup"]);
        let mut bad=next["readiness_followup"].clone();bad["session"]["state"]["readiness"]="forged".into();
        assert!(from_json(&serde_json::json!({"schema":"rinkcraft.unit-studio-command.v1","command":{"kind":"import_readiness_followup","followup":bad}}).to_string()).is_err());
    }
    #[test]
    fn readiness_commands_keep_scene_exact_and_import_verified_reports() {
        let replay:serde_json::Value=serde_json::from_str(include_str!("../../tests/fixtures/rinkcraft-unit-v04.json")).unwrap();
        let value=command(serde_json::json!({"kind":"assess_readiness","replay":replay,"situation":"reset","seed":0,"lateral_feed_declared":false}),None);
        assert_eq!(value["replay"],replay);
        assert_eq!(value["goalie_readiness"]["input"]["replay"],replay);
        assert_eq!(value["goalie_readiness"]["assessment"]["actor_capability"],"Recovery");
        let imported=command(serde_json::json!({"kind":"import_readiness","report":value["goalie_readiness"]}),None);
        assert_eq!(imported["replay"],replay);assert_eq!(imported["goalie_readiness"],value["goalie_readiness"]);
        let mut bad=value["goalie_readiness"].clone();bad["assessment"]["readiness"]="reset".into();bad["assessment"]["source_scene_sha256"]="forged".into();
        let raw=serde_json::json!({"schema":"rinkcraft.unit-studio-command.v1","command":{"kind":"import_readiness","report":bad}});
        assert!(from_json(&raw.to_string()).is_err());
    }
    fn file() -> units::Replay {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/rinkcraft-legacy-possession.json"
        ))
        .unwrap();
        units::export(units::freeze_legacy(&golden["cases"][0].to_string()).unwrap()).unwrap()
    }
    fn command(
        command: serde_json::Value,
        sheet: Option<game::GameCharacter>,
    ) -> serde_json::Value {
        serde_json::from_str(&from_json(&serde_json::json!({"schema":"rinkcraft.unit-studio-command.v1","command":command,"sheet_character":sheet}).to_string()).unwrap()).unwrap()
    }
    #[test]
    fn hero_selection_preserves_events_and_reset_applies_setup_only_to_new_history() {
        let file = file();
        let selected = command(
            serde_json::json!({"kind":"select_hero","replay":file,"hero_id":"home-D2"}),
            None,
        );
        assert_eq!(
            selected["replay"]["session"]["scene"]["events"],
            file.session["scene"]["events"]
        );
        assert_eq!(
            selected["replay"]["session"]["units_sha256"],
            file.session["units_sha256"]
        );
        let reset = command(
            serde_json::json!({"kind":"reset","replay":selected["replay"],"assignments":file.input.request.assignments,"hero_id":"home-D2","seed":26,"commit_support":true}),
            None,
        );
        assert_eq!(
            reset["replay"]["input"]["request"]["actions"],
            serde_json::json!([])
        );
        assert_eq!(
            reset["replay"]["session"]["scene"]["state"]["cover_owner"],
            serde_json::Value::Null
        );
    }
    #[test]
    fn rust_projects_compatible_placements_and_placing_does_not_reassign_duties() {
        let file = file();
        let member = file
            .input
            .request
            .members
            .iter()
            .find(|m| m.slot == units::Slot::D2 && m.side == units::Side::Home)
            .unwrap();
        let sheet = member.character.clone();
        let imported = command(
            serde_json::json!({"kind":"import","replay":file}),
            Some(sheet.clone()),
        );
        assert_eq!(
            imported["layout"]["compatible_sheet_slots"],
            serde_json::json!(["D1", "D2"])
        );
        let placed = command(
            serde_json::json!({"kind":"place_character","replay":file,"slot":"D2"}),
            Some(sheet),
        );
        assert_eq!(
            placed["replay"]["input"]["request"]["assignments"],
            serde_json::to_value(&file.input.request.assignments).unwrap()
        );
        assert_eq!(placed["replay"]["input"]["request"]["hero_id"], "home-D2");
        assert_eq!(
            placed["replay"]["input"]["request"]["actions"],
            serde_json::json!([])
        );
        let mut foreign = member.character.clone();
        foreign.rules_sha256 = "wrong".into();
        let view = command(
            serde_json::json!({"kind":"import","replay":file}),
            Some(foreign.clone()),
        );
        assert_eq!(
            view["layout"]["compatible_sheet_slots"],
            serde_json::json!([])
        );
        assert!(from_json(&serde_json::json!({"schema":"rinkcraft.unit-studio-command.v1","sheet_character":foreign,"command":{"kind":"place_character","replay":file,"slot":"D2"}}).to_string()).is_err());
    }
    #[test]
    fn all_command_boundaries_reject_unknowns_oversize_and_inconsistent_sources() {
        assert!(from_json(&"x".repeat(MAX_COMMAND_BYTES + 1)).is_err());
        assert!(from_json("{}").is_err());
        let file = file();
        let mut raw = serde_json::json!({"schema":"rinkcraft.unit-studio-command.v1","sheet_character":null,"command":{"kind":"import","replay":file,"ignored":true}});
        assert!(from_json(&raw.to_string()).is_err());
        raw["command"].as_object_mut().unwrap().remove("ignored");
        raw["command"]["replay"]["session"]["units_sha256"] = "wrong".into();
        assert!(from_json(&raw.to_string()).is_err());
    }
    #[test]
    fn receiving_upgrade_starts_new_history_without_rewriting_v02() {
        let file = file();
        let upgraded = command(
            serde_json::json!({"kind":"start_receiving","replay":file}),
            None,
        );
        assert_eq!(
            upgraded["replay"]["input"]["rules"]["version"],
            "0.3.0-playtest"
        );
        assert_eq!(
            upgraded["replay"]["input"]["request"]["actions"],
            serde_json::json!([])
        );
        assert_eq!(
            upgraded["replay"]["session"]["units_sha256"],
            file.session["units_sha256"]
        );
        assert_eq!(file.input.rules.version, "0.2.0-playtest");
        assert!(!file.input.request.actions.is_empty());
    }
    #[test]
    fn shooting_upgrade_preserves_units_but_starts_new_history() {
        let file = file();
        let upgraded = command(serde_json::json!({"kind":"start_shooting","replay":file}), None);
        assert_eq!(upgraded["replay"]["input"]["rules"]["version"], "0.4.0-playtest");
        assert_eq!(upgraded["replay"]["input"]["request"]["actions"], serde_json::json!([]));
        assert_eq!(upgraded["replay"]["session"]["units_sha256"], file.session["units_sha256"]);
        assert_eq!(file.input.rules.version,"0.2.0-playtest");
    }
}
