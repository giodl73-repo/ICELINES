use serde::{Deserialize, Serialize};

use crate::view_model::context::{
    EmptyKind, EmptyState, SourceKind, SourceState, ViewContext, ViewWarning,
};
use crate::view_model::scores::ScheduledGameInput;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleView {
    pub context: ViewContext,
    pub season: String,
    pub season_pretty: String,
    pub active_team: String,
    pub active_date: Option<String>,
    pub team_chips: Vec<TeamChipView>,
    pub rows: Vec<ScheduleGameRow>,
    pub total: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl ScheduleView {
    pub fn from_games(
        mut context: ViewContext,
        season: String,
        active_team: String,
        active_date: Option<String>,
        team_abbrevs: &[&str],
        games: Vec<ScheduledGameInput>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Schedule));

        let team_upper = active_team.trim().to_ascii_uppercase();
        let team_chips = team_abbrevs
            .iter()
            .map(|abbrev| TeamChipView {
                abbrev: (*abbrev).to_string(),
                is_active: abbrev.eq_ignore_ascii_case(&team_upper),
            })
            .collect();

        let mut rows: Vec<ScheduleGameRow> = games
            .into_iter()
            .map(|game| schedule_row(game, &team_upper))
            .collect();
        rows.sort_by(|a, b| a.date.cmp(&b.date));
        let total = rows.len();
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No scheduled games".to_string(),
                detail: Some("No games matched the selected schedule query.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            season_pretty: pretty_season(&season),
            season,
            active_team: team_upper,
            active_date,
            team_chips,
            rows,
            total,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamChipView {
    pub abbrev: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleGameRow {
    pub game_id: u64,
    pub date: String,
    pub away_abbrev: String,
    pub home_abbrev: String,
    pub start_time_utc: String,
    pub away_score_str: String,
    pub home_score_str: String,
    pub state_label: String,
    pub home_or_away: String,
    pub opponent_abbrev: String,
    pub is_playoff: bool,
}

fn pretty_season(season: &str) -> String {
    if season.len() == 8 {
        format!("{}-{}", &season[0..4], &season[6..8])
    } else {
        season.to_string()
    }
}

fn schedule_row(game: ScheduledGameInput, active_team: &str) -> ScheduleGameRow {
    let is_home = !active_team.is_empty() && game.home_abbrev.eq_ignore_ascii_case(active_team);
    let opponent = if active_team.is_empty() {
        String::new()
    } else if is_home {
        game.away_abbrev.clone()
    } else {
        game.home_abbrev.clone()
    };
    let home_or_away = if active_team.is_empty() {
        "—".to_string()
    } else if is_home {
        "Home".to_string()
    } else {
        "Away".to_string()
    };

    ScheduleGameRow {
        game_id: game.game_id,
        date: game.date,
        away_abbrev: game.away_abbrev,
        home_abbrev: game.home_abbrev,
        start_time_utc: game.start_time_utc,
        away_score_str: game
            .away_score
            .map(|score| score.to_string())
            .unwrap_or_default(),
        home_score_str: game
            .home_score
            .map(|score| score.to_string())
            .unwrap_or_default(),
        state_label: state_label(game.game_state.as_deref(), game.last_period.as_deref()),
        home_or_away,
        opponent_abbrev: opponent,
        is_playoff: game.game_type == 3,
    }
}

fn state_label(state: Option<&str>, last_period: Option<&str>) -> String {
    match state {
        Some("FINAL") | Some("OFF") => match last_period {
            Some("OT") => "FINAL/OT".to_string(),
            Some("SO") => "FINAL/SO".to_string(),
            _ => "FINAL".to_string(),
        },
        Some("LIVE") | Some("CRIT") => "LIVE".to_string(),
        Some("PRE") => "Pre-game".to_string(),
        Some("FUT") | None => "Scheduled".to_string(),
        Some(value) => value.to_string(),
    }
}
