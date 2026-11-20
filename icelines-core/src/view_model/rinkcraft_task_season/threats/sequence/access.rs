//! Opening access and collection are distinct, separately versioned assumptions.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Legacy,
    SharedCollection,
    FirstAccessCollection,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub mode: Mode,
}
pub fn inspect(
    cr: &game::GameRules,
    r: &super::Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
    policies: &HolderPolicies,
    rr: &recovery::Rules,
    ar: &Rules,
) -> Result<serde_json::Value, String> {
    validate(ar, p)?;
    policies.validate(a, b, p)?;
    rr.validate()?;
    inspect_inner(
        cr,
        r,
        a,
        b,
        p,
        Some(policies),
        Some(rr),
        Some(ar),
        None,
        None,
    )
}
pub(super) fn validate(ar: &Rules, p: &Plan) -> Result<(), String> {
    if ar.schema != "rinkcraft.opening-access-rules.0.1"
        || ar.version != "0.1.0-playtest"
        || ar.kind != "authored_game_scenario_not_nhl_estimate"
        || p.opening != Threat::DumpRetrieve
        || p.completed_prefix != 3
    {
        return Err(
            "Opening access model requires its rules and dump prefix through collection".into(),
        );
    }
    Ok(())
}
pub(super) fn collect(
    mut s: State,
    w: f64,
    side: usize,
    p: &Plan,
    r: &super::Rules,
    units: [&super::super::super::Unit; 2],
    teams: [&Team; 2],
    members: [&BTreeMap<&str, &game::GameCharacter>; 2],
    t: &super::super::super::Table,
    l: &mut Ledger,
    policies: Option<&HolderPolicies>,
    rr: Option<&recovery::Rules>,
    ar: &Rules,
    sr: Option<&support::Rules>,
    arrival: Option<&support::arrival::Rules>,
) {
    let other = 1 - side;
    let actor = if side == 0 {
        Slot::Forechecker
    } else {
        Slot::Interceptor
    };
    let opponent = if other == 0 {
        Slot::Forechecker
    } else {
        Slot::Interceptor
    };
    let aid = actor.id(units[side]);
    let did = opponent.id(units[other]);
    let opp_cap = if ar.mode == Mode::SharedCollection {
        1
    } else {
        5
    };
    let cost = r.opening.action_seconds["gather"];
    s.control = Control::Loose;
    s.receive = false;
    let status = if s.elapsed + cost > p.budget_seconds {
        "clock_expired"
    } else {
        "completed"
    };
    let key = (
        teams[side].team.clone(),
        aid.into(),
        did.into(),
        "Handling".into(),
        capability(opponent, opp_cap).into(),
        s.elapsed,
        status.into(),
    );
    let action = if side == 0 {
        "access_collect_hunter"
    } else {
        "access_collect_defender"
    };
    if status == "clock_expired" {
        let audit = l.access_collections.entry(key).or_default();
        audit[0] += w;
        audit[3] += w;
        l.trace(
            action,
            s,
            aid,
            did,
            "",
            &teams[side].team,
            [w, 0.0, w, 0.0, 0.0, 0.0],
        );
        s.elapsed = p.budget_seconds;
        l.end(w, "time_expired", s, units, teams, false);
        return;
    }
    let q = t[members[side][aid].capabilities[1].score as usize]
        [members[other][did].capabilities[opp_cap].score as usize];
    l.trace(
        action,
        s,
        aid,
        did,
        "",
        &teams[side].team,
        [w, w, 0.0, w * q[0], w * q[1], w * q[2]],
    );
    l.task(&teams[side].team, aid, "Handling", w);
    l.task(&teams[other].team, did, capability(opponent, opp_cap), w);
    s.elapsed += cost;
    {
        let audit = l.access_collections.entry(key).or_default();
        audit[0] += w;
        audit[1] += w * q[0];
        if ar.mode == Mode::SharedCollection {
            audit[2] += w * q[2];
            audit[3] += w * q[1];
        } else {
            audit[3] += w * (q[1] + q[2]);
        }
    }
    if ar.mode == Mode::FirstAccessCollection && sr.is_some() {
        support::resolve(
            s,
            w * q[1],
            side,
            "access_collection_unresolved",
            p,
            r,
            units,
            teams,
            members,
            t,
            l,
            policies,
            rr,
            sr.unwrap(),
            arrival,
        );
    } else {
        l.end(
            w * q[1],
            "access_collection_unresolved",
            s,
            units,
            teams,
            false,
        );
    }
    if ar.mode == Mode::SharedCollection {
        let mut opponent_state = s;
        opponent_state.side = other;
        opponent_state.slot = opponent;
        opponent_state.control = Control::Attacking;
        if other == 1 {
            l.switches += w * q[2];
        }
        follow(
            opponent_state,
            w * q[2],
            p,
            r,
            units,
            teams,
            members,
            t,
            l,
            policies,
            rr,
            rr.map_or(0, |x| x.max_contests),
        );
    } else {
        if let Some(sr) = sr {
            support::resolve(
                s,
                w * q[2],
                side,
                "access_collection_disrupted",
                p,
                r,
                units,
                teams,
                members,
                t,
                l,
                policies,
                rr,
                sr,
                arrival,
            );
        } else {
            l.end(
                w * q[2],
                "access_collection_disrupted",
                s,
                units,
                teams,
                false,
            );
        }
    }
    let mut own = s;
    own.side = side;
    own.slot = actor;
    own.control = Control::Attacking;
    if side == 1 {
        l.switches += w * q[0];
    }
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
        (cr, r, a, b, p, ps, rr)
    }
    fn rule(mode: Mode) -> Rules {
        Rules {
            schema: "rinkcraft.opening-access-rules.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: "authored_game_scenario_not_nhl_estimate".into(),
            mode,
        }
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
    fn legacy_mode_matches_existing_mechanics() {
        let (cr, r, a, b, p, ps, rr) = fixture();
        let old = recovery::inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &rule(Mode::Legacy)).unwrap();
        for k in [
            "terminal_states",
            "action_audits",
            "task_opportunities",
            "goal_probability_by_team",
            "expected_elapsed_seconds",
            "opening_control_switch_probability",
        ] {
            assert_eq!(old[k], v[k], "{k}");
        }
    }
    #[test]
    fn race_win_never_implies_control_and_cost_must_fit() {
        let (cr, r, a, b, mut p, ps, rr) = fixture();
        p.budget_seconds = 5;
        let v = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &rule(Mode::FirstAccessCollection),
        )
        .unwrap();
        assert_eq!(v["terminal_control_probability_by_team"]["Alpha"], 0.0);
        assert_eq!(v["terminal_control_probability_by_team"]["Beta"], 0.0);
        assert!(v["opening_collection_audits"]
            .as_array()
            .unwrap()
            .iter()
            .all(|x| x["status"] == "clock_expired"));
        for x in v["opening_collection_audits"].as_array().unwrap() {
            assert_eq!(x["reached_probability"], x["loose_probability"]);
            assert_eq!(x["collector_control_probability"], 0.0);
            assert_eq!(x["opponent_control_probability"], 0.0);
        }
    }
    #[test]
    fn neutral_aggregate_equivalence_hides_different_collection_exposure() {
        let (cr, r, a, b, p, ps, rr) = fixture();
        let old = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &rule(Mode::Legacy)).unwrap();
        let new = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &rule(Mode::FirstAccessCollection),
        )
        .unwrap();
        for team in ["Alpha", "Beta"] {
            assert!(
                (old["goal_probability_by_team"][team].as_f64().unwrap()
                    - new["goal_probability_by_team"][team].as_f64().unwrap())
                .abs()
                    < 1e-12
            );
        }
        let expected = new["opening_collection_audits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| num(x, "reached_probability"))
            .sum::<f64>();
        assert!((expected - 0.95).abs() < 1e-12);
        assert_ne!(old["task_opportunities"], new["task_opportunities"]);
    }
    #[test]
    fn shared_access_can_remove_directional_skating_advantage() {
        let (cr, r, mut a, b, mut p, ps, rr) = fixture();
        p.budget_seconds = 6;
        change(&cr, &mut a, 2, 0);
        let first = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &rule(Mode::FirstAccessCollection),
        )
        .unwrap();
        let shared = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &rule(Mode::SharedCollection)).unwrap();
        assert!(
            first["terminal_control_probability_by_team"]["Alpha"]
                .as_f64()
                .unwrap()
                > first["terminal_control_probability_by_team"]["Beta"]
                    .as_f64()
                    .unwrap()
        );
        assert!(
            (shared["terminal_control_probability_by_team"]["Alpha"]
                .as_f64()
                .unwrap()
                - shared["terminal_control_probability_by_team"]["Beta"]
                    .as_f64()
                    .unwrap())
            .abs()
                < 1e-12
        );
    }
    #[test]
    fn first_access_defending_disrupts_without_awarding_possession() {
        let (cr, r, a, mut b, mut p, ps, rr) = fixture();
        p.budget_seconds = 6;
        change(&cr, &mut b, 3, 5);
        let v = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &rule(Mode::FirstAccessCollection),
        )
        .unwrap();
        assert!(
            (v["terminal_control_probability_by_team"]["Beta"]
                .as_f64()
                .unwrap()
                - 0.225625)
                .abs()
                < 1e-12
        );
        assert!(
            v["terminal_control_probability_by_team"]["Alpha"]
                .as_f64()
                .unwrap()
                < 0.225625
        );
    }
    #[test]
    fn ledger_and_opening_admission_are_bounded() {
        let (cr, r, a, b, mut p, ps, rr) = fixture();
        for mode in [Mode::SharedCollection, Mode::FirstAccessCollection] {
            for budget in [4, 5, 6, 24] {
                p.budget_seconds = budget;
                let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &rule(mode)).unwrap();
                assert!(
                    (v["terminal_states"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| num(x, "probability"))
                        .sum::<f64>()
                        - 1.0)
                        .abs()
                        < 1e-12
                );
                let attempted = v["opening_collection_audits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| num(x, "reached_probability"))
                    .sum::<f64>();
                assert!((attempted - 0.95).abs() < 1e-12);
                for x in v["opening_collection_audits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|x| x["status"] == "completed")
                {
                    assert!(
                        (num(x, "collector_control_probability")
                            + num(x, "opponent_control_probability")
                            + num(x, "loose_probability")
                            - num(x, "reached_probability"))
                        .abs()
                            < 1e-12
                    );
                }
            }
        }
        p.completed_prefix = 2;
        assert!(inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &rule(Mode::FirstAccessCollection)
        )
        .is_err());
    }
}
