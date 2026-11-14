//! One declared shared-access support window after a failed opening collection.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub cost_seconds: u16,
    pub max_contests: u8,
}
pub fn inspect(
    cr: &game::GameRules,
    r: &super::Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
    ps: &HolderPolicies,
    rr: &recovery::Rules,
    ar: &access::Rules,
    sr: &Rules,
) -> Result<serde_json::Value, String> {
    access::validate(ar, p)?;
    rr.validate()?;
    ps.validate(a, b, p)?;
    if sr.schema != "rinkcraft.opening-support-rules.0.1"
        || sr.version != "0.1.0-playtest"
        || sr.kind != "authored_game_scenario_not_nhl_estimate"
        || sr.cost_seconds == 0
        || sr.cost_seconds > 30
        || sr.max_contests > 1
        || ar.mode != access::Mode::FirstAccessCollection
        || a.units.iter().any(|u| u.receiver == u.forechecker)
        || b.units.iter().any(|u| u.gap_defender == u.interceptor)
    {
        return Err(
            "Support needs first-access rules, distinct support skaters, cost 1-30 and bound 0/1"
                .into(),
        );
    }
    inspect_inner(cr, r, a, b, p, Some(ps), Some(rr), Some(ar), Some(sr))
}
pub(super) fn resolve(
    mut s: State,
    w: f64,
    race_side: usize,
    trigger: &str,
    p: &Plan,
    r: &super::Rules,
    units: [&super::super::super::Unit; 2],
    teams: [&Team; 2],
    members: [&BTreeMap<&str, &game::GameCharacter>; 2],
    t: &super::super::super::Table,
    l: &mut Ledger,
    ps: Option<&HolderPolicies>,
    rr: Option<&recovery::Rules>,
    sr: &Rules,
) {
    let aid = Slot::Receiver.id(units[0]);
    let bid = Slot::Gap.id(units[1]);
    let status = if sr.max_contests == 0 {
        "disabled"
    } else if s.elapsed + sr.cost_seconds > p.budget_seconds {
        "clock_expired"
    } else {
        "completed"
    };
    let key = (
        teams[race_side].team.clone(),
        trigger.into(),
        aid.into(),
        bid.into(),
        s.elapsed,
        status.into(),
    );
    if status != "completed" {
        let q = l.supports.entry(key).or_default();
        q[0] += w;
        q[3] += w;
        if status == "disabled" {
            l.end(w, trigger, s, units, teams, false);
            return;
        }
        l.trace(
            "support_collect",
            s,
            aid,
            bid,
            "",
            &teams[0].team,
            [w, 0.0, w, 0.0, 0.0, 0.0],
        );
        s.elapsed = p.budget_seconds;
        l.end(w, "time_expired", s, units, teams, false);
        return;
    }
    let q = t[members[0][aid].capabilities[1].score as usize]
        [members[1][bid].capabilities[1].score as usize];
    let audit = l.supports.entry(key).or_default();
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
        "support_collect",
        s,
        aid,
        bid,
        "",
        &teams[0].team,
        [w, w, 0.0, w * q[0], w * q[1], w * q[2]],
    );
    l.task(&teams[0].team, aid, "Handling", w);
    l.task(&teams[1].team, bid, "Handling", w);
    s.elapsed += sr.cost_seconds;
    s.receive = false;
    s.control = Control::Loose;
    l.end(
        w * q[1],
        "support_collection_unresolved",
        s,
        units,
        teams,
        false,
    );
    let mut own = s;
    own.side = 0;
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
        ps,
        rr,
        rr.map_or(0, |x| x.max_contests),
    );
    let mut other = s;
    other.side = 1;
    other.slot = Slot::Gap;
    other.control = Control::Attacking;
    l.switches += w * q[2];
    follow(
        other,
        w * q[2],
        p,
        r,
        units,
        teams,
        members,
        t,
        l,
        ps,
        rr,
        rr.map_or(0, |x| x.max_contests),
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
        recovery::Rules,
        access::Rules,
        Rules,
    ) {
        let (cr, r, a, b, mut p) = super::super::tests::fixture();
        p.budget_seconds = 24;
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
        let rr = recovery::Rules {
            schema: "rinkcraft.pass-recovery-rules.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: r.kind.clone(),
            cost_seconds: 2,
            max_contests: 1,
        };
        let ar = access::Rules {
            schema: "rinkcraft.opening-access-rules.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: r.kind.clone(),
            mode: access::Mode::FirstAccessCollection,
        };
        let sr = Rules {
            schema: "rinkcraft.opening-support-rules.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: r.kind.clone(),
            cost_seconds: 2,
            max_contests: 1,
        };
        (cr, r, a, b, p, ps, rr, ar, sr)
    }
    fn n(v: &serde_json::Value, k: &str) -> f64 {
        v[k].as_f64().unwrap()
    }
    #[test]
    fn disabled_support_preserves_prior_mechanics() {
        let (cr, r, a, b, p, ps, rr, ar, mut sr) = fixture();
        sr.max_contests = 0;
        let old = access::inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar).unwrap();
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
        for k in [
            "terminal_states",
            "action_audits",
            "task_opportunities",
            "goal_probability_by_team",
            "expected_elapsed_seconds",
            "opening_collection_audits",
            "opening_control_switch_probability",
            "expected_total_control_changes",
        ] {
            assert_eq!(old[k], v[k], "{k}");
        }
    }
    #[test]
    fn support_requires_completed_collection_and_its_own_clock() {
        let (cr, r, a, b, mut p, ps, rr, ar, sr) = fixture();
        for budget in [5, 6, 7] {
            p.budget_seconds = budget;
            let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
            if budget == 5 {
                assert!(v["support_audits"].as_array().unwrap().is_empty());
            } else {
                for x in v["support_audits"].as_array().unwrap() {
                    assert_eq!(x["status"], "clock_expired");
                    assert_eq!(x["loose_probability"], x["reached_probability"]);
                }
            }
            assert!(!v["task_opportunities"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x["member_id"] == a.units[0].receiver && x["capability"] == "Handling"));
        }
    }
    #[test]
    fn support_has_named_distinct_collectors_and_disjoint_initial_control() {
        let (cr, r, a, b, mut p, ps, rr, ar, sr) = fixture();
        p.budget_seconds = 8;
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
        let rows = v["support_audits"].as_array().unwrap();
        let reached = rows
            .iter()
            .map(|x| n(x, "reached_probability"))
            .sum::<f64>();
        assert!((reached - 0.49875).abs() < 1e-12);
        for x in rows {
            assert_eq!(x["collector_id"], a.units[0].receiver);
            assert_eq!(x["opposing_collector_id"], b.units[0].gap_defender);
            assert_eq!(x["start_seconds"], 6);
        }
        for team in [&a.team, &b.team] {
            assert!(
                (v["first_possession_probability_by_team"][team]
                    .as_f64()
                    .unwrap()
                    - 0.46253125)
                    .abs()
                    < 1e-12
            );
        }
        assert!((n(&v, "opening_control_switch_probability") - 0.46253125).abs() < 1e-12);
        assert_ne!(
            v["first_possession_probability_by_team"],
            v["terminal_control_probability_by_team"]
        );
    }
    #[test]
    fn actual_support_holders_keep_zone_and_policy() {
        let (cr, r, a, b, mut p, mut ps, rr, ar, sr) = fixture();
        p.budget_seconds = 8;
        ps.first.overrides.push(HolderChoice {
            member_id: a.units[0].receiver.clone(),
            choice: Choice::ConnectThenShoot,
        });
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
        assert!(v["terminal_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |x| x["holder_id"] == b.units[0].gap_defender && x["zone_offensive_for"] == a.team
            ));
        assert!(v["policy_decisions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["holder_id"] == a.units[0].receiver
                && x["start_seconds"] == 8
                && x["self_pass_guard"] == true));
    }
    #[test]
    fn improved_support_handling_changes_only_eligible_control_branches() {
        let (cr, r, mut a, b, p, ps, rr, ar, sr) = fixture();
        let old = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
        let id = a.units[0].receiver.clone();
        let m = a.members.iter_mut().find(|m| m.id == id).unwrap();
        let c = &m.character;
        let mut scores = [10; 6];
        scores[1] = 16;
        m.character = game::build(
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
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
        assert_eq!(
            v["opening_collection_audits"],
            old["opening_collection_audits"]
        );
        assert!(
            v["first_possession_probability_by_team"][&a.team]
                .as_f64()
                .unwrap()
                > old["first_possession_probability_by_team"][&a.team]
                    .as_f64()
                    .unwrap()
        );
        assert!(
            v["first_possession_probability_by_team"][&b.team]
                .as_f64()
                .unwrap()
                < old["first_possession_probability_by_team"][&b.team]
                    .as_f64()
                    .unwrap()
        );
    }
    #[test]
    fn support_bounds_identity_and_scope_reject() {
        let (cr, r, mut a, b, p, ps, rr, mut ar, mut sr) = fixture();
        for (cost, bound) in [(0, 1), (31, 1), (2, 2)] {
            sr.cost_seconds = cost;
            sr.max_contests = bound;
            assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).is_err());
        }
        sr.cost_seconds = 2;
        sr.max_contests = 1;
        ar.mode = access::Mode::SharedCollection;
        assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).is_err());
        ar.mode = access::Mode::FirstAccessCollection;
        a.units[0].receiver = a.units[0].forechecker.clone();
        assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).is_err());
    }
    #[test]
    fn terminal_and_collection_mass_conserve_without_repeated_support() {
        let (cr, r, a, b, mut p, ps, rr, ar, mut sr) = fixture();
        for cost in [1, 2, 8] {
            sr.cost_seconds = cost;
            for budget in [4, 5, 6, 7, 8, 24, 32] {
                p.budget_seconds = budget;
                let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
                assert!(
                    (v["terminal_states"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| n(x, "probability"))
                        .sum::<f64>()
                        - 1.0)
                        .abs()
                        < 1e-12
                );
                let reached = v["support_audits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| n(x, "reached_probability"))
                    .sum::<f64>();
                assert!(reached <= 0.49875 + 1e-12);
                for x in v["support_audits"].as_array().unwrap() {
                    assert!(
                        (n(x, "first_team_control_probability")
                            + n(x, "second_team_control_probability")
                            + n(x, "loose_probability")
                            - n(x, "reached_probability"))
                        .abs()
                            < 1e-12
                    );
                }
            }
        }
    }
}
