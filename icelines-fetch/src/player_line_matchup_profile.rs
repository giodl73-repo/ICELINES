//! Point-in-time adapter from existing IceLines lineup, role, and official
//! shift evidence into the player profiles consumed by The Matchup.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use icelines_core::{
    LineChemistryEvidenceInput, LineChemistryEvidenceKind, PlayerForecastProfileDimensions,
    PlayerForecastProfileInput, TeamCeilingLens, TeamLineupPlayerView, TeamLineupProjectionView,
    TeamPlayerMatchupRoleEvidenceView, LINE_CHEMISTRY_EVIDENCE_SCHEMA,
    PLAYER_FORECAST_PROFILE_SCHEMA, TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::shift_chart::{ShiftOverlapReport, SHIFT_OVERLAP_SCHEMA};

pub const PLAYER_LINE_MATCHUP_PROFILE_ADAPTER_SCHEMA: &str =
    "player_line_matchup_profile_adapter.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupProfileAdapterView {
    pub schema: String,
    pub team: String,
    pub lineup_season: u32,
    pub evidence_season: u32,
    pub evidence_cutoff_at: DateTime<Utc>,
    pub dressed_skaters: usize,
    pub profiles_built: usize,
    pub coverage: f64,
    pub profiles: Vec<PlayerForecastProfileInput>,
    /// Exact shared deployment converted to non-causal chemistry inputs.
    pub deployment_chemistry: Vec<LineChemistryEvidenceInput>,
    pub missing_role_player_ids: Vec<u32>,
    pub missing_shift_player_ids: Vec<u32>,
    pub warnings: Vec<String>,
    pub disclosures: Vec<String>,
    pub source_fingerprints: Vec<String>,
}

