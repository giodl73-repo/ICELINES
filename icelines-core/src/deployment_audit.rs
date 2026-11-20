//! Exploratory captured-data audit. Usage tiers are not observed lines/pairs.
//! Counts remain descriptive; prior-rate comparators are conditional on realized TOI.
use serde_json::{json, Value};
use std::{collections::BTreeMap, error::Error, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub(crate) fn rows(path: &Path) -> Result<Vec<Value>> {
    let value: Value = serde_json::from_slice(&std::fs::read(path)?)?;
    Ok(value
        .get("data")
        .and_then(Value::as_array)
        .ok_or("Missing data array")?
        .clone())
}

pub(crate) fn number(row: &Value, field: &str) -> Result<f64> {
    let value = row
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("Missing numeric {field}"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!("Invalid {field}").into());
    }
    Ok(value)
}

pub(crate) fn id(row: &Value, field: &str) -> Result<u64> {
    row.get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Missing integer {field}").into())
}

pub(crate) fn index(rows: Vec<Value>, field: &str) -> Result<BTreeMap<u64, Value>> {
    let mut out = BTreeMap::new();
    for row in rows {
        let key = id(&row, field)?;
        if out.insert(key, row).is_some() {
            return Err(format!("Duplicate {field}: {key}").into());
        }
    }
    Ok(out)
}

fn ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if denominator > 0.0 {
        Some(numerator / denominator)
    } else {
        None
    }
}

fn prior_baseline(
    points: f64,
    prior_seconds: f64,
    current_seconds: f64,
    minimum: f64,
) -> Option<f64> {
    if prior_seconds >= minimum && prior_seconds > 0.0 {
        Some(points * current_seconds / prior_seconds)
    } else {
        None
    }
}

// Rank total EV exposure in this team window. Availability affects ordering.
// All lower-ranked players remain in the reserve bucket; no goals disappear.
fn tiers(players: &[Value], defense: bool) -> Result<Vec<Value>> {
    let mut selected: Vec<_> = players
        .iter()
        .filter(|p| (p["position"] == "D") == defense)
        .collect();
    selected.sort_by(|a, b| {
        b["ev_seconds"]
            .as_f64()
            .unwrap()
            .total_cmp(&a["ev_seconds"].as_f64().unwrap())
            .then_with(|| a["id"].as_u64().cmp(&b["id"].as_u64()))
    });
    let size = if defense { 2 } else { 3 };
    let groups = if defense { 3 } else { 4 };
    let mut out = Vec::new();
    for tier in 0..=groups {
        let start = tier * size;
        let members: Vec<_> = if tier == groups {
            selected.iter().skip(start).copied().collect()
        } else {
            selected.iter().skip(start).take(size).copied().collect()
        };
        let sum = |key: &str| {
            members
                .iter()
                .map(|p| p[key].as_f64().unwrap())
                .sum::<f64>()
        };
        let covered: Vec<_> = members
            .iter()
            .filter(|p| p["ev_prior_baseline"].is_number())
            .collect();
        out.push(json!({"tier": if tier == groups { "reserve".to_string() } else { (tier+1).to_string() },
            "members":members.iter().map(|p|p["name"].clone()).collect::<Vec<_>>(),
            "goals":sum("goals"),"assists":sum("assists"),"points":sum("points"),
            "ev_points":sum("ev_points"),"ev_seconds":sum("ev_seconds"),
            "baseline_covered_players":covered.len(),"players":members.len(),
            "ev_baseline_covered":covered.iter().map(|p|p["ev_prior_baseline"].as_f64().unwrap()).sum::<f64>(),
            "ev_actual_covered":covered.iter().map(|p|p["ev_points"].as_f64().unwrap()).sum::<f64>()}));
    }
    Ok(out)
}

