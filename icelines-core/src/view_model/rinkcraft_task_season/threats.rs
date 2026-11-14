//! Exact, clock-bounded single-threat experiment. No season integration.
use super::{
    game, hash, patterns,
    routes::{capability, Slot},
    table, ScoreRule, Team,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Threat {
    Rush,
    SlotPass,
    LateralPass,
    DumpRetrieve,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub score_rule: ScoreRule,
    /// Authored whole seconds charged before completing each named action.
    pub action_seconds: BTreeMap<String, u16>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum Control {
    Attacking,
    Defending,
    Loose,
    Unknown,
    Dead,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum Zone {
    Neutral,
    Offensive,
}
#[derive(Clone, Copy)]
struct Duel {
    actor: Slot,
    cap: usize,
    opponent: Slot,
    opp_cap: usize,
}
struct Step {
    name: &'static str,
    duel: Option<Duel>,
    success: Control,
    zone: Zone,
    failure: &'static str,
    failure_control: Control,
    tie_control: Control,
}
fn steps(threat: Threat) -> Vec<Step> {
    use Control::*;
    use Slot::*;
    use Zone::*;
    let duel =
        |name, actor, cap, opponent, opp_cap, success, failure, failure_control, tie_control| {
            Step {
                name,
                duel: Some(Duel {
                    actor,
                    cap,
                    opponent,
                    opp_cap,
                }),
                success,
                zone: Offensive,
                failure,
                failure_control,
                tie_control,
            }
        };
    let pass = || {
        duel(
            "slot_connection",
            Creator,
            2,
            Interceptor,
            4,
            Loose,
            "pass_disrupted",
            Loose,
            Loose,
        )
    };
    let receive = || {
        duel(
            "receive",
            Receiver,
            1,
            Gap,
            5,
            Attacking,
            "reception_disrupted",
            Loose,
            Loose,
        )
    };
    let finish = |actor| {
        duel(
            "finish",
            actor,
            3,
            Goalie,
            0,
            Dead,
            "goalie_contact",
            Loose,
            Unknown,
        )
    };
    match threat {
        Threat::Rush => vec![
            duel(
                "entry_duel",
                Creator,
                0,
                Gap,
                0,
                Attacking,
                "entry_disrupted",
                Loose,
                Unknown,
            ),
            duel(
                "protect",
                Creator,
                1,
                Gap,
                5,
                Attacking,
                "control_disrupted",
                Loose,
                Loose,
            ),
            finish(Creator),
        ],
        Threat::SlotPass => vec![pass(), receive(), finish(Receiver)],
        Threat::LateralPass => vec![
            pass(),
            duel(
                "lateral_window",
                Creator,
                2,
                Goalie,
                2,
                Loose,
                "window_closed",
                Loose,
                Loose,
            ),
            receive(),
            finish(Receiver),
        ],
        Threat::DumpRetrieve => vec![
            Step {
                name: "send_deep",
                duel: None,
                success: Loose,
                zone: Offensive,
                failure: "",
                failure_control: Loose,
                tie_control: Loose,
            },
            duel(
                "retrieval_race",
                Forechecker,
                0,
                Interceptor,
                0,
                Loose,
                "access_lost",
                Loose,
                Loose,
            ),
            duel(
                "gather",
                Forechecker,
                1,
                Interceptor,
                1,
                Attacking,
                "defending_collection",
                Defending,
                Loose,
            ),
            duel(
                "retrieval_connection",
                Forechecker,
                2,
                Gap,
                4,
                Loose,
                "pass_disrupted",
                Loose,
                Loose,
            ),
            receive(),
            finish(Receiver),
        ],
    }
}
fn validate(r: &Rules) -> Result<(), String> {
    if r.schema != "rinkcraft.threat-clock-rules.0.1"
        || r.version != "0.1.0-playtest"
        || r.kind != "authored_game_scenario_not_nhl_estimate"
    {
        return Err("Invalid threat rules identity".into());
    }
    let names: BTreeSet<_> = [
        Threat::Rush,
        Threat::SlotPass,
        Threat::LateralPass,
        Threat::DumpRetrieve,
    ]
    .into_iter()
    .flat_map(steps)
    .map(|s| s.name)
    .collect();
    if r.action_seconds.len() != names.len()
        || names.iter().any(|n| !r.action_seconds.contains_key(*n))
        || r.action_seconds.values().any(|&s| s == 0 || s > 30)
    {
        return Err("Need exact positive whole-second action costs, at most 30 each".into());
    }
    Ok(())
}

/// One fixed threat, no retry, policy, route selection or outcome-conditioned
/// lineup change. A contest completes only when its whole cost fits the clock.
pub fn inspect(
    char_rules: &game::GameRules,
    r: &Rules,
    attack: &Team,
    defense: &Team,
    threat: Threat,
    budget: u16,
) -> Result<serde_json::Value, String> {
    validate(r)?;
    if budget == 0
        || budget > 120
        || attack.team.trim().is_empty()
        || defense.team.trim().is_empty()
        || attack.team == defense.team
    {
        return Err("Need distinct named teams and budget 1-120 seconds".into());
    }
    char_rules.validate()?;
    let mut global = BTreeSet::new();
    patterns(attack, char_rules, &mut global)?;
    patterns(defense, char_rules, &mut global)?;
    let am: BTreeMap<_, _> = attack
        .members
        .iter()
        .map(|m| (m.id.as_str(), &m.character))
        .collect();
    let dm: BTreeMap<_, _> = defense
        .members
        .iter()
        .map(|m| (m.id.as_str(), &m.character))
        .collect();
    let table = table(r.score_rule)?;
    let ss = steps(threat);
    let start_zone = if matches!(threat, Threat::Rush | Threat::DumpRetrieve) {
        Zone::Neutral
    } else {
        Zone::Offensive
    };
    // reached, completed, advanced, unresolved, stopped, expired
    let mut gates = vec![[0.0; 6]; ss.len()];
    let mut terminal = BTreeMap::<(String, Control, Zone, u16), f64>::new();
    let mut opportunities = BTreeMap::<(String, String, String), f64>::new();
    for au in &attack.units {
        for du in &defense.units {
            let mut alive = au.weight * du.weight;
            let mut elapsed = 0;
            let mut control = Control::Attacking;
            let mut zone = start_zone;
            for (i, s) in ss.iter().enumerate() {
                let cost = r.action_seconds[s.name];
                gates[i][0] += alive;
                if elapsed + cost > budget {
                    gates[i][5] += alive;
                    *terminal
                        .entry(("time_expired".into(), control, zone, budget))
                        .or_default() += alive;
                    break;
                }
                elapsed += cost;
                gates[i][1] += alive;
                if let Some(d) = s.duel {
                    let aid = d.actor.id(au);
                    let did = d.opponent.id(du);
                    let ac = am[aid].capabilities[d.cap].score as usize;
                    let dc = dm[did].capabilities[d.opp_cap].score as usize;
                    let p = table[ac][dc];
                    for (team, id, name) in [
                        (&attack.team, aid, capability(d.actor, d.cap)),
                        (&defense.team, did, capability(d.opponent, d.opp_cap)),
                    ] {
                        *opportunities
                            .entry((team.clone(), id.into(), name.into()))
                            .or_default() += alive;
                    }
                    gates[i][3] += alive * p[1];
                    gates[i][4] += alive * p[2];
                    *terminal
                        .entry(("unresolved".into(), s.tie_control, zone, elapsed))
                        .or_default() += alive * p[1];
                    *terminal
                        .entry((s.failure.into(), s.failure_control, zone, elapsed))
                        .or_default() += alive * p[2];
                    alive *= p[0];
                }
                gates[i][2] += alive;
                control = s.success;
                zone = s.zone;
                if i + 1 == ss.len() {
                    *terminal
                        .entry(("goal".into(), Control::Dead, zone, elapsed))
                        .or_default() += alive;
                }
            }
        }
    }
    let terminal_rows:Vec<_>=terminal.iter().map(|((outcome,control,zone,elapsed),p)|serde_json::json!({
        "outcome":outcome,"control":control,"zone_in_attack_coordinates":zone,"elapsed_seconds":elapsed,"probability":p
    })).collect();
    let expected_seconds: f64 = terminal
        .iter()
        .map(|((_, _, _, s), p)| f64::from(*s) * p)
        .sum();
    let goal: f64 = terminal
        .iter()
        .filter(|((o, _, _, _), _)| o == "goal")
        .map(|(_, p)| p)
        .sum();
    let expired: f64 = terminal
        .iter()
        .filter(|((o, _, _, _), _)| o == "time_expired")
        .map(|(_, p)| p)
        .sum();
    let opps:Vec<_>=opportunities.into_iter().map(|((team,id,cap),p)|serde_json::json!({"team":team,"member_id":id,"capability":cap,"expected_contests":p})).collect();
    let stage_rows:Vec<_>=ss.iter().zip(&gates).map(|(s,p)|serde_json::json!({"action":s.name,"cost_seconds":r.action_seconds[s.name],
        "contest":s.duel.map(|d|serde_json::json!({"actor_slot":d.actor.name(),"actor_capability":capability(d.actor,d.cap),
            "opponent_slot":d.opponent.name(),"opponent_capability":capability(d.opponent,d.opp_cap)})),
        "success_control":s.success,"reached_probability":p[0],"completed_probability":p[1],"advanced_probability":p[2],
        "unresolved_probability":p[3],"stopped_probability":p[4],"expired_probability":p[5]})).collect();
    Ok(
        serde_json::json!({"schema":"rinkcraft.threat-clock-inspection.0.1","kind":r.kind,
        "input_sha256":hash(&(char_rules,r,attack,defense,threat,budget))?,"rules_sha256":hash(r)?,"seed":null,"trials":null,
        "threat":threat,"budget_seconds":budget,"start_control":"attacking","start_zone":start_zone,
        "goal_probability":goal,"time_expired_probability":expired,"expected_elapsed_seconds":expected_seconds,
        "terminal_states":terminal_rows,"stages":stage_rows,"task_opportunities":opps,
        "limits":["Authored single-threat task and clock rules, not NHL probabilities or measured action durations",
        "Time cost completes before a fresh opposed d20; incomplete actions expire at the last completed boundary state, no contest or completion credit",
        "Race only establishes access; gather explicitly contests control; other disruptions and goalie contact do not award possession",
        "No continuous flight geometry: loose includes declared in-transit puck; zone is last completed boundary in attacking coordinates",
        "Only gather opposition win gives defending control; ties remain unresolved and no partial actions are inferred",
        "Fixed initial independent unit mixtures; no retries, threat selection policy, full-match clock, fatigue, changes, PP/PK or season integration",
        "Terminal state is a continuation boundary only; no following route is automatically selected",
        "Cross-threat comparisons have different starting states, clocks and gate counts; no optimal policy or balance claim"]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (game::GameRules, Rules, Team, Team) {
        let cr = game::GameRules {
            schema: game::RULES_SCHEMA.into(),
            version: "test".into(),
            title: "Threat test".into(),
            standard_array: [17, 16, 15, 13, 11, 9],
            templates: [
                ("skater", game::Branch::Skater),
                ("goalie", game::Branch::Goalie),
            ]
            .into_iter()
            .map(|(id, branch)| game::GameTemplate {
                id: id.into(),
                branch,
                class_id: id.into(),
                class_name: id.into(),
                specialty: "Test".into(),
                description: "Test".into(),
                priority: [0, 1, 2, 3, 4, 5],
                task_skills: vec![],
                methods: vec![],
                tendencies: vec![],
                playbook_options: vec![],
            })
            .collect(),
        };
        let team = |name: &str| {
            let id = |i| format!("{name}:{i}");
            Team {
                team: name.into(),
                members: (0..6)
                    .map(|i| super::super::Member {
                        id: id(i),
                        character: game::build(
                            &cr,
                            game::BuildRequest {
                                template_id: if i == 5 { "goalie" } else { "skater" }.into(),
                                name: id(i),
                                position: if i == 5 {
                                    "G"
                                } else if i >= 3 {
                                    "D"
                                } else {
                                    "F"
                                }
                                .into(),
                                scores: game::ScoreMethod::Manual { scores: [10; 6] },
                                inspiration: None,
                            },
                        )
                        .unwrap(),
                    })
                    .collect(),
                units: vec![super::super::Unit {
                    id: "unit".into(),
                    weight: 1.0,
                    forwards: [id(0), id(1), id(2)],
                    defense: [id(3), id(4)],
                    goalie: id(5),
                    creator: id(0),
                    receiver: id(1),
                    forechecker: id(2),
                    interceptor: id(3),
                    gap_defender: id(4),
                }],
            }
        };
        let a = team("Alpha");
        let b = team("Beta");
        let r = Rules {
            schema: "rinkcraft.threat-clock-rules.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: "authored_game_scenario_not_nhl_estimate".into(),
            score_rule: ScoreRule::HalfOffset,
            action_seconds: [
                ("entry_duel", 3),
                ("protect", 2),
                ("slot_connection", 2),
                ("receive", 1),
                ("finish", 1),
                ("lateral_window", 1),
                ("send_deep", 1),
                ("retrieval_race", 3),
                ("gather", 2),
                ("retrieval_connection", 2),
            ]
            .into_iter()
            .map(|(n, s)| (n.into(), s))
            .collect(),
        };
        (cr, r, a, b)
    }
    fn num(v: &serde_json::Value, k: &str) -> f64 {
        v[k].as_f64().unwrap()
    }
    fn change(cr: &game::GameRules, t: &mut Team, i: usize, cap: usize) {
        let c = &t.members[i].character;
        let mut scores = [10; 6];
        scores[cap] = 16;
        t.members[i].character = game::build(
            cr,
            game::BuildRequest {
                template_id: c.template_id.clone(),
                name: c.name.clone(),
                position: c.position.clone(),
                scores: game::ScoreMethod::Manual { scores },
                inspiration: None,
            },
        )
        .unwrap();
    }
    #[test]
    fn mass_clock_and_task_exposure_conserve_at_every_budget() {
        let (cr, r, a, b) = fixture();
        for threat in [
            Threat::Rush,
            Threat::SlotPass,
            Threat::LateralPass,
            Threat::DumpRetrieve,
        ] {
            for budget in 1..=12 {
                let v = inspect(&cr, &r, &a, &b, threat, budget).unwrap();
                let terminals = v["terminal_states"].as_array().unwrap();
                assert!(
                    (terminals.iter().map(|x| num(x, "probability")).sum::<f64>() - 1.0).abs()
                        < 1e-12
                );
                let elapsed = terminals
                    .iter()
                    .map(|x| num(x, "probability") * num(x, "elapsed_seconds"))
                    .sum::<f64>();
                assert!((elapsed - num(&v, "expected_elapsed_seconds")).abs() < 1e-12);
                assert!(elapsed <= f64::from(budget) + 1e-12);
                let mut alive = 1.0;
                let mut count = 0.0;
                for s in v["stages"].as_array().unwrap() {
                    assert!((num(s, "reached_probability") - alive).abs() < 1e-12);
                    assert!(
                        (num(s, "completed_probability") + num(s, "expired_probability") - alive)
                            .abs()
                            < 1e-12
                    );
                    assert!(
                        (num(s, "advanced_probability")
                            + num(s, "unresolved_probability")
                            + num(s, "stopped_probability")
                            - num(s, "completed_probability"))
                        .abs()
                            < 1e-12
                    );
                    if !s["contest"].is_null() {
                        count += 2.0 * num(s, "completed_probability");
                    }
                    alive = if num(s, "expired_probability") > 0.0 {
                        0.0
                    } else {
                        num(s, "advanced_probability")
                    };
                }
                let opps = v["task_opportunities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| num(x, "expected_contests"))
                    .sum::<f64>();
                assert!((count - opps).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn incomplete_action_has_no_roll_and_keeps_last_boundary_state() {
        let (cr, r, a, b) = fixture();
        let v = inspect(&cr, &r, &a, &b, Threat::Rush, 2).unwrap();
        assert_eq!(num(&v, "time_expired_probability"), 1.0);
        assert!(v["task_opportunities"].as_array().unwrap().is_empty());
        assert_eq!(v["terminal_states"][0]["control"], "attacking");
        assert_eq!(
            v["terminal_states"][0]["zone_in_attack_coordinates"],
            "neutral"
        );
        let v = inspect(&cr, &r, &a, &b, Threat::DumpRetrieve, 3).unwrap();
        assert_eq!(num(&v, "time_expired_probability"), 1.0);
        assert_eq!(v["terminal_states"][0]["control"], "loose");
        assert_eq!(
            v["terminal_states"][0]["zone_in_attack_coordinates"],
            "offensive"
        );
    }
    #[test]
    fn clock_boundary_completes_finish_and_goals_have_dead_puck() {
        let (cr, r, a, b) = fixture();
        let no = inspect(&cr, &r, &a, &b, Threat::SlotPass, 3).unwrap();
        let yes = inspect(&cr, &r, &a, &b, Threat::SlotPass, 4).unwrap();
        assert_eq!(num(&no, "goal_probability"), 0.0);
        assert!((num(&yes, "goal_probability") - 0.475_f64.powi(3)).abs() < 1e-12);
        assert_eq!(num(&yes, "time_expired_probability"), 0.0);
        assert_eq!(yes["stages"][2]["success_control"], "dead");
        for x in yes["terminal_states"].as_array().unwrap() {
            if x["outcome"] == "goal" {
                assert_eq!(x["control"], "dead");
            }
        }
    }
    #[test]
    fn race_never_awards_control_and_gather_is_explicit() {
        let (cr, r, a, b) = fixture();
        let race = inspect(&cr, &r, &a, &b, Threat::DumpRetrieve, 4).unwrap();
        for x in race["terminal_states"].as_array().unwrap() {
            assert_eq!(x["control"], "loose");
        }
        let gather = inspect(&cr, &r, &a, &b, Threat::DumpRetrieve, 6).unwrap();
        let p = gather["terminal_states"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["outcome"] == "defending_collection")
            .unwrap();
        assert_eq!(p["control"], "defending");
        assert!((num(p, "probability") - 0.475 * 0.475).abs() < 1e-12);
        let held = gather["terminal_states"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["outcome"] == "time_expired")
            .unwrap();
        assert_eq!(held["control"], "attacking");
    }
    #[test]
    fn defensive_skating_and_goalie_movement_are_threat_specific() {
        let (cr, r, a, mut b) = fixture();
        for threat in [Threat::Rush, Threat::SlotPass] {
            let x = inspect(&cr, &r, &a, &b, threat, 12).unwrap();
            let mut changed = b.clone();
            change(&cr, &mut changed, 4, 0);
            let y = inspect(&cr, &r, &a, &changed, threat, 12).unwrap();
            if threat == Threat::Rush {
                assert!(num(&y, "goal_probability") < num(&x, "goal_probability"));
            } else {
                assert_eq!(x["goal_probability"], y["goal_probability"]);
            }
        }
        let x = inspect(&cr, &r, &a, &b, Threat::LateralPass, 12).unwrap();
        change(&cr, &mut b, 5, 2);
        let y = inspect(&cr, &r, &a, &b, Threat::LateralPass, 12).unwrap();
        assert!(num(&y, "goal_probability") < num(&x, "goal_probability"));
        let z = inspect(&cr, &r, &a, &b, Threat::SlotPass, 12).unwrap();
        assert!((num(&z, "goal_probability") - 0.475_f64.powi(3)).abs() < 1e-12);
    }
    #[test]
    fn invalid_costs_clocks_and_forged_characters_reject() {
        let (cr, mut r, mut a, b) = fixture();
        assert!(inspect(&cr, &r, &a, &b, Threat::Rush, 0).is_err());
        assert!(inspect(&cr, &r, &a, &b, Threat::Rush, 121).is_err());
        r.action_seconds.insert("finish".into(), 0);
        assert!(inspect(&cr, &r, &a, &b, Threat::Rush, 12).is_err());
        r.action_seconds.insert("finish".into(), 1);
        r.action_seconds.insert("extra".into(), 1);
        assert!(inspect(&cr, &r, &a, &b, Threat::Rush, 12).is_err());
        r.action_seconds.remove("extra");
        a.members[0].character.capabilities[0].score = 20;
        assert!(inspect(&cr, &r, &a, &b, Threat::Rush, 12).is_err());
    }
}
