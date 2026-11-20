//! Explicit authored 1–20 profile differences adapted to the existing season engine.
//! This is a game rule, never an empirical NHL ability or chemistry estimate.
use super::{
    rinkcraft_game as game, team_game_forecast::TeamGameForecastView,
    team_season_forecast as season,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const SCHEMA: &str = "rinkcraft.profile-season-request.0.1";
pub const RULES_SCHEMA: &str = "rinkcraft.profile-season-rules.0.1";
const KIND: &str = "authored_game_scenario_not_nhl_estimate";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRules {
    pub schema: String,
    pub version: String,
    pub kind: String,
    /// Ordered by rinkcraft_game::SKATER and rinkcraft_game::GOALIE respectively.
    pub forward_weights: [f64; 6],
    pub defense_weights: [f64; 6],
    pub goalie_weights: [f64; 6],
    /// Forward / defense / goalie channel weights, not player-minute shares.
    pub channel_weights: [f64; 3],
    /// Max roster-reference difference in the engine's 0–100 strength units.
    pub strength_span: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub id: String,
    pub character: game::GameCharacter,
    /// Planned fraction within the forward, defense or goalie bucket. Each sums to one.
    pub exposure_share: f64,
    /// Optional authored task emphasis, separate from the character's class.
    #[serde(default)]
    pub assignment: Option<Assignment>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub label: String,
    pub capability_weights: [f64; 6],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScoreEdit {
    pub member_id: String,
    pub capability: usize,
    pub score: u8,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExposureTransfer {
    pub from: String,
    pub to: String,
    pub share: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssignmentEdit {
    pub member_id: String,
    pub assignment: Assignment,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeCase {
    pub id: String,
    pub label: String,
    pub team: String,
    pub edits: Vec<ScoreEdit>,
    pub exposure_transfer: Option<ExposureTransfer>,
    pub assignment: Option<AssignmentEdit>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamChange {
    pub team: String,
    pub reference: Vec<Member>,
    pub proposed: Vec<Member>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema: String,
    pub kind: String,
    pub label: String,
    pub character_rules: game::GameRules,
    pub profile_rules: ProfileRules,
    pub forecast: TeamGameForecastView,
    pub teams: Vec<TeamChange>,
    pub effective_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub as_of_date: Option<NaiveDate>,
    pub config: season::TeamSeasonSimulationConfig,
}
fn fingerprint<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)
    ))
}
fn weights(values: &[f64]) -> bool {
    values
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0 && *v <= 1.0)
        && (values.iter().sum::<f64>() - 1.0).abs() < 1e-9
}
impl ProfileRules {
    fn validate(&self) -> Result<(), String> {
        if self.schema != RULES_SCHEMA
            || self.kind != KIND
            || self.version.trim().is_empty()
            || !weights(&self.forward_weights)
            || !weights(&self.defense_weights)
            || !weights(&self.goalie_weights)
            || !weights(&self.channel_weights)
            || self.channel_weights.iter().any(|w| *w <= 0.0)
            || !self.strength_span.is_finite()
            || self.strength_span <= 0.0
            || self.strength_span > 10.0
        {
            return Err(
                "Invalid authored profile-season rules, normalized weights or span (0,10]".into(),
            );
        }
        Ok(())
    }
}
fn bucket(c: &game::GameCharacter) -> usize {
    if c.branch == game::Branch::Goalie {
        2
    } else if ["D", "LD", "RD"].contains(&c.position.as_str()) {
        1
    } else {
        0
    }
}
fn roster(
    members: &[Member],
    rules: &game::GameRules,
    mapping: &ProfileRules,
    global: &mut BTreeSet<String>,
) -> Result<serde_json::Value, String> {
    if members.len() > 40 {
        return Err("Roster exceeds 40 profile members".into());
    }
    let rules_json = serde_json::to_string(rules).map_err(|e| e.to_string())?;
    let mut sums = [0.0; 3];
    let mut values = [0.0; 3];
    let mut counts = [0; 3];
    let mut rows = Vec::new();
    for m in members {
        if m.id.trim().is_empty()
            || m.id.len() > 100
            || !global.insert(m.id.clone())
            || !m.exposure_share.is_finite()
            || !(0.0..=1.0).contains(&m.exposure_share)
        {
            return Err("Duplicate/invalid player ID or exposure share".into());
        }
        // Rebuild from original rules/request: do not accept edited capability exports.
        game::import_from_json(
            &rules_json,
            &serde_json::to_string(&m.character).map_err(|e| e.to_string())?,
        )?;
        let b = bucket(&m.character);
        let defaults = [
            mapping.forward_weights,
            mapping.defense_weights,
            mapping.goalie_weights,
        ][b];
        let w = if let Some(a) = &m.assignment {
            if a.label.trim().is_empty() || a.label.len() > 100 || !weights(&a.capability_weights) {
                return Err("Invalid authored assignment capability weights".into());
            }
            a.capability_weights
        } else {
            defaults
        };
        counts[b] += 1;
        sums[b] += m.exposure_share;
        let mut ability = Vec::new();
        let mut score = 0.0;
        for (c, weight) in m.character.capabilities.iter().zip(w) {
            let term = (f64::from(c.score) - 10.5) / 9.5 * weight;
            score += term;
            ability.push(serde_json::json!({"id":c.id,"score":c.score,"weight":weight,"normalized_term":term}));
        }
        values[b] += score * m.exposure_share;
        rows.push(serde_json::json!({"id":m.id,"name":m.character.name,"bucket":(["forward","defense","goalie"][b]),
            "class_id":m.character.class_id,"assignment":m.assignment,"exposure_share":m.exposure_share,"weighted_profile":score,
            "channel_contribution":score*m.exposure_share*mapping.channel_weights[b],"abilities":ability}));
    }
    if counts[0] < 3
        || counts[1] < 2
        || counts[2] < 1
        || sums.iter().any(|s| (s - 1.0).abs() > 1e-9)
    {
        return Err(
            "Need >=3 F, >=2 D, >=1 G and exposure shares summing to one in each bucket".into(),
        );
    }
    let total: f64 = values
        .iter()
        .zip(mapping.channel_weights)
        .map(|(v, w)| v * w)
        .sum();
    Ok(serde_json::json!({"bucket_values":values,"weighted_profile":total,"players":rows}))
}
/// Produces an inspectable standard engine scenario; no baseline forecast is mutated.
pub fn adapt(input: &Request) -> Result<(season::TeamSeasonScenario, serde_json::Value), String> {
    if input.schema != SCHEMA
        || input.kind != KIND
        || input.label.trim().is_empty()
        || input.teams.is_empty()
        || input.teams.len() > 32
    {
        return Err("Invalid authored Rinkcraft season request".into());
    }
    input.character_rules.validate()?;
    input.profile_rules.validate()?;
    if input.effective_date < input.forecast.schedule_start
        || input.effective_date > input.forecast.schedule_end
        || input
            .end_date
            .is_some_and(|d| d < input.effective_date || d > input.forecast.schedule_end)
        || input.as_of_date.is_some_and(|d| input.effective_date <= d)
    {
        return Err("Profile effect must begin within schedule and after any as-of cutoff; invalid end date".into());
    }
    let teams: BTreeSet<_> = input
        .forecast
        .games
        .iter()
        .flat_map(|g| [g.home_team.as_str(), g.away_team.as_str()])
        .collect();
    let mut chosen = BTreeSet::new();
    let mut ref_ids = BTreeSet::new();
    let mut proposed_ids = BTreeSet::new();
    let mut audit = Vec::new();
    let mut events = Vec::new();
    for t in &input.teams {
        if !teams.contains(t.team.as_str()) || !chosen.insert(t.team.clone()) {
            return Err("Unknown/duplicate scenario team".into());
        }
        let reference = roster(
            &t.reference,
            &input.character_rules,
            &input.profile_rules,
            &mut ref_ids,
        )?;
        let proposed = roster(
            &t.proposed,
            &input.character_rules,
            &input.profile_rules,
            &mut proposed_ids,
        )?;
        // Range is [-2,2]; half-span ensures a complete min->max change stays bounded.
        let delta = (proposed["weighted_profile"].as_f64().unwrap()
            - reference["weighted_profile"].as_f64().unwrap())
            * input.profile_rules.strength_span
            / 2.0;
        audit.push(serde_json::json!({"team":t.team,"reference":reference,"proposed":proposed,"strength_delta":delta}));
        // Neutral identity must use the unchanged baseline simulation path exactly.
        if delta.abs() > 1e-12 {
            events.push(season::TeamSeasonScenarioEvent {
                id: format!("rinkcraft-profile:{}", t.team),
                kind: season::TeamSeasonScenarioEventKind::Custom,
                team: t.team.clone(),
                player: None,
                effective_date: input.effective_date,
                end_date: input.end_date,
                strength_delta: delta,
                occurrence_probability: 1.0,
                correlation_key: None,
                label: format!("{}: authored profile/reference difference", t.team),
            });
        }
    }
    Ok((
        season::TeamSeasonScenario {
            name: input.label.clone(),
            trade_deadline: None,
            events,
            adaptive_lineup_policies: Vec::new(),
            opening_roster_policies: Vec::new(),
        },
        serde_json::json!({"schema":"rinkcraft.profile-season-audit.0.1","kind":KIND,"teams":audit,
            "limits":["Authored 1–20 game profiles and explicit mapping, not NHL ability estimates or calibrated effect sizes",
                "Reference roster is an explicit baseline assumption; only proposed-minus-reference effects enter the existing engine",
                "Class, methods, inspiration and shared deployment confer no independent bonus",
                "Exposure shares are within F/D/G buckets; goalie weight is an authored channel, not skater minutes",
                "Aggregate strength adapter, not shift-level tactics, PP/PK stations, clock, fatigue or player point accounting",
                "Unspecified teams retain the input forecast; they are not assigned zero-rated profiles",
                "External forecast effects remain intact; caller must avoid duplicating a roster change already in its baseline"]}),
    ))
}
pub fn run(input: &Request) -> Result<serde_json::Value, String> {
    let (scenario, audit) = adapt(input)?;
    let simulate = |s| match input.as_of_date {
        Some(date) => season::simulate_team_season_forecast_as_of_with_scenario(
            &input.forecast,
            input.config,
            s,
            date,
        ),
        None => {
            season::simulate_team_season_forecast_with_scenario(&input.forecast, input.config, s)
        }
    };
    let baseline = simulate(None)?;
    let proposed = if scenario.events.is_empty() {
        baseline.clone()
    } else {
        simulate(Some(scenario.clone()))?
    };
    let impact = season::compare_team_season_forecast_scenarios(&baseline, &proposed)?;
    Ok(
        serde_json::json!({"schema":"rinkcraft.profile-season-result.0.1","kind":KIND,"input_sha256":fingerprint(input)?,
        "character_rules_sha256":fingerprint(&input.character_rules)?,"profile_rules_sha256":fingerprint(&input.profile_rules)?,
        "forecast_sha256":fingerprint(&input.forecast)?,"audit":audit,"scenario":scenario,"baseline":baseline,"proposed":proposed,"impacts":impact}),
    )
}

/// Noiseless regular-season projection with dynamic form/race edge frozen at zero.
/// Uses the sampler's actual probability kernel, not a duplicate approximation.
pub fn probe(input: &Request) -> Result<serde_json::Value, String> {
    let (scenario, audit) = adapt(input)?;
    let mut totals = std::collections::BTreeMap::<String, [f64; 4]>::new();
    let mut games = Vec::new();
    let active = |team: &str, date: NaiveDate| -> f64 {
        scenario
            .events
            .iter()
            .filter(|e| {
                e.team == team
                    && e.effective_date <= date
                    && e.end_date.is_none_or(|end| date <= end)
            })
            .map(|e| e.strength_delta)
            .sum()
    };
    for g in &input.forecast.games {
        if input.as_of_date.is_some_and(|date| g.date <= date) {
            continue;
        }
        let ot = g.overtime_probability;
        if !ot.is_finite()
            || !(0.0..=1.0).contains(&ot)
            || !g.home_overall_win_probability.is_finite()
            || !(0.0..=1.0).contains(&g.home_overall_win_probability)
            || !input.forecast.parameters.strength_edge_scale.is_finite()
            || input.forecast.parameters.strength_edge_scale < 0.0
        {
            return Err("Invalid probability probe inputs".into());
        }
        let before = season::game_probability_components(
            g,
            0.0,
            0.0,
            input.forecast.parameters.strength_edge_scale,
            0.0,
        );
        let after = season::game_probability_components(
            g,
            active(&g.home_team, g.date),
            active(&g.away_team, g.date),
            input.forecast.parameters.strength_edge_scale,
            0.0,
        );
        let points = |p: [f64; 3]| {
            [
                2.0 * p[0] + ot * (1.0 + p[2]),
                2.0 * p[1] + ot * (2.0 - p[2]),
            ]
        };
        let a = points(before);
        let b = points(after);
        for (index, team) in [&g.home_team, &g.away_team].into_iter().enumerate() {
            let row = totals.entry(team.clone()).or_default();
            row[0] += a[index];
            row[1] += b[index];
            row[2] += 1.0;
            if (a[index] - b[index]).abs() > 1e-14 {
                row[3] += 1.0;
            }
        }
        games.push(serde_json::json!({"game_id":g.game_id,"date":g.date,"home":g.home_team,"away":g.away_team,
            "overtime_probability":ot,"baseline_expected_points":a,"proposed_expected_points":b,
            "baseline_components":before,"proposed_components":after}));
    }
    let rows:Vec<_>=totals.into_iter().map(|(team,v)|serde_json::json!({"team":team,"baseline_expected_points":v[0],"proposed_expected_points":v[1],
        "expected_points_delta":v[1]-v[0],"games":v[2] as u64,"games_with_probability_change":v[3] as u64})).collect();
    Ok(
        serde_json::json!({"schema":"rinkcraft.profile-probability-probe.0.1","kind":KIND,"input_sha256":fingerprint(input)?,
        "scope":"frozen-state regular-season projection; dynamic edge zero; no playoffs or endogenous feedback",
        "component_order":["home_regulation_win","away_regulation_win","home_overtime_win_given_overtime"],"points_order":["home","away"],
        "strength_edge_scale":input.forecast.parameters.strength_edge_scale,"audit":audit,"teams":rows,"games":games,
        "limits":["Exact for the declared per-game frozen-state probability kernel, not the full dynamic season model's expectation",
            "Form, race, adaptive choices, injuries and playoff paths are not integrated; no claim of empirical NHL effect sizes",
            "As-of past dates excluded; event dates applied; overtime probability unchanged so expected point deltas sum to zero",
            "No random trials or uncertainty estimate; probe is deterministic authored-model arithmetic"]}),
    )
}

pub fn apply_probe_case(base: &Request, case: &ProbeCase) -> Result<Request, String> {
    adapt(base)?;
    if case.id.is_empty()
        || case.id.len() > 80
        || !case
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || case.label.trim().is_empty()
    {
        return Err("Invalid probe case ID/label".into());
    }
    let mut r = base.clone();
    r.label = format!("Sensitivity: {}", case.label);
    let t = r
        .teams
        .iter_mut()
        .find(|t| t.team == case.team)
        .ok_or("Unknown probe team")?;
    let mut seen = BTreeSet::new();
    for edit in &case.edits {
        if edit.capability >= 6
            || !(1..=20).contains(&edit.score)
            || !seen.insert((&edit.member_id, edit.capability))
        {
            return Err("Invalid or duplicate capability edit".into());
        }
        let m = t
            .proposed
            .iter_mut()
            .find(|m| m.id == edit.member_id)
            .ok_or("Unknown edited member")?;
        let mut scores: [u8; 6] = m
            .character
            .capabilities
            .iter()
            .map(|c| c.score)
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| "Need six capabilities")?;
        scores[edit.capability] = edit.score;
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
    if let Some(move_share) = &case.exposure_transfer {
        if move_share.from == move_share.to
            || !move_share.share.is_finite()
            || move_share.share <= 0.0
        {
            return Err("Invalid exposure transfer".into());
        }
        for roster in [&mut t.reference, &mut t.proposed] {
            let from = roster
                .iter()
                .position(|m| m.id == move_share.from)
                .ok_or("Missing exposure donor")?;
            let to = roster
                .iter()
                .position(|m| m.id == move_share.to)
                .ok_or("Missing exposure receiver")?;
            if bucket(&roster[from].character) != bucket(&roster[to].character) {
                return Err("Cannot transfer shares between F/D/G buckets".into());
            }
            roster[from].exposure_share -= move_share.share;
            roster[to].exposure_share += move_share.share;
        }
    }
    if let Some(edit) = &case.assignment {
        for roster in [&mut t.reference, &mut t.proposed] {
            roster
                .iter_mut()
                .find(|m| m.id == edit.member_id)
                .ok_or("Unknown task-emphasis member")?
                .assignment = Some(edit.assignment.clone());
        }
    }
    adapt(&r)?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::super::team_game_forecast::{
        build_team_game_forecast, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput,
    };
    use super::*;
    fn character_rules() -> game::GameRules {
        game::GameRules {
            schema: game::RULES_SCHEMA.into(),
            version: "test".into(),
            title: "Test authored characters".into(),
            standard_array: [17, 16, 15, 13, 11, 9],
            templates: [
                ("skater", game::Branch::Skater),
                ("other-skater", game::Branch::Skater),
                ("goalie", game::Branch::Goalie),
            ]
            .into_iter()
            .map(|(id, branch)| game::GameTemplate {
                id: id.into(),
                branch,
                class_id: id.into(),
                class_name: id.into(),
                specialty: "Test".into(),
                description: "Test build".into(),
                priority: [0, 1, 2, 3, 4, 5],
                task_skills: vec![],
                methods: vec![],
                tendencies: vec![],
                playbook_options: vec![],
            })
            .collect(),
        }
    }
    fn member(id: usize, rules: &game::GameRules) -> Member {
        let (position, template, share) = if id < 3 {
            ("F", "skater", 1.0 / 3.0)
        } else if id < 5 {
            ("D", "skater", 0.5)
        } else {
            ("G", "goalie", 1.0)
        };
        Member {
            id: format!("p{id}"),
            assignment: None,
            exposure_share: share,
            character: game::build(
                rules,
                game::BuildRequest {
                    template_id: template.into(),
                    name: format!("Test {id}"),
                    position: position.into(),
                    scores: game::ScoreMethod::Manual { scores: [10; 6] },
                    inspiration: None,
                },
            )
            .unwrap(),
        }
    }
    fn fixture() -> Request {
        let names = [
            "BOS", "BUF", "DET", "FLA", "MTL", "OTT", "TBL", "TOR", "CAR", "CBJ", "NJD", "NYI",
            "NYR", "PHI", "PIT", "WSH", "CHI", "COL", "DAL", "MIN", "NSH", "STL", "UTA", "WPG",
            "ANA", "CGY", "EDM", "LAK", "SEA", "SJS", "VAN", "VGK",
        ];
        let games = (0..8)
            .flat_map(|round| {
                names
                    .chunks_exact(2)
                    .enumerate()
                    .map(move |(i, p)| TeamForecastGameInput {
                        game_id: (round * 16 + i) as u64,
                        date: NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()
                            + chrono::Duration::days(round as i64),
                        away_team: p[round % 2].into(),
                        home_team: p[1 - round % 2].into(),
                        away_score: None,
                        home_score: None,
                        final_result: false,
                        last_period: None,
                    })
            })
            .collect();
        let forecast = build_team_game_forecast(
            20262027,
            games,
            names
                .iter()
                .map(|t| TeamForecastStrengthInput {
                    team: (*t).into(),
                    strength: 50.0,
                })
                .collect(),
            TeamForecastParameters::default(),
            Some(128),
            Some(8),
        )
        .unwrap();
        let rules = character_rules();
        let roster: Vec<_> = (0..6).map(|id| member(id, &rules)).collect();
        Request {
            schema: SCHEMA.into(),
            kind: KIND.into(),
            label: "Test authored profile".into(),
            character_rules: rules,
            profile_rules: ProfileRules {
                schema: RULES_SCHEMA.into(),
                version: "test".into(),
                kind: KIND.into(),
                forward_weights: [1.0 / 6.0; 6],
                defense_weights: [1.0 / 6.0; 6],
                goalie_weights: [1.0 / 6.0; 6],
                channel_weights: [0.45, 0.30, 0.25],
                strength_span: 10.0,
            },
            effective_date: forecast.schedule_start,
            end_date: None,
            as_of_date: None,
            forecast,
            teams: vec![TeamChange {
                team: "NYR".into(),
                reference: roster.clone(),
                proposed: roster,
            }],
            config: season::TeamSeasonSimulationConfig {
                trials: 16,
                seed: 7,
            },
        }
    }
    fn replace(input: &mut Request, index: usize, ability: usize, score: u8) {
        let old = &input.teams[0].proposed[index].character;
        let mut scores = [10; 6];
        scores[ability] = score;
        input.teams[0].proposed[index].character = game::build(
            &input.character_rules,
            game::BuildRequest {
                template_id: old.template_id.clone(),
                name: old.name.clone(),
                position: old.position.clone(),
                scores: game::ScoreMethod::Manual { scores },
                inspiration: None,
            },
        )
        .unwrap();
    }
    #[test]
    fn each_of_twelve_capabilities_has_inspectable_effect() {
        for index in [0, 3, 5] {
            for ability in 0..6 {
                let mut i = fixture();
                replace(&mut i, index, ability, 20);
                let (s, a) = adapt(&i).unwrap();
                assert!(s.events[0].strength_delta > 0.0);
                assert_eq!(
                    a["teams"][0]["proposed"]["players"][index]["abilities"][ability]["score"],
                    20
                );
            }
        }
    }
    #[test]
    fn role_weights_and_exposure_control_effect_without_class_bonus() {
        let mut i = fixture();
        i.profile_rules.forward_weights = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        replace(&mut i, 0, 0, 20);
        assert!(adapt(&i).unwrap().0.events.is_empty());
        replace(&mut i, 0, 3, 20);
        let delta = adapt(&i).unwrap().0.events[0].strength_delta;
        i.teams[0].proposed[0].exposure_share = 1.0 / 6.0;
        i.teams[0].proposed[1].exposure_share = 0.5;
        assert!((adapt(&i).unwrap().0.events[0].strength_delta - delta / 2.0).abs() < 1e-9);
    }
    #[test]
    fn task_emphasis_is_separate_from_class_and_scores() {
        let mut i = fixture();
        replace(&mut i, 0, 2, 20);
        let normal = adapt(&i).unwrap().0.events[0].strength_delta;
        i.teams[0].reference[0].assignment = Some(Assignment {
            label: "Distributor".into(),
            capability_weights: [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        });
        i.teams[0].proposed[0].assignment = i.teams[0].reference[0].assignment.clone();
        assert!(adapt(&i).unwrap().0.events[0].strength_delta > normal);
        let mut j = fixture();
        let old = &j.teams[0].proposed[0].character;
        j.teams[0].proposed[0].character = game::build(
            &j.character_rules,
            game::BuildRequest {
                template_id: "other-skater".into(),
                name: old.name.clone(),
                position: old.position.clone(),
                scores: game::ScoreMethod::Manual { scores: [10; 6] },
                inspiration: None,
            },
        )
        .unwrap();
        assert!(adapt(&j).unwrap().0.events.is_empty());
        j.teams[0].proposed[0].assignment = Some(Assignment {
            label: "Invalid".into(),
            capability_weights: [1.0; 6],
        });
        assert!(adapt(&j).is_err());
    }
    #[test]
    fn forged_profiles_duplicates_missing_goalies_and_bad_weights_rejected() {
        let mut i = fixture();
        i.teams[0].proposed[0].character.capabilities[0].score = 20;
        assert!(adapt(&i).is_err());
        let mut i = fixture();
        i.teams[0].proposed[1].id = i.teams[0].proposed[0].id.clone();
        assert!(adapt(&i).is_err());
        let mut i = fixture();
        i.teams[0].proposed.pop();
        assert!(adapt(&i).is_err());
        let mut i = fixture();
        i.profile_rules.channel_weights = [0.5, 0.5, 0.0];
        assert!(adapt(&i).is_err());
        let mut i = fixture();
        i.profile_rules.strength_span = f64::NAN;
        assert!(adapt(&i).is_err());
        let mut i = fixture();
        i.teams[0].proposed[0].exposure_share = 0.0;
        assert!(adapt(&i).is_err());
    }
    #[test]
    fn effect_window_and_reference_difference_are_explicit() {
        let mut i = fixture();
        i.as_of_date = Some(i.effective_date);
        assert!(adapt(&i).is_err());
        let mut i = fixture();
        i.end_date = Some(i.effective_date - chrono::Duration::days(1));
        assert!(adapt(&i).is_err());
        let mut i = fixture();
        replace(&mut i, 5, 0, 20);
        i.teams[0].reference = i.teams[0].proposed.clone();
        assert!(adapt(&i).unwrap().0.events.is_empty());
    }
    #[test]
    fn zero_difference_preserves_existing_engine_exactly_and_runs_replay() {
        let i = fixture();
        let r = run(&i).unwrap();
        assert_eq!(r["baseline"], r["proposed"]);
        let direct = season::simulate_team_season_forecast(&i.forecast, i.config).unwrap();
        assert_eq!(r["baseline"], serde_json::to_value(direct).unwrap());
        let mut changed = i.clone();
        replace(&mut changed, 5, 0, 20);
        let first = run(&changed).unwrap();
        assert_eq!(first, run(&changed).unwrap());
        assert!(
            first["audit"]["teams"][0]["strength_delta"]
                .as_f64()
                .unwrap()
                > 0.0
        );
        let direct = season::simulate_team_season_forecast_with_scenario(
            &changed.forecast,
            changed.config,
            Some(adapt(&changed).unwrap().0),
        )
        .unwrap();
        assert_eq!(first["proposed"], serde_json::to_value(direct).unwrap());
    }
    fn nyr_delta(p: &serde_json::Value) -> f64 {
        p["teams"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["team"] == "NYR")
            .unwrap()["expected_points_delta"]
            .as_f64()
            .unwrap()
    }
    #[test]
    fn frozen_probe_is_monotone_neutral_and_conserves_league_points() {
        let i = fixture();
        assert_eq!(nyr_delta(&probe(&i).unwrap()), 0.0);
        let mut old = f64::NEG_INFINITY;
        for score in [1, 5, 10, 15, 20] {
            let mut r = i.clone();
            replace(&mut r, 0, 3, score);
            let p = probe(&r).unwrap();
            let delta = nyr_delta(&p);
            assert!(delta > old);
            old = delta;
            let sum: f64 = p["teams"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t["expected_points_delta"].as_f64().unwrap())
                .sum();
            assert!(sum.abs() < 1e-10);
            for g in p["games"].as_array().unwrap() {
                let points: f64 = g["proposed_expected_points"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p.as_f64().unwrap())
                    .sum();
                assert!((points - 2.0 - g["overtime_probability"].as_f64().unwrap()).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn probe_respects_effect_dates_and_strength_scale() {
        let mut i = fixture();
        replace(&mut i, 0, 3, 20);
        let full = nyr_delta(&probe(&i).unwrap());
        assert!(full > 0.0);
        i.end_date = Some(i.effective_date);
        let short = nyr_delta(&probe(&i).unwrap());
        assert!(short >= 0.0 && short < full);
        i.forecast.parameters.strength_edge_scale = 0.0;
        assert_eq!(nyr_delta(&probe(&i).unwrap()), 0.0);
        i.forecast.parameters.strength_edge_scale = f64::NAN;
        assert!(probe(&i).is_err());
    }
    #[test]
    fn declared_case_edits_preserve_reference_and_reject_cross_bucket_transfers() {
        let i = fixture();
        let mut c = ProbeCase {
            id: "passing".into(),
            label: "Passing".into(),
            team: "NYR".into(),
            edits: vec![ScoreEdit {
                member_id: "p0".into(),
                capability: 2,
                score: 20,
            }],
            exposure_transfer: None,
            assignment: None,
        };
        let r = apply_probe_case(&i, &c).unwrap();
        assert_eq!(r.teams[0].reference[0].character.capabilities[2].score, 10);
        assert_eq!(r.teams[0].proposed[0].character.capabilities[2].score, 20);
        c.exposure_transfer = Some(ExposureTransfer {
            from: "p0".into(),
            to: "p5".into(),
            share: 0.1,
        });
        assert!(apply_probe_case(&i, &c).is_err());
        c.id = "../invalid".into();
        assert!(apply_probe_case(&i, &c).is_err());
    }
}
