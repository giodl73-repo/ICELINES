//! Chronology-safe MoneyPuck line-game adapter.
//!
//! MoneyPuck supplies the observed score/venue-adjusted 5-on-5 xG. A separate
//! pregame baseline must declare individual, opponent, and deployment inputs;
//! this module never treats shared ice or raw unit xG as causal chemistry.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use icelines_sources::moneypuck_line_game::MoneyPuckLineGameRow;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::line_chemistry_outcome::{
    build_shift_adjusted_chemistry_evidence, ShiftAdjustedChemistryAdapterView,
    ShiftAdjustedUnitOutcomeInput, SHIFT_ADJUSTED_UNIT_OUTCOME_SCHEMA,
};

pub const PREGAME_UNIT_XG_BASELINE_SCHEMA: &str = "pregame_unit_xg_baseline.v1";
pub const MONEYPUCK_LINE_CHEMISTRY_SCHEMA: &str = "moneypuck_line_chemistry.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitBaselineComponent {
    Individual,
    Opponent,
    Deployment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PregameUnitXgBaseline {
    pub schema: String,
    pub game_id: u64,
    pub team: String,
    pub player_ids: Vec<u32>,
    /// Must precede the game date. Same-day backfills fail closed because the
    /// MoneyPuck line-game source does not carry puck-drop timestamps.
    pub computed_at: DateTime<Utc>,
    pub expected_xg_share: f64,
    pub components: BTreeSet<UnitBaselineComponent>,
    pub method: String,
    pub source_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoneyPuckLineChemistryView {
    pub schema: String,
    pub team: String,
    pub forecast_at: DateTime<Utc>,
    pub eligible_5v5_rows: usize,
    pub modeled_rows: usize,
    pub excluded_missing_baseline: usize,
    pub excluded_zero_xg: usize,
    pub baseline_coverage: f64,
    pub minimum_shared_minutes: f64,
    pub units_below_minimum: usize,
    pub chemistry: ShiftAdjustedChemistryAdapterView,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

pub fn build_moneypuck_line_chemistry(
    team: &str,
    forecast_at: DateTime<Utc>,
    rows: &[MoneyPuckLineGameRow],
    baselines: Vec<PregameUnitXgBaseline>,
    minimum_shared_minutes: f64,
) -> Result<MoneyPuckLineChemistryView, String> {
    let team = team.trim().to_ascii_uppercase();
    if rows.is_empty()
        || baselines.is_empty()
        || !minimum_shared_minutes.is_finite()
        || minimum_shared_minutes <= 0.0
    {
        return Err(
            "MoneyPuck line chemistry requires rows, baselines, and positive minimum minutes"
                .into(),
        );
    }

    let mut baseline_by_key = BTreeMap::new();
    for mut baseline in baselines {
        baseline.player_ids.sort_unstable();
        baseline.source_fingerprints.sort();
        baseline.source_fingerprints.dedup();
        let required_components = BTreeSet::from([
            UnitBaselineComponent::Individual,
            UnitBaselineComponent::Opponent,
            UnitBaselineComponent::Deployment,
        ]);
        if baseline.schema != PREGAME_UNIT_XG_BASELINE_SCHEMA
            || !baseline.team.eq_ignore_ascii_case(&team)
            || !(2..=3).contains(&baseline.player_ids.len())
            || baseline.player_ids.contains(&0)
            || baseline
                .player_ids
                .windows(2)
                .any(|pair| pair[0] == pair[1])
            || !valid_share(baseline.expected_xg_share)
            || baseline.method.trim().is_empty()
            || !required_components.is_subset(&baseline.components)
            || baseline.source_fingerprints.is_empty()
            || baseline
                .source_fingerprints
                .iter()
                .any(|value| !valid_fingerprint(value))
            || baseline_by_key
                .insert((baseline.game_id, baseline.player_ids.clone()), baseline)
                .is_some()
        {
            return Err("pregame unit baselines must be unique, complete, sealed, and include individual/opponent/deployment components".into());
        }
    }

    let mut eligible = rows
        .iter()
        .filter(|row| {
            row.team == team
                && row.situation == "5on5"
                && row.date < forecast_at.date_naive()
                && row.ice_time_seconds > 0.0
        })
        .collect::<Vec<_>>();
    eligible.sort_by_key(|row| (row.date, row.game_id, row.player_ids.clone()));
    if eligible.is_empty() {
        return Err("no strictly-prior MoneyPuck 5-on-5 line-game rows for team".into());
    }
    let mut row_keys = BTreeSet::new();
    if eligible
        .iter()
        .any(|row| !row_keys.insert((row.game_id, row.player_ids.clone(), row.situation.clone())))
    {
        return Err("duplicate MoneyPuck unit/game/situation row across line-game inputs".into());
    }

    #[derive(Default)]
    struct Aggregate<'a> {
        rows: Vec<&'a MoneyPuckLineGameRow>,
        baselines: Vec<&'a PregameUnitXgBaseline>,
        observed_share_seconds: f64,
        baseline_seconds: f64,
        seconds: f64,
        games: BTreeSet<u64>,
    }

    let mut aggregates: BTreeMap<Vec<u32>, Aggregate<'_>> = BTreeMap::new();
    let mut excluded_missing_baseline = 0;
    let mut excluded_zero_xg = 0;
    for row in &eligible {
        let key = (row.game_id, row.player_ids.clone());
        let Some(baseline) = baseline_by_key.get(&key) else {
            excluded_missing_baseline += 1;
            continue;
        };
        if baseline.computed_at.date_naive() >= row.date {
            return Err(format!(
                "baseline for game {} was not frozen before the game date",
                row.game_id
            ));
        }
        if row.score_venue_adjusted_xg_for + row.score_venue_adjusted_xg_against <= 0.0 {
            excluded_zero_xg += 1;
            continue;
        }
        let aggregate = aggregates.entry(row.player_ids.clone()).or_default();
        aggregate.rows.push(row);
        aggregate.baselines.push(baseline);
        aggregate.observed_share_seconds += row.score_venue_adjusted_xg_for
            / (row.score_venue_adjusted_xg_for + row.score_venue_adjusted_xg_against)
            * row.ice_time_seconds;
        aggregate.baseline_seconds += baseline.expected_xg_share * row.ice_time_seconds;
        aggregate.seconds += row.ice_time_seconds;
        aggregate.games.insert(row.game_id);
    }

    let mut outcomes = Vec::new();
    let mut units_below_minimum = 0;
    for (player_ids, aggregate) in aggregates {
        let shared_minutes = aggregate.seconds / 60.0;
        if shared_minutes < minimum_shared_minutes {
            units_below_minimum += 1;
            continue;
        }
        let latest_date = aggregate
            .rows
            .iter()
            .map(|row| row.date)
            .max()
            .expect("aggregate has rows");
        let evidence_cutoff_at = latest_date
            .and_hms_opt(23, 59, 59)
            .expect("valid end of day")
            .and_utc();
        outcomes.push(ShiftAdjustedUnitOutcomeInput {
            schema: SHIFT_ADJUSTED_UNIT_OUTCOME_SCHEMA.to_owned(),
            team: team.clone(),
            player_ids,
            evidence_cutoff_at,
            shared_games: aggregate.games.len() as u32,
            shared_minutes,
            observed_xg_share: aggregate.observed_share_seconds / aggregate.seconds,
            baseline_xg_share: aggregate.baseline_seconds / aggregate.seconds,
            deployment_affinity: None,
            outcome_source_fingerprint: fingerprint(&aggregate.rows)?,
            baseline_source_fingerprint: fingerprint(&aggregate.baselines)?,
        });
    }
    if outcomes.is_empty() {
        return Err("no pair/trio cleared the baseline and shared-minute gates".into());
    }

    let modeled_rows = eligible.len() - excluded_missing_baseline - excluded_zero_xg;
    let chemistry = build_shift_adjusted_chemistry_evidence(&team, forecast_at, outcomes)?;
    let mut view = MoneyPuckLineChemistryView {
        schema: MONEYPUCK_LINE_CHEMISTRY_SCHEMA.to_owned(),
        team,
        forecast_at,
        eligible_5v5_rows: eligible.len(),
        modeled_rows,
        excluded_missing_baseline,
        excluded_zero_xg,
        baseline_coverage: modeled_rows as f64 / eligible.len() as f64,
        minimum_shared_minutes,
        units_below_minimum,
        chemistry,
        disclosures: vec![
            "Observed results use MoneyPuck score-and-venue-adjusted 5-on-5 xG from games strictly before the forecast date.".to_owned(),
            "Every modeled game requires a separately sealed pregame baseline with individual, opponent, and deployment components.".to_owned(),
            "A unit residual is evidence for validation, not proof that shared deployment caused the result.".to_owned(),
            "MoneyPuck data requires source credit and is subject to its published usage terms.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    view.fingerprint = fingerprint(&view)?;
    Ok(view)
}

fn valid_share(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use icelines_sources::moneypuck_line_game::parse_moneypuck_line_games;

    use super::*;

    const HEADER: &str = "lineId,name,gameId,playerTeam,opposingTeam,home_or_away,gameDate,position,situation,icetime,scoreVenueAdjustedxGoalsFor,scoreVenueAdjustedxGoalsAgainst\n";

    fn seal(character: char) -> String {
        format!("sha256:{}", character.to_string().repeat(64))
    }

    fn baseline(game_id: u64, computed_day: u32, expected_xg_share: f64) -> PregameUnitXgBaseline {
        PregameUnitXgBaseline {
            schema: PREGAME_UNIT_XG_BASELINE_SCHEMA.to_owned(),
            game_id,
            team: "NYR".to_owned(),
            player_ids: vec![8_477_987, 8_481_624, 8_484_144],
            computed_at: Utc.timestamp_opt(computed_day as i64 * 86_400, 0).unwrap(),
            expected_xg_share,
            components: BTreeSet::from([
                UnitBaselineComponent::Individual,
                UnitBaselineComponent::Opponent,
                UnitBaselineComponent::Deployment,
            ]),
            method: "frozen-test-baseline".to_owned(),
            source_fingerprints: vec![seal('a')],
        }
    }

    #[test]
    fn aggregates_only_strictly_prior_games_against_pregame_baselines() {
        let csv = format!(
            "{HEADER}847798784816248484144,Donato-Bedard-Mikheyev,2025020008,NYR,BOS,AWAY,20251009,line,5on5,600,0.7,0.3\n\
             847798784816248484144,Donato-Bedard-Mikheyev,2025020048,NYR,UTA,HOME,20251013,line,5on5,600,0.4,0.6\n\
             847798784816248484144,Donato-Bedard-Mikheyev,2025020050,NYR,SEA,HOME,20251014,line,5on5,600,9.0,0.0\n"
        );
        let rows = parse_moneypuck_line_games(&csv).unwrap();
        let baselines = vec![
            baseline(2025020008, 20_369, 0.50),
            baseline(2025020048, 20_373, 0.52),
        ];
        let forecast_at = Utc.with_ymd_and_hms(2025, 10, 14, 12, 0, 0).unwrap();
        let view = build_moneypuck_line_chemistry("NYR", forecast_at, &rows, baselines, 5.0)
            .expect("valid frozen chemistry");
        assert_eq!(view.eligible_5v5_rows, 2);
        assert_eq!(view.modeled_rows, 2);
        assert!((view.chemistry.evidence[0].performance_residual.unwrap() - 0.08).abs() < 1e-9);
    }

    #[test]
    fn rejects_same_day_baseline_and_missing_components() {
        let csv = format!(
            "{HEADER}847798784816248484144,Donato-Bedard-Mikheyev,2025020008,NYR,BOS,AWAY,20251009,line,5on5,600,0.7,0.3\n"
        );
        let rows = parse_moneypuck_line_games(&csv).unwrap();
        let forecast_at = Utc.with_ymd_and_hms(2025, 10, 10, 12, 0, 0).unwrap();
        let same_day = PregameUnitXgBaseline {
            computed_at: Utc.with_ymd_and_hms(2025, 10, 9, 1, 0, 0).unwrap(),
            ..baseline(2025020008, 20_369, 0.5)
        };
        assert!(
            build_moneypuck_line_chemistry("NYR", forecast_at, &rows, vec![same_day], 5.0).is_err()
        );

        let mut incomplete = baseline(2025020008, 20_369, 0.5);
        incomplete
            .components
            .remove(&UnitBaselineComponent::Deployment);
        assert!(
            build_moneypuck_line_chemistry("NYR", forecast_at, &rows, vec![incomplete], 5.0)
                .is_err()
        );
    }

    #[test]
    fn rejects_duplicate_rows_across_input_files() {
        let csv = format!(
            "{HEADER}847798784816248484144,Donato-Bedard-Mikheyev,2025020008,NYR,BOS,AWAY,20251009,line,5on5,600,0.7,0.3\n"
        );
        let mut rows = parse_moneypuck_line_games(&csv).unwrap();
        rows.push(rows[0].clone());
        let forecast_at = Utc.with_ymd_and_hms(2025, 10, 10, 12, 0, 0).unwrap();
        assert!(build_moneypuck_line_chemistry(
            "NYR",
            forecast_at,
            &rows,
            vec![baseline(2025020008, 20_369, 0.5)],
            5.0,
        )
        .is_err());
    }
}