pub fn build_player_line_matchup_profiles(
    lineup: &TeamLineupProjectionView,
    role_evidence: &TeamPlayerMatchupRoleEvidenceView,
    shift_overlap: &ShiftOverlapReport,
    evidence_cutoff_at: DateTime<Utc>,
    recency: f64,
    source_fingerprints: Vec<String>,
) -> Result<PlayerLineMatchupProfileAdapterView, String> {
    let team = lineup.team.trim().to_ascii_uppercase();
    if role_evidence.schema != TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA
        || shift_overlap.schema != SHIFT_OVERLAP_SCHEMA
        || !role_evidence.team.eq_ignore_ascii_case(&team)
        || !shift_overlap.team.eq_ignore_ascii_case(&team)
        || role_evidence.season != shift_overlap.season
        || !recency.is_finite()
        || !(0.0..=1.0).contains(&recency)
        || source_fingerprints.is_empty()
        || source_fingerprints
            .iter()
            .any(|fingerprint| !valid_fingerprint(fingerprint))
    {
        return Err(
            "player-line profile adapter requires matching lineup/role/shift authority and valid seals"
                .to_owned(),
        );
    }
    let players = dressed_skaters(lineup)?;
    let shift_report_fingerprint = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(shift_overlap).map_err(|error| error.to_string())?)
    );
    let roles = role_evidence
        .roles
        .iter()
        .map(|row| (row.role.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let shifts = shift_overlap
        .players
        .iter()
        .map(|row| (row.player_id, row))
        .collect::<BTreeMap<_, _>>();
    let mut profiles = Vec::new();
    let mut missing_role_player_ids = Vec::new();
    let mut missing_shift_player_ids = Vec::new();
    for player in players.values() {
        let Some(role) = roles.get(&player.player_id) else {
            missing_role_player_ids.push(player.player_id);
            continue;
        };
        let Some(shift) = shifts.get(&player.player_id) else {
            missing_shift_player_ids.push(player.player_id);
            continue;
        };
        if shift.shift_intervals == 0 || role.even_strength_toi_seconds == 0 {
            missing_shift_player_ids.push(player.player_id);
            continue;
        }
        let points = lens(player, TeamCeilingLens::PointsPace);
        let goals = lens(player, TeamCeilingLens::GoalScoring);
        let fantasy = lens(player, TeamCeilingLens::Fantasy);
        let dimensions = PlayerForecastProfileDimensions {
            scoring_creation: points,
            finishing: goals,
            passing_transition: mean_options(&[points, Some(role.role.transition_score)]),
            forecheck_retrieval: Some(role.role.forecheck_score),
            defensive_suppression: Some(role.role.defensive_score),
            physical_matchup: mean_options(&[Some(role.role.physical_score), fantasy]),
            // The current role adapter's transition score explicitly includes
            // takeaway/giveaway security; keep the proxy labeled in disclosures.
            discipline_puck_security: Some(role.role.transition_score),
            faceoffs: None,
            power_play: player.power_play_role_score,
            penalty_kill: player.penalty_kill_role_score,
        };
        let games_played = [
            player.score.sample_games,
            role.role.evidence_games,
            shift.games,
        ]
        .into_iter()
        .filter(|games| *games > 0)
        .min()
        .unwrap_or(0);
        if games_played == 0 {
            missing_shift_player_ids.push(player.player_id);
            continue;
        }
        profiles.push(PlayerForecastProfileInput {
            schema: PLAYER_FORECAST_PROFILE_SCHEMA.to_owned(),
            player_id: player.player_id,
            team: team.clone(),
            evidence_cutoff_at,
            games_played,
            even_strength_minutes: f64::from(role.even_strength_toi_seconds) / 60.0,
            observed_shifts: shift.shift_intervals,
            recency,
            dimensions,
            source_fingerprints: source_fingerprints.clone(),
        });
    }
    profiles.sort_by_key(|row| row.player_id);
    missing_role_player_ids.sort_unstable();
    missing_shift_player_ids.sort_unstable();
    missing_shift_player_ids.dedup();
    let coverage = profiles.len() as f64 / players.len() as f64;
    let player_ids = players.keys().copied().collect::<BTreeSet<_>>();
    let mut deployment_chemistry = shift_overlap
        .pairs
        .iter()
        .filter(|row| {
            row.shared_games >= 5
                && row.shared_seconds >= 300
                && player_ids.contains(&row.player_one_id)
                && player_ids.contains(&row.player_two_id)
        })
        .map(|row| LineChemistryEvidenceInput {
            schema: LINE_CHEMISTRY_EVIDENCE_SCHEMA.to_owned(),
            player_ids: vec![row.player_one_id, row.player_two_id],
            team: team.clone(),
            evidence_cutoff_at,
            shared_games: row.shared_games,
            shared_minutes: row.shared_seconds as f64 / 60.0,
            performance_residual: None,
            deployment_affinity: Some(row.lower_player_overlap_pct),
            kind: LineChemistryEvidenceKind::ShiftDeployment,
            source_fingerprint: shift_report_fingerprint.clone(),
        })
        .collect::<Vec<_>>();
    deployment_chemistry.extend(
        shift_overlap
            .trios
            .iter()
            .filter(|row| {
                row.shared_games >= 5
                    && row.shared_seconds >= 300
                    && row.player_ids.iter().all(|id| player_ids.contains(id))
            })
            .map(|row| LineChemistryEvidenceInput {
                schema: LINE_CHEMISTRY_EVIDENCE_SCHEMA.to_owned(),
                player_ids: row.player_ids.to_vec(),
                team: team.clone(),
                evidence_cutoff_at,
                shared_games: row.shared_games,
                shared_minutes: row.shared_seconds as f64 / 60.0,
                performance_residual: None,
                deployment_affinity: None,
                kind: LineChemistryEvidenceKind::ShiftDeployment,
                source_fingerprint: shift_report_fingerprint.clone(),
            }),
    );
    deployment_chemistry.sort_by(|left, right| left.player_ids.cmp(&right.player_ids));
    let mut warnings = Vec::new();
    if coverage < 0.75 {
        warnings.push(
            "Fewer than 75% of dressed skaters have both role and exact-shift evidence; The Matchup will withhold its edge feature."
                .to_owned(),
        );
    }
    if !missing_role_player_ids.is_empty() {
        warnings.push(format!(
            "Missing role evidence for player IDs: {}",
            join_ids(&missing_role_player_ids)
        ));
    }
    if !missing_shift_player_ids.is_empty() {
        warnings.push(format!(
            "Missing exact shift-volume evidence for player IDs: {}",
            join_ids(&missing_shift_player_ids)
        ));
    }
    Ok(PlayerLineMatchupProfileAdapterView {
        schema: PLAYER_LINE_MATCHUP_PROFILE_ADAPTER_SCHEMA.to_owned(),
        team,
        lineup_season: lineup.roster_season,
        evidence_season: role_evidence.season,
        evidence_cutoff_at,
        dressed_skaters: players.len(),
        profiles_built: profiles.len(),
        coverage: round9(coverage),
        profiles,
        deployment_chemistry,
        missing_role_player_ids,
        missing_shift_player_ids,
        warnings,
        disclosures: vec![
            "Scoring creation and finishing use the existing points-pace and goal-scoring lenses; upside is excluded from current-game quality.".to_owned(),
            "Defense, transition, forecheck, and physical dimensions use league/position-relative role evidence.".to_owned(),
            "Even-strength minutes come from official season TOI; observed shift count comes from exact official shift-chart intervals.".to_owned(),
            "Discipline/puck security remains a takeaway/giveaway transition proxy until a dedicated event profile is available.".to_owned(),
            "This adapter builds player profiles only; it does not infer chemistry from shared deployment.".to_owned(),
        ],
        source_fingerprints: {
            let mut fingerprints = source_fingerprints;
            fingerprints.push(shift_report_fingerprint);
            fingerprints.sort();
            fingerprints.dedup();
            fingerprints
        },
    })
}

fn dressed_skaters(
    lineup: &TeamLineupProjectionView,
) -> Result<BTreeMap<u32, &TeamLineupPlayerView>, String> {
    let mut players = BTreeMap::new();
    for player in lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .flatten()
    {
        if players.insert(player.player_id, player).is_some() {
            return Err("player-line profile adapter found duplicate dressed skaters".to_owned());
        }
    }
    if players.len() != 18 {
        return Err("player-line profile adapter requires a complete 12F/6D lineup".to_owned());
    }
    Ok(players)
}

fn lens(player: &TeamLineupPlayerView, lens: TeamCeilingLens) -> Option<f64> {
    player
        .score
        .components
        .iter()
        .find(|component| component.lens == lens)
        .and_then(|component| component.normalized_value)
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
}

fn mean_options(values: &[Option<f64>]) -> Option<f64> {
    let values = values.iter().flatten().copied().collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn valid_fingerprint(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn join_ids(ids: &[u32]) -> String {
    ids.iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn round9(value: f64) -> f64 {
    (value * 1_000_000_000.0).round() / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use crate::shift_chart::{ShiftOverlapPairRow, ShiftOverlapPlayerRow};
    use chrono::TimeZone;
    use icelines_core::{
        build_player_matchup_role_evidence, PlayerRoleSeasonFactsInput,
        TeamPlayerMatchupRoleEvidenceView,
    };

    use super::*;

    #[test]
    fn adapter_refuses_mismatched_authority_before_profile_math() {
        let lineup: TeamLineupProjectionView = serde_json::from_value(serde_json::json!({
            "schema": "team_lineup_projection.v1",
            "score_schema": "icelines_player_score.v1",
            "score_method": "team_ceiling_multilens.v1",
            "team": "NYR",
            "roster_season": 20262027,
            "assignment_evidence": [],
            "forward_lines": [],
            "defense_pairs": [],
            "goalies": {"starter": null, "backup": null},
            "special_teams": {"power_play": [], "penalty_kill": [], "warnings": []},
            "extras": [],
            "warnings": []
        }))
        .unwrap();
        let roles = TeamPlayerMatchupRoleEvidenceView {
            schema: TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA.to_owned(),
            team: "SEA".to_owned(),
            season: 20252026,
            season_type: icelines_core::season_stats::SeasonType::Regular,
            roster_skaters: 0,
            rated_skaters: 0,
            league_forward_peers: 0,
            league_defense_peers: 0,
            roles: vec![],
            warnings: vec![],
            disclosures: vec![],
        };
        let shifts = ShiftOverlapReport {
            schema: SHIFT_OVERLAP_SCHEMA.to_owned(),
            source: "test".to_owned(),
            team: "NYR".to_owned(),
            season: 20252026,
            games_requested: 0,
            games_loaded: 0,
            players: vec![],
            pairs: vec![],
            trios: vec![],
            disclosures: vec![],
        };
        let error = build_player_line_matchup_profiles(
            &lineup,
            &roles,
            &shifts,
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
            1.0,
            vec![format!("sha256:{}", "a".repeat(64))],
        )
        .unwrap_err();
        assert!(error.contains("matching lineup/role/shift authority"));
    }

    #[test]
    fn role_builder_retains_even_strength_minutes_for_profile_confidence() {
        let rows = build_player_matchup_role_evidence(&[
            PlayerRoleSeasonFactsInput {
                player_id: 1,
                is_defenseman: false,
                games_played: 20,
                even_strength_toi_seconds: 12_000,
                short_handed_toi_seconds: 200,
                hits: 20,
                blocked_shots: 10,
                takeaways: 8,
                giveaways: 6,
                offensive_zone_start_pct: Some(50.0),
            },
            PlayerRoleSeasonFactsInput {
                player_id: 2,
                is_defenseman: false,
                games_played: 20,
                even_strength_toi_seconds: 10_000,
                short_handed_toi_seconds: 100,
                hits: 10,
                blocked_shots: 5,
                takeaways: 4,
                giveaways: 8,
                offensive_zone_start_pct: Some(50.0),
            },
        ])
        .unwrap();
        assert_eq!(rows[0].even_strength_toi_seconds, 12_000);
        assert_eq!(rows[0].short_handed_toi_seconds, 200);
    }

    #[test]
    fn adapter_builds_complete_profiles_and_noncausal_deployment_evidence() {
        let lineup: TeamLineupProjectionView =
            serde_json::from_str(include_str!("../../examples/team-lineup-nyr-2026-27.json"))
                .unwrap();
        let players = dressed_skaters(&lineup).unwrap();
        let facts = players
            .values()
            .map(|player| PlayerRoleSeasonFactsInput {
                player_id: player.player_id,
                is_defenseman: player.primary_position == icelines_core::model::Position::Defense,
                games_played: 82,
                even_strength_toi_seconds: 60_000,
                short_handed_toi_seconds: 1_000,
                hits: 80,
                blocked_shots: 50,
                takeaways: 30,
                giveaways: 25,
                offensive_zone_start_pct: Some(50.0),
            })
            .collect::<Vec<_>>();
        let roles = build_player_matchup_role_evidence(&facts).unwrap();
        let role_view = TeamPlayerMatchupRoleEvidenceView {
            schema: TEAM_PLAYER_MATCHUP_ROLE_EVIDENCE_SCHEMA.to_owned(),
            team: "NYR".to_owned(),
            season: 20252026,
            season_type: icelines_core::season_stats::SeasonType::Regular,
            roster_skaters: 18,
            rated_skaters: 18,
            league_forward_peers: 12,
            league_defense_peers: 6,
            roles,
            warnings: vec![],
            disclosures: vec![],
        };
        let ids = players.keys().copied().collect::<Vec<_>>();
        let shift_view = ShiftOverlapReport {
            schema: SHIFT_OVERLAP_SCHEMA.to_owned(),
            source: "official-test".to_owned(),
            team: "NYR".to_owned(),
            season: 20252026,
            games_requested: 82,
            games_loaded: 82,
            players: ids
                .iter()
                .map(|id| ShiftOverlapPlayerRow {
                    player_id: *id,
                    display_name: format!("Player {id}"),
                    games: 82,
                    shift_intervals: 1_200,
                    ice_seconds: 60_000,
                })
                .collect(),
            pairs: vec![ShiftOverlapPairRow {
                player_one_id: ids[0],
                player_two_id: ids[1],
                shared_games: 60,
                shared_seconds: 30_000,
                lower_player_overlap_pct: 0.5,
            }],
            trios: vec![],
            disclosures: vec![],
        };
        let view = build_player_line_matchup_profiles(
            &lineup,
            &role_view,
            &shift_view,
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
            1.0,
            vec![format!("sha256:{}", "a".repeat(64))],
        )
        .unwrap();
        assert_eq!(view.profiles_built, 18);
        assert_eq!(view.coverage, 1.0);
        assert_eq!(view.deployment_chemistry.len(), 1);
        assert_eq!(
            view.deployment_chemistry[0].kind,
            LineChemistryEvidenceKind::ShiftDeployment
        );
        assert!(view.deployment_chemistry[0].performance_residual.is_none());
    }
}
