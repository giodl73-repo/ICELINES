//! Official NHL current-team evidence for AHL preseason organization review.
//!
//! Current-team presence can prove retained/departed relative to one NHL
//! organization. Missing team data proves nothing, and this module never
//! infers an other-league assignment or finalizes a human review.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::AhlFeedError,
    ahl_rollover::{
        AhlPreseasonDecisionKind, AhlPreseasonLeagueOrganizationReview,
        AHL_PRESEASON_LEAGUE_ORGANIZATION_REVIEW_SCHEMA,
    },
    career_landing::CareerHistoryStore,
};

pub const AHL_ORGANIZATION_STATUS_LEDGER_SCHEMA: &str = "ahl_organization_status_ledger.v1";
pub const AHL_ORGANIZATION_STATUS_APPLICATION_SCHEMA: &str =
    "ahl_organization_status_application.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlOrganizationStatusUnavailableReason {
    MissingOfficialLandingFact,
    MissingCurrentNhlTeam,
    StaleOfficialLandingFact,
    OfficialLandingInactive,
    CurrentTeamOutsideTargetCohort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlOrganizationStatusLedgerRow {
    pub nhl_team: String,
    pub nhl_player_id: u32,
    pub display_name: String,
    #[serde(default)]
    pub decision: Option<AhlPreseasonDecisionKind>,
    #[serde(default)]
    pub unavailable_reason: Option<AhlOrganizationStatusUnavailableReason>,
    #[serde(default)]
    pub observed_current_team: Option<String>,
    #[serde(default)]
    pub observed_at: Option<String>,
    #[serde(default)]
    pub evidence_url: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlOrganizationStatusLedgerCounts {
    pub decisions_required: usize,
    pub resolved: usize,
    pub retained: usize,
    pub departed: usize,
    pub unresolved: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlOrganizationStatusLedgerView {
    pub schema: String,
    pub prior_season: u32,
    pub target_season: u32,
    pub as_of: String,
    pub maximum_fact_age_days: u32,
    pub review_fingerprint: String,
    pub counts: AhlOrganizationStatusLedgerCounts,
    pub rows: Vec<AhlOrganizationStatusLedgerRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlOrganizationStatusApplicationView {
    pub schema: String,
    pub ledger_fingerprint: String,
    pub decisions_applied: usize,
    pub decisions_remaining: usize,
    pub review: AhlPreseasonLeagueOrganizationReview,
    pub disclosures: Vec<String>,
}

pub fn build_ahl_organization_status_ledger(
    review: &AhlPreseasonLeagueOrganizationReview,
    career_store: &CareerHistoryStore,
    as_of: impl Into<String>,
    maximum_fact_age_days: u32,
) -> Result<AhlOrganizationStatusLedgerView, AhlFeedError> {
    validate_review_draft(review)?;
    let as_of = as_of.into();
    let as_of_time = DateTime::parse_from_rfc3339(&as_of)
        .map_err(|_| AhlFeedError::Validation("organization status as_of must be RFC 3339".into()))?
        .with_timezone(&Utc);
    if maximum_fact_age_days == 0 {
        return Err(AhlFeedError::Validation(
            "organization status maximum fact age must be positive".into(),
        ));
    }
    let cohort = review
        .reviews
        .iter()
        .map(|child| child.nhl_team.as_str())
        .collect::<BTreeSet<_>>();
    if cohort.len() != review.reviews.len() {
        return Err(AhlFeedError::Validation(
            "organization review contains duplicate NHL teams".into(),
        ));
    }

    let mut keys = BTreeSet::new();
    let mut rows = Vec::new();
    for child in &review.reviews {
        for row in &child.rows {
            if !(row.identity_reviewed && row.in_current_camp == Some(false)) {
                continue;
            }
            let player_id = row.nhl_player_id.ok_or_else(|| {
                AhlFeedError::Validation("reviewed organization row has no NHL player ID".into())
            })?;
            if !keys.insert((child.nhl_team.as_str(), player_id)) {
                return Err(AhlFeedError::Validation(format!(
                    "organization review duplicates NHL player {player_id} for {}",
                    child.nhl_team
                )));
            }
            rows.push(status_row(
                &child.nhl_team,
                player_id,
                &row.display_name,
                career_store,
                &cohort,
                as_of_time,
                maximum_fact_age_days,
            )?);
        }
    }
    rows.sort_by(|left, right| {
        left.nhl_team
            .cmp(&right.nhl_team)
            .then_with(|| left.nhl_player_id.cmp(&right.nhl_player_id))
    });
    let retained = rows
        .iter()
        .filter(|row| row.decision == Some(AhlPreseasonDecisionKind::Retained))
        .count();
    let departed = rows
        .iter()
        .filter(|row| row.decision == Some(AhlPreseasonDecisionKind::Departed))
        .count();
    let resolved = retained + departed;
    Ok(AhlOrganizationStatusLedgerView {
        schema: AHL_ORGANIZATION_STATUS_LEDGER_SCHEMA.into(),
        prior_season: review.prior_season,
        target_season: review.target_season,
        as_of,
        maximum_fact_age_days,
        review_fingerprint: fingerprint(review)?,
        counts: AhlOrganizationStatusLedgerCounts {
            decisions_required: rows.len(),
            resolved,
            retained,
            departed,
            unresolved: rows.len() - resolved,
        },
        rows,
        disclosures: vec![
            "Official NHL current-team equality establishes retained status for that organization; a different team in the target NHL cohort establishes departed status.".into(),
            "Missing, stale, or non-cohort current-team facts remain unresolved. Absence never implies departure or another league.".into(),
            "This ledger establishes organization status only. It does not establish NHL/AHL assignment, contract rights, waivers, lineup role, or recall probability.".into(),
        ],
    })
}

fn status_row(
    nhl_team: &str,
    player_id: u32,
    display_name: &str,
    career_store: &CareerHistoryStore,
    cohort: &BTreeSet<&str>,
    as_of: DateTime<Utc>,
    maximum_fact_age_days: u32,
) -> Result<AhlOrganizationStatusLedgerRow, AhlFeedError> {
    let Some(fact) = career_store.organization_fact(player_id) else {
        return Ok(unavailable_row(
            nhl_team,
            player_id,
            display_name,
            AhlOrganizationStatusUnavailableReason::MissingOfficialLandingFact,
            "No dated official NHL landing organization fact is cached.",
        ));
    };
    if fact.player_id != player_id || !fact.source_url.starts_with("https://api-web.nhle.com/") {
        return Err(AhlFeedError::Validation(format!(
            "official organization fact for NHL player {player_id} has invalid identity or source"
        )));
    }
    let observed_at = DateTime::parse_from_rfc3339(&fact.observed_at)
        .map_err(|_| {
            AhlFeedError::Validation(format!(
                "organization fact for NHL player {player_id} has invalid observed_at"
            ))
        })?
        .with_timezone(&Utc);
    if observed_at > as_of {
        return Err(AhlFeedError::Validation(format!(
            "organization fact for NHL player {player_id} is later than ledger as_of"
        )));
    }
    let age = as_of.signed_duration_since(observed_at);
    if age > chrono::Duration::days(i64::from(maximum_fact_age_days)) {
        return Ok(AhlOrganizationStatusLedgerRow {
            nhl_team: nhl_team.into(),
            nhl_player_id: player_id,
            display_name: display_name.into(),
            decision: None,
            unavailable_reason: Some(
                AhlOrganizationStatusUnavailableReason::StaleOfficialLandingFact,
            ),
            observed_current_team: fact.current_team_abbrev.clone(),
            observed_at: Some(fact.observed_at.clone()),
            evidence_url: Some(fact.source_url.clone()),
            note: format!("Official NHL landing fact is older than {maximum_fact_age_days} days."),
        });
    }
    let Some(current_team) = fact.current_team_abbrev.as_deref() else {
        return Ok(AhlOrganizationStatusLedgerRow {
            nhl_team: nhl_team.into(),
            nhl_player_id: player_id,
            display_name: display_name.into(),
            decision: None,
            unavailable_reason: Some(AhlOrganizationStatusUnavailableReason::MissingCurrentNhlTeam),
            observed_current_team: None,
            observed_at: Some(fact.observed_at.clone()),
            evidence_url: Some(fact.source_url.clone()),
            note: "Official NHL landing has no current team; no status is inferred.".into(),
        });
    };
    if fact.is_active == Some(false) {
        return Ok(AhlOrganizationStatusLedgerRow {
            nhl_team: nhl_team.into(),
            nhl_player_id: player_id,
            display_name: display_name.into(),
            decision: None,
            unavailable_reason: Some(
                AhlOrganizationStatusUnavailableReason::OfficialLandingInactive,
            ),
            observed_current_team: fact.current_team_abbrev.clone(),
            observed_at: Some(fact.observed_at.clone()),
            evidence_url: Some(fact.source_url.clone()),
            note: "Official NHL landing marks the player inactive; no current organization status is inferred.".into(),
        });
    }
    if !cohort.contains(current_team) {
        return Ok(AhlOrganizationStatusLedgerRow {
            nhl_team: nhl_team.into(),
            nhl_player_id: player_id,
            display_name: display_name.into(),
            decision: None,
            unavailable_reason: Some(
                AhlOrganizationStatusUnavailableReason::CurrentTeamOutsideTargetCohort,
            ),
            observed_current_team: Some(current_team.into()),
            observed_at: Some(fact.observed_at.clone()),
            evidence_url: Some(fact.source_url.clone()),
            note: format!(
                "Official current team {current_team} is outside the sealed target NHL cohort."
            ),
        });
    }
    let decision = if current_team == nhl_team {
        AhlPreseasonDecisionKind::Retained
    } else {
        AhlPreseasonDecisionKind::Departed
    };
    Ok(AhlOrganizationStatusLedgerRow {
        nhl_team: nhl_team.into(),
        nhl_player_id: player_id,
        display_name: display_name.into(),
        decision: Some(decision),
        unavailable_reason: None,
        observed_current_team: Some(current_team.into()),
        observed_at: Some(fact.observed_at.clone()),
        evidence_url: Some(fact.source_url.clone()),
        note: format!(
            "Official NHL landing current team {current_team} establishes {} relative to {nhl_team}.",
            match decision {
                AhlPreseasonDecisionKind::Retained => "retained status",
                AhlPreseasonDecisionKind::Departed => "departure",
                AhlPreseasonDecisionKind::OtherLeague => unreachable!(),
            }
        ),
    })
}

fn unavailable_row(
    nhl_team: &str,
    player_id: u32,
    display_name: &str,
    reason: AhlOrganizationStatusUnavailableReason,
    note: &str,
) -> AhlOrganizationStatusLedgerRow {
    AhlOrganizationStatusLedgerRow {
        nhl_team: nhl_team.into(),
        nhl_player_id: player_id,
        display_name: display_name.into(),
        decision: None,
        unavailable_reason: Some(reason),
        observed_current_team: None,
        observed_at: None,
        evidence_url: None,
        note: note.into(),
    }
}

pub fn apply_ahl_organization_status_ledger(
    review: &AhlPreseasonLeagueOrganizationReview,
    ledger: &AhlOrganizationStatusLedgerView,
) -> Result<AhlOrganizationStatusApplicationView, AhlFeedError> {
    validate_review_draft(review)?;
    validate_ledger(review, ledger)?;
    let rows = ledger
        .rows
        .iter()
        .map(|row| ((row.nhl_team.as_str(), row.nhl_player_id), row))
        .collect::<BTreeMap<_, _>>();
    let mut updated = review.clone();
    let mut applied = 0usize;
    for child in &mut updated.reviews {
        for row in &mut child.rows {
            if !(row.identity_reviewed && row.in_current_camp == Some(false)) {
                continue;
            }
            let ledger_row = rows[&(child.nhl_team.as_str(), row.nhl_player_id.unwrap())];
            if let (Some(existing), Some(sourced)) = (row.decision_kind, ledger_row.decision) {
                if existing != sourced {
                    return Err(AhlFeedError::Validation(format!(
                        "organization review decision for {} NHL player {} conflicts with official current-team evidence",
                        child.nhl_team,
                        row.nhl_player_id.unwrap()
                    )));
                }
            } else if row.decision_kind.is_none() {
                let Some(decision) = ledger_row.decision else {
                    continue;
                };
                row.decision_kind = Some(decision);
                row.evidence_urls = vec![ledger_row.evidence_url.clone().unwrap()];
                row.note = ledger_row.note.clone();
                applied += 1;
            }
        }
    }
    let remaining = updated
        .reviews
        .iter()
        .flat_map(|child| &child.rows)
        .filter(|row| {
            row.identity_reviewed
                && row.in_current_camp == Some(false)
                && row.decision_kind.is_none()
        })
        .count();
    Ok(AhlOrganizationStatusApplicationView {
        schema: AHL_ORGANIZATION_STATUS_APPLICATION_SCHEMA.into(),
        ledger_fingerprint: fingerprint(ledger)?,
        decisions_applied: applied,
        decisions_remaining: remaining,
        review: updated,
        disclosures: vec![
            "The nested organization review remains a draft and requires a named reviewer, review timestamp, and sourced resolution of every remaining decision before it can be finalized.".into(),
            "Existing non-required rows and unresolved decisions are preserved without inference.".into(),
        ],
    })
}

fn validate_review_draft(
    review: &AhlPreseasonLeagueOrganizationReview,
) -> Result<(), AhlFeedError> {
    if review.schema != AHL_PRESEASON_LEAGUE_ORGANIZATION_REVIEW_SCHEMA
        || !review.draft
        || review.teams_requested != review.teams_built
        || review.reviews.len() != review.teams_built
        || !review.failures.is_empty()
        || review.decisions_required
            != review
                .reviews
                .iter()
                .map(|child| child.decisions_required)
                .sum::<usize>()
        || review.reviews.iter().any(|child| {
            child.schema != crate::ahl_rollover::AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA
                || !child.draft
                || child.prior_season != review.prior_season
                || child.target_season != review.target_season
                || child.decisions_required
                    != child
                        .rows
                        .iter()
                        .filter(|row| row.identity_reviewed && row.in_current_camp == Some(false))
                        .count()
                || child.rows.iter().any(|row| {
                    row.identity_reviewed
                        && row.in_current_camp == Some(false)
                        && row.nhl_player_id.is_none()
                })
        })
    {
        return Err(AhlFeedError::Validation(
            "organization status requires an internally consistent league review draft".into(),
        ));
    }
    Ok(())
}

fn validate_ledger(
    review: &AhlPreseasonLeagueOrganizationReview,
    ledger: &AhlOrganizationStatusLedgerView,
) -> Result<(), AhlFeedError> {
    let as_of = DateTime::parse_from_rfc3339(&ledger.as_of)
        .map_err(|_| AhlFeedError::Validation("organization ledger as_of is invalid".into()))?
        .with_timezone(&Utc);
    let cohort = review
        .reviews
        .iter()
        .map(|child| child.nhl_team.as_str())
        .collect::<BTreeSet<_>>();
    let expected_keys = review
        .reviews
        .iter()
        .flat_map(|child| {
            child
                .rows
                .iter()
                .filter(|row| row.identity_reviewed && row.in_current_camp == Some(false))
                .map(|row| {
                    (
                        child.nhl_team.as_str(),
                        row.nhl_player_id
                            .expect("validated reviewed organization row"),
                    )
                })
        })
        .collect::<BTreeSet<_>>();
    let actual_keys = ledger
        .rows
        .iter()
        .map(|row| (row.nhl_team.as_str(), row.nhl_player_id))
        .collect::<BTreeSet<_>>();
    let resolved = ledger
        .rows
        .iter()
        .filter(|row| row.decision.is_some())
        .count();
    let retained = ledger
        .rows
        .iter()
        .filter(|row| row.decision == Some(AhlPreseasonDecisionKind::Retained))
        .count();
    let departed = ledger
        .rows
        .iter()
        .filter(|row| row.decision == Some(AhlPreseasonDecisionKind::Departed))
        .count();
    if ledger.schema != AHL_ORGANIZATION_STATUS_LEDGER_SCHEMA
        || ledger.prior_season != review.prior_season
        || ledger.target_season != review.target_season
        || ledger.maximum_fact_age_days == 0
        || ledger.review_fingerprint != fingerprint(review)?
        || ledger.rows.len() != ledger.counts.decisions_required
        || actual_keys.len() != ledger.rows.len()
        || actual_keys != expected_keys
        || resolved != ledger.counts.resolved
        || retained != ledger.counts.retained
        || departed != ledger.counts.departed
        || ledger.counts.unresolved + resolved != ledger.rows.len()
        || ledger.rows.iter().any(|row| {
            row.decision.is_some() == row.unavailable_reason.is_some()
                || row
                    .decision
                    .is_some_and(|decision| decision == AhlPreseasonDecisionKind::OtherLeague)
                || row.decision.is_some()
                    && (row.evidence_url.is_none() || row.observed_at.is_none())
                || row.decision.is_some_and(|decision| {
                    let Some(observed_team) = row.observed_current_team.as_deref() else {
                        return true;
                    };
                    let expected = if observed_team == row.nhl_team {
                        AhlPreseasonDecisionKind::Retained
                    } else {
                        AhlPreseasonDecisionKind::Departed
                    };
                    let Some(observed_at) = row.observed_at.as_deref().and_then(|value| {
                        DateTime::parse_from_rfc3339(value)
                            .ok()
                            .map(|value| value.with_timezone(&Utc))
                    }) else {
                        return true;
                    };
                    decision != expected
                        || !cohort.contains(observed_team)
                        || row.note.trim().is_empty()
                        || row
                            .evidence_url
                            .as_deref()
                            .is_none_or(|url| !url.starts_with("https://api-web.nhle.com/"))
                        || observed_at > as_of
                        || as_of.signed_duration_since(observed_at)
                            > chrono::Duration::days(i64::from(ledger.maximum_fact_age_days))
                })
        })
    {
        return Err(AhlFeedError::Validation(
            "organization status ledger is stale or internally inconsistent".into(),
        ));
    }
    Ok(())
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, AhlFeedError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        AhlFeedError::Validation(format!("fingerprint serialization failed: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ahl_rollover::{
            AhlPreseasonLeagueOrganizationReviewFailure, AhlPreseasonOrganizationReview,
            AhlPreseasonOrganizationReviewRow, AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA,
        },
        career_landing::OfficialNhlOrganizationFact,
    };

    fn review() -> AhlPreseasonLeagueOrganizationReview {
        let child = |team: &str, player_id: u32| AhlPreseasonOrganizationReview {
            schema: AHL_PRESEASON_ORGANIZATION_REVIEW_SCHEMA.into(),
            prior_season: 20252026,
            target_season: 20262027,
            nhl_team: team.into(),
            ahl_team: format!("{team} AHL"),
            provider: "AHL".into(),
            roster_fetched_at: "2026-06-01T00:00:00Z".into(),
            crosswalk_fingerprint: "sha256:test".into(),
            draft: true,
            reviewer: None,
            reviewed_at: None,
            identity_blockers: 0,
            decisions_required: 1,
            rows: vec![AhlPreseasonOrganizationReviewRow {
                provider_player_id: player_id.to_string(),
                display_name: format!("Player {player_id}"),
                nhl_player_id: Some(player_id),
                identity_reviewed: true,
                in_current_camp: Some(false),
                decision_kind: None,
                evidence_urls: Vec::new(),
                note: String::new(),
            }],
        };
        AhlPreseasonLeagueOrganizationReview {
            schema: AHL_PRESEASON_LEAGUE_ORGANIZATION_REVIEW_SCHEMA.into(),
            prior_season: 20252026,
            target_season: 20262027,
            draft: true,
            teams_requested: 2,
            teams_built: 2,
            identity_blockers: 0,
            decisions_required: 2,
            reviews: vec![child("NYR", 1), child("SEA", 2)],
            failures: Vec::<AhlPreseasonLeagueOrganizationReviewFailure>::new(),
            disclosures: Vec::new(),
        }
    }

    fn fact(player_id: u32, team: Option<&str>) -> OfficialNhlOrganizationFact {
        OfficialNhlOrganizationFact {
            player_id,
            current_team_abbrev: team.map(str::to_owned),
            current_team_id: None,
            is_active: Some(true),
            observed_at: "2026-07-28T00:00:00Z".into(),
            source_url: format!("https://api-web.nhle.com/v1/player/{player_id}/landing"),
        }
    }

    #[test]
    fn same_team_is_retained_and_other_team_is_departed() {
        let review = review();
        let mut store = CareerHistoryStore::new();
        store.upsert_organization_fact(fact(1, Some("NYR")));
        store.upsert_organization_fact(fact(2, Some("NYR")));
        let ledger =
            build_ahl_organization_status_ledger(&review, &store, "2026-07-28T12:00:00Z", 14)
                .unwrap();
        assert_eq!(ledger.counts.retained, 1);
        assert_eq!(ledger.counts.departed, 1);
        let application = apply_ahl_organization_status_ledger(&review, &ledger).unwrap();
        assert_eq!(application.decisions_applied, 2);
        assert_eq!(application.decisions_remaining, 0);
        assert!(application.review.draft);
    }

    #[test]
    fn missing_current_team_remains_unresolved() {
        let review = review();
        let mut store = CareerHistoryStore::new();
        store.upsert_organization_fact(fact(1, None));
        let ledger =
            build_ahl_organization_status_ledger(&review, &store, "2026-07-28T12:00:00Z", 14)
                .unwrap();
        assert_eq!(ledger.counts.resolved, 0);
        assert_eq!(ledger.counts.unresolved, 2);
    }

    #[test]
    fn inactive_or_stale_landing_fact_remains_unresolved() {
        let review = review();
        let mut store = CareerHistoryStore::new();
        let mut inactive = fact(1, Some("NYR"));
        inactive.is_active = Some(false);
        store.upsert_organization_fact(inactive);
        let mut stale = fact(2, Some("SEA"));
        stale.observed_at = "2026-06-01T00:00:00Z".into();
        store.upsert_organization_fact(stale);
        let ledger =
            build_ahl_organization_status_ledger(&review, &store, "2026-07-28T12:00:00Z", 14)
                .unwrap();
        assert_eq!(ledger.counts.resolved, 0);
        assert!(ledger.rows.iter().any(|row| {
            row.unavailable_reason
                == Some(AhlOrganizationStatusUnavailableReason::OfficialLandingInactive)
        }));
        assert!(ledger.rows.iter().any(|row| {
            row.unavailable_reason
                == Some(AhlOrganizationStatusUnavailableReason::StaleOfficialLandingFact)
        }));
    }

    #[test]
    fn application_rejects_a_stale_review() {
        let review = review();
        let ledger = build_ahl_organization_status_ledger(
            &review,
            &CareerHistoryStore::new(),
            "2026-07-28T12:00:00Z",
            14,
        )
        .unwrap();
        let mut changed = review;
        changed.reviews[0].rows[0].display_name = "Changed".into();
        assert!(apply_ahl_organization_status_ledger(&changed, &ledger).is_err());
    }

    #[test]
    fn application_rejects_reversed_or_conflicting_decisions() {
        let review = review();
        let mut store = CareerHistoryStore::new();
        store.upsert_organization_fact(fact(1, Some("NYR")));
        store.upsert_organization_fact(fact(2, Some("SEA")));
        let ledger =
            build_ahl_organization_status_ledger(&review, &store, "2026-07-28T12:00:00Z", 14)
                .unwrap();

        let mut reversed = ledger.clone();
        reversed.rows[0].decision = Some(AhlPreseasonDecisionKind::Departed);
        reversed.counts.retained -= 1;
        reversed.counts.departed += 1;
        assert!(apply_ahl_organization_status_ledger(&review, &reversed).is_err());

        let mut conflicting_review = review;
        conflicting_review.reviews[0].rows[0].decision_kind =
            Some(AhlPreseasonDecisionKind::Departed);
        let conflicting_ledger = build_ahl_organization_status_ledger(
            &conflicting_review,
            &store,
            "2026-07-28T12:00:00Z",
            14,
        )
        .unwrap();
        assert!(
            apply_ahl_organization_status_ledger(&conflicting_review, &conflicting_ledger).is_err()
        );
    }
}
