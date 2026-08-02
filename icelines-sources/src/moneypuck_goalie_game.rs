use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REQUIRED_COLUMNS: &[&str] = &[
    "playerId",
    "season",
    "gameId",
    "playerTeam",
    "opposingTeam",
    "gameDate",
    "situation",
    "icetime",
    "xGoals",
    "goals",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckGoalieGameRow {
    pub player_id: u32,
    pub season: u32,
    pub game_id: u64,
    pub date: NaiveDate,
    pub team: String,
    pub opponent: String,
    pub situation: String,
    pub ice_time_seconds: f64,
    pub expected_goals_against: f64,
    pub goals_against: f64,
}

#[derive(Debug, Error)]
pub enum MoneyPuckGoalieGameError {
    #[error("MoneyPuck goalie game CSV missing required column(s): {0}")]
    MissingColumns(String),
    #[error("MoneyPuck goalie game CSV parse error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid MoneyPuck goalie game row: {0}")]
    InvalidRow(String),
    #[error("no eligible MoneyPuck appearances for goalie {player_id} before {before_date}")]
    NoEligibleAppearances {
        player_id: u32,
        before_date: NaiveDate,
    },
}

#[derive(Debug, Deserialize)]
struct RawRow {
    #[serde(rename = "playerId")]
    player_id: u32,
    season: u32,
    #[serde(rename = "gameId")]
    game_id: u64,
    #[serde(rename = "playerTeam")]
    player_team: String,
    #[serde(rename = "opposingTeam")]
    opposing_team: String,
    #[serde(rename = "gameDate")]
    game_date: u32,
    situation: String,
    icetime: f64,
    #[serde(rename = "xGoals")]
    expected_goals: f64,
    goals: f64,
}

pub fn parse_moneypuck_goalie_games(
    csv_text: &str,
) -> Result<Vec<MoneyPuckGoalieGameRow>, MoneyPuckGoalieGameError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers()?;
    let missing = REQUIRED_COLUMNS
        .iter()
        .filter(|column| !headers.iter().any(|header| header == **column))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(MoneyPuckGoalieGameError::MissingColumns(missing.join(", ")));
    }
    let mut rows = Vec::new();
    let mut identities = BTreeSet::new();
    let mut player_id = None;
    for raw in reader.deserialize::<RawRow>() {
        let raw = raw?;
        let date = NaiveDate::parse_from_str(&raw.game_date.to_string(), "%Y%m%d")
            .map_err(|_| MoneyPuckGoalieGameError::InvalidRow("invalid gameDate".to_owned()))?;
        let team = raw.player_team.trim().to_ascii_uppercase();
        let opponent = raw.opposing_team.trim().to_ascii_uppercase();
        let situation = raw.situation.trim().to_ascii_lowercase();
        if raw.player_id == 0
            || player_id.is_some_and(|id| id != raw.player_id)
            || raw.game_id == 0
            || team.is_empty()
            || opponent.is_empty()
            || !raw.icetime.is_finite()
            || raw.icetime < 0.0
            || !raw.expected_goals.is_finite()
            || raw.expected_goals < 0.0
            || !raw.goals.is_finite()
            || raw.goals < 0.0
            || !identities.insert((raw.game_id, situation.clone()))
        {
            return Err(MoneyPuckGoalieGameError::InvalidRow(format!(
                "invalid values or duplicate situation for game {}",
                raw.game_id
            )));
        }
        player_id = Some(raw.player_id);
        rows.push(MoneyPuckGoalieGameRow {
            player_id: raw.player_id,
            season: raw.season * 10_000 + raw.season + 1,
            game_id: raw.game_id,
            date,
            team,
            opponent,
            situation,
            ice_time_seconds: raw.icetime,
            expected_goals_against: raw.expected_goals,
            goals_against: raw.goals,
        });
    }
    rows.sort_by_key(|row| (row.date, row.game_id, row.situation.clone()));
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::parse_moneypuck_goalie_games;

    const HEADER: &str =
        "playerId,season,gameId,playerTeam,opposingTeam,gameDate,situation,icetime,xGoals,goals\n";

    #[test]
    fn normalizes_goalie_game_rows() {
        let rows = parse_moneypuck_goalie_games(&format!(
            "{HEADER}8478048,2025,1,nyr,sea,20251001,ALL,3600,2.5,2\n"
        ))
        .expect("valid goalie row");
        assert_eq!(rows[0].season, 20_252_026);
        assert_eq!(rows[0].team, "NYR");
        assert_eq!(rows[0].opponent, "SEA");
        assert_eq!(rows[0].situation, "all");
    }

    #[test]
    fn rejects_mixed_goalie_identity() {
        let csv = format!(
            "{HEADER}1,2025,1,NYR,SEA,20251001,all,3600,2,2\n\
             2,2025,2,NYR,BOS,20251002,all,3600,2,2\n"
        );
        assert!(parse_moneypuck_goalie_games(&csv).is_err());
    }
}
