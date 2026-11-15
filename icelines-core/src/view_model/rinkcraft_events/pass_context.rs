//! Declared pressure admission, not a geometry classifier or NHL calibration.
use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Context {
    Open { target: u8, basis: String },
    Defended { defender: String },
    Unknown { reason: String },
}
impl Context {
    fn defender(&self) -> Option<&str> {
        if let Self::Defended { defender } = self {
            Some(defender)
        } else {
            None
        }
    }
    fn reason(&self) -> Option<&str> {
        if let Self::Unknown { reason } = self {
            Some(reason)
        } else {
            None
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub passer: String,
    pub receiver: String,
    pub lane: Context,
    pub receipt: Context,
    pub receive_seconds: u16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Audit {
    Open {
        task: passing::Task,
        actor: String,
        capability_id: String,
        score: u8,
        score_rule: contest::ScoreRule,
        modifier: i16,
        basis: String,
        target: u8,
        die: u8,
        total: i16,
        success: bool,
        exact_successes: u16,
        combinations: u16,
    },
    Defended {
        audit: passing::Audit,
    },
}
impl Audit {
    pub fn success(&self) -> bool {
        match self {
            Self::Open { success, .. } => *success,
            Self::Defended { audit } => audit.outcome == contest::Outcome::ActorWins,
        }
    }
}
pub(super) fn validate(q: &Request, p: &collection::Profiles) -> Result<(), String> {
    for step in &q.steps {
        if let Action::PassWithContext { plan } = &step.action {
            if !matches!(
                q.schema.as_str(),
                "rinkcraft.event-episode.0.4"
                    | "rinkcraft.event-episode.0.5"
                    | "rinkcraft.event-episode.0.6"
                    | "rinkcraft.event-episode.0.7"
                    | "rinkcraft.event-episode.0.8"
            ) {
                return Err("context passes require explicit 0.4 episode".into());
            }
            if plan.passer == plan.receiver || !(1..=1200).contains(&plan.receive_seconds) {
                return Err(
                    "context pass needs distinct receiver and positive bounded receipt time".into(),
                );
            }
            let mut ids = vec![plan.passer.as_str(), plan.receiver.as_str()];
            for context in [&plan.lane, &plan.receipt] {
                match context {
                    Context::Open { target, basis }
                        if !(1..=40).contains(target)
                            || basis.trim().is_empty()
                            || basis.chars().count() > 240 =>
                    {
                        return Err("open check needs target 1..40 and declared basis".into())
                    }
                    Context::Unknown { reason }
                        if reason.trim().is_empty() || reason.chars().count() > 240 =>
                    {
                        return Err("unknown context needs bounded reason".into())
                    }
                    Context::Defended { defender } => ids.push(defender),
                    _ => {}
                }
            }
            for id in ids {
                if collection::profile(p, id)?.branch != game::Branch::Skater {
                    return Err("context pass requires skater profiles".into());
                }
            }
        }
    }
    Ok(())
}
pub(super) fn resolve(
    p: &collection::Profiles,
    actor: &str,
    context: &Context,
    task: passing::Task,
    draw: u16,
) -> Result<Audit, String> {
    match context {
        Context::Defended { defender } => Ok(Audit::Defended {
            audit: passing::resolve(p, actor, defender, task, draw)?,
        }),
        Context::Open { target, basis } => {
            if draw >= 20 || !(1..=40).contains(target) {
                return Err("open draw/target out of bounds".into());
            }
            let index = if task == passing::Task::Delivery {
                2
            } else {
                1
            };
            let capability = &collection::profile(p, actor)?.capabilities[index];
            let modifier = p.score_rule.term(capability.score)?;
            let die = draw as u8 + 1;
            let total = i16::from(die) + modifier;
            let exact_successes = (1..=20)
                .filter(|die| die + modifier >= i16::from(*target))
                .count() as u16;
            Ok(Audit::Open {
                task,
                actor: actor.into(),
                capability_id: capability.id.clone(),
                score: capability.score,
                score_rule: p.score_rule,
                modifier,
                basis: basis.clone(),
                target: *target,
                die,
                total,
                success: total >= i16::from(*target),
                exact_successes,
                combinations: 20,
            })
        }
        Context::Unknown { .. } => Err("unknown context cannot be rolled".into()),
    }
}
impl Engine<'_> {
    fn context_check(
        &mut self,
        i: usize,
        actor: &str,
        context: &Context,
        task: passing::Task,
    ) -> Result<bool, String> {
        let draw = self.below(if matches!(context, Context::Open { .. }) {
            20
        } else {
            400
        });
        let audit = resolve(
            self.q.profiles.as_ref().unwrap(),
            actor,
            context,
            task,
            draw,
        )?;
        let success = audit.success();
        self.emit(
            i,
            if task == passing::Task::Delivery {
                "context_delivery_check"
            } else {
                "context_receipt_check"
            },
            Some(actor.into()),
            None,
            None,
        );
        self.events.last_mut().unwrap().context_task = Some(audit);
        Ok(success)
    }
    pub(super) fn context_pass(&mut self, i: usize, plan: &Plan) -> Result<(), String> {
        let side = self.active(&plan.passer)?.side;
        if self.state.puck
            != (Puck::Controlled {
                player: plan.passer.clone(),
            })
        {
            return Err("context pass requires actual controller".into());
        }
        if self.active(&plan.receiver)?.side != side {
            return Err("context receiver must be teammate".into());
        }
        for context in [&plan.lane, &plan.receipt] {
            if let Some(id) = context.defender() {
                if self.active(id)?.side == side {
                    return Err("context defender must be opposing active skater".into());
                }
            }
        }
        if let Some(reason) = plan.lane.reason() {
            self.unresolved(i, format!("lane context unknown: {reason}"));
            return Ok(());
        }
        self.state.puck = Puck::Flight;
        self.state.attempt = None;
        self.shooter_side = None;
        self.state.last_touch = Some(plan.passer.clone());
        self.state.last_deliberate = Some(plan.passer.clone());
        self.emit(i, "pass_released", Some(plan.passer.clone()), None, None);
        if !self.context_check(i, &plan.passer, &plan.lane, passing::Task::Delivery)? {
            self.state.puck = Puck::Loose;
            self.emit(
                i,
                "pass_delivery_denied",
                plan.lane.defender().map(str::to_owned),
                None,
                None,
            );
            return Ok(());
        }
        self.emit(i, "pass_arriving", Some(plan.receiver.clone()), None, None);
        if let Some(reason) = plan.receipt.reason() {
            self.unresolved(i, format!("receipt context unknown: {reason}"));
            return Ok(());
        }
        if !self.advance(i, plan.receive_seconds)? {
            return Ok(());
        }
        self.active(&plan.receiver)?;
        if let Some(id) = plan.receipt.defender() {
            self.active(id)?;
        }
        self.state.puck = Puck::Loose;
        self.state.last_touch = Some(plan.receiver.clone());
        self.emit(
            i,
            "receiver_contact",
            Some(plan.receiver.clone()),
            None,
            None,
        );
        if self.context_check(i, &plan.receiver, &plan.receipt, passing::Task::Receipt)? {
            self.control(i, plan.receiver.clone(), None)?;
        } else {
            self.emit(
                i,
                "pass_receipt_denied",
                plan.receipt.defender().map(str::to_owned),
                None,
                None,
            );
        }
        Ok(())
    }
}
