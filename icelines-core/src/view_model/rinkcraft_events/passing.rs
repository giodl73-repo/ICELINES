//! A declared skater pass lane and reachable receiver, not inferred geometry.
//! Successful delivery opens a separate, timed Handling collection opportunity.
use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    Delivery,
    Receipt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub task: Task,
    pub actor: String,
    pub opposition: String,
    pub capability_ids: [String; 2],
    pub scores: [u8; 2],
    pub score_rule: contest::ScoreRule,
    pub modifiers: [i16; 2],
    pub exact_counts: [u16; 3],
    pub pair_index: u16,
    pub dice: [u8; 2],
    pub totals: [i16; 2],
    pub outcome: contest::Outcome,
}

pub(super) fn validate(q: &Request, p: &collection::Profiles) -> Result<(), String> {
    for step in &q.steps {
        if let Action::PassUnderPressure {
            passer,
            receiver,
            lane_defender,
            pressure,
            receive_seconds,
        } = &step.action
        {
            if passer == receiver || *receive_seconds == 0 || *receive_seconds > 1200 {
                return Err("pass needs distinct receiver and 1..1200 live receipt seconds".into());
            }
            for id in [passer, receiver, lane_defender, pressure] {
                if collection::profile(p, id)?.branch != game::Branch::Skater {
                    return Err("skater passing cannot substitute goalie capabilities".into());
                }
            }
        }
    }
    Ok(())
}

pub(super) fn resolve(
    p: &collection::Profiles,
    actor: &str,
    opposition: &str,
    task: Task,
    pair_index: u16,
) -> Result<Audit, String> {
    if pair_index >= 400 {
        return Err("pass contest pair index must be 0..399".into());
    }
    let (a, b) = match task {
        Task::Delivery => (2, 4),
        Task::Receipt => (1, 5),
    };
    let ac = &collection::profile(p, actor)?.capabilities[a];
    let bc = &collection::profile(p, opposition)?.capabilities[b];
    let c = contest::Contest {
        actor_score: ac.score,
        opposition_score: bc.score,
        score_rule: p.score_rule,
        actor_adjustment: 0,
        opposition_adjustment: 0,
    };
    let counts = c.exact_counts()?;
    let dice = [(pair_index / 20 + 1) as u8, (pair_index % 20 + 1) as u8];
    let r = c.resolve(dice[0], dice[1])?;
    Ok(Audit {
        task,
        actor: actor.into(),
        opposition: opposition.into(),
        capability_ids: [ac.id.clone(), bc.id.clone()],
        scores: [ac.score, bc.score],
        score_rule: p.score_rule,
        modifiers: c.terms()?,
        exact_counts: [counts.actor_wins, counts.stalemates, counts.opposition_wins],
        pair_index,
        dice,
        totals: [r.actor_total, r.opposition_total],
        outcome: r.outcome,
    })
}
