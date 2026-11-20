//! Hold-eligible contact only: Rebounds controls containment independently of
//! Tracking's opportunity to contact. Safe/dangerous redirection remains open.
use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Context {
    HoldEligible { target: u8, basis: String },
    Unknown { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub goalie: String,
    pub capability_id: String,
    pub score: u8,
    pub score_rule: contest::ScoreRule,
    pub modifier: i16,
    pub target: u8,
    pub basis: String,
    pub die: u8,
    pub total: i16,
    pub held: bool,
    pub exact_holds: u16,
    pub combinations: u16,
}
pub(super) fn validate(c: &Context) -> Result<(), String> {
    match c {
        Context::HoldEligible { target, basis }
            if !(1..=40).contains(target)
                || basis.trim().is_empty()
                || basis.chars().count() > 240 =>
        {
            Err("hold context needs target 1..40 and declared basis".into())
        }
        Context::Unknown { reason } if reason.trim().is_empty() || reason.chars().count() > 240 => {
            Err("unknown containment needs a bounded reason".into())
        }
        _ => Ok(()),
    }
}
pub(super) fn resolve(
    p: &collection::Profiles,
    goalie: &str,
    c: &Context,
    draw: u16,
) -> Result<Audit, String> {
    validate(c)?;
    let (target, basis) = if let Context::HoldEligible { target, basis } = c {
        (*target, basis)
    } else {
        return Err("unknown containment cannot be rolled".into());
    };
    if draw >= 20 {
        return Err("containment draw must be 0..19".into());
    }
    let character = collection::profile(p, goalie)?;
    if character.branch != game::Branch::Goalie {
        return Err("containment requires verified goalie capabilities".into());
    }
    let capability = &character.capabilities[3];
    let modifier = p.score_rule.term(capability.score)?;
    let die = draw as u8 + 1;
    let total = i16::from(die) + modifier;
    let exact_holds = (1..=20)
        .filter(|die| die + modifier >= i16::from(target))
        .count() as u16;
    Ok(Audit {
        goalie: goalie.into(),
        capability_id: capability.id.clone(),
        score: capability.score,
        score_rule: p.score_rule,
        modifier,
        target,
        basis: basis.clone(),
        die,
        total,
        held: total >= i16::from(target),
        exact_holds,
        combinations: 20,
    })
}
impl Engine<'_> {
    pub(super) fn contain_contact(
        &mut self,
        i: usize,
        goalie: &str,
        c: &Context,
    ) -> Result<(), String> {
        if self.state.puck != Puck::Loose
            || self.state.last_touch.as_deref() != Some(goalie)
            || self.events.last().map(|e| e.kind.as_str()) != Some("goalie_contact")
        {
            return Err("containment requires the immediate unresolved goalie contact".into());
        }
        self.active(goalie)?;
        if let Context::Unknown { reason } = c {
            self.unresolved(i, format!("containment context unknown: {reason}"));
            return Ok(());
        }
        let draw = self.below(20);
        let audit = resolve(self.q.profiles.as_ref().unwrap(), goalie, c, draw)?;
        let held = audit.held;
        self.emit(i, "goalie_rebounds_check", Some(goalie.into()), None, None);
        self.events.last_mut().unwrap().rebound_task = Some(audit);
        if held {
            self.whistle(
                i,
                format!("goalie_hold:{}", self.player(goalie)?.side.index()),
            );
        } else {
            self.emit(i, "goalie_rebound", Some(goalie.into()), None, None);
        }
        Ok(())
    }
}
