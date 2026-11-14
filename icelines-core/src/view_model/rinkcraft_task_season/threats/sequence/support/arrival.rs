//! Authored arrival times and commitments; no conversion of capabilities to speed.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Commitment {
    Hold,
    Arrive { after_seconds: u16 },
}
impl Commitment {
    fn delay(&self) -> Option<u16> {
        match self {
            Self::Hold => None,
            Self::Arrive { after_seconds } => Some(*after_seconds),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub first: Commitment,
    pub second: Commitment,
    pub puck_challenge_score: u8,
}
impl Rules {
    pub(in super::super) fn wait_seconds(&self) -> Option<u16> {
        self.first
            .delay()
            .into_iter()
            .chain(self.second.delay())
            .min()
    }
    fn selection(&self, cost: u16) -> (&'static str, Option<usize>, bool) {
        let a = self.first.delay();
        let b = self.second.delay();
        if a.is_none() && b.is_none() {
            return ("uncommitted", None, false);
        }
        if a == b {
            return ("shared", None, false);
        }
        let side = if a.is_some() && (b.is_none() || a.unwrap() < b.unwrap()) {
            0
        } else {
            1
        };
        let other = if side == 0 { b } else { a };
        let pressured = other.is_some_and(|d| d <= self.wait_seconds().unwrap() + cost);
        (
            match (side, pressured) {
                (0, true) => "first_pressured",
                (0, false) => "first_unpressured",
                (1, true) => "second_pressured",
                _ => "second_unpressured",
            },
            Some(side),
            pressured,
        )
    }
}
pub fn inspect(
    cr: &game::GameRules,
    r: &super::super::Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
    ps: &HolderPolicies,
    rr: &recovery::Rules,
    ar: &access::Rules,
    sr: &super::Rules,
    arr: &Rules,
) -> Result<serde_json::Value, String> {
    access::validate(ar, p)?;
    super::validate(a, b, ar, sr)?;
    rr.validate()?;
    ps.validate(a, b, p)?;
    if arr.schema != "rinkcraft.support-arrival-rules.0.1"
        || arr.version != "0.1.0-playtest"
        || arr.kind != "authored_game_scenario_not_nhl_estimate"
        || arr.first.delay().is_some_and(|d| d > 30)
        || arr.second.delay().is_some_and(|d| d > 30)
        || !(1..=20).contains(&arr.puck_challenge_score)
        || sr.max_contests != 1
    {
        return Err(
            "Arrival needs explicit commitments, delays 0-30, challenge 1-20 and enabled support"
                .into(),
        );
    }
    inspect_inner(
        cr,
        r,
        a,
        b,
        p,
        Some(ps),
        Some(rr),
        Some(ar),
        Some(sr),
        Some(arr),
    )
}
pub(super) fn resolve(
    mut s: State,
    w: f64,
    race_side: usize,
    trigger: &str,
    p: &Plan,
    r: &super::super::Rules,
    units: [&super::super::super::super::Unit; 2],
    teams: [&Team; 2],
    members: [&BTreeMap<&str, &game::GameCharacter>; 2],
    t: &super::super::super::super::Table,
    l: &mut Ledger,
    ps: Option<&HolderPolicies>,
    rr: Option<&recovery::Rules>,
    sr: &super::Rules,
    arr: &Rules,
) {
    let (mode, side, pressured) = arr.selection(sr.cost_seconds);
    let wait = arr.wait_seconds();
    let start = s.elapsed;
    let status = if wait.is_none() {
        "uncommitted"
    } else if start + wait.unwrap() > p.budget_seconds {
        "wait_expired"
    } else {
        mode
    };
    *l.arrivals
        .entry((
            teams[race_side].team.clone(),
            trigger.into(),
            start,
            status.into(),
        ))
        .or_default() += w;
    if wait.is_none() {
        l.end(w, "support_uncommitted", s, units, teams, false);
        return;
    }
    let wait = wait.unwrap();
    if wait > 0 {
        if start + wait > p.budget_seconds {
            l.trace(
                "support_arrive",
                s,
                "",
                "",
                "",
                &teams[side.unwrap_or(0)].team,
                [w, 0.0, w, 0.0, 0.0, 0.0],
            );
            s.elapsed = p.budget_seconds;
            l.end(w, "time_expired", s, units, teams, false);
            return;
        }
        l.trace(
            "support_arrive",
            s,
            "",
            "",
            "",
            &teams[side.unwrap_or(0)].team,
            [w, w, 0.0, w, 0.0, 0.0],
        );
        s.elapsed += wait;
    }
    if mode == "shared" {
        super::resolve(
            s, w, race_side, trigger, p, r, units, teams, members, t, l, ps, rr, sr, None,
        );
        return;
    }
    let side = side.unwrap();
    let other = 1 - side;
    let slots = [Slot::Receiver, Slot::Gap];
    let ids = [slots[0].id(units[0]), slots[1].id(units[1])];
    let aid = ids[side];
    let bid = if pressured { ids[other] } else { "" };
    let action = if side == 0 {
        "support_collect_first"
    } else {
        "support_collect_second"
    };
    let status = if s.elapsed + sr.cost_seconds > p.budget_seconds {
        "clock_expired"
    } else {
        "completed"
    };
    let key = (
        teams[race_side].team.clone(),
        trigger.into(),
        ids[0].into(),
        ids[1].into(),
        s.elapsed,
        status.into(),
    );
    if status == "clock_expired" {
        let audit = l.supports.entry(key).or_default();
        audit[0] += w;
        audit[3] += w;
        l.trace(
            action,
            s,
            aid,
            bid,
            "",
            &teams[side].team,
            [w, 0.0, w, 0.0, 0.0, 0.0],
        );
        s.elapsed = p.budget_seconds;
        l.end(w, "time_expired", s, units, teams, false);
        return;
    }
    let opposing_score = if pressured {
        members[other][bid].capabilities[5].score
    } else {
        arr.puck_challenge_score
    };
    let q = t[members[side][aid].capabilities[1].score as usize][opposing_score as usize];
    let audit = l.supports.entry(key).or_default();
    audit[0] += w;
    audit[1 + side] += w * q[0];
    audit[3] += w * (q[1] + q[2]);
    l.trace(
        action,
        s,
        aid,
        bid,
        "",
        &teams[side].team,
        [w, w, 0.0, w * q[0], w * q[1], w * q[2]],
    );
    l.task(&teams[side].team, aid, "Handling", w);
    if pressured {
        l.task(&teams[other].team, bid, "Defending", w);
    }
    s.elapsed += sr.cost_seconds;
    s.control = Control::Loose;
    s.receive = false;
    l.end(
        w * q[1],
        "support_collection_unresolved",
        s,
        units,
        teams,
        false,
    );
    l.end(
        w * q[2],
        "support_collection_failed",
        s,
        units,
        teams,
        false,
    );
    s.side = side;
    s.slot = slots[side];
    s.control = Control::Attacking;
    if side == 1 {
        l.switches += w * q[0];
    }
    follow(
        s,
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
}
pub(in super::super) fn describe(
    result: &mut serde_json::Value,
    arr: &Rules,
    sr: &super::Rules,
    rows: BTreeMap<(String, String, u16, String), f64>,
) -> Result<(), String> {
    let (mode, side, pressured) = arr.selection(sr.cost_seconds);
    result["schema"] = "rinkcraft.support-arrival-inspection.0.1".into();
    result["support_arrival_rules"] = serde_json::to_value(arr).map_err(|e| e.to_string())?;
    result["arrival_context"] = serde_json::json!({"mode":mode,"wait_seconds":arr.wait_seconds(),"collection_seconds":sr.cost_seconds,
        "clock_origin":"completed first collection ends loose","pressure_boundary":"opponent arrival <= collector start + collection cost",
        "unpressured_challenge_score":arr.puck_challenge_score});
    result["arrival_audits"]=rows.into_iter().map(|((race,trigger,start,status),w)|serde_json::json!({"race_winner_team":race,"trigger_outcome":trigger,"start_seconds":start,"status":status,"reached_probability":w})).collect::<Vec<_>>().into();
    for row in result["support_audits"].as_array_mut().unwrap() {
        let a = row.as_object_mut().unwrap().remove("collector_id").unwrap();
        let b = row
            .as_object_mut()
            .unwrap()
            .remove("opposing_collector_id")
            .unwrap();
        let (actor, opponent) = if side == Some(1) {
            (b.clone(), a.clone())
        } else {
            (a.clone(), b.clone())
        };
        row["first_candidate_id"] = a;
        row["second_candidate_id"] = b;
        row["handling_actor_id"] = actor;
        row["opposing_actor_id"] = if mode == "shared" || pressured {
            opponent
        } else {
            serde_json::Value::Null
        };
        row["opposing_capability"] = if mode == "shared" {
            "Handling"
        } else if pressured {
            "Defending"
        } else {
            "PuckChallenge"
        }
        .into();
        row["collection_mode"] = mode.into();
    }
    result["support_limits"][0]="Support availability uses explicit authored commitments and arrival delays; no travel physics or F2/F3 assignment inferred".into();
    result["support_limits"][1]="Equal arrivals use shared Handling; earlier collector uses Handling vs present Defending, or an external puck challenge if unpressured; failure stays loose".into();
    result["arrival_limits"]=serde_json::json!(["Arrival delays are scenario inputs, not measured proximity or converted Skating ratings; waiting gives no Skating task credit",
        "Hold means deliberately unavailable to this support window, not missing data; no cover benefit, fatigue or later tactical cost is inferred",
        "Only the first eligible support collection occurs; a later arrival does not receive automatic possession after failure",
        "The unpressured puck challenge uses a fresh authored d20 comparison; it is an environmental game parameter, not another player's rating",
        "No goalie exchange, rebound, continuous game or season integration; previous rules remain separate"]);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rules(a: Commitment, b: Commitment) -> Rules {
        Rules {
            schema: "rinkcraft.support-arrival-rules.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: "authored_game_scenario_not_nhl_estimate".into(),
            first: a,
            second: b,
            puck_challenge_score: 10,
        }
    }
    fn at(d: u16) -> Commitment {
        Commitment::Arrive { after_seconds: d }
    }
    fn n(v: &serde_json::Value, k: &str) -> f64 {
        v[k].as_f64().unwrap()
    }
    fn change(cr: &game::GameRules, a: &mut Team, id: &str, cap: usize) {
        let m = a.members.iter_mut().find(|m| m.id == id).unwrap();
        let c = &m.character;
        let mut scores = [10; 6];
        scores[cap] = 16;
        m.character = game::build(
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
    fn zero_equal_arrivals_preserve_shared_support_mechanics() {
        let (cr, r, a, b, p, ps, rr, ar, sr) = super::super::tests::fixture();
        let arr = rules(at(0), at(0));
        let old = super::super::inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr).unwrap();
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).unwrap();
        for k in [
            "terminal_states",
            "action_audits",
            "task_opportunities",
            "goal_probability_by_team",
            "expected_elapsed_seconds",
            "first_possession_probability_by_team",
            "opening_control_switch_probability",
        ] {
            assert_eq!(v[k], old[k], "{k}");
        }
    }
    #[test]
    fn arrival_wait_and_collection_must_each_fit() {
        let (cr, r, a, b, mut p, ps, rr, ar, sr) = super::super::tests::fixture();
        let arr = rules(at(2), at(2));
        for budget in [7, 8, 9, 10] {
            p.budget_seconds = budget;
            let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).unwrap();
            if budget == 7 {
                assert!(v["support_audits"].as_array().unwrap().is_empty());
                assert!(v["arrival_audits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|x| x["status"] == "wait_expired"));
            }
            if budget < 10 {
                assert!(!v["task_opportunities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(
                        |x| x["member_id"] == a.units[0].receiver && x["capability"] == "Handling"
                    ));
            } else {
                assert!(
                    (v["first_possession_probability_by_team"][&a.team]
                        .as_f64()
                        .unwrap()
                        - 0.46253125)
                        .abs()
                        < 1e-12
                );
            }
        }
    }
    #[test]
    fn two_holds_do_not_invent_support_or_task_credit() {
        let (cr, r, a, b, p, ps, rr, ar, sr) = super::super::tests::fixture();
        let arr = rules(Commitment::Hold, Commitment::Hold);
        let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).unwrap();
        assert!(v["support_audits"].as_array().unwrap().is_empty());
        for team in [&a.team, &b.team] {
            assert!(
                (v["first_possession_probability_by_team"][team]
                    .as_f64()
                    .unwrap()
                    - 0.225625)
                    .abs()
                    < 1e-12
            );
        }
        assert!(!v["action_audits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["action"].as_str().unwrap().starts_with("support_")));
    }
    #[test]
    fn pressure_boundary_is_inclusive_and_absent_defender_has_no_credit() {
        let (cr, r, a, b, p, ps, rr, ar, sr) = super::super::tests::fixture();
        let yes = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &ar,
            &sr,
            &rules(at(0), at(2)),
        )
        .unwrap();
        let no = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &ar,
            &sr,
            &rules(at(0), at(3)),
        )
        .unwrap();
        assert_eq!(yes["arrival_context"]["mode"], "first_pressured");
        assert_eq!(no["arrival_context"]["mode"], "first_unpressured");
        assert!(yes["task_opportunities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["member_id"] == b.units[0].gap_defender && x["capability"] == "Defending"));
        assert!(!no["task_opportunities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["member_id"] == b.units[0].gap_defender));
        assert!(no["support_audits"]
            .as_array()
            .unwrap()
            .iter()
            .all(
                |x| x["opposing_actor_id"].is_null() && x["opposing_capability"] == "PuckChallenge"
            ));
    }
    #[test]
    fn present_defending_changes_collection_but_absent_defending_does_not() {
        let (cr, r, a, mut b, p, ps, rr, ar, sr) = super::super::tests::fixture();
        let id = b.units[0].gap_defender.clone();
        let before = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &ar,
            &sr,
            &rules(at(0), at(3)),
        )
        .unwrap();
        change(&cr, &mut b, &id, 5);
        let clear = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &ar,
            &sr,
            &rules(at(0), at(3)),
        )
        .unwrap();
        let pressured = inspect(
            &cr,
            &r,
            &a,
            &b,
            &p,
            &ps,
            &rr,
            &ar,
            &sr,
            &rules(at(0), at(1)),
        )
        .unwrap();
        assert_eq!(
            before["goal_probability_by_team"],
            clear["goal_probability_by_team"]
        );
        assert_eq!(before["task_opportunities"], clear["task_opportunities"]);
        assert!(
            pressured["first_possession_probability_by_team"][&a.team]
                .as_f64()
                .unwrap()
                < clear["first_possession_probability_by_team"][&a.team]
                    .as_f64()
                    .unwrap()
        );
    }
    #[test]
    fn unpressured_collection_is_not_automatic_and_is_symmetric() {
        let (cr, r, a, b, p, ps, rr, ar, sr) = super::super::tests::fixture();
        for (first, second, own, other) in [
            (at(0), Commitment::Hold, &a.team, &b.team),
            (Commitment::Hold, at(2), &b.team, &a.team),
        ] {
            let mut arr = rules(first, second);
            let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).unwrap();
            for x in v["action_audits"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["action"] == "support_arrive")
            {
                assert_eq!(x["team"], *own);
                assert_eq!(x["actor_id"], "");
            }
            assert!(
                (v["first_possession_probability_by_team"][own]
                    .as_f64()
                    .unwrap()
                    - 0.46253125)
                    .abs()
                    < 1e-12
            );
            assert!(
                (v["first_possession_probability_by_team"][other]
                    .as_f64()
                    .unwrap()
                    - 0.225625)
                    .abs()
                    < 1e-12
            );
            arr.puck_challenge_score = 20;
            let hard = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).unwrap();
            assert!(
                hard["first_possession_probability_by_team"][own]
                    .as_f64()
                    .unwrap()
                    < v["first_possession_probability_by_team"][own]
                        .as_f64()
                        .unwrap()
            );
        }
    }
    #[test]
    fn missing_commitments_invalid_scores_and_disabled_support_reject() {
        let (cr, r, a, b, p, ps, rr, ar, mut sr) = super::super::tests::fixture();
        let mut arr = rules(at(0), at(0));
        for score in [0, 21] {
            arr.puck_challenge_score = score;
            assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).is_err());
        }
        arr.puck_challenge_score = 10;
        arr.first = at(31);
        assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).is_err());
        arr.first = at(0);
        sr.max_contests = 0;
        assert!(inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).is_err());
        let mut v = serde_json::to_value(arr).unwrap();
        v.as_object_mut().unwrap().remove("second");
        assert!(serde_json::from_value::<Rules>(v).is_err());
    }
    #[test]
    fn terminal_clock_and_task_ledgers_conserve_across_arrival_regimes() {
        let (cr, r, a, b, mut p, ps, rr, ar, sr) = super::super::tests::fixture();
        for (first, second) in [
            (at(0), at(0)),
            (at(2), at(2)),
            (at(0), at(1)),
            (at(0), at(3)),
            (at(3), at(0)),
            (at(0), Commitment::Hold),
            (Commitment::Hold, Commitment::Hold),
        ] {
            let arr = rules(first, second);
            let (mode, _, pressured) = arr.selection(sr.cost_seconds);
            for budget in [4, 6, 7, 8, 10, 24] {
                p.budget_seconds = budget;
                let v = inspect(&cr, &r, &a, &b, &p, &ps, &rr, &ar, &sr, &arr).unwrap();
                let mut mass = 0.0;
                let mut seconds = 0.0;
                for x in v["terminal_states"].as_array().unwrap() {
                    mass += n(x, "probability");
                    seconds += n(x, "probability") * n(x, "elapsed_seconds");
                }
                assert!((mass - 1.0).abs() < 1e-12);
                assert!((seconds - n(&v, "expected_elapsed_seconds")).abs() < 1e-12);
                let mut charged = 0.0;
                let mut tasks = 0.0;
                for x in v["action_audits"].as_array().unwrap() {
                    let c = n(x, "completed_probability");
                    charged += c * n(x, "cost_seconds")
                        + n(x, "expired_probability") * (budget as f64 - n(x, "start_seconds"));
                    let name = x["action"].as_str().unwrap();
                    if !matches!(name, "send_deep" | "support_arrive") {
                        tasks += c * if name.starts_with("support_collect_")
                            && mode != "shared"
                            && !pressured
                        {
                            1.0
                        } else {
                            2.0
                        };
                    }
                }
                assert!((charged - seconds).abs() < 1e-12);
                assert!(
                    (v["task_opportunities"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| n(x, "expected_contests"))
                        .sum::<f64>()
                        - tasks)
                        .abs()
                        < 1e-12
                );
            }
        }
    }
}
