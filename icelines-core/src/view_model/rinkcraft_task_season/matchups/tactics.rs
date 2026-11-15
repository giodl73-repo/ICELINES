//! Versioned opportunity policies, reusing the previously inspected route engine.
use super::*;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ForwardRoles {
    Declared,
    ForwardCycle,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamPlan {
    pub team: String,
    pub forward_roles: ForwardRoles,
    pub mixture: routes::Mixture,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub teams: Vec<TeamPlan>,
}
fn opportunities(v: &serde_json::Value, attack: &Team, defense: &Team) -> Vec<TaskExposure> {
    let people: BTreeMap<_, _> = attack
        .members
        .iter()
        .chain(&defense.members)
        .map(|m| (m.id.as_str(), &m.character))
        .collect();
    v["task_opportunities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            let id = x["member_id"].as_str().unwrap();
            let cap = x["capability_index"].as_u64().unwrap() as usize;
            TaskExposure {
                attack_team: attack.team.clone(),
                team: x["team"].as_str().unwrap().into(),
                member_id: id.into(),
                capability: people[id].capabilities[cap].label.clone(),
                task: "route-mixture capability checks".into(),
                expected_contests_per_attack_window: x["expected_contests_per_selected_window"]
                    .as_f64()
                    .unwrap(),
            }
        })
        .collect()
}
pub fn project(
    cr: &game::GameRules,
    r: &super::super::Rules,
    teams: &[Team],
    rules: &Rules,
) -> Result<(Vec<Team>, Vec<Matchup>), String> {
    // Validate the original exports/weights before any role transformation.
    super::project(cr, r, teams)?;
    if rules.schema != "rinkcraft.tournament-tactics.0.1"
        || rules.version != "0.2.0-playtest"
        || rules.kind != KIND
        || rules.teams.len() != teams.len()
    {
        return Err("Invalid tournament tactics identity or coverage".into());
    }
    let names: BTreeSet<_> = teams.iter().map(|t| t.team.as_str()).collect();
    let mut plans = BTreeMap::new();
    for p in &rules.teams {
        if !names.contains(p.team.as_str()) || plans.insert(p.team.as_str(), p).is_some() {
            return Err("Unknown/duplicate tactical team".into());
        }
    }
    let mut expanded = teams.to_vec();
    for t in &mut expanded {
        if plans[t.team.as_str()].forward_roles == ForwardRoles::Declared {
            continue;
        }
        let mut units = Vec::new();
        for u in &t.units {
            let actors = [u.creator.clone(), u.receiver.clone(), u.forechecker.clone()];
            if actors.iter().collect::<BTreeSet<_>>() != u.forwards.iter().collect::<BTreeSet<_>>()
            {
                return Err("Forward cycling requires three distinct forward task actors".into());
            }
            for i in 0..3 {
                let mut v = u.clone();
                v.id = format!("{}:cycle-{i}", u.id);
                v.weight /= 3.0;
                v.creator = actors[i].clone();
                v.receiver = actors[(i + 1) % 3].clone();
                v.forechecker = actors[(i + 2) % 3].clone();
                units.push(v);
            }
        }
        if units.len() > 48 {
            return Err(
                "Role expansion exceeds 48 active units; condition on the starting goalie first"
                    .into(),
            );
        }
        t.units = units;
    }
    let table = table(r.score_rule)?;
    let anchor = route([10; 9], [10; 9], &table).2;
    let mut pairs = Vec::new();
    let mut ordered: Vec<_> = expanded.iter().collect();
    ordered.sort_by(|a, b| a.team.cmp(&b.team));
    for home in &ordered {
        for away in &ordered {
            if home.team == away.team {
                continue;
            }
            let mut h = routes::inspect(
                cr,
                r.score_rule,
                home,
                away,
                &plans[home.team.as_str()].mixture,
            )?;
            let mut a = routes::inspect(
                cr,
                r.score_rule,
                away,
                home,
                &plans[away.team.as_str()].mixture,
            )?;
            let raw_h = r.neutral_goals_per_team * h["goal_probability"].as_f64().unwrap() / anchor
                * r.home_goals_multiplier;
            let raw_a = r.neutral_goals_per_team * a["goal_probability"].as_f64().unwrap() / anchor;
            let hm = raw_h.clamp(r.minimum_goals_mean, r.maximum_goals_mean);
            let am = raw_a.clamp(r.minimum_goals_mean, r.maximum_goals_mean);
            let mut tasks = opportunities(&h, home, away);
            tasks.extend(opportunities(&a, away, home));
            for (v, plan) in [
                (&mut h, plans[home.team.as_str()]),
                (&mut a, plans[away.team.as_str()]),
            ] {
                v["tournament_bridge"] = serde_json::json!({"common_legacy_neutral_window_anchor":anchor,"neutral_goals_per_team":r.neutral_goals_per_team,
            "anchor_policy":"fixed across teams/routes/variants; not the route-mixture neutral denominator","forward_roles":plan.forward_roles});
            }
            pairs.push(Matchup {
                home: home.team.clone(),
                away: away.team.clone(),
                home_attack: h,
                away_attack: a,
                home_model_goals_mean: hm,
                away_model_goals_mean: am,
                rate_cap_applied: hm != raw_h || am != raw_a,
                probabilities: outcome(hm, am),
                task_opportunities: tasks,
            });
        }
    }
    Ok((expanded, pairs))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (
        game::GameRules,
        super::super::super::Rules,
        Vec<Team>,
        Rules,
    ) {
        let q = super::super::super::tests::fixture();
        let teams = q.reference[..2].to_vec();
        let mut r = q.rules;
        r.home_goals_multiplier = 1.0;
        let plans = Rules {
            schema: "rinkcraft.tournament-tactics.0.1".into(),
            version: "0.2.0-playtest".into(),
            kind: KIND.into(),
            teams: teams
                .iter()
                .map(|t| TeamPlan {
                    team: t.team.clone(),
                    forward_roles: ForwardRoles::Declared,
                    mixture: routes::Mixture {
                        schema: "rinkcraft.route-mixture.0.1".into(),
                        version: "0.1.0-playtest".into(),
                        kind: KIND.into(),
                        routes: vec![routes::RouteWeight {
                            route: routes::RouteKind::LegacyOutletPass,
                            weight: 1.0,
                        }],
                    },
                })
                .collect(),
        };
        (q.character_rules, r, teams, plans)
    }
    fn set_routes(p: &mut Rules, route: routes::RouteKind) {
        for x in &mut p.teams {
            x.mixture.routes = vec![routes::RouteWeight { route, weight: 1.0 }];
        }
    }
    fn change(cr: &game::GameRules, t: &mut Team, id: &str, cap: usize, score: u8) {
        let m = t.members.iter_mut().find(|m| m.id == id).unwrap();
        let c = &m.character;
        let mut scores = std::array::from_fn(|i| c.capabilities[i].score);
        scores[cap] = score;
        m.character = game::build(
            cr,
            game::BuildRequest {
                template_id: c.template_id.clone(),
                name: c.name.clone(),
                position: c.position.clone(),
                scores: game::ScoreMethod::Manual { scores },
                inspiration: None,
            },
        )
        .unwrap();
    }
    #[test]
    fn declared_legacy_reuses_existing_non_neutral_goal_model() {
        let (cr, r, mut teams, p) = fixture();
        let id = teams[0].units[0].creator.clone();
        change(&cr, &mut teams[0], &id, 2, 14);
        let old = super::super::project(&cr, &r, &teams).unwrap();
        let (_, new) = project(&cr, &r, &teams, &p).unwrap();
        for (a, b) in old.iter().zip(new) {
            assert!((a.home_model_goals_mean - b.home_model_goals_mean).abs() < 1e-12);
            assert!((a.away_model_goals_mean - b.away_model_goals_mean).abs() < 1e-12);
        }
    }
    #[test]
    fn cycling_exposes_passing_on_the_previously_unused_hunter() {
        let (cr, r, mut teams, mut p) = fixture();
        let id = teams[0].units[0].forechecker.clone();
        change(&cr, &mut teams[0], &id, 2, 16);
        let (_, fixed) = project(&cr, &r, &teams, &p).unwrap();
        for x in &mut p.teams {
            x.forward_roles = ForwardRoles::ForwardCycle;
        }
        let (_, cycle) = project(&cr, &r, &teams, &p).unwrap();
        assert!(cycle[0].home_model_goals_mean > fixed[0].home_model_goals_mean);
        assert!(cycle[0].task_opportunities.iter().any(|x| x.member_id == id
            && x.capability == "Passing"
            && x.expected_contests_per_attack_window > 0.0));
    }
    #[test]
    fn new_pass_route_tests_shooting_once_and_lane_reading_separately() {
        let (cr, r, teams, mut p) = fixture();
        set_routes(&mut p, routes::RouteKind::OutletPass);
        let (_, new) = project(&cr, &r, &teams, &p).unwrap();
        let stages = new[0].home_attack["routes"][0]["stages"]
            .as_array()
            .unwrap();
        assert_eq!(
            stages
                .iter()
                .filter(|s| s["actor_capability"] == "Shooting")
                .count(),
            1
        );
        assert!(stages
            .iter()
            .any(|s| s["task"] == "lane_read" && s["actor_capability"] == "Reading"));
    }
    #[test]
    fn carry_creates_a_skating_task_without_changing_the_pass_route() {
        let (cr, r, mut teams, mut p) = fixture();
        let id = teams[0].units[0].creator.clone();
        let (_, old) = project(&cr, &r, &teams, &p).unwrap();
        change(&cr, &mut teams[0], &id, 0, 16);
        let (_, pass) = project(&cr, &r, &teams, &p).unwrap();
        assert_eq!(old[0].home_model_goals_mean, pass[0].home_model_goals_mean);
        set_routes(&mut p, routes::RouteKind::Carry);
        let (_, carry) = project(&cr, &r, &teams, &p).unwrap();
        assert!(carry[0]
            .task_opportunities
            .iter()
            .any(|x| x.member_id == id && x.capability == "Skating"));
    }
    #[test]
    fn mixed_routes_use_one_common_anchor_without_hidden_rate_normalization() {
        let (cr, r, teams, mut p) = fixture();
        for x in &mut p.teams {
            x.mixture.routes = vec![
                routes::RouteWeight {
                    route: routes::RouteKind::OutletPass,
                    weight: 0.5,
                },
                routes::RouteWeight {
                    route: routes::RouteKind::Carry,
                    weight: 0.5,
                },
            ];
        }
        let (_, pairs) = project(&cr, &r, &teams, &p).unwrap();
        assert!((pairs[0].home_model_goals_mean - 3.0 * (0.5 + 0.5 / 0.475)).abs() < 1e-12);
    }
    #[test]
    fn invalid_policy_coverage_mixtures_and_forward_bindings_reject() {
        let (cr, r, mut teams, p) = fixture();
        let mut bad = p.clone();
        bad.teams.pop();
        assert!(project(&cr, &r, &teams, &bad).is_err());
        bad = p.clone();
        bad.teams[1].team = bad.teams[0].team.clone();
        assert!(project(&cr, &r, &teams, &bad).is_err());
        bad = p.clone();
        bad.teams[0].mixture.routes[0].weight = 0.5;
        assert!(project(&cr, &r, &teams, &bad).is_err());
        bad = p;
        for x in &mut bad.teams {
            x.forward_roles = ForwardRoles::ForwardCycle;
        }
        teams[0].units[0].creator = teams[0].units[0].interceptor.clone();
        assert!(project(&cr, &r, &teams, &bad).is_err());
    }
    #[test]
    fn cycling_preserves_characters_weights_and_all_three_forward_tasks() {
        let (cr, r, teams, mut p) = fixture();
        let original = serde_json::to_value(&teams).unwrap();
        for x in &mut p.teams {
            x.forward_roles = ForwardRoles::ForwardCycle;
        }
        let (expanded, _) = project(&cr, &r, &teams, &p).unwrap();
        assert_eq!(original, serde_json::to_value(&teams).unwrap());
        for (a, b) in teams.iter().zip(expanded) {
            assert_eq!(
                serde_json::to_value(&a.members).unwrap(),
                serde_json::to_value(&b.members).unwrap()
            );
            assert_eq!(b.units.len(), 3 * a.units.len());
            assert!((b.units.iter().map(|u| u.weight).sum::<f64>() - 1.0).abs() < 1e-12);
            for id in &a.units[0].forwards {
                assert_eq!(b.units.iter().filter(|u| &u.creator == id).count(), 1);
                assert_eq!(b.units.iter().filter(|u| &u.receiver == id).count(), 1);
                assert_eq!(b.units.iter().filter(|u| &u.forechecker == id).count(), 1);
            }
        }
    }
    #[test]
    fn wider_named_coverage_does_not_create_extra_team_passing_strength() {
        let (cr, r, mut teams, mut p) = fixture();
        let ids = teams[0].units[0].forwards.clone();
        for id in &ids {
            change(&cr, &mut teams[0], id, 2, 14);
        }
        let (_, fixed) = project(&cr, &r, &teams, &p).unwrap();
        for x in &mut p.teams {
            x.forward_roles = ForwardRoles::ForwardCycle;
        }
        let (_, cycle) = project(&cr, &r, &teams, &p).unwrap();
        assert!((fixed[0].home_model_goals_mean - cycle[0].home_model_goals_mean).abs() < 1e-12);
        let passes = |m: &Matchup| {
            m.task_opportunities
                .iter()
                .filter(|x| {
                    x.team == teams[0].team
                        && x.attack_team == teams[0].team
                        && x.capability == "Passing"
                })
                .map(|x| x.expected_contests_per_attack_window)
                .sum::<f64>()
        };
        assert!((passes(&fixed[0]) - passes(&cycle[0])).abs() < 1e-12);
    }
}
