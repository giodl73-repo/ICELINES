//! One explicitly shared-access pass-recovery window per path, opt-in only.
use super::*;
pub mod sensitivity;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub cost_seconds: u16,
    pub max_contests: u8,
}
impl Rules {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.pass-recovery-rules.0.1"
            || self.version != "0.1.0-playtest"
            || self.kind != "authored_game_scenario_not_nhl_estimate"
            || self.cost_seconds == 0
            || self.cost_seconds > 30
            || self.max_contests > 1
        {
            return Err("Invalid pass-recovery rules, cost or bound".into());
        }
        Ok(())
    }
}
pub fn inspect(
    cr: &game::GameRules,
    r: &super::Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
    policies: &HolderPolicies,
    recovery: &Rules,
) -> Result<serde_json::Value, String> {
    recovery.validate()?;
    policies.validate(a, b, p)?;
    inspect_inner(cr, r, a, b, p, Some(policies), Some(recovery), None, None)
}
pub(super) fn resolve(
    mut s: State,
    w: f64,
    trigger: &str,
    outcome: &str,
    p: &Plan,
    r: &super::Rules,
    units: [&super::super::super::Unit; 2],
    teams: [&Team; 2],
    members: [&BTreeMap<&str, &game::GameCharacter>; 2],
    t: &super::super::super::Table,
    l: &mut Ledger,
    policies: Option<&HolderPolicies>,
    rr: Option<&Rules>,
    left: u8,
) {
    if rr.is_none()
        || !matches!(s.control, Control::Loose)
        || !matches!(trigger, "connect_holder" | "receive")
    {
        l.end(w, outcome, s, units, teams, false);
        return;
    }
    let rr = rr.unwrap();
    let side = s.side;
    let other = 1 - side;
    let opponent = if trigger == "connect_holder" {
        Slot::Interceptor
    } else {
        Slot::Gap
    };
    let actor_id = Slot::Receiver.id(units[side]);
    let opponent_id = opponent.id(units[other]);
    let status = if left == 0 {
        "limit_reached"
    } else if s.elapsed + rr.cost_seconds > p.budget_seconds {
        "clock_expired"
    } else {
        "completed"
    };
    let key = (
        trigger.into(),
        outcome.into(),
        teams[side].team.clone(),
        actor_id.into(),
        opponent_id.into(),
        s.elapsed,
        status.into(),
    );
    if left == 0 {
        l.recoveries.entry(key).or_default()[0] += w;
        l.end(w, outcome, s, units, teams, false);
        return;
    }
    let action = if trigger == "connect_holder" {
        "loose_collect_connect"
    } else {
        "loose_collect_receive"
    };
    if status == "clock_expired" {
        l.recoveries.entry(key).or_default()[0] += w;
        l.trace(
            action,
            s,
            actor_id,
            opponent_id,
            "",
            &teams[side].team,
            [w, 0.0, w, 0.0, 0.0, 0.0],
        );
        s.elapsed = p.budget_seconds;
        l.end(w, "time_expired", s, units, teams, false);
        return;
    }
    let q = t[members[side][actor_id].capabilities[1].score as usize]
        [members[other][opponent_id].capabilities[1].score as usize];
    let audit = l.recoveries.entry(key).or_default();
    for i in 0..4 {
        audit[i] += w * if i == 0 {
            1.0
        } else if i == 1 {
            q[0]
        } else if i == 2 {
            q[2]
        } else {
            q[1]
        };
    }
    l.trace(
        action,
        s,
        actor_id,
        opponent_id,
        "",
        &teams[side].team,
        [w, w, 0.0, w * q[0], w * q[1], w * q[2]],
    );
    l.task(&teams[side].team, actor_id, "Handling", w);
    l.task(&teams[other].team, opponent_id, "Handling", w);
    s.elapsed += rr.cost_seconds;
    s.receive = false;
    l.end(
        w * q[1],
        "loose_collection_unresolved",
        s,
        units,
        teams,
        false,
    );
    let mut own = s;
    own.slot = Slot::Receiver;
    own.control = Control::Attacking;
    follow(
        own,
        w * q[0],
        p,
        r,
        units,
        teams,
        members,
        t,
        l,
        policies,
        Some(rr),
        left - 1,
    );
    let mut changed = s;
    changed.side = other;
    changed.slot = opponent;
    changed.control = Control::Attacking;
    l.recovery_changes += w * q[2];
    follow(
        changed,
        w * q[2],
        p,
        r,
        units,
        teams,
        members,
        t,
        l,
        policies,
        Some(rr),
        left - 1,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        game::GameRules,
        super::super::Rules,
        Team,
        Team,
        Plan,
        HolderPolicies,
        Rules,
    ) {
        let (cr, r, a, b, mut p) = super::super::tests::fixture();
        p.budget_seconds = 20;
        let ps = HolderPolicies {
            schema: "rinkcraft.holder-policies.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: r.kind.clone(),
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
        };
        let rr = Rules {
            schema: "rinkcraft.pass-recovery-rules.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: r.kind.clone(),
            cost_seconds: 2,
            max_contests: 1,
        };
        (cr, r, a, b, p, ps, rr)
    }
    fn num(v: &serde_json::Value, k: &str) -> f64 {
        v[k].as_f64().unwrap()
    }
    #[test]
    fn disabled_recovery_preserves_prior_mechanics() {
        let (cr, r, a, b, mut p, mut ps, mut rr) = fixture();
        p.first_policy = Choice::ConnectThenShoot;
        ps.first.default = p.first_policy;
        rr.max_contests = 0;
        let old = inspect_with_policies(&cr, &r, &a, &b, &p, &ps).unwrap();
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        for k in [
            "terminal_states",
            "action_audits",
            "task_opportunities",
            "goal_probability_by_team",
            "terminal_control_probability_by_team",
            "expected_elapsed_seconds",
            "policy_decisions",
        ] {
            assert_eq!(old[k], v[k], "{k}");
        }
        assert_eq!(
            old["control_switch_probability"],
            v["opening_control_switch_probability"]
        );
        assert!(v.get("control_switch_probability").is_none());
        assert_eq!(num(&v, "expected_recovery_control_changes"), 0.0);
    }
    #[test]
    fn pass_risk_is_clock_conditional_and_shooting_contact_stays_closed() {
        let (cr, r, a, b, mut p, mut ps, rr) = fixture();
        let shoot = inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        assert!(shoot["recovery_audits"].as_array().unwrap().is_empty());
        p.first_policy = Choice::ConnectThenShoot;
        ps.first.default = p.first_policy;
        let pass = inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        assert!(
            pass["goal_probability_by_team"]["Beta"].as_f64().unwrap()
                > shoot["goal_probability_by_team"]["Beta"].as_f64().unwrap()
        );
        p.budget_seconds = 16;
        let short = inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        assert_eq!(
            short["goal_probability_by_team"]["Beta"],
            shoot["goal_probability_by_team"]["Beta"]
        );
    }
    #[test]
    fn recovery_clock_terminal_and_capability_ledgers_conserve() {
        let (cr, r, a, b, mut p, mut ps, rr) = fixture();
        p.first_policy = Choice::ConnectThenShoot;
        p.second_policy = Choice::ConnectThenShoot;
        ps.first.default = p.first_policy;
        ps.second.default = p.second_policy;
        for budget in [6, 9, 16, 20] {
            p.budget_seconds = budget;
            let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
            let ts = v["terminal_states"].as_array().unwrap();
            assert!((ts.iter().map(|x| num(x, "probability")).sum::<f64>() - 1.0).abs() < 1e-12);
            let elapsed = ts
                .iter()
                .map(|x| num(x, "probability") * num(x, "elapsed_seconds"))
                .sum::<f64>();
            let mut charged = 0.0;
            let mut contests = 0.0;
            let mut collections = 0.0;
            for x in v["action_audits"].as_array().unwrap() {
                assert!(
                    (num(x, "completed_probability") + num(x, "expired_probability")
                        - num(x, "reached_probability"))
                    .abs()
                        < 1e-12
                );
                assert!(
                    (num(x, "win_probability")
                        + num(x, "tie_probability")
                        + num(x, "loss_probability")
                        - num(x, "completed_probability"))
                    .abs()
                        < 1e-12
                );
                charged += num(x, "completed_probability") * num(x, "cost_seconds")
                    + num(x, "expired_probability") * (f64::from(budget) - num(x, "start_seconds"));
                if x["action"] != "send_deep" {
                    contests += 2.0 * num(x, "completed_probability");
                }
                if x["action"].as_str().unwrap().starts_with("loose_collect_") {
                    collections += num(x, "completed_probability");
                }
            }
            assert!((elapsed - charged).abs() < 1e-12);
            assert!((elapsed - num(&v, "expected_elapsed_seconds")).abs() < 1e-12);
            assert!(collections <= 1.0 + 1e-12);
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
            assert!(
                (num(&v, "opening_control_switch_probability")
                    + num(&v, "expected_recovery_control_changes")
                    - num(&v, "expected_total_control_changes"))
                .abs()
                    < 1e-12
            );
        }
    }
    #[test]
    fn collector_identity_and_second_control_change_are_explicit() {
        let (cr, r, a, b, mut p, mut ps, rr) = fixture();
        p.first_policy = Choice::ConnectThenShoot;
        p.second_policy = Choice::ConnectThenShoot;
        ps.first.default = p.first_policy;
        ps.second.default = p.second_policy;
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        let audits = v["recovery_audits"].as_array().unwrap();
        assert!(audits.iter().any(|x| x["initial_team"] == "Alpha"
            && x["collector_id"] == "Alpha:1"
            && x["opposing_collector_id"] == "Beta:3"));
        assert!(audits.iter().any(|x| x["initial_team"] == "Beta"
            && x["opposing_collector_id"] == "Alpha:3"
            && num(x, "other_team_control_probability") > 0.0));
        assert!(audits.iter().any(|x| x["status"] == "limit_reached"));
    }
    #[test]
    fn incomplete_collection_has_no_handling_contest() {
        let (cr, r, a, b, mut p, mut ps, rr) = fixture();
        p.first_policy = Choice::ConnectThenShoot;
        ps.first.default = p.first_policy;
        p.budget_seconds = 9;
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        let actions = v["action_audits"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["action"].as_str().unwrap().starts_with("loose_collect_"))
            .collect::<Vec<_>>();
        assert!(!actions.is_empty());
        assert!(actions
            .iter()
            .all(|x| num(x, "completed_probability") == 0.0));
    }
    #[test]
    fn invalid_recovery_bounds_and_costs_reject() {
        let (cr, r, a, b, p, ps, mut rr) = fixture();
        rr.cost_seconds = 0;
        assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr).is_err());
        rr.cost_seconds = 2;
        rr.max_contests = 2;
        assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr).is_err());
    }
}
