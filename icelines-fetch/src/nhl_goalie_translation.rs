//! Builds a sealed career-paired AHL-to-NHL goalie evaluation ledger.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    calibrate_nhl_goalie_translation, estimate_nhl_goalie_quality, CareerGameType,
    NhlGoalieTranslationCalibration, NhlGoalieTranslationEstimate, NhlGoalieTranslationPair,
    NhlGoalieTranslationPolicy,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::career_landing::CareerHistoryStore;

pub const NHL_GOALIE_TRANSLATION_LEDGER_SCHEMA: &str = "nhl_goalie_translation_ledger.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NhlGoalieTranslationLedgerRow {
    pub player_id: u32,
    pub estimate: NhlGoalieTranslationEstimate,
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NhlGoalieTranslationUnavailableRow {
    pub player_id: u32,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NhlGoalieTranslationLedgerView {
    pub schema: String,
    pub target_season: u32,
    pub stats_season: u32,
    pub career_store_fetched_at: String,
    pub policy: NhlGoalieTranslationPolicy,
    pub calibration_pairs: Vec<NhlGoalieTranslationPair>,
    pub calibration: NhlGoalieTranslationCalibration,
    pub candidates_requested: usize,
    pub candidates_estimated: usize,
    pub candidates_unavailable: usize,
    pub players: Vec<NhlGoalieTranslationLedgerRow>,
    pub unavailable: Vec<NhlGoalieTranslationUnavailableRow>,
    pub disclosures: Vec<String>,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, Default)]
struct GoalieAggregate {
    games: u32,
    shots: u32,
    saves: f64,
}

impl GoalieAggregate {
    fn save_percentage(&self) -> Option<f64> {
        (self.shots > 0).then(|| self.saves / f64::from(self.shots))
    }
}

pub fn build_nhl_goalie_translation_ledger(
    career_store: &CareerHistoryStore,
    target_season: u32,
    stats_season: u32,
    candidate_ids: impl IntoIterator<Item = u32>,
    policy: &NhlGoalieTranslationPolicy,
) -> Result<NhlGoalieTranslationLedgerView, String> {
    if target_season != advance_season(stats_season, 1)? {
        return Err(
            "goalie translation target must immediately follow the stats season".to_owned(),
        );
    }
    let fetched_at = career_store
        .fetched_at
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "goalie translation requires a dated official career store".to_owned())?;
    if career_store.schema_version == 0 {
        return Err("goalie translation requires a supported career store".to_owned());
    }
    let aggregates = aggregate_goalie_careers(career_store, stats_season)?;
    let pairs = calibration_pairs(&aggregates, policy)?;
    let calibration = calibrate_nhl_goalie_translation(policy, &pairs)?;
    let candidates = candidate_ids.into_iter().collect::<BTreeSet<_>>();
    if candidates.contains(&0) {
        return Err("goalie translation candidate IDs must be positive".to_owned());
    }
    let mut players = Vec::new();
    let mut unavailable = Vec::new();
    for player_id in &candidates {
        let source = most_recent_ahl_source(
            &aggregates,
            *player_id,
            stats_season,
            policy.maximum_source_lookback_seasons,
        );
        let Some((source_season, aggregate)) = source else {
            unavailable.push(NhlGoalieTranslationUnavailableRow {
                player_id: *player_id,
                reason: "no recent AHL regular-season goalie sample".to_owned(),
            });
            continue;
        };
        let Some(save_percentage) = aggregate.save_percentage() else {
            unavailable.push(NhlGoalieTranslationUnavailableRow {
                player_id: *player_id,
                reason: "recent AHL sample has no shot-based save percentage".to_owned(),
            });
            continue;
        };
        match estimate_nhl_goalie_quality(
            policy,
            &calibration,
            source_season,
            aggregate.games,
            aggregate.shots,
            save_percentage,
        ) {
            Ok(estimate) => players.push(NhlGoalieTranslationLedgerRow {
                player_id: *player_id,
                estimate,
                source_url: format!("https://api-web.nhle.com/v1/player/{player_id}/landing"),
            }),
            Err(reason) => unavailable.push(NhlGoalieTranslationUnavailableRow {
                player_id: *player_id,
                reason,
            }),
        }
    }
    players.sort_by_key(|row| row.player_id);
    unavailable.sort_by_key(|row| row.player_id);
    let mut ledger = NhlGoalieTranslationLedgerView {
        schema: NHL_GOALIE_TRANSLATION_LEDGER_SCHEMA.to_owned(),
        target_season,
        stats_season,
        career_store_fetched_at: fetched_at.to_owned(),
        policy: policy.clone(),
        calibration_pairs: pairs,
        calibration,
        candidates_requested: candidates.len(),
        candidates_estimated: players.len(),
        candidates_unavailable: unavailable.len(),
        players,
        unavailable,
        disclosures: vec![
            "Evaluation-only AHL-to-NHL goalie translation; not a universal equivalency or roster decision.".to_owned(),
            "Observed NHL goalie quality always takes precedence; calibration confidence discounts workload before prior shrinkage.".to_owned(),
        ],
        source_fingerprint: String::new(),
    };
    ledger.source_fingerprint = fingerprint(&ledger)?;
    Ok(ledger)
}

