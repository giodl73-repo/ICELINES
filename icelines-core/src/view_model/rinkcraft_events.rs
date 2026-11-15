//! Bounded event episodes, not a continuous game or calibrated puck physics.
//! All probabilities and reachable contacts are declared game inputs. Scores are
//! reduced from goal events; no team score is sampled in advance.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub mod collection;
pub mod pass_context;
pub mod passing;
pub mod rebound;
pub mod shot;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    A,
    B,
}
impl Side {
    fn index(self) -> usize {
        if self == Self::A {
            0
        } else {
            1
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Player {
    pub id: String,
    pub side: Side,
    pub goalie: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub seed: u64,
    pub players: Vec<Player>,
    pub controller: String,
    pub seconds_remaining: u16,
    pub steps: Vec<Step>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profiles: Option<collection::Profiles>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub seconds: u16,
    pub action: Action,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Release {
        actor: String,
        shot: bool,
    },
    Flight {
        choices: Vec<Choice<Contact>>,
    },
    Collect {
        choices: Vec<Choice<Collection>>,
    },
    CollectUnderPressure {
        collector: String,
        pressure: String,
    },
    PassUnderPressure {
        passer: String,
        receiver: String,
        lane_defender: String,
        pressure: String,
        receive_seconds: u16,
    },
    PassWithContext {
        plan: pass_context::Plan,
    },
    ResolveShot {
        plan: shot::Plan,
    },
    Challenge {
        actor: String,
        victim: String,
        choices: Vec<Choice<Challenge>>,
    },
    Restart {
        winner: String,
    },
    Wait,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice<T> {
    pub weight: u16,
    pub outcome: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Contact {
    Receive { player: String },
    Deflect { player: String },
    Post,
    Boards,
    Loose,
    Goalie { player: String, hold: bool },
    Goal,
    Out { deflected: bool, ruling: OutRuling },
    IcingCandidate,
}

/// The first slice accepts ordinary out-of-play only with an explicit ruling;
/// it does not infer Rule 63.2 exceptions or a faceoff location from a flight.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutRuling {
    Ordinary { restart: String },
    Unresolved { reason: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Collection {
    Control { player: String },
    RemainsLoose,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Challenge {
    pub dislodge: bool,
    pub ruling: ChallengeRuling,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChallengeRuling {
    Legal,
    /// Declared ordinary skater minor; penalty-shot/major/etc. facts must instead
    /// use Unresolved. No foul probabilities are inferred from player attributes.
    CalledMinor {
        infraction: String,
    },
    Unresolved {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Puck {
    Controlled { player: String },
    Flight,
    Loose,
    Dead,
    Scored,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Penalty {
    pub offender: String,
    pub victim: String,
    pub infraction: String,
    pub remaining: u16,
    pub delayed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct State {
    pub seconds_remaining: u16,
    pub score: [u16; 2],
    pub active: Vec<String>,
    pub puck: Puck,
    pub last_touch: Option<String>,
    pub last_deliberate: Option<String>,
    pub attempt: Option<u32>,
    pub penalty: Option<Penalty>,
    pub restart: Option<String>,
    pub unresolved: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: u32,
    pub cause: Option<u32>,
    pub step: usize,
    pub kind: String,
    pub actor: Option<String>,
    pub attempt: Option<u32>,
    pub parent_attempt: Option<u32>,
    pub draw: Option<u16>,
    pub state: State,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<collection::Audit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_task: Option<passing::Audit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_task: Option<pass_context::Audit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_task: Option<shot::Audit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rebound_task: Option<rebound::Audit>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Goal {
    pub event: u32,
    pub attempt: u32,
    pub side: Side,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultSet {
    pub schema: String,
    pub request: Request,
    pub events: Vec<Event>,
    pub goals: Vec<Goal>,
    pub final_state: State,
    pub limitations: Vec<String>,
}

struct Engine<'a> {
    q: &'a Request,
    state: State,
    events: Vec<Event>,
    goals: Vec<Goal>,
    rng: u64,
    attempts: u32,
    shooter_side: Option<Side>,
    used_minor: bool,
}

impl Engine<'_> {
    fn player(&self, id: &str) -> Result<&Player, String> {
        self.q
            .players
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| format!("unknown player {id}"))
    }
    fn active(&self, id: &str) -> Result<&Player, String> {
        if !self.state.active.iter().any(|p| p == id) {
            return Err(format!("inactive player {id}"));
        }
        self.player(id)
    }
    fn emit(
        &mut self,
        step: usize,
        kind: &str,
        actor: Option<String>,
        draw: Option<u16>,
        parent: Option<u32>,
    ) {
        let id = self.events.len() as u32 + 1;
        self.events.push(Event {
            id,
            cause: if id > 1 { Some(id - 1) } else { None },
            step,
            kind: kind.into(),
            actor,
            attempt: self.state.attempt,
            parent_attempt: parent,
            draw,
            state: self.state.clone(),
            task: None,
            pass_task: None,
            context_task: None,
            shot_task: None,
            rebound_task: None,
        });
    }
    fn choose<T: Serialize>(&mut self, choices: &[Choice<T>]) -> Result<(usize, u16), String> {
        if choices.is_empty() || choices.len() > 32 {
            return Err("need 1..32 competing outcomes".into());
        }
        let mut seen = BTreeSet::new();
        let mut total = 0u32;
        for c in choices {
            if c.weight == 0
                || !seen.insert(serde_json::to_string(&c.outcome).map_err(|e| e.to_string())?)
            {
                return Err("zero weight or duplicate outcome".into());
            }
            total += c.weight as u32;
        }
        if total != 10_000 {
            return Err("choice weights must sum to 10000".into());
        }
        let draw = self.below(10_000);
        let mut end = 0u32;
        for (i, c) in choices.iter().enumerate() {
            end += c.weight as u32;
            if (draw as u32) < end {
                return Ok((i, draw));
            }
        }
        unreachable!()
    }
    // SplitMix64 plus rejection sampling. Keeping the 10000-bin path unchanged
    // preserves all saved 0.1 RNG draws. Task contests use 400 equiprobable pairs.
    fn below(&mut self, upper: u16) -> u16 {
        let modulus = u64::from(upper);
        loop {
            self.rng = self.rng.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.rng;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^= z >> 31;
            if z >= modulus.wrapping_neg() % modulus {
                return (z % modulus) as u16;
            }
        }
    }
    fn unresolved(&mut self, step: usize, reason: String) {
        self.state.unresolved = Some(reason);
        self.emit(step, "unresolved", None, None, None);
    }
    fn release_penalty(&mut self, step: usize, why: &str) {
        if let Some(p) = self.state.penalty.take() {
            if !p.delayed {
                self.state.active.push(p.offender.clone());
                self.state.active.sort();
            }
            self.emit(step, why, Some(p.offender), None, None);
        }
    }
    fn whistle(&mut self, step: usize, restart: String) {
        self.state.puck = Puck::Dead;
        self.state.restart = Some(restart);
        self.emit(step, "whistle", None, None, None);
        let offender = self.state.penalty.as_mut().filter(|p| p.delayed).map(|p| {
            p.delayed = false;
            p.offender.clone()
        });
        if let Some(id) = offender {
            self.state.active.retain(|p| p != &id);
            self.state.restart = Some(format!(
                "defensive_zone:{}",
                self.player(&id).unwrap().side.index()
            ));
            self.emit(step, "minor_service_started", Some(id), None, None);
        }
    }
    fn control(&mut self, step: usize, id: String, draw: Option<u16>) -> Result<(), String> {
        let side = self.active(&id)?.side;
        self.state.puck = Puck::Controlled { player: id.clone() };
        self.state.last_touch = Some(id.clone());
        self.emit(step, "control", Some(id), draw, None);
        if let Some(p) = &self.state.penalty {
            if p.delayed && self.player(&p.offender)?.side == side {
                self.whistle(step, "pending_minor".into());
            }
        }
        Ok(())
    }
    fn advance(&mut self, step: usize, seconds: u16) -> Result<bool, String> {
        if seconds == 0 {
            return Err("live actions require positive seconds".into());
        }
        if seconds >= self.state.seconds_remaining {
            self.unresolved(step, "period boundary requires a period transition".into());
            return Ok(false);
        }
        let expiry = self
            .state
            .penalty
            .as_ref()
            .filter(|p| !p.delayed && p.remaining <= seconds)
            .map(|p| p.remaining);
        if let Some(until_expiry) = expiry {
            self.state.seconds_remaining -= until_expiry;
            self.state.penalty.as_mut().unwrap().remaining = 0;
            self.release_penalty(step, "minor_expired");
            self.state.seconds_remaining -= seconds - until_expiry;
        } else {
            self.state.seconds_remaining -= seconds;
            if let Some(p) = self.state.penalty.as_mut().filter(|p| !p.delayed) {
                p.remaining -= seconds;
            }
        }
        self.emit(step, "clock_advanced", None, None, None);
        Ok(true)
    }
    fn contact_valid(&self, c: &Contact) -> Result<(), String> {
        match c {
            Contact::Receive { player } | Contact::Deflect { player } => {
                self.active(player)?;
            }
            Contact::Goalie { player, .. } => {
                let p = self.active(player)?;
                if !p.goalie || self.shooter_side == Some(p.side) {
                    return Err("goalie contact requires opposing active goalie".into());
                }
            }
            Contact::Goal | Contact::Post => {
                if self.state.attempt.is_none() {
                    return Err("goal/post needs a live shot chain".into());
                }
            }
            Contact::Out {
                ruling: OutRuling::Ordinary { restart },
                ..
            } if restart.trim().is_empty() => return Err("out requires declared restart".into()),
            _ => {}
        }
        Ok(())
    }
    fn record_goal(&mut self, i: usize, draw: Option<u16>) -> Result<(), String> {
        let side = self.shooter_side.ok_or("goal requires shot side")?;
        let attempt = self.state.attempt.ok_or("goal requires attempt")?;
        if let Some(p) = &self.state.penalty {
            if p.delayed && self.player(&p.offender)?.side == side {
                self.unresolved(
                    i,
                    "offending-team goal during delayed penalty requires adjudication".into(),
                );
                return Ok(());
            }
        }
        if self.goals.iter().any(|g| g.attempt == attempt) {
            return Err("duplicate goal in attempt".into());
        }
        self.state.score[side.index()] += 1;
        self.state.puck = Puck::Scored;
        self.state.restart = Some("center".into());
        self.emit(i, "goal", None, draw, None);
        self.goals.push(Goal {
            event: self.events.len() as u32,
            attempt,
            side,
        });
        if let Some(p) = &self.state.penalty {
            let penalized_side = self.player(&p.offender)?.side;
            if side != penalized_side {
                self.release_penalty(
                    i,
                    if p.delayed {
                        "pending_minor_washed"
                    } else {
                        "minor_ended_by_goal"
                    },
                );
            }
        }

        Ok(())
    }
    fn apply(&mut self, i: usize, step: &Step) -> Result<(), String> {
        if let Action::Restart { winner } = &step.action {
            if step.seconds != 0
                || self.state.restart.is_none()
                || !matches!(self.state.puck, Puck::Dead | Puck::Scored)
            {
                return Err("restart needs stoppage and zero live seconds".into());
            }
            self.active(winner)?;
            self.state.restart = None;
            self.state.attempt = None;
            self.shooter_side = None;
            self.emit(i, "faceoff_adjudicated", Some(winner.clone()), None, None);
            return self.control(i, winner.clone(), None);
        }
        if matches!(self.state.puck, Puck::Dead | Puck::Scored) {
            return Err("live action after whistle; explicit restart required".into());
        }
        if !self.advance(i, step.seconds)? {
            return Ok(());
        }
        match &step.action {
            Action::Release { actor, shot } => {
                self.active(actor)?;
                if self.state.puck
                    != (Puck::Controlled {
                        player: actor.clone(),
                    })
                {
                    return Err("release requires puck controller".into());
                }
                let parent = if self.shooter_side == Some(self.player(actor)?.side) {
                    self.state.attempt
                } else {
                    None
                };
                self.state.puck = Puck::Flight;
                self.state.last_touch = Some(actor.clone());
                self.state.last_deliberate = Some(actor.clone());
                if *shot {
                    self.attempts += 1;
                    self.state.attempt = Some(self.attempts);
                    self.shooter_side = Some(self.player(actor)?.side);
                } else {
                    self.state.attempt = None;
                    self.shooter_side = None;
                }
                self.emit(
                    i,
                    if *shot {
                        "shot_released"
                    } else {
                        "puck_released"
                    },
                    Some(actor.clone()),
                    None,
                    if *shot { parent } else { None },
                );
            }
            Action::Flight { choices } => {
                if self.state.puck != Puck::Flight {
                    return Err("flight contacts require in-flight puck".into());
                }
                for c in choices {
                    self.contact_valid(&c.outcome)?;
                }
                let (n, draw) = self.choose(choices)?;
                match &choices[n].outcome {
                    Contact::Receive { player } => self.control(i, player.clone(), Some(draw))?,
                    Contact::Deflect { player } => {
                        self.state.last_touch = Some(player.clone());
                        self.emit(
                            i,
                            "player_deflection",
                            Some(player.clone()),
                            Some(draw),
                            None,
                        );
                    }
                    Contact::Post | Contact::Boards => self.emit(
                        i,
                        if matches!(choices[n].outcome, Contact::Post) {
                            "post_contact"
                        } else {
                            "boards_contact"
                        },
                        None,
                        Some(draw),
                        None,
                    ),
                    Contact::Loose => {
                        self.state.puck = Puck::Loose;
                        self.emit(i, "puck_loose", None, Some(draw), None);
                    }
                    Contact::Goalie { player, hold } => {
                        self.state.last_touch = Some(player.clone());
                        self.state.puck = Puck::Loose;
                        self.emit(i, "goalie_contact", Some(player.clone()), Some(draw), None);
                        if *hold {
                            self.whistle(
                                i,
                                format!("goalie_hold:{}", self.player(player)?.side.index()),
                            );
                        }
                    }
                    Contact::Goal => self.record_goal(i, Some(draw))?,
                    Contact::Out { deflected, ruling } => {
                        self.emit(
                            i,
                            if *deflected {
                                "out_deflected"
                            } else {
                                "out_direct"
                            },
                            None,
                            Some(draw),
                            None,
                        );
                        match ruling {
                            OutRuling::Ordinary { restart } => self.whistle(i, restart.clone()),
                            OutRuling::Unresolved { reason } => self.unresolved(i, reason.clone()),
                        }
                    }
                    Contact::IcingCandidate => {
                        self.emit(i, "icing_candidate", None, Some(draw), None);
                        self.unresolved(i,"hybrid icing requires origin, strength, race and official adjudication".into());
                    }
                }
            }
            Action::Collect { choices } => {
                if self.state.puck != Puck::Loose {
                    return Err("collection requires loose puck".into());
                }
                for c in choices {
                    if let Collection::Control { player } = &c.outcome {
                        self.active(player)?;
                    }
                }
                let (n, draw) = self.choose(choices)?;
                match &choices[n].outcome {
                    Collection::Control { player } => {
                        self.control(i, player.clone(), Some(draw))?
                    }
                    Collection::RemainsLoose => {
                        self.emit(i, "collection_failed", None, Some(draw), None)
                    }
                }
            }
            Action::CollectUnderPressure {
                collector,
                pressure,
            } => {
                if self.state.puck != Puck::Loose {
                    return Err("pressure collection requires loose puck".into());
                }
                let a = self.active(collector)?;
                let b = self.active(pressure)?;
                if a.side == b.side || a.goalie || b.goalie {
                    return Err("pressure collection requires opposing active skaters".into());
                }
                let pair_index = self.below(400);
                let profiles = self
                    .q
                    .profiles
                    .as_ref()
                    .ok_or("task requires 0.2 profiles")?;
                let audit = collection::resolve(profiles, collector, pressure, pair_index)?;
                let won = audit.outcome == super::rinkcraft_contest::Outcome::ActorWins;
                self.emit(i, "collection_contest", Some(collector.clone()), None, None);
                self.events.last_mut().unwrap().task = Some(audit);
                if won {
                    self.control(i, collector.clone(), None)?;
                } else {
                    self.emit(i, "collection_denied", Some(pressure.clone()), None, None);
                }
            }
            Action::PassUnderPressure {
                passer,
                receiver,
                lane_defender,
                pressure,
                receive_seconds,
            } => {
                let side = self.active(passer)?.side;
                if self.state.puck
                    != (Puck::Controlled {
                        player: passer.clone(),
                    })
                {
                    return Err("pass requires actual puck controller".into());
                }
                for id in [passer, receiver, lane_defender, pressure] {
                    self.active(id)?;
                }
                if self.player(receiver)?.side != side
                    || self.player(lane_defender)?.side == side
                    || self.player(pressure)?.side == side
                {
                    return Err("pass requires a teammate receiver and opposing defenders".into());
                }
                self.state.puck = Puck::Flight;
                self.state.attempt = None;
                self.shooter_side = None;
                self.state.last_touch = Some(passer.clone());
                self.state.last_deliberate = Some(passer.clone());
                self.emit(i, "pass_released", Some(passer.clone()), None, None);
                let pair = self.below(400);
                let audit = passing::resolve(
                    self.q.profiles.as_ref().unwrap(),
                    passer,
                    lane_defender,
                    passing::Task::Delivery,
                    pair,
                )?;
                let delivered = audit.outcome == super::rinkcraft_contest::Outcome::ActorWins;
                self.emit(i, "pass_delivery_contest", Some(passer.clone()), None, None);
                self.events.last_mut().unwrap().pass_task = Some(audit);
                if !delivered {
                    self.state.puck = Puck::Loose;
                    self.emit(
                        i,
                        "pass_delivery_denied",
                        Some(lane_defender.clone()),
                        None,
                        None,
                    );
                    return Ok(());
                }
                self.emit(i, "pass_arriving", Some(receiver.clone()), None, None);
                if !self.advance(i, *receive_seconds)? {
                    return Ok(());
                }
                self.active(receiver)?;
                self.active(pressure)?;
                self.state.puck = Puck::Loose;
                self.state.last_touch = Some(receiver.clone());
                self.emit(i, "receiver_contact", Some(receiver.clone()), None, None);
                let pair = self.below(400);
                let audit = passing::resolve(
                    self.q.profiles.as_ref().unwrap(),
                    receiver,
                    pressure,
                    passing::Task::Receipt,
                    pair,
                )?;
                let controlled = audit.outcome == super::rinkcraft_contest::Outcome::ActorWins;
                self.emit(
                    i,
                    "pass_receipt_contest",
                    Some(receiver.clone()),
                    None,
                    None,
                );
                self.events.last_mut().unwrap().pass_task = Some(audit);
                if controlled {
                    self.control(i, receiver.clone(), None)?;
                } else {
                    self.emit(i, "pass_receipt_denied", Some(pressure.clone()), None, None);
                }
            }
            Action::PassWithContext { plan } => self.context_pass(i, plan)?,
            Action::ResolveShot { plan } => self.resolve_shot(i, plan)?,
            Action::Challenge {
                actor,
                victim,
                choices,
            } => {
                let a = self.active(actor)?;
                let v = self.active(victim)?;
                if a.side == v.side
                    || a.goalie
                    || v.goalie
                    || self.state.puck
                        != (Puck::Controlled {
                            player: victim.clone(),
                        })
                {
                    return Err("challenge requires opposing skater puck carrier".into());
                }
                if choices
                    .iter()
                    .any(|c| matches!(c.outcome.ruling, ChallengeRuling::CalledMinor { .. }))
                    && self.used_minor
                {
                    return Err("only one minor lifecycle supported per episode".into());
                }
                let (n, draw) = self.choose(choices)?;
                let c = &choices[n].outcome;
                if c.dislodge {
                    self.state.puck = Puck::Loose;
                }
                self.emit(
                    i,
                    if c.dislodge {
                        "challenge_dislodged"
                    } else {
                        "challenge_no_separation"
                    },
                    Some(actor.clone()),
                    Some(draw),
                    None,
                );
                match &c.ruling {
                    ChallengeRuling::Legal => {
                        self.emit(i, "contact_legal", Some(actor.clone()), None, None)
                    }
                    ChallengeRuling::CalledMinor { infraction } => {
                        if infraction.trim().is_empty() {
                            return Err("minor requires named adjudicated infraction".into());
                        }
                        self.used_minor = true;
                        self.state.penalty = Some(Penalty {
                            offender: actor.clone(),
                            victim: victim.clone(),
                            infraction: infraction.clone(),
                            remaining: 120,
                            delayed: true,
                        });
                        self.emit(i, "minor_signaled", Some(actor.clone()), None, None);
                    }
                    ChallengeRuling::Unresolved { reason } => self.unresolved(i, reason.clone()),
                }
            }
            Action::Wait => self.emit(i, "wait", None, None, None),
            Action::Restart { .. } => unreachable!(),
        }
        Ok(())
    }
}

pub fn run(q: &Request) -> Result<ResultSet, String> {
    collection::validate(q)?;
    if q.steps.is_empty()
        || q.steps.len() > 256
        || q.seconds_remaining == 0
        || q.seconds_remaining > 1200
    {
        return Err("unsupported schema, episode bounds or period clock".into());
    }
    let ids: BTreeSet<_> = q.players.iter().map(|p| p.id.as_str()).collect();
    if ids.len() != q.players.len() || ids.iter().any(|id| id.trim().is_empty()) {
        return Err("unique nonempty player IDs required".into());
    }
    for side in [Side::A, Side::B] {
        if q.players.iter().filter(|p| p.side == side).count() != 6
            || q.players
                .iter()
                .filter(|p| p.side == side && p.goalie)
                .count()
                != 1
        {
            return Err("initial lineup must have five skaters and one goalie per side".into());
        }
    }
    let mut e = Engine {
        q,
        state: State {
            seconds_remaining: q.seconds_remaining,
            score: [0, 0],
            active: ids.into_iter().map(str::to_owned).collect(),
            puck: Puck::Controlled {
                player: q.controller.clone(),
            },
            last_touch: Some(q.controller.clone()),
            last_deliberate: None,
            attempt: None,
            penalty: None,
            restart: None,
            unresolved: None,
        },
        events: vec![],
        goals: vec![],
        rng: q.seed,
        attempts: 0,
        shooter_side: None,
        used_minor: false,
    };
    e.active(&q.controller)?;
    for (i, s) in q.steps.iter().enumerate() {
        e.apply(i, s).map_err(|why| format!("step {i}: {why}"))?;
        if e.state.unresolved.is_some() {
            break;
        }
    }
    Ok(ResultSet { schema:match q.schema.as_str() { "rinkcraft.event-episode.0.6" => "rinkcraft.event-result.0.6", "rinkcraft.event-episode.0.5" => "rinkcraft.event-result.0.5", "rinkcraft.event-episode.0.4" => "rinkcraft.event-result.0.4", "rinkcraft.event-episode.0.3" => "rinkcraft.event-result.0.3", "rinkcraft.event-episode.0.2"=>"rinkcraft.event-result.0.2", _=>"rinkcraft.event-result.0.1" }.into(),request:q.clone(),events:e.events,goals:e.goals,final_state:e.state,
        limitations:vec![if q.schema == "rinkcraft.event-episode.0.6" { "Rebounds affects containment only after a hold-eligible goalie contact; eligibility/target, finish weights, geometry and other tasks remain authored, not NHL-calibrated." } else if q.schema == "rinkcraft.event-episode.0.5" { "Shooting versus goalie Tracking changes a declared shot contest; finish/contact branches, context, other goalie capabilities and rates remain authored, not NHL-calibrated." } else if q.schema == "rinkcraft.event-episode.0.4" { "Pass lane/receipt pressure and open-task targets are declared game context, not inferred geometry or NHL-calibrated difficulty; other contacts, shots and calls remain authored." } else if q.schema == "rinkcraft.event-episode.0.3" { "Skater passing/receiving and pressure collection use verified fictional capabilities; access, timing, other contacts, shots and calls remain authored, not NHL-calibrated." } else if q.profiles.is_some() { "Only skater pressure collection maps verified fictional Handling versus Defending through the declared d20 rule; other contacts, reachability and timing remain authored, not NHL-calibrated." } else { "Authored contact distributions and declared reachability; no calibrated physics or capability-to-probability mapping." }.into(),
            "Bounded action script, fixed initial 5v5, one ordinary skater minor, no coaching substitutions or complete NHL adjudication.".into(),
            "Raw contacts and goals only; no official SOG, save, scorer or assist attribution.".into()] })
}

#[cfg(test)]
mod tests;
