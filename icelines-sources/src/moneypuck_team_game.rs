use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REQUIRED_COLUMNS: &[&str] = &[
    "team",
    "season",
    "gameId",
    "playerTeam",
    "opposingTeam",
    "home_or_away",
    "gameDate",
    "situation",
    "iceTime",
    "scoreVenueAdjustedxGoalsFor",
    "scoreVenueAdjustedxGoalsAgainst",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckTeamGameRow {
    pub season: u32,
    pub game_id: u64,
    pub date: NaiveDate,
    pub team: String,
    pub opponent: String,
    pub home: bool,
    pub situation: String,
    pub ice_time_seconds: f64,
    pub score_venue_adjusted_xg_for: f64,
    pub score_venue_adjusted_xg_against: f64,
}

#[derive(Debug, Error)]
pub enum MoneyPuckTeamGameError {
    #[error("MoneyPuck team game CSV missing required column(s): {0}")]
    MissingColumns(String),
    #[error("MoneyPuck team game CSV parse error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid MoneyPuck team game row: {0}")]
    InvalidRow(String),
    #[error("no eligible MoneyPuck games for {team} before {before_date}")]
    NoEligibleGames {
        team: String,
        before_date: NaiveDate,
    },
}

#[derive(Debug, Deserialize)]
struct RawRow {
    team: String,
    season: u32,
    #[serde(rename = "gameId")]
    game_id: u64,
    #[serde(rename = "playerTeam")]
    player_team: String,
    #[serde(rename = "opposingTeam")]
    opposing_team: String,
    home_or_away: String,
    #[serde(rename = "gameDate")]
    game_date: u32,
    situation: String,
    #[serde(rename = "iceTime")]
    ice_time: f64,
    #[serde(rename = "scoreVenueAdjustedxGoalsFor")]
    adjusted_xg_for: f64,
    #[serde(rename = "scoreVenueAdjustedxGoalsAgainst")]
    adjusted_xg_against: f64,
}

pub fn parse_moneypuck_team_games(
    csv_text: &str,
) -> Result<Vec<MoneyPuckTeamGameRow>, MoneyPuckTeamGameError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers()?;
    let missing = REQUIRED_COLUMNS
        .iter()
        .filter(|column| !headers.iter().any(|header| header == **column))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(MoneyPuckTeamGameError::MissingColumns(missing.join(", ")));
    }
    let mut rows = Vec::new();
    let mut identities = BTreeSet::new();
    for raw in reader.deserialize::<RawRow>() {
        let raw = raw?;
        let date = NaiveDate::parse_from_str(&raw.game_date.to_string(), "%Y%m%d")
            .map_err(|_| MoneyPuckTeamGameError::InvalidRow("invalid gameDate".to_owned()))?;
        let team = raw.player_team.trim().to_ascii_uppercase();
        if raw.team.trim().to_ascii_uppercase() != team
            || team.is_empty()
            || raw.opposing_team.trim().is_empty()
            || !raw.ice_time.is_finite()
            || raw.ice_time < 0.0
            || !raw.adjusted_xg_for.is_finite()
            || raw.adjusted_xg_for < 0.0
            || !raw.adjusted_xg_against.is_finite()
            || raw.adjusted_xg_against < 0.0
        {
            return Err(MoneyPuckTeamGameError::InvalidRow(format!(
                "invalid values for game {}",
                raw.game_id
            )));
        }
        let home = match raw.home_or_away.trim().to_ascii_uppercase().as_str() {
            "HOME" => true,
            "AWAY" => false,
            _ => {
                return Err(MoneyPuckTeamGameError::InvalidRow(format!(
                    "invalid home_or_away for game {}",
                    raw.game_id
                )))
            }
        };
        let situation = raw.situation.trim().to_ascii_lowercase();
        if !identities.insert((raw.game_id, team.clone(), situation.clone())) {
            return Err(MoneyPuckTeamGameError::InvalidRow(format!(
                "duplicate game/team/situation row for game {}",
                raw.game_id
            )));
        }
        rows.push(MoneyPuckTeamGameRow {
            season: raw.season * 10_000 + raw.season + 1,
            game_id: raw.game_id,
            date,
            team,
            opponent: raw.opposing_team.trim().to_ascii_uppercase(),
            home,
            situation,
            ice_time_seconds: raw.ice_time,
            score_venue_adjusted_xg_for: raw.adjusted_xg_for,
            score_venue_adjusted_xg_against: raw.adjusted_xg_against,
        });
    }
    rows.sort_by_key(|row| (row.date, row.game_id, row.situation.clone()));
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::parse_moneypuck_team_games;

    const HEADER: &str = "team,season,gameId,playerTeam,opposingTeam,home_or_away,gameDate,situation,iceTime,scoreVenueAdjustedxGoalsFor,scoreVenueAdjustedxGoalsAgainst\n";

    #[test]
    fn normalizes_team_game_rows() {
        let rows = parse_moneypuck_team_games(&format!(
            "{HEADER}nyr,2025,1,NYR,sea,HOME,20251001,ALL,3600,3.0,2.0\n"
        ))
        .expect("valid row");
        assert_eq!(rows[0].season, 20_252_026);
        assert_eq!(rows[0].team, "NYR");
        assert_eq!(rows[0].opponent, "SEA");
        assert_eq!(rows[0].situation, "all");
    }

    #[test]
    fn rejects_duplicate_game_team_situation() {
        let csv = format!(
            "{HEADER}NYR,2025,1,NYR,SEA,HOME,20251001,all,3600,3.0,2.0\n\
             NYR,2025,1,NYR,SEA,HOME,20251001,all,3600,3.0,2.0\n"
        );
        assert!(parse_moneypuck_team_games(&csv).is_err());
    }
}
