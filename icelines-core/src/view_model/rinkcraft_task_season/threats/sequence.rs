//! Exact controlled follow-ups on fixed units and one shared clock.
use super::*;
pub mod access;
pub mod matchups;
pub mod recovery;
pub mod support;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    ShootHolder,
    ConnectThenShoot,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HolderChoice {
    pub member_id: String,
    pub choice: Choice,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamPolicy {
    pub team: String,
    pub default: Choice,
    pub overrides: Vec<HolderChoice>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HolderPolicies {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub first: TeamPolicy,
    pub second: TeamPolicy,
}
impl HolderPolicies {
    fn validate(&self, a: &Team, b: &Team, p: &Plan) -> Result<(), String> {
        if self.schema != "rinkcraft.holder-policies.0.1"
            || self.version != "0.1.0-playtest"
            || self.kind != "authored_game_scenario_not_nhl_estimate"
            || self.first.default != p.first_policy
            || self.second.default != p.second_policy
        {
            return Err("Invalid holder-policy identity or conflicting plan defaults".into());
        }
        for (policy, team) in [(&self.first, a), (&self.second, b)] {
            if policy.team != team.team || policy.overrides.len() > 40 {
                return Err("Holder policy team/size mismatch".into());
            }
            let mut seen = BTreeSet::new();
            for entry in &policy.overrides {
                if !seen.insert(&entry.member_id)
                    || !team.members.iter().any(|m| {
                        m.id == entry.member_id && m.character.branch == game::Branch::Skater
                    })
                {
                    return Err("Unknown/duplicate/non-skater holder override".into());
                }
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub opening: super::Rules,
    pub exit_seconds: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> (game::GameRules, Rules, Team, Team, Plan) {
        let (cr, r, a, b) = super::super::tests::fixture();
        (
            cr,
            Rules {
                schema: "rinkcraft.control-sequence-rules.0.1".into(),
                version: "0.1.0-playtest".into(),
                kind: r.kind.clone(),
                opening: r,
                exit_seconds: 3,
            },
            a,
            b,
            Plan {
                opening: Threat::DumpRetrieve,
                completed_prefix: 3,
                budget_seconds: 16,
                first_policy: Choice::ShootHolder,
                second_policy: Choice::ShootHolder,
            },
        )
    }
    fn num(v: &serde_json::Value, k: &str) -> f64 {
        v[k].as_f64().unwrap()
    }
    fn policies(a: &Team, b: &Team, p: &Plan) -> HolderPolicies {
        HolderPolicies {
            schema: "rinkcraft.holder-policies.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: "authored_game_scenario_not_nhl_estimate".into(),
            first: TeamPolicy {
                team: a.team.clone(),
                default: p.first_policy,
                overrides: vec![],
            },
            second: TeamPolicy {
                team: b.team.clone(),
                default: p.second_policy,
                overrides: vec![],
            },
        }
    }
    #[test]
    fn empty_policy_overrides_preserve_existing_mechanics_and_old_shape() {
        let (cr, r, a, b, p) = fixture();
        let ps = policies(&a, &b, &p);
        let old = inspect(&cr, &r, &a, &b, &p).unwrap();
        let new = inspect_with_policies(&cr, &r, &a, &b, &p, &ps).unwrap();
        for key in [
            "terminal_states",
            "action_audits",
            "task_opportunities",
            "goal_probability_by_team",
            "terminal_control_probability_by_team",
            "expected_elapsed_seconds",
            "control_switch_probability",
        ] {
            assert_eq!(old[key], new[key], "{key}");
        }
        assert!(old.get("holder_policies").is_none());
        assert_ne!(old["input_sha256"], new["input_sha256"]);
        assert_eq!(
            new,
            inspect_with_policies(&cr, &r, &a, &b, &p, &ps).unwrap()
        );
    }
    #[test]
    fn override_follows_actual_holder_and_receiver_guard_is_audited() {
        let (cr, r, a, b, p) = fixture();
        let mut ps = policies(&a, &b, &p);
        ps.first.overrides = vec![
            HolderChoice {
                member_id: "Alpha:2".into(),
                choice: Choice::ConnectThenShoot,
            },
            HolderChoice {
                member_id: "Alpha:1".into(),
                choice: Choice::ConnectThenShoot,
            },
        ];
        let v = inspect_with_policies(&cr, &r, &a, &b, &p, &ps).unwrap();
        let ds = v["policy_decisions"].as_array().unwrap();
        assert!(ds.iter().any(|x| x["holder_id"] == "Alpha:2"
            && x["selected_action"] == "connect_holder"
            && x["selection_source"] == "member_override"));
        assert!(ds.iter().any(|x| x["holder_id"] == "Alpha:1"
            && x["self_pass_guard"] == true
            && x["selected_action"] == "shoot_holder"));
        assert!(ds
            .iter()
            .filter(|x| x["team"] == "Beta")
            .all(|x| x["selection_source"] == "team_default"));
        let mut short = p;
        short.budget_seconds = 6;
        let v = inspect_with_policies(&cr, &r, &a, &b, &short, &ps).unwrap();
        assert!(v["policy_decisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["holder_id"] == "Alpha:2" && x["remaining_seconds"] == 0));
        assert!(v["action_audits"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["action"] == "connect_holder")
            .all(|x| num(x, "completed_probability") == 0.0));
    }
    #[test]
    fn declared_specialist_policy_changes_fit_without_a_bonus() {
        let (cr, r, mut a, b, p) = fixture();
        for (i, scores) in [(2, [10, 10, 20, 1, 10, 10]), (1, [10, 20, 10, 20, 10, 10])] {
            let c = &a.members[i].character;
            a.members[i].character = game::build(
                &cr,
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
        let base = inspect(&cr, &r, &a, &b, &p).unwrap();
        let mut ps = policies(&a, &b, &p);
        ps.first.overrides.push(HolderChoice {
            member_id: "Alpha:2".into(),
            choice: Choice::ConnectThenShoot,
        });
        let v = inspect_with_policies(&cr, &r, &a, &b, &p, &ps).unwrap();
        assert!(
            v["goal_probability_by_team"]["Alpha"].as_f64().unwrap()
                > base["goal_probability_by_team"]["Alpha"].as_f64().unwrap()
        );
        assert_eq!(
            v["goal_probability_by_team"]["Beta"],
            base["goal_probability_by_team"]["Beta"]
        );
        assert_eq!(
            v["control_switch_probability"],
            base["control_switch_probability"]
        );
    }
    #[test]
    fn policy_identity_duplicates_unknowns_and_goalies_reject() {
        let (cr, r, a, b, p) = fixture();
        let base = policies(&a, &b, &p);
        for id in ["missing", "Beta:3", "Alpha:5"] {
            let mut ps = base.clone();
            ps.first.overrides.push(HolderChoice {
                member_id: id.into(),
                choice: Choice::ConnectThenShoot,
            });
            assert!(inspect_with_policies(&cr, &r, &a, &b, &p, &ps).is_err());
        }
        let mut ps = base.clone();
        ps.first.overrides = vec![
            HolderChoice {
                member_id: "Alpha:2".into(),
                choice: Choice::ConnectThenShoot
            };
            2
        ];
        assert!(inspect_with_policies(&cr, &r, &a, &b, &p, &ps).is_err());
        let mut ps = base.clone();
        ps.first.default = Choice::ConnectThenShoot;
        assert!(inspect_with_policies(&cr, &r, &a, &b, &p, &ps).is_err());
        let mut ps = base;
        ps.second.team = "Alpha".into();
        assert!(inspect_with_policies(&cr, &r, &a, &b, &p, &ps).is_err());
    }
    #[test]
    fn terminal_clock_action_and_task_ledgers_conserve() {
        let (cr, r, a, b, mut p) = fixture();
        for budget in 1..=16 {
            p.budget_seconds = budget;
            let v = inspect(&cr, &r, &a, &b, &p).unwrap();
            let ts = v["terminal_states"].as_array().unwrap();
            assert!((ts.iter().map(|x| num(x, "probability")).sum::<f64>() - 1.0).abs() < 1e-12);
            let seconds = ts
                .iter()
                .map(|x| num(x, "probability") * num(x, "elapsed_seconds"))
                .sum::<f64>();
            assert!((seconds - num(&v, "expected_elapsed_seconds")).abs() < 1e-12);
            let mut charged = 0.0;
            let mut contests = 0.0;
            for x in v["action_audits"].as_array().unwrap() {
                let reached = num(x, "reached_probability");
                let completed = num(x, "completed_probability");
                assert!((completed + num(x, "expired_probability") - reached).abs() < 1e-12);
                assert!(
                    (num(x, "win_probability")
                        + num(x, "tie_probability")
                        + num(x, "loss_probability")
                        - completed)
                        .abs()
                        < 1e-12
                );
                charged += completed * num(x, "cost_seconds")
                    + num(x, "expired_probability") * (f64::from(budget) - num(x, "start_seconds"));
                if x["action"] != "send_deep" {
                    contests += 2.0 * completed;
                }
            }
            assert!((charged - seconds).abs() < 1e-12);
            assert!(
                (v["task_opportunities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| num(x, "expected_contests"))
                    .sum::<f64>()
                    - contests)
                    .abs()
                    < 1e-12
            );
        }
    }
    #[test]
    fn collection_changes_actor_and_requires_exit_before_counterattack() {
        let (cr, r, a, b, p) = fixture();
        let v = inspect(&cr, &r, &a, &b, &p).unwrap();
        let actions = v["action_audits"].as_array().unwrap();
        for x in actions.iter().filter(|x| x["team"] == "Beta") {
            if x["action"] == "exit_holder" {
                assert_eq!(x["actor_id"], "Beta:3");
                assert_eq!(x["start_seconds"], 6);
            }
            if x["action"] == "advance_holder" {
                assert_eq!(x["actor_id"], "Beta:3");
                assert_eq!(x["start_seconds"], 9);
            }
            if x["action"] == "shoot_holder" {
                assert_eq!(x["actor_id"], "Beta:3");
                assert_eq!(x["start_seconds"], 12);
            }
        }
        assert!(
            (v["goal_probability_by_team"]["Alpha"].as_f64().unwrap() - 0.475_f64.powi(3)).abs()
                < 1e-12
        );
        assert!(
            (v["goal_probability_by_team"]["Beta"].as_f64().unwrap() - 0.475_f64.powi(5)).abs()
                < 1e-12
        );
        for x in v["terminal_states"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["goal_team"] == "Beta")
        {
            assert_eq!(x["zone_offensive_for"], "Beta");
        }
    }
    #[test]
    fn six_second_boundary_preserves_both_collectors_without_reset() {
        let (cr, r, a, b, mut p) = fixture();
        p.budget_seconds = 6;
        let v = inspect(&cr, &r, &a, &b, &p).unwrap();
        for team in ["Alpha", "Beta"] {
            assert_eq!(v["goal_probability_by_team"][team], 0.0);
            assert!(
                (v["terminal_control_probability_by_team"][team]
                    .as_f64()
                    .unwrap()
                    - 0.225625)
                    .abs()
                    < 1e-12
            );
        }
        let ts = v["terminal_states"].as_array().unwrap();
        assert!(ts
            .iter()
            .any(|x| x["holder_id"] == "Alpha:2" && x["zone_offensive_for"] == "Alpha"));
        assert!(ts
            .iter()
            .any(|x| x["holder_id"] == "Beta:3" && x["zone_offensive_for"] == "Alpha"));
    }
    #[test]
    fn pass_requires_reception_and_self_pass_is_replaced_by_shot() {
        let (cr, r, a, b, mut p) = fixture();
        p.first_policy = Choice::ConnectThenShoot;
        p.second_policy = Choice::ConnectThenShoot;
        let v = inspect(&cr, &r, &a, &b, &p).unwrap();
        for x in v["action_audits"].as_array().unwrap() {
            if x["action"] == "connect_holder" {
                assert_ne!(x["actor_id"], x["target_id"]);
            }
            if x["action"] == "shoot_holder" {
                assert!(x["actor_id"] == "Alpha:1" || x["actor_id"] == "Beta:1");
            }
        }
        assert!(
            (v["goal_probability_by_team"]["Beta"].as_f64().unwrap() - 0.475_f64.powi(7)).abs()
                < 1e-12
        );
    }
    #[test]
    fn loose_opening_boundaries_do_not_select_a_holder() {
        let (cr, r, a, b, mut p) = fixture();
        p.completed_prefix = 1;
        let v = inspect(&cr, &r, &a, &b, &p).unwrap();
        assert_eq!(v["terminal_states"][0]["control_status"], "loose");
        assert!(v["terminal_states"][0]["holder_id"].is_null());
        assert_eq!(v["action_audits"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn full_opening_matches_standalone_goal_mass_and_invalid_inputs_reject() {
        let (cr, mut r, mut a, b, mut p) = fixture();
        p.opening = Threat::SlotPass;
        p.completed_prefix = 3;
        let v = inspect(&cr, &r, &a, &b, &p).unwrap();
        let old = super::super::inspect(&cr, &r.opening, &a, &b, Threat::SlotPass, 16).unwrap();
        assert_eq!(
            v["goal_probability_by_team"]["Alpha"],
            old["goal_probability"]
        );
        p.completed_prefix = 99;
        assert!(inspect(&cr, &r, &a, &b, &p).is_err());
        p.completed_prefix = 3;
        r.exit_seconds = 0;
        assert!(inspect(&cr, &r, &a, &b, &p).is_err());
        r.exit_seconds = 3;
        a.members[0].character.capabilities[0].score = 20;
        assert!(inspect(&cr, &r, &a, &b, &p).is_err());
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub opening: Threat,
    pub completed_prefix: usize,
    pub budget_seconds: u16,
    pub first_policy: Choice,
    pub second_policy: Choice,
}
#[derive(Clone, Copy)]
struct State {
    side: usize,
    slot: Slot,
    control: Control,
    zone: i8,
    elapsed: u16,
    receive: bool,
}
type TerminalKey = (String, String, String, String, i8, u16, String);
type TraceKey = (String, String, String, String, String, u16);
struct Ledger {
    terminal: BTreeMap<TerminalKey, f64>,
    traces: BTreeMap<TraceKey, [f64; 6]>,
    tasks: BTreeMap<(String, String, String), f64>,
    switches: f64,
    decisions: BTreeMap<(String, String, u16, String, String, String), f64>,
    recoveries: BTreeMap<(String, String, String, String, String, u16, String), [f64; 4]>,
    recovery_changes: f64,
    access_collections: BTreeMap<(String, String, String, String, String, u16, String), [f64; 4]>,
    supports: BTreeMap<(String, String, String, String, u16, String), [f64; 4]>,
    arrivals: BTreeMap<(String, String, u16, String), f64>,
}
impl Ledger {
    fn end(
        &mut self,
        w: f64,
        reason: &str,
        s: State,
        units: [&super::super::Unit; 2],
        teams: [&Team; 2],
        goal: bool,
    ) {
        let held = matches!(s.control, Control::Attacking | Control::Defending);
        let status = if held {
            "controlled"
        } else {
            match s.control {
                Control::Loose => "loose",
                Control::Dead => "dead",
                _ => "unknown",
            }
        };
        let team = if held {
            teams[s.side].team.clone()
        } else {
            String::new()
        };
        let holder = if held {
            s.slot.id(units[s.side]).to_owned()
        } else {
            String::new()
        };
        let scorer = if goal {
            teams[s.side].team.clone()
        } else {
            String::new()
        };
        *self
            .terminal
            .entry((
                reason.into(),
                status.into(),
                team,
                holder,
                s.zone,
                s.elapsed,
                scorer,
            ))
            .or_default() += w;
    }
    fn trace(
        &mut self,
        name: &str,
        s: State,
        actor: &str,
        opponent: &str,
        target: &str,
        team: &str,
        values: [f64; 6],
    ) {
        let entry = self
            .traces
            .entry((
                name.into(),
                team.into(),
                actor.into(),
                opponent.into(),
                target.into(),
                s.elapsed,
            ))
            .or_default();
        for i in 0..6 {
            entry[i] += values[i];
        }
    }
    fn task(&mut self, team: &str, id: &str, cap: &str, w: f64) {
        *self
            .tasks
            .entry((team.into(), id.into(), cap.into()))
            .or_default() += w;
    }
}
fn follow(
    mut s: State,
    mut w: f64,
    p: &Plan,
    r: &Rules,
    units: [&super::super::Unit; 2],
    teams: [&Team; 2],
    members: [&BTreeMap<&str, &game::GameCharacter>; 2],
    t: &super::super::Table,
    l: &mut Ledger,
    policies: Option<&HolderPolicies>,
    recovery: Option<&recovery::Rules>,
    recovery_left: u8,
) {
    loop {
        let side = s.side;
        let other = 1 - side;
        let u = units[side];
        let d = units[other];
        let relative = if side == 0 { s.zone } else { -s.zone };
        let default_choice = if side == 0 {
            p.first_policy
        } else {
            p.second_policy
        };
        let override_choice = policies.and_then(|ps| {
            let team_policy = if side == 0 { &ps.first } else { &ps.second };
            team_policy
                .overrides
                .iter()
                .find(|x| x.member_id == s.slot.id(u))
                .map(|x| x.choice)
        });
        let choice = override_choice.unwrap_or(default_choice);
        let (name, cost, cap, opponent, opp_cap) = if s.receive {
            (
                "receive",
                r.opening.action_seconds["receive"],
                1,
                Slot::Gap,
                5,
            )
        } else if relative < 0 {
            ("exit_holder", r.exit_seconds, 0, Slot::Forechecker, 5)
        } else if relative == 0 {
            (
                "advance_holder",
                r.opening.action_seconds["entry_duel"],
                0,
                Slot::Gap,
                0,
            )
        } else if matches!(choice, Choice::ConnectThenShoot) && s.slot.id(u) != u.receiver {
            (
                "connect_holder",
                r.opening.action_seconds["slot_connection"],
                2,
                Slot::Interceptor,
                4,
            )
        } else {
            (
                "shoot_holder",
                r.opening.action_seconds["finish"],
                3,
                Slot::Goalie,
                0,
            )
        };
        let aid = s.slot.id(u);
        if policies.is_some() && !s.receive && relative > 0 {
            let declared = if matches!(choice, Choice::ConnectThenShoot) {
                "connect_then_shoot"
            } else {
                "shoot_holder"
            };
            let source = if override_choice.is_some() {
                "member_override"
            } else {
                "team_default"
            };
            *l.decisions
                .entry((
                    teams[side].team.clone(),
                    aid.into(),
                    s.elapsed,
                    declared.into(),
                    source.into(),
                    name.into(),
                ))
                .or_default() += w;
        }
        let did = opponent.id(d);
        let target = if name == "connect_holder" {
            u.receiver.as_str()
        } else {
            ""
        };
        if s.elapsed + cost > p.budget_seconds {
            l.trace(
                name,
                s,
                aid,
                did,
                target,
                &teams[side].team,
                [w, 0.0, w, 0.0, 0.0, 0.0],
            );
            s.elapsed = p.budget_seconds;
            l.end(w, "time_expired", s, units, teams, false);
            return;
        }
        let a = members[side][aid].capabilities[cap].score as usize;
        let b = members[other][did].capabilities[opp_cap].score as usize;
        let q = t[a][b];
        l.trace(
            name,
            s,
            aid,
            did,
            target,
            &teams[side].team,
            [w, w, 0.0, w * q[0], w * q[1], w * q[2]],
        );
        l.task(&teams[side].team, aid, capability(s.slot, cap), w);
        l.task(&teams[other].team, did, capability(opponent, opp_cap), w);
        s.elapsed += cost;
        let mut tied = s;
        tied.control = if s.receive || name == "connect_holder" {
            Control::Loose
        } else {
            Control::Unknown
        };
        recovery::resolve(
            tied,
            w * q[1],
            name,
            "unresolved",
            p,
            r,
            units,
            teams,
            members,
            t,
            l,
            policies,
            recovery,
            recovery_left,
        );
        let mut lost = s;
        lost.control = Control::Loose;
        recovery::resolve(
            lost,
            w * q[2],
            name,
            if name == "shoot_holder" {
                "goalie_contact"
            } else {
                "disrupted"
            },
            p,
            r,
            units,
            teams,
            members,
            t,
            l,
            policies,
            recovery,
            recovery_left,
        );
        w *= q[0];
        match name {
            "shoot_holder" => {
                s.control = Control::Dead;
                l.end(w, "goal", s, units, teams, true);
                return;
            }
            "connect_holder" => {
                s.slot = Slot::Receiver;
                s.control = Control::Loose;
                s.receive = true;
            }
            "receive" => {
                s.control = Control::Attacking;
                s.receive = false;
            }
            "exit_holder" => s.zone = 0,
            "advance_holder" => s.zone = if side == 0 { 1 } else { -1 },
            _ => unreachable!(),
        }
    }
}

pub fn inspect(
    cr: &game::GameRules,
    r: &Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
) -> Result<serde_json::Value, String> {
    inspect_inner(cr, r, a, b, p, None, None, None, None, None)
}
pub fn inspect_with_policies(
    cr: &game::GameRules,
    r: &Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
    policies: &HolderPolicies,
) -> Result<serde_json::Value, String> {
    policies.validate(a, b, p)?;
    inspect_inner(cr, r, a, b, p, Some(policies), None, None, None, None)
}
fn inspect_inner(
    cr: &game::GameRules,
    r: &Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
    policies: Option<&HolderPolicies>,
    recovery: Option<&recovery::Rules>,
    access: Option<&access::Rules>,
    support: Option<&support::Rules>,
    arrival: Option<&support::arrival::Rules>,
) -> Result<serde_json::Value, String> {
    super::validate(&r.opening)?;
    if r.schema != "rinkcraft.control-sequence-rules.0.1"
        || r.version != "0.1.0-playtest"
        || r.kind != "authored_game_scenario_not_nhl_estimate"
        || r.exit_seconds == 0
        || r.exit_seconds > 30
        || p.budget_seconds == 0
        || p.budget_seconds > 60
    {
        return Err("Invalid control-sequence rules or clock".into());
    }
    let ss = super::steps(p.opening);
    if p.completed_prefix == 0
        || p.completed_prefix > ss.len()
        || a.team.trim().is_empty()
        || b.team.trim().is_empty()
        || a.team == b.team
    {
        return Err("Invalid opening prefix or team identities".into());
    }
    cr.validate()?;
    let mut ids = BTreeSet::new();
    patterns(a, cr, &mut ids)?;
    patterns(b, cr, &mut ids)?;
    let am: BTreeMap<_, _> = a
        .members
        .iter()
        .map(|m| (m.id.as_str(), &m.character))
        .collect();
    let bm: BTreeMap<_, _> = b
        .members
        .iter()
        .map(|m| (m.id.as_str(), &m.character))
        .collect();
    let members = [&am, &bm];
    let teams = [a, b];
    let table = table(r.opening.score_rule)?;
    let mut l = Ledger {
        terminal: BTreeMap::new(),
        traces: BTreeMap::new(),
        tasks: BTreeMap::new(),
        switches: 0.0,
        decisions: BTreeMap::new(),
        recoveries: BTreeMap::new(),
        recovery_changes: 0.0,
        access_collections: BTreeMap::new(),
        supports: BTreeMap::new(),
        arrivals: BTreeMap::new(),
    };
    for au in &a.units {
        for bu in &b.units {
            let units = [au, bu];
            let mut w = au.weight * bu.weight;
            let mut s = State {
                side: 0,
                slot: Slot::Creator,
                control: Control::Attacking,
                zone: if matches!(p.opening, Threat::Rush | Threat::DumpRetrieve) {
                    0
                } else {
                    1
                },
                elapsed: 0,
                receive: false,
            };
            let mut ended = false;
            for step in ss.iter().take(p.completed_prefix) {
                let cost = r.opening.action_seconds[step.name];
                let aid = step.duel.map(|d| d.actor.id(au)).unwrap_or("");
                let did = step.duel.map(|d| d.opponent.id(bu)).unwrap_or("");
                if s.elapsed + cost > p.budget_seconds {
                    l.trace(
                        step.name,
                        s,
                        aid,
                        did,
                        "",
                        &a.team,
                        [w, 0.0, w, 0.0, 0.0, 0.0],
                    );
                    s.elapsed = p.budget_seconds;
                    l.end(w, "time_expired", s, units, teams, false);
                    ended = true;
                    break;
                }
                let before = s;
                s.elapsed += cost;
                if let Some(d) = step.duel {
                    let q = table[am[aid].capabilities[d.cap].score as usize]
                        [bm[did].capabilities[d.opp_cap].score as usize];
                    l.trace(
                        step.name,
                        before,
                        aid,
                        did,
                        "",
                        &a.team,
                        [w, w, 0.0, w * q[0], w * q[1], w * q[2]],
                    );
                    l.task(&a.team, aid, capability(d.actor, d.cap), w);
                    l.task(&b.team, did, capability(d.opponent, d.opp_cap), w);
                    if step.name == "retrieval_race"
                        && access.is_some_and(|x| x.mode != access::Mode::Legacy)
                    {
                        let mut tied = s;
                        tied.control = step.tie_control;
                        l.end(w * q[1], "unresolved", tied, units, teams, false);
                        access::collect(
                            s,
                            w * q[0],
                            0,
                            p,
                            r,
                            units,
                            teams,
                            members,
                            &table,
                            &mut l,
                            policies,
                            recovery,
                            access.unwrap(),
                            support,
                            arrival,
                        );
                        access::collect(
                            s,
                            w * q[2],
                            1,
                            p,
                            r,
                            units,
                            teams,
                            members,
                            &table,
                            &mut l,
                            policies,
                            recovery,
                            access.unwrap(),
                            support,
                            arrival,
                        );
                        ended = true;
                        break;
                    }
                    let mut tied = s;
                    tied.control = step.tie_control;
                    l.end(w * q[1], "unresolved", tied, units, teams, false);
                    let mut lost = s;
                    lost.control = step.failure_control;
                    if matches!(lost.control, Control::Defending) {
                        lost.side = 1;
                        lost.slot = d.opponent;
                        lost.control = Control::Attacking;
                        l.switches += w * q[2];
                        follow(
                            lost,
                            w * q[2],
                            p,
                            r,
                            units,
                            teams,
                            members,
                            &table,
                            &mut l,
                            policies,
                            recovery,
                            recovery.map_or(0, |x| x.max_contests),
                        );
                    } else {
                        l.end(w * q[2], step.failure, lost, units, teams, false);
                    }
                    w *= q[0];
                    s.slot = d.actor;
                } else {
                    l.trace(
                        step.name,
                        before,
                        aid,
                        did,
                        "",
                        &a.team,
                        [w, w, 0.0, w, 0.0, 0.0],
                    );
                }
                s.control = step.success;
                s.zone = if step.zone == Zone::Neutral { 0 } else { 1 };
                if step.name == "finish" {
                    l.end(w, "goal", s, units, teams, true);
                    ended = true;
                    break;
                }
            }
            if !ended {
                if matches!(s.control, Control::Attacking) {
                    follow(
                        s,
                        w,
                        p,
                        r,
                        units,
                        teams,
                        members,
                        &table,
                        &mut l,
                        policies,
                        recovery,
                        recovery.map_or(0, |x| x.max_contests),
                    );
                } else {
                    l.end(w, "opening_boundary", s, units, teams, false);
                }
            }
        }
    }
    let mut goals = BTreeMap::from([(a.team.clone(), 0.0), (b.team.clone(), 0.0)]);
    let mut held = goals.clone();
    let mut expected = 0.0;
    let terminals:Vec<_>=l.terminal.iter().map(|((outcome,control,team,holder,zone,elapsed,scorer),w)|{
        expected+=f64::from(*elapsed)*w;
        if !scorer.is_empty(){*goals.get_mut(scorer).unwrap()+=w;}
        if !team.is_empty(){*held.get_mut(team).unwrap()+=w;}
        serde_json::json!({"outcome":outcome,"control_status":control,"control_team":if team.is_empty(){None}else{Some(team)},
            "holder_id":if holder.is_empty(){None}else{Some(holder)},"zone_offensive_for":match zone{1=>Some(&a.team),-1=>Some(&b.team),_=>None},
            "elapsed_seconds":elapsed,"goal_team":if scorer.is_empty(){None}else{Some(scorer)},"probability":w})
    }).collect();
    let traces:Vec<_>=l.traces.into_iter().map(|((action,team,actor,opponent,target,elapsed),q)|serde_json::json!({
        "action":action,"team":team,"actor_id":actor,"opponent_id":opponent,"target_id":if target.is_empty(){None}else{Some(target)},
        "start_seconds":elapsed,"reached_probability":q[0],"completed_probability":q[1],"expired_probability":q[2],
        "win_probability":q[3],"tie_probability":q[4],"loss_probability":q[5],
        "cost_seconds":match action.as_str(){"exit_holder"=>r.exit_seconds,"advance_holder"=>r.opening.action_seconds["entry_duel"],
        "connect_holder"=>r.opening.action_seconds["slot_connection"],"shoot_holder"=>r.opening.action_seconds["finish"],
        "loose_collect_connect"|"loose_collect_receive"=>recovery.expect("recovery traces require rules").cost_seconds,
        "access_collect_hunter"|"access_collect_defender"=>r.opening.action_seconds["gather"],
        "support_collect"|"support_collect_first"|"support_collect_second"=>support.expect("support trace requires rules").cost_seconds,
        "support_arrive"=>arrival.expect("arrival trace requires rules").wait_seconds().unwrap(),_=>r.opening.action_seconds[&action]}
    })).collect();
    let tasks:Vec<_>=l.tasks.into_iter().map(|((team,id,cap),w)|serde_json::json!({"team":team,"member_id":id,"capability":cap,"expected_contests":w})).collect();
    let mut result = serde_json::json!({"schema":"rinkcraft.control-sequence-inspection.0.1","kind":r.kind,
        "input_sha256":hash(&(cr,r,a,b,p))?,"plan":p,"seed":null,"trials":null,"terminal_states":terminals,"action_audits":traces,
        "task_opportunities":tasks,"goal_probability_by_team":goals,"terminal_control_probability_by_team":held,
        "expected_elapsed_seconds":expected,"control_switch_probability":l.switches,
        "limits":["Authored bounded exact sequence; no NHL calibration, measured timing, optimized policy or season integration",
        "Unit pair is selected once and retained; holder IDs persist through control changes and carries; no automatic playmaker handoff",
        "Absolute offensive-zone ownership changes perspective with the controlling team; a defensive collection must carry out then advance",
        "Opening source is recomputed from verified characters; arbitrary aggregate terminal reports are not imported as playable states",
        "Loose/unknown/dead states terminate except explicit in-flight reception after a chosen connection; no rebound recycle or interception possession",
        "No self-pass: connect policy shoots if holder already is the declared receiver; no other implicit teammate transfer",
        "One shared clock; completed actions pay full costs, incomplete actions expire at last boundary without a roll",
        "Costs fixed by action; no fatigue, goalie readiness, penalties, line changes or full-game outcome/points ledger"]});
    if let Some(ps) = policies {
        result["schema"] = "rinkcraft.holder-policy-inspection.0.1".into();
        result["input_sha256"] = hash(&(cr, r, a, b, p, ps))?.into();
        result["holder_policies"] = serde_json::to_value(ps).map_err(|e| e.to_string())?;
        result["policy_decisions"]=l.decisions.into_iter().map(|((team,id,time,choice,source,action),w)|serde_json::json!({
            "team":team,"holder_id":id,"start_seconds":time,"remaining_seconds":p.budget_seconds-time,"declared_choice":choice,
            "selection_source":source,"selected_action":action,"reached_probability":w,
            "self_pass_guard":choice=="connect_then_shoot"&&action=="shoot_holder"
        })).collect::<Vec<_>>().into();
        result["policy_limits"]=serde_json::json!(["Declared member-ID choices only at offensive controlled states; no oracle, score-based optimization or clock fallback",
            "Selection can occur without time to complete; check action completion/expiry before crediting execution",
            "Known but unused overrides have no effect; no skill absence inferred; receiver self-pass guard still applies"]);
    }
    if let Some(rr) = recovery {
        result["schema"] = "rinkcraft.loose-recovery-inspection.0.1".into();
        result["input_sha256"] = hash(&(cr, r, a, b, p, policies, rr))?.into();
        result["recovery_rules"] = serde_json::to_value(rr).map_err(|e| e.to_string())?;
        result
            .as_object_mut()
            .unwrap()
            .remove("control_switch_probability");
        result["opening_control_switch_probability"] = l.switches.into();
        result["expected_recovery_control_changes"] = l.recovery_changes.into();
        result["expected_total_control_changes"] = (l.switches + l.recovery_changes).into();
        result["recovery_audits"]=l.recoveries.into_iter().map(|((trigger,outcome,team,actor,opponent,time,status),q)|serde_json::json!({
            "trigger_action":trigger,"trigger_outcome":outcome,"initial_team":team,"collector_id":actor,"opposing_collector_id":opponent,
            "start_seconds":time,"status":status,"reached_probability":q[0],"same_team_control_probability":q[1],
            "other_team_control_probability":q[2],"unresolved_probability":q[3]
        })).collect::<Vec<_>>().into();
        let limits = result["limits"].as_array_mut().unwrap();
        limits.retain(|v|v.as_str()!=Some("Loose/unknown/dead states terminate except explicit in-flight reception after a chosen connection; no rebound recycle or interception possession"));
        limits.push(serde_json::json!("Eligible follow-up connection/reception ties or disruptions enter a declared shared-access Handling contest; no access race, proximity or geometry inferred"));
        limits.push(serde_json::json!("At most one recovery contest per path on the same clock; later loose states, shot contact, unknown and dead states terminate; no rebounds or full continuous play"));
        limits.push(serde_json::json!("Control-change outputs distinguish opening switch probability from expected additional/total change counts; a branch can change control twice"));
    }
    if let Some(ar) = access {
        result["schema"] = "rinkcraft.opening-access-inspection.0.1".into();
        result["input_sha256"] = hash(&(cr, r, a, b, p, policies, recovery, ar))?.into();
        result["opening_access_rules"] = serde_json::to_value(ar).map_err(|e| e.to_string())?;
        result["opening_collection_audits"]=l.access_collections.into_iter().map(|((race_team,actor,opponent,actor_cap,op_cap,time,status),q)|serde_json::json!({
            "race_winner_team":race_team,"collector_id":actor,"opponent_id":opponent,"collector_capability":actor_cap,"opponent_capability":op_cap,
            "start_seconds":time,"status":status,"reached_probability":q[0],"collector_control_probability":q[1],
            "opponent_control_probability":q[2],"loose_probability":q[3]
        })).collect::<Vec<_>>().into();
        result["access_limits"]=serde_json::json!(["Declared alternative collection models; no inferred geometry, empirical access advantage or class bonus",
            "Shared mode permits Handling control by either named skater after either race winner; race direction need not determine the controller",
            "First-access mode gives winner a Handling-vs-Defending attempt; disruption does not give the defender control",
            "Only dump retrieval through collection is supported; race ties, failed first attempts and unsupported loose play still stop",
            "Opening collection does not consume the separate pass-recovery allowance; fixed units and one shared clock remain"]);
    }
    if let Some(sr) = support {
        result["access_limits"][3] = serde_json::json!("Only dump retrieval through collection is supported; race ties and unsupported loose play stop; completed first-collection failures may enter the declared support window");
        result["schema"] = "rinkcraft.opening-support-inspection.0.1".into();
        result["input_sha256"] = hash(&(cr, r, a, b, p, policies, recovery, access, sr))?.into();
        result["support_rules"] = serde_json::to_value(sr).map_err(|e| e.to_string())?;
        result["support_audits"] = l.supports.into_iter().map(|((race_team,trigger,actor,opponent,time,status),q)|serde_json::json!({
            "race_winner_team":race_team,"trigger_outcome":trigger,"collector_id":actor,"opposing_collector_id":opponent,
            "start_seconds":time,"status":status,"reached_probability":q[0],"first_team_control_probability":q[1],
            "second_team_control_probability":q[2],"loose_probability":q[3]
        })).collect::<Vec<_>>().into();
        let mut initial = BTreeMap::from([(a.team.clone(), 0.0), (b.team.clone(), 0.0)]);
        for x in result["opening_collection_audits"].as_array().unwrap() {
            let own = x["race_winner_team"].as_str().unwrap();
            let other = if own == a.team { &b.team } else { &a.team };
            *initial.get_mut(own).unwrap() += x["collector_control_probability"].as_f64().unwrap();
            *initial.get_mut(other).unwrap() += x["opponent_control_probability"].as_f64().unwrap();
        }
        for x in result["support_audits"].as_array().unwrap() {
            *initial.get_mut(&a.team).unwrap() +=
                x["first_team_control_probability"].as_f64().unwrap();
            *initial.get_mut(&b.team).unwrap() +=
                x["second_team_control_probability"].as_f64().unwrap();
        }
        result["first_possession_probability_by_team"] =
            serde_json::to_value(initial).map_err(|e| e.to_string())?;
        result["support_limits"] = serde_json::json!(["Declared shared support access after first-access collection tie/disruption only; no geometric proximity or F2/F3 assignment inferred",
            "First receiver versus second gap defender, distinct from the initial collectors; Handling versus Handling, at most once",
            "Race ties, uncompleted collection, carry failures and goalie contacts do not open this support window",
            "Opening audit loose mass is the immediate collection outcome, not final terminal mass; support audits show its continuation",
            "First-possession probabilities are disjoint initial acquisition events, separate from terminal possession and eventual goals",
            "Separate pass recovery allowance is retained; support tie ends loose, no repeated collection, rebound or full-game integration"]);
    }
    if let Some(arr) = arrival {
        support::arrival::describe(&mut result, arr, support.unwrap(), l.arrivals)?;
        result["input_sha256"] =
            hash(&(cr, r, a, b, p, policies, recovery, access, support, arr))?.into();
    }
    Ok(result)
}
