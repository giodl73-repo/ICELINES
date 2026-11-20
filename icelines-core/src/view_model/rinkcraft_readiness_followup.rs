//! Explicit playable branch from a verified readiness report. Parent unit scenes stay frozen.
use super::{
    rinkcraft_contest::{Contest, ExactCounts, Outcome, Resolution, ScoreRule},
    rinkcraft_goalie_readiness as readiness, rinkcraft_possession as possession,
    rinkcraft_units as units,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const MAX_REPLAY_BYTES: usize = 640 * 1024;
pub const MAX_ACTIONS: usize = 8;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            schema: "rinkcraft.readiness-followup-rules.v1".into(),
            version: "0.1.0-playtest".into(),
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Shoot,
    Hold,
    TrackSave,
    ScrambleSave,
    RecoverSet,
    Contain,
    Smother,
    Gather,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    ReadyRelease,
    ShotFlight,
    SaveContact,
    LooseRebound,
    SkaterControl,
    GoalieHeld,
    Held,
    Goal,
    ShotDisrupted,
    Unresolved,
    WindowEnded,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct State {
    pub phase: Phase,
    pub readiness: readiness::Readiness,
    pub control: Option<String>,
    pub loose: bool,
    pub in_flight_to: Option<String>,
    pub scored_by: Option<String>,
    pub cover_owner: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub rules: Rules,
    pub report: readiness::Report,
    pub seed: u32,
    pub actions: Vec<Action>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    pub schema: String,
    pub input: Input,
    pub session: serde_json::Value,
}
#[derive(Serialize)]
pub struct Choice {
    pub action: Action,
    pub actor: String,
    pub opposition: Option<String>,
    pub actor_capability: Option<String>,
    pub opposition_capability: Option<String>,
}
#[derive(Serialize)]
pub struct Event {
    pub action: Action,
    pub actor: String,
    pub opposition: Option<String>,
    pub actor_capability: Option<String>,
    pub opposition_capability: Option<String>,
    pub contest: Option<Contest>,
    pub resolution: Option<Resolution>,
    pub counts: Option<ExactCounts>,
    pub terms: Option<[i16; 2]>,
    pub before: State,
    pub after: State,
    pub explanation: String,
}
#[derive(Serialize)]
pub struct Session {
    pub schema: String,
    pub rules_version: String,
    pub rules_sha256: String,
    pub source_report_sha256: String,
    pub seed: u32,
    pub state: State,
    pub events: Vec<Event>,
    pub legal_actions: Vec<Action>,
    pub choices: Vec<Choice>,
    pub note: String,
}
fn hash<T: Serialize>(v: &T) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(v).map_err(|e| e.to_string())?)
    ))
}
fn ready(r: readiness::Readiness) -> bool {
    matches!(
        r,
        readiness::Readiness::AngleSet | readiness::Readiness::Across | readiness::Readiness::Reset
    )
}
fn legal(s: &State) -> Vec<Action> {
    match s.phase {
        Phase::ReadyRelease | Phase::SkaterControl => vec![Action::Shoot, Action::Hold],
        Phase::ShotFlight if ready(s.readiness) => vec![Action::TrackSave, Action::ScrambleSave],
        Phase::ShotFlight => vec![Action::ScrambleSave, Action::RecoverSet],
        Phase::SaveContact => vec![Action::Contain],
        Phase::LooseRebound if ready(s.readiness) => vec![Action::Smother, Action::Gather],
        Phase::LooseRebound => vec![Action::RecoverSet, Action::Gather],
        _ => vec![],
    }
}
fn pairing(
    action: Action,
    _s: &State,
    receiver: &str,
    gap: &str,
) -> Option<(String, String, usize, usize)> {
    Some(match action {
        Action::Hold => return None,
        Action::Shoot => (receiver.into(), gap.into(), 3, 5),
        Action::TrackSave => ("away:G".into(), receiver.into(), 0, 3),
        Action::ScrambleSave => ("away:G".into(), receiver.into(), 2, 3),
        Action::RecoverSet => ("away:G".into(), receiver.into(), 4, 0),
        Action::Contain | Action::Smother => ("away:G".into(), receiver.into(), 3, 1),
        Action::Gather => (receiver.into(), "away:G".into(), 1, 4),
    })
}
pub fn replay(input: &Input) -> Result<Session, String> {
    if input.schema != "rinkcraft.readiness-followup-input.v1"
        || input.rules.schema != Rules::default().schema
        || input.rules.version != Rules::default().version
    {
        return Err("Unsupported readiness follow-up input or rules".into());
    }
    if input.actions.len() > MAX_ACTIONS {
        return Err("Follow-up exceeds eight declared actions".into());
    }
    let report =
        readiness::import(&serde_json::to_string(&input.report).map_err(|e| e.to_string())?)?;
    let assessed = readiness::assess(&report.input)?;
    let unit = units::replay(&report.input.replay.input)?;
    let receiver = report
        .input
        .replay
        .input
        .request
        .assignments
        .receiver
        .address(units::Side::Home);
    let gap = report
        .input
        .replay
        .input
        .request
        .assignments
        .away_gap
        .address(units::Side::Away);
    let mut state = State {
        phase: match report.input.situation {
            readiness::Situation::SetAngle => Phase::ReadyRelease,
            readiness::Situation::MoveAcross => Phase::ShotFlight,
            readiness::Situation::Reset => Phase::LooseRebound,
        },
        readiness: assessed.readiness,
        control: assessed.puck_state.control,
        loose: assessed.puck_state.loose,
        in_flight_to: assessed.puck_state.in_flight_to,
        scored_by: assessed.puck_state.scored_by,
        cover_owner: assessed.puck_state.cover_owner,
    };
    if state.readiness == readiness::Readiness::Unresolved {
        state.phase = Phase::Unresolved;
    }
    let participant = |address: &str| {
        unit.scene
            .participants
            .iter()
            .find(|p| p.slot == address)
            .ok_or_else(|| "Missing follow-up participant".to_string())
    };
    let mut rng = u64::from(input.seed);
    let mut events = Vec::new();
    for action in &input.actions {
        if !legal(&state).contains(action) {
            return Err(format!(
                "{action:?} is unavailable in {:?} with {:?}",
                state.phase, state.readiness
            ));
        }
        let before = state.clone();
        if *action == Action::Hold {
            state.phase = Phase::Held;
            events.push(Event {action:*action,actor:receiver.clone(),opposition:None,actor_capability:None,opposition_capability:None,contest:None,resolution:None,counts:None,terms:None,before,after:state.clone(),explanation:"Receiver holds the controlled puck and ends this window. No dice, shot or goal.".into()});
            continue;
        }
        let (actor, opposition, ai, oi) =
            pairing(*action, &state, &receiver, &gap).ok_or("Missing contest pairing")?;
        let a = &participant(&actor)?.character.capabilities[ai];
        let b = &participant(&opposition)?.character.capabilities[oi];
        let contest = Contest {
            actor_score: a.score,
            opposition_score: b.score,
            score_rule: ScoreRule::HalfOffset,
            actor_adjustment: 0,
            opposition_adjustment: 0,
        };
        let resolution = contest.resolve(possession::d20(&mut rng), possession::d20(&mut rng))?;
        let explanation=if resolution.outcome==Outcome::Stalemate {
            state.phase=Phase::Unresolved;"Contest tied; the window ends with puck status preserved. This is unresolved, not a save or defensive win."
        } else {match (action,resolution.outcome) {
            (Action::Shoot,Outcome::ActorWins)=>{state.phase=Phase::ShotFlight;state.control=None;state.in_flight_to=Some("away:G".into());"On-target release gets past the declared gap defender. Readiness determines available goalie responses; no goal yet."},
            (Action::Shoot,_)=>{state.phase=Phase::ShotDisrupted;state.control=None;state.loose=true;"Release is disrupted into a loose puck. This bounded window ends without automatic opponent control."},
            (Action::TrackSave|Action::ScrambleSave,Outcome::ActorWins)=>{state.phase=Phase::SaveContact;state.in_flight_to=None;state.loose=true;"Goalie stops the shot; contact remains uncontrolled and containment is required."},
            (Action::TrackSave|Action::ScrambleSave,_)=>{state.phase=Phase::Goal;state.in_flight_to=None;state.scored_by=Some(receiver.clone());"The on-target shot beats the chosen save response. This branch scores, without altering its parent replay."},
            (Action::RecoverSet,Outcome::ActorWins)=>{state.readiness=readiness::Readiness::Reset;"Goalie gets reset before the assigned receiver closes. Puck status is unchanged; tracking or smothering is now available."},
            (Action::RecoverSet,_)=>{state.phase=Phase::Unresolved;"Goalie does not reset within this response window. Puck status is preserved; no automatic goal, save or control is awarded."},
            (Action::Contain,Outcome::ActorWins)|(Action::Smother,Outcome::ActorWins)|(Action::Gather,Outcome::OppositionWins)=>{state.phase=Phase::GoalieHeld;state.control=Some("away:G".into());state.loose=false;"Goalie wins the declared containment or possession contest and secures the puck."},
            (Action::Contain,_)=>{state.phase=Phase::LooseRebound;state.loose=true;state.readiness=readiness::Readiness::Unreset;"Containment fails: the saved puck stays loose and the goalie must respond from an unreset state. No automatic skater control."},
            (Action::Smother,Outcome::OppositionWins)|(Action::Gather,Outcome::ActorWins)=>{state.phase=Phase::SkaterControl;state.control=Some(receiver.clone());state.loose=false;"Assigned receiver wins an explicit possession contest, gaining control but no shot or goal."},
            _=>return Err("Unsupported follow-up resolution".into()),
        }}.to_string();
        events.push(Event {
            action: *action,
            actor,
            opposition: Some(opposition),
            actor_capability: Some(a.label.clone()),
            opposition_capability: Some(b.label.clone()),
            contest: Some(contest),
            resolution: Some(resolution),
            counts: Some(contest.exact_counts()?),
            terms: Some(contest.terms()?),
            before,
            after: state.clone(),
            explanation,
        });
    }
    if input.actions.len() == MAX_ACTIONS && !legal(&state).is_empty() {
        state.phase = Phase::WindowEnded;
    }
    let legal_actions = legal(&state);
    let choices = legal_actions
        .iter()
        .map(|action| -> Result<Choice, String> {
            if let Some((actor, opposition, ai, oi)) = pairing(*action, &state, &receiver, &gap) {
                Ok(Choice {
                    action: *action,
                    actor: actor.clone(),
                    opposition: Some(opposition.clone()),
                    actor_capability: Some(
                        participant(&actor)?.character.capabilities[ai]
                            .label
                            .clone(),
                    ),
                    opposition_capability: Some(
                        participant(&opposition)?.character.capabilities[oi]
                            .label
                            .clone(),
                    ),
                })
            } else {
                Ok(Choice {
                    action: *action,
                    actor: receiver.clone(),
                    opposition: None,
                    actor_capability: None,
                    opposition_capability: None,
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Session{schema:"icelines.readiness-followup-session.v1".into(),rules_version:input.rules.version.clone(),rules_sha256:hash(&input.rules)?,source_report_sha256:hash(&report)?,seed:input.seed,state,events,legal_actions,choices,note:"Authored readiness follow-up 0.1, a separate branch with its own explicit seed. Ready goalies may track or scramble; unready goalies choose movement-based scramble or a recovery race. Gather always opposes goalie Recovery; getting ready never weakens that response. Recovering changes legal responses without a bonus; failure/tie ends unresolved, not saved. Failed containment stays loose; Smother/Gather are explicit possession contests. Eight-action cap, fixed actors and no geometry, clock, fatigue, opponent policy, NHL probabilities or balance claim. Parent unit and readiness report remain unchanged.".into()})
}
pub fn create(report: readiness::Report, seed: u32) -> Result<Replay, String> {
    export(Input {
        schema: "rinkcraft.readiness-followup-input.v1".into(),
        rules: Rules::default(),
        report,
        seed,
        actions: vec![],
    })
}
pub fn export(input: Input) -> Result<Replay, String> {
    let session = serde_json::to_value(replay(&input)?).map_err(|e| e.to_string())?;
    let file = Replay {
        schema: "rinkcraft.readiness-followup-replay.v1".into(),
        input,
        session,
    };
    if serde_json::to_vec(&file).map_err(|e| e.to_string())?.len() > MAX_REPLAY_BYTES {
        return Err("Follow-up replay exceeds 640 KiB".into());
    }
    Ok(file)
}
pub fn import(raw: &str) -> Result<Replay, String> {
    if raw.len() > MAX_REPLAY_BYTES {
        return Err("Follow-up replay exceeds 640 KiB".into());
    }
    let f: Replay = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if f.schema != "rinkcraft.readiness-followup-replay.v1" {
        return Err("Unsupported follow-up replay schema".into());
    }
    let expected = export(f.input.clone())?;
    if expected.session != f.session {
        return Err("Follow-up differs from recomputed session".into());
    }
    Ok(f)
}
pub fn step(file: Replay, action: Action) -> Result<Replay, String> {
    let verified = import(&serde_json::to_string(&file).map_err(|e| e.to_string())?)?;
    let mut input = verified.input;
    input.actions.push(action);
    export(input)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn report(situation: readiness::Situation, score: u8) -> readiness::Report {
        use super::super::rinkcraft_game as game;
        let frozen = readiness::import(include_str!(
            "../../tests/fixtures/rinkcraft-readiness-v01.json"
        ))
        .unwrap();
        let mut input = frozen.input;
        input.situation = situation;
        input.lateral_feed_declared = situation == readiness::Situation::MoveAcross;
        input.replay.input.request.actions = vec![
            possession::Action::Outlet,
            possession::Action::Carry,
            possession::Action::Pass,
            possession::Action::Receive,
            possession::Action::Shoot,
            possession::Action::Save,
            possession::Action::Rebound,
        ];
        input
            .replay
            .input
            .request
            .actions
            .truncate(match situation {
                readiness::Situation::SetAngle => 4,
                readiness::Situation::MoveAcross => 5,
                readiness::Situation::Reset => 7,
            });
        let m = input
            .replay
            .input
            .request
            .members
            .iter_mut()
            .find(|m| m.side == units::Side::Away && m.slot == units::Slot::G)
            .unwrap();
        let mut scores = std::array::from_fn(|i| m.character.capabilities[i].score);
        scores[match situation {
            readiness::Situation::SetAngle => 1,
            readiness::Situation::MoveAcross => 2,
            readiness::Situation::Reset => 4,
        }] = score;
        m.character = game::build(
            &input.replay.input.character_rules,
            game::BuildRequest {
                template_id: m.character.template_id.clone(),
                name: m.character.name.clone(),
                position: m.character.position.clone(),
                scores: game::ScoreMethod::Manual { scores },
                inspiration: None,
            },
        )
        .unwrap();
        input.replay = units::export(input.replay.input).unwrap();
        readiness::export(input).unwrap()
    }
    fn actions(f: &Replay) -> Vec<Action> {
        replay(&f.input).unwrap().legal_actions
    }
    fn compact_report(situation: readiness::Situation) -> readiness::Report {
        use super::super::rinkcraft_game as game;
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/rinkcraft-legacy-possession.json"
        ))
        .unwrap();
        let mut input = units::freeze_legacy(&golden["cases"][0].to_string()).unwrap();
        input.rules.version = "0.4.0-playtest".into();
        input.request.seed = 286;
        for m in &mut input.request.members {
            let scores = if m.side == units::Side::Away && m.slot == units::Slot::G {
                [20, 20, 20, 1, 20, 12]
            } else if m.side == units::Side::Home && m.slot == units::Slot::F2 {
                [12, 20, 12, 12, 12, 12]
            } else {
                [12; 6]
            };
            m.character = game::build(
                &input.character_rules,
                game::BuildRequest {
                    template_id: m.character.template_id.clone(),
                    name: m.character.name.clone(),
                    position: m.character.position.clone(),
                    scores: game::ScoreMethod::Manual { scores },
                    inspiration: None,
                },
            )
            .unwrap();
        }
        input.request.actions = vec![
            possession::Action::Outlet,
            possession::Action::Carry,
            possession::Action::Pass,
            possession::Action::Receive,
            possession::Action::Shoot,
            possession::Action::Save,
            possession::Action::Rebound,
        ];
        input.request.actions.truncate(match situation {
            readiness::Situation::SetAngle => 4,
            readiness::Situation::MoveAcross => 5,
            readiness::Situation::Reset => 7,
        });
        readiness::export(readiness::Input {
            schema: "rinkcraft.goalie-readiness-input.v1".into(),
            rules: readiness::Rules::default(),
            replay: units::export(input).unwrap(),
            situation,
            seed: 0,
            lateral_feed_declared: situation == readiness::Situation::MoveAcross,
        })
        .unwrap()
    }
    #[test]
    fn frozen_readiness_stays_exact_and_readiness_changes_legal_responses() {
        let raw = include_str!("../../tests/fixtures/rinkcraft-readiness-v01.json");
        assert_eq!(
            serde_json::to_value(readiness::import(raw).unwrap()).unwrap(),
            serde_json::from_str::<serde_json::Value>(raw).unwrap()
        );
        for situation in [
            readiness::Situation::SetAngle,
            readiness::Situation::MoveAcross,
        ] {
            let mut low = create(report(situation, 1), 43).unwrap();
            let mut high = create(report(situation, 20), 43).unwrap();
            if situation == readiness::Situation::SetAngle {
                low = step(low, Action::Shoot).unwrap();
                high = step(high, Action::Shoot).unwrap();
                assert_eq!(
                    low.session["events"][0]["resolution"],
                    high.session["events"][0]["resolution"]
                );
            }
            assert_eq!(
                actions(&low),
                vec![Action::ScrambleSave, Action::RecoverSet]
            );
            assert_eq!(
                actions(&high),
                vec![Action::TrackSave, Action::ScrambleSave]
            );
            assert!(step(low, Action::TrackSave).is_err());
            assert!(step(high, Action::RecoverSet).is_err());
        }
        let low = create(report(readiness::Situation::Reset, 1), 43).unwrap();
        let high = create(report(readiness::Situation::Reset, 20), 43).unwrap();
        assert_eq!(actions(&low), vec![Action::RecoverSet, Action::Gather]);
        assert_eq!(actions(&high), vec![Action::Smother, Action::Gather]);
        assert!(step(low, Action::Smother).is_err());
    }
    #[test]
    fn recovery_opens_tracking_without_control_or_bonus_and_failure_is_unresolved() {
        let parent_report = report(readiness::Situation::MoveAcross, 1);
        let original = serde_json::to_value(&parent_report).unwrap();
        let file = step(create(parent_report, 43).unwrap(), Action::RecoverSet).unwrap();
        assert_eq!(file.session["state"]["readiness"], "reset");
        assert_eq!(file.session["state"]["in_flight_to"], "away:G");
        assert!(file.session["state"]["control"].is_null());
        assert_eq!(
            actions(&file),
            vec![Action::TrackSave, Action::ScrambleSave]
        );
        assert_eq!(serde_json::to_value(&file.input.report).unwrap(), original);
        let saved = step(file, Action::TrackSave).unwrap();
        assert_eq!(saved.session["state"]["phase"], "save_contact");
        assert_eq!(saved.session["state"]["loose"], true);
        assert!(saved.session["state"]["control"].is_null());
        for e in saved.session["events"].as_array().unwrap() {
            assert_eq!(e["contest"]["actor_adjustment"], 0);
            assert_eq!(e["contest"]["opposition_adjustment"], 0);
        }
        let failed = step(
            create(report(readiness::Situation::Reset, 1), 43).unwrap(),
            Action::RecoverSet,
        )
        .unwrap();
        assert_eq!(failed.session["state"]["phase"], "unresolved");
        assert_eq!(failed.session["state"]["loose"], true);
        assert!(failed.session["state"]["scored_by"].is_null());
    }
    #[test]
    fn scrambled_save_and_explicit_possession_contests_have_distinct_effects() {
        let saved = step(
            step(
                create(report(readiness::Situation::SetAngle, 1), 43).unwrap(),
                Action::Shoot,
            )
            .unwrap(),
            Action::ScrambleSave,
        )
        .unwrap();
        // The scramble uses Movement, never an invented sum of goalie attributes.
        assert_eq!(saved.session["events"][1]["actor_capability"], "Movement");
        let gathered = step(
            create(report(readiness::Situation::Reset, 20), 43).unwrap(),
            Action::Gather,
        )
        .unwrap();
        assert_eq!(gathered.session["state"]["phase"], "skater_control");
        assert_eq!(gathered.session["state"]["control"], "home:F2");
        assert!(gathered.session["state"]["scored_by"].is_null());
        let held = step(gathered, Action::Hold).unwrap();
        assert_eq!(held.session["state"]["phase"], "held");
        assert!(held.session["events"][1]["contest"].is_null());
        assert!(actions(&held).is_empty());
        let mut found = false;
        for seed in 0..64 {
            let f = create(report(readiness::Situation::MoveAcross, 20), seed).unwrap();
            let s = step(f, Action::TrackSave).unwrap();
            if !actions(&s).contains(&Action::Contain) {
                continue;
            }
            let s = step(s, Action::Contain).unwrap();
            if s.session["state"]["phase"] == "loose_rebound" {
                assert_eq!(s.session["state"]["readiness"], "unreset");
                assert!(s.session["state"]["control"].is_null());
                assert_eq!(actions(&s), vec![Action::RecoverSet, Action::Gather]);
                found = true;
                break;
            }
        }
        assert!(found);
    }
    #[test]
    fn declared_paths_conserve_puck_and_stop_at_cap_without_retries_or_auto_goals() {
        let mut cap_seen = false;
        let parents = [
            readiness::Situation::SetAngle,
            readiness::Situation::MoveAcross,
            readiness::Situation::Reset,
        ]
        .map(compact_report);
        for seed in 0..32 {
            for parent in &parents {
                let mut input = create(parent.clone(), seed).unwrap().input;
                for _ in 0..MAX_ACTIONS {
                    let choices = replay(&input).unwrap().legal_actions;
                    if choices.is_empty() {
                        break;
                    }
                    input.actions.push(choices[0]);
                }
                assert!(input.actions.len() <= MAX_ACTIONS);
                let s = replay(&input).unwrap();
                for state in s
                    .events
                    .iter()
                    .flat_map(|e| [&e.before, &e.after])
                    .chain(std::iter::once(&s.state))
                {
                    assert_eq!(
                        u8::from(state.control.is_some())
                            + u8::from(state.loose)
                            + u8::from(state.in_flight_to.is_some())
                            + u8::from(state.scored_by.is_some()),
                        1
                    );
                }
                if s.state.phase == Phase::WindowEnded {
                    cap_seen = true;
                    assert!(s.legal_actions.is_empty());
                    assert!(s.state.scored_by.is_none());
                }
                let f = export(input).unwrap();
                assert!(import(&serde_json::to_string(&f).unwrap()).is_ok());
            }
        }
        assert!(cap_seen);
    }
    #[test]
    fn unresolved_readiness_and_tied_contests_do_not_grant_legal_responses() {
        let mut tied = false;
        for seed in 0..128 {
            let mut input = report(readiness::Situation::SetAngle, 20).input;
            input.seed = seed;
            let r = readiness::export(input).unwrap();
            if r.assessment["readiness"] == "unresolved" {
                let f = create(r, 0).unwrap();
                assert!(actions(&f).is_empty());
                assert_eq!(f.session["state"]["phase"], "unresolved");
                tied = true;
                break;
            }
        }
        assert!(tied);
        let mut tied = false;
        for seed in 0..64 {
            let f = step(
                create(report(readiness::Situation::MoveAcross, 20), seed).unwrap(),
                Action::TrackSave,
            )
            .unwrap();
            if f.session["state"]["phase"] == "unresolved" {
                assert_eq!(f.session["state"]["in_flight_to"], "away:G");
                assert!(f.session["state"]["scored_by"].is_null());
                assert!(actions(&f).is_empty());
                tied = true;
                break;
            }
        }
        assert!(tied);
    }
    #[test]
    fn replay_binds_parent_rules_actions_and_results_with_strict_bounds() {
        let f = create(report(readiness::Situation::MoveAcross, 20), 0).unwrap();
        let raw = serde_json::to_string(&f).unwrap();
        assert_eq!(
            serde_json::to_value(import(&raw).unwrap()).unwrap(),
            serde_json::to_value(&f).unwrap()
        );
        for pointer in [
            "/session/state/control",
            "/session/source_report_sha256",
            "/input/report/assessment/readiness",
        ] {
            let mut bad = serde_json::to_value(&f).unwrap();
            *bad.pointer_mut(pointer).unwrap() = "forged".into();
            assert!(import(&bad.to_string()).is_err());
        }
        let mut input = f.input.clone();
        input.rules.version = "future".into();
        assert!(export(input).is_err());
        let mut input = f.input.clone();
        input.actions = vec![Action::TrackSave; MAX_ACTIONS + 1];
        assert!(export(input).is_err());
        let mut bad = serde_json::to_value(&f).unwrap();
        bad["unknown"] = true.into();
        assert!(import(&bad.to_string()).is_err());
        assert!(import(&" ".repeat(MAX_REPLAY_BYTES + 1)).is_err());
    }
    #[test]
    fn readiness_opens_options_without_weakening_existing_responses_or_adding_modifiers() {
        for (situation, retained, extra) in [
            (
                readiness::Situation::MoveAcross,
                Action::ScrambleSave,
                Action::TrackSave,
            ),
            (readiness::Situation::Reset, Action::Gather, Action::Smother),
        ] {
            let ready_report = report(situation, 20);
            let mut unready = None;
            for seed in 0..128 {
                let mut input = ready_report.input.clone();
                input.seed = seed;
                let candidate = readiness::export(input).unwrap();
                if candidate.assessment["resolution"]["outcome"] == "opposition_wins" {
                    unready = Some(candidate);
                    break;
                }
            }
            let unready = unready.expect("Need an unready outcome with the same goalie build");
            assert_eq!(
                serde_json::to_value(&ready_report.input.replay).unwrap(),
                serde_json::to_value(&unready.input.replay).unwrap()
            );
            let high = create(ready_report, 43).unwrap();
            let low = create(unready, 43).unwrap();
            assert!(actions(&high).contains(&retained) && actions(&high).contains(&extra));
            assert!(actions(&low).contains(&retained) && !actions(&low).contains(&extra));
            let a = step(high, retained).unwrap();
            let b = step(low, retained).unwrap();
            assert_eq!(
                a.session["events"][0]["contest"],
                b.session["events"][0]["contest"]
            );
            assert_eq!(
                a.session["events"][0]["resolution"],
                b.session["events"][0]["resolution"]
            );
        }
    }
}
