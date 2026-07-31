//! MoneyPuck team game-by-game adapter for point-in-time xG and special teams.
//!
//! Only rows strictly before the requested cutoff are eligible. IceLines uses
//! MoneyPuck's score-and-venue-adjusted xG columns and retains the source seal
//! so historical forecasts can be replayed without reading later games.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckTrailingXgForm {
    pub team: String,
    pub before_date: NaiveDate,
    pub requested_games: usize,
    pub games: usize,
    pub latest_game_date: NaiveDate,
    pub score_venue_adjusted_xg_for: f64,
    pub score_venue_adjusted_xg_against: f64,
    pub xg_share: f64,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckOpponentAdjustedXgForm {
    pub team: String,
    pub before_date: NaiveDate,
    pub requested_games: usize,
    pub games: usize,
    pub latest_game_date: NaiveDate,
    /// A neutral .500 plus average game xG share above/below what each
    /// opponent's strictly-prior trailing form implied.
    pub adjusted_xg_share: f64,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckTrailingSpecialTeamsForm {
    pub team: String,
    pub before_date: NaiveDate,
    pub games: usize,
    pub latest_game_date: NaiveDate,
    pub power_play_xg_for_per_60: Option<f64>,
    pub penalty_kill_xg_against_per_60: Option<f64>,
    pub source_fingerprint: String,
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

pub fn derive_trailing_xg_form(
    rows: &[MoneyPuckTeamGameRow],
    team: &str,
    before_date: NaiveDate,
    requested_games: usize,
) -> Result<MoneyPuckTrailingXgForm, MoneyPuckTeamGameError> {
    let team = team.trim().to_ascii_uppercase();
    let selected = select_trailing_games(rows, &team, before_date, requested_games, "all")?;
    let xg_for = selected
        .iter()
        .map(|row| row.score_venue_adjusted_xg_for)
        .sum::<f64>();
    let xg_against = selected
        .iter()
        .map(|row| row.score_venue_adjusted_xg_against)
        .sum::<f64>();
    let total = xg_for + xg_against;
    let xg_share = if total > 0.0 { xg_for / total } else { 0.5 };
    Ok(MoneyPuckTrailingXgForm {
        team,
        before_date,
        requested_games,
        games: selected.len(),
        latest_game_date: selected.last().expect("eligible games").date,
        score_venue_adjusted_xg_for: xg_for,
        score_venue_adjusted_xg_against: xg_against,
        xg_share,
        source_fingerprint: fingerprint_rows(&selected),
    })
}

pub fn derive_opponent_adjusted_xg_form(
    rows: &[MoneyPuckTeamGameRow],
    rows_by_team: &BTreeMap<String, &[MoneyPuckTeamGameRow]>,
    team: &str,
    before_date: NaiveDate,
    requested_games: usize,
) -> Result<MoneyPuckOpponentAdjustedXgForm, MoneyPuckTeamGameError> {
    let team = team.trim().to_ascii_uppercase();
    let selected = select_trailing_games(rows, &team, before_date, requested_games, "all")?;
    let mut residuals = Vec::new();
    let mut opponent_seals = Vec::new();
    for row in &selected {
        let Some(opponent_rows) = rows_by_team.get(&row.opponent) else {
            continue;
        };
        let Ok(opponent_form) =
            derive_trailing_xg_form(opponent_rows, &row.opponent, row.date, requested_games)
        else {
            continue;
        };
        let total = row.score_venue_adjusted_xg_for + row.score_venue_adjusted_xg_against;
        let game_share = if total > 0.0 {
            row.score_venue_adjusted_xg_for / total
        } else {
            0.5
        };
        residuals.push(game_share - (1.0 - opponent_form.xg_share));
        opponent_seals.push((
            row.game_id,
            row.opponent.clone(),
            opponent_form.source_fingerprint,
        ));
    }
    if residuals.is_empty() {
        return Err(MoneyPuckTeamGameError::NoEligibleGames { team, before_date });
    }
    let adjusted_xg_share =
        (0.5 + residuals.iter().sum::<f64>() / residuals.len() as f64).clamp(0.0, 1.0);
    let sealed = (
        "opponent-adjusted-xg.v1",
        &team,
        before_date,
        requested_games,
        &selected,
        &opponent_seals,
    );
    let bytes = serde_json::to_vec(&sealed).expect("opponent-adjusted xG inputs serialize");
    Ok(MoneyPuckOpponentAdjustedXgForm {
        team,
        before_date,
        requested_games,
        games: residuals.len(),
        latest_game_date: selected.last().expect("eligible games").date,
        adjusted_xg_share,
        source_fingerprint: format!("sha256:{:x}", Sha256::digest(bytes)),
    })
}

pub fn derive_trailing_special_teams_form(
    rows: &[MoneyPuckTeamGameRow],
    team: &str,
    before_date: NaiveDate,
    requested_games: usize,
) -> Result<MoneyPuckTrailingSpecialTeamsForm, MoneyPuckTeamGameError> {
    let team = team.trim().to_ascii_uppercase();
    let all = select_trailing_games(rows, &team, before_date, requested_games, "all")?;
    let game_ids = all.iter().map(|row| row.game_id).collect::<BTreeSet<_>>();
    let situation = |name: &str| {
        rows.iter()
            .filter(|row| {
                row.team == team && game_ids.contains(&row.game_id) && row.situation == name
            })
            .collect::<Vec<_>>()
    };
    let power_play = situation("5on4");
    let penalty_kill = situation("4on5");
    let per_60 = |rows: &[&MoneyPuckTeamGameRow], against: bool| {
        let seconds = rows.iter().map(|row| row.ice_time_seconds).sum::<f64>();
        (seconds > 0.0).then(|| {
            rows.iter()
                .map(|row| {
                    if against {
                        row.score_venue_adjusted_xg_against
                    } else {
                        row.score_venue_adjusted_xg_for
                    }
                })
                .sum::<f64>()
                / seconds
                * 3_600.0
        })
    };
    let mut sealed = power_play.clone();
    sealed.extend(penalty_kill.iter().copied());
    sealed.sort_by_key(|row| (row.date, row.game_id, row.situation.clone()));
    Ok(MoneyPuckTrailingSpecialTeamsForm {
        team,
        before_date,
        games: all.len(),
        latest_game_date: all.last().expect("eligible games").date,
        power_play_xg_for_per_60: per_60(&power_play, false),
        penalty_kill_xg_against_per_60: per_60(&penalty_kill, true),
        source_fingerprint: fingerprint_rows(&sealed),
    })
}

pub fn moneypuck_team_game_url(team: &str) -> String {
    format!(
        "https://moneypuck.com/moneypuck/playerData/careers/gameByGame/regular/teams/{}.csv",
        team.trim().to_ascii_uppercase()
    )
}

fn select_trailing_games<'a>(
    rows: &'a [MoneyPuckTeamGameRow],
    team: &str,
    before_date: NaiveDate,
    requested_games: usize,
    situation: &str,
) -> Result<Vec<&'a MoneyPuckTeamGameRow>, MoneyPuckTeamGameError> {
    if requested_games == 0 {
        return Err(MoneyPuckTeamGameError::InvalidRow(
            "trailing window must contain at least one game".to_owned(),
        ));
    }
    let mut eligible = rows
        .iter()
        .filter(|row| row.team == team && row.date < before_date && row.situation == situation)
        .collect::<Vec<_>>();
    eligible.sort_by_key(|row| (std::cmp::Reverse(row.date), std::cmp::Reverse(row.game_id)));
    eligible.truncate(requested_games);
    if eligible.is_empty() {
        return Err(MoneyPuckTeamGameError::NoEligibleGames {
            team: team.to_owned(),
            before_date,
        });
    }
    eligible.sort_by_key(|row| (row.date, row.game_id));
    Ok(eligible)
}

