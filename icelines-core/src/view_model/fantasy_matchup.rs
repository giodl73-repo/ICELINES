use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::model::Season;
use crate::season_stats::SeasonType;
use crate::view_model::context::{
    Completeness, EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWindow,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupWeekView {
    pub context: ViewContext,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub league: String,
    pub scoring_scheme: String,
    pub matchups: Vec<FantasyMatchupRow>,
    pub teams: Vec<FantasyMatchupTeamRow>,
    pub source_state: Vec<SourceState>,
    pub warnings: Vec<String>,
    pub empty_state: Option<EmptyState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupRow {
    pub rank: usize,
    pub matchup_id: Option<String>,
    pub home: FantasyMatchupSideRow,
    pub away: Option<FantasyMatchupSideRow>,
    pub winner: Option<String>,
    pub margin: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupSideRow {
    pub team: String,
    pub owner: Option<String>,
    pub is_user_team: bool,
    pub weekly_points: Option<f32>,
    pub days_scored: u8,
    pub rostered_players: u16,
    pub scored_players: u16,
    pub outcome: FantasyMatchupOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupTeamRow {
    pub rank: usize,
    pub team: String,
    pub owner: String,
    pub is_user_team: bool,
    pub weekly_points: f32,
    pub days_scored: u8,
    pub rostered_players: u16,
    pub scored_players: u16,
    pub scheduled: bool,
    pub opponent: Option<String>,
    pub outcome: FantasyMatchupOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyMatchupOutcome {
    Win,
    Loss,
    Tie,
    Bye,
    Pending,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupTeamTotalInput {
    pub team: String,
    pub owner: String,
    pub is_user_team: bool,
    pub weekly_points: f32,
    pub days_scored: u8,
    pub rostered_players: u16,
    pub scored_players: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyMatchupScheduleInput {
    pub matchup_id: Option<String>,
    pub home_team: String,
    pub away_team: Option<String>,
}

pub struct FantasyMatchupWeekInput {
    pub season: Season,
    pub season_type: SeasonType,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub league: String,
    pub scoring_scheme: String,
    pub team_totals: Vec<FantasyMatchupTeamTotalInput>,
    pub schedule: Vec<FantasyMatchupScheduleInput>,
    pub warnings: Vec<String>,
    pub source_state: Vec<SourceState>,
}

impl FantasyMatchupWeekView {
    pub fn from_input(input: FantasyMatchupWeekInput) -> Self {
        let mut source_state = input.source_state;
        if source_state.is_empty() {
            source_state = vec![
                SourceState::complete(SourceKind::FantasyImport),
                SourceState::complete(SourceKind::Schedule),
                SourceState::complete(SourceKind::Boxscore),
            ];
        }
        if input.schedule.is_empty() {
            if let Some(schedule_source) = source_state
                .iter_mut()
                .find(|source| source.source == SourceKind::Schedule)
            {
                *schedule_source = SourceState::missing(SourceKind::Schedule);
            } else {
                source_state.push(SourceState::missing(SourceKind::Schedule));
            }
        }

        let sources_complete = source_state
            .iter()
            .all(|source| source.state == Completeness::Complete);
        let totals = input
            .team_totals
            .into_iter()
            .map(|team| (team.team.clone(), team))
            .collect::<BTreeMap<_, _>>();
        let mut schedule = input.schedule;
        schedule.sort_by(|a, b| {
            a.home_team
                .cmp(&b.home_team)
                .then_with(|| a.away_team.cmp(&b.away_team))
                .then_with(|| a.matchup_id.cmp(&b.matchup_id))
        });

        let mut team_outcomes = BTreeMap::<String, TeamScheduleState>::new();
        let mut scheduled_teams = BTreeSet::<String>::new();
        let mut matchups = schedule
            .into_iter()
            .enumerate()
            .map(|(idx, matchup)| {
                let away_total = matchup.away_team.as_ref().and_then(|team| totals.get(team));
                let home_total = totals.get(&matchup.home_team);
                let (home_outcome, away_outcome, winner, margin) = matchup_outcome(
                    home_total,
                    away_total,
                    matchup.away_team.is_none(),
                    sources_complete,
                );

                scheduled_teams.insert(matchup.home_team.clone());
                team_outcomes.insert(
                    matchup.home_team.clone(),
                    TeamScheduleState {
                        opponent: matchup.away_team.clone(),
                        outcome: home_outcome,
                    },
                );
                if let Some(away_team) = &matchup.away_team {
                    scheduled_teams.insert(away_team.clone());
                    team_outcomes.insert(
                        away_team.clone(),
                        TeamScheduleState {
                            opponent: Some(matchup.home_team.clone()),
                            outcome: away_outcome.unwrap_or(FantasyMatchupOutcome::Pending),
                        },
                    );
                }

                FantasyMatchupRow {
                    rank: idx + 1,
                    matchup_id: matchup.matchup_id,
                    home: side_row(&matchup.home_team, home_total, home_outcome),
                    away: matchup.away_team.as_ref().map(|team| {
                        side_row(
                            team,
                            away_total,
                            away_outcome.unwrap_or(FantasyMatchupOutcome::Pending),
                        )
                    }),
                    winner,
                    margin,
                }
            })
            .collect::<Vec<_>>();
        for (idx, matchup) in matchups.iter_mut().enumerate() {
            matchup.rank = idx + 1;
        }

        let mut teams = totals
            .values()
            .map(|team| {
                let state = team_outcomes.get(&team.team);
                FantasyMatchupTeamRow {
                    rank: 0,
                    team: team.team.clone(),
                    owner: team.owner.clone(),
                    is_user_team: team.is_user_team,
                    weekly_points: team.weekly_points,
                    days_scored: team.days_scored,
                    rostered_players: team.rostered_players,
                    scored_players: team.scored_players,
                    scheduled: scheduled_teams.contains(&team.team),
                    opponent: state.and_then(|state| state.opponent.clone()),
                    outcome: state
                        .map(|state| state.outcome)
                        .unwrap_or(FantasyMatchupOutcome::Pending),
                }
            })
            .collect::<Vec<_>>();
        teams.sort_by(|a, b| {
            b.weekly_points
                .total_cmp(&a.weekly_points)
                .then_with(|| a.team.cmp(&b.team))
        });
        for (idx, team) in teams.iter_mut().enumerate() {
            team.rank = idx + 1;
        }

        let empty_state = if matchups.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No fantasy matchups scheduled".to_string(),
                detail: Some(
                    "Add local matchup schedule rows before viewing weekly results.".to_string(),
                ),
                recovery: Vec::new(),
            })
        } else {
            None
        };
        let any_pending = matchups
            .iter()
            .flat_map(|matchup| {
                std::iter::once(matchup.home.outcome)
                    .chain(matchup.away.as_ref().map(|away| away.outcome))
            })
            .any(|outcome| outcome == FantasyMatchupOutcome::Pending);
        let completeness = if empty_state.is_some()
            || source_state
                .iter()
                .any(|source| source.state == Completeness::Unavailable)
        {
            Completeness::Unavailable
        } else if any_pending
            || source_state
                .iter()
                .any(|source| source.state != Completeness::Complete)
        {
            Completeness::Partial
        } else {
            Completeness::Complete
        };
        let mut context = ViewContext::new(ViewWindow::new(input.season, input.season_type));
        context.source_state = source_state.clone();
        context.completeness = completeness;

        Self {
            context,
            week_start: input.week_start,
            week_end: input.week_end,
            league: input.league,
            scoring_scheme: input.scoring_scheme,
            matchups,
            teams,
            source_state,
            warnings: input.warnings,
            empty_state,
        }
    }
}

#[derive(Debug, Clone)]
struct TeamScheduleState {
    opponent: Option<String>,
    outcome: FantasyMatchupOutcome,
}

fn matchup_outcome(
    home: Option<&FantasyMatchupTeamTotalInput>,
    away: Option<&FantasyMatchupTeamTotalInput>,
    is_bye: bool,
    sources_complete: bool,
) -> (
    FantasyMatchupOutcome,
    Option<FantasyMatchupOutcome>,
    Option<String>,
    Option<f32>,
) {
    if is_bye {
        return if home.is_some() {
            (FantasyMatchupOutcome::Bye, None, None, None)
        } else {
            (FantasyMatchupOutcome::Pending, None, None, None)
        };
    }
    let (Some(home), Some(away)) = (home, away) else {
        return (
            FantasyMatchupOutcome::Pending,
            Some(FantasyMatchupOutcome::Pending),
            None,
            None,
        );
    };
    if !sources_complete {
        return (
            FantasyMatchupOutcome::Pending,
            Some(FantasyMatchupOutcome::Pending),
            None,
            Some((home.weekly_points - away.weekly_points).abs()),
        );
    }
    match home.weekly_points.total_cmp(&away.weekly_points) {
        std::cmp::Ordering::Greater => (
            FantasyMatchupOutcome::Win,
            Some(FantasyMatchupOutcome::Loss),
            Some(home.team.clone()),
            Some(home.weekly_points - away.weekly_points),
        ),
        std::cmp::Ordering::Less => (
            FantasyMatchupOutcome::Loss,
            Some(FantasyMatchupOutcome::Win),
            Some(away.team.clone()),
            Some(away.weekly_points - home.weekly_points),
        ),
        std::cmp::Ordering::Equal => (
            FantasyMatchupOutcome::Tie,
            Some(FantasyMatchupOutcome::Tie),
            None,
            Some(0.0),
        ),
    }
}

fn side_row(
    team: &str,
    total: Option<&FantasyMatchupTeamTotalInput>,
    outcome: FantasyMatchupOutcome,
) -> FantasyMatchupSideRow {
    FantasyMatchupSideRow {
        team: team.to_string(),
        owner: total.map(|team| team.owner.clone()),
        is_user_team: total.is_some_and(|team| team.is_user_team),
        weekly_points: total.map(|team| team.weekly_points),
        days_scored: total.map(|team| team.days_scored).unwrap_or_default(),
        rostered_players: total.map(|team| team.rostered_players).unwrap_or_default(),
        scored_players: total.map(|team| team.scored_players).unwrap_or_default(),
        outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, day).expect("valid test date")
    }

    fn team(name: &str, points: f32) -> FantasyMatchupTeamTotalInput {
        FantasyMatchupTeamTotalInput {
            team: name.to_string(),
            owner: format!("{name} Owner"),
            is_user_team: name == "Alpha",
            weekly_points: points,
            days_scored: 3,
            rostered_players: 12,
            scored_players: 8,
        }
    }

    fn input(
        teams: Vec<FantasyMatchupTeamTotalInput>,
        schedule: Vec<FantasyMatchupScheduleInput>,
    ) -> FantasyMatchupWeekInput {
        FantasyMatchupWeekInput {
            season: Season(20252026),
            season_type: SeasonType::Regular,
            week_start: d(12),
            week_end: d(18),
            league: "Office League".to_string(),
            scoring_scheme: "yahoo-standard".to_string(),
            team_totals: teams,
            schedule,
            warnings: Vec::new(),
            source_state: Vec::new(),
        }
    }

    fn matchup(home: &str, away: Option<&str>) -> FantasyMatchupScheduleInput {
        FantasyMatchupScheduleInput {
            matchup_id: Some(format!("{home}-matchup")),
            home_team: home.to_string(),
            away_team: away.map(str::to_string),
        }
    }

    #[test]
    fn l0_fantasy_matchup_week_assigns_win_loss_and_team_ranks() {
        let view = FantasyMatchupWeekView::from_input(input(
            vec![team("Alpha", 42.5), team("Bravo", 39.0)],
            vec![matchup("Alpha", Some("Bravo"))],
        ));

        assert_eq!(view.context.completeness, Completeness::Complete);
        assert_eq!(view.matchups[0].winner.as_deref(), Some("Alpha"));
        assert_eq!(view.matchups[0].home.outcome, FantasyMatchupOutcome::Win);
        assert_eq!(
            view.matchups[0].away.as_ref().map(|away| away.outcome),
            Some(FantasyMatchupOutcome::Loss)
        );
        assert_eq!(view.matchups[0].margin, Some(3.5));
        assert_eq!(view.teams[0].team, "Alpha");
        assert_eq!(view.teams[0].rank, 1);
    }

    #[test]
    fn l0_fantasy_matchup_week_preserves_ties_and_byes() {
        let view = FantasyMatchupWeekView::from_input(input(
            vec![
                team("Alpha", 12.0),
                team("Bravo", 12.0),
                team("Charlie", 8.0),
            ],
            vec![matchup("Alpha", Some("Bravo")), matchup("Charlie", None)],
        ));

        let tied = &view.matchups[0];
        assert_eq!(tied.home.outcome, FantasyMatchupOutcome::Tie);
        assert_eq!(
            tied.away.as_ref().map(|away| away.outcome),
            Some(FantasyMatchupOutcome::Tie)
        );
        assert_eq!(tied.winner, None);
        let bye = &view.matchups[1];
        assert_eq!(bye.home.outcome, FantasyMatchupOutcome::Bye);
        assert!(bye.away.is_none());
        assert_eq!(
            view.teams
                .iter()
                .find(|team| team.team == "Charlie")
                .map(|team| team.outcome),
            Some(FantasyMatchupOutcome::Bye)
        );
    }

    #[test]
    fn l0_fantasy_matchup_week_missing_schedule_is_empty_state() {
        let view = FantasyMatchupWeekView::from_input(input(vec![team("Alpha", 10.0)], Vec::new()));

        assert!(view.matchups.is_empty());
        assert_eq!(view.context.completeness, Completeness::Unavailable);
        assert_eq!(
            view.empty_state.as_ref().map(|empty| empty.kind),
            Some(EmptyKind::NoRows)
        );
        assert!(view
            .source_state
            .iter()
            .any(|source| source.source == SourceKind::Schedule
                && source.state == Completeness::Unavailable));
    }

    #[test]
    fn l0_fantasy_matchup_week_partial_sources_make_matchups_pending() {
        let mut input = input(
            vec![team("Alpha", 42.5), team("Bravo", 39.0)],
            vec![matchup("Alpha", Some("Bravo"))],
        );
        input.source_state = vec![
            SourceState::complete(SourceKind::FantasyImport),
            SourceState::complete(SourceKind::Schedule),
            SourceState::missing(SourceKind::Boxscore),
        ];

        let view = FantasyMatchupWeekView::from_input(input);

        assert_eq!(view.context.completeness, Completeness::Unavailable);
        assert_eq!(view.matchups[0].winner, None);
        assert_eq!(
            view.matchups[0].home.outcome,
            FantasyMatchupOutcome::Pending
        );
        assert_eq!(
            view.matchups[0].away.as_ref().map(|away| away.outcome),
            Some(FantasyMatchupOutcome::Pending)
        );
        assert!(view
            .source_state
            .iter()
            .any(|source| source.source == SourceKind::Boxscore
                && source.state == Completeness::Unavailable));
    }

    #[test]
    fn l0_fantasy_matchup_week_tied_team_ranks_use_team_name() {
        let view = FantasyMatchupWeekView::from_input(input(
            vec![
                team("Bravo", 10.0),
                team("Alpha", 10.0),
                team("Charlie", 9.0),
            ],
            vec![matchup("Alpha", Some("Bravo")), matchup("Charlie", None)],
        ));

        assert_eq!(
            view.teams
                .iter()
                .map(|team| team.team.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Bravo", "Charlie"]
        );
    }
}
