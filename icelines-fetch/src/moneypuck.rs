//! MoneyPuck advanced metrics — silo'd fetch and parse module.
//!
//! Downloads free CSV data from moneypuck.com and converts to a per-player
//! stats struct. All fields on Player derived from MoneyPuck are Option<f32> —
//! None when data hasn't been fetched. Removing this module only requires
//! removing the Option fields from Player and the sort metrics that use them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

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

#[derive(Debug, Error)]
pub enum MoneyPuckCsvError {
    #[error("MoneyPuck CSV missing required column(s): {0}")]
    MissingColumns(String),
    #[error("MoneyPuck CSV parse error: {0}")]
    Parse(#[from] csv::Error),
}

/// One row of the MoneyPuck skaters CSV (per player per situation).
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

/// Processed MoneyPuck stats for one player — JSON-serializable for snapshot storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoneyPuckStats {
    pub player_id: u32,
    pub xg_all: f32,                // ixG all situations
    pub xg_per_60: f32,             // ixG per 60 minutes
    pub cf_pct_5v5: f32,            // CF% at 5v5 (0–100)
    pub ff_pct_5v5: f32,            // FF% at 5v5
    pub on_ice_xg_for_5v5: f32,     // xGF at 5v5
    pub on_ice_xg_against_5v5: f32, // xGA at 5v5
    pub xgf_pct_5v5: f32,           // xGF% at 5v5
}

/// Parse a MoneyPuck CSV string into a player_id → MoneyPuckStats map.
pub fn parse_csv(csv_text: &str) -> HashMap<u32, MoneyPuckStats> {
    parse_csv_checked(csv_text).unwrap_or_default()
}

/// Parse a MoneyPuck CSV string, failing explicitly on header or row drift.
pub fn parse_csv_checked(
    csv_text: &str,
) -> Result<HashMap<u32, MoneyPuckStats>, MoneyPuckCsvError> {
    let mut rdr = csv::Reader::from_reader(csv_text.as_bytes());
    let headers = rdr.headers()?;
    let missing: Vec<_> = REQUIRED_COLUMNS
        .iter()
        .copied()
        .filter(|column| !headers.iter().any(|header| header == *column))
        .collect();
    if !missing.is_empty() {
        return Err(MoneyPuckCsvError::MissingColumns(missing.join(", ")));
    }

    let mut by_player: HashMap<u64, Vec<MoneyPuckRow>> = HashMap::new();

    for row in rdr.deserialize::<MoneyPuckRow>() {
        let row = row?;
        by_player.entry(row.player_id).or_default().push(row);
    }

    let stats = by_player
        .iter()
        .filter_map(|(&pid, rows)| {
            let all_sit = rows.iter().find(|r| r.situation == "all")?;
            let five_v5 = rows.iter().find(|r| r.situation == "5on5")?;

            let xg_per_60 = if all_sit.icetime > 0.0 {
                all_sit.i_f_x_goals / all_sit.icetime * 3600.0
            } else {
                0.0
            };

            let cf_total = five_v5.on_ice_corsi_for + five_v5.on_ice_corsi_against;
            let cf_pct = if cf_total > 0.0 {
                five_v5.on_ice_corsi_for / cf_total * 100.0
            } else {
                50.0
            };

            let ff_total = five_v5.on_ice_fenwick_for + five_v5.on_ice_fenwick_against;
            let ff_pct = if ff_total > 0.0 {
                five_v5.on_ice_fenwick_for / ff_total * 100.0
            } else {
                50.0
            };

            let xg_total = five_v5.on_ice_x_goals_for + five_v5.on_ice_x_goals_against;
            let xgf_pct = if xg_total > 0.0 {
                five_v5.on_ice_x_goals_for / xg_total * 100.0
            } else {
                50.0
            };

            Some((
                pid as u32,
                MoneyPuckStats {
                    player_id: pid as u32,
                    xg_all: all_sit.i_f_x_goals,
                    xg_per_60,
                    cf_pct_5v5: cf_pct,
                    ff_pct_5v5: ff_pct,
                    on_ice_xg_for_5v5: five_v5.on_ice_x_goals_for,
                    on_ice_xg_against_5v5: five_v5.on_ice_x_goals_against,
                    xgf_pct_5v5: xgf_pct,
                },
            ))
        })
        .collect();
    Ok(stats)
}

/// Convert Vec<MoneyPuckStats> to HashMap for O(1) lookup.
pub fn index(stats: Vec<MoneyPuckStats>) -> HashMap<u32, MoneyPuckStats> {
    stats.into_iter().map(|s| (s.player_id, s)).collect()
}

