//! Exact convex scenario mixture, NOT a spatial access model or estimated rate.
use super::*;

/// Private components can only be obtained by evaluating verified game inputs.
pub struct Endpoints {
    disabled: serde_json::Value,
    enabled: serde_json::Value,
}
pub fn prepare(
    cr: &game::GameRules,
    r: &super::super::Rules,
    a: &Team,
    b: &Team,
    p: &Plan,
    policies: &HolderPolicies,
    enabled: &super::Rules,
) -> Result<Endpoints, String> {
    enabled.validate()?;
    if enabled.max_contests != 1 {
        return Err("Sensitivity needs the enabled one-window rule".into());
    }
    let mut off = enabled.clone();
    off.max_contests = 0;
    Ok(Endpoints {
        disabled: super::inspect(cr, r, a, b, p, policies, &off)?,
        enabled: super::inspect(cr, r, a, b, p, policies, enabled)?,
    })
}
fn blend_rows(
    a: &serde_json::Value,
    b: &serde_json::Value,
    field: &str,
    measures: &[&str],
    share: f64,
) -> serde_json::Value {
    let mut grouped =
        BTreeMap::<String, (serde_json::Map<String, serde_json::Value>, Vec<f64>)>::new();
    for (v, weight) in [(a, 1.0 - share), (b, share)] {
        if weight == 0.0 {
            continue;
        }
        for row in v[field].as_array().unwrap() {
            let mut identity = row.as_object().unwrap().clone();
            let values: Vec<_> = measures
                .iter()
                .map(|m| identity.remove(*m).unwrap().as_f64().unwrap())
                .collect();
            let key = serde_json::to_string(&identity).unwrap();
            let entry = grouped
                .entry(key)
                .or_insert_with(|| (identity, vec![0.0; measures.len()]));
            for i in 0..values.len() {
                entry.1[i] += weight * values[i];
            }
        }
    }
    grouped
        .into_values()
        .map(|(mut row, values)| {
            for (name, v) in measures.iter().zip(values) {
                row.insert((*name).into(), v.into());
            }
            serde_json::Value::Object(row)
        })
        .collect::<Vec<_>>()
        .into()
}
impl Endpoints {
    pub fn disabled(&self) -> &serde_json::Value {
        &self.disabled
    }
    pub fn enabled(&self) -> &serde_json::Value {
        &self.enabled
    }
    pub fn project(&self, share: f64) -> Result<serde_json::Value, String> {
        if !share.is_finite() || !(0.0..=1.0).contains(&share) {
            return Err("Admission share must be finite in [0,1]".into());
        }
        let a = &self.disabled;
        let b = &self.enabled;
        let scalar =
            |key: &str| (1.0 - share) * a[key].as_f64().unwrap() + share * b[key].as_f64().unwrap();
        let map = |key: &str| {
            a[key]
                .as_object()
                .unwrap()
                .iter()
                .map(|(team, x)| {
                    (
                        team.clone(),
                        serde_json::json!(
                            (1.0 - share) * x.as_f64().unwrap()
                                + share * b[key][team].as_f64().unwrap()
                        ),
                    )
                })
                .collect::<serde_json::Map<_, _>>()
        };
        let action_measures = [
            "reached_probability",
            "completed_probability",
            "expired_probability",
            "win_probability",
            "tie_probability",
            "loss_probability",
        ];
        Ok(
            serde_json::json!({"schema":"rinkcraft.recovery-assumption-mixture.0.1","kind":"authored_game_scenario_not_nhl_estimate",
            "input_sha256":hash(&(&a["input_sha256"],&b["input_sha256"],share))?,"declared_admission_share":share,
            "cost_seconds":b["recovery_rules"]["cost_seconds"],"budget_seconds":b["plan"]["budget_seconds"],"seed":null,"trials":null,
            "component_inputs":{"disabled":a["input_sha256"],"enabled":b["input_sha256"]},
            "goal_probability_by_team":map("goal_probability_by_team"),"terminal_control_probability_by_team":map("terminal_control_probability_by_team"),
            "expected_elapsed_seconds":scalar("expected_elapsed_seconds"),"opening_control_switch_probability":scalar("opening_control_switch_probability"),
            "expected_recovery_control_changes":scalar("expected_recovery_control_changes"),"expected_total_control_changes":scalar("expected_total_control_changes"),
            "terminal_states":blend_rows(a,b,"terminal_states",&["probability"],share),
            "action_audits":blend_rows(a,b,"action_audits",&action_measures,share),
            "task_opportunities":blend_rows(a,b,"task_opportunities",&["expected_contests"],share),
            "policy_decisions":blend_rows(a,b,"policy_decisions",&["reached_probability"],share),
            "recovery_audits":blend_rows(a,b,"recovery_audits",&["reached_probability","same_team_control_probability","other_team_control_probability","unresolved_probability"],share),
            "limits":["Exact declared convex mixture of two fixed-policy start scenarios: (1-share)*closed-loose + share*shared-access recovery",
            "Share is not an estimated frequency, physical access probability, uncertainty interval or one-sided possession model",
            "Unavailable recovery retains the old loose terminal; it does not award exclusive access to either team",
            "Policies are held fixed before selecting the scenario; no access-dependent optimizer or adaptive response",
            "One-window bound remains; no repeated recovery, access race, puck geometry, shot rebounds, empirical NHL calibration or full-match/season conversion",
            "Cost/deadline changes can censor counter shots while retaining opposing control; compare terminal states as well as goals"]}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        game::GameRules,
        super::super::super::Rules,
        Team,
        Team,
        Plan,
        HolderPolicies,
        super::Rules,
    ) {
        let (cr, r, a, b, mut p) = super::super::super::tests::fixture();
        p.first_policy = Choice::ConnectThenShoot;
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
        let rr = super::Rules {
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
    fn endpoint_metrics_and_mixture_scalars_are_exact() {
        let (cr, r, a, b, p, ps, rr) = fixture();
        let e = prepare(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        for share in [0.0, 0.25, 0.5, 1.0] {
            let v = e.project(share).unwrap();
            for key in [
                "expected_elapsed_seconds",
                "opening_control_switch_probability",
                "expected_recovery_control_changes",
                "expected_total_control_changes",
            ] {
                assert_eq!(
                    num(&v, key),
                    (1.0 - share) * num(e.disabled(), key) + share * num(e.enabled(), key)
                );
            }
            for team in ["Alpha", "Beta"] {
                let k = "goal_probability_by_team";
                assert_eq!(
                    v[k][team].as_f64().unwrap(),
                    (1.0 - share) * e.disabled()[k][team].as_f64().unwrap()
                        + share * e.enabled()[k][team].as_f64().unwrap()
                );
            }
        }
        assert_eq!(
            e.project(0.0).unwrap()["goal_probability_by_team"],
            e.disabled()["goal_probability_by_team"]
        );
        assert_eq!(
            e.project(1.0).unwrap()["goal_probability_by_team"],
            e.enabled()["goal_probability_by_team"]
        );
    }
    #[test]
    fn mixed_terminal_clock_and_task_ledgers_conserve() {
        let (cr, r, a, b, p, ps, rr) = fixture();
        let e = prepare(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        let v = e.project(0.25).unwrap();
        let ts = v["terminal_states"].as_array().unwrap();
        assert!((ts.iter().map(|x| num(x, "probability")).sum::<f64>() - 1.0).abs() < 1e-12);
        let elapsed = ts
            .iter()
            .map(|x| num(x, "probability") * num(x, "elapsed_seconds"))
            .sum::<f64>();
        let mut charged = 0.0;
        let mut contests = 0.0;
        for x in v["action_audits"].as_array().unwrap() {
            assert!(
                (num(x, "completed_probability") + num(x, "expired_probability")
                    - num(x, "reached_probability"))
                .abs()
                    < 1e-12
            );
            charged += num(x, "completed_probability") * num(x, "cost_seconds")
                + num(x, "expired_probability") * (20.0 - num(x, "start_seconds"));
            if x["action"] != "send_deep" {
                contests += 2.0 * num(x, "completed_probability");
            }
        }
        assert!((elapsed - num(&v, "expected_elapsed_seconds")).abs() < 1e-12);
        assert!((charged - elapsed).abs() < 1e-12);
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
    #[test]
    fn share_and_enabled_template_admission_are_strict() {
        let (cr, r, a, b, p, ps, mut rr) = fixture();
        let e = prepare(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        for share in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert!(e.project(share).is_err());
        }
        rr.max_contests = 0;
        assert!(prepare(&cr, &r, &a, &b, &p, &ps, &rr).is_err());
        rr.max_contests = 1;
        rr.cost_seconds = 0;
        assert!(prepare(&cr, &r, &a, &b, &p, &ps, &rr).is_err());
    }
    #[test]
    fn input_and_component_custody_remain_unchanged() {
        let (cr, r, a, b, p, ps, rr) = fixture();
        let before = hash(&(&cr, &r, &a, &b, &p, &ps, &rr)).unwrap();
        let e = prepare(&cr, &r, &a, &b, &p, &ps, &rr).unwrap();
        assert_eq!(
            *e.enabled(),
            super::super::inspect(&cr, &r, &a, &b, &p, &ps, &rr).unwrap()
        );
        assert_eq!(before, hash(&(&cr, &r, &a, &b, &p, &ps, &rr)).unwrap());
        assert_eq!(e.project(0.5).unwrap(), e.project(0.5).unwrap());
    }
    #[test]
    fn longer_collection_can_censor_counter_goals_without_removing_control() {
        let (cr, r, a, b, p, ps, mut rr) = fixture();
        let short = prepare(&cr, &r, &a, &b, &p, &ps, &rr)
            .unwrap()
            .project(1.0)
            .unwrap();
        rr.cost_seconds = 8;
        let long = prepare(&cr, &r, &a, &b, &p, &ps, &rr)
            .unwrap()
            .project(1.0)
            .unwrap();
        assert!(
            long["goal_probability_by_team"]["Beta"].as_f64().unwrap()
                < short["goal_probability_by_team"]["Beta"].as_f64().unwrap()
        );
        assert!(
            long["terminal_control_probability_by_team"]["Beta"]
                .as_f64()
                .unwrap()
                > short["terminal_control_probability_by_team"]["Beta"]
                    .as_f64()
                    .unwrap()
        );
    }
}
