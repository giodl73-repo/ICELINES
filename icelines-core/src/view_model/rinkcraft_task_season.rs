//! Opt-in authored task route -> goal-rate -> game-probability model.
//! No empirical NHL probabilities or automatic class bonuses.
use super::{
    rinkcraft_contest::{Contest, ScoreRule},
    rinkcraft_game as game, team_game_forecast as forecast, team_season_forecast as season,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
pub const SCHEMA: &str = "rinkcraft.task-season-request.0.1";
pub const RULES_SCHEMA: &str = "rinkcraft.task-season-rules.0.1";
const KIND: &str = "authored_game_scenario_not_nhl_estimate";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    pub score_rule: ScoreRule,
    pub neutral_goals_per_team: f64,
    pub minimum_goals_mean: f64,
    pub maximum_goals_mean: f64,
    pub home_goals_multiplier: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub id: String,
    pub character: game::GameCharacter,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub id: String,
    pub weight: f64,
    pub forwards: [String; 3],
    pub defense: [String; 2],
    pub goalie: String,
    pub creator: String,
    pub receiver: String,
    pub forechecker: String,
    pub interceptor: String,
    pub gap_defender: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Team {
    pub team: String,
    pub members: Vec<Member>,
    pub units: Vec<Unit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub kind: String,
    pub character_rules: game::GameRules,
    pub rules: Rules,
    /// Only game IDs, dates, opponents and season identity are used.
    pub calendar: forecast::TeamGameForecastView,
    pub reference: Vec<Team>,
    pub proposed: Vec<Team>,
    pub config: season::TeamSeasonSimulationConfig,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitWeightEdit {
    pub unit_id: String,
    pub weight: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudyCase {
    pub id: String,
    pub label: String,
    pub team: String,
    pub edits: Vec<super::rinkcraft_season::ScoreEdit>,
    pub swap_creator_receiver: Vec<String>,
    pub unit_weights: Vec<UnitWeightEdit>,
}
fn hash<T: Serialize>(v: &T) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(v).map_err(|e| e.to_string())?)
    ))
}
fn validate_rules(r: &Rules) -> Result<(), String> {
    if r.schema != RULES_SCHEMA
        || r.kind != KIND
        || r.version.trim().is_empty()
        || [
            r.neutral_goals_per_team,
            r.minimum_goals_mean,
            r.maximum_goals_mean,
            r.home_goals_multiplier,
        ]
        .iter()
        .any(|v| !v.is_finite())
        || r.minimum_goals_mean <= 0.0
        || r.maximum_goals_mean > 12.0
        || r.maximum_goals_mean < r.neutral_goals_per_team
        || r.neutral_goals_per_team < r.minimum_goals_mean
        || !(0.5..=2.0).contains(&r.home_goals_multiplier)
    {
        return Err("Invalid authored task rules or goal-rate bounds".into());
    }
    Ok(())
}
type Table = [[[f64; 3]; 21]; 21];
fn table(rule: ScoreRule) -> Result<Table, String> {
    let mut t = [[[0.0; 3]; 21]; 21];
    for a in 1..=20 {
        for b in 1..=20 {
            let c = Contest {
                actor_score: a,
                opposition_score: b,
                score_rule: rule,
                actor_adjustment: 0,
                opposition_adjustment: 0,
            }
            .exact_counts()?;
            t[a as usize][b as usize] = [
                f64::from(c.actor_wins) / 400.0,
                f64::from(c.stalemates) / 400.0,
                f64::from(c.opposition_wins) / 400.0,
            ];
        }
    }
    Ok(t)
}
#[derive(Clone)]
struct Pattern {
    scores: [u8; 9],
    weight: f64,
}
fn patterns(
    t: &Team,
    rules: &game::GameRules,
    global: &mut BTreeSet<String>,
) -> Result<Vec<Pattern>, String> {
    if t.members.len() > 40 || t.units.is_empty() || t.units.len() > 48 {
        return Err("Need 1-48 units and <=40 roster members".into());
    }
    let rj = serde_json::to_string(rules).map_err(|e| e.to_string())?;
    let mut members = BTreeMap::new();
    for m in &t.members {
        if m.id.trim().is_empty()
            || !global.insert(m.id.clone())
            || members.insert(m.id.clone(), &m.character).is_some()
        {
            return Err("Duplicate/invalid task member identity".into());
        }
        game::import_from_json(
            &rj,
            &serde_json::to_string(&m.character).map_err(|e| e.to_string())?,
        )?;
    }
    let mut folded = BTreeMap::<[u8; 9], f64>::new();
    let mut ids = BTreeSet::new();
    let mut total = 0.0;
    for u in &t.units {
        if u.id.trim().is_empty()
            || !ids.insert(&u.id)
            || !u.weight.is_finite()
            || u.weight <= 0.0
            || u.weight > 1.0
        {
            return Err("Invalid/duplicate unit or weight".into());
        }
        let participants: BTreeSet<_> = u
            .forwards
            .iter()
            .chain(u.defense.iter())
            .chain(std::iter::once(&u.goalie))
            .collect();
        if participants.len() != 6 || participants.iter().any(|id| !members.contains_key(*id)) {
            return Err("A unit needs six unique roster members".into());
        }
        if u.forwards.iter().any(|id| {
            !matches!(
                members[id].position.as_str(),
                "F" | "C" | "LW" | "RW" | "L" | "R"
            )
        }) || u
            .defense
            .iter()
            .any(|id| !matches!(members[id].position.as_str(), "D" | "LD" | "RD"))
            || members[&u.goalie].branch != game::Branch::Goalie
        {
            return Err("Invalid forward/defense/goalie unit positions".into());
        }
        let skaters: BTreeSet<_> = u.forwards.iter().chain(u.defense.iter()).collect();
        if u.creator == u.receiver
            || !skaters.contains(&u.creator)
            || !skaters.contains(&u.receiver)
            || !u.forwards.contains(&u.forechecker)
            || !u.defense.contains(&u.interceptor)
            || !u.defense.contains(&u.gap_defender)
            || u.interceptor == u.gap_defender
        {
            return Err(
                "Invalid task actors or self-pass; defense tasks must use different defenders"
                    .into(),
            );
        }
        let score = |id: &str, cap: usize| members[id].capabilities[cap].score;
        let signature = [
            score(&u.goalie, 5),
            score(&u.creator, 2),
            score(&u.receiver, 1),
            score(&u.receiver, 3),
            score(&u.forechecker, 5),
            score(&u.interceptor, 4),
            score(&u.interceptor, 5),
            score(&u.gap_defender, 5),
            score(&u.goalie, 0),
        ];
        *folded.entry(signature).or_default() += u.weight;
        total += u.weight;
    }
    if (total - 1.0).abs() > 1e-9 {
        return Err("Joint unit weights must sum to one".into());
    }
    Ok(folded
        .into_iter()
        .map(|(scores, weight)| Pattern { scores, weight })
        .collect())
}
/// Disjoint terminal masses: four stage disruptions, four stage ties, save/tie/goal.
fn route(a: [u8; 9], b: [u8; 9], t: &Table) -> ([f64; 5], f64, f64, f64) {
    let contests = [
        t[a[0] as usize][b[4] as usize],
        t[a[1] as usize][b[5] as usize],
        t[a[2] as usize][b[6] as usize],
        t[a[3] as usize][b[7] as usize],
        t[b[8] as usize][a[3] as usize],
    ];
    let mut alive = 1.0;
    let mut stopped = [0.0; 5];
    let mut tied = 0.0;
    for i in 0..4 {
        stopped[i] = alive * contests[i][2];
        tied += alive * contests[i][1];
        alive *= contests[i][0];
    }
    stopped[4] = alive * contests[4][0];
    tied += alive * contests[4][1];
    let goal = alive * contests[4][2];
    (stopped, tied, goal, alive)
}
fn attack(a: &[Pattern], b: &[Pattern], t: &Table) -> serde_json::Value {
    let mut stopped = [0.0; 5];
    let mut ties = 0.0;
    let mut goals = 0.0;
    let mut flights = 0.0;
    for x in a {
        for y in b {
            let w = x.weight * y.weight;
            let (s, tied, goal, flight) = route(x.scores, y.scores, t);
            for i in 0..5 {
                stopped[i] += w * s[i];
            }
            ties += w * tied;
            goals += w * goal;
            flights += w * flight;
        }
    }
    serde_json::json!({"bounded_window_goal_probability":goals,"unresolved_probability":ties,"terminal_stage_probabilities":stopped,"on_target_flight_probability":flights})
}
fn poisson(mean: f64) -> Vec<f64> {
    let mut v = vec![(-mean).exp()];
    for i in 1..=128 {
        v.push(v[i - 1] * mean / i as f64);
    }
    v
}
fn outcome(home: f64, away: f64) -> season::GameOutcomeProbabilities {
    let h = poisson(home);
    let a = poisson(away);
    let mut cumulative = 0.0;
    let mut win = 0.0;
    let mut tie = 0.0;
    for i in 0..h.len() {
        win += h[i] * cumulative;
        tie += h[i] * a[i];
        cumulative += a[i];
    }
    season::GameOutcomeProbabilities {
        home_regulation_win: win,
        away_regulation_win: (1.0 - win - tie).max(0.0),
        overtime: tie,
        home_overtime_win: home / (home + away),
    }
}
pub fn project(
    input: &Request,
    teams: &[Team],
) -> Result<
    (
        forecast::TeamGameForecastView,
        season::SeasonProbabilityModel,
        serde_json::Value,
    ),
    String,
