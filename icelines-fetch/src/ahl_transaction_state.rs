//! Cutoff-aware AHL roster state from explicit official transactions.
//!
//! This ledger interprets only the latest dated event set observed for a
//! provider player at or before a caller-supplied cutoff. Provider identities
//! are joined to NHL identities only through a separately reviewed crosswalk.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use icelines_core::{AhlAffiliationCatalogView, AHL_AFFILIATION_CATALOG_SCHEMA};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ahl::{
        AhlFeedError, AhlIdentityLeagueCrosswalkView, AhlIdentityReviewStatus,
        AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA,
    },
    ahl_transactions::{
        AhlTransactionKind, AhlTransactionRow, AhlTransactionSnapshot,
        AHL_TRANSACTION_SNAPSHOT_SCHEMA,
    },
};

pub const AHL_TRANSACTION_STATE_LEDGER_SCHEMA: &str = "ahl_transaction_state_ledger.v1";
pub const AHL_TRANSACTION_STATE_METHOD: &str = "latest_dated_explicit_event_set.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlTransactionRosterState {
    Assigned,
    Removed,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlTransactionStateReason {
    SingleLatestAdd,
    LatestAddAfterDeleteFromOtherTeam,
    LatestDeleteWithoutAdd,
    MultipleLatestAdds,
    SameTeamAddAndDelete,
    UnknownLatestEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionStateEvent {
    pub provider_team_id: String,
    pub ahl_team: String,
    pub kind: AhlTransactionKind,
    pub raw_type: String,
    pub description: String,
    pub source_page: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionStateRow {
    pub provider_player_id: String,
    pub display_name: String,
    #[serde(default)]
    pub nhl_player_id: Option<u32>,
    pub latest_event_date: String,
    pub state: AhlTransactionRosterState,
    pub reason: AhlTransactionStateReason,
    #[serde(default)]
    pub assigned_provider_team_id: Option<String>,
    #[serde(default)]
    pub assigned_ahl_team: Option<String>,
    #[serde(default)]
    pub assigned_nhl_team: Option<String>,
    pub latest_events: Vec<AhlTransactionStateEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionStateCounts {
    pub source_events: usize,
    pub events_through_cutoff: usize,
    pub players_with_events: usize,
    pub assigned: usize,
    pub removed: usize,
    pub ambiguous: usize,
    pub canonically_identified: usize,
    pub identity_unavailable: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlTransactionStateLedgerView {
    pub schema: String,
    pub season: u32,
    pub cutoff: String,
    pub method: String,
    pub transaction_snapshot_fingerprint: String,
    pub identity_crosswalk_fingerprint: String,
    pub affiliation_catalog_fingerprint: String,
    pub source_url: String,
    pub counts: AhlTransactionStateCounts,
    pub source_fingerprint: String,
    pub players: Vec<AhlTransactionStateRow>,
    pub disclosures: Vec<String>,
}

pub fn build_ahl_transaction_state_ledger(
    snapshot: &AhlTransactionSnapshot,
    identities: &AhlIdentityLeagueCrosswalkView,
    affiliations: &AhlAffiliationCatalogView,
    cutoff: impl Into<String>,
) -> Result<AhlTransactionStateLedgerView, AhlFeedError> {
    snapshot.validate()?;
    let cutoff = cutoff.into();
    let cutoff_date = NaiveDate::parse_from_str(&cutoff, "%Y-%m-%d").map_err(|_| {
        AhlFeedError::Validation("AHL transaction cutoff must be YYYY-MM-DD".into())
    })?;
    if snapshot.schema != AHL_TRANSACTION_SNAPSHOT_SCHEMA
        || identities.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA
        || identities.provider != snapshot.provider
        || identities.season > snapshot.season
        || affiliations.schema != AHL_AFFILIATION_CATALOG_SCHEMA
        || affiliations.season != snapshot.season
    {
        return Err(AhlFeedError::Validation(
            "transaction state requires compatible source, reviewed identity, and target affiliation authorities"
                .into(),
        ));
    }

    let team_names = snapshot
        .teams
        .iter()
        .map(|team| (team.provider_team_id.as_str(), team.team_name.as_str()))
        .collect::<BTreeMap<_, _>>();
    if team_names.len() != snapshot.teams.len() {
        return Err(AhlFeedError::Validation(
            "transaction snapshot contains duplicate provider team identities".into(),
        ));
    }
    let organization_by_affiliate = affiliations
        .affiliations
        .iter()
        .map(|row| (row.ahl_team.as_str(), row.nhl_team.as_str()))
        .collect::<BTreeMap<_, _>>();
    if organization_by_affiliate.len() != affiliations.affiliations.len() {
        return Err(AhlFeedError::Validation(
            "affiliation catalog contains duplicate AHL teams".into(),
        ));
    }
    for team_name in team_names.values() {
        if !organization_by_affiliate.contains_key(team_name) {
            return Err(AhlFeedError::Validation(format!(
                "transaction provider team `{team_name}` has no target affiliation"
            )));
        }
    }

    let canonical_ids = reviewed_identity_map(identities)?;
    let mut by_player: BTreeMap<&str, Vec<&AhlTransactionRow>> = BTreeMap::new();
    let mut events_through_cutoff = 0usize;
    for event in &snapshot.transactions {
        let date = NaiveDate::parse_from_str(&event.transaction_date, "%Y-%m-%d")
            .map_err(|_| AhlFeedError::Validation("invalid transaction event date".into()))?;
        if date <= cutoff_date {
            events_through_cutoff += 1;
            by_player
                .entry(event.provider_player_id.as_str())
                .or_default()
                .push(event);
        }
    }

    let mut players = Vec::with_capacity(by_player.len());
    for (provider_player_id, events) in by_player {
        players.push(resolve_latest_state(
            provider_player_id,
            &events,
            canonical_ids.get(provider_player_id).copied(),
            &team_names,
            &organization_by_affiliate,
        )?);
    }
    players.sort_by(|left, right| left.provider_player_id.cmp(&right.provider_player_id));
    let assigned = players
        .iter()
        .filter(|row| row.state == AhlTransactionRosterState::Assigned)
        .count();
    let removed = players
        .iter()
        .filter(|row| row.state == AhlTransactionRosterState::Removed)
        .count();
    let ambiguous = players.len() - assigned - removed;
    let canonically_identified = players
        .iter()
        .filter(|row| row.nhl_player_id.is_some())
        .count();
    let mut ledger = AhlTransactionStateLedgerView {
        schema: AHL_TRANSACTION_STATE_LEDGER_SCHEMA.into(),
        season: snapshot.season,
        cutoff,
        method: AHL_TRANSACTION_STATE_METHOD.into(),
        transaction_snapshot_fingerprint: fingerprint(snapshot)?,
        identity_crosswalk_fingerprint: fingerprint(identities)?,
        affiliation_catalog_fingerprint: fingerprint(affiliations)?,
        source_url: snapshot.source_url.clone(),
        counts: AhlTransactionStateCounts {
            source_events: snapshot.transactions.len(),
            events_through_cutoff,
            players_with_events: players.len(),
            assigned,
            removed,
            ambiguous,
            canonically_identified,
            identity_unavailable: players.len() - canonically_identified,
        },
        source_fingerprint: String::new(),
        players,
        disclosures: vec![
            "Only the latest dated explicit event set at or before cutoff is interpreted; source absence is never an assignment fact.".into(),
            "A single latest ADD assigns the player only when no same-team ADD/DEL or multiple-ADD conflict exists. A latest DEL set without an ADD establishes removal from the observed AHL transaction state.".into(),
            "Same-date feed rows have no trusted intraday order. Unknown kinds, multiple destinations, and same-team ADD/DEL sets remain ambiguous.".into(),
            "This ledger does not establish NHL contract rights, waiver clearance, organization retention, lineup role, or recall probability.".into(),
        ],
    };
    ledger.source_fingerprint = fingerprint_without_ledger_fingerprint(&ledger)?;
    validate_ahl_transaction_state_ledger(&ledger, snapshot, identities, affiliations)?;
    Ok(ledger)
}

pub fn validate_ahl_transaction_state_ledger(
    ledger: &AhlTransactionStateLedgerView,
    snapshot: &AhlTransactionSnapshot,
    identities: &AhlIdentityLeagueCrosswalkView,
    affiliations: &AhlAffiliationCatalogView,
) -> Result<(), AhlFeedError> {
    let cutoff = NaiveDate::parse_from_str(&ledger.cutoff, "%Y-%m-%d")
        .map_err(|_| AhlFeedError::Validation("transaction-state cutoff is invalid".into()))?;
    let expected_events = snapshot
        .transactions
        .iter()
        .filter(|event| {
            NaiveDate::parse_from_str(&event.transaction_date, "%Y-%m-%d")
                .is_ok_and(|date| date <= cutoff)
        })
        .count();
    let assigned = ledger
        .players
        .iter()
        .filter(|row| row.state == AhlTransactionRosterState::Assigned)
        .count();
    let removed = ledger
        .players
        .iter()
        .filter(|row| row.state == AhlTransactionRosterState::Removed)
        .count();
    let ambiguous = ledger.players.len() - assigned - removed;
    let identified = ledger
        .players
        .iter()
        .filter(|row| row.nhl_player_id.is_some())
        .count();
    let mut provider_ids = BTreeSet::new();
    if ledger.schema != AHL_TRANSACTION_STATE_LEDGER_SCHEMA
        || ledger.season != snapshot.season
        || ledger.method != AHL_TRANSACTION_STATE_METHOD
        || ledger.source_url != snapshot.source_url
        || ledger.transaction_snapshot_fingerprint != fingerprint(snapshot)?
        || ledger.identity_crosswalk_fingerprint != fingerprint(identities)?
        || ledger.affiliation_catalog_fingerprint != fingerprint(affiliations)?
        || ledger.source_fingerprint != fingerprint_without_ledger_fingerprint(ledger)?
        || ledger.counts.source_events != snapshot.transactions.len()
        || ledger.counts.events_through_cutoff != expected_events
        || ledger.counts.players_with_events != ledger.players.len()
        || ledger.counts.assigned != assigned
        || ledger.counts.removed != removed
        || ledger.counts.ambiguous != ambiguous
        || ledger.counts.canonically_identified != identified
        || ledger.counts.identity_unavailable != ledger.players.len() - identified
        || ledger.players.iter().any(|row| {
            row.provider_player_id.trim().is_empty()
                || row.display_name.trim().is_empty()
                || !provider_ids.insert(row.provider_player_id.as_str())
                || row.latest_events.is_empty()
                || row.latest_events.iter().any(|event| {
                    event.provider_team_id.trim().is_empty()
                        || event.ahl_team.trim().is_empty()
                        || event.raw_type.trim().is_empty()
                        || event.description.trim().is_empty()
                })
                || match row.state {
                    AhlTransactionRosterState::Assigned => {
                        row.assigned_provider_team_id.is_none()
                            || row.assigned_ahl_team.is_none()
                            || row.assigned_nhl_team.is_none()
                    }
                    AhlTransactionRosterState::Removed | AhlTransactionRosterState::Ambiguous => {
                        row.assigned_provider_team_id.is_some()
                            || row.assigned_ahl_team.is_some()
                            || row.assigned_nhl_team.is_some()
                    }
                }
        })
    {
        return Err(AhlFeedError::Validation(
            "transaction-state ledger is inconsistent, tampered, or bound to different authorities"
                .into(),
        ));
    }
    Ok(())
}

fn reviewed_identity_map(
    identities: &AhlIdentityLeagueCrosswalkView,
) -> Result<BTreeMap<&str, u32>, AhlFeedError> {
    let mut output = BTreeMap::new();
    for crosswalk in &identities.crosswalks {
        for row in &crosswalk.rows {
            if row.review_status != AhlIdentityReviewStatus::Reviewed {
                continue;
            }
            let player_id = row.nhl_player_id.ok_or_else(|| {
                AhlFeedError::Validation("reviewed AHL identity has no NHL player ID".into())
            })?;
            if output
                .insert(row.provider_player_id.as_str(), player_id)
                .is_some_and(|existing| existing != player_id)
            {
                return Err(AhlFeedError::Validation(format!(
                    "reviewed provider identity {} maps to conflicting NHL players",
                    row.provider_player_id
                )));
            }
        }
    }
    Ok(output)
}

fn resolve_latest_state(
    provider_player_id: &str,
    events: &[&AhlTransactionRow],
    nhl_player_id: Option<u32>,
    team_names: &BTreeMap<&str, &str>,
    organization_by_affiliate: &BTreeMap<&str, &str>,
) -> Result<AhlTransactionStateRow, AhlFeedError> {
    let latest_date = events
        .iter()
        .map(|event| event.transaction_date.as_str())
        .max()
        .expect("non-empty transaction group");
    let mut latest = events
        .iter()
        .copied()
        .filter(|event| event.transaction_date == latest_date)
        .collect::<Vec<_>>();
    latest.sort_by(|left, right| {
        left.provider_team_id
            .cmp(&right.provider_team_id)
            .then_with(|| left.raw_type.cmp(&right.raw_type))
            .then_with(|| left.description.cmp(&right.description))
            .then_with(|| left.source_page.cmp(&right.source_page))
    });
    let adds = latest
        .iter()
        .filter(|event| event.kind == AhlTransactionKind::Add)
        .map(|event| event.provider_team_id.as_str())
        .collect::<BTreeSet<_>>();
    let deletes = latest
        .iter()
        .filter(|event| event.kind == AhlTransactionKind::Delete)
        .map(|event| event.provider_team_id.as_str())
        .collect::<BTreeSet<_>>();
    let has_other = latest
        .iter()
        .any(|event| event.kind == AhlTransactionKind::Other);
    let same_team_conflict = adds.iter().any(|team| deletes.contains(team));
    let (state, reason, assigned_provider_team_id) = if has_other {
        (
            AhlTransactionRosterState::Ambiguous,
            AhlTransactionStateReason::UnknownLatestEventKind,
            None,
        )
    } else if adds.len() > 1 {
        (
            AhlTransactionRosterState::Ambiguous,
            AhlTransactionStateReason::MultipleLatestAdds,
            None,
        )
    } else if same_team_conflict {
        (
            AhlTransactionRosterState::Ambiguous,
            AhlTransactionStateReason::SameTeamAddAndDelete,
            None,
        )
    } else if let Some(team) = adds.first() {
        (
            AhlTransactionRosterState::Assigned,
            if deletes.is_empty() {
                AhlTransactionStateReason::SingleLatestAdd
            } else {
                AhlTransactionStateReason::LatestAddAfterDeleteFromOtherTeam
            },
            Some((*team).to_owned()),
        )
    } else {
        (
            AhlTransactionRosterState::Removed,
            AhlTransactionStateReason::LatestDeleteWithoutAdd,
            None,
        )
    };
    let assigned_ahl_team = assigned_provider_team_id
        .as_deref()
        .map(|team| team_names[team].to_owned());
    let assigned_nhl_team = assigned_ahl_team
        .as_deref()
        .map(|team| organization_by_affiliate[team].to_owned());
    let display_names = latest
        .iter()
        .map(|event| event.display_name.as_str())
        .collect::<BTreeSet<_>>();
    if display_names.len() != 1 {
        return Err(AhlFeedError::Validation(format!(
            "latest transaction rows disagree on display name for provider player {provider_player_id}"
        )));
    }
    Ok(AhlTransactionStateRow {
        provider_player_id: provider_player_id.into(),
        display_name: (*display_names.first().unwrap()).into(),
        nhl_player_id,
        latest_event_date: latest_date.into(),
        state,
        reason,
        assigned_provider_team_id,
        assigned_ahl_team,
        assigned_nhl_team,
        latest_events: latest
            .into_iter()
            .map(|event| AhlTransactionStateEvent {
                provider_team_id: event.provider_team_id.clone(),
                ahl_team: team_names[event.provider_team_id.as_str()].into(),
                kind: event.kind,
                raw_type: event.raw_type.clone(),
                description: event.description.clone(),
                source_page: event.source_page,
            })
            .collect(),
    })
}

fn fingerprint<T: Serialize>(value: &T) -> Result<String, AhlFeedError> {
    let bytes =
        serde_json::to_vec(value).map_err(|error| AhlFeedError::Validation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn fingerprint_without_ledger_fingerprint(
    ledger: &AhlTransactionStateLedgerView,
) -> Result<String, AhlFeedError> {
    let mut canonical = ledger.clone();
    canonical.source_fingerprint.clear();
    fingerprint(&canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ahl::{AhlIdentityCrosswalkCounts, AhlIdentityCrosswalkRow, AhlIdentityCrosswalkView},
        ahl_transactions::{
            AhlTransactionPageEvidence, AhlTransactionTeamIdentity, AHL_TRANSACTION_SOURCE_URL,
        },
    };
    use icelines_core::AhlAffiliationView;

    fn event(team: &str, kind: AhlTransactionKind, date: &str) -> AhlTransactionRow {
        AhlTransactionRow {
            transaction_date: date.into(),
            provider_player_id: "provider-1".into(),
            display_name: "Player One".into(),
            position: Some("C".into()),
            provider_team_id: team.into(),
            team_display_name: if team == "1" { "Alpha" } else { "Beta" }.into(),
            kind,
            raw_type: match kind {
                AhlTransactionKind::Add => "ADD",
                AhlTransactionKind::Delete => "DEL",
                AhlTransactionKind::Other => "OTHER",
            }
            .into(),
            description: "Official transaction".into(),
            source_page: 1,
        }
    }

    fn inputs(
        events: Vec<AhlTransactionRow>,
    ) -> (
        AhlTransactionSnapshot,
        AhlIdentityLeagueCrosswalkView,
        AhlAffiliationCatalogView,
    ) {
        let snapshot = AhlTransactionSnapshot {
            schema: AHL_TRANSACTION_SNAPSHOT_SCHEMA.into(),
            season: 20262027,
            provider: crate::ahl::AHL_PROVIDER.into(),
            provider_season_id: "91".into(),
            provider_season_name: "2026-27 Regular Season".into(),
            source_url: AHL_TRANSACTION_SOURCE_URL.into(),
            total_results: events.len(),
            teams: vec![
                AhlTransactionTeamIdentity {
                    provider_team_id: "1".into(),
                    team_code: "ALP".into(),
                    team_name: "Alpha".into(),
                },
                AhlTransactionTeamIdentity {
                    provider_team_id: "2".into(),
                    team_code: "BET".into(),
                    team_name: "Beta".into(),
                },
            ],
            pages: vec![AhlTransactionPageEvidence {
                page: 1,
                first: 0,
                limit: 200,
                dataset_id: "test".into(),
                fetched_at: "2026-07-28T12:00:00Z".into(),
                feed_url: "https://example.test/feed".into(),
                rows: events.len(),
            }],
            transactions: events,
            disclosures: Vec::new(),
        };
        let crosswalk = AhlIdentityCrosswalkView {
            schema: crate::ahl::AHL_IDENTITY_CROSSWALK_SCHEMA.into(),
            season: 20252026,
            provider: crate::ahl::AHL_PROVIDER.into(),
            ahl_team: "Alpha".into(),
            nhl_affiliate: Some("AAA".into()),
            roster_fetched_at: "2026-04-20T00:00:00Z".into(),
            candidates_checked_at: "2026-07-28T00:00:00Z".into(),
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
                provider_player_id: "provider-1".into(),
                ahl_display_name: "Player One".into(),
                ahl_birth_date: "2000-01-01".into(),
                match_basis: crate::ahl::AhlIdentityMatchBasis::ExactNameAndBirthDate,
                review_status: AhlIdentityReviewStatus::Reviewed,
                nhl_player_id: Some(8470001),
                nhl_display_name: Some("Player One".into()),
                nhl_birth_date: Some("2000-01-01".into()),
                evidence_urls: vec!["https://example.test/player".into()],
                note: "reviewed".into(),
            }],
            disclosures: Vec::new(),
        };
        (
            snapshot,
            AhlIdentityLeagueCrosswalkView {
                schema: AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA.into(),
                season: 20252026,
                provider: crate::ahl::AHL_PROVIDER.into(),
                roster_fetched_at: "2026-04-20T00:00:00Z".into(),
                candidates_checked_at: "2026-07-28T00:00:00Z".into(),
                teams: 1,
                roster_appearances: 1,
                unique_provider_players: 1,
                crosswalks: vec![crosswalk],
                disclosures: Vec::new(),
            },
            AhlAffiliationCatalogView {
                schema: AHL_AFFILIATION_CATALOG_SCHEMA.into(),
                season: 20262027,
                checked_at: "2026-07-28".into(),
                source_url: "https://example.test/affiliations".into(),
                affiliations: vec![
                    AhlAffiliationView {
                        nhl_team: "AAA".into(),
                        ahl_team: "Alpha".into(),
                    },
                    AhlAffiliationView {
                        nhl_team: "BBB".into(),
                        ahl_team: "Beta".into(),
                    },
                ],
            },
        )
    }

    #[test]
    fn latest_add_after_other_team_delete_assigns_destination() {
        let (snapshot, identities, affiliations) = inputs(vec![
            event("1", AhlTransactionKind::Delete, "2026-07-20"),
            event("2", AhlTransactionKind::Add, "2026-07-20"),
        ]);
        let ledger =
            build_ahl_transaction_state_ledger(&snapshot, &identities, &affiliations, "2026-07-28")
                .unwrap();
        assert_eq!(ledger.counts.assigned, 1);
        assert_eq!(ledger.players[0].assigned_ahl_team.as_deref(), Some("Beta"));
        assert_eq!(ledger.players[0].assigned_nhl_team.as_deref(), Some("BBB"));
        assert_eq!(ledger.players[0].nhl_player_id, Some(8470001));
    }

    #[test]
    fn same_team_add_delete_is_ambiguous_without_intraday_order() {
        let (snapshot, identities, affiliations) = inputs(vec![
            event("1", AhlTransactionKind::Delete, "2026-07-20"),
            event("1", AhlTransactionKind::Add, "2026-07-20"),
        ]);
        let ledger =
            build_ahl_transaction_state_ledger(&snapshot, &identities, &affiliations, "2026-07-28")
                .unwrap();
        assert_eq!(
            ledger.players[0].state,
            AhlTransactionRosterState::Ambiguous
        );
        assert_eq!(
            ledger.players[0].reason,
            AhlTransactionStateReason::SameTeamAddAndDelete
        );
    }

    #[test]
    fn cutoff_excludes_later_events_and_empty_snapshot_resolves_nothing() {
        let (snapshot, identities, affiliations) =
            inputs(vec![event("1", AhlTransactionKind::Add, "2026-08-01")]);
        let ledger =
            build_ahl_transaction_state_ledger(&snapshot, &identities, &affiliations, "2026-07-28")
                .unwrap();
        assert_eq!(ledger.counts.events_through_cutoff, 0);
        assert!(ledger.players.is_empty());
    }

    #[test]
    fn validation_rejects_tampered_or_rebound_ledgers() {
        let (snapshot, identities, affiliations) =
            inputs(vec![event("1", AhlTransactionKind::Add, "2026-07-20")]);
        let mut ledger =
            build_ahl_transaction_state_ledger(&snapshot, &identities, &affiliations, "2026-07-28")
                .unwrap();
        ledger.players[0].assigned_nhl_team = Some("BBB".into());
        assert!(validate_ahl_transaction_state_ledger(
            &ledger,
            &snapshot,
            &identities,
            &affiliations
        )
        .is_err());
    }
}
