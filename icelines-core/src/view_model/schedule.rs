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
pub struct ScheduleTeamView {
    pub context: ViewContext,
    pub season: String,
    pub season_pretty: String,
    pub team: String,
    pub record: ScheduleRecord,
    pub rows: Vec<ScheduleGameRow>,
    pub total: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl ScheduleTeamView {
    pub fn from_games(
        mut context: ViewContext,
        season: String,
        team: String,
        games: Vec<ScheduledGameInput>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Schedule));

        let team_upper = team.trim().to_ascii_uppercase();
        let mut rows: Vec<ScheduleGameRow> = games
            .into_iter()
            .map(|game| schedule_row(game, &team_upper))
            .filter(|row| row.involves(&team_upper))
            .collect();
        rows.sort_by(|a, b| a.date.cmp(&b.date));

        let record = ScheduleRecord::for_team(&team_upper, &rows);
        let total = rows.len();
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No team schedule games".to_string(),
                detail: Some("No games matched the selected team schedule.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            season_pretty: pretty_season(&season),
            season,
            team: team_upper,
            record,
            rows,
            total,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleMatchupView {
    pub context: ViewContext,
    pub season: String,
    pub season_pretty: String,
    pub team: String,
    pub opponent: String,
    pub regular_record: ScheduleMatchupRecord,
    pub playoff_record: ScheduleMatchupRecord,
    pub regular_rows: Vec<ScheduleGameRow>,
    pub playoff_rows: Vec<ScheduleGameRow>,
    pub rows: Vec<ScheduleGameRow>,
    pub total: usize,
    pub warnings: Vec<ViewWarning>,
    pub empty_state: Option<EmptyState>,
}

impl ScheduleMatchupView {
    pub fn from_games(
        mut context: ViewContext,
        season: String,
        team: String,
        opponent: String,
        games: Vec<ScheduledGameInput>,
    ) -> Self {
        context
            .source_state
            .push(SourceState::complete(SourceKind::Schedule));

        let team_upper = team.trim().to_ascii_uppercase();
        let opponent_upper = opponent.trim().to_ascii_uppercase();
        let mut rows: Vec<ScheduleGameRow> = games
            .into_iter()
            .map(|game| schedule_row(game, &team_upper))
            .filter(|row| row.involves(&team_upper) && row.involves(&opponent_upper))
            .collect();
        rows.sort_by(|a, b| a.date.cmp(&b.date));

        let regular_rows: Vec<ScheduleGameRow> = rows
            .iter()
            .filter(|row| !row.is_playoff())
            .cloned()
            .collect();
        let playoff_rows: Vec<ScheduleGameRow> = rows
            .iter()
            .filter(|row| row.is_playoff())
            .cloned()
            .collect();
        let regular_record =
            ScheduleMatchupRecord::for_team(&team_upper, &opponent_upper, &regular_rows);
        let playoff_record =
            ScheduleMatchupRecord::for_team(&team_upper, &opponent_upper, &playoff_rows);
        let total = rows.len();
        let empty_state = if rows.is_empty() {
            Some(EmptyState {
                kind: EmptyKind::NoRows,
                title: "No matchup games".to_string(),
                detail: Some("No games matched the selected season matchup.".to_string()),
                recovery: Vec::new(),
            })
        } else {
            None
        };

        Self {
            context,
            season_pretty: pretty_season(&season),
            season,
            team: team_upper,
            opponent: opponent_upper,
            regular_record,
            playoff_record,
            regular_rows,
            playoff_rows,
            rows,
            total,
            warnings: Vec::new(),
            empty_state,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleRecord {
    pub wins: u32,
    pub losses: u32,
    pub overtime_losses: u32,
    pub played: u32,
}

impl ScheduleRecord {
    fn for_team(team: &str, rows: &[ScheduleGameRow]) -> Self {
        let mut record = Self::default();
        for row in rows
            .iter()
            .filter(|row| !row.is_preseason() && row.is_final())
        {
            let Some(team_score) = row.team_score(team) else {
                continue;
            };
            let Some(opponent_score) = row.opponent_score(team) else {
                continue;
            };
            if team_score > opponent_score {
                record.wins += 1;
            } else if row.is_ot_or_so() {
                record.overtime_losses += 1;
            } else {
                record.losses += 1;
            }
        }
        record.played = record.wins + record.losses + record.overtime_losses;
        record
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleMatchupRecord {
    pub team: String,
    pub opponent: String,
    pub wins: u32,
    pub losses: u32,
}

impl ScheduleMatchupRecord {
    fn for_team(team: &str, opponent: &str, rows: &[ScheduleGameRow]) -> Self {
        let mut record = Self {
            team: team.to_string(),
            opponent: opponent.to_string(),
            wins: 0,
            losses: 0,
        };
        for row in rows.iter().filter(|row| row.is_final()) {
            let Some(team_score) = row.team_score(team) else {
                continue;
            };
            let Some(opponent_score) = row.opponent_score(team) else {
                continue;
            };
            if team_score > opponent_score {
                record.wins += 1;
            } else {
                record.losses += 1;
            }
        }
        record
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
    pub game_type: u8,
    pub away_abbrev: String,
    pub home_abbrev: String,
    pub start_time_utc: String,
    pub away_score: Option<u8>,
    pub home_score: Option<u8>,
    pub away_score_str: String,
    pub home_score_str: String,
    pub state_label: String,
    pub last_period: Option<String>,
    pub home_or_away: String,
    pub opponent_abbrev: String,
    pub is_playoff: bool,
    pub series_game: Option<String>,
    pub series_context: String,
}

impl ScheduleGameRow {
    pub fn involves(&self, team: &str) -> bool {
        self.away_abbrev.eq_ignore_ascii_case(team) || self.home_abbrev.eq_ignore_ascii_case(team)
    }

    pub fn is_playoff(&self) -> bool {
        self.is_playoff
    }

    pub fn is_preseason(&self) -> bool {
        self.game_type == 1
    }

    pub fn is_final(&self) -> bool {
        self.state_label.starts_with("FINAL")
    }

    pub fn is_live(&self) -> bool {
        self.state_label == "LIVE"
    }

    pub fn series_label(&self) -> Option<String> {
        if self.series_context.is_empty() {
            None
        } else {
            Some(self.series_context.clone())
        }
    }

    pub fn team_is_away(&self, team: &str) -> bool {
        self.away_abbrev.eq_ignore_ascii_case(team)
    }

    pub fn team_score(&self, team: &str) -> Option<u8> {
        if self.away_abbrev.eq_ignore_ascii_case(team) {
            self.away_score
        } else if self.home_abbrev.eq_ignore_ascii_case(team) {
            self.home_score
        } else {
            None
        }
    }

    pub fn opponent_score(&self, team: &str) -> Option<u8> {
        if self.away_abbrev.eq_ignore_ascii_case(team) {
            self.home_score
        } else if self.home_abbrev.eq_ignore_ascii_case(team) {
            self.away_score
        } else {
            None
        }
    }

    pub fn opponent_abbrev_for(&self, team: &str) -> Option<&str> {
        if self.away_abbrev.eq_ignore_ascii_case(team) {
            Some(&self.home_abbrev)
        } else if self.home_abbrev.eq_ignore_ascii_case(team) {
            Some(&self.away_abbrev)
        } else {
            None
        }
    }

    pub fn venue_label_for(&self, team: &str) -> Option<&'static str> {
        if self.away_abbrev.eq_ignore_ascii_case(team) {
            Some("@")
        } else if self.home_abbrev.eq_ignore_ascii_case(team) {
            Some("vs")
        } else {
            None
        }
    }

    pub fn is_ot_or_so(&self) -> bool {
        matches!(self.last_period.as_deref(), Some("OT") | Some("SO"))
    }
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
    let state_label = state_label(game.game_state.as_deref(), game.last_period.as_deref());
    let last_period = game.last_period.clone();
    let series_game = game.series_game.clone();
    let series_context = series_context(&game);

    ScheduleGameRow {
        game_id: game.game_id,
        date: game.date,
        game_type: game.game_type,
        away_abbrev: game.away_abbrev,
        home_abbrev: game.home_abbrev,
        start_time_utc: game.start_time_utc,
        away_score: game.away_score,
        home_score: game.home_score,
        away_score_str: game
            .away_score
            .map(|score| score.to_string())
            .unwrap_or_default(),
        home_score_str: game
            .home_score
            .map(|score| score.to_string())
            .unwrap_or_default(),
        state_label,
        last_period,
        home_or_away,
        opponent_abbrev: opponent,
        is_playoff: game.game_type == 3,
        series_game,
        series_context,
    }
}

fn series_context(game: &ScheduledGameInput) -> String {
    if game.game_type != 3 {
        return String::new();
    }

    let Some(series_game) = game.series_game.as_deref() else {
        return "Playoffs · Game ?".to_string();
    };
    let (Some(away_wins), Some(home_wins)) = (game.away_wins, game.home_wins) else {
        return format!("Playoffs · {series_game}");
    };
    format!(
        "{} {away_wins}–{home_wins} {} · {series_game}",
        game.away_abbrev, game.home_abbrev
    )
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