/// CSV download URL for a given 8-digit season (e.g. "20252026" → year 2025).
pub fn csv_url(season: &str) -> Option<String> {
    let year: u32 = season.get(..4)?.parse().ok()?;
    Some(format!(
        "https://moneypuck.com/moneypuck/playerData/seasonSummary/{year}/regular/skaters.csv"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_csv_url_format() {
        assert_eq!(
            csv_url("20252026"),
            Some(
                "https://moneypuck.com/moneypuck/playerData/seasonSummary/2025/regular/skaters.csv"
                    .to_owned()
            )
        );
        assert_eq!(
            csv_url("20242025"),
            Some(
                "https://moneypuck.com/moneypuck/playerData/seasonSummary/2024/regular/skaters.csv"
                    .to_owned()
            )
        );
        assert_eq!(csv_url("short"), None);
    }

    #[test]
    fn l0_parse_empty_csv_returns_empty() {
        let csv = "playerId,situation,icetime,I_F_xGoals,onIce_xGoalsFor,onIce_xGoalsAgainst,onIce_corsiFor,onIce_corsiAgainst,onIce_fenwickFor,onIce_fenwickAgainst\n";
        let result = parse_csv(csv);
        assert!(result.is_empty());
    }

    #[test]
    fn l0_parse_csv_checked_rejects_missing_required_column() {
        let csv = "playerId,situation,icetime,I_F_xGoals,onIce_xGoalsFor,onIce_xGoalsAgainst,onIce_corsiFor,onIce_corsiAgainst,onIce_fenwickFor\n\
                   8478402,all,5000,20.0,0,0,0,0,0\n";
        let err = parse_csv_checked(csv).expect_err("missing column must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("onIce_fenwickAgainst"),
            "expected missing column name, got: {msg}"
        );
    }

    #[test]
    fn l0_parse_csv_checked_rejects_bad_numeric_row() {
        let csv = format!(
            "{}{}",
            csv_header(),
            "8478402,all,not-a-number,20.0,0,0,0,0,0,0\n"
        );
        let err = parse_csv_checked(&csv).expect_err("malformed row must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("parse error"),
            "expected parse error, got: {msg}"
        );
    }

    #[test]
    fn l0_parse_csv_computes_pct() {
        let csv = "playerId,situation,icetime,I_F_xGoals,onIce_xGoalsFor,onIce_xGoalsAgainst,onIce_corsiFor,onIce_corsiAgainst,onIce_fenwickFor,onIce_fenwickAgainst\n\
                   8478402,all,5000,20.0,0,0,0,0,0,0\n\
                   8478402,5on5,3000,0,30.0,20.0,60.0,40.0,45.0,35.0\n";
        let result = parse_csv(csv);
        let stats = result.get(&8478402).expect("player must be found");
        assert!((stats.xg_all - 20.0).abs() < 0.01);
        assert!((stats.on_ice_xg_for_5v5 - 30.0).abs() < 0.01);
        assert!((stats.on_ice_xg_against_5v5 - 20.0).abs() < 0.01);
        assert!(
            (stats.cf_pct_5v5 - 60.0).abs() < 0.1,
            "CF% should be 60/(60+40)*100=60.0, got {}",
            stats.cf_pct_5v5
        );
        assert!(
            (stats.xgf_pct_5v5 - 60.0).abs() < 0.1,
            "xGF% should be 30/(30+20)*100=60.0, got {}",
            stats.xgf_pct_5v5
        );
        assert!((stats.xg_per_60 - 20.0 / 5000.0 * 3600.0).abs() < 0.01);
    }

    #[test]
    fn l0_parse_committed_moneypuck_schema_fixture() {
        let csv = include_str!("../../tests/fixtures/moneypuck/skaters_schema_sample.csv");
        let result = parse_csv_checked(csv).expect("schema fixture must parse");
        assert_eq!(result.len(), 2);

        let elite = result.get(&8480001).expect("fixture player must parse");
        assert!((elite.xg_all - 20.0).abs() < 0.01);
        assert!((elite.on_ice_xg_for_5v5 - 30.0).abs() < 0.01);
        assert!((elite.on_ice_xg_against_5v5 - 20.0).abs() < 0.01);
        assert!((elite.cf_pct_5v5 - 60.0).abs() < 0.1);
        assert!((elite.ff_pct_5v5 - 56.25).abs() < 0.1);
        assert!((elite.xgf_pct_5v5 - 60.0).abs() < 0.1);

        let solid = result
            .get(&8480002)
            .expect("second fixture player must parse");
        assert!((solid.cf_pct_5v5 - 44.0).abs() < 0.1);
        assert!((solid.ff_pct_5v5 - 45.0).abs() < 0.1);
        assert!((solid.xgf_pct_5v5 - 48.0).abs() < 0.1);
    }

    fn csv_header() -> &'static str {
        "playerId,situation,icetime,I_F_xGoals,onIce_xGoalsFor,onIce_xGoalsAgainst,onIce_corsiFor,onIce_corsiAgainst,onIce_fenwickFor,onIce_fenwickAgainst\n"
    }

    #[test]
    fn l0_parse_csv_handles_missing_5on5_row() {
        // Player has an "all" row but no "5on5" row — filter_map should return None for them
        let csv = format!(
            "{}{}",
            csv_header(),
            "9999001,all,4000,15.0,0,0,0,0,0,0\n\
             9999001,4on5,2000,0,10.0,8.0,30.0,25.0,28.0,22.0\n"
        );
        let result = parse_csv(&csv);
        // Player without 5on5 row must be absent from results
        assert!(
            !result.contains_key(&9999001),
            "player missing 5on5 row should not appear in results"
        );
    }

    #[test]
    fn l0_parse_csv_handles_zero_icetime() {
        // Player with icetime=0 in "all" row — xg_per_60 should be 0.0 (no divide-by-zero)
        let csv = format!(
            "{}{}",
            csv_header(),
            "9999002,all,0,5.0,0,0,0,0,0,0\n\
             9999002,5on5,0,0,20.0,20.0,50.0,50.0,40.0,40.0\n"
        );
        let result = parse_csv(&csv);
        let stats = result
            .get(&9999002)
            .expect("player must be found despite zero icetime");
        assert!(
            (stats.xg_per_60 - 0.0).abs() < 0.001,
            "xg_per_60 must be 0.0 when icetime=0, got {}",
            stats.xg_per_60
        );
        // cf_pct should be 50.0 when totals are zero
        assert!(
            (stats.cf_pct_5v5 - 50.0).abs() < 0.1,
            "cf_pct should default to 50.0 when cf_for+cf_against=0"
        );
    }

    #[test]
    fn l0_parse_csv_multiple_players() {
        // Two complete players — both must appear in the result
        let csv = format!(
            "{}{}",
            csv_header(),
            "8480001,all,5000,20.0,0,0,0,0,0,0\n\
             8480001,5on5,3000,0,30.0,20.0,60.0,40.0,45.0,35.0\n\
             8480002,all,4000,10.0,0,0,0,0,0,0\n\
             8480002,5on5,2500,0,25.0,25.0,50.0,50.0,40.0,40.0\n"
        );
        let result = parse_csv(&csv);
        assert_eq!(result.len(), 2, "both players should be in result");
        assert!(result.contains_key(&8480001));
        assert!(result.contains_key(&8480002));

        // Verify player 2 has 50% CF (balanced corsi)
        let p2 = result.get(&8480002).unwrap();
        assert!(
            (p2.cf_pct_5v5 - 50.0).abs() < 0.1,
            "balanced corsi should give 50%, got {}",
            p2.cf_pct_5v5
        );
    }

    #[test]
    fn l0_parse_csv_xg_per_60_calculation() {
        // 18.0 xG in 3600s ice time = 18.0 per 60 (exactly)
        let csv = format!(
            "{}{}",
            csv_header(),
            "7777001,all,3600,18.0,0,0,0,0,0,0\n\
             7777001,5on5,2000,0,10.0,10.0,50.0,50.0,40.0,40.0\n"
        );
        let result = parse_csv(&csv);
        let stats = result.get(&7777001).expect("player must be found");
        assert!(
            (stats.xg_per_60 - 18.0).abs() < 0.01,
            "xg_per_60 should be 18.0, got {}",
            stats.xg_per_60
        );
    }

    #[test]
    fn l0_index_builds_hashmap_by_player_id() {
        let stats = vec![
            MoneyPuckStats {
                player_id: 100,
                xg_all: 10.0,
                xg_per_60: 2.0,
                cf_pct_5v5: 55.0,
                ff_pct_5v5: 54.0,
                on_ice_xg_for_5v5: 31.0,
                on_ice_xg_against_5v5: 25.0,
                xgf_pct_5v5: 56.0,
            },
            MoneyPuckStats {
                player_id: 200,
                xg_all: 20.0,
                xg_per_60: 3.0,
                cf_pct_5v5: 48.0,
                ff_pct_5v5: 47.0,
                on_ice_xg_for_5v5: 20.0,
                on_ice_xg_against_5v5: 21.0,
                xgf_pct_5v5: 49.0,
            },
        ];
        let map = index(stats);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&100).unwrap().xg_all, 10.0);
        assert_eq!(map.get(&200).unwrap().xg_all, 20.0);
    }
}
