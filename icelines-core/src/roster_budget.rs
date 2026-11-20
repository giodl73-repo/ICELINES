//! Conditional deployment-continuation scenarios; no calibrated forecasting claim.
use crate::deployment_audit::{index, number, rows};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Copy, Default)]
struct Rate {
    g: f64,
    a: f64,
    seconds: f64,
}
fn group(p: &Value) -> Result<&'static str> {
    match p["positionCode"].as_str() {
        Some("D") => Ok("D"),
        Some("C" | "L" | "R") => Ok("F"),
        _ => Err("Unsupported skater position".into()),
    }
}
fn rate(p: &Value, t: &Value, state: &str) -> Result<Rate> {
    let g = number(p, &format!("{state}Goals"))?;
    let points = number(p, &format!("{state}Points"))?;
    if points < g {
        return Err("State points below goals".into());
    }
    Ok(Rate {
        g,
        a: points - g,
        seconds: number(t, &format!("{state}TimeOnIce"))?,
    })
}
fn contribution(r: Rate, seconds: f64) -> Result<(f64, f64)> {
    if r.seconds <= 0.0 {
        return Err("No reference exposure".into());
    }
    Ok((r.g * seconds / r.seconds, r.a * seconds / r.seconds))
}
fn choose_rate(individual: Option<Rate>, pooled: Rate, minimum: f64) -> (Rate, &'static str) {
    match individual {
        Some(r) if r.seconds >= minimum => (r, "prior_individual"),
        _ => (pooled, "prior_position_pool"),
    }
}

