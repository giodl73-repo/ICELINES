use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::model::Season;
use crate::name::normalize_name;
use crate::scheme::{
    compute_fantasy_score, compute_goalie_fantasy_score, GoalieScoreStats, Scheme, SkaterStats,
};
use crate::season_stats::SeasonType;
use crate::stats_repository::PlayerView;
use crate::view_model::context::{Completeness, SourceKind, SourceState, ViewContext, ViewWindow};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySimulationView {
    pub context: ViewContext,
    pub league: String,
    pub scoring_scheme: String,
    pub horizon: FantasySimulationHorizon,
    pub user_team: String,
    pub rows: Vec<FantasySimulationTeamRow>,
    pub scenarios: Vec<FantasySimulationScenarioRow>,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
    pub source_state: Vec<SourceState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasySimulationHorizon {
    FullSeason,
    RemainingSeason,
    Weeks(u8),
    Custom {
        start_date: String,
        end_date: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySimulationTeamInput {
    pub team: String,
    pub owner: String,
    pub projected_score: f64,
    pub games_remaining: u32,
    pub rostered_players: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySimulationRosterTeamInput {
    pub team: String,
    pub owner: String,
    pub roster: Vec<String>,
}

pub struct FantasySimulationBuildInput {
    pub season: Season,
    pub season_type: SeasonType,
    pub league: String,
    pub scoring_scheme: String,
    pub horizon: FantasySimulationHorizon,
    pub user_team: String,
    pub teams: Vec<FantasySimulationRosterTeamInput>,
    pub remaining_by_team: HashMap<String, u32>,
    pub scenarios: Vec<FantasySimulationScenarioInput>,
    pub scenario_rosters: Vec<FantasySimulationScenarioRosterInput>,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
    pub schedule_available: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySimulationScenarioRosterInput {
    pub id: String,
    pub label: String,
    pub add_player: Option<String>,
    pub drop_player: Option<String>,
    pub baseline_roster: Vec<String>,
    pub scenario_roster: Vec<String>,
    pub confidence: FantasySimulationConfidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyScenarioRosterResolution {
    pub roster: Vec<String>,
    pub resolved_add_player: Option<String>,
    pub resolved_drop_player: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySimulationScenarioInput {
    pub id: String,
    pub label: String,
    pub add_player: Option<String>,
    pub drop_player: Option<String>,
    pub projected_score_delta: f64,
    pub projected_games_delta: i32,
    pub confidence: FantasySimulationConfidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySimulationTeamRow {
    pub rank: usize,
    pub team: String,
    pub owner: String,
    pub is_user_team: bool,
    pub projected_score: f64,
    pub games_remaining: u32,
    pub rostered_players: u16,
    pub score_gap_to_leader: f64,
}

pub struct FantasyRosterScore<'a, 'r> {
    pub player: &'a PlayerView<'r>,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySimulationScenarioRow {
    pub id: String,
    pub label: String,
    pub action: FantasySimulationAction,
    pub add_player: Option<String>,
    pub drop_player: Option<String>,
    pub projected_score_delta: f64,
    pub projected_games_delta: i32,
    pub confidence: FantasySimulationConfidence,
    pub explanation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasySimulationAction {
    Improve,
    Watch,
    Avoid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasySimulationConfidence {
    High,
    Medium,
    Low,
}

pub struct FantasySimulationInput {
    pub season: Season,
    pub season_type: SeasonType,
    pub league: String,
    pub scoring_scheme: String,
    pub horizon: FantasySimulationHorizon,
    pub user_team: String,
    pub teams: Vec<FantasySimulationTeamInput>,
    pub scenarios: Vec<FantasySimulationScenarioInput>,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
    pub schedule_available: bool,
}

impl FantasySimulationView {
    pub fn from_projection(input: FantasySimulationInput) -> Self {
        let mut context = ViewContext::new(ViewWindow::new(input.season, input.season_type));
        context.completeness = Completeness::Partial;
        context.source_state = vec![
            SourceState::complete(SourceKind::FantasyImport),
            if input.schedule_available {
                SourceState::complete(SourceKind::Schedule)
            } else {
                SourceState::missing(SourceKind::Schedule)
            },
        ];

        let mut rows = input
            .teams
            .into_iter()
            .map(|team| FantasySimulationTeamRow {
                rank: 0,
                is_user_team: team.team == input.user_team,
                team: team.team,
                owner: team.owner,
                projected_score: team.projected_score,
                games_remaining: team.games_remaining,
                rostered_players: team.rostered_players,
                score_gap_to_leader: 0.0,
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            b.projected_score
                .total_cmp(&a.projected_score)
                .then_with(|| a.team.cmp(&b.team))
        });
        let leader_score = rows
            .first()
            .map(|row| row.projected_score)
            .unwrap_or_default();
        for (idx, row) in rows.iter_mut().enumerate() {
            row.rank = idx + 1;
            row.score_gap_to_leader = leader_score - row.projected_score;
        }

        let scenarios = input
            .scenarios
            .into_iter()
            .map(|scenario| {
                let action = classify_scenario(scenario.projected_score_delta);
                let explanation = scenario_explanation(&scenario, action);
                FantasySimulationScenarioRow {
                    id: scenario.id,
                    label: scenario.label,
                    action,
                    add_player: scenario.add_player,
                    drop_player: scenario.drop_player,
                    projected_score_delta: scenario.projected_score_delta,
                    projected_games_delta: scenario.projected_games_delta,
                    confidence: scenario.confidence,
                    explanation,
                }
            })
            .collect();

        Self {
            context: context.clone(),
            league: input.league,
            scoring_scheme: input.scoring_scheme,
            horizon: input.horizon,
            user_team: input.user_team,
            rows,
            scenarios,
            assumptions: input.assumptions,
            warnings: input.warnings,
            source_state: context.source_state,
        }
    }
}

pub fn build_fantasy_simulation_view(
    input: FantasySimulationBuildInput,
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
    scheme: &Scheme,
) -> FantasySimulationView {
    let scenario_inputs = build_scenario_inputs(
        input.scenarios,
        input.scenario_rosters,
        skaters,
        goalies,
        scheme,
        &input.remaining_by_team,
    );
    let teams = input
        .teams
        .into_iter()
        .map(|team| {
            let scored = score_fantasy_roster(&team.roster, skaters, goalies, scheme);
            let current_score = scored.iter().map(|row| row.score as f64).sum::<f64>();
            let games_remaining = fantasy_roster_games_remaining(
                &team.roster,
                skaters,
                goalies,
                &input.remaining_by_team,
            );
            let projected_score = project_fantasy_roster_score(
                current_score,
                &team.roster,
                skaters,
                goalies,
                &input.remaining_by_team,
            );
            FantasySimulationTeamInput {
                team: team.team,
                owner: team.owner,
                projected_score,
                games_remaining,
                rostered_players: team.roster.len().min(u16::MAX as usize) as u16,
            }
        })
        .collect();

    FantasySimulationView::from_projection(FantasySimulationInput {
        season: input.season,
        season_type: input.season_type,
        league: input.league,
        scoring_scheme: input.scoring_scheme,
        horizon: input.horizon,
        user_team: input.user_team,
        teams,
        scenarios: scenario_inputs,
        assumptions: input.assumptions,
        warnings: input.warnings,
        schedule_available: input.schedule_available,
    })
}

fn build_scenario_inputs(
    scenarios: Vec<FantasySimulationScenarioInput>,
    scenario_rosters: Vec<FantasySimulationScenarioRosterInput>,
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
    scheme: &Scheme,
    remaining_by_team: &HashMap<String, u32>,
) -> Vec<FantasySimulationScenarioInput> {
    let mut inputs = scenarios;
    inputs.extend(scenario_rosters.into_iter().map(|scenario| {
        project_fantasy_scenario(scenario, skaters, goalies, scheme, remaining_by_team)
    }));
    inputs
}

pub fn project_fantasy_scenario(
    scenario: FantasySimulationScenarioRosterInput,
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
    scheme: &Scheme,
    remaining_by_team: &HashMap<String, u32>,
) -> FantasySimulationScenarioInput {
    let before_score = score_fantasy_roster(&scenario.baseline_roster, skaters, goalies, scheme)
        .iter()
        .map(|row| row.score as f64)
        .sum::<f64>();
    let after_score = score_fantasy_roster(&scenario.scenario_roster, skaters, goalies, scheme)
        .iter()
        .map(|row| row.score as f64)
        .sum::<f64>();
    let before_projected = project_fantasy_roster_score(
        before_score,
        &scenario.baseline_roster,
        skaters,
        goalies,
        remaining_by_team,
    );
    let after_projected = project_fantasy_roster_score(
        after_score,
        &scenario.scenario_roster,
        skaters,
        goalies,
        remaining_by_team,
    );
    let before_remaining = fantasy_roster_games_remaining(
        &scenario.baseline_roster,
        skaters,
        goalies,
        remaining_by_team,
    );
    let after_remaining = fantasy_roster_games_remaining(
        &scenario.scenario_roster,
        skaters,
        goalies,
        remaining_by_team,
    );
    FantasySimulationScenarioInput {
        id: scenario.id,
        label: scenario.label,
        add_player: scenario.add_player,
        drop_player: scenario.drop_player,
        projected_score_delta: after_projected - before_projected,
        projected_games_delta: after_remaining as i32 - before_remaining as i32,
        confidence: scenario.confidence,
    }
}

pub fn project_fantasy_roster_score(
    current_score: f64,
    roster: &[String],
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
    remaining_by_team: &HashMap<String, u32>,
) -> f64 {
    let games_played = fantasy_roster_games_played(roster, skaters, goalies);
    let games_remaining =
        fantasy_roster_games_remaining(roster, skaters, goalies, remaining_by_team);
    current_score
        + if games_played > 0 {
            current_score / games_played as f64 * games_remaining as f64
        } else {
            0.0
        }
}

pub fn resolve_fantasy_scenario_roster(
    baseline: &[String],
    add_player: Option<&str>,
    drop_player: Option<&str>,
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
) -> Result<Vec<String>, String> {
    resolve_fantasy_scenario_roster_details(baseline, add_player, drop_player, skaters, goalies)
        .map(|resolution| resolution.roster)
}

pub fn resolve_fantasy_scenario_roster_details(
    baseline: &[String],
    add_player: Option<&str>,
    drop_player: Option<&str>,
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
) -> Result<FantasyScenarioRosterResolution, String> {
    let mut roster = Vec::with_capacity(baseline.len() + usize::from(add_player.is_some()));
    let mut resolved_drop_player = None;
    if let Some(drop_player) = drop_player {
        let drop_norm = normalize_name(drop_player);
        for name in baseline {
            if normalized_name_contains(name, drop_norm.as_str()) {
                if resolved_drop_player.is_none() {
                    resolved_drop_player = Some(
                        find_fantasy_roster_player(name, skaters, goalies)
                            .map(|view| view.identity.full_name.clone())
                            .unwrap_or_else(|| name.clone()),
                    );
                }
            } else {
                roster.push(name.clone());
            }
        }
        if resolved_drop_player.is_none() {
            return Err(format!(
                "drop player '{drop_player}' was not found on the active fantasy roster"
            ));
        }
    } else {
        roster.extend_from_slice(baseline);
    }
    let mut resolved_add_player = None;
    if let Some(add_player) = add_player {
        let add_norm = normalize_name(add_player);
        let player = skaters
            .iter()
            .find(|view| {
                normalized_name_contains(&view.identity.name_normalized, add_norm.as_str())
            })
            .or_else(|| {
                goalies.iter().find(|view| {
                    normalized_name_contains(&view.identity.name_normalized, add_norm.as_str())
                })
            })
            .ok_or_else(|| format!("no player found matching '{add_player}'"))?;
        resolved_add_player = Some(player.identity.full_name.clone());
        let full_name = player.identity.name_normalized.clone();
        if !roster.iter().any(|name| name == &full_name) {
            roster.push(full_name);
        }
    }
    Ok(FantasyScenarioRosterResolution {
        roster,
        resolved_add_player,
        resolved_drop_player,
    })
}

pub fn score_fantasy_roster<'a, 'r>(
    roster_norms: &[String],
    skaters: &'a [PlayerView<'r>],
    goalies: &'a [PlayerView<'r>],
    scheme: &Scheme,
) -> Vec<FantasyRosterScore<'a, 'r>> {
    let mut results = Vec::new();
    for name in roster_norms {
        if let Some(player) = find_fantasy_roster_player(name, skaters, goalies) {
            let gp = player.gp();
            let score = if player.position().abbreviation() == "G" {
                goalie_scheme_stats_from_view(player)
                    .and_then(|stats| compute_goalie_fantasy_score(&stats, &scheme.goalie, gp))
            } else {
                compute_fantasy_score(&skater_scheme_stats_from_view(player), &scheme.skater, gp)
            }
            .map(|score| score.total)
            .unwrap_or(0.0);
            results.push(FantasyRosterScore { player, score });
        }
    }
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

pub fn find_fantasy_roster_player<'a, 'r>(
    name: &str,
    skaters: &'a [PlayerView<'r>],
    goalies: &'a [PlayerView<'r>],
) -> Option<&'a PlayerView<'r>> {
    let norm = normalize_name(name);
    skaters
        .iter()
        .find(|view| normalized_name_contains(&view.identity.name_normalized, norm.as_str()))
        .or_else(|| {
            goalies.iter().find(|view| {
                normalized_name_contains(&view.identity.name_normalized, norm.as_str())
            })
        })
}

fn normalized_name_contains(candidate: &str, query: &str) -> bool {
    candidate.contains(query) || compact_name_key(candidate).contains(&compact_name_key(query))
}

fn compact_name_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn fantasy_roster_games_played(
    roster: &[String],
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
) -> u32 {
    roster
        .iter()
        .filter_map(|name| find_fantasy_roster_player(name, skaters, goalies))
        .map(|view| view.gp())
        .sum()
}

pub fn fantasy_roster_games_remaining(
    roster: &[String],
    skaters: &[PlayerView<'_>],
    goalies: &[PlayerView<'_>],
    remaining_by_team: &std::collections::HashMap<String, u32>,
) -> u32 {
    roster
        .iter()
        .filter_map(|name| find_fantasy_roster_player(name, skaters, goalies))
        .filter_map(|view| remaining_by_team.get(view.team_display()).copied())
        .sum()
}

pub fn skater_scheme_stats_from_view(v: &PlayerView<'_>) -> SkaterStats {
    let totals = &v.stats.totals;
    SkaterStats {
        goals: totals.goals,
        assists: totals.assists,
        pp_goals: totals.pp_goals,
        pp_assists: totals.pp_points.saturating_sub(totals.pp_goals),
        sh_goals: totals.sh_goals,
        sh_assists: totals.sh_points.saturating_sub(totals.sh_goals),
        gwg: totals.gwg,
        ot_goals: totals.ot_goals,
        hits: v.hits().unwrap_or(0),
        blocks: v.blocked_shots().unwrap_or(0),
        shots_on_goal: v.shots(),
        plus_minus: v.plus_minus(),
        takeaways: v.takeaways().unwrap_or(0),
        giveaways: v.giveaways().unwrap_or(0),
        faceoff_wins: 0,
    }
}

pub fn goalie_scheme_stats_from_view(v: &PlayerView<'_>) -> Option<GoalieScoreStats> {
    let g = v.stats.goalie.as_ref()?;
    Some(GoalieScoreStats {
        games_played: g.games_started,
        wins: g.wins,
        losses: g.losses,
        saves: g.saves,
        goals_against: g.goals_against,
        shutouts: g.shutouts,
        save_pct: g.save_pct.unwrap_or(0.0),
    })
}

fn classify_scenario(projected_score_delta: f64) -> FantasySimulationAction {
    if projected_score_delta >= 5.0 {
        FantasySimulationAction::Improve
    } else if projected_score_delta > 0.0 {
        FantasySimulationAction::Watch
    } else {
        FantasySimulationAction::Avoid
    }
}

fn scenario_explanation(
    scenario: &FantasySimulationScenarioInput,
    action: FantasySimulationAction,
) -> String {
    let verb = match action {
        FantasySimulationAction::Improve => "improves",
        FantasySimulationAction::Watch => "slightly improves",
        FantasySimulationAction::Avoid => "does not improve",
    };
    match (&scenario.add_player, &scenario.drop_player) {
        (Some(add), Some(drop)) => format!(
            "{add} for {drop} {verb} projected score by {:.1}.",
            scenario.projected_score_delta
        ),
        (Some(add), None) => format!(
            "Adding {add} {verb} projected score by {:.1}.",
            scenario.projected_score_delta
        ),
        (None, Some(drop)) => format!(
            "Dropping {drop} {verb} projected score by {:.1}.",
            scenario.projected_score_delta
        ),
        (None, None) => format!(
            "Scenario {verb} projected score by {:.1}.",
            scenario.projected_score_delta
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures, Scheme};
    use std::collections::HashMap;

    #[test]
    fn fantasy_simulation_view_ranks_teams_and_classifies_scenarios() {
        let view = FantasySimulationView::from_projection(FantasySimulationInput {
            season: Season(20252026),
            season_type: SeasonType::Regular,
            league: "Main".to_string(),
            scoring_scheme: "yahoo-standard".to_string(),
            horizon: FantasySimulationHorizon::Weeks(4),
            user_team: "Mine".to_string(),
            teams: vec![
                FantasySimulationTeamInput {
                    team: "Mine".to_string(),
                    owner: "me".to_string(),
                    projected_score: 1200.0,
                    games_remaining: 42,
                    rostered_players: 16,
                },
                FantasySimulationTeamInput {
                    team: "Leader".to_string(),
                    owner: "rival".to_string(),
                    projected_score: 1250.0,
                    games_remaining: 44,
                    rostered_players: 16,
                },
            ],
            scenarios: vec![
                FantasySimulationScenarioInput {
                    id: "add-block-helper".to_string(),
                    label: "Add Block Helper".to_string(),
                    add_player: Some("Block Helper".to_string()),
                    drop_player: Some("Bench Forward".to_string()),
                    projected_score_delta: 8.5,
                    projected_games_delta: 1,
                    confidence: FantasySimulationConfidence::Medium,
                },
                FantasySimulationScenarioInput {
                    id: "tiny-stream".to_string(),
                    label: "Tiny Stream".to_string(),
                    add_player: Some("Streamer".to_string()),
                    drop_player: None,
                    projected_score_delta: 1.5,
                    projected_games_delta: 2,
                    confidence: FantasySimulationConfidence::Low,
                },
                FantasySimulationScenarioInput {
                    id: "bad-drop".to_string(),
                    label: "Bad Drop".to_string(),
                    add_player: None,
                    drop_player: Some("Core Player".to_string()),
                    projected_score_delta: -3.0,
                    projected_games_delta: -1,
                    confidence: FantasySimulationConfidence::High,
                },
            ],
            assumptions: vec!["uses current rostered players only".to_string()],
            warnings: vec!["schedule weighting not wired yet".to_string()],
            schedule_available: false,
        });

        assert_eq!(view.rows[0].team, "Leader");
        assert_eq!(view.rows[0].rank, 1);
        assert_eq!(view.rows[1].team, "Mine");
        assert_eq!(view.rows[1].score_gap_to_leader, 50.0);
        assert_eq!(view.scenarios[0].action, FantasySimulationAction::Improve);
        assert_eq!(view.scenarios[1].action, FantasySimulationAction::Watch);
        assert_eq!(view.scenarios[2].action, FantasySimulationAction::Avoid);
        assert_eq!(view.source_state[0].source, SourceKind::FantasyImport);
        assert_eq!(view.source_state[1].source, SourceKind::Schedule);
    }

    #[test]
    fn fantasy_simulation_builder_projects_from_rosters_and_remaining_games() {
        let id = fixtures::identity(8478402)
            .name("Connor McDavid", "connor_mcdavid")
            .build();
        let stats = fixtures::stats(8478402, 20252026, "EDM").build();
        let repo = fixtures::test_repo_with(id, stats);
        let skaters = repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect::<Vec<_>>();
        let goalies = Vec::new();
        let mut remaining_by_team = HashMap::new();
        remaining_by_team.insert("EDM".to_string(), 10);

        let view = build_fantasy_simulation_view(
            FantasySimulationBuildInput {
                season: Season(20252026),
                season_type: SeasonType::Regular,
                league: "Main".to_string(),
                scoring_scheme: "simple-pts".to_string(),
                horizon: FantasySimulationHorizon::Weeks(4),
                user_team: "Mine".to_string(),
                teams: vec![
                    FantasySimulationRosterTeamInput {
                        team: "Mine".to_string(),
                        owner: "me".to_string(),
                        roster: vec!["connor_mcdavid".to_string()],
                    },
                    FantasySimulationRosterTeamInput {
                        team: "Empty".to_string(),
                        owner: "them".to_string(),
                        roster: Vec::new(),
                    },
                ],
                remaining_by_team,
                scenarios: Vec::new(),
                scenario_rosters: vec![FantasySimulationScenarioRosterInput {
                    id: "add-mcdavid".to_string(),
                    label: "Add McDavid".to_string(),
                    add_player: Some("Connor McDavid".to_string()),
                    drop_player: None,
                    baseline_roster: Vec::new(),
                    scenario_roster: vec!["connor_mcdavid".to_string()],
                    confidence: FantasySimulationConfidence::High,
                }],
                assumptions: Vec::new(),
                warnings: Vec::new(),
                schedule_available: true,
            },
            &skaters,
            &goalies,
            &Scheme::simple_pts(),
        );

        assert_eq!(view.rows[0].team, "Mine");
        assert_eq!(view.rows[0].games_remaining, 10);
        assert!(view.rows[0].projected_score > 80.0);
        assert_eq!(view.rows[1].team, "Empty");
        assert_eq!(view.scenarios[0].action, FantasySimulationAction::Improve);
        assert!(view.scenarios[0].projected_score_delta > 80.0);
        assert_eq!(view.scenarios[0].projected_games_delta, 10);
        assert_eq!(view.source_state[1].state, Completeness::Complete);
    }

    #[test]
    fn fantasy_scenario_roster_resolves_add_and_drop() {
        let id = fixtures::identity(8478402)
            .name("Connor McDavid", "connor_mcdavid")
            .build();
        let stats = fixtures::stats(8478402, 20252026, "EDM").build();
        let repo = fixtures::test_repo_with(id, stats);
        let skaters = repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect::<Vec<_>>();
        let goalies = Vec::new();
        let baseline = vec!["bench_forward".to_string()];

        let resolution = resolve_fantasy_scenario_roster_details(
            &baseline,
            Some("Connor McDavid"),
            Some("bench"),
            &skaters,
            &goalies,
        )
        .expect("scenario roster resolves");

        assert_eq!(resolution.roster, vec!["connor_mcdavid".to_string()]);
        assert_eq!(
            resolution.resolved_add_player.as_deref(),
            Some("Connor McDavid")
        );
        assert_eq!(
            resolution.resolved_drop_player.as_deref(),
            Some("bench_forward")
        );
    }

    #[test]
    fn fantasy_scenario_roster_errors_when_drop_is_not_on_roster() {
        let id = fixtures::identity(8478402)
            .name("Connor McDavid", "connor_mcdavid")
            .build();
        let stats = fixtures::stats(8478402, 20252026, "EDM").build();
        let repo = fixtures::test_repo_with(id, stats);
        let skaters = repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect::<Vec<_>>();
        let goalies = Vec::new();
        let baseline = vec!["bench_forward".to_string()];

        let err = resolve_fantasy_scenario_roster_details(
            &baseline,
            Some("Connor McDavid"),
            Some("Ghost Player"),
            &skaters,
            &goalies,
        )
        .expect_err("missing drop player should error");

        assert!(err.contains("was not found on the active fantasy roster"));
    }
}
