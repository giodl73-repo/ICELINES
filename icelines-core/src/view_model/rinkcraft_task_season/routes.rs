//! Isolated exact route experiment. Does not supply season probabilities.
use super::{game, hash, patterns, table, ScoreRule, Team};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const KIND: &str = "authored_game_scenario_not_nhl_estimate";
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteKind {
    LegacyOutletPass,
    OutletPass,
    Carry,
    ZonePass,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteWeight {
    pub route: RouteKind,
    pub weight: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mixture {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub routes: Vec<RouteWeight>,
}

// Slot resolves to an actual member on each selected six-player unit. A defender
// can be creator/receiver. Carry uses the creator as its shooter: no implicit pass.
#[derive(Clone, Copy)]
pub(super) enum Slot {
    Creator,
    Receiver,
    Goalie,
    Forechecker,
    Interceptor,
    Gap,
}
impl Slot {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Creator => "creator",
            Self::Receiver => "receiver",
            Self::Goalie => "goalie",
            Self::Forechecker => "forechecker",
            Self::Interceptor => "interceptor",
            Self::Gap => "gap_defender",
        }
    }
    pub(super) fn id(self, unit: &super::Unit) -> &str {
        match self {
            Self::Creator => &unit.creator,
            Self::Receiver => &unit.receiver,
            Self::Goalie => &unit.goalie,
            Self::Forechecker => &unit.forechecker,
            Self::Interceptor => &unit.interceptor,
            Self::Gap => &unit.gap_defender,
        }
    }
}
#[derive(Clone, Copy)]
struct Stage {
    name: &'static str,
    actor: Slot,
    cap: usize,
    opponent: Slot,
    opp_cap: usize,
}
fn stages(route: RouteKind) -> Vec<Stage> {
    use Slot::*;
    let outlet = Stage {
        name: "outlet",
        actor: Goalie,
        cap: 5,
        opponent: Forechecker,
        opp_cap: 5,
    };
    let pass = Stage {
        name: "creation",
        actor: Creator,
        cap: 2,
        opponent: Interceptor,
        opp_cap: 4,
    };
    let receive = Stage {
        name: "reception",
        actor: Receiver,
        cap: 1,
        opponent: Interceptor,
        opp_cap: 5,
    };
    let lane = |actor| Stage {
        name: "lane_read",
        actor,
        cap: 4,
        opponent: Gap,
        opp_cap: 5,
    };
    let finish = |actor| Stage {
        name: "finish",
        actor,
        cap: 3,
        opponent: Goalie,
        opp_cap: 0,
    };
    match route {
        RouteKind::LegacyOutletPass => vec![
            outlet,
            pass,
            receive,
            Stage {
                name: "release",
                actor: Receiver,
                cap: 3,
                opponent: Gap,
                opp_cap: 5,
            },
            finish(Receiver),
        ],
        RouteKind::OutletPass => vec![outlet, pass, receive, lane(Receiver), finish(Receiver)],
        RouteKind::ZonePass => vec![pass, receive, lane(Receiver), finish(Receiver)],
        RouteKind::Carry => vec![
            Stage {
                name: "entry",
                actor: Creator,
                cap: 0,
                opponent: Gap,
                opp_cap: 5,
            },
            Stage {
                name: "protection",
                actor: Creator,
                cap: 1,
                opponent: Interceptor,
                opp_cap: 4,
            },
            lane(Creator),
            finish(Creator),
        ],
    }
}
pub(super) fn capability(slot: Slot, cap: usize) -> &'static str {
    if matches!(slot, Slot::Goalie) {
        [
            "Tracking",
            "Positioning",
            "Movement",
            "Rebounds",
            "Recovery",
            "Puck Play",
        ][cap]
    } else {
        [
            "Skating",
            "Handling",
            "Passing",
            "Shooting",
            "Reading",
            "Defending",
        ][cap]
    }
}

