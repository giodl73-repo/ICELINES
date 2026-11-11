//! Replayable fictional possession scene. Never an NHL probability or full-match model.
use super::rinkcraft_contest::{Contest, ExactCounts, Outcome, Resolution, ScoreRule};
use super::rinkcraft_game::{
    self as game, Branch, BuildRequest, GameCharacter, GameRules, ScoreMethod,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub support_bonus: u8,
}
impl Rules {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.possession-rules.v1"
            || self.version != "0.1.0-playtest"
            || self.support_bonus > 2
        {
            return Err("Unsupported possession rules; use v0.1.0-playtest and support 0–2".into());
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Outlet,
    Carry,
    Pass,
    Recover,
    Counter,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub hero: GameCharacter,
    pub seed: u32,
    pub commit_support: bool,
    pub actions: Vec<Action>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    GoalieControl,
    Breakout,
    Advanced,
    Loose,
    Counter,
    Complete,
    Stalemate,
    CounterAdvanced,
    CounterStopped,
    WindowEnded,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct State {
    pub phase: Phase,
    pub control: Option<String>,
    pub loose: bool,
    pub cover_owner: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Participant {
    pub slot: String,
    pub station: String,
    pub character: GameCharacter,
}
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub action: Action,
    pub actor: String,
    pub opposition: String,
    pub actor_capability: String,
    pub opposition_capability: String,
    pub contest: Contest,
    pub resolution: Resolution,
    pub counts: ExactCounts,
    pub terms: [i16; 2],
    pub before: State,
    pub after: State,
    pub explanation: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Session {
    pub schema: String,
    pub rules_version: String,
    pub rules_sha256: String,
    pub seed: u32,
    pub commit_support: bool,
    pub hero_slot: String,
    pub participants: Vec<Participant>,
    pub state: State,
    pub events: Vec<Event>,
    pub legal_actions: Vec<Action>,
    pub note: String,
}
fn legal(s: &State) -> Vec<Action> {
    match s.phase {
        Phase::GoalieControl => vec![Action::Outlet],
        Phase::Breakout => vec![Action::Carry],
        Phase::Advanced => vec![Action::Pass],
        Phase::Loose => vec![Action::Recover],
        Phase::Counter => vec![Action::Counter],
        _ => vec![],
    }
}
fn d20(state: &mut u64) -> u8 {
    let limit = u32::MAX - u32::MAX % 20;
    loop {
        let n = game::random_word(state);
        if n < limit {
            return (n % 20 + 1) as u8;
        }
    }
}
pub fn replay(char_rules: &GameRules, rules: &Rules, request: Request) -> Result<Session, String> {
    rules.validate()?;
    char_rules.validate()?;
    if request.actions.len() > 16 {
        return Err("Possession window allows at most 16 opportunities".into());
    }
    // Validate the entire fictional export under its original character rules.
    game::import_from_json(
        &serde_json::to_string(char_rules).map_err(|e| e.to_string())?,
        &serde_json::to_string(&request.hero).map_err(|e| e.to_string())?,
    )?;
    let skaters: Vec<_> = char_rules
        .templates
        .iter()
        .filter(|t| t.branch == Branch::Skater)
        .collect();
    let goalies: Vec<_> = char_rules
        .templates
        .iter()
        .filter(|t| t.branch == Branch::Goalie)
        .collect();
    if skaters.is_empty() || goalies.is_empty() {
        return Err("Scene requires both skater and goalie templates".into());
    }
    let hero_slot = if request.hero.branch == Branch::Goalie {
        "home:G"
    } else if ["D", "LD", "RD"].contains(&request.hero.position.as_str()) {
        "home:D1"
    } else {
        "home:F1"
    }
    .to_string();
    let carrier = if hero_slot == "home:D1" {
        "home:D1"
    } else {
        "home:F1"
    };
    let mut participants = Vec::new();
    for (team, offset) in [("home", 0), ("away", 2)] {
        for (i, (position, station)) in [
            ("F1", "wall carrier"),
            ("F2", "middle receiver"),
            ("F3", "high cover"),
            ("D1", "point support"),
            ("D2", "last line"),
            ("G", "crease exchange"),
        ]
        .iter()
        .enumerate()
        {
            let slot = format!("{team}:{position}");
            let template = if *position == "G" {
                goalies[offset % goalies.len()]
            } else {
                skaters[(i + offset) % skaters.len()]
            };
            let character = if slot == hero_slot {
                request.hero.clone()
            } else {
                game::build(
                    char_rules,
                    BuildRequest {
                        template_id: template.id.clone(),
                        name: format!("{team} {position}"),
                        position: if *position == "G" {
                            "G"
                        } else if position.starts_with('D') {
                            "D"
                        } else {
                            "F"
                        }
                        .into(),
                        scores: ScoreMethod::StandardArray,
                        inspiration: None,
                    },
                )?
            };
            participants.push(Participant {
                slot: slot.clone(),
                station: if slot == "home:F3" && request.commit_support {
                    "outlet support; high cover vacated".into()
                } else {
                    station.to_string()
                },
                character,
            });
        }
    }
    let mut state = State {
        phase: Phase::GoalieControl,
        control: Some("home:G".into()),
        loose: false,
        cover_owner: if request.commit_support {
            None
        } else {
            Some("home:F3".into())
        },
    };
    let mut rng = u64::from(request.seed);
    let mut events = Vec::new();
    for action in &request.actions {
        if !legal(&state).contains(action) {
            return Err(format!(
                "{action:?} has no legal opportunity in {:?}",
                state.phase
            ));
        }
        let before = state.clone();
        let (actor, opposition, ai, oi) = match action {
            Action::Outlet => ("home:G".to_string(), "away:F1", 5, 4),
            Action::Carry => (
                state.control.clone().ok_or("Carry requires control")?,
                "away:D1",
                0,
                5,
            ),
            Action::Pass => (
                state.control.clone().ok_or("Pass requires control")?,
                "away:F2",
                2,
                4,
            ),
            Action::Recover => (carrier.into(), "away:F1", 4, 1),
            Action::Counter => ("away:F1".into(), "home:D2", 0, 5),
        };
        let a = &participants
            .iter()
            .find(|p| p.slot == actor)
            .ok_or("Missing actor")?
            .character
            .capabilities[ai];
        let b = &participants
            .iter()
            .find(|p| p.slot == opposition)
            .ok_or("Missing opposition")?
            .character
            .capabilities[oi];
        let support = if request.commit_support
            && matches!(action, Action::Outlet | Action::Carry | Action::Pass)
        {
            rules.support_bonus as i8
        } else {
            0
        };
        let cover = if *action == Action::Counter && state.cover_owner.is_some() {
            rules.support_bonus as i8
        } else {
            0
        };
        let contest = Contest {
            actor_score: a.score,
            opposition_score: b.score,
            score_rule: ScoreRule::HalfOffset,
            actor_adjustment: support,
            opposition_adjustment: cover,
        };
        let resolution = contest.resolve(d20(&mut rng), d20(&mut rng))?;
        let explanation=match resolution.outcome {
            Outcome::Stalemate=>{state.phase=Phase::Stalemate;"Opportunity ends tied. Existing control or loose-puck status is preserved; no automatic retry."},
            Outcome::ActorWins=>match action {
                Action::Outlet=>{state.phase=Phase::Breakout;state.control=Some(carrier.into());"Goalie outlet reaches the carrier. This is possession, not a scoring chance."},
                Action::Carry=>{state.phase=Phase::Advanced;"Carrier advances; the middle passing opportunity opens."},
                Action::Pass=>{state.phase=Phase::Complete;state.control=Some("home:F2".into());"Receiver controls the pass. Possession sequence complete; no shot or goal is awarded."},
                Action::Recover=>{state.phase=Phase::Breakout;state.control=Some(carrier.into());state.loose=false;"Home recovers the loose puck, creating a new breakout opportunity."},
                Action::Counter=>{state.phase=Phase::CounterAdvanced;"Away advances the counter. The scene ends without a shot or goal."},
            },
            Outcome::OppositionWins=>match action {
                Action::Recover=>{state.phase=Phase::Counter;state.control=Some("away:F1".into());state.loose=false;"Away wins recovery and gains control. High cover will affect the counter defense."},
                Action::Counter=>{state.phase=Phase::CounterStopped;state.control=None;state.loose=true;"Home disrupts the counter into a loose puck. No automatic home possession or goal."},
                _=>{state.phase=Phase::Loose;state.control=None;state.loose=true;"The attempted action breaks down into a loose puck; a lost roll is not an automatic turnover."},
            },
        }.to_string();
        events.push(Event {
            action: *action,
            actor,
            opposition: opposition.into(),
            actor_capability: a.label.clone(),
            opposition_capability: b.label.clone(),
            counts: contest.exact_counts()?,
            terms: contest.terms()?,
            contest,
            resolution,
            before,
            after: state.clone(),
            explanation,
        });
        let station = match state.phase {
            Phase::Breakout => Some("breakout lane"),
            Phase::Advanced => Some("advanced carrier lane"),
            Phase::Complete => Some("controlled receiving lane"),
            Phase::Counter => Some("counter lane"),
            Phase::CounterAdvanced => Some("advanced counter lane"),
            _ => None,
        };
        if let (Some(station), Some(holder)) = (station, state.control.as_ref()) {
            participants
                .iter_mut()
                .find(|p| &p.slot == holder)
                .ok_or("Missing puck holder")?
                .station = station.into();
        }
    }
    if request.actions.len() == 16 && !legal(&state).is_empty() {
        state.phase = Phase::WindowEnded;
    }
    let canonical = serde_json::to_vec(rules).map_err(|e| e.to_string())?;
    Ok(Session {schema:"icelines.possession-session.v1".into(),rules_version:rules.version.clone(),rules_sha256:format!("{:x}",Sha256::digest(canonical)),seed:request.seed,commit_support:request.commit_support,hero_slot,participants,legal_actions:legal(&state),state,events,note:"Fictional outlet/carry/pass scene; scripted opponent responses, half-offset arithmetic and a support/cover tradeoff. No shots, goals, clock, fatigue or NHL probabilities. F1/F2/F3 name this scene's duties only.".into()})
}
pub fn from_json(char_rules: &str, rules: &str, request: &str) -> Result<String, String> {
    if char_rules.len() > 65536 || rules.len() > 8192 || request.len() > 32768 {
        return Err("Possession input exceeds bounds".into());
    }
    let c: GameRules = serde_json::from_str(char_rules).map_err(|e| e.to_string())?;
    let r: Rules = serde_json::from_str(rules).map_err(|e| e.to_string())?;
    let q: Request = serde_json::from_str(request).map_err(|e| e.to_string())?;
    serde_json::to_string(&replay(&c, &r, q)?).map_err(|e| e.to_string())
}

/// A self-contained, internally verifiable saved scene. This is not an author signature.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayFile {
    pub schema: String,
    pub character_rules: GameRules,
    pub rules: Rules,
    pub request: Request,
    pub session: serde_json::Value,
}
pub const MAX_REPLAY_BYTES: usize = 256 * 1024;

pub fn export_replay_from_json(
    character_rules: &str,
    rules: &str,
    request: &str,
) -> Result<String, String> {
    let session = serde_json::from_str(&from_json(character_rules, rules, request)?)
        .map_err(|e| e.to_string())?;
    let file = ReplayFile {
        schema: "rinkcraft.possession-replay.v1".into(),
        character_rules: serde_json::from_str(character_rules).map_err(|e| e.to_string())?,
        rules: serde_json::from_str(rules).map_err(|e| e.to_string())?,
        request: serde_json::from_str(request).map_err(|e| e.to_string())?,
        session,
    };
    let raw = serde_json::to_string(&file).map_err(|e| e.to_string())?;
    if raw.len() > MAX_REPLAY_BYTES {
        return Err("Replay exceeds 256 KiB".into());
    }
    Ok(raw)
}

pub fn import_replay(raw: &str) -> Result<ReplayFile, String> {
    if raw.len() > MAX_REPLAY_BYTES {
        return Err("Replay exceeds 256 KiB".into());
    }
    let file: ReplayFile = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if file.schema != "rinkcraft.possession-replay.v1" {
        return Err("Unsupported possession replay schema".into());
    }
    let expected: serde_json::Value = serde_json::from_str(&from_json(
        &serde_json::to_string(&file.character_rules).map_err(|e| e.to_string())?,
        &serde_json::to_string(&file.rules).map_err(|e| e.to_string())?,
        &serde_json::to_string(&file.request).map_err(|e| e.to_string())?,
    )?)
    .map_err(|e| e.to_string())?;
    if file.session != expected {
        return Err(
            "Replay does not match its character, rules, dice, actions or computed state".into(),
        );
    }
    Ok(file)
}
pub fn import_replay_from_json(raw: &str) -> Result<String, String> {
    serde_json::to_string(&import_replay(raw)?).map_err(|e| e.to_string())
}

#[derive(Clone, Debug, Serialize)]
pub struct Inspection {
    pub schema: String,
    pub parent_replay_sha256: String,
    pub step: usize,
    pub total_steps: usize,
    pub can_branch: bool,
    pub support_change_allowed: bool,
    pub view: Session,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForkRequest {
    pub step: usize,
    pub commit_support: Option<bool>,
}
#[derive(Clone, Debug, Serialize)]
pub struct BranchRecord {
    pub schema: String,
    pub parent_replay_sha256: String,
    pub selected_step: usize,
    pub discarded_actions: Vec<Action>,
    pub support_changed: bool,
    pub replay: ReplayFile,
}
pub(crate) fn replay_fingerprint(file: &ReplayFile) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(file).map_err(|e| e.to_string())?)
    ))
}
fn prefix_request(file: &ReplayFile, step: usize) -> Result<Request, String> {
    if step > file.request.actions.len() {
        return Err("Review step is outside the recorded history".into());
    }
    let mut request = file.request.clone();
    request.actions.truncate(step);
    Ok(request)
}
pub fn inspect_replay(raw: &str, step: usize) -> Result<Inspection, String> {
    let file = import_replay(raw)?;
    let request = prefix_request(&file, step)?;
    let view = replay(&file.character_rules, &file.rules, request)?;
    Ok(Inspection {
        schema: "icelines.possession-inspection.v1".into(),
        parent_replay_sha256: replay_fingerprint(&file)?,
        step,
        total_steps: file.request.actions.len(),
        can_branch: !view.legal_actions.is_empty(),
        support_change_allowed: step == 0,
        view,
    })
}
pub fn inspect_replay_from_json(raw: &str, step: usize) -> Result<String, String> {
    serde_json::to_string(&inspect_replay(raw, step)?).map_err(|e| e.to_string())
}
pub fn branch_replay(raw: &str, fork: ForkRequest) -> Result<BranchRecord, String> {
    let file = import_replay(raw)?;
    let mut request = prefix_request(&file, fork.step)?;
    let support_changed = fork
        .commit_support
        .is_some_and(|value| value != request.commit_support);
    if support_changed && fork.step != 0 {
        return Err(
            "Support is committed before the first roll; branch at step 0 to change it".into(),
        );
    }
    if let Some(value) = fork.commit_support {
        request.commit_support = value;
    }
    let session = replay(&file.character_rules, &file.rules, request.clone())?;
    if session.legal_actions.is_empty() {
        return Err("The selected moment has no legal continuation; choose an earlier step".into());
    }
    let child = export_replay_from_json(
        &serde_json::to_string(&file.character_rules).map_err(|e| e.to_string())?,
        &serde_json::to_string(&file.rules).map_err(|e| e.to_string())?,
        &serde_json::to_string(&request).map_err(|e| e.to_string())?,
    )?;
    Ok(BranchRecord {
        schema: "rinkcraft.possession-branch.v1".into(),
        parent_replay_sha256: replay_fingerprint(&file)?,
        selected_step: fork.step,
        discarded_actions: file.request.actions[fork.step..].to_vec(),
        support_changed,
        replay: import_replay(&child)?,
    })
}
pub fn branch_replay_from_json(raw: &str, fork: &str) -> Result<String, String> {
    if fork.len() > 1024 {
        return Err("Branch request exceeds 1 KiB".into());
    }
    let fork: ForkRequest = serde_json::from_str(fork).map_err(|e| e.to_string())?;
    serde_json::to_string(&branch_replay(raw, fork)?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(goalie: bool) -> (GameRules, Rules, Request) {
        let templates:Vec<_>=[("s","skater"),("g","goalie")].iter().map(|(id,branch)|serde_json::json!({"id":id,"branch":branch,"class_id":"test","class_name":"Test","specialty":"Test","description":"Fictional fixture","priority":[0,1,2,3,4,5],"task_skills":[],"methods":[],"tendencies":[],"playbook_options":[]})).collect();
        let c:GameRules=serde_json::from_value(serde_json::json!({"schema":"rinkcraft.game-rules.v1","version":"test","title":"Test","standard_array":[12,12,12,12,12,12],"templates":templates})).unwrap();
        let hero = game::build(
            &c,
            BuildRequest {
                template_id: if goalie { "g" } else { "s" }.into(),
                name: "Hero".into(),
                position: if goalie { "G" } else { "F" }.into(),
                scores: ScoreMethod::StandardArray,
                inspiration: None,
            },
        )
        .unwrap();
        (
            c,
            Rules {
                schema: "rinkcraft.possession-rules.v1".into(),
                version: "0.1.0-playtest".into(),
                support_bonus: 2,
            },
            Request {
                hero,
                seed: 0,
                commit_support: false,
                actions: vec![],
            },
        )
    }
    #[test]
    fn six_per_side_unique_slots_active_goalie_and_replay() {
        for goalie in [false, true] {
            let (c, r, mut q) = fixture(goalie);
            q.actions = vec![Action::Outlet];
            let a = replay(&c, &r, q.clone()).unwrap();
            let b = replay(&c, &r, q).unwrap();
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&b).unwrap()
            );
            assert_eq!(a.participants.len(), 12);
            assert_eq!(
                a.participants
                    .iter()
                    .map(|p| &p.slot)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                12
            );
            assert_eq!(a.events[0].actor, "home:G");
            assert_eq!(a.events[0].actor_capability, "Puck Play");
            assert_eq!(a.hero_slot, if goalie { "home:G" } else { "home:F1" });
            assert!(a
                .participants
                .iter()
                .all(|p| (p.slot.ends_with(":G")) == (p.character.branch == Branch::Goalie)));
        }
    }
    #[test]
    fn closed_lane_rejected_before_any_roll_and_exports_validated() {
        let (c, r, mut q) = fixture(false);
        q.actions = vec![Action::Pass];
        assert!(replay(&c, &r, q.clone()).is_err());
        q.actions.clear();
        q.hero.capabilities[0].score = 20;
        assert!(replay(&c, &r, q).is_err());
    }
    #[test]
    fn deterministic_sequences_cover_failures_recovery_counter_and_conserve_control() {
        let (c, r, base) = fixture(false);
        let mut phases = std::collections::BTreeSet::new();
        for seed in 0..100 {
            let mut q = base.clone();
            q.seed = seed;
            loop {
                let s = replay(&c, &r, q.clone()).unwrap();
                assert_eq!(s.state.control.is_some(), !s.state.loose);
                phases.insert(format!("{:?}", s.state.phase));
                for e in &s.events {
                    assert_eq!(e.contest.score_rule as u8, ScoreRule::HalfOffset as u8);
                    assert_eq!(e.after.control.is_some(), !e.after.loose);
                }
                if let Some(a) = s.legal_actions.first() {
                    q.actions.push(*a)
                } else {
                    break;
                }
            }
        }
        for p in [
            "Complete",
            "Loose",
            "Counter",
            "CounterAdvanced",
            "CounterStopped",
            "Stalemate",
        ] {
            assert!(phases.contains(p), "Missing branch {p}");
        }
    }
    #[test]
    fn support_vacates_cover_and_changes_counter_defense_not_class() {
        let (c, r, base) = fixture(false);
        let mut found = false;
        for seed in 0..500 {
            let mut q = base.clone();
            q.seed = seed;
            q.actions = vec![Action::Outlet, Action::Recover, Action::Counter];
            if let Ok(hold) = replay(&c, &r, q.clone()) {
                q.commit_support = true;
                if let Ok(commit) = replay(&c, &r, q) {
                    assert_eq!(hold.events[2].contest.opposition_adjustment, 2);
                    assert_eq!(commit.events[2].contest.opposition_adjustment, 0);
                    assert_eq!(commit.events[0].contest.actor_adjustment, 2);
                    assert_eq!(hold.events[0].contest.actor_adjustment, 0);
                    assert!(hold.state.cover_owner.is_some());
                    assert!(commit.state.cover_owner.is_none());
                    assert!(commit
                        .participants
                        .iter()
                        .find(|p| p.slot == "home:F3")
                        .unwrap()
                        .station
                        .contains("vacated"));
                    found = true;
                    break;
                }
            }
        }
        assert!(found);
    }
    fn saved_replay(goalie: bool, steps: usize) -> String {
        let (c, r, mut q) = fixture(goalie);
        for _ in 0..steps {
            let s = replay(&c, &r, q.clone()).unwrap();
            if let Some(a) = s.legal_actions.first() {
                q.actions.push(*a)
            } else {
                break;
            }
        }
        export_replay_from_json(
            &serde_json::to_string(&c).unwrap(),
            &serde_json::to_string(&r).unwrap(),
            &serde_json::to_string(&q).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn replay_import_accepts_current_exports_at_initial_intermediate_and_terminal_states() {
        for goalie in [false, true] {
            for steps in [0, 1, 16] {
                let raw = saved_replay(goalie, steps);
                assert_eq!(import_replay_from_json(&raw).unwrap(), raw);
                let saved = import_replay(&raw).unwrap();
                assert_eq!(
                    saved.request.hero.branch,
                    if goalie {
                        Branch::Goalie
                    } else {
                        Branch::Skater
                    }
                );
            }
        }
    }
    #[test]
    fn replay_import_recomputes_dice_state_participants_rules_hash_and_legal_actions() {
        let original: serde_json::Value = serde_json::from_str(&saved_replay(false, 1)).unwrap();
        let mut cases = Vec::new();
        let mut x = original.clone();
        x["session"]["state"]["control"] = serde_json::json!("away:G");
        cases.push(x);
        let mut x = original.clone();
        x["session"]["events"][0]["resolution"]["actor_die"] = serde_json::json!(0);
        cases.push(x);
        let mut x = original.clone();
        x["session"]["participants"][0]["character"]["capabilities"][0]["score"] =
            serde_json::json!(0);
        cases.push(x);
        let mut x = original.clone();
        x["session"]["rules_sha256"] = serde_json::json!("bad");
        cases.push(x);
        let mut x = original.clone();
        x["session"]["legal_actions"] = serde_json::json!(["pass"]);
        cases.push(x);
        let mut x = original;
        x["session"]["nhl_verified"] = serde_json::json!(true);
        cases.push(x);
        for x in cases {
            assert!(import_replay_from_json(&x.to_string()).is_err());
        }
    }
    #[test]
    fn replay_import_rejects_invalid_characters_histories_versions_schemas_and_sizes() {
        let original: serde_json::Value = serde_json::from_str(&saved_replay(false, 1)).unwrap();
        let mut cases = Vec::new();
        let mut x = original.clone();
        x["request"]["hero"]["capabilities"][0]["score"] = serde_json::json!(0);
        cases.push(x);
        let mut x = original.clone();
        x["request"]["actions"] = serde_json::json!(["pass"]);
        cases.push(x);
        let mut x = original.clone();
        x["rules"]["version"] = serde_json::json!("future");
        cases.push(x);
        let mut x = original.clone();
        x["schema"] = serde_json::json!("rinkcraft.character-catalog.v1");
        cases.push(x);
        let mut x = original;
        x["unknown"] = serde_json::json!(true);
        cases.push(x);
        for x in cases {
            assert!(import_replay_from_json(&x.to_string()).is_err());
        }
        assert!(import_replay_from_json(&"x".repeat(MAX_REPLAY_BYTES + 1)).is_err());
        assert!(import_replay_from_json("{}").is_err());
    }
    #[test]
    fn inspection_reconstructs_every_prefix_and_keeps_parent_unchanged() {
        let raw = saved_replay(false, 16);
        let file = import_replay(&raw).unwrap();
        for step in 0..=file.request.actions.len() {
            let snapshot = inspect_replay(&raw, step).unwrap();
            let mut q = file.request.clone();
            q.actions.truncate(step);
            assert_eq!(
                serde_json::to_value(snapshot.view).unwrap(),
                serde_json::to_value(replay(&file.character_rules, &file.rules, q).unwrap())
                    .unwrap()
            );
            assert_eq!(snapshot.support_change_allowed, step == 0);
            assert_eq!(snapshot.total_steps, file.request.actions.len());
        }
        assert_eq!(import_replay_from_json(&raw).unwrap(), raw);
        assert!(inspect_replay(&raw, file.request.actions.len() + 1).is_err());
        let mut corrupt: serde_json::Value = serde_json::from_str(&raw).unwrap();
        corrupt["session"]["seed"] = serde_json::json!(u32::MAX);
        assert!(inspect_replay(&corrupt.to_string(), 0).is_err());
    }
    #[test]
    fn branches_retain_seed_and_verified_history_and_have_content_based_parent_ids() {
        let raw = saved_replay(true, 16);
        let parent = import_replay(&raw).unwrap();
        let branch = branch_replay(
            &raw,
            ForkRequest {
                step: 0,
                commit_support: Some(true),
            },
        )
        .unwrap();
        assert_eq!(branch.replay.request.seed, parent.request.seed);
        assert!(branch.replay.request.commit_support);
        assert!(branch.support_changed);
        assert!(branch.replay.request.actions.is_empty());
        assert_eq!(branch.discarded_actions, parent.request.actions);
        assert_eq!(branch.replay.request.hero.branch, Branch::Goalie);
        assert_eq!(
            branch.replay.session["state"]["cover_owner"],
            serde_json::Value::Null
        );
        let pretty = serde_json::to_string_pretty(&parent).unwrap();
        assert_eq!(
            inspect_replay(&pretty, 0).unwrap().parent_replay_sha256,
            branch.parent_replay_sha256
        );
        assert_eq!(import_replay_from_json(&raw).unwrap(), raw);
        let same = branch_replay(
            &raw,
            ForkRequest {
                step: 0,
                commit_support: None,
            },
        )
        .unwrap();
        assert!(!same.support_changed);
    }
    #[test]
    fn later_branch_retains_prefix_and_continues_with_the_same_next_dice() {
        let (c, r, mut q) = fixture(false);
        q.seed = 7;
        q.actions = vec![Action::Outlet];
        let next = replay(&c, &r, q.clone()).unwrap().legal_actions[0];
        q.actions.push(next);
        let raw = export_replay_from_json(
            &serde_json::to_string(&c).unwrap(),
            &serde_json::to_string(&r).unwrap(),
            &serde_json::to_string(&q).unwrap(),
        )
        .unwrap();
        let b = branch_replay(
            &raw,
            ForkRequest {
                step: 1,
                commit_support: None,
            },
        )
        .unwrap();
        assert_eq!(b.replay.request.actions, vec![Action::Outlet]);
        assert_eq!(b.discarded_actions, vec![next]);
        let mut continued = b.replay.request;
        continued.actions.push(next);
        assert_eq!(
            serde_json::to_value(replay(&c, &r, continued).unwrap()).unwrap(),
            import_replay(&raw).unwrap().session
        );
    }
    #[test]
    fn branching_rejects_retroactive_support_terminal_steps_and_unknown_requests() {
        let raw = saved_replay(false, 16);
        let file = import_replay(&raw).unwrap();
        assert!(!file.request.actions.is_empty());
        assert!(branch_replay(
            &raw,
            ForkRequest {
                step: 1,
                commit_support: Some(true)
            }
        )
        .is_err());
        assert!(branch_replay(
            &raw,
            ForkRequest {
                step: file.request.actions.len(),
                commit_support: None
            }
        )
        .is_err());
        assert!(branch_replay_from_json(&raw, r#"{"step":0,"seed":999}"#).is_err());
        assert!(branch_replay_from_json(&raw, &"x".repeat(1025)).is_err());
    }
    #[test]
    fn terminal_windows_bounds_and_unknown_state_are_rejected() {
        let (c, r, mut q) = fixture(false);
        q.actions = vec![Action::Outlet; 17];
        assert!(replay(&c, &r, q.clone()).is_err());
        let mut value = serde_json::to_value(q).unwrap();
        value["state"] = serde_json::json!({"control":"home:F2"});
        assert!(serde_json::from_value::<Request>(value).is_err());
        let mut r = r;
        r.support_bonus = 3;
        assert!(r.validate().is_err());
    }
}
