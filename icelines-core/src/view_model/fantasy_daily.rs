use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::favorites::{Decision, GoalieNightLine, SkaterNightLine};
use crate::model::Season;
use crate::scheme::{GoalieWeights, Scheme, SkaterWeights};
use crate::season_stats::SeasonType;
use crate::view_model::context::{Completeness, SourceKind, SourceState, ViewContext, ViewWindow};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDailyDeltaView {
    pub context: ViewContext,
    pub date: NaiveDate,
    pub league: String,
    pub scoring_scheme: String,
    pub teams: Vec<FantasyDailyTeamRow>,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDailyTeamRow {
    pub rank: usize,
    pub team: String,
    pub owner: String,
    pub is_user_team: bool,
    pub daily_points: f32,
    pub rostered_players: u16,
    pub scored_players: u16,
    pub unscored_players: u16,
    pub players: Vec<FantasyDailyPlayerRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDailyPlayerRow {
    pub display_name: String,
    pub roster_key: String,
    pub position: String,
    pub nhl_team: Option<String>,
    pub opponent: Option<String>,
    pub daily_points: f32,
    pub breakdown: BTreeMap<String, f32>,
    pub status: FantasyDailyPlayerStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDailyPlayerStatus {
    Scored,
    NoFinalLine,
    Unfinalized,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyDailyTeamInput {
    pub team: String,
    pub owner: String,
    pub is_user_team: bool,
    pub roster: Vec<FantasyDailyPlayerInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyDailyPlayerInput {
    pub display_name: String,
    pub roster_key: String,
    pub position: String,
    pub line: Option<FantasyDailyLineInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FantasyDailyLineInput {
    Skater(SkaterNightLine),
    Goalie(GoalieNightLine),
}

pub struct FantasyDailyDeltaInput {
    pub season: Season,
    pub season_type: SeasonType,
    pub date: NaiveDate,
    pub league: String,
    pub scoring_scheme: String,
    pub teams: Vec<FantasyDailyTeamInput>,
    pub warnings: Vec<String>,
    pub source_state: Vec<SourceState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyDailyScore {
    pub total: f32,
    pub breakdown: BTreeMap<String, f32>,
}

impl FantasyDailyDeltaView {
    pub fn from_input(input: FantasyDailyDeltaInput, scheme: &Scheme) -> Self {
        let mut source_state = input.source_state;
        if source_state.is_empty() {
            source_state = vec![
                SourceState::complete(SourceKind::FantasyImport),
                SourceState::complete(SourceKind::Boxscore),
            ];
        }

        let mut warnings = input.warnings;
        let mut teams = input
            .teams
            .into_iter()
            .map(|team| {
                let mut players = team
                    .roster
                    .into_iter()
                    .map(|player| daily_player_row(player, scheme, &mut warnings))
                    .collect::<Vec<_>>();
                players.sort_by(|a, b| {
                    b.daily_points
                        .total_cmp(&a.daily_points)
                        .then_with(|| a.display_name.cmp(&b.display_name))
                });
                let daily_points = players.iter().map(|player| player.daily_points).sum();
                let scored_players = players
                    .iter()
                    .filter(|player| player.status == FantasyDailyPlayerStatus::Scored)
                    .count() as u16;
                let rostered_players = players.len() as u16;
                FantasyDailyTeamRow {
                    rank: 0,
                    team: team.team,
                    owner: team.owner,
                    is_user_team: team.is_user_team,
                    daily_points,
                    rostered_players,
                    scored_players,
                    unscored_players: rostered_players.saturating_sub(scored_players),
                    players,
                }
            })
            .collect::<Vec<_>>();
        teams.sort_by(|a, b| {
            b.daily_points
                .total_cmp(&a.daily_points)
                .then_with(|| a.team.cmp(&b.team))
        });
        for (idx, team) in teams.iter_mut().enumerate() {
            team.rank = idx + 1;
        }

        let mut context = ViewContext::new(ViewWindow::new(input.season, input.season_type));
        context.source_state = source_state.clone();
        context.completeness = if teams
            .iter()
            .flat_map(|team| team.players.iter())
            .any(|player| player.status != FantasyDailyPlayerStatus::Scored)
        {
            Completeness::Partial
        } else {
            Completeness::Complete
        };

        Self {
            context,
            date: input.date,
            league: input.league,
            scoring_scheme: input.scoring_scheme,
            teams,
            source_state,
            warnings,
        }
    }
}

pub fn score_daily_skater_line(
    line: &SkaterNightLine,
    weights: &SkaterWeights,
) -> Option<FantasyDailyScore> {
    if !line.game_state.is_finalized() {
        return None;
    }

    Some(daily_score_from_entries(&[
        (weights.goals * line.goals as f32, "goals"),
        (weights.assists * line.assists as f32, "assists"),
        (weights.pp_goals * line.power_play_goals as f32, "pp_goals"),
        (
            weights.pp_assists * line.power_play_assists as f32,
            "pp_assists",
        ),
        (weights.sh_goals * line.shorthanded_goals as f32, "sh_goals"),
        (weights.sh_assists * 0.0, "sh_assists"),
        (weights.gwg * 0.0, "gwg"),
        (weights.ot_goals * 0.0, "ot_goals"),
        (weights.hits * line.hits.unwrap_or(0) as f32, "hits"),
        (weights.blocks * line.blocks.unwrap_or(0) as f32, "blocks"),
        (
            weights.shots_on_goal * line.shots.unwrap_or(0) as f32,
            "shots_on_goal",
        ),
        (weights.plus_minus * line.plus_minus as f32, "plus_minus"),
        (
            weights.takeaways * line.takeaways.unwrap_or(0) as f32,
            "takeaways",
        ),
        (
            weights.giveaways * line.giveaways.unwrap_or(0) as f32,
            "giveaways",
        ),
        (weights.faceoff_wins * 0.0, "faceoff_wins"),
    ]))
}

pub fn score_daily_goalie_line(
    line: &GoalieNightLine,
    weights: &GoalieWeights,
) -> Option<FantasyDailyScore> {
    if !line.game_state.is_finalized() {
        return None;
    }
    let wins = u32::from(line.decision == Some(Decision::Win));
    let losses = u32::from(line.decision == Some(Decision::Loss));
    let shutouts = u32::from(line.shutout);

    Some(daily_score_from_entries(&[
        (weights.wins * wins as f32, "wins"),
        (weights.losses * losses as f32, "losses"),
        (weights.saves * line.saves as f32, "saves"),
        (
            weights.goals_against * line.goals_against as f32,
            "goals_against",
        ),
        (weights.shutouts * shutouts as f32, "shutouts"),
        (weights.save_pct * line.save_pct, "save_pct"),
    ]))
}

fn daily_player_row(
    player: FantasyDailyPlayerInput,
    scheme: &Scheme,
    warnings: &mut Vec<String>,
) -> FantasyDailyPlayerRow {
    let Some(line) = player.line else {
        return FantasyDailyPlayerRow {
            display_name: player.display_name,
            roster_key: player.roster_key,
            position: player.position,
            nhl_team: None,
            opponent: None,
            daily_points: 0.0,
            breakdown: BTreeMap::new(),
            status: FantasyDailyPlayerStatus::NoFinalLine,
        };
    };

    let (score, nhl_team, opponent, finalized) = match line {
        FantasyDailyLineInput::Skater(line) => (
            score_daily_skater_line(&line, &scheme.skater),
            Some(line.team.0.clone()),
            Some(line.opponent.0.clone()),
            line.game_state.is_finalized(),
        ),
        FantasyDailyLineInput::Goalie(line) => (
            score_daily_goalie_line(&line, &scheme.goalie),
            Some(line.team.0.clone()),
            Some(line.opponent.0.clone()),
            line.game_state.is_finalized(),
        ),
    };
    if !finalized {
        warnings.push(format!(
            "{} has a cached game line that is not finalized; daily fantasy points are not counted",
            player.display_name
        ));
    }
    let status = if finalized {
        FantasyDailyPlayerStatus::Scored
    } else {
        FantasyDailyPlayerStatus::Unfinalized
    };
    let score = score.unwrap_or_else(|| FantasyDailyScore {
        total: 0.0,
        breakdown: BTreeMap::new(),
    });

    FantasyDailyPlayerRow {
        display_name: player.display_name,
        roster_key: player.roster_key,
        position: player.position,
        nhl_team,
        opponent,
        daily_points: score.total,
        breakdown: score.breakdown,
        status,
    }
}

fn daily_score_from_entries(entries: &[(f32, &str)]) -> FantasyDailyScore {
    let total = entries.iter().map(|(value, _)| value).sum();
    let breakdown = entries
        .iter()
        .filter(|(value, _)| value.abs() > 0.001)
        .map(|(value, key)| ((*key).to_string(), *value))
        .collect::<BTreeMap<_, _>>();
    FantasyDailyScore { total, breakdown }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityRef;
    use crate::favorites::GameState;
    use crate::identity::PlayerId;
    use crate::model::TeamAbbr;

    fn skater_line(name_id: u32, goals: u32, assists: u32, state: GameState) -> SkaterNightLine {
        SkaterNightLine {
            player: EntityRef::Player(PlayerId(name_id)),
            team: TeamAbbr("SEA".to_string()),
            opponent: TeamAbbr("VAN".to_string()),
            home_or_away: crate::favorites::HomeAway::Home,
            team_score: 4,
            opponent_score: 2,
            result: crate::favorites::GameResult::Win,
            goals,
            assists,
            points: goals + assists,
            plus_minus: 1,
            shots: Some(5),
            hits: SkaterNightLine::gate_finalized(3, state),
            blocks: SkaterNightLine::gate_finalized(2, state),
            pim: SkaterNightLine::gate_finalized(0, state),
            takeaways: SkaterNightLine::gate_finalized(1, state),
            giveaways: SkaterNightLine::gate_finalized(0, state),
            toi_seconds: Some(1_100),
            power_play_goals: 1,
            power_play_assists: 0,
            shorthanded_goals: 0,
            game_state: state,
        }
    }

    fn goalie_line(name_id: u32, state: GameState) -> GoalieNightLine {
        GoalieNightLine {
            player: EntityRef::Player(PlayerId(name_id)),
            team: TeamAbbr("SEA".to_string()),
            opponent: TeamAbbr("VAN".to_string()),
            home_or_away: crate::favorites::HomeAway::Home,
            team_score: 4,
            opponent_score: 0,
            games_started: true,
            decision: Some(Decision::Win),
            saves: 30,
            shots_against: 30,
            goals_against: 0,
            save_pct: 1.0,
            gaa: 0.0,
            toi_seconds: Some(3_600),
            shutout: true,
            game_state: state,
        }
    }

    fn player(
        display_name: &str,
        position: &str,
        line: Option<FantasyDailyLineInput>,
    ) -> FantasyDailyPlayerInput {
        FantasyDailyPlayerInput {
            display_name: display_name.to_string(),
            roster_key: display_name.to_ascii_lowercase().replace(' ', "_"),
            position: position.to_string(),
            line,
        }
    }

    #[test]
    fn l0_fantasy_daily_skater_score_uses_one_game_weights_without_min_gp() {
        let score = score_daily_skater_line(
            &skater_line(1, 2, 1, GameState::Final),
            &Scheme::yahoo_standard().skater,
        )
        .expect("final skater line scores");

        // Yahoo standard: 2G*3 + 1A*2 + 1PPG*1 + 3HIT*0.5 + 2BLK*0.5 = 11.5.
        // Daily scoring is a one-game descriptive total, so the season MIN_GP
        // threshold in compute_fantasy_score must not suppress this row.
        assert!((score.total - 11.5).abs() < 0.001);
        assert_eq!(score.breakdown["goals"], 6.0);
        assert_eq!(score.breakdown["hits"], 1.5);
    }

    #[test]
    fn l0_fantasy_daily_goalie_score_uses_one_game_weights_without_min_gp() {
        let score = score_daily_goalie_line(
            &goalie_line(30, GameState::Final),
            &Scheme::yahoo_standard().goalie,
        )
        .expect("final goalie line scores");

        // Yahoo standard: W*5 + 30 saves*0.15 + 0 GA*-1 + SO*4 = 13.5.
        // OTL is not counted as a loss in the daily adapter; only Decision::Loss is.
        assert!((score.total - 13.5).abs() < 0.001);
        assert_eq!(score.breakdown["wins"], 5.0);
        assert_eq!(score.breakdown["saves"], 4.5);
    }

    #[test]
    fn l0_fantasy_daily_unfinalized_lines_do_not_count_zero_defaults() {
        let line = skater_line(1, 1, 0, GameState::Live);
        assert!(
            score_daily_skater_line(&line, &Scheme::yahoo_standard().skater).is_none(),
            "live NHL boxscore lines must not count defaulted physical stats"
        );
    }

    #[test]
    fn l0_fantasy_daily_view_sorts_teams_and_players_stably() {
        let view = FantasyDailyDeltaView::from_input(
            FantasyDailyDeltaInput {
                season: Season(20252026),
                season_type: SeasonType::Regular,
                date: NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
                league: "Office".to_string(),
                scoring_scheme: "yahoo-standard".to_string(),
                teams: vec![
                    FantasyDailyTeamInput {
                        team: "B Team".to_string(),
                        owner: "B".to_string(),
                        is_user_team: false,
                        roster: vec![player(
                            "Low Skater",
                            "C",
                            Some(FantasyDailyLineInput::Skater(skater_line(
                                2,
                                0,
                                1,
                                GameState::Final,
                            ))),
                        )],
                    },
                    FantasyDailyTeamInput {
                        team: "A Team".to_string(),
                        owner: "A".to_string(),
                        is_user_team: true,
                        roster: vec![
                            player(
                                "Goalie One",
                                "G",
                                Some(FantasyDailyLineInput::Goalie(goalie_line(
                                    30,
                                    GameState::Final,
                                ))),
                            ),
                            player(
                                "High Skater",
                                "C",
                                Some(FantasyDailyLineInput::Skater(skater_line(
                                    1,
                                    2,
                                    1,
                                    GameState::Final,
                                ))),
                            ),
                        ],
                    },
                ],
                warnings: Vec::new(),
                source_state: Vec::new(),
            },
            &Scheme::yahoo_standard(),
        );

        assert_eq!(view.teams[0].team, "A Team");
        assert_eq!(view.teams[0].rank, 1);
        assert_eq!(view.teams[0].players[0].display_name, "Goalie One");
        assert_eq!(view.teams[0].players[1].display_name, "High Skater");
        assert!((view.teams[0].daily_points - 25.0).abs() < 0.001);
        assert_eq!(view.context.completeness, Completeness::Complete);
    }

    #[test]
    fn l0_fantasy_daily_view_marks_missing_or_unfinalized_rows_partial() {
        let view = FantasyDailyDeltaView::from_input(
            FantasyDailyDeltaInput {
                season: Season(20252026),
                season_type: SeasonType::Regular,
                date: NaiveDate::from_ymd_opt(2026, 1, 15).expect("valid date"),
                league: "Office".to_string(),
                scoring_scheme: "yahoo-standard".to_string(),
                teams: vec![FantasyDailyTeamInput {
                    team: "A Team".to_string(),
                    owner: "A".to_string(),
                    is_user_team: true,
                    roster: vec![
                        player("Missing Player", "C", None),
                        player(
                            "Live Player",
                            "C",
                            Some(FantasyDailyLineInput::Skater(skater_line(
                                1,
                                1,
                                0,
                                GameState::Live,
                            ))),
                        ),
                    ],
                }],
                warnings: Vec::new(),
                source_state: vec![SourceState::missing(SourceKind::Boxscore)],
            },
            &Scheme::yahoo_standard(),
        );

        assert_eq!(view.context.completeness, Completeness::Partial);
        assert_eq!(view.teams[0].scored_players, 0);
        assert_eq!(view.teams[0].unscored_players, 2);
        assert_eq!(view.source_state[0].state, Completeness::Unavailable);
        assert!(
            view.warnings
                .iter()
                .any(|warning| warning.contains("not finalized")),
            "unfinalized cached lines must produce an explicit warning"
        );
    }
}
