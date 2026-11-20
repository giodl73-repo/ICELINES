//! Fixed-profile policy comparisons against declared opponent score edits.
use super::*;
use crate::view_model::rinkcraft_task_season as task;

/// Align the same own profile in reference/proposed; change only the opponent's
/// declared scores. No role, lineup, exposure, clock or policy optimization.
pub fn compare(
    base: &task::Request,
    own: &task::StudyCase,
    opponent: &task::StudyCase,
    rules: &Rules,
    plan: &Plan,
    policies: &HolderPolicies,
) -> Result<serde_json::Value, String> {
    if base.schema != task::SCHEMA
        || base.kind != "authored_game_scenario_not_nhl_estimate"
        || own.team == opponent.team
        || [own, opponent]
            .iter()
            .any(|c| !c.swap_creator_receiver.is_empty() || !c.unit_weights.is_empty())
    {
        return Err(
            "Matchup requires distinct teams, original request identity and score-only cases"
                .into(),
        );
    }
    let mut fixed = task::apply_study_case(base, own)?;
    fixed.reference = fixed.proposed.clone();
    let q = task::apply_study_case(&fixed, opponent)?;
    let get = |teams: &[Team], name: &str| {
        teams
            .iter()
            .find(|x| x.team == name)
            .cloned()
            .ok_or("Missing matchup team")
    };
    let a = inspect_with_policies(
        &q.character_rules,
        rules,
        &get(&q.reference, &own.team)?,
        &get(&q.reference, &opponent.team)?,
        plan,
        policies,
    )?;
    let b = inspect_with_policies(
        &q.character_rules,
        rules,
        &get(&q.proposed, &own.team)?,
        &get(&q.proposed, &opponent.team)?,
        plan,
        policies,
    )?;
    let metric =
        |v: &serde_json::Value, team: &str| v["goal_probability_by_team"][team].as_f64().unwrap();
    let af = metric(&a, &own.team);
    let aa = metric(&a, &opponent.team);
    let bf = metric(&b, &own.team);
    let ba = metric(&b, &opponent.team);
    Ok(
        serde_json::json!({"schema":"rinkcraft.policy-matchup-comparison.0.1","kind":base.kind,
        "input_sha256":hash(&(base,own,opponent,rules,plan,policies))?,"profile":own,"opponent_case":opponent,
        "reference":a,"proposed":b,"reference_goal_for":af,"proposed_goal_for":bf,
        "reference_goal_against":aa,"proposed_goal_against":ba,"goal_for_delta":bf-af,"goal_against_delta":ba-aa,
        "reference_bounded_goal_balance":af-aa,"proposed_bounded_goal_balance":bf-ba,
        "limits":["Same authored own profile and declared policy in reference/proposed; only opponent score edits differ",
        "Score-only interventions; role assignments, joint-unit exposure and costs cannot be edited by this comparison",
        "Menu comparisons are conditional on this bounded opening/clock, not optimized or generally recommended NHL tactics",
        "Bounded goal balance is the difference of mutually exclusive goal probabilities, not NHL xGF%, expected standings points or a season forecast",
        "Existing loose-state termination limits counterattack risk after failed passes; policy-independent counter goals can be structural"]}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn case(
        team: &str,
        id: &str,
        edits: Vec<crate::view_model::rinkcraft_season::ScoreEdit>,
    ) -> task::StudyCase {
        task::StudyCase {
            id: id.into(),
            label: id.into(),
            team: team.into(),
            edits,
            swap_creator_receiver: vec![],
            unit_weights: vec![],
        }
    }
    fn edit(
        id: &str,
        capability: usize,
        score: u8,
    ) -> crate::view_model::rinkcraft_season::ScoreEdit {
        crate::view_model::rinkcraft_season::ScoreEdit {
            member_id: id.into(),
            capability,
            score,
        }
    }
    fn fixture() -> (task::Request, Rules, Plan, HolderPolicies, task::StudyCase) {
        let q = task::tests::fixture();
        let (_, r, _, _, p) = super::super::tests::fixture();
        let ps = HolderPolicies {
            schema: "rinkcraft.holder-policies.0.1".into(),
            version: "0.1.0-playtest".into(),
            kind: q.kind.clone(),
            first: TeamPolicy {
                team: "NYR".into(),
                default: Choice::ShootHolder,
                overrides: vec![],
            },
            second: TeamPolicy {
                team: "CAR".into(),
                default: Choice::ShootHolder,
                overrides: vec![],
            },
        };
        let own = case(
            "NYR",
            "specialists",
            vec![
                edit("NYR:2", 2, 20),
                edit("NYR:2", 3, 1),
                edit("NYR:1", 1, 20),
                edit("NYR:1", 3, 20),
            ],
        );
        (q, r, p, ps, own)
    }
    #[test]
    fn fixed_profile_and_input_identity_are_preserved() {
        let (q, r, p, ps, own) = fixture();
        let before = hash(&q).unwrap();
        let v = compare(&q, &own, &case("CAR", "neutral", vec![]), &r, &p, &ps).unwrap();
        assert_eq!(v["reference"], v["proposed"]);
        assert_eq!(hash(&q).unwrap(), before);
        let fixed = task::apply_study_case(&q, &own).unwrap();
        let a = fixed.proposed.iter().find(|x| x.team == "NYR").unwrap();
        let b = fixed.proposed.iter().find(|x| x.team == "CAR").unwrap();
        assert_eq!(
            v["reference"],
            inspect_with_policies(&q.character_rules, &r, a, b, &p, &ps).unwrap()
        );
    }
    #[test]
    fn strong_lane_reading_reverses_the_specialist_choice() {
        let (q, r, p, shoot, own) = fixture();
        let mut pass = shoot.clone();
        pass.first.overrides.push(HolderChoice {
            member_id: "NYR:2".into(),
            choice: Choice::ConnectThenShoot,
        });
        for (opponent, passing_wins) in [
            (case("CAR", "neutral", vec![]), true),
            (case("CAR", "reading", vec![edit("CAR:3", 4, 20)]), false),
        ] {
            let a = compare(&q, &own, &opponent, &r, &p, &shoot).unwrap();
            let b = compare(&q, &own, &opponent, &r, &p, &pass).unwrap();
            assert_eq!(
                b["reference_goal_for"].as_f64().unwrap()
                    > a["reference_goal_for"].as_f64().unwrap(),
                true
            );
            assert_eq!(
                b["proposed_goal_for"].as_f64().unwrap() > a["proposed_goal_for"].as_f64().unwrap(),
                passing_wins
            );
            assert_eq!(a["proposed_goal_against"], b["proposed_goal_against"]);
        }
    }
    #[test]
    fn unused_opponent_movement_is_not_overall_goalie_quality() {
        let (q, r, p, ps, own) = fixture();
        let v = compare(
            &q,
            &own,
            &case("CAR", "movement", vec![edit("CAR:5", 2, 20)]),
            &r,
            &p,
            &ps,
        )
        .unwrap();
        assert_eq!(v["reference_goal_for"], v["proposed_goal_for"]);
        assert_eq!(v["reference_goal_against"], v["proposed_goal_against"]);
        assert_ne!(
            v["reference"]["input_sha256"],
            v["proposed"]["input_sha256"]
        );
    }
    #[test]
    fn deployment_edits_and_same_team_comparisons_reject() {
        let (q, r, p, ps, own) = fixture();
        assert!(compare(&q, &own, &own, &r, &p, &ps).is_err());
        let mut op = case("CAR", "weights", vec![]);
        op.unit_weights.push(task::UnitWeightEdit {
            unit_id: "test".into(),
            weight: 1.0,
        });
        assert!(compare(&q, &own, &op, &r, &p, &ps).is_err());
        op.unit_weights.clear();
        op.edits.push(edit("CAR:3", 4, 21));
        assert!(compare(&q, &own, &op, &r, &p, &ps).is_err());
    }
}
