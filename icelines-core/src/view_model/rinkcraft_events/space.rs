//! Timestamped authored local-grid access. No NHL rink dimensions, continuous
//! trajectories or ability-to-speed mapping are inferred by this first slice.
use super::*;
use crate::view_model::rinkcraft_game as game;
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cell {
    pub x: i16,
    pub y: i16,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    East,
    West,
    North,
    South,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Intent {
    Hold,
    Pursue {
        seconds_per_cell: u16,
        reaction_seconds: u16,
        turn_seconds: u16,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Located {
    pub player: String,
    pub cell: Cell,
    pub facing: Facing,
    pub intent: Intent,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub snapshot_seconds_remaining: u16,
    pub basis: String,
    pub puck: Cell,
    pub reach_cells: u16,
    pub collection_seconds: u16,
    pub open_target: u8,
    pub actors: Vec<Located>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Arrival {
    pub player: String,
    pub distance: u16,
    pub travel_cells: u16,
    pub turned: bool,
    pub seconds: Option<u16>,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Roll {
    pub player: String,
    pub capability_id: String,
    pub score: u8,
    pub modifier: i16,
    pub die: u8,
    pub total: i16,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub snapshot_seconds_remaining: u16,
    pub arrivals: Vec<Arrival>,
    pub earliest_seconds: Option<u16>,
    pub earliest: Vec<String>,
    pub access_rolls: Vec<Roll>,
    pub winner: Option<String>,
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectionAudit {
    pub collector: Roll,
    pub pressure: Vec<Roll>,
    pub target: Option<u8>,
    pub controlled: bool,
    pub exact_successes: u64,
    pub combinations: u64,
}
fn distance(a: Cell, b: Cell) -> u16 {
    (i32::from(a.x) - i32::from(b.x)).unsigned_abs() as u16
        + (i32::from(a.y) - i32::from(b.y)).unsigned_abs() as u16
}
pub(super) fn arrivals(plan: &Plan) -> Vec<Arrival> {
    let mut result: Vec<_> = plan
        .actors
        .iter()
        .map(|a| {
            let d = distance(a.cell, plan.puck);
            let travel = d.saturating_sub(plan.reach_cells);
            let aligned = match a.facing {
                Facing::East => plan.puck.x > a.cell.x,
                Facing::West => plan.puck.x < a.cell.x,
                Facing::North => plan.puck.y > a.cell.y,
                Facing::South => plan.puck.y < a.cell.y,
            };
            let (seconds, turned, reason) = match a.intent {
                Intent::Hold => (
                    if travel == 0 { Some(0) } else { None },
                    false,
                    if travel == 0 {
                        "holding_in_reach"
                    } else {
                        "holding_outside_reach"
                    },
                ),
                Intent::Pursue {
                    seconds_per_cell,
                    reaction_seconds,
                    turn_seconds,
                } => {
                    let turned = travel > 0 && !aligned;
                    (
                        Some(
                            reaction_seconds
                                + travel * seconds_per_cell
                                + if turned { turn_seconds } else { 0 },
                        ),
                        turned,
                        "declared_movement",
                    )
                }
            };
            Arrival {
                player: a.player.clone(),
                distance: d,
                travel_cells: travel,
                turned,
                seconds,
                reason: reason.into(),
            }
        })
        .collect();
    result.sort_by(|a, b| a.player.cmp(&b.player));
    result
}
pub(super) fn validate(q: &Request, p: &collection::Profiles) -> Result<(), String> {
    for step in &q.steps {
        if let Action::LoosePuckAccess { plan } = &step.action {
            if !matches!(
                q.schema.as_str(),
                "rinkcraft.event-episode.0.8" | "rinkcraft.event-episode.0.9"
            ) || plan.schema != "rinkcraft.access-grid.0.1"
            {
                return Err("spatial access requires explicit 0.8/grid0.1".into());
            }
            validate_plan(q, p, plan, step.seconds)?;
        }
    }
    Ok(())
}
pub(super) fn validate_plan(
    q: &Request,
    p: &collection::Profiles,
    plan: &Plan,
    window: u16,
) -> Result<(), String> {
    if plan.schema != "rinkcraft.access-grid.0.1" {
        return Err("unsupported access grid".into());
    }
    if !(1..=1200).contains(&window)
        || !(1..=window).contains(&plan.collection_seconds)
        || !(1..=40).contains(&plan.open_target)
        || plan.reach_cells > 2
        || plan.basis.trim().is_empty()
        || plan.basis.chars().count() > 240
    {
        return Err("invalid grid window, collection, target, reach or basis".into());
    }
    let cell = |c: Cell| (-20..=20).contains(&c.x) && (-20..=20).contains(&c.y);
    if !cell(plan.puck) || plan.actors.is_empty() || plan.actors.len() > 12 {
        return Err("invalid local grid puck/pool".into());
    }
    let mut ids = BTreeSet::new();
    for a in &plan.actors {
        if !ids.insert(&a.player) || !cell(a.cell) || !q.players.iter().any(|p| p.id == a.player) {
            return Err("invalid/duplicate grid actor".into());
        }
        if let Intent::Pursue {
            seconds_per_cell,
            reaction_seconds,
            turn_seconds,
        } = a.intent
        {
            if !(1..=10).contains(&seconds_per_cell) || reaction_seconds > 60 || turn_seconds > 60 {
                return Err("declared movement rates/delays out of bounds".into());
            }
        }
    }
    // All potentially involved skaters need profiles, not just the eventual winner.
    for a in arrivals(plan)
        .iter()
        .filter(|a| a.seconds.is_some_and(|t| t <= window))
    {
        let player = q.players.iter().find(|p| p.id == a.player).unwrap();
        if !player.goalie && collection::profile(p, &a.player)?.branch != game::Branch::Skater {
            return Err("spatial skater domain mismatch".into());
        }
    }
    Ok(())
}
pub(super) fn roll(
    p: &collection::Profiles,
    player: &str,
    index: usize,
    draw: u16,
) -> Result<Roll, String> {
    if draw >= 20 {
        return Err("spatial task die out of bounds".into());
    }
    let c = collection::profile(p, player)?;
    if c.branch != game::Branch::Skater {
        return Err("goalie access/collection requires a separate task map".into());
    }
    let a = &c.capabilities[index];
    let modifier = p.score_rule.term(a.score)?;
    let die = draw as u8 + 1;
    Ok(Roll {
        player: player.into(),
        capability_id: a.id.clone(),
        score: a.score,
        modifier,
        die,
        total: i16::from(die) + modifier,
    })
}
pub(super) fn exact_collection(actor_term: i16, opposition: &[i16], target: u8) -> (u64, u64) {
    if opposition.is_empty() {
        return (
            (1..=20)
                .filter(|d| d + actor_term >= i16::from(target))
                .count() as u64,
            20,
        );
    }
    let combinations = 20u64.pow(opposition.len() as u32 + 1);
    let successes = (1..=20)
        .map(|die| {
            opposition
                .iter()
                .map(|term| i64::from((die + actor_term - term - 1).clamp(0, 20)) as u64)
                .product::<u64>()
        })
        .sum();
    (successes, combinations)
}
impl Engine<'_> {
    pub(super) fn spatial_advance(&mut self, i: usize, seconds: u16) -> Result<bool, String> {
        if let Some(until) = self
            .state
            .penalty
            .as_ref()
            .filter(|p| !p.delayed && p.remaining <= seconds)
            .map(|p| p.remaining)
        {
            if self.advance(i, until)? {
                self.unresolved(
                    i,
                    "active pool changed during spatial window; new snapshot required".into(),
                );
            }
            return Ok(false);
        }
        if seconds == 0 {
            Ok(true)
        } else {
            self.advance(i, seconds)
        }
    }
    pub(super) fn spatial_access(
        &mut self,
        i: usize,
        window: u16,
        plan: &Plan,
    ) -> Result<(), String> {
        if self.state.puck != Puck::Loose {
            return Err("grid access requires a loose puck".into());
        }
        if plan.snapshot_seconds_remaining != self.state.seconds_remaining {
            return Err("stale spatial snapshot clock".into());
        }
        let supplied: BTreeSet<_> = plan.actors.iter().map(|a| a.player.as_str()).collect();
        let active: BTreeSet<_> = self.state.active.iter().map(String::as_str).collect();
        if supplied != active {
            return Err("spatial snapshot must cover exactly the actual active pool".into());
        }
        let arrivals = arrivals(plan);
        let earliest_seconds = arrivals
            .iter()
            .filter_map(|a| a.seconds)
            .filter(|t| *t <= window)
            .min();
        let earliest: Vec<_> = arrivals
            .iter()
            .filter(|a| a.seconds == earliest_seconds && earliest_seconds.is_some())
            .map(|a| a.player.clone())
            .collect();
        let mut audit = Audit {
            snapshot_seconds_remaining: self.state.seconds_remaining,
            arrivals,
            earliest_seconds,
            earliest,
            access_rolls: vec![],
            winner: None,
            status: "no_reachable_collection".into(),
        };
        let Some(time) = earliest_seconds else {
            self.emit(i, "spatial_access", None, None, None);
            self.events.last_mut().unwrap().access_task = Some(audit);
            if self.spatial_advance(i, window)? {
                self.emit(i, "spatial_window_ended", None, None, None);
            }
            return Ok(());
        };
        if time + plan.collection_seconds > window {
            audit.status = "arrival_without_collection_window".into();
            self.emit(i, "spatial_access", None, None, None);
            self.events.last_mut().unwrap().access_task = Some(audit);
            if self.spatial_advance(i, window)? {
                self.emit(i, "spatial_window_ended", None, None, None);
            }
            return Ok(());
        }
        if !self.spatial_advance(i, time)? {
            return Ok(());
        }
        if audit
            .earliest
            .iter()
            .any(|id| self.player(id).unwrap().goalie)
        {
            audit.status = "goalie_access_unmapped".into();
            self.emit(i, "spatial_access", None, None, None);
            self.events.last_mut().unwrap().access_task = Some(audit);
            self.unresolved(
                i,
                "goalie reaches first; skater access cannot substitute goalie capabilities".into(),
            );
            return Ok(());
        }
        let winner = if audit.earliest.len() == 1 {
            Some(audit.earliest[0].clone())
        } else {
            for id in &audit.earliest {
                let draw = self.below(20);
                audit
                    .access_rolls
                    .push(roll(self.q.profiles.as_ref().unwrap(), id, 0, draw)?);
            }
            let best = audit.access_rolls.iter().map(|r| r.total).max().unwrap();
            let top: Vec<_> = audit
                .access_rolls
                .iter()
                .filter(|r| r.total == best)
                .collect();
            if top.len() == 1 {
                Some(top[0].player.clone())
            } else {
                None
            }
        };
        audit.winner = winner.clone();
        audit.status = if winner.is_some() {
            "access_only"
        } else {
            "access_tie_unresolved"
        }
        .into();
        self.emit(i, "spatial_access", winner.clone(), None, None);
        self.events.last_mut().unwrap().access_task = Some(audit.clone());
        let Some(winner) = winner else {
            self.unresolved(
                i,
                "simultaneous access tied; no collection owner assumed".into(),
            );
            return Ok(());
        };
        let side = self.player(&winner)?.side;
        let pressure: Vec<_> = audit
            .arrivals
            .iter()
            .filter(|a| {
                a.seconds
                    .is_some_and(|t| t <= time + plan.collection_seconds)
                    && self.player(&a.player).unwrap().side != side
            })
            .map(|a| a.player.clone())
            .collect();
        if pressure.iter().any(|id| self.player(id).unwrap().goalie) {
            self.unresolved(
                i,
                "goalie can pressure collection; goalie task map required".into(),
            );
            return Ok(());
        }
        if !self.spatial_advance(i, plan.collection_seconds)? {
            return Ok(());
        }
        let draw = self.below(20);
        let collector = roll(self.q.profiles.as_ref().unwrap(), &winner, 1, draw)?;
        let mut rolls = vec![];
        for id in pressure {
            let draw = self.below(20);
            rolls.push(roll(self.q.profiles.as_ref().unwrap(), &id, 5, draw)?);
        }
        let terms: Vec<_> = rolls.iter().map(|r| r.modifier).collect();
        let (successes, combinations) =
            exact_collection(collector.modifier, &terms, plan.open_target);
        let controlled = if rolls.is_empty() {
            collector.total >= i16::from(plan.open_target)
        } else {
            rolls.iter().all(|r| collector.total > r.total)
        };
        let check = CollectionAudit {
            collector,
            pressure: rolls,
            target: if terms.is_empty() {
                Some(plan.open_target)
            } else {
                None
            },
            controlled,
            exact_successes: successes,
            combinations,
        };
        self.emit(i, "spatial_collection", Some(winner.clone()), None, None);
        self.events.last_mut().unwrap().spatial_collection = Some(check);
        if controlled {
            self.control(i, winner, None)?;
        } else {
            self.emit(i, "spatial_collection_denied", None, None, None);
        }
        Ok(())
    }
}
