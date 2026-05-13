use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::career_history::{CareerGameType, CareerHistory, CareerStint};
use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};

pub const CAREER_HISTORY_FETCH_COMMAND: &str = "icelines fetch career --bundled-seasons 5";
pub const CAREER_HISTORY_STORE_PATH: &str = "~/.icelines/career_history.json";
pub const CAREER_HISTORY_MISSING_STORE_MESSAGE: &str = "career history store is empty — run `icelines fetch career --bundled-seasons 5` to populate ~/.icelines/career_history.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CareerView {
    pub context: ViewContext,
    pub league: String,
    pub season: u32,
    pub sort: CareerSortKey,
    pub rows: Vec<CareerRow>,
    pub count: usize,
    pub total: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl CareerView {
    pub fn from_histories(
        mut context: ViewContext,
        league: String,
        season: Option<u32>,
        sort: CareerSortKey,
        top: usize,
        histories: Vec<(u32, CareerHistory)>,
        names: HashMap<u32, String>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Career));

        let needle = league.to_ascii_uppercase();
        let mut matched: Vec<(u32, CareerStint)> = Vec::new();
        for (pid, history) in histories {
            for stint in history.stints {
                if stint.league.0.to_ascii_uppercase() != needle {
                    continue;
                }
                if !matches!(stint.game_type, CareerGameType::Regular) {
                    continue;
                }
                if let Some(want) = season {
                    if stint.season.0 != want {
                        continue;
                    }
                }
                matched.push((pid, stint));
            }
        }

        if season.is_none() {
            if let Some(latest) = matched.iter().map(|(_, stint)| stint.season.0).max() {
                matched.retain(|(_, stint)| stint.season.0 == latest);
            }
        }

        matched.sort_by(|(pid_a, a), (pid_b, b)| {
            let a_metric = metric(a, sort);
            let b_metric = metric(b, sort);
            b_metric
                .partial_cmp(&a_metric)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| pid_a.cmp(pid_b))
        });

        let total = matched.len();
        let resolved_season = matched
            .first()
            .map(|(_, stint)| stint.season.0)
            .unwrap_or(0);
        let rows: Vec<CareerRow> = matched
            .into_iter()
            .take(top)
            .enumerate()
            .map(|(index, (pid, stint))| {
                let points_per_game = stint.points_per_game().map(f64::from);
                CareerRow {
                    rank: index + 1,
                    player_id: pid,
                    name: names
                        .get(&pid)
                        .cloned()
                        .unwrap_or_else(|| format!("player:{pid}")),
                    team: stint.team,
                    gp: stint.gp,
                    goals: stint.goals,
                    assists: stint.assists,
                    points: stint.points,
                    points_per_game,
                }
            })
            .collect();
        let count = rows.len();
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No career rows".to_string(),
                detail: Some(
                    "No career-history stints matched the selected league and season.".to_string(),
                ),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            league,
            season: resolved_season,
            sort,
            rows,
            count,
            total,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CareerRow {
    pub rank: usize,
    pub player_id: u32,
    pub name: String,
    pub team: String,
    pub gp: u32,
    pub goals: Option<u32>,
    pub assists: Option<u32>,
    pub points: Option<u32>,
    pub points_per_game: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CareerSortKey {
    Points,
    Goals,
    Assists,
    Gp,
    Ppg,
}

impl CareerSortKey {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "points" | "p" | "pts" => Some(Self::Points),
            "goals" | "g" => Some(Self::Goals),
            "assists" | "a" => Some(Self::Assists),
            "gp" | "games" => Some(Self::Gp),
            "ppg" | "points-per-game" => Some(Self::Ppg),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Points => "points",
            Self::Goals => "goals",
            Self::Assists => "assists",
            Self::Gp => "gp",
            Self::Ppg => "ppg",
        }
    }
}

fn metric(stint: &CareerStint, sort: CareerSortKey) -> Option<f64> {
    match sort {
        CareerSortKey::Points => stint.points.map(f64::from),
        CareerSortKey::Goals => stint.goals.map(f64::from),
        CareerSortKey::Assists => stint.assists.map(f64::from),
        CareerSortKey::Gp => Some(f64::from(stint.gp)),
        CareerSortKey::Ppg => stint.points_per_game().map(f64::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_missing_store_message_names_fetch_command_and_store_path() {
        assert!(CAREER_HISTORY_MISSING_STORE_MESSAGE.contains(CAREER_HISTORY_FETCH_COMMAND));
        assert!(CAREER_HISTORY_MISSING_STORE_MESSAGE.contains(CAREER_HISTORY_STORE_PATH));
    }
}