/// Exact terminal masses and probability-weighted task opportunities. Route
/// selection and own/opposing unit mixtures are authored and independent.
pub fn inspect(
    rules: &game::GameRules,
    score_rule: ScoreRule,
    attack: &Team,
    defense: &Team,
    mixture: &Mixture,
) -> Result<serde_json::Value, String> {
    if mixture.schema != "rinkcraft.route-mixture.0.1"
        || mixture.version != "0.1.0-playtest"
        || mixture.kind != KIND
        || mixture.routes.is_empty()
        || mixture.routes.len() > 4
    {
        return Err("Invalid route mixture identity or size".into());
    }
    let mut kinds = BTreeSet::new();
    let mut total = 0.0;
    for r in &mixture.routes {
        if !kinds.insert(r.route) || !r.weight.is_finite() || r.weight <= 0.0 || r.weight > 1.0 {
            return Err("Invalid/duplicate route or weight".into());
        }
        total += r.weight;
    }
    if (total - 1.0).abs() > 1e-9 {
        return Err("Route weights must sum to one".into());
    }
    if attack.team.trim().is_empty()
        || defense.team.trim().is_empty()
        || attack.team == defense.team
    {
        return Err("Need distinct named route teams".into());
    }
    rules.validate()?;
    let mut global = BTreeSet::new();
    patterns(attack, rules, &mut global)?;
    patterns(defense, rules, &mut global)?;
    let t = table(score_rule)?;
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
    let mut routes = Vec::new();
    let mut exposure = BTreeMap::<(String, String, usize), f64>::new();
    let mut mixed = [0.0; 3]; // goal, unresolved, stopped
    let mut neutral = 0.0;
    for r in &mixture.routes {
        let ss = stages(r.route);
        let mut gate = vec![[0.0; 4]; ss.len()]; // reached, advance, unresolved, stopped
        let mut terminal = [0.0; 3];
        for au in &attack.units {
            for du in &defense.units {
                let w = au.weight * du.weight;
                let mut alive = w;
                for (i, s) in ss.iter().enumerate() {
                    let aid = s.actor.id(au);
                    let did = s.opponent.id(du);
                    let ac = am[aid].capabilities[s.cap].score as usize;
                    let dc = dm[did].capabilities[s.opp_cap].score as usize;
                    let p = t[ac][dc];
                    gate[i][0] += alive;
                    for (team, id, cap) in
                        [(&attack.team, aid, s.cap), (&defense.team, did, s.opp_cap)]
                    {
                        *exposure.entry((team.clone(), id.into(), cap)).or_default() +=
                            r.weight * alive;
                    }
                    gate[i][1] += alive * p[0];
                    gate[i][2] += alive * p[1];
                    gate[i][3] += alive * p[2];
                    terminal[1] += alive * p[1];
                    terminal[2] += alive * p[2];
                    alive *= p[0];
                }
                terminal[0] += alive;
            }
        }
        let n = t[10][10][0].powi(ss.len() as i32);
        neutral += r.weight * n;
        for i in 0..3 {
            mixed[i] += r.weight * terminal[i];
        }
        let audit:Vec<_>=ss.iter().zip(gate).map(|(s,p)|serde_json::json!({
            "task":s.name,"actor_slot":s.actor.name(),"actor_capability":capability(s.actor,s.cap),
            "opponent_slot":s.opponent.name(),"opponent_capability":capability(s.opponent,s.opp_cap),
            "reached_probability":p[0],"advance_probability":p[1],"unresolved_probability":p[2],"stopped_probability":p[3],
            "stop_meaning":if s.name=="finish" {"goalie_save_contact"} else {"task_disruption"}
        })).collect();
        routes.push(serde_json::json!({"route":r.route,"selection_weight":r.weight,"stages":audit,
            "goal_probability":terminal[0],"unresolved_probability":terminal[1],"stopped_probability":terminal[2],
            "neutral_goal_probability":n,"weighted_goal_probability":r.weight*terminal[0]}));
    }
    let opportunities:Vec<_>=exposure.into_iter().map(|((team,id,cap),v)|serde_json::json!({
        "team":team,"member_id":id,"capability_index":cap,"expected_contests_per_selected_window":v
    })).collect();
    Ok(
        serde_json::json!({"schema":"rinkcraft.route-inspection.0.1","kind":KIND,
        "input_sha256":hash(&(rules,score_rule,attack,defense,mixture))?,"mixture":mixture,
        "seed":null,"trials":null,"routes":routes,"task_opportunities":opportunities,
        "goal_probability":mixed[0],"unresolved_probability":mixed[1],"stopped_probability":mixed[2],
        "neutral_goal_probability":neutral,"relative_to_same_mixture_neutral":mixed[0]/neutral,
        "limits":["Authored independent d20 gates; not empirical NHL probabilities or a season forecast",
        "Mixture selects a starting context, not an optimal policy; route length, clock and entry costs are not equalized",
        "No per-route goal-rate normalization: shorter routes naturally have higher neutral success",
        "One bounded window: ties terminate unresolved; disruptions/save contact do not award possession or official statistics",
        "Lane Reading replaces the legacy Shooting release only in new routes; this is an authored alternative, not a validated skill mapping",
        "No dump retrieval, rebounds, traffic, lateral readiness, PP/PK, fatigue or adaptive choices; unused capabilities remain unused",
        "Carry creator is also shooter; no implicit pass to receiver or automatic class/chemistry bonus",
        "Opportunity counts are probability-weighted contests, not TOI, usage rates or capability ratings"]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (game::GameRules, Team, Team) {
        let rules = game::GameRules {
            schema: game::RULES_SCHEMA.into(),
            version: "test".into(),
            title: "Route test".into(),
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
                description: "Authored test".into(),
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
                            &rules,
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
        (rules, a, b)
    }
    fn mix(routes: &[(RouteKind, f64)]) -> Mixture {
        Mixture {
            schema: "rinkcraft.route-mixture.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: KIND.into(),
            routes: routes
                .iter()
                .map(|&(route, weight)| RouteWeight { route, weight })
                .collect(),
        }
    }
    fn num(v: &serde_json::Value, k: &str) -> f64 {
        v[k].as_f64().unwrap()
    }
    fn change(r: &game::GameRules, t: &mut Team, i: usize, scores: [u8; 6]) {
        let c = &t.members[i].character;
        t.members[i].character = game::build(
            r,
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
    fn terminal_and_reach_ledgers_conserve_for_all_routes() {
        let (r, a, b) = fixture();
        for kind in [
            RouteKind::LegacyOutletPass,
            RouteKind::OutletPass,
            RouteKind::Carry,
            RouteKind::ZonePass,
        ] {
            let v = inspect(&r, ScoreRule::HalfOffset, &a, &b, &mix(&[(kind, 1.0)])).unwrap();
            assert!(
                (num(&v, "goal_probability")
                    + num(&v, "unresolved_probability")
                    + num(&v, "stopped_probability")
                    - 1.0)
                    .abs()
                    < 1e-12
            );
            assert!((num(&v, "relative_to_same_mixture_neutral") - 1.0).abs() < 1e-12);
            let mut reached = 1.0;
            for s in v["routes"][0]["stages"].as_array().unwrap() {
                assert!((num(s, "reached_probability") - reached).abs() < 1e-12);
                assert!(
                    (num(s, "advance_probability")
                        + num(s, "unresolved_probability")
                        + num(s, "stopped_probability")
                        - reached)
                        .abs()
                        < 1e-12
                );
                reached = num(s, "advance_probability");
            }
            assert!((reached - num(&v, "goal_probability")).abs() < 1e-12);
        }
    }
    #[test]
    fn legacy_goal_matches_original_route_with_non_neutral_scores() {
        let (r, mut a, mut b) = fixture();
        change(&r, &mut a, 5, [10, 10, 10, 10, 10, 14]);
        change(&r, &mut a, 0, [10, 10, 16, 10, 10, 10]);
        change(&r, &mut a, 1, [10, 12, 10, 18, 10, 10]);
        change(&r, &mut b, 2, [10, 10, 10, 10, 10, 12]);
        change(&r, &mut b, 3, [10, 10, 10, 10, 14, 16]);
        change(&r, &mut b, 4, [10, 10, 10, 10, 10, 18]);
        change(&r, &mut b, 5, [16, 10, 10, 10, 10, 10]);
        let old = super::super::route(
            [14, 16, 12, 18, 10, 10, 10, 10, 10],
            [10, 10, 10, 10, 12, 14, 16, 18, 16],
            &table(ScoreRule::HalfOffset).unwrap(),
        );
        let v = inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &b,
            &mix(&[(RouteKind::LegacyOutletPass, 1.0)]),
        )
        .unwrap();
        assert!((num(&v, "goal_probability") - old.2).abs() < 1e-14);
        assert!((num(&v, "unresolved_probability") - old.1).abs() < 1e-14);
    }
    #[test]
    fn mix_is_weighted_probability_not_average_scores_or_route_normalization() {
        let (r, a, b) = fixture();
        let one = inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &b,
            &mix(&[(RouteKind::OutletPass, 1.0)]),
        )
        .unwrap();
        let two = inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &b,
            &mix(&[(RouteKind::Carry, 1.0)]),
        )
        .unwrap();
        let v = inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &b,
            &mix(&[(RouteKind::OutletPass, 0.2), (RouteKind::Carry, 0.8)]),
        )
        .unwrap();
        assert!(
            (num(&v, "goal_probability")
                - 0.2 * num(&one, "goal_probability")
                - 0.8 * num(&two, "goal_probability"))
            .abs()
                < 1e-12
        );
        assert!(num(&two, "goal_probability") > num(&one, "goal_probability"));
        let outlet = v["task_opportunities"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["member_id"] == "Alpha:5")
            .unwrap();
        assert!((num(outlet, "expected_contests_per_selected_window") - 0.2).abs() < 1e-12);
    }
    #[test]
    fn carry_uses_creator_as_shooter_without_receiver_or_goalie_outlet() {
        let (r, mut a, b) = fixture();
        let m = mix(&[(RouteKind::Carry, 1.0)]);
        let v = inspect(&r, ScoreRule::HalfOffset, &a, &b, &m).unwrap();
        let opportunities = v["task_opportunities"].as_array().unwrap();
        assert!(!opportunities
            .iter()
            .any(|x| x["member_id"] == "Alpha:1" || x["member_id"] == "Alpha:5"));
        change(&r, &mut a, 1, [20; 6]);
        let w = inspect(&r, ScoreRule::HalfOffset, &a, &b, &m).unwrap();
        assert_eq!(v["goal_probability"], w["goal_probability"]);
        change(&r, &mut a, 0, [16, 10, 10, 10, 10, 10]);
        let w = inspect(&r, ScoreRule::HalfOffset, &a, &b, &m).unwrap();
        assert!(num(&w, "goal_probability") > num(&v, "goal_probability"));
    }
    #[test]
    fn new_pass_uses_shooting_once_and_reading_before_finish() {
        let (r, mut a, b) = fixture();
        let original = inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &b,
            &mix(&[(RouteKind::OutletPass, 1.0)]),
        )
        .unwrap();
        for (kind, count) in [(RouteKind::LegacyOutletPass, 2), (RouteKind::OutletPass, 1)] {
            let v = inspect(&r, ScoreRule::HalfOffset, &a, &b, &mix(&[(kind, 1.0)])).unwrap();
            assert_eq!(
                v["routes"][0]["stages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x| x["actor_capability"] == "Shooting")
                    .count(),
                count
            );
        }
        change(&r, &mut a, 1, [10, 10, 10, 10, 16, 10]);
        let v = inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &b,
            &mix(&[(RouteKind::OutletPass, 1.0)]),
        )
        .unwrap();
        assert!(num(&v, "goal_probability") > num(&original, "goal_probability"));
    }
    #[test]
    fn rejects_bad_weights_identity_and_forged_characters() {
        let (r, mut a, b) = fixture();
        for bad in [
            mix(&[]),
            mix(&[(RouteKind::Carry, 0.5)]),
            mix(&[(RouteKind::Carry, 0.5), (RouteKind::Carry, 0.5)]),
            mix(&[(RouteKind::Carry, f64::NAN)]),
        ] {
            assert!(inspect(&r, ScoreRule::HalfOffset, &a, &b, &bad).is_err());
        }
        assert!(inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &a,
            &mix(&[(RouteKind::Carry, 1.0)])
        )
        .is_err());
        a.members[0].character.capabilities[0].score = 20;
        assert!(inspect(
            &r,
            ScoreRule::HalfOffset,
            &a,
            &b,
            &mix(&[(RouteKind::Carry, 1.0)])
        )
        .is_err());
    }
    #[test]
    fn defenders_can_be_carriers_and_unused_capabilities_have_no_effect() {
        let (r, mut a, b) = fixture();
        a.units[0].creator = "Alpha:3".into();
        let m = mix(&[(RouteKind::Carry, 1.0)]);
        let v = inspect(&r, ScoreRule::HalfOffset, &a, &b, &m).unwrap();
        change(&r, &mut a, 3, [16, 10, 10, 10, 10, 10]);
        let w = inspect(&r, ScoreRule::HalfOffset, &a, &b, &m).unwrap();
        assert!(num(&w, "goal_probability") > num(&v, "goal_probability"));
        change(&r, &mut a, 5, [10, 10, 10, 20, 10, 10]);
        let u = inspect(&r, ScoreRule::HalfOffset, &a, &b, &m).unwrap();
        assert_eq!(w["goal_probability"], u["goal_probability"]);
    }
}
