use serde::{Deserialize, Serialize};

use std::collections::BTreeMap;

use crate::view_model::{SourceKind, SourceState, ViewContext};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerGameLineInput {
    pub game_id: u64,
    pub date: Option<String>,
    pub player_id: u32,
    pub player_name: String,
    pub team: String,
    pub opponent: String,
    pub goals: u32,
    pub assists: u32,
}

impl PlayerGameLineInput {
    pub fn points(&self) -> u32 {
        self.goals + self.assists
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStreakRow {
    pub metric: String,
    pub current: u32,
    pub longest: u32,
    pub longest_start_date: Option<String>,
    pub longest_end_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerStreaksView {
    pub context: ViewContext,
    pub player_id: u32,
    pub player_name: String,
    pub rows: Vec<PlayerStreakRow>,
    pub games_loaded: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamPlayerStreakLeaderRow {
    pub metric: String,
    pub player_id: u32,
    pub player_name: String,
    pub current: u32,
    pub longest: u32,
    pub longest_start_date: Option<String>,
    pub longest_end_date: Option<String>,
    pub games_loaded: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamPlayerStreaksView {
    pub context: ViewContext,
    pub team: String,
    pub rows: Vec<TeamPlayerStreakLeaderRow>,
    pub games_loaded: usize,
    pub players_loaded: usize,
}

impl PlayerStreaksView {
    pub fn from_game_lines(
        mut context: ViewContext,
        player_id: u32,
        player_name: impl Into<String>,
        lines: &[PlayerGameLineInput],
    ) -> Self {
        let mut player_lines = lines
            .iter()
            .filter(|line| line.player_id == player_id)
            .cloned()
            .collect::<Vec<_>>();
        player_lines.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.game_id.cmp(&b.game_id)));

        context.source_state.push(if player_lines.is_empty() {
            SourceState::missing(SourceKind::Boxscore)
        } else {
            SourceState::complete(SourceKind::Boxscore)
        });

        let rows = vec![
            streak_row("goals", &player_lines, |line| line.goals > 0),
            streak_row("assists", &player_lines, |line| line.assists > 0),
            streak_row("points", &player_lines, |line| line.points() > 0),
        ];
        Self {
            context,
            player_id,
            player_name: player_name.into(),
            rows,
            games_loaded: player_lines.len(),
        }
    }
}

impl TeamPlayerStreaksView {
    pub fn from_game_lines(
        mut context: ViewContext,
        team: impl Into<String>,
        lines: &[PlayerGameLineInput],
    ) -> Self {
        let team = team.into().to_ascii_uppercase();
        let mut team_lines = lines
            .iter()
            .filter(|line| line.team.eq_ignore_ascii_case(&team))
            .cloned()
            .collect::<Vec<_>>();
        team_lines.sort_by(|a, b| {
            a.player_id
                .cmp(&b.player_id)
                .then_with(|| a.date.cmp(&b.date))
                .then_with(|| a.game_id.cmp(&b.game_id))
        });

        context.source_state.push(if team_lines.is_empty() {
            SourceState::missing(SourceKind::Boxscore)
        } else {
            SourceState::complete(SourceKind::Boxscore)
        });

        let mut by_player: BTreeMap<u32, Vec<PlayerGameLineInput>> = BTreeMap::new();
        for line in team_lines {
            by_player.entry(line.player_id).or_default().push(line);
        }
        let players_loaded = by_player.len();
        let games_loaded = by_player.values().map(Vec::len).sum();
        let rows = ["goals", "assists", "points"]
            .into_iter()
            .filter_map(|metric| best_team_leader(metric, &by_player))
            .collect();

        Self {
            context,
            team,
            rows,
            games_loaded,
            players_loaded,
        }
    }
}

fn best_team_leader(
    metric: &str,
    by_player: &BTreeMap<u32, Vec<PlayerGameLineInput>>,
) -> Option<TeamPlayerStreakLeaderRow> {
    by_player
        .iter()
        .filter_map(|(&player_id, lines)| {
            let row = match metric {
                "goals" => streak_row(metric, lines, |line| line.goals > 0),
                "assists" => streak_row(metric, lines, |line| line.assists > 0),
                "points" => streak_row(metric, lines, |line| line.points() > 0),
                _ => return None,
            };
            let player_name = lines
                .first()
                .map(|line| line.player_name.clone())
                .unwrap_or_else(|| player_id.to_string());
            Some(TeamPlayerStreakLeaderRow {
                metric: row.metric,
                player_id,
                player_name,
                current: row.current,
                longest: row.longest,
                longest_start_date: row.longest_start_date,
                longest_end_date: row.longest_end_date,
                games_loaded: lines.len(),
            })
        })
        .max_by(compare_team_streak_leaders)
}

fn compare_team_streak_leaders(
    a: &TeamPlayerStreakLeaderRow,
    b: &TeamPlayerStreakLeaderRow,
) -> std::cmp::Ordering {
    a.longest
        .cmp(&b.longest)
        .then_with(|| a.current.cmp(&b.current))
        .then_with(|| b.player_name.cmp(&a.player_name))
}

fn streak_row(
    metric: &str,
    lines: &[PlayerGameLineInput],
    qualifies: impl Fn(&PlayerGameLineInput) -> bool,
) -> PlayerStreakRow {
    let mut current_run = 0u32;
    let mut best_run = 0u32;
    let mut run_start: Option<String> = None;
    let mut best_start: Option<String> = None;
    let mut best_end: Option<String> = None;

    for line in lines {
        if qualifies(line) {
            current_run += 1;
            if run_start.is_none() {
                run_start = line.date.clone();
            }
            if current_run > best_run {
                best_run = current_run;
                best_start = run_start.clone();
                best_end = line.date.clone();
            }
        } else {
            current_run = 0;
            run_start = None;
        }
    }

    PlayerStreakRow {
        metric: metric.to_string(),
        current: current_run,
        longest: best_run,
        longest_start_date: best_start,
        longest_end_date: best_end,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::view_model::{ViewContext, ViewWindow};

    fn line(game_id: u64, date: &str, goals: u32, assists: u32) -> PlayerGameLineInput {
        PlayerGameLineInput {
            game_id,
            date: Some(date.to_string()),
            player_id: 97,
            player_name: "Test Player".to_string(),
            team: "EDM".to_string(),
            opponent: "SEA".to_string(),
            goals,
            assists,
        }
    }

    #[test]
    fn l0_streaks_use_game_rows_not_totals() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = PlayerStreaksView::from_game_lines(
            context,
            97,
            "Test Player",
            &[
                line(1, "2025-10-01", 1, 0),
                line(2, "2025-10-02", 0, 2),
                line(3, "2025-10-03", 0, 0),
                line(4, "2025-10-04", 1, 1),
            ],
        );
        let points = view.rows.iter().find(|row| row.metric == "points").unwrap();
        assert_eq!(points.longest, 2);
        assert_eq!(points.current, 1);
        let goals = view.rows.iter().find(|row| row.metric == "goals").unwrap();
        assert_eq!(goals.longest, 1);
    }

    fn team_line(
        game_id: u64,
        date: &str,
        player_id: u32,
        name: &str,
        goals: u32,
        assists: u32,
    ) -> PlayerGameLineInput {
        PlayerGameLineInput {
            game_id,
            date: Some(date.to_string()),
            player_id,
            player_name: name.to_string(),
            team: "EDM".to_string(),
            opponent: "SEA".to_string(),
            goals,
            assists,
        }
    }

    #[test]
    fn l0_team_player_streaks_pick_best_goal_assist_point_leaders() {
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = TeamPlayerStreaksView::from_game_lines(
            context,
            "EDM",
            &[
                team_line(1, "2025-10-01", 97, "Goal Leader", 1, 0),
                team_line(2, "2025-10-02", 97, "Goal Leader", 1, 0),
                team_line(3, "2025-10-03", 97, "Goal Leader", 0, 0),
                team_line(1, "2025-10-01", 29, "Assist Leader", 0, 1),
                team_line(2, "2025-10-02", 29, "Assist Leader", 0, 1),
                team_line(3, "2025-10-03", 29, "Assist Leader", 0, 1),
                team_line(1, "2025-10-01", 93, "Point Leader", 0, 1),
                team_line(2, "2025-10-02", 93, "Point Leader", 1, 0),
                team_line(3, "2025-10-03", 93, "Point Leader", 1, 1),
                team_line(4, "2025-10-04", 93, "Point Leader", 0, 1),
            ],
        );

        let goals = view.rows.iter().find(|row| row.metric == "goals").unwrap();
        assert_eq!(goals.player_name, "Goal Leader");
        assert_eq!(goals.longest, 2);
        let assists = view
            .rows
            .iter()
            .find(|row| row.metric == "assists")
            .unwrap();
        assert_eq!(assists.player_name, "Assist Leader");
        assert_eq!(assists.longest, 3);
        let points = view.rows.iter().find(|row| row.metric == "points").unwrap();
        assert_eq!(points.player_name, "Point Leader");
        assert_eq!(points.longest, 4);
    }
}
