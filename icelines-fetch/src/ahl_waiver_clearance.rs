//! Reviewed target-season NHL waiver outcomes for AHL assignment gates.
//!
//! Waiver eligibility, placement, clearance, and assignment are distinct.
//! This module writes only explicit cleared/claimed outcomes supplied with
//! dated source URLs and reviewer authority.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::AhlFeedError,
    ahl_preseason_facts::{
        fingerprint_workboard, recompute_workboard, validate_workboard, AhlPreseasonFactBlocker,
        AhlPreseasonFactsCandidateStatus, AhlPreseasonLeagueFactsWorkboardView,
        AhlPreseasonWaiverAuthority,
    },
};

pub const AHL_WAIVER_CLEARANCE_REVIEW_SCHEMA: &str = "ahl_waiver_clearance_review.v1";
pub const AHL_WAIVER_CLEARANCE_DECISIONS_SCHEMA: &str = "ahl_waiver_clearance_decisions.v1";
pub const AHL_WAIVER_CLEARANCE_APPLICATION_SCHEMA: &str = "ahl_waiver_clearance_application.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlWaiverOutcome {
    Cleared,
    Claimed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlWaiverClearanceReviewRow {
    pub nhl_team: String,
    pub nhl_player_id: u32,
    pub display_name: String,
    #[serde(default)]
    pub outcome: Option<AhlWaiverOutcome>,
    #[serde(default)]
    pub waiver_date: Option<String>,
    #[serde(default)]
    pub claimed_by: Option<String>,
    #[serde(default)]
    pub source_urls: Vec<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlWaiverClearanceCounts {
    pub decisions_required: usize,
    pub resolved: usize,
    pub cleared: usize,
    pub claimed: usize,
    pub pending: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlWaiverClearanceReviewView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub cutoff: String,
    pub workboard_fingerprint: String,
    pub draft: bool,
    #[serde(default)]
    pub reviewer: Option<String>,
    #[serde(default)]
    pub reviewed_at: Option<String>,
    pub counts: AhlWaiverClearanceCounts,
    pub source_fingerprint: String,
    pub rows: Vec<AhlWaiverClearanceReviewRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlWaiverClearanceDecisionRow {
    pub nhl_team: String,
    pub nhl_player_id: u32,
    pub outcome: AhlWaiverOutcome,
    pub waiver_date: String,
    #[serde(default)]
    pub claimed_by: Option<String>,
    pub source_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlWaiverClearanceDecisionsView {
    pub schema: String,
    pub workboard_fingerprint: String,
    pub reviewer: String,
    pub reviewed_at: String,
    pub decisions: Vec<AhlWaiverClearanceDecisionRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlWaiverClearanceApplicationView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub source_workboard_fingerprint: String,
    pub waiver_review_fingerprint: String,
    pub cleared_applied: usize,
    pub claimed_applied: usize,
    pub pending_review_rows: usize,
    pub candidates_missing_waiver_clearance: usize,
    pub workboard: AhlPreseasonLeagueFactsWorkboardView,
    pub disclosures: Vec<String>,
}

pub fn build_ahl_waiver_clearance_review_draft(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    cutoff: impl Into<String>,
) -> Result<AhlWaiverClearanceReviewView, AhlFeedError> {
    validate_workboard(workboard)?;
    let cutoff = cutoff.into();
    parse_date(&cutoff, "waiver review cutoff")?;
    let mut rows = Vec::new();
    for team in &workboard.team_workboards {
        for player in &team.players {
            if player.status != AhlPreseasonFactsCandidateStatus::Candidate
                || !player
                    .blockers
                    .contains(&AhlPreseasonFactBlocker::WaiverClearance)
            {
                continue;
            }
            rows.push(AhlWaiverClearanceReviewRow {
                nhl_team: team.nhl_team.clone(),
                nhl_player_id: player.nhl_player_id.ok_or_else(|| {
                    AhlFeedError::Validation(
                        "waiver-gated candidate has no canonical NHL identity".into(),
                    )
                })?,
                display_name: player.display_name.clone(),
                outcome: None,
                waiver_date: None,
                claimed_by: None,
                source_urls: Vec::new(),
                note: String::new(),
            });
        }
    }
    rows.sort_by(|left, right| {
        left.nhl_team
            .cmp(&right.nhl_team)
            .then_with(|| left.nhl_player_id.cmp(&right.nhl_player_id))
    });
    let required = rows.len();
    let mut review = AhlWaiverClearanceReviewView {
        schema: AHL_WAIVER_CLEARANCE_REVIEW_SCHEMA.into(),
        prior_season: workboard.prior_season,
        target_season: workboard.target_season,
        cutoff,
        workboard_fingerprint: workboard.source_fingerprint.clone(),
        draft: true,
        reviewer: None,
        reviewed_at: None,
        counts: AhlWaiverClearanceCounts {
            decisions_required: required,
            resolved: 0,
            cleared: 0,
            claimed: 0,
            pending: required,
        },
        source_fingerprint: String::new(),
        rows,
        disclosures: vec![
            "This is a non-applicable review queue. Waiver eligibility or placement is not clearance; only an explicit dated cleared or claimed result may resolve a row.".into(),
            "Target-season absence is no-read. A prior-season clearance does not automatically survive 10 NHL games or 30 cumulative NHL-roster days.".into(),
            "PuckPedia's waiver wire, an NHL/team report, or another directly reviewable result page may be cited; every applied row requires an absolute source URL.".into(),
        ],
    };
    review.source_fingerprint = fingerprint_review(&review)?;
    Ok(review)
}

pub fn finalize_ahl_waiver_clearance_review(
    draft: &AhlWaiverClearanceReviewView,
    decisions: &AhlWaiverClearanceDecisionsView,
) -> Result<AhlWaiverClearanceReviewView, AhlFeedError> {
    validate_review_envelope(draft)?;
    if !draft.draft
        || decisions.schema != AHL_WAIVER_CLEARANCE_DECISIONS_SCHEMA
        || decisions.workboard_fingerprint != draft.workboard_fingerprint
        || decisions.reviewer.trim().is_empty()
        || DateTime::parse_from_rfc3339(&decisions.reviewed_at).is_err()
        || decisions.decisions.is_empty()
    {
        return Err(AhlFeedError::Validation(
            "waiver finalization requires an intact draft and explicit reviewer decisions".into(),
        ));
    }
    let cutoff = parse_date(&draft.cutoff, "waiver review cutoff")?;
    let mut output = draft.clone();
    let mut rows = output
        .rows
        .iter_mut()
        .map(|row| ((row.nhl_team.clone(), row.nhl_player_id), row))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    for decision in &decisions.decisions {
        let key = (decision.nhl_team.clone(), decision.nhl_player_id);
        let row = rows.get_mut(&key).ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "waiver decision references unknown candidate {}/{}",
                decision.nhl_team, decision.nhl_player_id
            ))
        })?;
        let waiver_date = parse_date(&decision.waiver_date, "waiver outcome date")?;
        if !seen.insert(key)
            || waiver_date > cutoff
            || decision.source_urls.is_empty()
            || decision.source_urls.iter().any(|url| !absolute_url(url))
            || decision.note.trim().is_empty()
            || match decision.outcome {
                AhlWaiverOutcome::Cleared => decision.claimed_by.is_some(),
                AhlWaiverOutcome::Claimed => decision
                    .claimed_by
                    .as_deref()
                    .is_none_or(|team| team.trim().is_empty() || team == decision.nhl_team),
            }
        {
            return Err(AhlFeedError::Validation(
                "waiver decision is duplicate, future-dated, contradictory, or unsourced".into(),
            ));
        }
        row.outcome = Some(decision.outcome);
        row.waiver_date = Some(decision.waiver_date.clone());
        row.claimed_by.clone_from(&decision.claimed_by);
        row.source_urls.clone_from(&decision.source_urls);
        row.note.clone_from(&decision.note);
    }
    output.draft = false;
    output.reviewer = Some(decisions.reviewer.clone());
    output.reviewed_at = Some(decisions.reviewed_at.clone());
    output.counts = counts(&output.rows);
    output.source_fingerprint.clear();
    output.source_fingerprint = fingerprint_review(&output)?;
    validate_review_envelope(&output)?;
    Ok(output)
}

