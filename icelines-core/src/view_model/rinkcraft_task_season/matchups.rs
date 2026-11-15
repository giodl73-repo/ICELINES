//! Calendar-independent matchup boundary using the unchanged task-season route.
use super::*;
pub mod tactics;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskExposure {
    pub attack_team: String,
    pub team: String,
    pub member_id: String,
    pub capability: String,
    pub task: String,
    pub expected_contests_per_attack_window: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Matchup {
    pub home: String,
    pub away: String,
    pub home_attack: serde_json::Value,
    pub away_attack: serde_json::Value,
    pub home_model_goals_mean: f64,
    pub away_model_goals_mean: f64,
    pub rate_cap_applied: bool,
    pub probabilities: season::GameOutcomeProbabilities,
    pub task_opportunities: Vec<TaskExposure>,
}
fn exposures(a: &Team, b: &Team, t: &Table) -> Vec<TaskExposure> {
    let am: BTreeMap<_, _> = a
        .members
        .iter()
        .map(|m| (m.id.as_str(), &m.character))
        .collect();
    let bm: BTreeMap<_, _> = b
        .members
        .iter()
        .map(|m| (m.id.as_str(), &m.character))
        .collect();
    let mut rows = BTreeMap::<(String, String, String, String), f64>::new();
    for x in &a.units {
        for y in &b.units {
            let duels = [
                (&x.goalie, 5, &y.forechecker, 5, "outlet"),
                (&x.creator, 2, &y.interceptor, 4, "connection"),
                (&x.receiver, 1, &y.interceptor, 5, "reception"),
                (&x.receiver, 3, &y.gap_defender, 5, "release"),
                (&x.receiver, 3, &y.goalie, 0, "finish"),
            ];
            let mut reach = x.weight * y.weight;
            for (aid, ac, bid, bc, task) in duels {
                for (team, id, cap) in [
                    (
                        &a.team,
                        aid,
                        am[aid.as_str()].capabilities[ac].label.clone(),
                    ),
                    (
                        &b.team,
                        bid,
                        bm[bid.as_str()].capabilities[bc].label.clone(),
                    ),
                ] {
                    *rows
                        .entry((team.clone(), id.clone(), cap, task.into()))
                        .or_default() += reach;
                }
                reach *= if task == "finish" {
                    t[bm[bid.as_str()].capabilities[bc].score as usize]
                        [am[aid.as_str()].capabilities[ac].score as usize][2]
                } else {
                    t[am[aid.as_str()].capabilities[ac].score as usize]
                        [bm[bid.as_str()].capabilities[bc].score as usize][0]
                };
            }
        }
    }
    rows.into_iter()
        .map(|((team, member_id, capability, task), v)| TaskExposure {
            attack_team: a.team.clone(),
            team,
            member_id,
            capability,
            task,
            expected_contests_per_attack_window: v,
        })
        .collect()
}
pub fn project(cr: &game::GameRules, r: &Rules, teams: &[Team]) -> Result<Vec<Matchup>, String> {
    validate_rules(r)?;
    cr.validate()?;
    if !(2..=32).contains(&teams.len()) {
        return Err("Matchups require 2-32 teams".into());
    }
    let mut members = BTreeSet::new();
    let mut compiled = BTreeMap::new();
    let mut rosters = BTreeMap::new();
    for team in teams {
        if team.team.trim().is_empty()
            || team.team.len() > 80
            || team.team.trim() != team.team
            || compiled.contains_key(&team.team)
        {
            return Err("Invalid/duplicate matchup team".into());
        }
        compiled.insert(team.team.clone(), patterns(team, cr, &mut members)?);
        rosters.insert(team.team.clone(), team);
    }
    let t = table(r.score_rule)?;
    let neutral = route([10; 9], [10; 9], &t).2;
    let mut rows = Vec::new();
    for (home, hp) in &compiled {
        for (away, ap) in &compiled {
            if home == away {
                continue;
            }
            let h = attack(hp, ap, &t);
            let a = attack(ap, hp, &t);
            let raw_home = r.neutral_goals_per_team
                * h["bounded_window_goal_probability"].as_f64().unwrap()
                / neutral
                * r.home_goals_multiplier;
            let raw_away = r.neutral_goals_per_team
                * a["bounded_window_goal_probability"].as_f64().unwrap()
                / neutral;
            let hm = raw_home.clamp(r.minimum_goals_mean, r.maximum_goals_mean);
            let am = raw_away.clamp(r.minimum_goals_mean, r.maximum_goals_mean);
            let mut tasks = exposures(rosters[home], rosters[away], &t);
            tasks.extend(exposures(rosters[away], rosters[home], &t));
            rows.push(Matchup {
                home: home.clone(),
                away: away.clone(),
                home_attack: h,
                away_attack: a,
                home_model_goals_mean: hm,
                away_model_goals_mean: am,
                rate_cap_applied: hm != raw_home || am != raw_away,
                probabilities: outcome(hm, am),
                task_opportunities: tasks,
            });
        }
    }
    Ok(rows)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn n_team_boundary_matches_all_existing_32_team_pairs() {
        let q = super::super::tests::fixture();
        let (_, _, audit) = super::super::project(&q, &q.reference).unwrap();
        let new = project(&q.character_rules, &q.rules, &q.reference).unwrap();
        assert_eq!(new.len(), 992);
        for (n, o) in new.iter().zip(audit["pair_audits"].as_array().unwrap()) {
            let v = serde_json::to_value(n).unwrap();
            for (k, value) in o.as_object().unwrap() {
                assert_eq!(&v[k], value, "{k}");
            }
        }
    }
    #[test]
    fn neutral_symmetry_task_identity_and_invalid_rosters() {
        let mut q = super::super::tests::fixture();
        q.rules.home_goals_multiplier = 1.0;
        let teams = q.reference[..3].to_vec();
        let rows = project(&q.character_rules, &q.rules, &teams).unwrap();
        assert_eq!(rows.len(), 6);
        for r in &rows {
            assert!((r.home_model_goals_mean - 3.0).abs() < 1e-12);
            assert!(
                (r.probabilities.home_regulation_win - r.probabilities.away_regulation_win).abs()
                    < 1e-12
            );
            for x in &r.task_opportunities {
                assert!(teams
                    .iter()
                    .any(|t| t.team == x.team && t.members.iter().any(|m| m.id == x.member_id)));
                assert!(x.expected_contests_per_attack_window > 0.0);
            }
        }
        let mut bad = teams.clone();
        bad[1].team = bad[0].team.clone();
        assert!(project(&q.character_rules, &q.rules, &bad).is_err());
        bad = teams.clone();
        bad[0].members[0].character.capabilities[0].score = 20;
        assert!(project(&q.character_rules, &q.rules, &bad).is_err());
        assert!(project(&q.character_rules, &q.rules, &teams[..1]).is_err());
    }
}
