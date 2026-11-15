//! Bounded, conditional attacking follow-up after an actual loose goalie rebound.
//! Declared access is not an arrival model. Recovery changes only the next shot.
use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub collector: String,
    pub pressure: String,
    pub access_basis: String,
    pub recovery_basis: String,
    pub late_tracking_adjustment: i8,
    pub release_seconds: u16,
    pub shot_seconds: u16,
    pub shot: shot::Plan,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub goalie: String,
    pub threat: String,
    pub capability_ids: [String; 2],
    pub scores: [u8; 2],
    pub score_rule: contest::ScoreRule,
    pub modifiers: [i16; 2],
    pub exact_counts: [u16; 3],
    pub pair_index: u16,
    pub dice: [u8; 2],
    pub totals: [i16; 2],
    pub outcome: contest::Outcome,
    pub readiness: String,
    pub next_goalie_adjustment: Option<i8>,
}
pub(super) fn validate(q: &Request, p: &collection::Profiles) -> Result<(), String> {
    for (i, step) in q.steps.iter().enumerate() {
        if let Action::ReboundFollowup { plan } = &step.action {
            if !matches!(
                q.schema.as_str(),
                "rinkcraft.event-episode.0.7" | "rinkcraft.event-episode.0.8"
            ) {
                return Err("rebound followup requires a supported explicit version".into());
            }
            let source = match i
                .checked_sub(1)
                .and_then(|i| q.steps.get(i))
                .map(|s| &s.action)
            {
                Some(Action::ResolveShot { plan }) => plan,
                _ => {
                    return Err(
                        "followup must immediately follow an explicit shot resolution".into(),
                    )
                }
            };
            if source.goalie != plan.shot.goalie || plan.collector != plan.shot.shooter {
                return Err("followup goalie and actual collector/shooter must match".into());
            }
            let side = |id: &str| {
                q.players
                    .iter()
                    .find(|player| player.id == id)
                    .map(|player| player.side)
                    .ok_or("unknown followup player")
            };
            if side(&plan.collector)? != side(&source.shooter)?
                || side(&plan.pressure)? != side(&source.goalie)?
                || side(&plan.collector)? == side(&source.goalie)?
            {
                return Err(
                    "followup plan needs attacking collector and opposing pressure/goalie".into(),
                );
            }
            for basis in [&plan.access_basis, &plan.recovery_basis] {
                if basis.trim().is_empty() || basis.chars().count() > 240 {
                    return Err("followup needs bounded access and recovery context".into());
                }
            }
            if !(-20..=-1).contains(&plan.late_tracking_adjustment)
                || [step.seconds, plan.release_seconds, plan.shot_seconds]
                    .iter()
                    .any(|s| !(1..=1200).contains(s))
            {
                return Err("followup requires bounded positive costs and declared negative late adjustment".into());
            }
            for id in [&plan.collector, &plan.pressure] {
                if collection::profile(p, id)?.branch != game::Branch::Skater {
                    return Err("followup collection requires skater profiles".into());
                }
            }
            shot::validate_plan(&q.schema, p, &plan.shot)?;
        }
    }
    Ok(())
}
pub(super) fn resolve(
    p: &collection::Profiles,
    goalie: &str,
    threat: &str,
    pair_index: u16,
    late: i8,
) -> Result<Audit, String> {
    if pair_index >= 400 || !(-20..=-1).contains(&late) {
        return Err("recovery pair/late adjustment out of bounds".into());
    }
    let g = collection::profile(p, goalie)?;
    let t = collection::profile(p, threat)?;
    if g.branch != game::Branch::Goalie || t.branch != game::Branch::Skater {
        return Err("recovery needs goalie versus skater domains".into());
    }
    let a = &g.capabilities[4];
    let b = &t.capabilities[0];
    let c = contest::Contest {
        actor_score: a.score,
        opposition_score: b.score,
        score_rule: p.score_rule,
        actor_adjustment: 0,
        opposition_adjustment: 0,
    };
    let counts = c.exact_counts()?;
    let dice = [(pair_index / 20 + 1) as u8, (pair_index % 20 + 1) as u8];
    let r = c.resolve(dice[0], dice[1])?;
    let (readiness, adjustment) = match r.outcome {
        contest::Outcome::ActorWins => ("reset", Some(0)),
        contest::Outcome::OppositionWins => ("late", Some(late)),
        contest::Outcome::Stalemate => ("unresolved", None),
    };
    Ok(Audit {
        goalie: goalie.into(),
        threat: threat.into(),
        capability_ids: [a.id.clone(), b.id.clone()],
        scores: [a.score, b.score],
        score_rule: p.score_rule,
        modifiers: c.terms()?,
        exact_counts: [counts.actor_wins, counts.stalemates, counts.opposition_wins],
        pair_index,
        dice,
        totals: [r.actor_total, r.opposition_total],
        outcome: r.outcome,
        readiness: readiness.into(),
        next_goalie_adjustment: adjustment,
    })
}
impl Engine<'_> {
    pub(super) fn follow_rebound(
        &mut self,
        i: usize,
        collection_seconds: u16,
        plan: &Plan,
    ) -> Result<(), String> {
        let source = self.events.last().ok_or("missing source shot events")?;
        if source.step + 1 != i {
            return Err("followup source must be the immediately preceding step".into());
        }
        if source.kind != "goalie_rebound" {
            self.emit(i, "followup_skipped", None, None, None);
            return Ok(());
        }
        if self.state.puck != Puck::Loose
            || source.actor.as_deref() != Some(plan.shot.goalie.as_str())
        {
            return Err("followup requires that goalie's actual loose rebound".into());
        }
        let side = self.active(&plan.collector)?.side;
        if self.active(&plan.pressure)?.side == side
            || self.active(&plan.shot.goalie)?.side == side
            || self.shooter_side != Some(side)
        {
            return Err(
                "followup needs an attacking collector and opposing pressure/goalie".into(),
            );
        }
        if !self.advance(i, collection_seconds)? {
            return Ok(());
        }
        self.active(&plan.collector)?;
        self.active(&plan.pressure)?;
        let pair = self.below(400);
        let audit = collection::resolve(
            self.q.profiles.as_ref().unwrap(),
            &plan.collector,
            &plan.pressure,
            pair,
        )?;
        let won = audit.outcome == contest::Outcome::ActorWins;
        self.emit(
            i,
            "followup_collection",
            Some(plan.collector.clone()),
            None,
            None,
        );
        self.events.last_mut().unwrap().task = Some(audit);
        if !won {
            self.emit(
                i,
                "followup_collection_denied",
                Some(plan.pressure.clone()),
                None,
                None,
            );
            return Ok(());
        }
        self.control(i, plan.collector.clone(), None)?;
        if matches!(self.state.puck, Puck::Dead | Puck::Scored) {
            return Ok(());
        }
        let pair = self.below(400);
        let audit = resolve(
            self.q.profiles.as_ref().unwrap(),
            &plan.shot.goalie,
            &plan.collector,
            pair,
            plan.late_tracking_adjustment,
        )?;
        let adjustment = audit.next_goalie_adjustment;
        self.emit(
            i,
            "goalie_recovery_contest",
            Some(plan.shot.goalie.clone()),
            None,
            None,
        );
        self.events.last_mut().unwrap().recovery_task = Some(audit);
        let Some(adjustment) = adjustment else {
            self.unresolved(i, "recovery race tied; readiness not adjudicated".into());
            return Ok(());
        };
        self.apply(
            i,
            &Step {
                seconds: plan.release_seconds,
                action: Action::Release {
                    actor: plan.collector.clone(),
                    shot: true,
                },
            },
        )?;
        if self.state.unresolved.is_some() {
            return Ok(());
        }
        if !self.advance(i, plan.shot_seconds)? {
            return Ok(());
        }
        self.resolve_shot_adjusted(i, &plan.shot, Some(adjustment))
    }
}
