//! A declared live shot reaches an opposing active goalie. Fictional capability
//! contests and conditional physical outcomes remain separate, authored rules.
use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Finish {
    Goal,
    Post,
    Loose,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Containment {
    Hold,
    Rebound,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub shooter: String,
    pub goalie: String,
    pub finish: Vec<Choice<Finish>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub containment: Vec<Choice<Containment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rebound_context: Option<rebound::Context>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub shooter: String,
    pub goalie: String,
    pub capability_ids: [String; 2],
    pub scores: [u8; 2],
    pub score_rule: contest::ScoreRule,
    pub modifiers: [i16; 2],
    pub exact_counts: [u16; 3],
    pub pair_index: u16,
    pub dice: [u8; 2],
    pub totals: [i16; 2],
    pub outcome: contest::Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goalie_adjustment: Option<i8>,
}
fn weights<T: Serialize>(choices: &[Choice<T>]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut sum = 0u32;
    for c in choices {
        if c.weight == 0
            || !seen.insert(serde_json::to_string(&c.outcome).map_err(|e| e.to_string())?)
        {
            return Err("shot branches need distinct positive outcomes".into());
        }
        sum += u32::from(c.weight);
    }
    if sum != 10_000 {
        return Err("shot branch weights must total 10000".into());
    }
    Ok(())
}
pub(super) fn validate(q: &Request, p: &collection::Profiles) -> Result<(), String> {
    for step in &q.steps {
        if let Action::ResolveShot { plan } = &step.action {
            validate_plan(&q.schema, p, plan)?;
        }
    }
    Ok(())
}
pub(super) fn validate_plan(
    schema: &str,
    p: &collection::Profiles,
    plan: &Plan,
) -> Result<(), String> {
    if !matches!(
        schema,
        "rinkcraft.event-episode.0.5"
            | "rinkcraft.event-episode.0.6"
            | "rinkcraft.event-episode.0.7"
    ) {
        return Err("shot contest needs supported explicit version".into());
    }
    if collection::profile(p, &plan.shooter)?.branch != game::Branch::Skater
        || collection::profile(p, &plan.goalie)?.branch != game::Branch::Goalie
    {
        return Err("shot task requires a skater and a verified goalie profile".into());
    }
    weights(&plan.finish)?;
    match (schema, &plan.rebound_context) {
        ("rinkcraft.event-episode.0.5", None) => weights(&plan.containment)?,
        ("rinkcraft.event-episode.0.6" | "rinkcraft.event-episode.0.7", Some(context))
            if plan.containment.is_empty() =>
        {
            rebound::validate(context)?
        }
        _ => return Err("use exclusive containment policy for selected episode version".into()),
    }
    Ok(())
}
pub(super) fn resolve(
    p: &collection::Profiles,
    shooter: &str,
    goalie: &str,
    pair_index: u16,
) -> Result<Audit, String> {
    resolve_adjusted(p, shooter, goalie, pair_index, None)
}
pub(super) fn resolve_adjusted(
    p: &collection::Profiles,
    shooter: &str,
    goalie: &str,
    pair_index: u16,
    goalie_adjustment: Option<i8>,
) -> Result<Audit, String> {
    if goalie_adjustment.is_some_and(|a| !(-20..=0).contains(&a)) {
        return Err("late goalie adjustment must be -20..0".into());
    }
    if pair_index >= 400 {
        return Err("shot contest pair index must be 0..399".into());
    }
    let a = &collection::profile(p, shooter)?.capabilities[3];
    let b = &collection::profile(p, goalie)?.capabilities[0];
    let c = contest::Contest {
        actor_score: a.score,
        opposition_score: b.score,
        score_rule: p.score_rule,
        actor_adjustment: 0,
        opposition_adjustment: goalie_adjustment.unwrap_or(0),
    };
    let counts = c.exact_counts()?;
    let dice = [(pair_index / 20 + 1) as u8, (pair_index % 20 + 1) as u8];
    let r = c.resolve(dice[0], dice[1])?;
    Ok(Audit {
        shooter: shooter.into(),
        goalie: goalie.into(),
        capability_ids: [a.id.clone(), b.id.clone()],
        scores: [a.score, b.score],
        score_rule: p.score_rule,
        modifiers: c.terms()?,
        exact_counts: [counts.actor_wins, counts.stalemates, counts.opposition_wins],
        pair_index,
        dice,
        totals: [r.actor_total, r.opposition_total],
        outcome: r.outcome,
        goalie_adjustment,
    })
}
impl Engine<'_> {
    pub(super) fn resolve_shot(&mut self, i: usize, plan: &Plan) -> Result<(), String> {
        self.resolve_shot_adjusted(i, plan, None)
    }
    pub(super) fn resolve_shot_adjusted(
        &mut self,
        i: usize,
        plan: &Plan,
        adjustment: Option<i8>,
    ) -> Result<(), String> {
        if self.state.puck != Puck::Flight || self.state.attempt.is_none() {
            return Err("shot contest requires a live in-flight attempt".into());
        }
        let a = self.active(&plan.shooter)?;
        let b = self.active(&plan.goalie)?;
        if a.goalie || !b.goalie || a.side == b.side {
            return Err("shot contest requires opposing active skater/goalie".into());
        }
        if self.state.last_deliberate.as_deref() != Some(plan.shooter.as_str())
            || self.shooter_side != Some(a.side)
        {
            return Err("shot contest shooter must match the attempt's deliberate release".into());
        }
        let pair = self.below(400);
        let audit = if adjustment.is_none() {
            resolve(
                self.q.profiles.as_ref().unwrap(),
                &plan.shooter,
                &plan.goalie,
                pair,
            )?
        } else {
            resolve_adjusted(
                self.q.profiles.as_ref().unwrap(),
                &plan.shooter,
                &plan.goalie,
                pair,
                adjustment,
            )?
        };
        let outcome = audit.outcome;
        self.emit(i, "shot_contest", Some(plan.shooter.clone()), None, None);
        self.events.last_mut().unwrap().shot_task = Some(audit);
        match outcome {
            contest::Outcome::ActorWins => {
                let (n, draw) = self.choose(&plan.finish)?;
                self.emit(i, "shot_finish_choice", None, Some(draw), None);
                match plan.finish[n].outcome {
                    Finish::Goal => self.record_goal(i, None)?,
                    Finish::Post => self.emit(i, "post_contact", None, None, None),
                    Finish::Loose => {
                        self.state.puck = Puck::Loose;
                        self.emit(i, "shot_loose", None, None, None);
                    }
                }
            }
            contest::Outcome::Stalemate => {
                self.state.puck = Puck::Loose;
                self.emit(i, "shot_contested_loose", None, None, None);
            }
            contest::Outcome::OppositionWins => {
                self.state.puck = Puck::Loose;
                self.state.last_touch = Some(plan.goalie.clone());
                self.emit(i, "goalie_contact", Some(plan.goalie.clone()), None, None);
                if let Some(context) = &plan.rebound_context {
                    return self.contain_contact(i, &plan.goalie, context);
                }
                let (n, draw) = self.choose(&plan.containment)?;
                self.emit(
                    i,
                    "goalie_containment_choice",
                    Some(plan.goalie.clone()),
                    Some(draw),
                    None,
                );
                match plan.containment[n].outcome {
                    Containment::Hold => self.whistle(
                        i,
                        format!("goalie_hold:{}", self.player(&plan.goalie)?.side.index()),
                    ),
                    Containment::Rebound => {
                        self.emit(i, "goalie_rebound", Some(plan.goalie.clone()), None, None)
                    }
                }
            }
        }
        Ok(())
    }
}
