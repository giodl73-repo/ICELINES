//! Point-in-time MoneyPuck goalie form and workload evidence.
//!
//! Career files are retrieved later for historical reconstruction, but only
//! `all`-situation appearances strictly before the requested game date may
//! contribute. The derived fingerprint seals exactly those eligible rows.

use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckTrailingGoalieForm {
    pub player_id: u32,
    pub before_date: NaiveDate,
    pub requested_appearances: usize,
    pub appearances: usize,
    pub latest_appearance_date: NaiveDate,
    pub days_rest: i64,
    pub minutes_last_seven_days: f64,
    pub appearances_last_seven_days: usize,
    pub goals_saved_above_expected: f64,
    pub gsax_per_60: f64,
    /// Bounded model input: 50 is neutral and 25 points represent 1 GSAx/60.
    pub form_quality: f64,
    /// Bounded model input: 100 is fully rested; recent workload lowers it.
    pub workload_readiness: f64,
    pub source_fingerprint: String,
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

pub fn derive_trailing_goalie_form(
    rows: &[MoneyPuckGoalieGameRow],
    player_id: u32,
    before_date: NaiveDate,
    requested_appearances: usize,
) -> Result<MoneyPuckTrailingGoalieForm, MoneyPuckGoalieGameError> {
    if player_id == 0 || requested_appearances == 0 {
        return Err(MoneyPuckGoalieGameError::InvalidRow(
            "goalie ID and trailing window must be non-zero".to_owned(),
        ));
    }
    let mut eligible = rows
        .iter()
        .filter(|row| {
            row.player_id == player_id && row.situation == "all" && row.date < before_date
        })
        .collect::<Vec<_>>();
    eligible.sort_by_key(|row| (row.date, row.game_id));
    let latest =
        eligible
            .last()
            .copied()
            .ok_or(MoneyPuckGoalieGameError::NoEligibleAppearances {
                player_id,
                before_date,
            })?;
    let trailing_start = eligible.len().saturating_sub(requested_appearances);
    let trailing = &eligible[trailing_start..];
    let workload = eligible
        .iter()
        .copied()
        .filter(|row| (before_date - row.date).num_days() <= 7)
        .collect::<Vec<_>>();
    let ice_time_seconds = trailing.iter().map(|row| row.ice_time_seconds).sum::<f64>();
    let goals_saved_above_expected = trailing
        .iter()
        .map(|row| row.expected_goals_against - row.goals_against)
        .sum::<f64>();
    let gsax_per_60 = if ice_time_seconds > 0.0 {
        goals_saved_above_expected / ice_time_seconds * 3_600.0
    } else {
        0.0
    };
    let minutes_last_seven_days =
        workload.iter().map(|row| row.ice_time_seconds).sum::<f64>() / 60.0;
    let days_rest = (before_date - latest.date).num_days().saturating_sub(1);
    let rest_penalty = match days_rest {
        0 => 35.0,
        1 => 15.0,
        _ => 0.0,
    };
    let minutes_penalty = ((minutes_last_seven_days - 120.0).max(0.0) / 120.0 * 30.0).min(30.0);
    let appearance_penalty = (workload.len().saturating_sub(2) as f64 * 10.0).min(20.0);
    let mut sealed = trailing.to_vec();
    sealed.extend(workload.iter().copied());
    sealed.sort_by_key(|row| (row.date, row.game_id));
    sealed.dedup_by_key(|row| row.game_id);
    Ok(MoneyPuckTrailingGoalieForm {
        player_id,
        before_date,
        requested_appearances,
        appearances: trailing.len(),
        latest_appearance_date: latest.date,
        days_rest,
        minutes_last_seven_days,
        appearances_last_seven_days: workload.len(),
        goals_saved_above_expected,
        gsax_per_60,
        form_quality: (50.0 + 25.0 * gsax_per_60).clamp(0.0, 100.0),
        workload_readiness: (100.0 - rest_penalty - minutes_penalty - appearance_penalty)
            .clamp(0.0, 100.0),
        source_fingerprint: fingerprint_rows(
            player_id,
            before_date,
            requested_appearances,
            &sealed,
        ),
    })
}

pub fn moneypuck_goalie_game_url(player_id: u32) -> String {
    format!(
        "https://moneypuck.com/moneypuck/playerData/careers/gameByGame/regular/goalies/{player_id}.csv"
    )
}

fn fingerprint_rows(
    player_id: u32,
    before_date: NaiveDate,
    requested_appearances: usize,
    rows: &[&MoneyPuckGoalieGameRow],
) -> String {
    let bytes = serde_json::to_vec(&(player_id, before_date, requested_appearances, rows))
        .expect("MoneyPuck goalie form boundary and rows are serializable");
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: &str = "playerId,season,name,gameId,playerTeam,opposingTeam,home_or_away,gameDate,position,situation,icetime,xGoals,goals\n";

    fn csv() -> String {
        format!(
            "{HEADER}8478048,2025,Goalie,1,NYR,SEA,HOME,20251001,G,all,3600,2.5,2\n\
             8478048,2025,Goalie,1,NYR,SEA,HOME,20251001,G,5on5,3000,2.0,2\n\
             8478048,2025,Goalie,2,NYR,BOS,AWAY,20251004,G,all,3600,3.0,4\n\
             8478048,2025,Goalie,3,NYR,NJD,HOME,20251006,G,all,3600,2.5,2\n\
             8478048,2025,Goalie,4,NYR,MTL,AWAY,20251007,G,all,1800,2.0,1\n\
             8478048,2025,Goalie,5,NYR,TOR,HOME,20251008,G,all,3600,2.0,8\n"
        )
    }

    #[test]
    fn trailing_form_excludes_same_day_and_future_rows() {
        let rows = parse_moneypuck_goalie_games(&csv()).unwrap();
        let form = derive_trailing_goalie_form(
            &rows,
            8_478_048,
            NaiveDate::from_ymd_opt(2025, 10, 8).unwrap(),
            3,
        )
        .unwrap();
        assert_eq!(form.appearances, 3);
        assert_eq!(form.latest_appearance_date.to_string(), "2025-10-07");
        assert_eq!(form.days_rest, 0);
        assert_eq!(form.appearances_last_seven_days, 4);
        assert!((form.goals_saved_above_expected - 0.5).abs() < 1e-12);
        assert!((form.gsax_per_60 - 0.2).abs() < 1e-12);
        assert_eq!(form.form_quality, 55.0);
        assert!(form.workload_readiness < 50.0);
        assert!(form.source_fingerprint.starts_with("sha256:"));
        let later_boundary = derive_trailing_goalie_form(
            &rows,
            8_478_048,
            NaiveDate::from_ymd_opt(2025, 10, 9).unwrap(),
            3,
        )
        .unwrap();
        assert_ne!(form.source_fingerprint, later_boundary.source_fingerprint);
    }

    #[test]
    fn parser_rejects_mixed_goalie_identity() {
        let mixed = format!(
            "{HEADER}1,2025,A,1,NYR,SEA,HOME,20251001,G,all,3600,2,2\n\
             2,2025,B,2,NYR,BOS,HOME,20251002,G,all,3600,2,2\n"
        );
        assert!(parse_moneypuck_goalie_games(&mixed).is_err());
    }
}
