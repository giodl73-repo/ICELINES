use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const REQUIRED_COLUMNS: &[&str] = &[
    "playerId",
    "gameId",
    "playerTeam",
    "opposingTeam",
    "gameDate",
    "position",
    "situation",
    "icetime",
    "OnIce_F_scoreVenueAdjustedxGoals",
    "OnIce_A_scoreVenueAdjustedxGoals",
    "I_F_oZoneShiftStarts",
    "I_F_dZoneShiftStarts",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckSkaterGameRow {
    pub player_id: u32,
    pub season: u32,
    pub game_id: u64,
    pub date: NaiveDate,
    pub team: String,
    pub opponent: String,
    pub position: String,
    pub situation: String,
    pub ice_time_seconds: f64,
    pub score_venue_adjusted_on_ice_xg_for: f64,
    pub score_venue_adjusted_on_ice_xg_against: f64,
    pub offensive_zone_shift_starts: f64,
    pub defensive_zone_shift_starts: f64,
}

#[derive(Debug, Error)]
pub enum MoneyPuckSkaterGameError {
    #[error("MoneyPuck skater game CSV missing required column(s): {0}")]
    MissingColumns(String),
    #[error("MoneyPuck skater game CSV parse error: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid MoneyPuck skater game row: {0}")]
    InvalidRow(String),
}

#[derive(Debug, Deserialize)]
struct RawRow {
    #[serde(rename = "playerId")]
    player_id: u32,
    #[serde(rename = "gameId")]
    game_id: u64,
    #[serde(rename = "playerTeam")]
    player_team: String,
    #[serde(rename = "opposingTeam")]
    opposing_team: String,
    #[serde(rename = "gameDate")]
    game_date: u32,
    position: String,
    situation: String,
    icetime: f64,
    #[serde(rename = "OnIce_F_scoreVenueAdjustedxGoals")]
    adjusted_xg_for: f64,
    #[serde(rename = "OnIce_A_scoreVenueAdjustedxGoals")]
    adjusted_xg_against: f64,
    #[serde(rename = "I_F_oZoneShiftStarts")]
    offensive_zone_shift_starts: f64,
    #[serde(rename = "I_F_dZoneShiftStarts")]
    defensive_zone_shift_starts: f64,
}

pub fn parse_moneypuck_skater_games(
    csv_text: &str,
) -> Result<Vec<MoneyPuckSkaterGameRow>, MoneyPuckSkaterGameError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers()?;
    let missing = REQUIRED_COLUMNS
        .iter()
        .filter(|column| !headers.iter().any(|header| header == **column))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(MoneyPuckSkaterGameError::MissingColumns(missing.join(", ")));
    }
    let mut rows = Vec::new();
    let mut identities = BTreeSet::new();
    for raw in reader.deserialize::<RawRow>() {
        let raw = raw?;
        let date = NaiveDate::parse_from_str(&raw.game_date.to_string(), "%Y%m%d")
            .map_err(|_| MoneyPuckSkaterGameError::InvalidRow("invalid gameDate".to_owned()))?;
        let team = raw.player_team.trim().to_ascii_uppercase();
        let situation = raw.situation.trim().to_ascii_lowercase();
        let measures = [
            raw.icetime,
            raw.adjusted_xg_for,
            raw.adjusted_xg_against,
            raw.offensive_zone_shift_starts,
            raw.defensive_zone_shift_starts,
        ];
        if raw.player_id == 0
            || team.is_empty()
            || raw.opposing_team.trim().is_empty()
            || measures
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
            || !identities.insert((raw.player_id, raw.game_id, situation.clone()))
        {
            return Err(MoneyPuckSkaterGameError::InvalidRow(format!(
                "invalid or duplicate player/game/situation row for player {} game {}",
                raw.player_id, raw.game_id
            )));
        }
        let start_year = raw
            .game_id
            .to_string()
            .get(..4)
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| {
                MoneyPuckSkaterGameError::InvalidRow(format!("invalid NHL game ID {}", raw.game_id))
            })?;
        rows.push(MoneyPuckSkaterGameRow {
            player_id: raw.player_id,
            season: start_year * 10_000 + start_year + 1,
            game_id: raw.game_id,
            date,
            team,
            opponent: raw.opposing_team.trim().to_ascii_uppercase(),
            position: raw.position.trim().to_ascii_uppercase(),
            situation,
            ice_time_seconds: raw.icetime,
            score_venue_adjusted_on_ice_xg_for: raw.adjusted_xg_for,
            score_venue_adjusted_on_ice_xg_against: raw.adjusted_xg_against,
            offensive_zone_shift_starts: raw.offensive_zone_shift_starts,
            defensive_zone_shift_starts: raw.defensive_zone_shift_starts,
        });
    }
    rows.sort_by_key(|row| (row.date, row.game_id, row.situation.clone()));
    Ok(rows)
}

pub fn moneypuck_skater_career_game_url(player_id: u32) -> String {
    format!(
        "https://moneypuck.com/moneypuck/playerData/careers/gameByGame/regular/skaters/{player_id}.csv"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "playerId,gameId,playerTeam,opposingTeam,gameDate,position,situation,icetime,OnIce_F_scoreVenueAdjustedxGoals,OnIce_A_scoreVenueAdjustedxGoals,I_F_oZoneShiftStarts,I_F_dZoneShiftStarts\n";

    #[test]
    fn parses_adjusted_xg_and_deployment_context() {
        let csv = format!("{HEADER}8478402,2025020006,EDM,CGY,20251008,C,5on5,1200,1.3,0.8,8,5\n");
        let rows = parse_moneypuck_skater_games(&csv).expect("valid skater game");
        assert_eq!(rows[0].player_id, 8_478_402);
        assert_eq!(rows[0].season, 20_252_026);
        assert_eq!(rows[0].offensive_zone_shift_starts, 8.0);
    }

    #[test]
    fn rejects_schema_drift_and_duplicates() {
        assert!(matches!(
            parse_moneypuck_skater_games("playerId,gameId\n1,2\n"),
            Err(MoneyPuckSkaterGameError::MissingColumns(_))
        ));
        let csv = format!(
            "{HEADER}8478402,2025020006,EDM,CGY,20251008,C,5on5,1200,1.3,0.8,8,5\n\
             8478402,2025020006,EDM,CGY,20251008,C,5on5,1200,1.3,0.8,8,5\n"
        );
        assert!(parse_moneypuck_skater_games(&csv).is_err());
    }
}