pub fn apply_ahl_waiver_clearance_review(
    workboard: &AhlPreseasonLeagueFactsWorkboardView,
    review: &AhlWaiverClearanceReviewView,
) -> Result<AhlWaiverClearanceApplicationView, AhlFeedError> {
    validate_workboard(workboard)?;
    validate_review_envelope(review)?;
    if review.draft
        || review.workboard_fingerprint != workboard.source_fingerprint
        || review.prior_season != workboard.prior_season
        || review.target_season != workboard.target_season
    {
        return Err(AhlFeedError::Validation(
            "waiver application requires a finalized review bound to the exact workboard".into(),
        ));
    }
    let reviewer = review.reviewer.as_ref().expect("validated reviewer");
    let reviewed_at = review.reviewed_at.as_ref().expect("validated timestamp");
    let source_workboard_fingerprint = workboard.source_fingerprint.clone();
    let mut applied = workboard.clone();
    let mut cleared_applied = 0usize;
    let mut claimed_applied = 0usize;
    let decided = review
        .rows
        .iter()
        .filter(|row| row.outcome.is_some())
        .map(|row| ((row.nhl_team.as_str(), row.nhl_player_id), row))
        .collect::<BTreeMap<_, _>>();
    for team in &mut applied.team_workboards {
        for player in &mut team.players {
            let Some(player_id) = player.nhl_player_id else {
                continue;
            };
            let Some(row) = decided.get(&(team.nhl_team.as_str(), player_id)).copied() else {
                continue;
            };
            let cleared = row.outcome == Some(AhlWaiverOutcome::Cleared);
            if player
                .waiver_cleared
                .is_some_and(|existing| existing != cleared)
            {
                return Err(AhlFeedError::Validation(format!(
                    "waiver result conflicts with existing fact for NHL player {player_id} in {}",
                    team.nhl_team
                )));
            }
            if player.waiver_cleared.is_some() {
                continue;
            }
            player.waiver_cleared = Some(cleared);
            player.waiver_authority = Some(AhlPreseasonWaiverAuthority {
                result: if cleared { "cleared" } else { "claimed" }.into(),
                waiver_date: row.waiver_date.clone().expect("validated date"),
                cutoff: review.cutoff.clone(),
                source_fingerprint: review.source_fingerprint.clone(),
                source_urls: row.source_urls.clone(),
                reviewer: reviewer.clone(),
                reviewed_at: reviewed_at.clone(),
            });
            if cleared {
                player
                    .blockers
                    .retain(|blocker| *blocker != AhlPreseasonFactBlocker::WaiverClearance);
                cleared_applied += 1;
            } else {
                claimed_applied += 1;
            }
        }
    }
    recompute_workboard(&mut applied)?;
    applied.disclosures.push(format!(
        "Waiver review {} applied {} cleared and {} claimed target-season results through {}; pending rows remain blocked.",
        review.source_fingerprint, cleared_applied, claimed_applied, review.cutoff
    ));
    applied.source_fingerprint = fingerprint_workboard(&applied)?;
    let candidates_missing_waiver_clearance = applied
        .blocker_counts
        .get(&AhlPreseasonFactBlocker::WaiverClearance)
        .copied()
        .unwrap_or_default();
    Ok(AhlWaiverClearanceApplicationView {
        schema: AHL_WAIVER_CLEARANCE_APPLICATION_SCHEMA.into(),
        prior_season: applied.prior_season,
        target_season: applied.target_season,
        source_workboard_fingerprint,
        waiver_review_fingerprint: review.source_fingerprint.clone(),
        cleared_applied,
        claimed_applied,
        pending_review_rows: review.counts.pending,
        candidates_missing_waiver_clearance,
        workboard: applied,
        disclosures: vec![
            "Only waiver_cleared is written. A claim does not choose an organization or affiliate; organization status and assignment remain separately sourced.".into(),
        ],
    })
}