/// Reads explicit NHL report captures; never fetches, imputes or modifies sources.
/// Prior EV minimum = 300 minutes, PP minimum = 60 minutes (authored audit gates).
pub fn audit_capture(
    root: &Path,
    prior_summary: &Path,
    prior_toi: &Path,
    horizon: f64,
    target: f64,
) -> Result<Value> {
    if !horizon.is_finite() || horizon <= 0.0 || !target.is_finite() || target <= 0.0 {
        return Err("Positive finite horizon/target required".into());
    }
    let teams = index(rows(&root.join("teams.json"))?, "teamId")?;
    let pp = index(rows(&root.join("pp.json"))?, "teamId")?;
    let pk = index(rows(&root.join("pk.json"))?, "teamId")?;
    if teams.keys().ne(pp.keys()) || teams.keys().ne(pk.keys()) {
        return Err("Team coverage differs".into());
    }
    let history = index(rows(prior_summary)?, "playerId")?;
    let history_toi = index(rows(prior_toi)?, "playerId")?;
    if history.keys().ne(history_toi.keys()) {
        return Err("Prior player coverage differs".into());
    }
    let mut out = Vec::new();
    for (team_id, team) in &teams {
        let gp = number(team, "gamesPlayed")?;
        if gp <= 0.0 {
            return Err("Team exposure must be positive".into());
        }
        let gf = number(team, "goalsFor")?;
        let ga = number(team, "goalsAgainst")?;
        let power = &pp[team_id];
        let kill = &pk[team_id];
        if number(power, "gamesPlayed")? != gp || number(kill, "gamesPlayed")? != gp {
            return Err("Special-teams game window differs".into());
        }
        let skaters = index(
            rows(&root.join(format!("team-{team_id}-summary.json")))?,
            "playerId",
        )?;
        let toi = index(
            rows(&root.join(format!("team-{team_id}-timeonice.json")))?,
            "playerId",
        )?;
        if skaters.keys().ne(toi.keys()) {
            return Err(format!("Team {team_id}: TOI coverage differs").into());
        }
        let mut players = Vec::new();
        for (player_id, p) in skaters {
            let t = &toi[&player_id];
            let player_gp = number(&p, "gamesPlayed")?;
            if number(t, "gamesPlayed")? != player_gp || player_gp > gp {
                return Err("Player game window differs".into());
            }
            let position = p["positionCode"].as_str().ok_or("Missing position")?;
            if !["D", "C", "L", "R"].contains(&position) {
                return Err("Unexpected skater position".into());
            }
            let ev_seconds = number(t, "evTimeOnIce")?;
            let pp_seconds = number(t, "ppTimeOnIce")?;
            let (ev_base, pp_base) = if let (Some(old), Some(old_toi)) =
                (history.get(&player_id), history_toi.get(&player_id))
            {
                (
                    prior_baseline(
                        number(old, "evPoints")?,
                        number(old_toi, "evTimeOnIce")?,
                        ev_seconds,
                        18000.0,
                    ),
                    prior_baseline(
                        number(old, "ppPoints")?,
                        number(old_toi, "ppTimeOnIce")?,
                        pp_seconds,
                        3600.0,
                    ),
                )
            } else {
                (None, None)
            };
            let goals = number(&p, "goals")?;
            let assists = number(&p, "assists")?;
            if number(&p, "points")? != goals + assists {
                return Err("Player G+A differs from points".into());
            }
            if goals != number(&p, "evGoals")? + number(&p, "ppGoals")? + number(&p, "shGoals")? {
                return Err("Player goal-state sum differs".into());
            }
            players.push(
                json!({"id":player_id,"name":p["skaterFullName"],"position":position,
                "games":player_gp,"goals":goals,"assists":assists,"points":goals+assists,
                "ev_goals":number(&p,"evGoals")?,"pp_goals":number(&p,"ppGoals")?,
                "ev_points":number(&p,"evPoints")?,"pp_points":number(&p,"ppPoints")?,
                "ev_seconds":ev_seconds,"pp_seconds":pp_seconds,
                "ev_prior_baseline":ev_base,"pp_prior_baseline":pp_base}),
            );
        }
        let sum = |key: &str, defense: Option<bool>| {
            players
                .iter()
                .filter(|p| defense.map(|d| (p["position"] == "D") == d).unwrap_or(true))
                .map(|p| p[key].as_f64().unwrap())
                .sum::<f64>()
        };
        let player_goals = sum("goals", None);
        let d_goals = sum("goals", Some(true));
        let pp_goals = number(power, "powerPlayGoalsFor")?;
        let pp_opps = number(power, "ppOpportunities")?;
        let pp_seconds = number(power, "ppTimeOnIcePerGame")? * gp;
        let pk_ga = number(kill, "ppGoalsAgainst")?;
        let pk_opps = number(kill, "timesShorthanded")?;
        let pk_seconds = number(kill, "pkTimeOnIcePerGame")? * gp;
        out.push(json!({"team_id":team_id,"team":team["teamFullName"],"games":gp,"gf":gf,"ga":ga,
            "gf_pace":gf/gp*horizon,"ga_pace":ga/gp*horizon,"net_pace":(gf-ga)/gp*horizon,
            "target_gap_pace":gf/gp*horizon-target,"player_goals":player_goals,"unallocated_goals":gf-player_goals,
            "d_goals":d_goals,"d_assists":sum("assists",Some(true)),"d_goal_pace":d_goals/gp*horizon,
            "f_goals":sum("goals",Some(false)),"d_share_of_credited_goals":ratio(d_goals,player_goals),
            "pp_goals":pp_goals,"pp_opportunities":pp_opps,"pp_pct":ratio(pp_goals,pp_opps).map(|r|100.0*r),
            "pp_minutes_per_game":pp_seconds/gp/60.0,"pp_gf60":ratio(3600.0*pp_goals,pp_seconds),
            "pk_ga":pk_ga,"pk_opportunities":pk_opps,"pk_pct":ratio(pk_opps-pk_ga,pk_opps).map(|r|100.0*r),
            "pk_ga60":ratio(3600.0*pk_ga,pk_seconds),
            "forward_usage_tiers":tiers(&players,false)?,"defense_usage_tiers":tiers(&players,true)?,"players":players}));
    }
    Ok(
        json!({"schema":"icelines.deployment-audit.0.1-experimental","horizon":horizon,"target":target,
        "minimum_prior_ev_seconds":18000,"minimum_prior_pp_seconds":3600,
        "tier_method":"descending total team-window EV seconds, ID tie break; F groups of 3, D groups of 2, reserves retained; not observed units",
        "limits":["Descriptive pace is not a forecast","Prior-rate comparator uses realized minutes, no contextual adjustment",
            "Broad EV is not 5v5-only","Scoring credits do not grade prevention","PP already included in player goals"],"teams":out}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_exposure_is_not_zero_skill() {
        assert_eq!(prior_baseline(0.0, 0.0, 600.0, 3600.0), None);
    }
    #[test]
    fn supported_zero_and_state_seconds() {
        assert_eq!(prior_baseline(0.0, 3600.0, 600.0, 3600.0), Some(0.0));
        assert_eq!(prior_baseline(6.0, 3600.0, 1800.0, 3600.0), Some(3.0));
    }
    #[test]
    fn sparse_history_and_undefined_share() {
        assert_eq!(prior_baseline(2.0, 3599.0, 600.0, 3600.0), None);
        assert_eq!(ratio(0.0, 0.0), None);
    }
    #[test]
    fn duplicates_rejected() {
        assert!(index(vec![json!({"id":1}), json!({"id":1})], "id").is_err());
    }
    #[test]
    fn invalid_numeric_input_rejected() {
        assert!(number(&json!({"g":-1}), "g").is_err());
        assert!(number(&json!({}), "g").is_err());
    }
    #[test]
    fn tiers_preserve_reserves_and_do_not_duplicate_credits() {
        let p: Vec<_> = (0..15)
            .map(|i| {
                json!({"id":i,"name":i.to_string(),"position":"C","ev_seconds":100-i,
            "goals":1,"assists":2,"points":3,"ev_points":2,"ev_prior_baseline":null})
            })
            .collect();
        let result = tiers(&p, false).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(result[4]["players"], 3);
        assert_eq!(
            result
                .iter()
                .map(|r| r["goals"].as_f64().unwrap())
                .sum::<f64>(),
            15.0
        );
        assert_eq!(result[0]["baseline_covered_players"], 0);
    }
}