pub fn validate_nhl_goalie_translation_ledger(
    ledger: &NhlGoalieTranslationLedgerView,
) -> Result<(), String> {
    if ledger.schema != NHL_GOALIE_TRANSLATION_LEDGER_SCHEMA
        || ledger.candidates_requested
            != ledger.candidates_estimated + ledger.candidates_unavailable
        || ledger.candidates_estimated != ledger.players.len()
        || ledger.candidates_unavailable != ledger.unavailable.len()
        || ledger.source_fingerprint != fingerprint(ledger)?
    {
        return Err("invalid or tampered NHL goalie translation ledger".to_owned());
    }
    if ledger.target_season != advance_season(ledger.stats_season, 1)? {
        return Err("goalie translation ledger has invalid season axes".to_owned());
    }
    let replayed_calibration =
        calibrate_nhl_goalie_translation(&ledger.policy, &ledger.calibration_pairs)?;
    if replayed_calibration != ledger.calibration {
        return Err("goalie translation calibration does not replay".to_owned());
    }
    let mut ids = BTreeSet::new();
    for row in &ledger.players {
        if !ids.insert(row.player_id) {
            return Err("goalie translation ledger contains duplicate players".to_owned());
        }
        let replayed = estimate_nhl_goalie_quality(
            &ledger.policy,
            &ledger.calibration,
            row.estimate.source_season,
            row.estimate.source_games,
            row.estimate.source_shots,
            row.estimate.source_save_percentage,
        )?;
        if replayed != row.estimate {
            return Err("goalie translation player estimate does not replay".to_owned());
        }
    }
    for row in &ledger.unavailable {
        if !ids.insert(row.player_id) {
            return Err("goalie translation player appears in multiple result states".to_owned());
        }
    }
    Ok(())
}

fn aggregate_goalie_careers(
    store: &CareerHistoryStore,
    through_season: u32,
) -> Result<BTreeMap<(u32, String, u32), GoalieAggregate>, String> {
    let mut rows = BTreeMap::<(u32, String, u32), GoalieAggregate>::new();
    for history in store.histories.values() {
        for stint in &history.stints {
            if stint.game_type != CareerGameType::Regular || stint.season.0 > through_season {
                continue;
            }
            let (Some(save_percentage), Some(shots)) = (stint.save_pct, stint.shots_against) else {
                continue;
            };
            if shots == 0 || !(0.0..=1.0).contains(&save_percentage) {
                continue;
            }
            let league = stint.league.as_str().trim().to_ascii_uppercase();
            if !matches!(league.as_str(), "AHL" | "NHL") {
                continue;
            }
            let row = rows
                .entry((history.player_id, league, stint.season.0))
                .or_default();
            row.games = row
                .games
                .checked_add(stint.gp)
                .ok_or_else(|| "goalie career games overflow".to_owned())?;
            row.shots = row
                .shots
                .checked_add(shots)
                .ok_or_else(|| "goalie career shots overflow".to_owned())?;
            row.saves += f64::from(save_percentage) * f64::from(shots);
        }
    }
    Ok(rows)
}

