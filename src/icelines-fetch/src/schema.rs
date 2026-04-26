use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PagedResponse<T> {
    pub data: Vec<T>,
    pub total: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkaterBio {
    pub player_id: u32,
    pub skater_full_name: String,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub current_team_abbrev: String,
    pub position_code: String,
    pub birth_date: Option<String>,
    pub birth_country: Option<String>,
    pub nationality_code: Option<String>,
    pub shoots_catches: Option<String>,
    pub draft_year: Option<u32>,
    pub draft_round: Option<u32>,
    pub draft_overall: Option<u32>,
    pub birth_city: Option<String>,
    pub birth_state_province_code: Option<String>,
    pub height: Option<u32>,
    pub weight: Option<u32>,
    pub first_season_for_game_type: Option<u32>,
    pub is_in_hall_of_fame_yn: Option<String>,
    pub last_name: String,
    pub points: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkaterStats {
    pub player_id: u32,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: f32,
    pub pp_goals: u32,
    pub pp_points: u32,
    pub sh_goals: u32,
    pub sh_points: u32,
    pub game_winning_goals: u32,
    pub ot_goals: u32,
    pub shots: u32,
    pub shooting_pctg: f32,
    pub plus_minus: i32,
    pub time_on_ice_per_game: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RosterResponse {
    pub forwards: Vec<RosterPlayer>,
    pub defensemen: Vec<RosterPlayer>,
    pub goalies: Vec<RosterPlayer>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterPlayer {
    pub id: u32,
    pub first_name: LocalizedString,
    pub last_name: LocalizedString,
    pub sweater_number: Option<u32>,
    pub position_code: String,
    pub shoots_catches: Option<String>,
    pub birth_date: Option<String>,
    pub birth_country: Option<String>,
    pub height_in_inches: Option<u32>,
    pub weight_in_pounds: Option<u32>,
    pub headshot: Option<String>,
    pub birth_city: Option<LocalizedString>,
    pub birth_state_province: Option<LocalizedString>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LocalizedString {
    Plain(String),
    Localized { default: String },
}

impl LocalizedString {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Localized { default } => default,
        }
    }
}

impl std::fmt::Display for LocalizedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