> {
    if input.schema != SCHEMA || input.kind != KIND {
        return Err("Invalid authored task request".into());
    }
    validate_rules(&input.rules)?;
    input.character_rules.validate()?;
    let known: BTreeSet<_> = input
        .calendar
        .games
        .iter()
        .flat_map(|g| [g.home_team.clone(), g.away_team.clone()])
        .collect();
    if known.len() != 32 || teams.len() != 32 {
        return Err("Task season requires all 32 teams".into());
    }
    let t = table(input.rules.score_rule)?;
    let neutral = route([10; 9], [10; 9], &t).2;
    let mut selected = BTreeMap::new();
    let mut global = BTreeSet::new();
    for team in teams {
        if !known.contains(&team.team) || selected.contains_key(&team.team) {
            return Err("Unknown/duplicate task team".into());
        }
        selected.insert(
            team.team.clone(),
            patterns(team, &input.character_rules, &mut global)?,
        );
    }
    let mut probabilities = BTreeMap::new();
    let mut pair_audits = Vec::new();
    let mut playoffs = Vec::new();
    for home in &known {
        for away in &known {
            if home == away {
                continue;
            }
            let h = attack(&selected[home], &selected[away], &t);
            let a = attack(&selected[away], &selected[home], &t);
            let raw_home = input.rules.neutral_goals_per_team
                * h["bounded_window_goal_probability"].as_f64().unwrap()
                / neutral
                * input.rules.home_goals_multiplier;
            let raw_away = input.rules.neutral_goals_per_team
                * a["bounded_window_goal_probability"].as_f64().unwrap()
                / neutral;
            let hm = raw_home.clamp(
                input.rules.minimum_goals_mean,
                input.rules.maximum_goals_mean,
            );
            let am = raw_away.clamp(
                input.rules.minimum_goals_mean,
                input.rules.maximum_goals_mean,
            );
            let p = outcome(hm, am);
            let overall = p.home_regulation_win + p.overtime * p.home_overtime_win;
            playoffs.push(season::SuppliedPlayoffProbability {
                home: home.clone(),
                away: away.clone(),
                home_win_probability: overall,
            });
            pair_audits.push(serde_json::json!({"home":home,"away":away,"home_attack":h,"away_attack":a,"home_model_goals_mean":hm,"away_model_goals_mean":am,
            "rate_cap_applied":hm!=raw_home||am!=raw_away,"probabilities":p}));
            probabilities.insert((home.clone(), away.clone()), p);
        }
    }
    let mut games = Vec::new();
    let mut model_games = Vec::new();
    for g in &input.calendar.games {
        games.push(forecast::TeamForecastGameInput {
            game_id: g.game_id,
            date: g.date,
            home_team: g.home_team.clone(),
            away_team: g.away_team.clone(),
            home_score: None,
            away_score: None,
            final_result: false,
            last_period: None,
        });
        model_games.push(season::SuppliedGameProbability {
            game_id: g.game_id,
            home: g.home_team.clone(),
            away: g.away_team.clone(),
            probabilities: probabilities[&(g.home_team.clone(), g.away_team.clone())].clone(),
        });
    }
    let f = forecast::build_team_game_forecast(
        input.calendar.season,
        games,
        known
            .iter()
            .map(|team| forecast::TeamForecastStrengthInput {
                team: team.clone(),
                strength: 50.0,
            })
            .collect(),
        forecast::TeamForecastParameters::default(),
        Some(input.calendar.schedule_games),
        None,
    )?;
    let model = season::SeasonProbabilityModel {
        schema: "season_probability_model.v1".into(),
        model_id: "rinkcraft.task-matchup.0.1".into(),
        source_sha256: hash(&(teams, &input.rules, &input.character_rules))?,
        games: model_games,
        playoffs,
    };
    let audit = serde_json::json!({"schema":"rinkcraft.task-matchup-audit.0.1","neutral_window_goal_probability":neutral,
        "terminal_stage_order":["outlet_disruption","creation_denial","reception_disruption","release_disruption","goalie_save_contact"],
        "task_order":["Puck Play vs forechecker Defending","creator Passing vs interceptor Reading","receiver Handling vs interceptor Defending","receiver Shooting vs gap Defending","goalie Tracking vs receiver Shooting"],
        "pair_audits":pair_audits,"limits":["Authored independent fresh d20 contests and independent unit-exposure mixtures, not empirical NHL probabilities",
            "Task ties remain unresolved; disruptions/save contact do not imply possession or official shot/block statistics",
            "Only first-goal-window model; no rebound recycle, traffic, lateral readiness, clock, fatigue, PP/PK or individual scoring ledger",
            "Goal means anchored to authored neutral rate and capped explicitly; Poisson goals are a game-model bridge, not measured NHL xG",
            "Skating and goalie Positioning/Movement/Rebounds/Recovery are unused in this fixed passing/clear-view route",
            "Four lines, three pairs and goalies are declared joint units; no fixed NHL line-number/coach assignment inferred",
            "Static task probabilities govern regular season and playoffs; old form/race and strength adjustments disabled only for this opt-in model"]});
    Ok((f, model, audit))
}
pub fn run(input: &Request) -> Result<serde_json::Value, String> {
    let (f, reference, ref_audit) = project(input, &input.reference)?;
    let (_, proposed, proposed_audit) = project(input, &input.proposed)?;
    let baseline =
        season::simulate_team_season_forecast_with_probabilities(&f, input.config, &reference)?;
    let changed = if hash(&reference)? == hash(&proposed)? {
        baseline.clone()
    } else {
        season::simulate_team_season_forecast_with_probabilities(&f, input.config, &proposed)?
    };
    let impacts = season::compare_team_season_forecast_scenarios(&baseline, &changed)?;
    Ok(
        serde_json::json!({"schema":"rinkcraft.task-season-result.0.1","kind":KIND,"input_sha256":hash(input)?,"rules_sha256":hash(&input.rules)?,
        "reference_model":reference,"proposed_model":proposed,"reference_audit":ref_audit,"proposed_audit":proposed_audit,"baseline":baseline,"proposed":changed,"impacts":impacts}),
    )
}