fn calibration_pairs(
    rows: &BTreeMap<(u32, String, u32), GoalieAggregate>,
    policy: &NhlGoalieTranslationPolicy,
) -> Result<Vec<NhlGoalieTranslationPair>, String> {
    let mut pairs = Vec::new();
    for ((player_id, league, ahl_season), ahl) in rows {
        if league != "AHL" {
            continue;
        }
        let mut matched = None;
        for gap in 0..=policy.maximum_pair_season_gap {
            let nhl_season = advance_season(*ahl_season, gap)?;
            if let Some(nhl) = rows.get(&(*player_id, "NHL".to_owned(), nhl_season)) {
                matched = Some((nhl_season, nhl));
                break;
            }
        }
        let Some((nhl_season, nhl)) = matched else {
            continue;
        };
        let paired_shots = ahl.shots.min(nhl.shots);
        if paired_shots < policy.minimum_pair_shots {
            continue;
        }
        pairs.push(NhlGoalieTranslationPair {
            player_id: *player_id,
            ahl_season: *ahl_season,
            nhl_season,
            ahl_save_percentage: ahl
                .save_percentage()
                .ok_or_else(|| "AHL goalie aggregate has no rate".to_owned())?,
            nhl_save_percentage: nhl
                .save_percentage()
                .ok_or_else(|| "NHL goalie aggregate has no rate".to_owned())?,
            paired_shots,
        });
    }
    Ok(pairs)
}

fn most_recent_ahl_source(
    rows: &BTreeMap<(u32, String, u32), GoalieAggregate>,
    player_id: u32,
    stats_season: u32,
    maximum_lookback: u32,
) -> Option<(u32, &GoalieAggregate)> {
    (0..maximum_lookback).find_map(|gap| {
        let season = retreat_season(stats_season, gap)?;
        rows.get(&(player_id, "AHL".to_owned(), season))
            .map(|row| (season, row))
    })
}

fn advance_season(season: u32, gap: u32) -> Result<u32, String> {
    season
        .checked_add(gap.saturating_mul(10_001))
        .ok_or_else(|| "career season advancement overflow".to_owned())
}

fn retreat_season(season: u32, gap: u32) -> Option<u32> {
    season.checked_sub(gap.saturating_mul(10_001))
}

fn fingerprint(ledger: &NhlGoalieTranslationLedgerView) -> Result<String, String> {
    let mut canonical = ledger.clone();
    canonical.source_fingerprint.clear();
    let bytes = serde_json::to_vec(&canonical).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{CareerHistory, CareerStint, LeagueAbbrev, Season};

    fn stint(season: u32, league: &str, save_pct: f32) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: league.to_owned(),
            game_type: CareerGameType::Regular,
            sequence: 0,
            gp: 20,
            goals: None,
            assists: None,
            points: None,
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: Some(18),
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: Some(save_pct),
            shots_against: Some(400),
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    fn store() -> CareerHistoryStore {
        let mut store = CareerHistoryStore::new();
        store.fetched_at = Some("2026-07-29T00:00:00Z".to_owned());
        for player_id in 1..=20 {
            store.histories.insert(
                player_id.to_string(),
                CareerHistory {
                    player_id,
                    stints: vec![stint(20242025, "AHL", 0.915), stint(20252026, "NHL", 0.905)],
                },
            );
        }
        store.histories.insert(
            "99".to_owned(),
            CareerHistory {
                player_id: 99,
                stints: vec![stint(20252026, "AHL", 0.920)],
            },
        );
        store
    }

    #[test]
    fn l1_builds_sealed_estimate_without_overwriting_observed_nhl_logic() {
        let ledger = build_nhl_goalie_translation_ledger(
            &store(),
            20262027,
            20252026,
            [99],
            &NhlGoalieTranslationPolicy::default(),
        )
        .unwrap();
        validate_nhl_goalie_translation_ledger(&ledger).unwrap();
        assert_eq!(ledger.candidates_estimated, 1);
        assert_eq!(ledger.players[0].player_id, 99);
        assert!(ledger.players[0].estimate.evidence_confidence < 1.0);
    }

    #[test]
    fn l1_tamper_and_missing_candidate_are_visible() {
        let mut ledger = build_nhl_goalie_translation_ledger(
            &store(),
            20262027,
            20252026,
            [100],
            &NhlGoalieTranslationPolicy::default(),
        )
        .unwrap();
        assert_eq!(ledger.candidates_unavailable, 1);
        ledger.unavailable[0].reason = "changed".to_owned();
        assert!(validate_nhl_goalie_translation_ledger(&ledger).is_err());
    }

    #[test]
    fn l1_resealed_formula_tamper_is_rejected_by_canonical_replay() {
        let mut ledger = build_nhl_goalie_translation_ledger(
            &store(),
            20262027,
            20252026,
            [99],
            &NhlGoalieTranslationPolicy::default(),
        )
        .unwrap();
        ledger.players[0].estimate.goalie_quality_score += 10.0;
        ledger.source_fingerprint = fingerprint(&ledger).unwrap();
        assert!(validate_nhl_goalie_translation_ledger(&ledger).is_err());
    }
}
