//! Same-character, same-seed rule comparisons; no automatic balance optimization.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variant {
    pub id: String,
    pub label: String,
    pub tactics: Option<matchups::tactics::Rules>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema: String,
    pub kind: String,
    pub variants: Vec<Variant>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StudyResult {
    pub schema: String,
    pub base_input_sha256: String,
    pub variants: Vec<(Variant, ResultSet)>,
}
pub fn run(base: &Request, plan: &Plan) -> Result<StudyResult, String> {
    let mut names = BTreeSet::new();
    let games = base.teams.len() * (base.teams.len().saturating_sub(1)) / 2 + 1;
    if plan.schema != "rinkcraft.tournament-rule-study.0.1"
        || plan.kind != KIND
        || plan.variants.is_empty()
        || plan.variants.len() > 16
        || games * base.batch_trials as usize * plan.variants.len() > 2000000
    {
        return Err("Invalid rule-study identity or work budget".into());
    }
    for v in &plan.variants {
        if v.id.is_empty()
            || v.id.len() > 64
            || !v
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
            || !names.insert(v.id.clone())
            || v.label.trim().is_empty()
            || v.label.len() > 160
        {
            return Err("Invalid/duplicate variant ID or label".into());
        }
    }
    let mut variants = Vec::new();
    for v in &plan.variants {
        let mut q = base.clone();
        q.tactics = v.tactics.clone();
        q.schema = if q.tactics.is_some() {
            TACTICAL_SCHEMA
        } else {
            TOURNAMENT_SCHEMA
        }
        .into();
        variants.push((v.clone(), super::run(&q)?));
    }
    Ok(StudyResult {
        schema: "rinkcraft.tournament-rule-study-result.0.1".into(),
        base_input_sha256: hash(base)?,
        variants,
    })
}
pub fn html(study: &StudyResult) -> String {
    let e = super::report::e;
    let names: Vec<_> = study.variants[0]
        .1
        .active_teams
        .iter()
        .map(|t| t.roster.team.clone())
        .collect();
    let mut s=String::from("<!doctype html><html lang=en><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>Rinkcraft rules trial</title><style>*{box-sizing:border-box}body{margin:0;background:#eef5f7;color:#112c3b;font:15px/1.45 system-ui}main{max-width:1200px;margin:auto;padding:20px}header{border-top:8px solid #006968;border-bottom:3px solid #112c3b}section{background:white;border:1px solid #b8cad1;padding:16px;margin:18px 0;min-width:0}h1{font-size:29px}h2{font-size:20px}table{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums}th,td{padding:8px;text-align:left;border-bottom:1px solid #dbe5e9}th{font-size:12px;text-transform:uppercase}.scroll{overflow-x:auto}small{color:#45616d}a{color:#006968}footer{overflow-wrap:anywhere;font-size:12px}@media(max-width:600px){main{padding:12px}th,td{padding:6px}}</style><main><header><p>RINKCRAFT · SAME CHARACTERS, NEW RULES</p><h1>Tournament rules trial</h1><p>Authored experiments, not NHL probabilities. Rosters, attributes, starters, tournament seeds and rate anchor are held fixed.</p></header><section><h2>What changes</h2><ol><li>Optional forward task cycling exposes passing/shooting outside fixed slots.</li><li>The new pass route uses Reading to open the lane, then one Shooting check against Tracking.</li><li>Declared passing/carrying opportunity mixtures bring Skating into the game model.</li></ol><p>Forward cycling is a tested usage option, not a universal coaching assignment. Route mixtures select different starting contexts; they are not free alternatives from an identical puck state.</p></section><section><h2>Title frequency by rule</h2><div class=scroll><table><tr><th>Variant</th>");
    for name in &names {
        s.push_str(&format!("<th>{}</th>", e(name)));
    }
    s.push_str("<th>Mean model goals / team</th><th>Capped ordered matchups</th></tr>");
    for (v, r) in &study.variants {
        s.push_str(&format!(
            "<tr><td><a href=\"{}/\">{}</a></td>",
            e(&v.id),
            e(&v.label)
        ));
        for name in &names {
            let t = r
                .title_frequencies
                .iter()
                .find(|t| &t.team == name)
                .unwrap();
            s.push_str(&format!("<td>{:.2}%</td>", 100.0 * t.frequency));
        }
        let mean = r
            .matchups
            .iter()
            .map(|m| m.home_model_goals_mean + m.away_model_goals_mean)
            .sum::<f64>()
            / (2 * r.matchups.len()) as f64;
        s.push_str(&format!(
            "<td>{:.3}</td><td>{}</td></tr>",
            mean,
            r.matchups.iter().filter(|m| m.rate_cap_applied).count()
        ));
    }
    s.push_str("</table></div><p><small>Each column uses the same tournament seed batch. The game model uses one common legacy neutral-window anchor; shorter routes can increase scoring. No per-route renormalization hides that change. Title frequency alone is not a rule-acceptance criterion.</small></p></section><section><h2>Attribute investment the model actually tests</h2><p>Nominal points above neutral 10, split by any tested capability in the declared opportunity policy. This is a coverage diagnostic, not an effectiveness rating or fair price.</p><div class=scroll><table><tr><th>Variant</th><th>Team</th><th>Roster delta</th><th>Tested delta</th></tr>");
    for (v, r) in &study.variants {
        let used: BTreeSet<_> = r
            .matchups
            .iter()
            .flat_map(|m| &m.task_opportunities)
            .filter(|x| x.expected_contests_per_attack_window > 0.0)
            .map(|x| (x.member_id.as_str(), x.capability.as_str()))
            .collect();
        for t in &r.active_teams {
            let mut budget = 0i32;
            let mut tested = 0i32;
            for m in &t.roster.members {
                for c in &m.character.capabilities {
                    let delta = c.score as i32 - 10;
                    budget += delta;
                    if used.contains(&(m.id.as_str(), c.label.as_str())) {
                        tested += delta;
                    }
                }
            }
            s.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                e(&v.label),
                e(&t.roster.team),
                budget,
                tested
            ));
        }
    }
    let r = &study.variants[0].1;
    s.push_str(&format!("</table></div></section><section><h2>Limits and review</h2><p>{} variants · {} unchanged characters · {} tournament seeds per variant. Open each variant for scores, standings, profiles, model gates and evidence.</p><p>These are rule changes under investigation. Do not choose a rule just because title odds become equal. Specialist role fit, neutral scoring inflation, caps and missing goalie/continuous-play mechanics remain visible. Goalkeeper Positioning, Movement, Rebounds and Recovery still have no task here.</p><p><a href=\"comparison.json\">Full comparison JSON</a></p></section><footer>Base input SHA-256 {}</footer></main></html>",study.variants.len(),r.request.teams.iter().map(|t|t.roster.members.len()).sum::<usize>(),r.request.batch_trials,e(&study.base_input_sha256)));
    s
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn omitted_tactics_stays_absent_in_legacy_serialization() {
        let q = super::super::tests::fixture();
        assert!(serde_json::to_value(&q).unwrap().get("tactics").is_none());
    }
    #[test]
    fn variant_coverage_ids_and_shared_inputs_are_validated() {
        let q = super::super::tests::fixture();
        let mut p = Plan {
            schema: "rinkcraft.tournament-rule-study.0.1".into(),
            kind: KIND.into(),
            variants: vec![Variant {
                id: "baseline".into(),
                label: "Frozen baseline".into(),
                tactics: None,
            }],
        };
        let r = run(&q, &p).unwrap();
        assert_eq!(
            serde_json::to_value(&r.variants[0].1.request).unwrap(),
            serde_json::to_value(&q).unwrap()
        );
        p.variants[0].id = "../escape".into();
        assert!(run(&q, &p).is_err());
        p.variants.clear();
        assert!(run(&q, &p).is_err());
    }
    #[test]
    fn tactical_requests_require_explicit_version_two() {
        let mut q = super::super::tests::fixture();
        q.tactics = Some(matchups::tactics::Rules {
            schema: "rinkcraft.tournament-tactics.0.1".into(),
            version: "0.2.0-playtest".into(),
            kind: KIND.into(),
            teams: q
                .teams
                .iter()
                .map(|t| matchups::tactics::TeamPlan {
                    team: t.roster.team.clone(),
                    forward_roles: matchups::tactics::ForwardRoles::Declared,
                    mixture: routes::Mixture {
                        schema: "rinkcraft.route-mixture.0.1".into(),
                        version: "0.1.0-playtest".into(),
                        kind: KIND.into(),
                        routes: vec![routes::RouteWeight {
                            route: routes::RouteKind::OutletPass,
                            weight: 1.0,
                        }],
                    },
                })
                .collect(),
        });
        assert!(super::super::run(&q).is_err());
        q.schema = TACTICAL_SCHEMA.into();
        let r = super::super::run(&q).unwrap();
        assert_eq!(r.model_id, "rinkcraft.task-matchup.0.2");
        q.tactics = None;
        assert!(super::super::run(&q).is_err());
    }
}
