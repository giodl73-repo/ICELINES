//! Automatic, strictly pregame baseline authority for MoneyPuck pair/trio xG.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use icelines_sources::{
    moneypuck_line_game::MoneyPuckLineGameRow, moneypuck_skater_game::MoneyPuckSkaterGameRow,
    moneypuck_team_game::MoneyPuckTeamGameRow,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::moneypuck_line_chemistry::{
    PregameUnitXgBaseline, UnitBaselineComponent, PREGAME_UNIT_XG_BASELINE_SCHEMA,
};

pub const MONEYPUCK_UNIT_BASELINE_SET_SCHEMA: &str = "moneypuck_unit_baseline_set.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckUnitBaselineConfig {
    pub trailing_games: usize,
    pub minimum_player_games: usize,
    pub individual_weight: f64,
    pub zone_start_coefficient: f64,
}

impl Default for MoneyPuckUnitBaselineConfig {
    fn default() -> Self {
        Self {
            trailing_games: 20,
            minimum_player_games: 3,
            individual_weight: 0.6,
            zone_start_coefficient: 0.03,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoneyPuckUnitBaselineExclusionReason {
    MissingPlayerHistory,
    MissingOpponentHistory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckUnitBaselineExclusion {
    pub game_id: u64,
    pub team: String,
    pub player_ids: Vec<u32>,
    pub reason: MoneyPuckUnitBaselineExclusionReason,
    pub missing_player_ids: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckUnitBaselineSetView {
    pub schema: String,
    pub team: String,
    pub forecast_at: DateTime<Utc>,
    pub requested_unit_games: usize,
    pub baselines_built: usize,
    pub coverage: f64,
    pub config: MoneyPuckUnitBaselineConfig,
    pub baselines: Vec<PregameUnitXgBaseline>,
    pub exclusions: Vec<MoneyPuckUnitBaselineExclusion>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

pub fn build_moneypuck_unit_baselines(
    team: &str,
    forecast_at: DateTime<Utc>,
    line_games: &[MoneyPuckLineGameRow],
    skater_games: &[MoneyPuckSkaterGameRow],
    team_games: &[MoneyPuckTeamGameRow],
    config: MoneyPuckUnitBaselineConfig,
) -> Result<MoneyPuckUnitBaselineSetView, String> {
    let team = team.trim().to_ascii_uppercase();
    if line_games.is_empty()
        || skater_games.is_empty()
        || team_games.is_empty()
        || config.trailing_games == 0
        || config.minimum_player_games == 0
        || config.minimum_player_games > config.trailing_games
        || !config.individual_weight.is_finite()
        || !(0.0..=1.0).contains(&config.individual_weight)
        || !config.zone_start_coefficient.is_finite()
        || !(0.0..=0.1).contains(&config.zone_start_coefficient)
    {
        return Err(
            "automatic unit baselines require valid source rows and bounded configuration".into(),
        );
    }

    let mut target_rows = line_games
        .iter()
        .filter(|row| {
            row.team == team
                && row.situation == "5on5"
                && row.date < forecast_at.date_naive()
                && row.ice_time_seconds > 0.0
        })
        .collect::<Vec<_>>();
    target_rows.sort_by_key(|row| (row.date, row.game_id, row.player_ids.clone()));
    let mut target_keys = BTreeSet::new();
    if target_rows.is_empty()
        || target_rows
            .iter()
            .any(|row| !target_keys.insert((row.game_id, row.player_ids.clone())))
    {
        return Err(
            "automatic unit baselines require unique, strictly-prior 5-on-5 unit games".into(),
        );
    }

    let mut skaters_by_id: BTreeMap<u32, Vec<&MoneyPuckSkaterGameRow>> = BTreeMap::new();
    for row in skater_games.iter().filter(|row| row.situation == "5on5") {
        skaters_by_id.entry(row.player_id).or_default().push(row);
    }
    for rows in skaters_by_id.values_mut() {
        rows.sort_by_key(|row| (row.date, row.game_id));
    }
    let mut teams: BTreeMap<String, Vec<&MoneyPuckTeamGameRow>> = BTreeMap::new();
    for row in team_games.iter().filter(|row| row.situation == "all") {
        teams.entry(row.team.clone()).or_default().push(row);
    }
    for rows in teams.values_mut() {
        rows.sort_by_key(|row| (row.date, row.game_id));
    }

    let mut baselines = Vec::new();
    let mut exclusions = Vec::new();
    for line in &target_rows {
        let mut selected_players = Vec::new();
        let mut missing_player_ids = Vec::new();
        for player_id in &line.player_ids {
            let selected = skaters_by_id
                .get(player_id)
                .into_iter()
                .flatten()
                .copied()
                .filter(|row| row.date < line.date && row.ice_time_seconds > 0.0)
                .rev()
                .take(config.trailing_games)
                .collect::<Vec<_>>();
            if selected.len() < config.minimum_player_games {
                missing_player_ids.push(*player_id);
            } else {
                selected_players.push(selected);
            }
        }
        if !missing_player_ids.is_empty() {
            exclusions.push(MoneyPuckUnitBaselineExclusion {
                game_id: line.game_id,
                team: team.clone(),
                player_ids: line.player_ids.clone(),
                reason: MoneyPuckUnitBaselineExclusionReason::MissingPlayerHistory,
                missing_player_ids,
            });
            continue;
        }

        let opponent_rows = teams
            .get(&line.opponent)
            .into_iter()
            .flatten()
            .copied()
            .filter(|row| row.date < line.date)
            .rev()
            .take(config.trailing_games)
            .collect::<Vec<_>>();
        if opponent_rows.len() < config.minimum_player_games {
            exclusions.push(MoneyPuckUnitBaselineExclusion {
                game_id: line.game_id,
                team: team.clone(),
                player_ids: line.player_ids.clone(),
                reason: MoneyPuckUnitBaselineExclusionReason::MissingOpponentHistory,
                missing_player_ids: Vec::new(),
            });
            continue;
        }

        let player_shares = selected_players
            .iter()
            .map(|rows| {
                let xg_for = rows
                    .iter()
                    .map(|row| row.score_venue_adjusted_on_ice_xg_for)
                    .sum::<f64>();
                let xg_against = rows
                    .iter()
                    .map(|row| row.score_venue_adjusted_on_ice_xg_against)
                    .sum::<f64>();
                share(xg_for, xg_against)
            })
            .collect::<Vec<_>>();
        let individual_share = player_shares.iter().sum::<f64>() / player_shares.len() as f64;
        let opponent_xg_for = opponent_rows
            .iter()
            .map(|row| row.score_venue_adjusted_xg_for)
            .sum::<f64>();
        let opponent_xg_against = opponent_rows
            .iter()
            .map(|row| row.score_venue_adjusted_xg_against)
            .sum::<f64>();
        let opponent_allowance = 1.0 - share(opponent_xg_for, opponent_xg_against);
        let offensive_starts = selected_players
            .iter()
            .flatten()
            .map(|row| row.offensive_zone_shift_starts)
            .sum::<f64>();
        let defensive_starts = selected_players
            .iter()
            .flatten()
            .map(|row| row.defensive_zone_shift_starts)
            .sum::<f64>();
        let zone_bias = if offensive_starts + defensive_starts > 0.0 {
            (offensive_starts - defensive_starts) / (offensive_starts + defensive_starts)
        } else {
            0.0
        };
        let expected_xg_share = (0.5
            + config.individual_weight * (individual_share - 0.5)
            + (1.0 - config.individual_weight) * (opponent_allowance - 0.5)
            + config.zone_start_coefficient * zone_bias)
            .clamp(0.2, 0.8);

        let mut source_fingerprints = selected_players
            .iter()
            .map(fingerprint)
            .collect::<Result<Vec<_>, _>>()?;
        source_fingerprints.push(fingerprint(&opponent_rows)?);
        source_fingerprints.sort();
        source_fingerprints.dedup();
        baselines.push(PregameUnitXgBaseline {
            schema: PREGAME_UNIT_XG_BASELINE_SCHEMA.to_owned(),
            game_id: line.game_id,
            team: team.clone(),
            player_ids: line.player_ids.clone(),
            computed_at: line
                .date
                .pred_opt()
                .ok_or_else(|| "cannot construct pregame cutoff".to_owned())?
                .and_hms_opt(23, 59, 59)
                .ok_or_else(|| "cannot construct pregame cutoff".to_owned())?
                .and_utc(),
            expected_xg_share,
            components: BTreeSet::from([
                UnitBaselineComponent::Individual,
                UnitBaselineComponent::Opponent,
                UnitBaselineComponent::Deployment,
            ]),
            method: "rolling-individual-opponent-zone-start.v1".to_owned(),
            source_fingerprints,
        });
    }

    let requested_unit_games = target_rows.len();
    let baselines_built = baselines.len();
    if baselines.is_empty() {
        return Err("no automatic unit baseline cleared historical coverage gates".into());
    }
    let mut view = MoneyPuckUnitBaselineSetView {
        schema: MONEYPUCK_UNIT_BASELINE_SET_SCHEMA.to_owned(),
        team,
        forecast_at,
        requested_unit_games,
        baselines_built,
        coverage: baselines_built as f64 / requested_unit_games as f64,
        config,
        baselines,
        exclusions,
        disclosures: vec![
            "Individual form uses each unit member's strictly prior score/venue-adjusted 5-on-5 on-ice xG.".to_owned(),
            "Opponent context uses the opponent's strictly prior team xG; deployment context uses strictly prior offensive and defensive-zone shift starts.".to_owned(),
            "Players and opponents without the configured prior-game minimum are excluded rather than assigned fabricated history.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

fn share(for_value: f64, against_value: f64) -> f64 {
    let total = for_value + against_value;
    if total > 0.0 {
        for_value / total
    } else {
        0.5
    }
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};

    use super::*;

    fn line(game_id: u64, day: u32) -> MoneyPuckLineGameRow {
        MoneyPuckLineGameRow {
            line_id: "847000184700028470003".to_owned(),
            player_ids: vec![8_470_001, 8_470_002, 8_470_003],
            name: "One-Two-Three".to_owned(),
            season: 20_252_026,
            game_id,
            date: NaiveDate::from_ymd_opt(2025, 10, day).unwrap(),
            team: "NYR".to_owned(),
            opponent: "BOS".to_owned(),
            home: true,
            position: "line".to_owned(),
            situation: "5on5".to_owned(),
            ice_time_seconds: 600.0,
            score_venue_adjusted_xg_for: 0.6,
            score_venue_adjusted_xg_against: 0.4,
        }
    }

    fn skater(player_id: u32, game_id: u64, day: u32, xg_for: f64) -> MoneyPuckSkaterGameRow {
        MoneyPuckSkaterGameRow {
            player_id,
            season: 20_252_026,
            game_id,
            date: NaiveDate::from_ymd_opt(2025, 10, day).unwrap(),
            team: "NYR".to_owned(),
            opponent: "NJD".to_owned(),
            position: "C".to_owned(),
            situation: "5on5".to_owned(),
            ice_time_seconds: 900.0,
            score_venue_adjusted_on_ice_xg_for: xg_for,
            score_venue_adjusted_on_ice_xg_against: 1.0 - xg_for,
            offensive_zone_shift_starts: 6.0,
            defensive_zone_shift_starts: 4.0,
        }
    }

    fn opponent(game_id: u64, day: u32) -> MoneyPuckTeamGameRow {
        MoneyPuckTeamGameRow {
            season: 20_252_026,
            game_id,
            date: NaiveDate::from_ymd_opt(2025, 10, day).unwrap(),
            team: "BOS".to_owned(),
            opponent: "NJD".to_owned(),
            home: true,
            situation: "all".to_owned(),
            ice_time_seconds: 3_600.0,
            score_venue_adjusted_xg_for: 0.4,
            score_venue_adjusted_xg_against: 0.6,
        }
    }

    #[test]
    fn builds_baseline_only_from_rows_before_each_game() {
        let lines = vec![line(2025020100, 10)];
        let mut skaters = Vec::new();
        for player_id in [8_470_001, 8_470_002, 8_470_003] {
            skaters.extend([
                skater(player_id, 2025020001, 5, 0.6),
                skater(player_id, 2025020002, 6, 0.6),
                skater(player_id, 2025020003, 7, 0.6),
                skater(player_id, 2025029999, 11, 0.0),
            ]);
        }
        let teams = vec![
            opponent(2025020001, 5),
            opponent(2025020002, 6),
            opponent(2025020003, 7),
        ];
        let view = build_moneypuck_unit_baselines(
            "NYR",
            Utc.with_ymd_and_hms(2025, 10, 12, 12, 0, 0).unwrap(),
            &lines,
            &skaters,
            &teams,
            MoneyPuckUnitBaselineConfig::default(),
        )
        .expect("covered baseline");
        assert_eq!(view.baselines_built, 1);
        // .600 individual, .600 opponent allowance, and +.006 zone-start context.
        assert!((view.baselines[0].expected_xg_share - 0.606).abs() < 1e-9);
        assert!(
            view.baselines[0].computed_at < lines[0].date.and_hms_opt(0, 0, 0).unwrap().and_utc()
        );
    }

    #[test]
    fn exposes_missing_rookie_history_instead_of_filling_neutral() {
        let lines = vec![line(2025020100, 10)];
        let skaters = vec![
            skater(8_470_001, 2025020001, 5, 0.6),
            skater(8_470_001, 2025020002, 6, 0.6),
            skater(8_470_001, 2025020003, 7, 0.6),
        ];
        let teams = vec![
            opponent(2025020001, 5),
            opponent(2025020002, 6),
            opponent(2025020003, 7),
        ];
        assert!(build_moneypuck_unit_baselines(
            "NYR",
            Utc.with_ymd_and_hms(2025, 10, 12, 12, 0, 0).unwrap(),
            &lines,
            &skaters,
            &teams,
            MoneyPuckUnitBaselineConfig::default(),
        )
        .is_err());
    }
}