fn fingerprint_rows(rows: &[&MoneyPuckTeamGameRow]) -> String {
    let bytes = serde_json::to_vec(rows).expect("MoneyPuck rows are serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "team,season,gameId,playerTeam,opposingTeam,home_or_away,gameDate,situation,iceTime,scoreVenueAdjustedxGoalsFor,scoreVenueAdjustedxGoalsAgainst\n";

    fn csv() -> String {
        format!(
            "{HEADER}NYR,2025,1,NYR,SEA,HOME,20251001,all,3600,3.0,2.0\n\
             NYR,2025,1,NYR,SEA,HOME,20251001,5on4,300,0.6,0.1\n\
             NYR,2025,1,NYR,SEA,HOME,20251001,4on5,240,0.1,0.5\n\
             NYR,2025,2,NYR,BOS,AWAY,20251003,all,3600,1.0,3.0\n\
             NYR,2025,2,NYR,BOS,AWAY,20251003,5on4,240,0.2,0.1\n\
             NYR,2025,2,NYR,BOS,AWAY,20251003,4on5,300,0.1,0.8\n\
             NYR,2025,3,NYR,NJD,HOME,20251005,all,3600,8.0,1.0\n"
        )
    }

    #[test]
    fn l0_parser_binds_score_venue_adjusted_columns() {
        let rows = parse_moneypuck_team_games(&csv()).unwrap();
        assert_eq!(rows[0].season, 20_252_026);
        assert_eq!(rows.len(), 7);
    }

    #[test]
    fn l0_trailing_xg_is_strictly_before_cutoff() {
        let rows = parse_moneypuck_team_games(&csv()).unwrap();
        let form = derive_trailing_xg_form(
            &rows,
            "NYR",
            NaiveDate::from_ymd_opt(2025, 10, 5).unwrap(),
            10,
        )
        .unwrap();
        assert_eq!(form.games, 2);
        assert!((form.xg_share - 4.0 / 9.0).abs() < 1e-12);
    }

    #[test]
    fn l0_opponent_adjustment_uses_only_each_opponents_prior_games() {
        let rows = parse_moneypuck_team_games(&csv()).unwrap();
        let opponent_csv = format!(
            "{HEADER}SEA,2025,90,SEA,BOS,HOME,20250928,all,3600,1.0,3.0\n\
             SEA,2025,91,SEA,BOS,HOME,20251002,all,3600,9.0,1.0\n\
             BOS,2025,92,BOS,SEA,HOME,20250929,all,3600,2.0,2.0\n"
        );
        let sea = parse_moneypuck_team_games(&opponent_csv)
            .unwrap()
            .into_iter()
            .filter(|row| row.team == "SEA")
            .collect::<Vec<_>>();
        let bos = parse_moneypuck_team_games(&opponent_csv)
            .unwrap()
            .into_iter()
            .filter(|row| row.team == "BOS")
            .collect::<Vec<_>>();
        let by_team = BTreeMap::from([
            ("SEA".to_owned(), sea.as_slice()),
            ("BOS".to_owned(), bos.as_slice()),
        ]);
        let form = derive_opponent_adjusted_xg_form(
            &rows,
            &by_team,
            "NYR",
            NaiveDate::from_ymd_opt(2025, 10, 5).unwrap(),
            10,
        )
        .unwrap();
        assert_eq!(form.games, 2);
        // NYR generated .600 against a SEA team whose prior xG share was .250
        // (implied opponent share .750), then .250 against a neutral BOS prior.
        // The SEA 9-1 row after their meeting is strictly excluded.
        assert!((form.adjusted_xg_share - 0.3).abs() < 1e-12);
    }

    #[test]
    fn l0_special_teams_rates_use_the_same_frozen_games() {
        let rows = parse_moneypuck_team_games(&csv()).unwrap();
        let form = derive_trailing_special_teams_form(
            &rows,
            "NYR",
            NaiveDate::from_ymd_opt(2025, 10, 5).unwrap(),
            2,
        )
        .unwrap();
        assert!((form.power_play_xg_for_per_60.unwrap() - 5.3333333333).abs() < 1e-8);
        assert!((form.penalty_kill_xg_against_per_60.unwrap() - 8.6666666667).abs() < 1e-8);
    }

    #[test]
    fn l0_schema_drift_fails_closed() {
        let error = parse_moneypuck_team_games("team,season\nNYR,2025\n").unwrap_err();
        assert!(matches!(error, MoneyPuckTeamGameError::MissingColumns(_)));
    }
}
