//! Authored, bounded three-zone routes. Fixed physical coordinates survive
//! possession changes. Crossings, access and control are separate facts.
use super::*;
use crate::view_model::rinkcraft_game as game;
use space::{Cell, Facing, Intent};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub west_blue_line: i16,
    pub east_blue_line: i16,
    pub a_attacks_east: bool,
    pub period: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    WestEnd,
    Neutral,
    EastEnd,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Defensive,
    Neutral,
    Offensive,
}
impl Frame {
    pub fn band(&self, puck: Cell) -> Band {
        if puck.x < self.west_blue_line {
            Band::WestEnd
        } else if puck.x > self.east_blue_line {
            Band::EastEnd
        } else {
            Band::Neutral
        }
    }
    fn east(&self, side: Side) -> bool {
        self.a_attacks_east == (side == Side::A)
    }
    pub fn zone(&self, puck: Cell, side: Side) -> Zone {
        match self.band(puck) {
            Band::Neutral => Zone::Neutral,
            Band::EastEnd if self.east(side) => Zone::Offensive,
            Band::WestEnd if !self.east(side) => Zone::Offensive,
            _ => Zone::Defensive,
        }
    }
    fn target(&self, side: Side, phase: &str) -> Cell {
        let east = self.east(side);
        let own = if east {
            self.west_blue_line
        } else {
            self.east_blue_line
        };
        let attack = if east {
            self.east_blue_line
        } else {
            self.west_blue_line
        };
        let sign = if east { 1 } else { -1 };
        Cell {
            x: match phase {
                "exit" => own + sign,
                "neutral" => attack - sign,
                "regroup" => own - sign,
                "clear" => (self.west_blue_line + self.east_blue_line) / 2,
                _ => attack + 2 * sign,
            },
            y: 0,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Ruling {
    Legal { basis: String },
    Unknown { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Move {
    Carry {
        context: pass_context::Context,
        seconds: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_failure: Option<Failure>,
    },
    Pass {
        receiver: String,
        lane: pass_context::Context,
        receipt: pass_context::Context,
        seconds: u16,
        receipt_seconds: u16,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_delivery_failure: Option<Failure>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        on_receipt_failure: Option<Failure>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub scene: String,
    pub basis: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Progress {
    Advanced,
    Recovered,
    Stop,
}
impl From<bool> for Progress {
    fn from(value: bool) -> Self {
        if value {
            Self::Advanced
        } else {
            Self::Stop
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Exit {
    Controlled {
        movement: Move,
        ruling: Ruling,
    },
    Clear {
        scene: String,
        seconds: u16,
        ruling: Ruling,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Entry {
    Controlled {
        movement: Move,
        ruling: Ruling,
    },
    Dump {
        landing: Cell,
        scene: String,
        seconds: u16,
        ruling: Ruling,
        basis: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Regroup {
    pub movement: Move,
    pub ruling: Ruling,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidePlan {
    pub side: Side,
    pub exit: Exit,
    pub neutral: Move,
    pub regroup: Option<Regroup>,
    pub entry: Entry,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub player: String,
    pub offset: Cell,
    pub facing: Facing,
    pub intent: Intent,
}
/// A declared projection around a landing point, not an observed stale snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: String,
    pub basis: String,
    pub window_seconds: u16,
    pub collection_seconds: u16,
    pub open_target: u8,
    pub reach_cells: u16,
    pub actors: Vec<Placement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_intent: Option<team_intent::Plan>,
}
impl Scene {
    fn project(&self, puck: Cell, clock: u16, active: Option<&[String]>) -> space::Plan {
        space::Plan {
            schema: "rinkcraft.access-grid.0.1".into(),
            snapshot_seconds_remaining: clock,
            basis: self.basis.clone(),
            puck,
            reach_cells: self.reach_cells,
            collection_seconds: self.collection_seconds,
            open_target: self.open_target,
            actors: self
                .actors
                .iter()
                .filter(|a| active.is_none_or(|ids| ids.contains(&a.player)))
                .map(|a| space::Located {
                    player: a.player.clone(),
                    cell: Cell {
                        x: puck.x + a.offset.x,
                        y: puck.y + a.offset.y,
                    },
                    facing: a.facing,
                    intent: a.intent.clone(),
                })
                .collect(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub frame: Frame,
    pub initiating_side: Side,
    pub start_puck: Cell,
    pub initial_scene: String,
    pub scenes: Vec<Scene>,
    pub sides: Vec<SidePlan>,
    pub max_transitions: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Crossing {
    pub from: Cell,
    pub to: Cell,
    pub side: Side,
    pub purpose: String,
    pub method: String,
    pub control_at_crossing: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub phase: String,
    pub outcome: String,
    pub frame: Frame,
    pub puck: Cell,
    pub band: Band,
    pub zone_for_a: Zone,
    pub zone_for_b: Zone,
    pub controller: Option<String>,
    pub controlling_side: Option<Side>,
    pub initiating_side: Side,
    pub crossing: Option<Crossing>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Check {
    pub task: String,
    pub context: pass_context::Context,
    pub actor: space::Roll,
    pub opposition: Option<space::Roll>,
    pub score_rule: crate::view_model::rinkcraft_contest::ScoreRule,
    pub exact_successes: u64,
    pub combinations: u64,
    pub success: bool,
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.chars().count() <= 240
}
fn cell(c: Cell) -> bool {
    (-20..=20).contains(&c.x) && (-20..=20).contains(&c.y)
}
fn duration(t: u16) -> bool {
    (1..=30).contains(&t)
}
fn ruling(r: &Ruling) -> Result<(), String> {
    let s = match r {
        Ruling::Legal { basis } => basis,
        Ruling::Unknown { reason } => reason,
    };
    if text(s) {
        Ok(())
    } else {
        Err("crossing needs bounded legal basis or unknown reason".into())
    }
}
fn skater(q: &Request, p: &collection::Profiles, id: &str, side: Side) -> Result<(), String> {
    if !q
        .players
        .iter()
        .any(|a| a.id == id && a.side == side && !a.goalie)
        || collection::profile(p, id)?.branch != game::Branch::Skater
    {
        return Err("transport actor has incompatible side or skater profile".into());
    }
    Ok(())
}
fn context(
    q: &Request,
    p: &collection::Profiles,
    c: &pass_context::Context,
    side: Side,
) -> Result<(), String> {
    match c {
        pass_context::Context::Open { target, basis }
            if !(1..=40).contains(target) || !text(basis) =>
        {
            Err("invalid transport open context".into())
        }
        pass_context::Context::Unknown { reason } if !text(reason) => {
            Err("invalid transport unknown context".into())
        }
        pass_context::Context::Defended { defender } => skater(
            q,
            p,
            defender,
            if side == Side::A { Side::B } else { Side::A },
        ),
        _ => Ok(()),
    }
}
fn movement(q: &Request, p: &collection::Profiles, m: &Move, side: Side) -> Result<(), String> {
    match m {
        Move::Carry {
            context: c,
            seconds,
            ..
        } => {
            if !duration(*seconds) {
                return Err("invalid movement time".into());
            }
            context(q, p, c, side)
        }
        Move::Pass {
            receiver,
            lane,
            receipt,
            seconds,
            receipt_seconds,
            ..
        } => {
            if !duration(*seconds) || !duration(*receipt_seconds) {
                return Err("invalid pass time".into());
            }
            skater(q, p, receiver, side)?;
            context(q, p, lane, side)?;
            context(q, p, receipt, side)
        }
    }
}
pub(super) fn validate(q: &Request, p: &collection::Profiles) -> Result<(), String> {
    for step in &q.steps {
        let Action::ThreeZoneSequence { plan: z } = &step.action else {
            continue;
        };
        let f = z.frame;
        if !matches!(
            q.schema.as_str(),
            "rinkcraft.event-episode.0.9"
                | "rinkcraft.event-episode.0.10"
                | "rinkcraft.event-episode.0.11"
        ) || !(1..=1200).contains(&step.seconds)
            || !(-18..=18).contains(&f.west_blue_line)
            || !(-18..=18).contains(&f.east_blue_line)
            || f.east_blue_line - f.west_blue_line < 4
            || !(1..=10).contains(&f.period)
            || !cell(z.start_puck)
            || f.zone(z.start_puck, z.initiating_side) != Zone::Defensive
            || !(1..=24).contains(&z.max_transitions)
            || z.sides.len() != 2
            || z.scenes.is_empty()
            || z.scenes.len()
                > if matches!(
                    q.schema.as_str(),
                    "rinkcraft.event-episode.0.10" | "rinkcraft.event-episode.0.11"
                ) {
                    33
                } else {
                    9
                }
        {
            return Err("invalid explicit 0.9 three-zone frame, window or route bounds".into());
        }
        // Every possible actual holder can be called upon by a side's carry policy.
        for a in q.players.iter().filter(|a| !a.goalie) {
            skater(q, p, &a.id, a.side)?;
        }
        let mut ids = BTreeSet::new();
        for scene in &z.scenes {
            if !text(&scene.id)
                || !ids.insert(&scene.id)
                || !(1..=1200).contains(&scene.window_seconds)
                || scene.actors.len() != q.players.len()
                || scene.collection_seconds > scene.window_seconds
                || scene.actors.iter().any(|a| !cell(a.offset))
            {
                return Err("invalid projection scene/pool".into());
            }
        }
        let mut refs = BTreeSet::new();
        let mut check_scene = |id: &String, at: Cell| -> Result<(), String> {
            refs.insert(id.clone());
            let scene = z
                .scenes
                .iter()
                .find(|s| s.id == *id)
                .ok_or("unknown scene reference")?;
            let projected = scene.project(at, q.seconds_remaining, None);
            space::validate_plan(q, p, &projected, scene.window_seconds)?;
            if let Some(intent) = &scene.team_intent {
                team_intent::validate(q, p, &projected, intent)?;
            }
            Ok(())
        };
        check_scene(&z.initial_scene, z.start_puck)?;
        for side in [Side::A, Side::B] {
            if z.sides.iter().filter(|s| s.side == side).count() != 1 {
                return Err("need unique A/B route plans".into());
            }
            let s = z.sides.iter().find(|s| s.side == side).unwrap();
            match &s.exit {
                Exit::Controlled {
                    movement: m,
                    ruling: r,
                } => {
                    movement(q, p, m, side)?;
                    ruling(r)?;
                }
                Exit::Clear {
                    scene,
                    seconds,
                    ruling: r,
                } => {
                    if !duration(*seconds) {
                        return Err("invalid clear time".into());
                    }
                    ruling(r)?;
                    check_scene(scene, f.target(side, "clear"))?;
                }
            }
            movement(q, p, &s.neutral, side)?;
            if let Some(r) = &s.regroup {
                movement(q, p, &r.movement, side)?;
                ruling(&r.ruling)?;
            }
            match &s.entry {
                Entry::Controlled {
                    movement: m,
                    ruling: r,
                } => {
                    movement(q, p, m, side)?;
                    ruling(r)?;
                }
                Entry::Dump {
                    landing,
                    scene,
                    seconds,
                    ruling: r,
                    basis,
                } => {
                    if !duration(*seconds)
                        || !cell(*landing)
                        || f.zone(*landing, side) != Zone::Offensive
                        || !text(basis)
                    {
                        return Err("invalid dump landing/time/basis".into());
                    }
                    ruling(r)?;
                    check_scene(scene, *landing)?;
                }
            }
        }
        let mut locations = vec![z.start_puck];
        for side in [Side::A, Side::B] {
            for phase in ["exit", "neutral", "regroup", "entry", "clear"] {
                locations.push(f.target(side, phase));
            }
        }
        for policy in &z.sides {
            if let Entry::Dump { landing, .. } = policy.entry {
                locations.push(landing);
            }
        }
        for policy in &z.sides {
            let mut moves = vec![("neutral", &policy.neutral)];
            if let Exit::Controlled { movement, .. } = &policy.exit {
                moves.push(("exit", movement));
            }
            if let Entry::Controlled { movement, .. } = &policy.entry {
                moves.push(("entry", movement));
            }
            if let Some(regroup) = &policy.regroup {
                moves.push(("regroup", &regroup.movement));
            }
            for (phase, m) in moves {
                let (departure, receipt) = match m {
                    Move::Carry { on_failure, .. } => (on_failure.as_ref(), None),
                    Move::Pass {
                        on_delivery_failure,
                        on_receipt_failure,
                        ..
                    } => (on_delivery_failure.as_ref(), on_receipt_failure.as_ref()),
                };
                for failure in departure.into_iter().chain(receipt) {
                    if !matches!(
                        q.schema.as_str(),
                        "rinkcraft.event-episode.0.10" | "rinkcraft.event-episode.0.11"
                    ) || !text(&failure.basis)
                    {
                        return Err(
                            "failure retrieval needs explicit 0.10 and bounded basis".into()
                        );
                    }
                }
                if let Some(failure) = departure {
                    let origin = if phase == "exit" {
                        Zone::Defensive
                    } else {
                        Zone::Neutral
                    };
                    for at in locations
                        .iter()
                        .filter(|c| f.zone(**c, policy.side) == origin)
                    {
                        check_scene(&failure.scene, *at)?;
                    }
                }
                if let Some(failure) = receipt {
                    check_scene(&failure.scene, f.target(policy.side, phase))?;
                }
            }
        }
        if refs.len() != z.scenes.len() {
            return Err("unused projection scene".into());
        }
    }
    Ok(())
}
impl Engine<'_> {
    fn territory(
        &mut self,
        i: usize,
        z: &Plan,
        puck: Cell,
        phase: &str,
        outcome: &str,
        crossing: Option<Crossing>,
    ) {
        let controller = match &self.state.puck {
            Puck::Controlled { player } => Some(player.clone()),
            _ => None,
        };
        let controlling_side = controller
            .as_ref()
            .and_then(|id| self.player(id).ok())
            .map(|p| p.side);
        let audit = Audit {
            phase: phase.into(),
            outcome: outcome.into(),
            frame: z.frame,
            puck,
            band: z.frame.band(puck),
            zone_for_a: z.frame.zone(puck, Side::A),
            zone_for_b: z.frame.zone(puck, Side::B),
            controller: controller.clone(),
            controlling_side,
            initiating_side: z.initiating_side,
            crossing,
        };
        self.emit(i, "territory", controller, None, None);
        self.events.last_mut().unwrap().territory = Some(audit);
    }
    fn zone_time(&mut self, i: usize, seconds: u16, budget: &mut u16) -> Result<bool, String> {
        if seconds > *budget {
            self.emit(i, "transport_window_ended", None, None, None);
            return Ok(false);
        }
        *budget -= seconds;
        self.spatial_advance(i, seconds)
    }
    fn lawful(&mut self, i: usize, r: &Ruling) -> bool {
        if let Ruling::Unknown { reason } = r {
            self.unresolved(i, format!("transport crossing ruling unknown: {reason}"));
            false
        } else {
            true
        }
    }
    fn transport_check(
        &mut self,
        i: usize,
        actor: &str,
        c: &pass_context::Context,
        index: usize,
        op_index: usize,
        task: &str,
    ) -> Result<bool, String> {
        if let pass_context::Context::Unknown { reason } = c {
            self.unresolved(i, format!("{task}: {reason}"));
            return Ok(false);
        }
        self.active(actor)?;
        if let pass_context::Context::Defended { defender } = c {
            self.active(defender)?;
        }
        let draw = self.below(20);
        let a = space::roll(self.q.profiles.as_ref().unwrap(), actor, index, draw)?;
        let opposition = if let pass_context::Context::Defended { defender } = c {
            let d = self.below(20);
            Some(space::roll(
                self.q.profiles.as_ref().unwrap(),
                defender,
                op_index,
                d,
            )?)
        } else {
            None
        };
        let target = if let pass_context::Context::Open { target, .. } = c {
            *target
        } else {
            1
        };
        let terms: Vec<_> = opposition.iter().map(|r| r.modifier).collect();
        let (exact_successes, combinations) = space::exact_collection(a.modifier, &terms, target);
        let success = opposition
            .as_ref()
            .map_or(a.total >= i16::from(target), |b| a.total > b.total);
        let audit = Check {
            task: task.into(),
            context: c.clone(),
            actor: a,
            opposition,
            score_rule: self.q.profiles.as_ref().unwrap().score_rule,
            exact_successes,
            combinations,
            success,
        };
        self.emit(i, "transport_task", Some(actor.into()), None, None);
        self.events.last_mut().unwrap().transport_task = Some(audit);
        Ok(success)
    }
    fn transport_failure(
        &mut self,
        i: usize,
        z: &Plan,
        puck: Cell,
        phase: &str,
        failure: Option<&Failure>,
        budget: &mut u16,
    ) -> Result<Progress, String> {
        if self.state.unresolved.is_some() {
            return Ok(Progress::Stop);
        }
        self.state.puck = Puck::Loose;
        let Some(failure) = failure else {
            return Ok(Progress::Stop);
        };
        self.territory(i, z, puck, phase, "failed_task_loose", None);
        if self.retrieve_scene(i, z, &failure.scene, puck, budget)? {
            self.territory(i, z, puck, phase, "failed_task_recovered", None);
            Ok(Progress::Recovered)
        } else {
            Ok(Progress::Stop)
        }
    }
    fn lawful_move(
        &mut self,
        i: usize,
        z: &Plan,
        puck: &mut Cell,
        m: &Move,
        phase: &str,
        rule: Option<&Ruling>,
        budget: &mut u16,
    ) -> Result<Progress, String> {
        if rule.is_some_and(|r| !self.lawful(i, r)) {
            return Ok(Progress::Stop);
        }
        let holder = match &self.state.puck {
            Puck::Controlled { player } => player,
            _ => return Err("move needs actual holder".into()),
        };
        let side = self.player(holder)?.side;
        self.transport_move(i, z, puck, z.frame.target(side, phase), m, phase, budget)
    }
    fn transport_move(
        &mut self,
        i: usize,
        z: &Plan,
        puck: &mut Cell,
        to: Cell,
        m: &Move,
        phase: &str,
        budget: &mut u16,
    ) -> Result<Progress, String> {
        let Puck::Controlled { player: actor } = &self.state.puck else {
            return Err("transport requires actual control".into());
        };
        let actor = actor.clone();
        let side = self.player(&actor)?.side;
        let from = *puck;
        let (method, success) = match m {
            Move::Carry {
                context,
                seconds,
                on_failure,
            } => {
                if !self.zone_time(i, *seconds, budget)? {
                    return Ok(Progress::Stop);
                }
                let index = if phase == "entry" { 1 } else { 0 };
                let ok =
                    self.transport_check(i, &actor, context, index, 5, &format!("{phase}_carry"))?;
                if !ok {
                    return self.transport_failure(i, z, *puck, phase, on_failure.as_ref(), budget);
                }
                self.state.last_deliberate = Some(actor.clone());
                ("carry", true)
            }
            Move::Pass {
                receiver,
                lane,
                receipt,
                seconds,
                receipt_seconds,
                on_delivery_failure,
                on_receipt_failure,
            } => {
                if receiver == &actor {
                    return Err("route pass cannot target actual holder".into());
                }
                self.active(receiver)?;
                if !self.zone_time(i, *seconds, budget)? {
                    return Ok(Progress::Stop);
                }
                // Unknown lane does not silently release a puck.
                if !self.transport_check(i, &actor, lane, 2, 4, &format!("{phase}_delivery"))? {
                    return self.transport_failure(
                        i,
                        z,
                        *puck,
                        phase,
                        on_delivery_failure.as_ref(),
                        budget,
                    );
                }
                self.state.puck = Puck::Flight;
                self.state.last_deliberate = Some(actor.clone());
                self.state.attempt = None;
                self.shooter_side = None;
                *puck = to;
                self.territory(
                    i,
                    z,
                    *puck,
                    phase,
                    "pass_crossing_in_flight",
                    Some(Crossing {
                        from,
                        to,
                        side,
                        purpose: phase.into(),
                        method: "pass".into(),
                        control_at_crossing: false,
                    }),
                );
                if !self.zone_time(i, *receipt_seconds, budget)? {
                    return Ok(Progress::Stop);
                }
                if !self.transport_check(i, receiver, receipt, 1, 5, &format!("{phase}_receipt"))? {
                    return self.transport_failure(
                        i,
                        z,
                        *puck,
                        phase,
                        on_receipt_failure.as_ref(),
                        budget,
                    );
                }
                self.control(i, receiver.clone(), None)?;
                ("pass", matches!(self.state.puck, Puck::Controlled { .. }))
            }
        };
        if !success {
            return Ok(Progress::Stop);
        }
        *puck = to;
        self.territory(
            i,
            z,
            *puck,
            phase,
            "control_retained",
            if method == "carry" {
                Some(Crossing {
                    from,
                    to,
                    side,
                    purpose: phase.into(),
                    method: method.into(),
                    control_at_crossing: true,
                })
            } else {
                None
            },
        );
        Ok(Progress::Advanced)
    }
    fn retrieve_scene(
        &mut self,
        i: usize,
        z: &Plan,
        id: &str,
        puck: Cell,
        budget: &mut u16,
    ) -> Result<bool, String> {
        let scene = z.scenes.iter().find(|s| s.id == id).unwrap();
        if *budget == 0 {
            self.emit(i, "transport_window_ended", None, None, None);
            return Ok(false);
        }
        let window = scene.window_seconds.min(*budget);
        let mut plan = scene.project(puck, self.state.seconds_remaining, Some(&self.state.active));
        if let Some(intent) = &scene.team_intent {
            if !self.apply_team_intent(i, &mut plan, intent)? {
                return Ok(false);
            }
        }
        let before = self.state.seconds_remaining;
        self.spatial_access(i, window, &plan)?;
        *budget = budget.saturating_sub(before - self.state.seconds_remaining);
        self.territory(i, z, puck, "retrieval", "projection_resolved", None);
        Ok(self.state.unresolved.is_none() && matches!(self.state.puck, Puck::Controlled { .. }))
    }
    fn transport_loose(
        &mut self,
        i: usize,
        z: &Plan,
        puck: &mut Cell,
        to: Cell,
        phase: &str,
        seconds: u16,
        scene: &str,
        budget: &mut u16,
    ) -> Result<bool, String> {
        let Puck::Controlled { player } = &self.state.puck else {
            return Err("clear/dump requires control".into());
        };
        let player = player.clone();
        let side = self.player(&player)?.side;
        if !self.zone_time(i, seconds, budget)? {
            return Ok(false);
        }
        let from = *puck;
        *puck = to;
        self.state.puck = Puck::Flight;
        self.state.last_deliberate = Some(player);
        self.state.attempt = None;
        self.shooter_side = None;
        self.territory(
            i,
            z,
            *puck,
            phase,
            "uncontrolled_crossing",
            Some(Crossing {
                from,
                to,
                side,
                purpose: phase.into(),
                method: phase.into(),
                control_at_crossing: false,
            }),
        );
        self.state.puck = Puck::Loose;
        self.territory(i, z, *puck, phase, "loose_landing", None);
        self.retrieve_scene(i, z, scene, *puck, budget)
    }
    pub(super) fn three_zones(&mut self, i: usize, window: u16, z: &Plan) -> Result<(), String> {
        if self.state.puck != Puck::Loose {
            return Err("three-zone sequence starts with defensive loose-puck retrieval".into());
        }
        let mut budget = window;
        let mut puck = z.start_puck;
        self.territory(i, z, puck, "start", "loose_defensive_puck", None);
        if !self.retrieve_scene(i, z, &z.initial_scene, puck, &mut budget)? {
            return Ok(());
        }
        let mut progressed = None;
        let mut regrouped = [false; 2];
        let mut previous = None;
        for _ in 0..z.max_transitions {
            let Puck::Controlled { player } = &self.state.puck else {
                break;
            };
            let side = self.player(player)?.side;
            if previous.is_some_and(|s| s != side) {
                progressed = None;
                self.territory(i, z, puck, "turnover", "actual_control_reversal", None);
            }
            previous = Some(side);
            let policy = z.sides.iter().find(|s| s.side == side).unwrap();
            let progress = match z.frame.zone(puck, side) {
                Zone::Offensive => {
                    self.territory(i, z, puck, "complete", "attack_established", None);
                    return Ok(());
                }
                Zone::Defensive => {
                    progressed = None;
                    match &policy.exit {
                        Exit::Controlled { movement, ruling } => self.lawful_move(
                            i,
                            z,
                            &mut puck,
                            movement,
                            "exit",
                            Some(ruling),
                            &mut budget,
                        )?,
                        Exit::Clear {
                            scene,
                            seconds,
                            ruling,
                        } => {
                            if self.lawful(i, ruling) {
                                self.transport_loose(
                                    i,
                                    z,
                                    &mut puck,
                                    z.frame.target(side, "clear"),
                                    "clear",
                                    *seconds,
                                    scene,
                                    &mut budget,
                                )?
                                .into()
                            } else {
                                Progress::Stop
                            }
                        }
                    }
                }
                Zone::Neutral => {
                    if let Some(r) = policy.regroup.as_ref().filter(|_| !regrouped[side.index()]) {
                        regrouped[side.index()] = true;
                        progressed = None;
                        self.lawful_move(
                            i,
                            z,
                            &mut puck,
                            &r.movement,
                            "regroup",
                            Some(&r.ruling),
                            &mut budget,
                        )?
                    } else if progressed != Some(side) {
                        progressed = Some(side);
                        self.lawful_move(
                            i,
                            z,
                            &mut puck,
                            &policy.neutral,
                            "neutral",
                            None,
                            &mut budget,
                        )?
                    } else {
                        match &policy.entry {
                            Entry::Controlled { movement, ruling } => self.lawful_move(
                                i,
                                z,
                                &mut puck,
                                movement,
                                "entry",
                                Some(ruling),
                                &mut budget,
                            )?,
                            Entry::Dump {
                                landing,
                                scene,
                                seconds,
                                ruling,
                                ..
                            } => {
                                if self.lawful(i, ruling) {
                                    self.transport_loose(
                                        i,
                                        z,
                                        &mut puck,
                                        *landing,
                                        "dump",
                                        *seconds,
                                        scene,
                                        &mut budget,
                                    )?
                                    .into()
                                } else {
                                    Progress::Stop
                                }
                            }
                        }
                    }
                }
            };
            match progress {
                Progress::Recovered => {
                    progressed = None;
                }
                Progress::Stop => {
                    self.territory(i, z, puck, "stop", "route_not_completed", None);
                    return Ok(());
                }
                Progress::Advanced => {}
            }
        }

        // A final successful entry can consume the last allowed transition.
        if let Puck::Controlled { player } = &self.state.puck {
            if z.frame.zone(puck, self.player(player)?.side) == Zone::Offensive {
                self.territory(i, z, puck, "complete", "attack_established", None);
                return Ok(());
            }
        }
        self.territory(i, z, puck, "stop", "transition_limit", None);
        Ok(())
    }
}
