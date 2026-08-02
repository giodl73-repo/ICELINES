//! MoneyPuck skater CSV parsing and source-local metric normalization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const REQUIRED_COLUMNS: &[&str] = &[
    "playerId",
    "situation",
    "icetime",
    "I_F_xGoals",
    "onIce_xGoalsFor",
    "onIce_xGoalsAgainst",
    "onIce_corsiFor",
    "onIce_corsiAgainst",
    "onIce_fenwickFor",
    "onIce_fenwickAgainst",
];

#[derive(Debug, thiserror::Error)]
pub enum MoneyPuckCsvError {
    #[error("MoneyPuck CSV missing required column(s): {0}")]
    MissingColumns(String),
    #[error("MoneyPuck CSV parse error: {0}")]
    Parse(#[from] csv::Error),
}

#[derive(Debug, Deserialize)]
pub struct MoneyPuckRow {
    #[serde(rename = "playerId")]
    pub player_id: u64,
    pub situation: String,
    pub icetime: f32,
    #[serde(rename = "I_F_xGoals", default)]
    pub i_f_x_goals: f32,
    #[serde(rename = "onIce_xGoalsFor", default)]
    pub on_ice_x_goals_for: f32,
    #[serde(rename = "onIce_xGoalsAgainst", default)]
    pub on_ice_x_goals_against: f32,
    #[serde(rename = "onIce_corsiFor", default)]
    pub on_ice_corsi_for: f32,
    #[serde(rename = "onIce_corsiAgainst", default)]
    pub on_ice_corsi_against: f32,
    #[serde(rename = "onIce_fenwickFor", default)]
    pub on_ice_fenwick_for: f32,
    #[serde(rename = "onIce_fenwickAgainst", default)]
    pub on_ice_fenwick_against: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyPuckStats {
    pub player_id: u32,
    pub xg_all: f32,
    pub xg_per_60: f32,
    pub cf_pct_5v5: f32,
    pub ff_pct_5v5: f32,
    pub on_ice_xg_for_5v5: f32,
    pub on_ice_xg_against_5v5: f32,
    pub xgf_pct_5v5: f32,
}

pub fn parse_csv(csv_text: &str) -> HashMap<u32, MoneyPuckStats> {
    parse_csv_checked(csv_text).unwrap_or_default()
}

pub fn parse_csv_checked(
    csv_text: &str,
) -> Result<HashMap<u32, MoneyPuckStats>, MoneyPuckCsvError> {
    let mut reader = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = reader.headers()?;
    let missing = REQUIRED_COLUMNS
        .iter()
        .copied()
        .filter(|column| !headers.iter().any(|header| header == *column))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(MoneyPuckCsvError::MissingColumns(missing.join(", ")));
    }

    let mut by_player: HashMap<u64, Vec<MoneyPuckRow>> = HashMap::new();
    for row in reader.deserialize::<MoneyPuckRow>() {
        let row = row?;
        by_player.entry(row.player_id).or_default().push(row);
    }

    Ok(by_player
        .iter()
        .filter_map(|(&player_id, rows)| {
            let all = rows.iter().find(|row| row.situation == "all")?;
            let five_on_five = rows.iter().find(|row| row.situation == "5on5")?;
            let xg_per_60 = if all.icetime > 0.0 {
                all.i_f_x_goals / all.icetime * 3600.0
            } else {
                0.0
            };
            let cf_total = five_on_five.on_ice_corsi_for + five_on_five.on_ice_corsi_against;
            let ff_total = five_on_five.on_ice_fenwick_for + five_on_five.on_ice_fenwick_against;
            let xg_total = five_on_five.on_ice_x_goals_for + five_on_five.on_ice_x_goals_against;
            Some((
                player_id as u32,
                MoneyPuckStats {
                    player_id: player_id as u32,
                    xg_all: all.i_f_x_goals,
                    xg_per_60,
                    cf_pct_5v5: percentage_or_neutral(five_on_five.on_ice_corsi_for, cf_total),
                    ff_pct_5v5: percentage_or_neutral(five_on_five.on_ice_fenwick_for, ff_total),
                    on_ice_xg_for_5v5: five_on_five.on_ice_x_goals_for,
                    on_ice_xg_against_5v5: five_on_five.on_ice_x_goals_against,
                    xgf_pct_5v5: percentage_or_neutral(five_on_five.on_ice_x_goals_for, xg_total),
                },
            ))
        })
        .collect())
}

fn percentage_or_neutral(value: f32, total: f32) -> f32 {
    if total > 0.0 {
        value / total * 100.0
    } else {
        50.0
    }
}

pub fn index(stats: Vec<MoneyPuckStats>) -> HashMap<u32, MoneyPuckStats> {
    stats
        .into_iter()
        .map(|stats| (stats.player_id, stats))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_complete_player_and_preserves_legacy_math() {
        let csv = "playerId,situation,icetime,I_F_xGoals,onIce_xGoalsFor,onIce_xGoalsAgainst,onIce_corsiFor,onIce_corsiAgainst,onIce_fenwickFor,onIce_fenwickAgainst\n8478402,all,5000,20,0,0,0,0,0,0\n8478402,5on5,3000,0,30,20,60,40,45,35\n";
        let result = parse_csv_checked(csv).unwrap();
        let stats = result.get(&8478402).unwrap();
        assert!((stats.cf_pct_5v5 - 60.0).abs() < 0.1);
        assert!((stats.ff_pct_5v5 - 56.25).abs() < 0.1);
        assert!((stats.xgf_pct_5v5 - 60.0).abs() < 0.1);
        assert!((stats.xg_per_60 - 14.4).abs() < 0.01);
    }
}
