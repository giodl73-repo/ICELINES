use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::view_model::{Completeness, SourceKind, SourceState, ViewContext};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerGoalRecordInput {
    pub game_id: u64,
    pub date: Option<String>,
    pub scorer_id: Option<u32>,
    pub scorer_name: String,
    pub scorer_team: String,
    pub opponent_team: String,
    pub period: u8,
    pub time_in_period: String,
    pub goalie_id: Option<u32>,
    pub goalie_name: Option<String>,
    pub empty_net: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FightRecordInput {
    pub game_id: u64,
    pub date: Option<String>,
    pub player_id: u32,
    pub player_name: String,
    pub player_team: String,
    pub opponent_id: u32,
    pub opponent_name: String,
    pub opponent_team: String,
    pub period: u8,
    pub time_in_period: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordsOpponentRow {
    pub key: String,
    pub label: String,
    pub count: u32,
    pub first_game_id: u64,
    pub first_date: Option<String>,
    pub last_game_id: u64,
    pub last_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRecordsView {
    pub context: ViewContext,
    pub player_id: u32,
    pub player_name: String,
    pub metric: String,
    pub rows: Vec<RecordsOpponentRow>,
    pub incomplete_goal_rows: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRecordsView {
    pub context: ViewContext,
    pub team: String,
    pub metric: String,
    pub rows: Vec<RecordsOpponentRow>,
    pub incomplete_goal_rows: u32,
}

impl PlayerRecordsView {
    pub fn teams_scored_against(
        mut context: ViewContext,
        player_id: u32,
        player_name: impl Into<String>,
        goals: &[PlayerGoalRecordInput],
    ) -> Self {
        let mut incomplete_goal_rows = 0;
        let mut grouped: BTreeMap<String, RecordsOpponentRow> = BTreeMap::new();
        for goal in goals {
            match goal.scorer_id {
                Some(id) if id == player_id => upsert_row(
                    &mut grouped,
                    &goal.opponent_team,
                    &goal.opponent_team,
                    goal.game_id,
                    goal.date.clone(),
                ),
                None if !goal.scorer_name.is_empty() => incomplete_goal_rows += 1,
                _ => {}
            }
        }
        finish_context(
            &mut context,
            goals.is_empty(),
            incomplete_goal_rows,
            SourceKind::Boxscore,
            "goal rows lacked scorer ids and were excluded",
        );
        let mut rows = grouped.into_values().collect::<Vec<_>>();
        sort_rows(&mut rows);
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            metric: "teams-scored-against".to_string(),
            rows,
            incomplete_goal_rows,
        }
    }

    pub fn goalies_scored_against(
        mut context: ViewContext,
        player_id: u32,
        player_name: impl Into<String>,
        goals: &[PlayerGoalRecordInput],
    ) -> Self {
        let mut incomplete_goal_rows = 0;
        let mut grouped: BTreeMap<String, RecordsOpponentRow> = BTreeMap::new();
        for goal in goals {
            if goal.scorer_id != Some(player_id) {
                if goal.scorer_id.is_none() && !goal.scorer_name.is_empty() {
                    incomplete_goal_rows += 1;
                }
                continue;
            }
            let Some(goalie_id) = goal.goalie_id else {
                incomplete_goal_rows += 1;
                continue;
            };
            let key = goalie_id.to_string();
            let label = goal
                .goalie_name
                .as_deref()
                .filter(|name| !name.is_empty())
                .unwrap_or(&key);
            upsert_row(&mut grouped, &key, label, goal.game_id, goal.date.clone());
        }
        finish_context(
            &mut context,
            goals.is_empty(),
            incomplete_goal_rows,
            SourceKind::PlayByPlay,
            "goal rows lacked scorer or goalie ids and were excluded",
        );
        let mut rows = grouped.into_values().collect::<Vec<_>>();
        sort_rows(&mut rows);
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            metric: "goalies-scored-against".to_string(),
            rows,
            incomplete_goal_rows,
        }
    }

    pub fn fight_opponents(
        mut context: ViewContext,
        player_id: u32,
        player_name: impl Into<String>,
        fights: &[FightRecordInput],
    ) -> Self {
        let mut grouped: BTreeMap<String, RecordsOpponentRow> = BTreeMap::new();
        for fight in fights {
            if fight.player_id != player_id {
                continue;
            }
            let key = fight.opponent_id.to_string();
            let label = if fight.opponent_name.is_empty() {
                key.as_str()
            } else {
                fight.opponent_name.as_str()
            };
            upsert_row(&mut grouped, &key, label, fight.game_id, fight.date.clone());
        }
        finish_context(
            &mut context,
            fights.is_empty(),
            0,
            SourceKind::PlayByPlay,
            "fight rows lacked participant ids and were excluded",
        );
        let mut rows = grouped.into_values().collect::<Vec<_>>();
        sort_rows(&mut rows);
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            metric: "fight-opponents".to_string(),
            rows,
            incomplete_goal_rows: 0,
        }
    }
}

impl TeamRecordsView {
    pub fn players_scored_against_team(
        mut context: ViewContext,
        team: impl Into<String>,
        goals: &[PlayerGoalRecordInput],
    ) -> Self {
        let team = team.into().to_uppercase();
        let mut incomplete_goal_rows = 0;
        let mut grouped: BTreeMap<String, RecordsOpponentRow> = BTreeMap::new();
        for goal in goals {
            if goal.opponent_team != team {
                continue;
            }
            if let Some(scorer_id) = goal.scorer_id {
                upsert_row(
                    &mut grouped,
                    &scorer_id.to_string(),
                    &goal.scorer_name,
                    goal.game_id,
                    goal.date.clone(),
                );
            } else if !goal.scorer_name.is_empty() {
                incomplete_goal_rows += 1;
            }
        }
        finish_context(
            &mut context,
            goals.is_empty(),
            incomplete_goal_rows,
            SourceKind::Boxscore,
            "goal rows lacked scorer ids and were excluded",
        );
        let mut rows = grouped.into_values().collect::<Vec<_>>();
        sort_rows(&mut rows);
        Self {
            context,
            team,
            metric: "players-scored-against-team".to_string(),
            rows,
            incomplete_goal_rows,
        }
    }

    pub fn goalies_beaten_by_team(
        mut context: ViewContext,
        team: impl Into<String>,
        goals: &[PlayerGoalRecordInput],
    ) -> Self {
        let team = team.into().to_uppercase();
        let mut incomplete_goal_rows = 0;
        let mut grouped: BTreeMap<String, RecordsOpponentRow> = BTreeMap::new();
        for goal in goals {
            if goal.scorer_team != team {
                continue;
            }
            let Some(goalie_id) = goal.goalie_id else {
                incomplete_goal_rows += 1;
                continue;
            };
            let key = goalie_id.to_string();
            let label = goal
                .goalie_name
                .as_deref()
                .filter(|name| !name.is_empty())
                .unwrap_or(&key);
            upsert_row(&mut grouped, &key, label, goal.game_id, goal.date.clone());
        }
        finish_context(
            &mut context,
            goals.is_empty(),
            incomplete_goal_rows,
            SourceKind::PlayByPlay,
            "goal rows lacked goalie ids and were excluded",
        );
        let mut rows = grouped.into_values().collect::<Vec<_>>();
        sort_rows(&mut rows);
        Self {
            context,
            team,
            metric: "goalies-beaten-by-team".to_string(),
            rows,
            incomplete_goal_rows,
        }
    }

    pub fn fight_opponents_by_team(
        mut context: ViewContext,
        team: impl Into<String>,
        fights: &[FightRecordInput],
    ) -> Self {
        let team = team.into().to_uppercase();
        let mut grouped: BTreeMap<String, RecordsOpponentRow> = BTreeMap::new();
        for fight in fights {
            if fight.player_team != team {
                continue;
            }
            let key = fight.opponent_id.to_string();
            let label = if fight.opponent_name.is_empty() {
                key.as_str()
            } else {
                fight.opponent_name.as_str()
            };
            upsert_row(&mut grouped, &key, label, fight.game_id, fight.date.clone());
        }
        finish_context(
            &mut context,
            fights.is_empty(),
            0,
            SourceKind::PlayByPlay,
            "fight rows lacked participant ids and were excluded",
        );
        let mut rows = grouped.into_values().collect::<Vec<_>>();
        sort_rows(&mut rows);
        Self {
            context,
            team,
            metric: "fight-opponents-by-team".to_string(),
            rows,
            incomplete_goal_rows: 0,
        }
    }
}

fn upsert_row(
    grouped: &mut BTreeMap<String, RecordsOpponentRow>,
    key: &str,
    label: &str,
    game_id: u64,
    date: Option<String>,
) {
    if key.is_empty() {
        return;
    }
    grouped
        .entry(key.to_string())
        .and_modify(|row| {
            row.count += 1;
            row.last_game_id = game_id;
            row.last_date = date.clone();
        })
        .or_insert_with(|| RecordsOpponentRow {
            key: key.to_string(),
            label: label.to_string(),
            count: 1,
            first_game_id: game_id,
            first_date: date.clone(),
            last_game_id: game_id,
            last_date: date,
        });
}

fn finish_context(
    context: &mut ViewContext,
    no_goals: bool,
    incomplete_goal_rows: u32,
    source_kind: SourceKind,
    incomplete_message: &str,
) {
    let mut source = if no_goals {
        SourceState::missing(source_kind)
    } else {
        SourceState::complete(source_kind)
    };
    if incomplete_goal_rows > 0 {
        source.state = Completeness::Partial;
        source.message = Some(format!("{incomplete_goal_rows} {incomplete_message}"));
        context.completeness = Completeness::Partial;
    } else if no_goals {
        context.completeness = Completeness::Unavailable;
    }
    context.source_state.push(source);
}

fn sort_rows(rows: &mut [RecordsOpponentRow]) {
    rows.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.label.cmp(&b.label))
            .then_with(|| a.key.cmp(&b.key))
    });
}