fn validate_review_envelope(review: &AhlWaiverClearanceReviewView) -> Result<(), AhlFeedError> {
    parse_date(&review.cutoff, "waiver review cutoff")?;
    let mut keys = BTreeSet::new();
    if review.schema != AHL_WAIVER_CLEARANCE_REVIEW_SCHEMA
        || review.workboard_fingerprint.trim().is_empty()
        || review.counts != counts(&review.rows)
        || review.source_fingerprint != fingerprint_review(review)?
        || review.rows.iter().any(|row| {
            row.nhl_team.trim().is_empty()
                || row.nhl_player_id == 0
                || row.display_name.trim().is_empty()
                || !keys.insert((row.nhl_team.as_str(), row.nhl_player_id))
                || match row.outcome {
                    None => {
                        row.waiver_date.is_some()
                            || row.claimed_by.is_some()
                            || !row.source_urls.is_empty()
                            || !row.note.is_empty()
                    }
                    Some(outcome) => {
                        row.waiver_date
                            .as_deref()
                            .is_none_or(|date| parse_date(date, "waiver result date").is_err())
                            || row.source_urls.is_empty()
                            || row.source_urls.iter().any(|url| !absolute_url(url))
                            || row.note.trim().is_empty()
                            || match outcome {
                                AhlWaiverOutcome::Cleared => row.claimed_by.is_some(),
                                AhlWaiverOutcome::Claimed => {
                                    row.claimed_by.as_deref().is_none_or(|team| {
                                        team.trim().is_empty() || team == row.nhl_team
                                    })
                                }
                            }
                    }
                }
        })
        || if review.draft {
            review.reviewer.is_some() || review.reviewed_at.is_some() || review.counts.resolved != 0
        } else {
            review
                .reviewer
                .as_deref()
                .is_none_or(|reviewer| reviewer.trim().is_empty())
                || review
                    .reviewed_at
                    .as_deref()
                    .is_none_or(|timestamp| DateTime::parse_from_rfc3339(timestamp).is_err())
        }
    {
        return Err(AhlFeedError::Validation(
            "waiver review is inconsistent, unsourced, or tampered".into(),
        ));
    }
    Ok(())
}

