use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REQUIRED_COLUMNS: &[&str] = &[
    "lineId",
    "name",
    "gameId",
    "playerTeam",
    "opposingTeam",
    "home_or_away",
    "gameDate",
    "position",
    "situation",
    "icetime",
    "scoreVenueAdjustedxGoalsFor",
    "scoreVenueAdjustedxGoalsAgainst",
];

/// One game of MoneyPuck pair/trio results. Player IDs come from `lineId`,
/// which concatenates the two or three seven-digit NHL player IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckLineGameRow {
    pub line_id: String,
    pub player_ids: Vec<u32>,
    pub name: String,
    pub season: u32,
    pub game_id: u64,
    pub date: NaiveDate,
    pub team: String,
    pub opponent: String,
    pub home: bool,
    pub position: String,
    pub situation: String,
    pub ice_time_seconds: f64,
    pub score_venue_adjusted_xg_for: f64,
    pub score_venue_adjusted_xg_against: f64,
}

#[derive(Debug, Error)]
pub enum MoneyPuckLineGameError {
    #[error("MoneyPuck line-game CSV missing required column(s): {0}")]
    MissingColumns(String),
    #[error("MoneyPuck line-game CSV parse error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid MoneyPuck line-game row: {0}")]
    InvalidRow(String),
}

#[derive(Debug, Deserialize)]
struct RawRow {
    #[serde(rename = "lineId")]
    line_id: String,
    name: String,
    #[serde(rename = "gameId")]
    game_id: u64,
    #[serde(rename = "playerTeam")]
    player_team: String,
    #[serde(rename = "opposingTeam")]
    opposing_team: String,
    home_or_away: String,
    #[serde(rename = "gameDate")]
    game_date: u32,
    position: String,
    situation: String,
    icetime: f64,
    #[serde(rename = "scoreVenueAdjustedxGoalsFor")]
    adjusted_xg_for: f64,
    #[serde(rename = "scoreVenueAdjustedxGoalsAgainst")]
    adjusted_xg_against: f64,
}

pub fn parse_moneypuck_line_games(
    csv_text: &str,
) -> Result<Vec<MoneyPuckLineGameRow>, MoneyPuckLineGameError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers()?;
    let missing = REQUIRED_COLUMNS
        .iter()
        .filter(|column| !headers.iter().any(|header| header == **column))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(MoneyPuckLineGameError::MissingColumns(missing.join(", ")));
    }

    let mut rows = Vec::new();
    let mut identities = BTreeSet::new();
    for raw in reader.deserialize::<RawRow>() {
        let raw = raw?;
        let date = NaiveDate::parse_from_str(&raw.game_date.to_string(), "%Y%m%d")
            .map_err(|_| MoneyPuckLineGameError::InvalidRow("invalid gameDate".to_owned()))?;
        let player_ids = parse_line_id(&raw.line_id)?;
        let expected_players = match raw.position.trim().to_ascii_lowercase().as_str() {
            "line" => 3,
            "pairing" => 2,
            value => {
                return Err(MoneyPuckLineGameError::InvalidRow(format!(
                    "unsupported position {value} for game {}",
                    raw.game_id
                )))
            }
        };
        if player_ids.len() != expected_players
            || raw.name.split('-').count() != expected_players
            || raw.player_team.trim().is_empty()
            || raw.opposing_team.trim().is_empty()
            || !raw.icetime.is_finite()
            || raw.icetime < 0.0
            || !raw.adjusted_xg_for.is_finite()
            || raw.adjusted_xg_for < 0.0
            || !raw.adjusted_xg_against.is_finite()
            || raw.adjusted_xg_against < 0.0
        {
            return Err(MoneyPuckLineGameError::InvalidRow(format!(
                "invalid values for game {} line {}",
                raw.game_id, raw.line_id
            )));
        }
        let home = match raw.home_or_away.trim().to_ascii_uppercase().as_str() {
            "HOME" => true,
            "AWAY" => false,
            _ => {
                return Err(MoneyPuckLineGameError::InvalidRow(format!(
                    "invalid home_or_away for game {}",
                    raw.game_id
                )))
            }
        };
        let situation = raw.situation.trim().to_ascii_lowercase();
        if !identities.insert((raw.game_id, situation.clone())) {
            return Err(MoneyPuckLineGameError::InvalidRow(format!(
                "duplicate game/situation row for game {} line {}",
                raw.game_id, raw.line_id
            )));
        }
        let start_year = raw
            .game_id
            .to_string()
            .get(..4)
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| {
                MoneyPuckLineGameError::InvalidRow(format!("invalid NHL game ID {}", raw.game_id))
            })?;
        rows.push(MoneyPuckLineGameRow {
            line_id: raw.line_id,
            player_ids,
            name: raw.name.trim().to_owned(),
            season: start_year * 10_000 + start_year + 1,
            game_id: raw.game_id,
            date,
            team: raw.player_team.trim().to_ascii_uppercase(),
            opponent: raw.opposing_team.trim().to_ascii_uppercase(),
            home,
            position: raw.position.trim().to_ascii_lowercase(),
            situation,
            ice_time_seconds: raw.icetime,
            score_venue_adjusted_xg_for: raw.adjusted_xg_for,
            score_venue_adjusted_xg_against: raw.adjusted_xg_against,
        });
    }
    rows.sort_by_key(|row| (row.date, row.game_id, row.situation.clone()));
    Ok(rows)
}