#[cfg(test)]
mod tests {
    use crate::{
        model::Season,
        season_stats::SeasonType,
        view_model::{
            FightRecordInput, PlayerGoalRecordInput, PlayerRecordsView, TeamRecordsView,
            ViewContext, ViewWindow,
        },
    };

    fn ctx() -> ViewContext {
        ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular))
    }

    fn goal(
        game_id: u64,
        scorer_id: Option<u32>,
        scorer: &str,
        team: &str,
        opp: &str,
    ) -> PlayerGoalRecordInput {
        PlayerGoalRecordInput {
            game_id,
            date: Some(format!("2025-10-{}", game_id % 10 + 10)),
            scorer_id,
            scorer_name: scorer.to_string(),
            scorer_team: team.to_string(),
            opponent_team: opp.to_string(),
            period: 1,
            time_in_period: "01:23".to_string(),
            goalie_id: None,
            goalie_name: None,
            empty_net: false,
        }
    }

    fn fight(
        game_id: u64,
        player_id: u32,
        player: &str,
        player_team: &str,
        opponent_id: u32,
        opponent: &str,
        opponent_team: &str,
    ) -> FightRecordInput {
        FightRecordInput {
            game_id,
            date: Some(format!("2025-10-{}", game_id % 10 + 10)),
            player_id,
            player_name: player.to_string(),
            player_team: player_team.to_string(),
            opponent_id,
            opponent_name: opponent.to_string(),
            opponent_team: opponent_team.to_string(),
            period: 1,
            time_in_period: "10:20".to_string(),
        }
    }

    #[test]
    fn l0_player_records_groups_teams_scored_against() {
        let goals = vec![
            goal(1, Some(10), "Andre Burakovsky", "SEA", "EDM"),
            goal(2, Some(10), "Andre Burakovsky", "SEA", "EDM"),
            goal(3, Some(10), "Andre Burakovsky", "SEA", "BOS"),
            goal(4, Some(20), "Other", "SEA", "EDM"),
        ];

        let view = PlayerRecordsView::teams_scored_against(ctx(), 10, "Andre Burakovsky", &goals);

        assert_eq!(view.metric, "teams-scored-against");
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].key, "EDM");
        assert_eq!(view.rows[0].count, 2);
        assert_eq!(view.rows[1].key, "BOS");
        assert_eq!(view.rows[1].count, 1);
    }

    #[test]
    fn l0_player_records_groups_goalies_scored_against() {
        let mut goals = vec![
            goal(1, Some(10), "Andre Burakovsky", "SEA", "EDM"),
            goal(2, Some(10), "Andre Burakovsky", "SEA", "BOS"),
            goal(3, Some(20), "Other", "SEA", "EDM"),
        ];
        goals[0].goalie_id = Some(30);
        goals[0].goalie_name = Some("Goalie One".to_string());
        goals[1].goalie_id = Some(30);
        goals[1].goalie_name = Some("Goalie One".to_string());
        goals[2].goalie_id = Some(40);

        let view = PlayerRecordsView::goalies_scored_against(ctx(), 10, "Andre Burakovsky", &goals);

        assert_eq!(view.metric, "goalies-scored-against");
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].key, "30");
        assert_eq!(view.rows[0].label, "Goalie One");
        assert_eq!(view.rows[0].count, 2);
    }

    #[test]
    fn l0_player_records_groups_fight_opponents() {
        let fights = vec![
            fight(1, 10, "Fighter One", "SEA", 20, "Fighter Two", "EDM"),
            fight(2, 10, "Fighter One", "SEA", 20, "Fighter Two", "EDM"),
            fight(3, 20, "Fighter Two", "EDM", 10, "Fighter One", "SEA"),
        ];

        let view = PlayerRecordsView::fight_opponents(ctx(), 10, "Fighter One", &fights);

        assert_eq!(view.metric, "fight-opponents");
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].key, "20");
        assert_eq!(view.rows[0].count, 2);
    }

    #[test]
    fn l0_team_records_groups_players_scored_against_team() {
        let goals = vec![
            goal(1, Some(10), "Andre Burakovsky", "SEA", "EDM"),
            goal(2, Some(10), "Andre Burakovsky", "SEA", "EDM"),
            goal(3, Some(20), "Other Scorer", "BOS", "EDM"),
            goal(4, Some(20), "Other Scorer", "BOS", "SEA"),
        ];

        let view = TeamRecordsView::players_scored_against_team(ctx(), "EDM", &goals);

        assert_eq!(view.metric, "players-scored-against-team");
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].label, "Andre Burakovsky");
        assert_eq!(view.rows[0].count, 2);
        assert_eq!(view.rows[1].label, "Other Scorer");
    }

    #[test]
    fn l0_team_records_groups_goalies_beaten_by_team() {
        let mut goals = vec![
            goal(1, Some(10), "Andre Burakovsky", "SEA", "EDM"),
            goal(2, Some(11), "Other", "SEA", "EDM"),
            goal(3, Some(20), "Other Scorer", "BOS", "SEA"),
        ];
        goals[0].goalie_id = Some(30);
        goals[0].goalie_name = Some("Goalie One".to_string());
        goals[1].goalie_id = Some(30);
        goals[1].goalie_name = Some("Goalie One".to_string());
        goals[2].goalie_id = Some(40);

        let view = TeamRecordsView::goalies_beaten_by_team(ctx(), "SEA", &goals);

        assert_eq!(view.metric, "goalies-beaten-by-team");
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].key, "30");
        assert_eq!(view.rows[0].count, 2);
    }

    #[test]
    fn l0_team_records_groups_fight_opponents_by_team() {
        let fights = vec![
            fight(1, 10, "Fighter One", "SEA", 20, "Fighter Two", "EDM"),
            fight(2, 11, "Fighter Three", "SEA", 21, "Fighter Four", "BOS"),
            fight(3, 20, "Fighter Two", "EDM", 10, "Fighter One", "SEA"),
        ];

        let view = TeamRecordsView::fight_opponents_by_team(ctx(), "SEA", &fights);

        assert_eq!(view.metric, "fight-opponents-by-team");
        assert_eq!(view.rows.len(), 2);
        assert_eq!(view.rows[0].label, "Fighter Four");
        assert_eq!(view.rows[1].label, "Fighter Two");
    }

    #[test]
    fn l0_records_marks_missing_scorer_ids_partial() {
        let goals = vec![
            goal(1, None, "Andre Burakovsky", "SEA", "EDM"),
            goal(2, Some(10), "Andre Burakovsky", "SEA", "BOS"),
        ];

        let view = PlayerRecordsView::teams_scored_against(ctx(), 10, "Andre Burakovsky", &goals);

        assert_eq!(view.incomplete_goal_rows, 1);
        assert_eq!(
            view.context.completeness,
            crate::view_model::Completeness::Partial
        );
    }
}