fn counts(rows: &[AhlWaiverClearanceReviewRow]) -> AhlWaiverClearanceCounts {
    let cleared = rows
        .iter()
        .filter(|row| row.outcome == Some(AhlWaiverOutcome::Cleared))
        .count();
    let claimed = rows
        .iter()
        .filter(|row| row.outcome == Some(AhlWaiverOutcome::Claimed))
        .count();
    AhlWaiverClearanceCounts {
        decisions_required: rows.len(),
        resolved: cleared + claimed,
        cleared,
        claimed,
        pending: rows.len() - cleared - claimed,
    }
}

fn fingerprint_review(review: &AhlWaiverClearanceReviewView) -> Result<String, AhlFeedError> {
    let mut canonical = review.clone();
    canonical.source_fingerprint.clear();
    canonical.rows.sort_by(|left, right| {
        left.nhl_team
            .cmp(&right.nhl_team)
            .then_with(|| left.nhl_player_id.cmp(&right.nhl_player_id))
    });
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn parse_date(value: &str, label: &str) -> Result<NaiveDate, AhlFeedError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| AhlFeedError::Validation(format!("{label} must be YYYY-MM-DD")))
}

fn absolute_url(value: &str) -> bool {
    reqwest::Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::Position;

    use crate::{
        ahl_preseason_facts::{
            AhlPreseasonFactsPlayerRow, AhlPreseasonFactsTeamCounts, AhlPreseasonFactsTeamView,
            AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA,
        },
        ahl_rollover::AhlPreseasonPositionGroup,
    };

    fn workboard() -> AhlPreseasonLeagueFactsWorkboardView {
        let mut board = AhlPreseasonLeagueFactsWorkboardView {
            schema: AHL_PRESEASON_LEAGUE_FACTS_WORKBOARD_SCHEMA.into(),
            prior_season: 20252026,
            target_season: 20262027,
            professional_game_policy_id: "test".into(),
            professional_game_policy_authority: "final".into(),
            professional_game_threshold: 260,
            source_fingerprint: String::new(),
            teams: 1,
            candidates: 0,
            facts_ready_candidates: 0,
            blocker_counts: BTreeMap::new(),
            team_workboards: vec![AhlPreseasonFactsTeamView {
                nhl_team: "NYR".into(),
                ahl_team: "Hartford Wolf Pack".into(),
                source_urls: Vec::new(),
                counts: AhlPreseasonFactsTeamCounts {
                    players: 0,
                    candidates: 0,
                    facts_ready_candidates: 0,
                    not_assigned: 0,
                    projected_nhl_roster: 0,
                    explicit_departures: 0,
                    identity_blocked: 0,
                    missing_assignment_authority: 0,
                    missing_organization_status: 0,
                    missing_waiver_clearance: 0,
                    missing_exact_position: 0,
                    missing_projected_score: 0,
                    missing_prospect_status: 0,
                    missing_recall_readiness: 0,
                    missing_professional_games: 0,
                    missing_development_rule_qualification: 0,
                },
                players: vec![AhlPreseasonFactsPlayerRow {
                    nhl_player_id: Some(8480001),
                    display_name: "Waiver Player".into(),
                    status: AhlPreseasonFactsCandidateStatus::Candidate,
                    origins: Vec::new(),
                    position_group: AhlPreseasonPositionGroup::Forward,
                    primary_position: Some(Position::Center),
                    eligible_positions: vec![Position::Center],
                    projected_score: Some(50.0),
                    projected_score_method: None,
                    projected_score_confidence: None,
                    projected_score_sample_games: None,
                    projected_score_source_fingerprint: None,
                    prospect: Some(false),
                    prospect_method: None,
                    prospect_source_fingerprint: None,
                    recall_readiness: Some(0.5),
                    recall_readiness_method: None,
                    recall_readiness_confidence: None,
                    recall_readiness_coverage: None,
                    recall_readiness_source_fingerprint: None,
                    assigned_to_affiliate: Some(true),
                    assignment_authority: None,
                    waiver_cleared: None,
                    waiver_authority: None,
                    review_source_urls: Vec::new(),
                    review_note: None,
                    reviewer: None,
                    reviewed_at: None,
                    professional_games_at_season_start: Some(300),
                    development_rule_qualified: Some(false),
                    blockers: vec![AhlPreseasonFactBlocker::WaiverClearance],
                }],
            }],
            disclosures: vec!["fixture".into()],
        };
        recompute_workboard(&mut board).unwrap();
        board.source_fingerprint = fingerprint_workboard(&board).unwrap();
        board
    }

    fn decisions(
        board: &AhlPreseasonLeagueFactsWorkboardView,
        outcome: AhlWaiverOutcome,
    ) -> AhlWaiverClearanceDecisionsView {
        AhlWaiverClearanceDecisionsView {
            schema: AHL_WAIVER_CLEARANCE_DECISIONS_SCHEMA.into(),
            workboard_fingerprint: board.source_fingerprint.clone(),
            reviewer: "waiver-reviewer".into(),
            reviewed_at: "2026-09-30T17:00:00Z".into(),
            decisions: vec![AhlWaiverClearanceDecisionRow {
                nhl_team: "NYR".into(),
                nhl_player_id: 8480001,
                outcome,
                waiver_date: "2026-09-29".into(),
                claimed_by: (outcome == AhlWaiverOutcome::Claimed).then(|| "SEA".into()),
                source_urls: vec!["https://puckpedia.com/waiver-wire".into()],
                note: "Reviewed explicit target-season waiver result.".into(),
            }],
        }
    }

    #[test]
    fn draft_lists_only_waiver_gated_candidates_and_cannot_apply() {
        let board = workboard();
        let draft = build_ahl_waiver_clearance_review_draft(&board, "2026-09-30").unwrap();
        assert_eq!(draft.counts.decisions_required, 1);
        assert_eq!(draft.counts.pending, 1);
        assert!(apply_ahl_waiver_clearance_review(&board, &draft).is_err());
    }

    #[test]
    fn reviewed_clearance_clears_only_waiver_blocker_with_provenance() {
        let board = workboard();
        let draft = build_ahl_waiver_clearance_review_draft(&board, "2026-09-30").unwrap();
        let review = finalize_ahl_waiver_clearance_review(
            &draft,
            &decisions(&board, AhlWaiverOutcome::Cleared),
        )
        .unwrap();
        let application = apply_ahl_waiver_clearance_review(&board, &review).unwrap();
        let player = &application.workboard.team_workboards[0].players[0];
        assert_eq!(application.cleared_applied, 1);
        assert_eq!(player.waiver_cleared, Some(true));
        assert!(player.waiver_authority.is_some());
        assert!(!player
            .blockers
            .contains(&AhlPreseasonFactBlocker::WaiverClearance));
    }

    #[test]
    fn claim_is_explicit_but_remains_assignment_blocking() {
        let board = workboard();
        let draft = build_ahl_waiver_clearance_review_draft(&board, "2026-09-30").unwrap();
        let review = finalize_ahl_waiver_clearance_review(
            &draft,
            &decisions(&board, AhlWaiverOutcome::Claimed),
        )
        .unwrap();
        let application = apply_ahl_waiver_clearance_review(&board, &review).unwrap();
        let player = &application.workboard.team_workboards[0].players[0];
        assert_eq!(application.claimed_applied, 1);
        assert_eq!(player.waiver_cleared, Some(false));
        assert!(player
            .blockers
            .contains(&AhlPreseasonFactBlocker::WaiverClearance));
        assert_eq!(player.assigned_to_affiliate, Some(true));
    }

    #[test]
    fn future_unsourced_and_tampered_reviews_fail_closed() {
        let board = workboard();
        let draft = build_ahl_waiver_clearance_review_draft(&board, "2026-09-30").unwrap();
        let mut invalid = decisions(&board, AhlWaiverOutcome::Cleared);
        invalid.decisions[0].waiver_date = "2026-10-01".into();
        assert!(finalize_ahl_waiver_clearance_review(&draft, &invalid).is_err());
        let mut review = finalize_ahl_waiver_clearance_review(
            &draft,
            &decisions(&board, AhlWaiverOutcome::Cleared),
        )
        .unwrap();
        review.rows[0].note = "tampered".into();
        assert!(apply_ahl_waiver_clearance_review(&board, &review).is_err());
    }
}
