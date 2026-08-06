//! Licensed/local source-package assembly for automatic MoneyPuck chemistry.
//!
//! IceLines deliberately does not bulk-scrape MoneyPuck. Callers provide CSV
//! bytes obtained under the provider's published terms or a separate license;
//! this layer discovers units, builds pregame baselines, and seals the result.

use std::collections::BTreeSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    build_moneypuck_line_chemistry, build_moneypuck_unit_baselines, parse_moneypuck_line_games,
    parse_moneypuck_line_summary, parse_moneypuck_skater_games, parse_moneypuck_team_games,
    MoneyPuckLineChemistryView, MoneyPuckUnitBaselineConfig, MoneyPuckUnitBaselineSetView,
};

pub const MONEYPUCK_LINE_CHEMISTRY_SOURCE_PACKAGE_SCHEMA: &str =
    "moneypuck_line_chemistry_source_package.v1";
pub const MONEYPUCK_LINE_CHEMISTRY_ACQUISITION_SCHEMA: &str =
    "moneypuck_line_chemistry_acquisition.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckLineChemistrySourcePackage {
    pub schema: String,
    pub summary_csv: String,
    pub line_game_csvs: Vec<String>,
    pub skater_game_csvs: Vec<String>,
    pub team_game_csvs: Vec<String>,
    /// User-declared authority or license note; never interpreted as permission
    /// by IceLines, but retained in the sealed result.
    pub rights_basis: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckLineChemistryAcquisitionView {
    pub schema: String,
    pub team: String,
    pub season: u32,
    pub forecast_at: DateTime<Utc>,
    pub summary_units_discovered: usize,
    pub line_game_documents: usize,
    pub skater_documents: usize,
    pub opponent_documents: usize,
    pub source_fingerprints: Vec<String>,
    pub rights_basis: String,
    pub baselines: MoneyPuckUnitBaselineSetView,
    pub chemistry: MoneyPuckLineChemistryView,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

pub fn build_moneypuck_line_chemistry_from_package(
    team: &str,
    season_start_year: u32,
    forecast_at: DateTime<Utc>,
    package: MoneyPuckLineChemistrySourcePackage,
    minimum_shared_minutes: f64,
    baseline_config: MoneyPuckUnitBaselineConfig,
) -> Result<MoneyPuckLineChemistryAcquisitionView, String> {
    let team = team.trim().to_ascii_uppercase();
    if package.schema != MONEYPUCK_LINE_CHEMISTRY_SOURCE_PACKAGE_SCHEMA
        || package.summary_csv.trim().is_empty()
        || package.line_game_csvs.is_empty()
        || package.skater_game_csvs.is_empty()
        || package.team_game_csvs.is_empty()
        || package.rights_basis.trim().is_empty()
        || !(2007..=3000).contains(&season_start_year)
        || !minimum_shared_minutes.is_finite()
        || minimum_shared_minutes <= 0.0
    {
        return Err("MoneyPuck automatic chemistry requires a complete, rights-declared local source package".into());
    }
    let season = season_start_year * 10_000 + season_start_year + 1;
    let summary = parse_moneypuck_line_summary(&package.summary_csv)
        .map_err(|error| format!("parse MoneyPuck line summary: {error}"))?;
    let unit_ids = summary
        .iter()
        .filter(|row| {
            row.team == team
                && row.season == season
                && row.situation == "5on5"
                && row.ice_time_seconds / 60.0 >= minimum_shared_minutes
        })
        .map(|row| row.line_id.clone())
        .collect::<BTreeSet<_>>();
    if unit_ids.is_empty() {
        return Err(format!(
            "MoneyPuck summary has no {team} units above {minimum_shared_minutes:.1} shared minutes"
        ));
    }

    let mut line_games = Vec::new();
    let mut line_documents = 0;
    for csv in &package.line_game_csvs {
        let rows = parse_moneypuck_line_games(csv)
            .map_err(|error| format!("parse MoneyPuck line-game document: {error}"))?;
        if rows.iter().any(|row| unit_ids.contains(&row.line_id)) {
            line_documents += 1;
            line_games.extend(
                rows.into_iter()
                    .filter(|row| unit_ids.contains(&row.line_id)),
            );
        }
    }
    let prior_lines = line_games
        .iter()
        .filter(|row| {
            row.team == team && row.situation == "5on5" && row.date < forecast_at.date_naive()
        })
        .collect::<Vec<_>>();
    if prior_lines.is_empty() {
        return Err(
            "local package has no discovered unit games before the forecast boundary".into(),
        );
    }
    let player_ids = prior_lines
        .iter()
        .flat_map(|row| row.player_ids.iter().copied())
        .collect::<BTreeSet<_>>();
    let opponents = prior_lines
        .iter()
        .map(|row| row.opponent.clone())
        .collect::<BTreeSet<_>>();

    let mut skater_games = Vec::new();
    let mut skater_documents = 0;
    for csv in &package.skater_game_csvs {
        let rows = parse_moneypuck_skater_games(csv)
            .map_err(|error| format!("parse MoneyPuck skater document: {error}"))?;
        if rows.iter().any(|row| player_ids.contains(&row.player_id)) {
            skater_documents += 1;
            skater_games.extend(
                rows.into_iter()
                    .filter(|row| player_ids.contains(&row.player_id)),
            );
        }
    }
    let covered_players = skater_games
        .iter()
        .map(|row| row.player_id)
        .collect::<BTreeSet<_>>();
    let missing_players = player_ids
        .difference(&covered_players)
        .copied()
        .collect::<Vec<_>>();
    if !missing_players.is_empty() {
        return Err(format!(
            "local MoneyPuck package is missing skater documents for IDs: {}",
            missing_players
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let mut team_games = Vec::new();
    let mut opponent_documents = 0;
    for csv in &package.team_game_csvs {
        let rows = parse_moneypuck_team_games(csv)
            .map_err(|error| format!("parse MoneyPuck team document: {error}"))?;
        if rows.iter().any(|row| opponents.contains(&row.team)) {
            opponent_documents += 1;
            team_games.extend(rows.into_iter().filter(|row| opponents.contains(&row.team)));
        }
    }
    let covered_opponents = team_games
        .iter()
        .map(|row| row.team.clone())
        .collect::<BTreeSet<_>>();
    let missing_opponents = opponents
        .difference(&covered_opponents)
        .cloned()
        .collect::<Vec<_>>();
    if !missing_opponents.is_empty() {
        return Err(format!(
            "local MoneyPuck package is missing opponent documents: {}",
            missing_opponents.join(", ")
        ));
    }

    let baselines = build_moneypuck_unit_baselines(
        &team,
        forecast_at,
        &line_games,
        &skater_games,
        &team_games,
        baseline_config,
    )?;
    let chemistry = build_moneypuck_line_chemistry(
        &team,
        forecast_at,
        &line_games,
        baselines.baselines.clone(),
        minimum_shared_minutes,
    )?;
    let mut source_fingerprints = vec![fingerprint(&package.summary_csv)?];
    source_fingerprints.extend(
        package
            .line_game_csvs
            .iter()
            .chain(&package.skater_game_csvs)
            .chain(&package.team_game_csvs)
            .map(fingerprint)
            .collect::<Result<Vec<_>, _>>()?,
    );
    source_fingerprints.sort();
    source_fingerprints.dedup();
    let mut view = MoneyPuckLineChemistryAcquisitionView {
        schema: MONEYPUCK_LINE_CHEMISTRY_ACQUISITION_SCHEMA.to_owned(),
        team,
        season,
        forecast_at,
        summary_units_discovered: unit_ids.len(),
        line_game_documents: line_documents,
        skater_documents,
        opponent_documents,
        source_fingerprints,
        rights_basis: package.rights_basis,
        baselines,
        chemistry,
        disclosures: vec![
            "IceLines did not bulk-fetch these documents; the caller supplied a local source package under a declared rights basis.".to_owned(),
            "The season summary discovers unit IDs only; only games and baseline evidence strictly before the forecast can affect chemistry.".to_owned(),
            "MoneyPuck source credit and its published data terms remain required.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn refuses_packages_without_declared_rights_basis() {
        let package = MoneyPuckLineChemistrySourcePackage {
            schema: MONEYPUCK_LINE_CHEMISTRY_SOURCE_PACKAGE_SCHEMA.to_owned(),
            summary_csv: "header".to_owned(),
            line_game_csvs: vec!["header".to_owned()],
            skater_game_csvs: vec!["header".to_owned()],
            team_game_csvs: vec!["header".to_owned()],
            rights_basis: String::new(),
        };
        assert!(build_moneypuck_line_chemistry_from_package(
            "NYR",
            2025,
            Utc::now(),
            package,
            30.0,
            MoneyPuckUnitBaselineConfig::default(),
        )
        .is_err());
    }

    #[test]
    fn local_package_discovers_units_and_builds_automatic_baselines() {
        let line_id = "847000184700028470003";
        let summary_csv = format!(
            "lineId,season,name,team,position,situation,games_played,icetime\n{line_id},2025,One-Two-Three,NYR,line,5on5,1,600\n"
        );
        let line_game_csv = format!(
            "lineId,name,gameId,playerTeam,opposingTeam,home_or_away,gameDate,position,situation,icetime,scoreVenueAdjustedxGoalsFor,scoreVenueAdjustedxGoalsAgainst\n{line_id},One-Two-Three,2025020100,NYR,BOS,HOME,20251010,line,5on5,600,0.6,0.4\n"
        );
        let skater_header = "playerId,gameId,playerTeam,opposingTeam,gameDate,position,situation,icetime,OnIce_F_scoreVenueAdjustedxGoals,OnIce_A_scoreVenueAdjustedxGoals,I_F_oZoneShiftStarts,I_F_dZoneShiftStarts\n";
        let skater_game_csvs = [8_470_001, 8_470_002, 8_470_003]
            .into_iter()
            .map(|player_id| {
                format!(
                    "{skater_header}{player_id},2025020001,NYR,NJD,20251005,C,5on5,900,0.6,0.4,6,4\n"
                )
            })
            .collect();
        let team_game_csv = "team,season,gameId,playerTeam,opposingTeam,home_or_away,gameDate,situation,iceTime,scoreVenueAdjustedxGoalsFor,scoreVenueAdjustedxGoalsAgainst\nBOS,2025,2025020002,BOS,NJD,HOME,20251005,all,3600,0.4,0.6\n";
        let package = MoneyPuckLineChemistrySourcePackage {
            schema: MONEYPUCK_LINE_CHEMISTRY_SOURCE_PACKAGE_SCHEMA.to_owned(),
            summary_csv,
            line_game_csvs: vec![line_game_csv],
            skater_game_csvs,
            team_game_csvs: vec![team_game_csv.to_owned()],
            rights_basis: "test fixture".to_owned(),
        };
        let view = build_moneypuck_line_chemistry_from_package(
            "NYR",
            2025,
            Utc.with_ymd_and_hms(2025, 10, 11, 12, 0, 0).unwrap(),
            package,
            5.0,
            MoneyPuckUnitBaselineConfig {
                minimum_player_games: 1,
                ..MoneyPuckUnitBaselineConfig::default()
            },
        )
        .expect("complete local source package");
        assert_eq!(view.summary_units_discovered, 1);
        assert_eq!(view.baselines.baselines_built, 1);
        assert_eq!(view.chemistry.chemistry.evidence.len(), 1);
        assert_eq!(view.rights_basis, "test fixture");
    }
}