/// Same captured window as deployment_audit; continuation includes observed availability.
pub fn build(
    root: &Path,
    prior_summary: &Path,
    prior_toi: &Path,
    horizon: f64,
    target: f64,
    desired_net: f64,
) -> Result<Value> {
    if ![horizon, target, desired_net].iter().all(|v| v.is_finite())
        || horizon <= 0.0
        || target <= 0.0
        || desired_net < 0.0
    {
        return Err("Invalid planning parameters".into());
    }
    let teams = index(rows(&root.join("teams.json"))?, "teamId")?;
    let pp = index(rows(&root.join("pp.json"))?, "teamId")?;
    let pk = index(rows(&root.join("pk.json"))?, "teamId")?;
    if teams.keys().ne(pp.keys()) || teams.keys().ne(pk.keys()) {
        return Err("Team report coverage mismatch".into());
    }
    let prior = index(rows(prior_summary)?, "playerId")?;
    let prior_toi = index(rows(prior_toi)?, "playerId")?;
    if prior.keys().ne(prior_toi.keys()) {
        return Err("Prior exposure coverage mismatch".into());
    }
    let mut pools = BTreeMap::<(&str, &str), Rate>::new();
    for (pid, p) in &prior {
        for state in ["ev", "pp", "sh"] {
            let r = rate(p, &prior_toi[pid], state)?;
            let pool = pools.entry((group(p)?, state)).or_default();
            pool.g += r.g;
            pool.a += r.a;
            pool.seconds += r.seconds;
        }
    }
    let pk_seconds: f64 = pk
        .values()
        .map(|t| number(t, "pkTimeOnIcePerGame").and_then(|s| Ok(s * number(t, "gamesPlayed")?)))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum();
    let pp_seconds: f64 = pp
        .values()
        .map(|t| number(t, "ppTimeOnIcePerGame").and_then(|s| Ok(s * number(t, "gamesPlayed")?)))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum();
    let pk_ga: f64 = pk
        .values()
        .map(|t| number(t, "ppGoalsAgainst"))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum();
    let pp_ga: f64 = pp
        .values()
        .map(|t| number(t, "shGoalsAgainst"))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum();
    if pk_seconds <= 0.0 || pp_seconds <= 0.0 {
        return Err("No special-teams comparator exposure".into());
    }
    let mut result = Vec::new();
    for (tid, team) in teams {
        let gp = number(&team, "gamesPlayed")?;
        if gp <= 0.0 {
            return Err("No team games".into());
        }
        let skaters = index(
            rows(&root.join(format!("team-{tid}-summary.json")))?,
            "playerId",
        )?;
        let toi = index(
            rows(&root.join(format!("team-{tid}-timeonice.json")))?,
            "playerId",
        )?;
        if skaters.keys().ne(toi.keys()) {
            return Err("Current exposure coverage mismatch".into());
        }
        let mut players = Vec::new();
        let mut goals = 0.0;
        let mut assists = 0.0;
        let mut dg = 0.0;
        for (pid, p) in skaters {
            let t = &toi[&pid];
            if number(t, "gamesPlayed")? != number(&p, "gamesPlayed")?
                || number(t, "gamesPlayed")? > gp
            {
                return Err("Player game window mismatch".into());
            }
            let pos = group(&p)?;
            let mut states = Vec::new();
            let mut pg = 0.0;
            let mut pa = 0.0;
            for (state, min) in [("ev", 18000.0), ("pp", 3600.0), ("sh", 3600.0)] {
                let seconds = number(t, &format!("{state}TimeOnIce"))? / gp * horizon;
                let individual =
                    if let (Some(old), Some(oldt)) = (prior.get(&pid), prior_toi.get(&pid)) {
                        if group(old)? == pos {
                            Some(rate(old, oldt, state)?)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                let (reference, basis) = choose_rate(individual, pools[&(pos, state)], min);
                let (g, a) = contribution(reference, seconds)?;
                pg += g;
                pa += a;
                states.push(json!({"state":state,"planned_seconds":seconds,"goals":g,"assists":a,"basis":basis,"reference_seconds":reference.seconds}));
            }
            goals += pg;
            assists += pa;
            if pos == "D" {
                dg += pg;
            }
            players.push(json!({"id":pid,"name":p["skaterFullName"],"position_group":pos,"goals":pg,"assists":pa,"points":pg+pa,"states":states}));
        }
        let pk_minutes = number(&pk[&tid], "pkTimeOnIcePerGame")? * horizon / 60.0;
        let pp_minutes = number(&pp[&tid], "ppTimeOnIcePerGame")? * horizon / 60.0;
        let pk_allowance = pk_minutes * 60.0 * pk_ga / pk_seconds;
        let pp_allowance = pp_minutes * 60.0 * pp_ga / pp_seconds;
        let actual_pk = number(&pk[&tid], "ppGoalsAgainst")? / gp * horizon;
        let ga_cap = goals - desired_net;
        result.push(json!({"team_id":tid,"team":team["teamFullName"],"games":gp,"conditional_goals":goals,"conditional_assists":assists,"conditional_points":goals+assists,
            "conditional_defense_goals":dg,"conditional_forward_goals":goals-dg,"gap_to_team_target":goals-target,
            "assist_credit_consistency":assists<=2.0*goals,"goalie_assist_projection":"unmodeled",
            "prevention":{"desired_net_goals":desired_net,"total_ga_ceiling_given_scoring_scenario":ga_cap,"total_ga_ceiling_at_team_target":target-desired_net,
                "pk_ga_at_league_rate_same_minutes":pk_allowance,"own_pp_ga_at_league_rate_same_minutes":pp_allowance,
                "remaining_ev_other_ga_ceiling":ga_cap-pk_allowance-pp_allowance,"pk_current_pace":actual_pk,"pk_excess_over_comparator":actual_pk-pk_allowance,
                "feasible_nonnegative_remainder":ga_cap>=pk_allowance+pp_allowance},"players":players}));
    }
    Ok(
        json!({"schema":"icelines.roster-budget.0.1-experimental","horizon":horizon,"team_target":target,"desired_net_goals":desired_net,
        "scenario":"repeat observed player state exposure and availability over horizon; prior-state individual rates or prior F/D pooled fallback",
        "minimum_prior_seconds":{"ev":18000,"pp":3600,"sh":3600},"league_pk_ga60":3600.0*pk_ga/pk_seconds,"league_own_pp_ga60":3600.0*pp_ga/pp_seconds,
        "limitations":["Conditional scenario, not calibrated roster forecast","Past roster participation repeated, including departed/injured players; not today's healthy roster",
            "Separate goals/assist rates need joint event reconciliation before forecast release","Goalie points not modeled; prevention includes goalie outcomes",
            "Desired net +20 is an authored scenario, not a playoff threshold","EV/other prevention is a residual ceiling, not individual responsibility","No forced rescaling to team target"],"teams":result}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fallback_uses_position_pool_for_missing_and_sparse_history() {
        let pool = Rate {
            g: 2.0,
            a: 4.0,
            seconds: 3600.0,
        };
        assert_eq!(choose_rate(None, pool, 3600.0).1, "prior_position_pool");
        assert_eq!(
            choose_rate(
                Some(Rate {
                    seconds: 20.0,
                    ..pool
                }),
                pool,
                3600.0
            )
            .1,
            "prior_position_pool"
        );
    }
    #[test]
    fn supported_zero_does_not_get_replaced() {
        let r = Rate {
            g: 0.0,
            a: 0.0,
            seconds: 3600.0,
        };
        let chosen = choose_rate(Some(r), Rate { g: 9.0, ..r }, 3600.0);
        assert_eq!(chosen.1, "prior_individual");
        assert_eq!(contribution(chosen.0, 1800.0).unwrap(), (0.0, 0.0));
    }
    #[test]
    fn minutes_scale_without_forcing_target() {
        let r = Rate {
            g: 2.0,
            a: 3.0,
            seconds: 3600.0,
        };
        assert_eq!(contribution(r, 1800.0).unwrap(), (1.0, 1.5));
        assert_eq!(contribution(r, 7200.0).unwrap(), (4.0, 6.0));
    }
    #[test]
    fn no_exposure_is_error() {
        assert!(contribution(Rate::default(), 600.0).is_err());
    }
}
