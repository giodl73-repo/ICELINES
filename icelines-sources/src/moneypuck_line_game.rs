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

const SUMMARY_REQUIRED_COLUMNS: &[&str] = &[
    "lineId",
    "season",
    "name",
    "team",
    "position",
    "situation",
    "games_played",
    "icetime",
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckLineSummaryRow {
    pub line_id: String,
    pub player_ids: Vec<u32>,
    pub name: String,
    pub season: u32,
    pub team: String,
    pub position: String,
    pub situation: String,
    pub games_played: u32,
    pub ice_time_seconds: f64,
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

#[derive(Debug, Deserialize)]
struct RawSummaryRow {
    #[serde(rename = "lineId")]
    line_id: String,
    season: u32,
    name: String,
    team: String,
    position: String,
    situation: String,
    games_played: u32,
    icetime: f64,
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
        let player_ids = parse_moneypuck_line_id(&raw.line_id)?;
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

pub fn moneypuck_line_summary_url(season_start_year: u32) -> String {
    format!(
        "https://moneypuck.com/moneypuck/playerData/seasonSummary/{season_start_year}/regular/lines.csv"
    )
}

pub fn parse_moneypuck_line_summary(
    csv_text: &str,
) -> Result<Vec<MoneyPuckLineSummaryRow>, MoneyPuckLineGameError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers()?;
    let missing = SUMMARY_REQUIRED_COLUMNS
        .iter()
        .filter(|column| !headers.iter().any(|header| header == **column))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(MoneyPuckLineGameError::MissingColumns(missing.join(", ")));
    }
    let mut rows = Vec::new();
    let mut identities = BTreeSet::new();
    for raw in reader.deserialize::<RawSummaryRow>() {
        let raw = raw?;
        let player_ids = parse_moneypuck_line_id(&raw.line_id)?;
        let position = raw.position.trim().to_ascii_lowercase();
        let expected_players = match position.as_str() {
            "line" => 3,
            "pairing" => 2,
            value => {
                return Err(MoneyPuckLineGameError::InvalidRow(format!(
                    "unsupported summary position {value}"
                )))
            }
        };
        let team = raw.team.trim().to_ascii_uppercase();
        let situation = raw.situation.trim().to_ascii_lowercase();
        if player_ids.len() != expected_players
            || team.is_empty()
            || raw.games_played == 0
            || !raw.icetime.is_finite()
            || raw.icetime <= 0.0
            || !identities.insert((raw.line_id.clone(), team.clone(), situation.clone()))
        {
            return Err(MoneyPuckLineGameError::InvalidRow(format!(
                "invalid or duplicate summary line {}",
                raw.line_id
            )));
        }
        rows.push(MoneyPuckLineSummaryRow {
            line_id: raw.line_id,
            player_ids,
            name: raw.name.trim().to_owned(),
            season: raw.season * 10_000 + raw.season + 1,
            team,
            position,
            situation,
            games_played: raw.games_played,
            ice_time_seconds: raw.icetime,
        });
    }
    rows.sort_by(|left, right| {
        left.team
            .cmp(&right.team)
            .then_with(|| right.ice_time_seconds.total_cmp(&left.ice_time_seconds))
            .then_with(|| left.line_id.cmp(&right.line_id))
    });
    Ok(rows)
}

pub fn parse_moneypuck_line_id(value: &str) -> Result<Vec<u32>, MoneyPuckLineGameError> {
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

    #[test]
    fn discovers_team_units_from_season_summary() {
        let csv = "lineId,season,name,team,position,situation,games_played,icetime\n\
                   847798784816248484144,2025,Donato-Bedard-Mikheyev,CHI,line,5on5,15,783\n\
                   84801968484305,2025,Bryson-Metsa,BUF,pairing,5on5,11,5189\n";
        let rows = parse_moneypuck_line_summary(csv).expect("valid line summary");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].team, "BUF");
        assert_eq!(rows[0].player_ids, vec![8_480_196, 8_484_305]);
        assert_eq!(rows[1].season, 20_252_026);
    }

    #[test]
    fn stable_ids_handle_hyphenated_player_names() {
        let summary = "lineId,season,name,team,position,situation,games_played,icetime\n\
                       84751718482174,2025,Ekman-Larsson-Villeneuve,TOR,pairing,5on5,3,2562\n";
        let rows = parse_moneypuck_line_summary(summary).expect("hyphenated surname is valid");
        assert_eq!(rows[0].player_ids, vec![8_475_171, 8_482_174]);
    }
}
