//! Verified same-team participation. No inferred line numbering or special-teams stations.
use super::rinkcraft_pressure_exposure::GameExposure;
use icelines_core::view_model::rinkcraft_unit_links::{Game, Member, Shared, Unit};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn from_analyzed(
    boxscore: &[u8],
    game_id: u64,
    admitted: &GameExposure,
) -> Result<BTreeMap<u32, Game>, String> {
    let b: Value = serde_json::from_slice(boxscore).map_err(|e| e.to_string())?;
    if b["id"].as_u64() != Some(game_id) || admitted.game_id != game_id {
        return Err("Unit source game differs from admitted shifts".into());
    }
    let date = b["gameDate"].as_str().ok_or("Missing unit game date")?;
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    for side in ["awayTeam", "homeTeam"] {
        let team_id = b[side]["id"].as_u64().ok_or("Missing unit team")?;
        let team = b[side]["abbrev"]
            .as_str()
            .ok_or("Missing unit team abbreviation")?;
        let mut roster = BTreeMap::<u32, (Member, String)>::new();
        for (group, family) in [
            ("forwards", "Forwards"),
            ("defense", "Defense"),
            ("goalies", "Goalies"),
        ] {
            for p in b["playerByGameStats"][side][group]
                .as_array()
                .ok_or("Missing unit roster")?
            {
                let id = u32::try_from(p["playerId"].as_u64().ok_or("Missing unit player")?)
                    .map_err(|e| e.to_string())?;
                let position = if family == "Goalies" {
                    "G"
                } else {
                    p["position"].as_str().ok_or("Missing unit position")?
                };
                let member = Member {
                    player_id: id,
                    name: p["name"]["default"]
                        .as_str()
                        .ok_or("Missing unit name")?
                        .into(),
                    position: position.into(),
                };
                roster.insert(id, (member, family.into()));
                if admitted.verified_players.contains(&id) && admitted.regulation_seconds[&id] > 0 {
                    out.insert(
                        id,
                        Game {
                            game_id,
                            date: date.into(),
                            group: family.into(),
                            states: BTreeMap::new(),
                            unresolved_seconds: 0,
                            units: BTreeMap::new(),
                            partners: Vec::new(),
                        },
                    );
                }
            }
        }
        for seg in admitted
            .segments
            .get(&team_id)
            .ok_or("Missing unit participation segments")?
        {
            let state = seg["state"].as_str().ok_or("Missing unit manpower state")?;
            let own = seg["own_participants"]
                .as_array()
                .ok_or("Missing own unit participants")?;
            let opp = seg["opponent_participants"]
                .as_array()
                .ok_or("Missing opposing unit participants")?;
            let ids = own
                .iter()
                .map(|v| {
                    v["id"]
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or("Invalid unit participant".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            if state == "invalid_occupancy"
                || own.iter().chain(opp).any(|v| {
                    v["id"]
                        .as_u64()
                        .and_then(|n| u32::try_from(n).ok())
                        .is_none_or(|id| !admitted.verified_players.contains(&id))
                })
            {
                continue;
            }
            let start = seg["start_seconds"].as_u64().ok_or("Missing unit start")?;
            let end = seg["end_seconds"].as_u64().ok_or("Missing unit end")?;
            let seconds = u32::try_from(end.checked_sub(start).ok_or("Reversed unit segment")?)
                .map_err(|e| e.to_string())?;
            let skaters = ids
                .iter()
                .filter(|id| roster[id].0.position != "G")
                .copied()
                .collect::<Vec<_>>();
            let defense = skaters
                .iter()
                .filter(|id| roster[id].0.position == "D")
                .copied()
                .collect::<Vec<_>>();
            let forwards = skaters
                .iter()
                .filter(|id| roster[id].0.position != "D")
                .copied()
                .collect::<Vec<_>>();
            for &id in &ids {
                let Some(g) = out.get_mut(&id) else { continue };
                *g.states.entry(state.into()).or_default() += seconds;
                let members = match state {
                    "5v5" if defense.len() == 2 && forwards.len() == 3 => {
                        if g.group == "Forwards" {
                            forwards.clone()
                        } else {
                            defense.clone()
                        }
                    }
                    "5v4" | "4v5" => skaters.clone(),
                    _ => Vec::new(),
                };
                if !members.is_empty() {
                    g.units.entry(state.into()).or_default().push(Unit {
                        team_id,
                        team: team.into(),
                        members: members
                            .into_iter()
                            .map(|id| roster[&id].0.clone())
                            .collect(),
                        seconds,
                    });
                }
                if state == "5v5" {
                    for &other in &skaters {
                        if other != id {
                            g.partners.push(Shared {
                                team_id,
                                team: team.into(),
                                member: roster[&other].0.clone(),
                                seconds,
                            });
                        }
                    }
                }
            }
        }
        for (&id, _) in &roster {
            if let Some(g) = out.get_mut(&id) {
                let known = g.states.values().copied().sum::<u32>();
                g.unresolved_seconds = admitted.regulation_seconds[&id]
                    .checked_sub(known)
                    .ok_or("Unit time exceeds admitted regulation TOI")?;
            }
        }
    }
    for g in out.values_mut() {
        for units in g.units.values_mut() {
            let mut merged = BTreeMap::<Vec<u32>, Unit>::new();
            for u in units.drain(..) {
                let ids = u.members.iter().map(|m| m.player_id).collect::<Vec<_>>();
                if let Some(prior) = merged.get_mut(&ids) {
                    prior.seconds += u.seconds;
                } else {
                    merged.insert(ids, u);
                }
            }
            *units = merged.into_values().collect();
        }
        let mut merged = BTreeMap::<u32, Shared>::new();
        for s in g.partners.drain(..) {
            if let Some(prior) = merged.get_mut(&s.member.player_id) {
                prior.seconds += s.seconds;
            } else {
                merged.insert(s.member.player_id, s);
            }
        }
        g.partners = merged.into_values().collect();
    }
    Ok(out)
}
