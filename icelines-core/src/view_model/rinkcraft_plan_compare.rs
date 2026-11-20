//! Conditional comparison on a recorded fictional path, never an adaptive strategy estimate.
use super::rinkcraft_possession::{
    import_replay, replay, replay_fingerprint, Action, Event, Phase, ReplayFile, Session,
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct BlockedStep {
    pub step: usize,
    pub requested_action: Action,
    pub phase: Phase,
    pub legal_actions: Vec<Action>,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct PlanRun {
    pub commit_support: bool,
    pub replayed_entire_recorded_path: bool,
    pub blocked: Option<BlockedStep>,
    pub session: Session,
}
#[derive(Clone, Debug, Serialize)]
pub struct PairedStep {
    pub step: usize,
    pub recorded_action: Action,
    pub hold: Option<Event>,
    pub commit: Option<Event>,
    pub same_dice: Option<bool>,
    pub terms_differ: Option<bool>,
    pub outcome_differs: Option<bool>,
    pub hold_blocks_here: bool,
    pub commit_blocks_here: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Comparison {
    pub schema: String,
    pub source_replay_sha256: String,
    pub seed: u32,
    pub recorded_steps: usize,
    pub first_terms_difference: Option<usize>,
    pub first_outcome_difference: Option<usize>,
    pub hold: PlanRun,
    pub commit: PlanRun,
    pub rows: Vec<PairedStep>,
    pub setup_note: String,
    pub interpretation: String,
}
fn run_plan(file: &ReplayFile, commit_support: bool) -> Result<PlanRun, String> {
    let mut request = file.request.clone();
    request.commit_support = commit_support;
    request.actions.clear();
    let mut session = replay(&file.character_rules, &file.rules, request.clone())?;
    let mut blocked = None;
    for (index, action) in file.request.actions.iter().enumerate() {
        if !session.legal_actions.contains(action) {
            blocked=Some(BlockedStep {step:index+1,requested_action:*action,phase:session.state.phase,legal_actions:session.legal_actions.clone(),reason:"The next recorded action is unavailable under this plan. No replacement action or dice were invented.".into()});
            break;
        }
        request.actions.push(*action);
        session = replay(&file.character_rules, &file.rules, request.clone())?;
    }
    Ok(PlanRun {
        commit_support,
        replayed_entire_recorded_path: blocked.is_none(),
        blocked,
        session,
    })
}
pub fn compare_replay(raw: &str) -> Result<Comparison, String> {
    let file = import_replay(raw)?;
    let hold = run_plan(&file, false)?;
    let commit = run_plan(&file, true)?;
    let mut rows = Vec::new();
    let mut first_terms_difference = None;
    let mut first_outcome_difference = None;
    for (index, action) in file.request.actions.iter().enumerate() {
        let a = hold.session.events.get(index);
        let b = commit.session.events.get(index);
        let same_dice = a.zip(b).map(|(a, b)| {
            a.resolution.actor_die == b.resolution.actor_die
                && a.resolution.opposition_die == b.resolution.opposition_die
        });
        let terms_differ = a.zip(b).map(|(a, b)| a.terms != b.terms);
        // Cover is a declared setup difference. Do not mislabel it as a roll outcome change.
        let outcome_differs = a.zip(b).map(|(a, b)| {
            a.resolution.outcome != b.resolution.outcome
                || a.after.phase != b.after.phase
                || a.after.control != b.after.control
                || a.after.loose != b.after.loose
        });
        if terms_differ == Some(true) && first_terms_difference.is_none() {
            first_terms_difference = Some(index + 1)
        }
        if outcome_differs == Some(true) && first_outcome_difference.is_none() {
            first_outcome_difference = Some(index + 1)
        }
        rows.push(PairedStep {
            step: index + 1,
            recorded_action: *action,
            hold: a.cloned(),
            commit: b.cloned(),
            same_dice,
            terms_differ,
            outcome_differs,
            hold_blocks_here: hold.blocked.as_ref().is_some_and(|x| x.step == index + 1),
            commit_blocks_here: commit.blocked.as_ref().is_some_and(|x| x.step == index + 1),
        });
    }
    Ok(Comparison {schema:"icelines.possession-plan-comparison.v1".into(),source_replay_sha256:replay_fingerprint(&file)?,seed:file.request.seed,recorded_steps:file.request.actions.len(),first_terms_difference,first_outcome_difference,hold,commit,rows,setup_note:"Hold keeps F3 high cover; commit moves F3 into attack support and vacates that cover. This setup difference exists before any roll.".into(),interpretation:"Both plans use the same fictional hero, rules, seed and recorded actions. Stop a plan at its first unavailable action; do not invent an adaptive continuation. Outcomes are conditional on this selected path, not whole-scene strategy value, game balance or NHL causal effects.".into()})
}
pub fn compare_replay_from_json(raw: &str) -> Result<String, String> {
    serde_json::to_string(&compare_replay(raw)?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::rinkcraft_game::{build, BuildRequest, GameRules, ScoreMethod};
    use crate::view_model::rinkcraft_possession::{export_replay_from_json, Request, Rules};
    fn fixture() -> (GameRules, Rules, Request) {
        let templates:Vec<_>=[("s","skater"),("g","goalie")].iter().map(|(id,branch)|serde_json::json!({"id":id,"branch":branch,"class_id":"test","class_name":"Test","specialty":"Test","description":"Fictional fixture","priority":[0,1,2,3,4,5],"task_skills":[],"methods":[],"tendencies":[],"playbook_options":[]})).collect();
        let c:GameRules=serde_json::from_value(serde_json::json!({"schema":"rinkcraft.game-rules.v1","version":"test","title":"Test","standard_array":[12,12,12,12,12,12],"templates":templates})).unwrap();
        let hero = build(
            &c,
            BuildRequest {
                template_id: "s".into(),
                name: "Hero".into(),
                position: "F".into(),
                scores: ScoreMethod::StandardArray,
                inspiration: None,
            },
        )
        .unwrap();
        (
            c,
            Rules {
                schema: "rinkcraft.possession-rules.v1".into(),
                version: "0.1.0-playtest".into(),
                support_bonus: 2,
            },
            Request {
                hero,
                seed: 0,
                commit_support: false,
                actions: vec![],
            },
        )
    }
    fn saved(c: &GameRules, r: &Rules, q: &Request) -> String {
        export_replay_from_json(
            &serde_json::to_string(c).unwrap(),
            &serde_json::to_string(r).unwrap(),
            &serde_json::to_string(q).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn stops_at_illegal_alternate_path_without_inventing_a_roll() {
        let (c, r, mut q) = fixture();
        let mut found = false;
        for seed in 0..256 {
            q.seed = seed;
            q.actions = vec![Action::Outlet];
            let h = replay(&c, &r, q.clone()).unwrap();
            let mut other = q.clone();
            other.commit_support = true;
            let a = replay(&c, &r, other).unwrap();
            if h.state.phase == Phase::Loose && a.state.phase == Phase::Breakout {
                q.actions.push(Action::Recover);
                let result = compare_replay(&saved(&c, &r, &q)).unwrap();
                assert!(result.hold.replayed_entire_recorded_path);
                assert!(!result.commit.replayed_entire_recorded_path);
                assert_eq!(result.commit.blocked.unwrap().step, 2);
                assert_eq!(result.commit.session.events.len(), 1);
                assert_eq!(result.first_outcome_difference, Some(1));
                assert_eq!(result.rows[0].same_dice, Some(true));
                assert!(result.rows[1].commit.is_none());
                assert_eq!(result.rows[1].same_dice, None);
                assert!(result.rows[1].commit_blocks_here);
                found = true;
                break;
            }
        }
        assert!(found);
    }
    #[test]
    fn counter_comparison_uses_same_dice_and_finds_outcome_not_setup_difference() {
        let (c, r, mut q) = fixture();
        let mut found = false;
        for seed in 0..1000 {
            q.seed = seed;
            q.actions = vec![Action::Outlet, Action::Recover, Action::Counter];
            if let Ok(h) = replay(&c, &r, q.clone()) {
                let mut other = q.clone();
                other.commit_support = true;
                if let Ok(a) = replay(&c, &r, other) {
                    if h.state.phase == Phase::CounterStopped
                        && a.state.phase == Phase::CounterAdvanced
                    {
                        let raw = saved(&c, &r, &q);
                        let before = raw.clone();
                        let result = compare_replay(&raw).unwrap();
                        assert_eq!(raw, before);
                        assert_eq!(result.first_terms_difference, Some(1));
                        assert_eq!(result.first_outcome_difference, Some(3));
                        assert!(result.rows.iter().all(|x| x.same_dice == Some(true)));
                        assert!(
                            result.hold.replayed_entire_recorded_path
                                && result.commit.replayed_entire_recorded_path
                        );
                        found = true;
                        break;
                    }
                }
            }
        }
        assert!(found);
    }
    #[test]
    fn zero_bonus_preserves_arithmetic_and_outcomes_despite_different_cover_setup() {
        let (c, mut r, mut q) = fixture();
        r.support_bonus = 0;
        q.actions = vec![Action::Outlet];
        let raw = saved(&c, &r, &q);
        let result = compare_replay(&raw).unwrap();
        assert_eq!(result.first_terms_difference, None);
        assert_eq!(result.first_outcome_difference, None);
        assert_ne!(
            result.hold.session.state.cover_owner,
            result.commit.session.state.cover_owner
        );
        assert!(result.rows.iter().all(|x| x.same_dice == Some(true)));
    }
    #[test]
    fn empty_history_is_setup_only_and_invalid_sources_are_rejected() {
        let (c, r, q) = fixture();
        let raw = saved(&c, &r, &q);
        let result = compare_replay(&raw).unwrap();
        assert_eq!(result.recorded_steps, 0);
        assert!(result.rows.is_empty());
        assert_eq!(result.first_outcome_difference, None);
        assert!(
            result.hold.replayed_entire_recorded_path
                && result.commit.replayed_entire_recorded_path
        );
        let mut bad: serde_json::Value = serde_json::from_str(&raw).unwrap();
        bad["session"]["state"]["control"] = serde_json::json!("away:G");
        assert!(compare_replay(&bad.to_string()).is_err());
        assert!(compare_replay(&"x".repeat(262145)).is_err());
        assert!(compare_replay("{}").is_err());
    }
}
