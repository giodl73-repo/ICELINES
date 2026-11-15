//! Prospective team permission, not observed rotation or assignment grading.
//! Cover forecasts can admit/abort a named chase; actual access/control still
//! use the spatial task engine. Outlet occupancy is not a derived pass lane.
use super::*;
use space::{Cell, Intent};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Duty {
    Pressure,
    Cover,
    Outlet,
    Support,
    Partner,
    Goalie,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub label: String,
    pub player: String,
    pub duty: Duty,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Trigger {
    Active { basis: String },
    Inactive { basis: String },
    Unknown { reason: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cover {
    Projected {
        player: String,
        slot_offset: Cell,
        motion: Intent,
        deadline_seconds: u16,
        reach_cells: u16,
        basis: String,
    },
    Unknown {
        reason: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outlet {
    pub player: String,
    pub station_offset: Cell,
    pub reach_cells: u16,
    pub pressure_reach_cells: u16,
    pub basis: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub system_id: String,
    pub basis: String,
    pub side: Side,
    pub legend: Vec<Slot>,
    pub trigger: Trigger,
    pub pincher: String,
    pub chase: Intent,
    pub cover: Cover,
    pub outlets: Vec<Outlet>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutletAudit {
    pub player: String,
    pub station: Cell,
    pub distance: u16,
    pub at_station: bool,
    pub pressure: Vec<String>,
    pub status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Audit {
    pub system_id: String,
    pub basis: String,
    pub side: Side,
    pub snapshot_seconds_remaining: u16,
    pub legend: Vec<Slot>,
    pub pincher: String,
    pub decision: String,
    pub cover_player: Option<String>,
    pub cover_slot: Option<Cell>,
    pub cover_arrival_seconds: Option<u16>,
    pub cover_deadline_seconds: Option<u16>,
    pub cover_status: String,
    pub executed_cover_confirmed: bool,
    pub outlets: Vec<OutletAudit>,
}
fn bounded(s: &str) -> bool {
    !s.trim().is_empty() && s.chars().count() <= 240
}
fn distance(a: Cell, b: Cell) -> u16 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
fn absolute(origin: Cell, offset: Cell) -> Result<Cell, String> {
    if !(-20..=20).contains(&offset.x) || !(-20..=20).contains(&offset.y) {
        return Err("team offset out of bounds".into());
    }
    let c = Cell {
        x: origin.x + offset.x,
        y: origin.y + offset.y,
    };
    if !(-20..=20).contains(&c.x) || !(-20..=20).contains(&c.y) {
        return Err("team station outside local grid".into());
    }
    Ok(c)
}
fn forecast(
    grid: &space::Plan,
    player: &str,
    slot: Cell,
    motion: &Intent,
    reach: u16,
) -> Result<space::Arrival, String> {
    let actor = grid
        .actors
        .iter()
        .find(|a| a.player == player)
        .ok_or("team actor absent from actual snapshot")?;
    let mut projection = grid.clone();
    projection.puck = slot;
    projection.reach_cells = reach;
    projection.actors = vec![space::Located {
        intent: motion.clone(),
        ..actor.clone()
    }];
    Ok(space::arrivals(&projection).remove(0))
}
pub(super) fn validate(
    q: &Request,
    p: &collection::Profiles,
    grid: &space::Plan,
    t: &Plan,
) -> Result<(), String> {
    if q.schema != "rinkcraft.event-episode.0.11"
        || t.schema != "rinkcraft.team-intent.0.1"
        || !bounded(&t.system_id)
        || !bounded(&t.basis)
        || t.legend.len() != 6
        || t.outlets.len() > 4
    {
        return Err(
            "team intent requires explicit 0.11, local system/basis and complete six-player legend"
                .into(),
        );
    }
    let mut ids = BTreeSet::new();
    let mut labels = BTreeSet::new();
    for slot in &t.legend {
        let actor = q
            .players
            .iter()
            .find(|a| a.id == slot.player && a.side == t.side)
            .ok_or("legend actor has wrong side")?;
        if !bounded(&slot.label)
            || !labels.insert(&slot.label)
            || !ids.insert(&slot.player)
            || actor.goalie != matches!(slot.duty, Duty::Goalie)
        {
            return Err("invalid local legend or goalie duty domain".into());
        }
    }
    let assigned = |id: &str, duty: fn(&Duty) -> bool| -> Result<(), String> {
        if !t.legend.iter().any(|s| s.player == id && duty(&s.duty))
            || collection::profile(p, id)?.branch
                != crate::view_model::rinkcraft_game::Branch::Skater
        {
            return Err("named team duty lacks compatible skater binding".into());
        }
        Ok(())
    };
    assigned(&t.pincher, |d| matches!(d, Duty::Pressure))?;
    if !matches!(t.chase, Intent::Pursue { .. }) {
        return Err("pinch chase requires declared pursuit".into());
    }
    let a = grid
        .actors
        .iter()
        .find(|a| a.player == t.pincher)
        .ok_or("pincher missing from projected grid")?;
    if !matches!(a.intent, Intent::Hold) {
        return Err(
            "conditional pincher must initially hold; team permission selects chase".into(),
        );
    }
    let mut chase_grid = grid.clone();
    chase_grid
        .actors
        .iter_mut()
        .find(|a| a.player == t.pincher)
        .unwrap()
        .intent = t.chase.clone();
    space::validate_plan(q, p, &chase_grid, 1200)?;
    let reason = match &t.trigger {
        Trigger::Active { basis } | Trigger::Inactive { basis } => basis,
        Trigger::Unknown { reason } => reason,
    };
    if !bounded(reason) {
        return Err("trigger needs bounded basis/reason".into());
    }
    match &t.cover {
        Cover::Unknown { reason } => {
            if !bounded(reason) {
                return Err("cover unknown needs reason".into());
            }
        }
        Cover::Projected {
            player,
            slot_offset,
            motion,
            deadline_seconds,
            reach_cells,
            basis,
        } => {
            assigned(player, |d| matches!(d, Duty::Cover))?;
            if player == &t.pincher
                || !(1..=60).contains(deadline_seconds)
                || *reach_cells > 2
                || !bounded(basis)
            {
                return Err("invalid prospective cover constraint".into());
            }
            let at = absolute(grid.puck, *slot_offset)?;
            let mut check = grid.clone();
            check.puck = at;
            check.reach_cells = *reach_cells;
            check.actors = vec![space::Located {
                intent: motion.clone(),
                ..grid
                    .actors
                    .iter()
                    .find(|a| a.player == *player)
                    .ok_or("coverer absent")?
                    .clone()
            }];
            space::validate_plan(q, p, &check, 1200)?;
            // First slice forecasts permission only: it must not imply executed motion.
            if !matches!(
                grid.actors
                    .iter()
                    .find(|a| a.player == *player)
                    .unwrap()
                    .intent,
                Intent::Hold
            ) {
                return Err("cover forecast needs explicit holding snapshot; executed cover motion not implemented".into());
            }
        }
    }
    let mut outlet_ids = BTreeSet::new();
    for outlet in &t.outlets {
        assigned(&outlet.player, |d| matches!(d, Duty::Outlet))?;
        if !outlet_ids.insert(&outlet.player)
            || outlet.reach_cells > 2
            || outlet.pressure_reach_cells > 3
            || !bounded(&outlet.basis)
        {
            return Err("invalid outlet specification".into());
        }
        absolute(grid.puck, outlet.station_offset)?;
    }
    Ok(())
}
/// No dice and no clock cost: inspect declared intent before an actual chase.
pub(super) fn evaluate(q: &Request, grid: &space::Plan, t: &Plan) -> Result<Audit, String> {
    let active = |id: &str| grid.actors.iter().any(|a| a.player == id);
    let mut audit = Audit {
        system_id: t.system_id.clone(),
        basis: t.basis.clone(),
        side: t.side,
        snapshot_seconds_remaining: grid.snapshot_seconds_remaining,
        legend: t.legend.clone(),
        pincher: t.pincher.clone(),
        decision: "unresolved".into(),
        cover_player: None,
        cover_slot: None,
        cover_arrival_seconds: None,
        cover_deadline_seconds: None,
        cover_status: "unassessed".into(),
        executed_cover_confirmed: false,
        outlets: vec![],
    };
    if t.legend.iter().any(|slot| !active(&slot.player)) {
        audit.cover_status = "named_actor_unavailable".into();
        return Ok(audit);
    }
    for outlet in &t.outlets {
        let Some(actor) = grid.actors.iter().find(|a| a.player == outlet.player) else {
            return Ok(audit);
        };
        let station = absolute(grid.puck, outlet.station_offset)?;
        let d = distance(actor.cell, station);
        let mut pressure: Vec<_> = grid
            .actors
            .iter()
            .filter(|a| {
                q.players
                    .iter()
                    .any(|p| p.id == a.player && p.side != t.side)
                    && distance(a.cell, actor.cell) <= outlet.pressure_reach_cells
            })
            .map(|a| a.player.clone())
            .collect();
        pressure.sort();
        let at_station = d <= outlet.reach_cells;
        let goalie_pressure = pressure
            .iter()
            .any(|id| q.players.iter().any(|p| p.id == *id && p.goalie));
        let status = if !at_station {
            "not_at_station"
        } else if goalie_pressure {
            "goalie_pressure_unmapped"
        } else if pressure.is_empty() {
            "at_station_unpressured"
        } else {
            "at_station_pressured"
        };
        audit.outlets.push(OutletAudit {
            player: outlet.player.clone(),
            station,
            distance: d,
            at_station,
            pressure,
            status: status.into(),
        });
    }
    if !active(&t.pincher) {
        audit.cover_status = "named_actor_unavailable".into();
        return Ok(audit);
    }
    if matches!(t.trigger, Trigger::Unknown { .. }) {
        audit.cover_status = "trigger_unknown".into();
        return Ok(audit);
    }
    if matches!(t.trigger, Trigger::Inactive { .. }) {
        audit.decision = "not_triggered".into();
        audit.cover_status = "not_required".into();
        return Ok(audit);
    }
    match &t.cover {
        Cover::Unknown { .. } => audit.cover_status = "cover_unknown".into(),
        Cover::Projected {
            player,
            slot_offset,
            motion,
            deadline_seconds,
            reach_cells,
            ..
        } => {
            audit.cover_player = Some(player.clone());
            audit.cover_deadline_seconds = Some(*deadline_seconds);
            let at = absolute(grid.puck, *slot_offset)?;
            audit.cover_slot = Some(at);
            if !active(player) {
                audit.cover_status = "named_actor_unavailable".into();
                return Ok(audit);
            }
            let arrival = forecast(grid, player, at, motion, *reach_cells)?.seconds;
            audit.cover_arrival_seconds = arrival;
            let timely = arrival.is_some_and(|time| time <= *deadline_seconds);
            audit.cover_status = if timely {
                "projected_timely"
            } else {
                "projected_late_or_unreachable"
            }
            .into();
            audit.decision = if timely {
                "pinch_permitted"
            } else {
                "pinch_aborted"
            }
            .into();
        }
    }
    Ok(audit)
}
impl Engine<'_> {
    pub(super) fn apply_team_intent(
        &mut self,
        i: usize,
        grid: &mut space::Plan,
        t: &Plan,
    ) -> Result<bool, String> {
        let audit = evaluate(self.q, grid, t)?;
        let decision = audit.decision.clone();
        self.emit(i, "team_intent", Some(t.pincher.clone()), None, None);
        self.events.last_mut().unwrap().team_intent = Some(audit);
        if decision == "unresolved" {
            self.unresolved(
                i,
                "team permission unresolved; no chase or executed cover assumed".into(),
            );
            return Ok(false);
        }
        if decision == "pinch_permitted" {
            grid.actors
                .iter_mut()
                .find(|a| a.player == t.pincher)
                .unwrap()
                .intent = t.chase.clone();
        }
        Ok(true)
    }
}
