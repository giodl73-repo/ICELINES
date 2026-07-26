//! Reviewed AHL season facts to UI-neutral prospect-discovery studies.
//!
//! Production comes from official AHL snapshots, identity comes only from
//! reviewed crosswalk rows, and analyst context stays separately authored.

use std::collections::{BTreeMap, BTreeSet};

use icelines_core::{
    build_prospect_development_study, build_prospect_discovery_board, ProspectAvailabilityStatus,
    ProspectDevelopmentSeasonInput, ProspectDevelopmentStudyConfig, ProspectDevelopmentStudyView,
    ProspectDiscoveryBoardView, ProspectOpportunityStatus, ProspectStudyEvidenceInput,
};
use serde::{Deserialize, Serialize};

use crate::ahl::{
    AhlIdentityCrosswalkView, AhlIdentityReviewStatus, AhlRosterStatsSnapshot,
    AHL_IDENTITY_CROSSWALK_SCHEMA, AHL_ROSTER_STATS_SCHEMA,
};

pub const PROSPECT_LEAGUE_CONTEXT_SCHEMA: &str = "prospect_league_context.v1";
pub const PROSPECT_LEAGUE_DISCOVERY_SCHEMA: &str = "prospect_league_discovery.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectLeaguePlayerContext {
    pub player_id: u32,
    pub player: String,
    pub organization: String,
    pub position: String,
    pub age: u8,
    pub nhl_games_played: u32,
    pub opportunity: ProspectOpportunityStatus,
    pub availability: ProspectAvailabilityStatus,
    pub attention_score: f64,
    pub attention_basis: String,
    #[serde(default)]
    pub evidence: Vec<ProspectStudyEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectLeagueContext {
    pub schema: String,
    pub players: Vec<ProspectLeaguePlayerContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectLeagueExclusionReason {
    MissingReviewedIdentity,
    MissingAhlSkaterStats,
    FewerThanTwoAhlSeasons,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectLeagueExclusionView {
    pub player_id: u32,
    pub player: String,
    pub reason: ProspectLeagueExclusionReason,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectLeagueDiscoveryView {
    pub schema: String,
    pub snapshot_seasons: Vec<u32>,
    pub context_players: usize,
    pub studies: Vec<ProspectDevelopmentStudyView>,
    pub excluded: Vec<ProspectLeagueExclusionView>,
    pub board: ProspectDiscoveryBoardView,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone)]
struct ReviewedSeasonIdentity {
    season: u32,
    ahl_team: String,
    provider_player_id: String,
    display_name: String,
    evidence_urls: Vec<String>,
}

pub fn build_prospect_league_discovery(
    mut snapshots: Vec<AhlRosterStatsSnapshot>,
    crosswalks: Vec<AhlIdentityCrosswalkView>,
    context: ProspectLeagueContext,
    config: ProspectDevelopmentStudyConfig,
) -> Result<ProspectLeagueDiscoveryView, String> {
    validate_authorities(&snapshots, &crosswalks, &context)?;
    snapshots.sort_by_key(|snapshot| snapshot.season);
    let snapshot_seasons = snapshots
        .iter()
        .map(|snapshot| snapshot.season)
        .collect::<Vec<_>>();

    let mut identities = BTreeMap::<u32, Vec<ReviewedSeasonIdentity>>::new();
    let mut reviewed_canonical_keys = BTreeSet::new();
    for crosswalk in &crosswalks {
        for row in crosswalk
            .rows
            .iter()
            .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
        {
            let Some(player_id) = row.nhl_player_id else {
                return Err(format!(
                    "reviewed AHL identity {} has no NHL player ID",
                    row.provider_player_id
                ));
            };
            let display_name = row.nhl_display_name.clone().ok_or_else(|| {
                format!(
                    "reviewed AHL identity {} has no NHL display name",
                    row.provider_player_id
                )
            })?;
            if !reviewed_canonical_keys.insert((
                crosswalk.season,
                crosswalk.ahl_team.as_str(),
                player_id,
            )) {
                return Err(format!(
                    "duplicate reviewed canonical player {} in {} for {}",
                    player_id, crosswalk.ahl_team, crosswalk.season
                ));
            }
            identities
                .entry(player_id)
                .or_default()
                .push(ReviewedSeasonIdentity {
                    season: crosswalk.season,
                    ahl_team: crosswalk.ahl_team.clone(),
                    provider_player_id: row.provider_player_id.clone(),
                    display_name,
                    evidence_urls: row.evidence_urls.clone(),
                });
        }
    }

    let mut studies = Vec::new();
    let mut excluded = Vec::new();
    for player in context.players {
        let Some(player_identities) = identities.get(&player.player_id) else {
            excluded.push(ProspectLeagueExclusionView {
                player_id: player.player_id,
                player: player.player,
                reason: ProspectLeagueExclusionReason::MissingReviewedIdentity,
                detail: "No reviewed AHL-to-NHL identity row matched this player.".to_owned(),
            });
            continue;
        };
        if player_identities.iter().any(|identity| {
            icelines_core::normalize_name(&identity.display_name)
                != icelines_core::normalize_name(&player.player)
        }) {
            return Err(format!(
                "reviewed AHL identity name conflicts with context for player {}",
                player.player_id
            ));
        }

        let mut season_totals = BTreeMap::<u32, (u32, u32, u32)>::new();
        let mut snapshot_evidence = BTreeSet::<(String, String)>::new();
        let mut identity_evidence = BTreeSet::<String>::new();
        for identity in player_identities {
            let snapshot = snapshots
                .iter()
                .find(|snapshot| snapshot.season == identity.season)
                .expect("crosswalk season validated against snapshots");
            let team = snapshot
                .teams
                .iter()
                .find(|team| team.team_name == identity.ahl_team)
                .expect("crosswalk team validated against snapshot");
            for row in team.skaters.iter().filter(|row| {
                row.provider_player_id == identity.provider_player_id && row.games_played > 0
            }) {
                let totals = season_totals.entry(snapshot.season).or_default();
                totals.0 = totals.0.saturating_add(row.games_played);
                totals.1 = totals.1.saturating_add(row.goals);
                totals.2 = totals.2.saturating_add(row.assists);
                snapshot_evidence.insert((
                    format!(
                        "Official AHL {} season snapshot includes {} with {}.",
                        snapshot.season, player.player, identity.ahl_team
                    ),
                    snapshot.source_url.clone(),
                ));
            }
            identity_evidence.extend(identity.evidence_urls.clone());
        }

        if season_totals.is_empty() {
            excluded.push(ProspectLeagueExclusionView {
                player_id: player.player_id,
                player: player.player,
                reason: ProspectLeagueExclusionReason::MissingAhlSkaterStats,
                detail: "Reviewed identity rows did not join to AHL skater season facts."
                    .to_owned(),
            });
            continue;
        }
        if season_totals.len() < 2 {
            excluded.push(ProspectLeagueExclusionView {
                player_id: player.player_id,
                player: player.player,
                reason: ProspectLeagueExclusionReason::FewerThanTwoAhlSeasons,
                detail: format!(
                    "Only {} reviewed AHL season joined; the study requires at least two.",
                    season_totals.len()
                ),
            });
            continue;
        }

        let mut evidence = player.evidence;
        evidence.extend(
            snapshot_evidence
                .into_iter()
                .map(|(label, source_url)| ProspectStudyEvidenceInput { label, source_url }),
        );
        for source_url in identity_evidence {
            evidence.push(ProspectStudyEvidenceInput {
                label: format!(
                    "Reviewed AHL-to-NHL identity evidence for {}.",
                    player.player
                ),
                source_url,
            });
        }
        evidence.sort_by(|left, right| {
            left.source_url
                .cmp(&right.source_url)
                .then_with(|| left.label.cmp(&right.label))
        });
        evidence.dedup_by(|left, right| {
            left.source_url == right.source_url && left.label == right.label
        });
        let seasons = season_totals
            .into_iter()
            .map(
                |(season, (games_played, goals, assists))| ProspectDevelopmentSeasonInput {
                    season,
                    league: "AHL".to_owned(),
                    games_played,
                    goals,
                    assists,
                },
            )
            .collect();
        studies.push(build_prospect_development_study(
            icelines_core::ProspectDevelopmentStudyInput {
                player_id: player.player_id,
                player: player.player,
                organization: player.organization,
                position: player.position,
                age: player.age,
                nhl_games_played: player.nhl_games_played,
                seasons,
                opportunity: player.opportunity,
                availability: player.availability,
                attention_score: player.attention_score,
                attention_basis: player.attention_basis,
                evidence,
            },
            config,
        )?);
    }

    studies.sort_by_key(|study| study.player_id);
    excluded.sort_by_key(|row| row.player_id);
    if studies.is_empty() {
        return Err("no eligible prospect studies remained after reviewed AHL joins".to_owned());
    }
    let board = build_prospect_discovery_board(studies.clone())?;
    Ok(ProspectLeagueDiscoveryView {
        schema: PROSPECT_LEAGUE_DISCOVERY_SCHEMA.to_owned(),
        snapshot_seasons,
        context_players: studies.len() + excluded.len(),
        studies,
        excluded,
        board,
        disclosures: vec![
            "AHL production is joined only through reviewed season/team identity crosswalk rows; provider-local IDs are never treated as NHL IDs.".to_owned(),
            "Organization, position, age, NHL games, opportunity, availability, and public attention remain explicit authored context rather than feed-derived guesses.".to_owned(),
            "Candidates without two joined AHL seasons are reported as exclusions and cannot enter the discovery board.".to_owned(),
            "Multiple reviewed team segments in one season are summed; source snapshot and identity evidence remain attached to each study.".to_owned(),
        ],
    })
}

fn validate_authorities(
    snapshots: &[AhlRosterStatsSnapshot],
    crosswalks: &[AhlIdentityCrosswalkView],
    context: &ProspectLeagueContext,
) -> Result<(), String> {
    if snapshots.len() < 2
        || crosswalks.is_empty()
        || context.schema != PROSPECT_LEAGUE_CONTEXT_SCHEMA
        || context.players.is_empty()
    {
        return Err(
            "prospect league discovery requires two snapshots, reviewed crosswalks, and context"
                .to_owned(),
        );
    }
    let mut seasons = BTreeSet::new();
    for snapshot in snapshots {
        if snapshot.schema != AHL_ROSTER_STATS_SCHEMA
            || snapshot.season == 0
            || snapshot.provider.trim().is_empty()
            || snapshot.source_url.trim().is_empty()
            || !seasons.insert(snapshot.season)
        {
            return Err("invalid or duplicate AHL roster-stats snapshot".to_owned());
        }
    }
    let mut crosswalk_keys = BTreeSet::new();
    for crosswalk in crosswalks {
        let Some(snapshot) = snapshots.iter().find(|row| row.season == crosswalk.season) else {
            return Err(format!(
                "AHL identity crosswalk season {} has no supplied snapshot",
                crosswalk.season
            ));
        };
        if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
            || crosswalk.provider != snapshot.provider
            || !snapshot
                .teams
                .iter()
                .any(|team| team.team_name == crosswalk.ahl_team)
            || !crosswalk_keys.insert((crosswalk.season, crosswalk.ahl_team.as_str()))
        {
            return Err(
                "invalid, duplicate, or snapshot-mismatched AHL identity crosswalk".to_owned(),
            );
        }
    }
    let mut player_ids = BTreeSet::new();
    if context.players.iter().any(|player| {
        player.player_id == 0
            || !player_ids.insert(player.player_id)
            || player.player.trim().is_empty()
            || player.organization.trim().is_empty()
            || player.position.trim().is_empty()
            || !player.attention_score.is_finite()
            || !(0.0..=1.0).contains(&player.attention_score)
            || player.attention_basis.trim().is_empty()
            || player.evidence.iter().any(|item| {
                item.label.trim().is_empty()
                    || !(item.source_url.starts_with("https://")
                        || item.source_url.starts_with("http://"))
            })
    }) {
        return Err("invalid or duplicate prospect league context player".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ahl::{
        AhlIdentityCrosswalkCounts, AhlIdentityCrosswalkRow, AhlIdentityMatchBasis,
        AhlSkaterSeasonRow, AhlTeamRosterStats, AHL_PROVIDER,
    };

    #[test]
    fn reviewed_two_season_facts_build_board_and_report_exclusions() {
        let context = ProspectLeagueContext {
            schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
            players: vec![
                player_context(10, "Joined Prospect"),
                player_context(20, "Missing Prospect"),
            ],
        };
        let view = build_prospect_league_discovery(
            vec![
                snapshot(20242025, 10, 40, 10, 10),
                snapshot(20252026, 10, 40, 20, 20),
            ],
            vec![
                crosswalk(20242025, 10, "Joined Prospect"),
                crosswalk(20252026, 10, "Joined Prospect"),
            ],
            context,
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();

        assert_eq!(view.schema, PROSPECT_LEAGUE_DISCOVERY_SCHEMA);
        assert_eq!(view.context_players, 2);
        assert_eq!(view.studies.len(), 1);
        assert_eq!(view.studies[0].player_id, 10);
        assert_eq!(view.studies[0].seasons[1].points, 40);
        assert_eq!(view.board.hidden_gems[0].player_id, 10);
        assert_eq!(view.excluded.len(), 1);
        assert_eq!(view.excluded[0].player_id, 20);
        assert_eq!(
            view.excluded[0].reason,
            ProspectLeagueExclusionReason::MissingReviewedIdentity
        );
    }

    #[test]
    fn pending_identity_does_not_join() {
        let mut row = crosswalk(20242025, 10, "Joined Prospect");
        row.rows[0].review_status = AhlIdentityReviewStatus::Pending;
        let error = build_prospect_league_discovery(
            vec![
                snapshot(20242025, 10, 40, 10, 10),
                snapshot(20252026, 10, 40, 20, 20),
            ],
            vec![row, crosswalk(20252026, 10, "Joined Prospect")],
            ProspectLeagueContext {
                schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
                players: vec![player_context(10, "Joined Prospect")],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("no eligible prospect studies"));
    }

    #[test]
    fn duplicate_canonical_identity_in_one_team_fails_closed() {
        let mut duplicate = crosswalk(20242025, 10, "Joined Prospect");
        let mut second_row = duplicate.rows[0].clone();
        second_row.provider_player_id = "11".to_owned();
        duplicate.rows.push(second_row);
        let error = build_prospect_league_discovery(
            vec![
                snapshot(20242025, 10, 40, 10, 10),
                snapshot(20252026, 10, 40, 20, 20),
            ],
            vec![duplicate, crosswalk(20252026, 10, "Joined Prospect")],
            ProspectLeagueContext {
                schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
                players: vec![player_context(10, "Joined Prospect")],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap_err();
        assert!(error.contains("duplicate reviewed canonical player"));
    }

    fn player_context(player_id: u32, player: &str) -> ProspectLeaguePlayerContext {
        ProspectLeaguePlayerContext {
            player_id,
            player: player.to_owned(),
            organization: "SEA".to_owned(),
            position: "RW".to_owned(),
            age: 22,
            nhl_games_played: 0,
            opportunity: ProspectOpportunityStatus::RecallCandidate,
            availability: ProspectAvailabilityStatus::Healthy,
            attention_score: 0.2,
            attention_basis: "Test analyst attention estimate.".to_owned(),
            evidence: vec![],
        }
    }

    fn snapshot(
        season: u32,
        provider_player_id: u32,
        games_played: u32,
        goals: u32,
        assists: u32,
    ) -> AhlRosterStatsSnapshot {
        AhlRosterStatsSnapshot {
            schema: AHL_ROSTER_STATS_SCHEMA.to_owned(),
            season,
            provider: AHL_PROVIDER.to_owned(),
            provider_season_id: season.to_string(),
            provider_season_name: season.to_string(),
            fetched_at: "2026-07-25T00:00:00Z".to_owned(),
            source_url: format!("https://theahl.com/stats/{season}"),
            roster_source_url: "https://theahl.com/stats/roster".to_owned(),
            identity_note: "Provider IDs are local.".to_owned(),
            teams: vec![AhlTeamRosterStats {
                provider: AHL_PROVIDER.to_owned(),
                provider_team_id: "1".to_owned(),
                team_code: "CV".to_owned(),
                team_name: "Coachella Valley Firebirds".to_owned(),
                nickname: "Firebirds".to_owned(),
                division_id: "1".to_owned(),
                logo_url: "https://example.com/logo.png".to_owned(),
                nhl_affiliate: Some("SEA".to_owned()),
                roster: vec![],
                skaters: vec![AhlSkaterSeasonRow {
                    provider: AHL_PROVIDER.to_owned(),
                    provider_player_id: provider_player_id.to_string(),
                    name: "Joined Prospect".to_owned(),
                    team_code: "CV".to_owned(),
                    position: "RW".to_owned(),
                    active: true,
                    rookie: false,
                    games_played,
                    goals,
                    assists,
                    points: goals + assists,
                    plus_minus: 0,
                    penalty_minutes: 0,
                    power_play_goals: 0,
                    short_handed_goals: 0,
                    shots: 100,
                }],
                goalies: vec![],
                source_warnings: vec![],
            }],
        }
    }

    fn crosswalk(season: u32, player_id: u32, player: &str) -> AhlIdentityCrosswalkView {
        AhlIdentityCrosswalkView {
            schema: AHL_IDENTITY_CROSSWALK_SCHEMA.to_owned(),
            season,
            provider: AHL_PROVIDER.to_owned(),
            ahl_team: "Coachella Valley Firebirds".to_owned(),
            nhl_affiliate: Some("SEA".to_owned()),
            roster_fetched_at: "2026-07-25T00:00:00Z".to_owned(),
            candidates_checked_at: "2026-07-25T00:00:00Z".to_owned(),
            counts: AhlIdentityCrosswalkCounts {
                roster_players: 1,
                exact_name_and_birth_date: 1,
                surname_and_birth_date: 0,
                exact_name_only: 0,
                ambiguous: 0,
                conflicts: 0,
                unmatched: 0,
                reviewed: 1,
            },
            rows: vec![AhlIdentityCrosswalkRow {
                provider_player_id: player_id.to_string(),
                ahl_display_name: player.to_owned(),
                ahl_birth_date: "2004-01-01".to_owned(),
                match_basis: AhlIdentityMatchBasis::ExactNameAndBirthDate,
                review_status: AhlIdentityReviewStatus::Reviewed,
                nhl_player_id: Some(player_id),
                nhl_display_name: Some(player.to_owned()),
                nhl_birth_date: Some("2004-01-01".to_owned()),
                evidence_urls: vec!["https://example.com/identity".to_owned()],
                note: "Reviewed test identity.".to_owned(),
            }],
            disclosures: vec![],
        }
    }
}