/// Exact regular-season expectations for this fixed-probability native model.
/// Playoff qualification still requires the season simulation.
pub fn inspect(input: &Request) -> Result<serde_json::Value, String> {
    let (_, reference, ref_audit) = project(input, &input.reference)?;
    let (_, proposed, proposed_audit) = project(input, &input.proposed)?;
    let summarize = |model: &season::SeasonProbabilityModel, audit: &serde_json::Value| {
        let pairs: BTreeMap<_, _> = audit["pair_audits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    (p["home"].as_str().unwrap(), p["away"].as_str().unwrap()),
                    p,
                )
            })
            .collect();
        let mut totals = BTreeMap::<String, [f64; 6]>::new();
        for row in &model.games {
            let p = &row.probabilities;
            let pair = pairs[&(row.home.as_str(), row.away.as_str())];
            let home_points =
                2.0 * p.home_regulation_win + p.overtime * (1.0 + p.home_overtime_win);
            let away_points =
                2.0 * p.away_regulation_win + p.overtime * (2.0 - p.home_overtime_win);
            let hm = pair["home_model_goals_mean"].as_f64().unwrap();
            let am = pair["away_model_goals_mean"].as_f64().unwrap();
            for (team, points, win, gf, ga) in [
                (
                    &row.home,
                    home_points,
                    p.home_regulation_win + p.overtime * p.home_overtime_win,
                    hm,
                    am,
                ),
                (
                    &row.away,
                    away_points,
                    p.away_regulation_win + p.overtime * (1.0 - p.home_overtime_win),
                    am,
                    hm,
                ),
            ] {
                let t = totals.entry(team.clone()).or_default();
                t[0] += 1.0;
                t[1] += points;
                t[2] += win;
                t[3] += gf;
                t[4] += ga;
                t[5] += p.overtime;
            }
        }
        totals
    };
    let a = summarize(&reference, &ref_audit);
    let b = summarize(&proposed, &proposed_audit);
    let teams:Vec<_>=a.iter().map(|(team,x)|{let y=b[team];serde_json::json!({"team":team,"games":x[0] as u64,
        "reference_expected_points":x[1],"proposed_expected_points":y[1],"expected_points_delta":y[1]-x[1],
        "reference_expected_wins":x[2],"proposed_expected_wins":y[2],"reference_model_goals_for":x[3],"proposed_model_goals_for":y[3],
        "reference_model_goals_against":x[4],"proposed_model_goals_against":y[4],"reference_expected_overtime_games":x[5],"proposed_expected_overtime_games":y[5]})}).collect();
    Ok(
        serde_json::json!({"schema":"rinkcraft.task-season-inspection.0.1","kind":KIND,"input_sha256":hash(input)?,"rules_sha256":hash(&input.rules)?,"seed":null,"trials":null,
        "scope":"exact regular-season expectations under fixed task probabilities; no playoff qualification probabilities",
        "teams":teams,"reference_audit":ref_audit,"proposed_audit":proposed_audit,
        "reference_model_sha256":hash(&reference)?,"proposed_model_sha256":hash(&proposed)?,
        "limits":["Fixed native mode has no dynamic form/race or adaptive feedback; expectation is a sum of supplied game probabilities, not Monte Carlo",
            "Model-goal totals are authored Poisson rate sums, not measured NHL xG or a sampled player scoring ledger",
            "League expected standings points can change when task-derived overtime probability changes",
            "No inference of balance, NHL ability, coaching intent or causal chemistry"]}),
    )
}
pub fn apply_study_case(base: &Request, case: &StudyCase) -> Result<Request, String> {
    if case.id.is_empty()
        || case.id.len() > 80
        || !case
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || case.label.trim().is_empty()
    {
        return Err("Invalid task study case ID/label".into());
    }
    let mut r = base.clone();
    let t = r
        .proposed
        .iter_mut()
        .find(|t| t.team == case.team)
        .ok_or("Unknown study team")?;
    let mut seen = BTreeSet::new();
    let rj = serde_json::to_string(&r.character_rules).map_err(|e| e.to_string())?;
    for e in &case.edits {
        if e.capability >= 6
            || !(1..=20).contains(&e.score)
            || !seen.insert((&e.member_id, e.capability))
        {
            return Err("Invalid/duplicate task score edit".into());
        }
        let m = t
            .members
            .iter_mut()
            .find(|m| m.id == e.member_id)
            .ok_or("Unknown task score member")?;
        game::import_from_json(
            &rj,
            &serde_json::to_string(&m.character).map_err(|e| e.to_string())?,
        )?;
        let mut scores: [u8; 6] = m
            .character
            .capabilities
            .iter()
            .map(|c| c.score)
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| "Need six task capabilities")?;
        scores[e.capability] = e.score;
        m.character = game::build(
            &r.character_rules,
            game::BuildRequest {
                template_id: m.character.template_id.clone(),
                name: m.character.name.clone(),
                position: m.character.position.clone(),
                scores: game::ScoreMethod::Manual { scores },
                inspiration: m.character.inspiration.clone(),
            },
        )?;
    }
    let mut swaps = BTreeSet::new();
    for id in &case.swap_creator_receiver {
        if !swaps.insert(id) {
            return Err("Duplicate task swap".into());
        }
        let u = t
            .units
            .iter_mut()
            .find(|u| &u.id == id)
            .ok_or("Unknown swapped unit")?;
        std::mem::swap(&mut u.creator, &mut u.receiver);
    }
    let reference = r
        .reference
        .iter_mut()
        .find(|t| t.team == case.team)
        .ok_or("Unknown reference study team")?;
    let mut weights = BTreeSet::new();
    for e in &case.unit_weights {
        if !weights.insert(&e.unit_id) {
            return Err("Duplicate unit weight edit".into());
        }
        t.units
            .iter_mut()
            .find(|u| u.id == e.unit_id)
            .ok_or("Unknown proposed weight unit")?
            .weight = e.weight;
        reference
            .units
            .iter_mut()
            .find(|u| u.id == e.unit_id)
            .ok_or("Unknown reference weight unit")?
            .weight = e.weight;
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Request {
        let rules = game::GameRules {
            schema: game::RULES_SCHEMA.into(),
            version: "test".into(),
            title: "Task test".into(),
            standard_array: [17, 16, 15, 13, 11, 9],
            templates: [
                ("skater", game::Branch::Skater),
                ("goalie", game::Branch::Goalie),
            ]
            .into_iter()
            .map(|(id, branch)| game::GameTemplate {
                id: id.into(),
                branch,
                class_id: id.into(),
                class_name: id.into(),
                specialty: "Test".into(),
                description: "Authored test".into(),
                priority: [0, 1, 2, 3, 4, 5],
                task_skills: vec![],
                methods: vec![],
                tendencies: vec![],
                playbook_options: vec![],
            })
            .collect(),
        };
        let names = [
            "BOS", "BUF", "DET", "FLA", "MTL", "OTT", "TBL", "TOR", "CAR", "CBJ", "NJD", "NYI",
            "NYR", "PHI", "PIT", "WSH", "CHI", "COL", "DAL", "MIN", "NSH", "STL", "UTA", "WPG",
            "ANA", "CGY", "EDM", "LAK", "SEA", "SJS", "VAN", "VGK",
        ];
        let mut teams = Vec::new();
        for name in names {
            let members = (0..6)
                .map(|i| Member {
                    id: format!("{name}:{i}"),
                    character: game::build(
                        &rules,
                        game::BuildRequest {
                            template_id: if i == 5 { "goalie" } else { "skater" }.into(),
                            name: format!("Test {name} {i}"),
                            position: if i == 5 {
                                "G"
                            } else if i >= 3 {
                                "D"
                            } else {
                                "F"
                            }
                            .into(),
                            scores: game::ScoreMethod::Manual { scores: [10; 6] },
                            inspiration: None,
                        },
                    )
                    .unwrap(),
                })
                .collect();
            let id = |i| format!("{name}:{i}");
            teams.push(Team {
                team: name.into(),
                members,
                units: vec![Unit {
                    id: "test".into(),
                    weight: 1.0,
                    forwards: [id(0), id(1), id(2)],
                    defense: [id(3), id(4)],
                    goalie: id(5),
                    creator: id(0),
                    receiver: id(1),
                    forechecker: id(2),
                    interceptor: id(3),
                    gap_defender: id(4),
                }],
            });
        }
        let games = (0..8)
            .flat_map(|round| {
                names.chunks_exact(2).enumerate().map(move |(i, p)| {
                    forecast::TeamForecastGameInput {
                        game_id: (round * 16 + i) as u64,
                        date: chrono::NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()
                            + chrono::Duration::days(round as i64),
                        away_team: p[round % 2].into(),
                        home_team: p[1 - round % 2].into(),
                        away_score: None,
                        home_score: None,
                        final_result: false,
                        last_period: None,
                    }
                })
            })
            .collect();
        let calendar = forecast::build_team_game_forecast(
            20262027,
            games,
            names
                .iter()
                .map(|team| forecast::TeamForecastStrengthInput {
                    team: (*team).into(),
                    strength: 50.0,
                })
                .collect(),
            forecast::TeamForecastParameters::default(),
            Some(128),
            Some(8),
        )
        .unwrap();
        Request {
            schema: SCHEMA.into(),
            kind: KIND.into(),
            character_rules: rules,
            rules: Rules {
                schema: RULES_SCHEMA.into(),
                version: "test".into(),
                kind: KIND.into(),
                score_rule: ScoreRule::HalfOffset,
                neutral_goals_per_team: 3.0,
                minimum_goals_mean: 0.05,
                maximum_goals_mean: 12.0,
                home_goals_multiplier: 1.05,
            },
            calendar,
            reference: teams.clone(),
            proposed: teams,
            config: season::TeamSeasonSimulationConfig { trials: 8, seed: 7 },
        }
    }
    #[test]
    fn route_retains_unresolved_mass_and_disjoint_terminal_outcomes() {
        let t = table(ScoreRule::HalfOffset).unwrap();
        let (s, tied, goal, flight) = route([10; 9], [10; 9], &t);
        assert!((goal - 0.475f64.powi(5)).abs() < 1e-14);
        assert!((s.iter().sum::<f64>() + tied + goal - 1.0).abs() < 1e-12);
        assert!(tied > 0.0);
        assert!(flight > goal);
    }
    #[test]
    fn role_bindings_distinguish_same_roster_and_goalie_changes_have_specific_stages() {
        let r = fixture();
        let mut team = r.proposed[0].clone();
        for (i, scores) in [(0, [10, 10, 16, 8, 10, 10]), (1, [10, 10, 8, 16, 10, 10])] {
            let c = &team.members[i].character;
            team.members[i].character = game::build(
                &r.character_rules,
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
        let a = patterns(&team, &r.character_rules, &mut BTreeSet::new()).unwrap();
        let t = table(ScoreRule::HalfOffset).unwrap();
        let neutral = Pattern {
            scores: [10; 9],
            weight: 1.0,
        };
        let good = attack(&a, &[neutral.clone()], &t);
        let u = &mut team.units[0];
        std::mem::swap(&mut u.creator, &mut u.receiver);
        let b = patterns(&team, &r.character_rules, &mut BTreeSet::new()).unwrap();
        let bad = attack(&b, &[neutral.clone()], &t);
        assert!(
            good["bounded_window_goal_probability"].as_f64().unwrap()
                > bad["bounded_window_goal_probability"].as_f64().unwrap()
        );
        let before = route([10; 9], [10; 9], &t);
        let mut goalie = [10; 9];
        goalie[8] = 16;
        let after = route([10; 9], goalie, &t);
        assert_eq!(&before.0[..4], &after.0[..4]);
        assert!(after.2 < before.2);
        let mut outlet = [10; 9];
        outlet[0] = 16;
        assert_eq!(route([10; 9], outlet, &t).2, before.2);
        assert!(route(outlet, [10; 9], &t).2 > before.2);
    }
    #[test]
    fn poisson_probabilities_are_normalized_and_symmetric_without_home_boost() {
        for m in [0.05, 3.0, 12.0] {
            let p = outcome(m, m);
            assert!(
                (p.home_regulation_win + p.away_regulation_win + p.overtime - 1.0).abs() < 1e-12
            );
            assert!((p.home_regulation_win + p.overtime * p.home_overtime_win - 0.5).abs() < 1e-12);
        }
        let p = outcome(6.0, 2.0);
        assert!(p.home_regulation_win + p.overtime * p.home_overtime_win > 0.5);
    }
    #[test]
    fn passing_and_shooting_value_change_with_opponent_task_weakness() {
        let t = table(ScoreRule::HalfOffset).unwrap();
        let mut passer = [10; 9];
        passer[1] = 20;
        let mut shooter = [10; 9];
        shooter[3] = 20;
        assert!(route(shooter, [10; 9], &t).2 > route(passer, [10; 9], &t).2);
        let mut opponent = [10; 9];
        opponent[5] = 20;
        opponent[7] = 1;
        opponent[8] = 1;
        assert!(route(passer, opponent, &t).2 > route(shooter, opponent, &t).2);
    }
    #[test]
    fn bad_units_forged_characters_and_missing_teams_are_rejected() {
        let mut r = fixture();
        r.proposed[0].units[0].receiver = r.proposed[0].units[0].creator.clone();
        assert!(project(&r, &r.proposed).is_err());
        let mut r = fixture();
        r.proposed[0].members[0].character.capabilities[0].score = 20;
        assert!(project(&r, &r.proposed).is_err());
        let mut r = fixture();
        r.proposed.pop();
        assert!(project(&r, &r.proposed).is_err());
        let mut r = fixture();
        r.proposed[0].units[0].weight = 0.5;
        assert!(project(&r, &r.proposed).is_err());
    }
    #[test]
    fn all_games_and_ordered_playoff_matchups_are_projected_and_neutral_replays() {
        let r = fixture();
        let (_, m, a) = project(&r, &r.proposed).unwrap();
        assert_eq!(m.games.len(), 128);
        assert_eq!(m.playoffs.len(), 992);
        assert_eq!(a["pair_audits"].as_array().unwrap().len(), 992);
        let first = run(&r).unwrap();
        assert_eq!(first["baseline"], first["proposed"]);
        assert_eq!(first, run(&r).unwrap());
    }
    #[test]
    fn supplied_probabilities_drive_regular_season_and_playoffs_without_scalar_clamp() {
        let r = fixture();
        let (f, mut m, _) = project(&r, &r.proposed).unwrap();
        for g in &mut m.games {
            if g.home == "NYR" {
                g.probabilities = season::GameOutcomeProbabilities {
                    home_regulation_win: 1.0,
                    away_regulation_win: 0.0,
                    overtime: 0.0,
                    home_overtime_win: 1.0,
                };
            } else if g.away == "NYR" {
                g.probabilities = season::GameOutcomeProbabilities {
                    home_regulation_win: 0.0,
                    away_regulation_win: 1.0,
                    overtime: 0.0,
                    home_overtime_win: 0.0,
                };
            }
        }
        for p in &mut m.playoffs {
            if p.home == "NYR" {
                p.home_win_probability = 1.0;
            } else if p.away == "NYR" {
                p.home_win_probability = 0.0;
            }
        }
        let s = season::simulate_team_season_forecast_with_probabilities(&f, r.config, &m).unwrap();
        let n = s.teams.iter().find(|t| t.team == "NYR").unwrap();
        assert_eq!(n.average_wins, 8.0);
        assert_eq!(n.stanley_cup_probability, 1.0);
        m.playoffs.pop();
        assert!(
            season::simulate_team_season_forecast_with_probabilities(&f, r.config, &m).is_err()
        );
    }
    #[test]
    fn inspection_has_exact_regular_ledger_and_no_sampling_dependence() {
        let mut r = fixture();
        let a = inspect(&r).unwrap();
        let teams = a["teams"].as_array().unwrap();
        let wins: f64 = teams
            .iter()
            .map(|t| t["proposed_expected_wins"].as_f64().unwrap())
            .sum();
        let points: f64 = teams
            .iter()
            .map(|t| t["proposed_expected_points"].as_f64().unwrap())
            .sum();
        let overtime: f64 = teams
            .iter()
            .map(|t| t["proposed_expected_overtime_games"].as_f64().unwrap())
            .sum();
        assert!((wins - 128.0).abs() < 1e-9);
        assert!((points - 256.0 - overtime / 2.0).abs() < 1e-9);
        assert!(teams.iter().all(|t| t["expected_points_delta"] == 0.0));
        r.config.seed = 999;
        r.config.trials = 1;
        let b = inspect(&r).unwrap();
        assert_eq!(a["teams"], b["teams"]);
    }
    #[test]
    fn complementary_gate_effects_compound_without_an_extra_bonus() {
        let t = table(ScoreRule::HalfOffset).unwrap();
        let base = [10; 9];
        let mut a = base;
        a[1] = 16;
        let mut b = base;
        b[3] = 16;
        let mut both = a;
        both[3] = 16;
        assert!(
            route(both, base, &t).2 - route(a, base, &t).2 - route(b, base, &t).2
                + route(base, base, &t).2
                > 0.0
        );
        let r = fixture();
        let case = StudyCase {
            id: "single".into(),
            label: "Single passer".into(),
            team: "NYR".into(),
            edits: vec![super::super::rinkcraft_season::ScoreEdit {
                member_id: "NYR:0".into(),
                capability: 2,
                score: 16,
            }],
            swap_creator_receiver: vec![],
            unit_weights: vec![],
        };
        let q = apply_study_case(&r, &case).unwrap();
        assert_eq!(
            q.reference
                .iter()
                .find(|t| t.team == "NYR")
                .unwrap()
                .members[0]
                .character
                .capabilities[2]
                .score,
            10
        );
        assert_eq!(
            q.proposed.iter().find(|t| t.team == "NYR").unwrap().members[0]
                .character
                .capabilities[2]
                .score,
            16
        );
        let mut forged = r.clone();
        forged
            .proposed
            .iter_mut()
            .find(|t| t.team == "NYR")
            .unwrap()
            .members[0]
            .character
            .capabilities[3]
            .score = 20;
        assert!(apply_study_case(&forged, &case).is_err());
    }
}