pub fn moneypuck_line_game_url(season_start_year: u32, line_id: &str) -> String {
    format!(
        "https://moneypuck.com/moneypuck/playerData/lineGameByGame/{season_start_year}/regular/{line_id}.csv"
    )
}

fn parse_line_id(value: &str) -> Result<Vec<u32>, MoneyPuckLineGameError> {
    let value = value.trim();
    if !matches!(value.len(), 14 | 21) || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(MoneyPuckLineGameError::InvalidRow(format!(
            "lineId must concatenate two or three seven-digit NHL player IDs: {value}"
        )));
    }
    let mut ids = value
        .as_bytes()
        .chunks_exact(7)
        .map(|chunk| {
            std::str::from_utf8(chunk)
                .ok()
                .and_then(|part| part.parse::<u32>().ok())
                .ok_or_else(|| MoneyPuckLineGameError::InvalidRow("invalid lineId".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort_unstable();
    if ids.contains(&0) || ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(MoneyPuckLineGameError::InvalidRow(
            "lineId contains an invalid or duplicate player ID".to_owned(),
        ));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "lineId,name,gameId,playerTeam,opposingTeam,home_or_away,gameDate,position,situation,icetime,scoreVenueAdjustedxGoalsFor,scoreVenueAdjustedxGoalsAgainst\n";

    #[test]
    fn parses_chronological_trio_rows_and_stable_ids() {
        let csv = format!(
            "{HEADER}847798784816248484144,Donato-Bedard-Mikheyev,2025020008,CHI,BOS,AWAY,20251009,line,5on5,102,0.24,0.71\n"
        );
        let rows = parse_moneypuck_line_games(&csv).expect("valid line-game row");
        assert_eq!(rows[0].player_ids, vec![8_477_987, 8_481_624, 8_484_144]);
        assert_eq!(rows[0].season, 20_252_026);
        assert_eq!(rows[0].situation, "5on5");
    }

    #[test]
    fn rejects_schema_drift_and_identity_ambiguity() {
        assert!(matches!(
            parse_moneypuck_line_games("lineId,name\n1,A-B\n"),
            Err(MoneyPuckLineGameError::MissingColumns(_))
        ));
        let csv = format!(
            "{HEADER}84779878481624,Only-One,2025020008,CHI,BOS,AWAY,20251009,line,5on5,102,0.24,0.71\n"
        );
        assert!(parse_moneypuck_line_games(&csv).is_err());
    }
}
