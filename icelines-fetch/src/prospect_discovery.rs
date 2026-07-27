//! Reviewed AHL season facts to UI-neutral prospect-discovery studies.
//!
//! Production comes from official AHL snapshots, identity comes only from
//! reviewed crosswalk rows, and analyst context stays separately authored.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use icelines_core::{
    build_prospect_development_study, build_prospect_discovery_board,
    build_prospect_goalie_development_study, AhlAffiliationCatalogView, ProspectAvailabilityStatus,
    ProspectDevelopmentSeasonInput, ProspectDevelopmentStudyConfig, ProspectDevelopmentStudyView,
    ProspectDiscoveryBoardView, ProspectGoalieDevelopmentSeasonInput,
    ProspectGoalieDevelopmentStudyConfig, ProspectGoalieDevelopmentStudyInput,
    ProspectGoalieDevelopmentStudyView, ProspectOpportunityStatus, ProspectStudyEvidenceInput,
    AHL_AFFILIATION_CATALOG_SCHEMA,
};
use serde::{Deserialize, Serialize};

use crate::ahl::{
    AhlIdentityCrosswalkView, AhlIdentityLeagueCrosswalkView, AhlIdentityReviewStatus,
    AhlRosterStatsSnapshot, AHL_IDENTITY_CROSSWALK_SCHEMA, AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA,
    AHL_ROSTER_STATS_SCHEMA,
};

pub const PROSPECT_LEAGUE_CONTEXT_SCHEMA: &str = "prospect_league_context.v1";
pub const PROSPECT_LEAGUE_DISCOVERY_SCHEMA: &str = "prospect_league_discovery.v1";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectLeagueContextAuthority {
    #[default]
    Authored,
    ObservedDraft,
}

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
    #[serde(default)]
    pub authority: ProspectLeagueContextAuthority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of_date: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snapshot_seasons: Vec<u32>,
    pub players: Vec<ProspectLeaguePlayerContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclusions: Vec<ProspectLeagueContextExclusionView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProspectLeagueContextDraftConfig {
    pub max_age: u8,
    pub as_of_date: NaiveDate,
    pub minimum_ahl_seasons: usize,
}

impl Default for ProspectLeagueContextDraftConfig {
    fn default() -> Self {
        Self {
            max_age: 24,
            as_of_date: NaiveDate::from_ymd_opt(2026, 9, 15).expect("valid default date"),
            minimum_ahl_seasons: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectLeagueContextExclusionReason {
    GoalieAdapterRequired,
    MissingAffiliation,
    AmbiguousOrganization,
    MissingBirthDate,
    InvalidBirthDate,
    AboveMaximumAge,
    FewerThanMinimumAhlSeasons,
    MissingLatestSkaterStats,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectLeagueContextExclusionView {
    pub player_id: u32,
    pub player: String,
    pub reason: ProspectLeagueContextExclusionReason,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectLeagueExclusionReason {
    MissingReviewedIdentity,
    MissingAhlSkaterStats,
    MissingAhlGoalieStats,
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
    #[serde(default)]
    pub goalie_studies: Vec<ProspectGoalieDevelopmentStudyView>,
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

/// Build a conservative, machine-generated context draft from reviewed league
/// identities. The draft is directly consumable by `prospect-league`, while
/// neutral defaults make the absence of authored injury/opportunity/attention
/// research explicit and prevent those fields from manufacturing upside.
pub fn build_prospect_league_context_draft(
    mut snapshots: Vec<AhlRosterStatsSnapshot>,
    league_crosswalks: Vec<AhlIdentityLeagueCrosswalkView>,
    affiliations: AhlAffiliationCatalogView,
    config: ProspectLeagueContextDraftConfig,
) -> Result<ProspectLeagueContext, String> {
    if snapshots.len() < config.minimum_ahl_seasons
        || config.minimum_ahl_seasons < 2
        || config.max_age == 0
        || affiliations.schema != AHL_AFFILIATION_CATALOG_SCHEMA
        || affiliations.affiliations.is_empty()
    {
        return Err("invalid prospect league context draft inputs".to_owned());
    }
    snapshots.sort_by_key(|snapshot| snapshot.season);
    let mut snapshot_seasons = BTreeSet::new();
    for snapshot in &snapshots {
        if snapshot.schema != AHL_ROSTER_STATS_SCHEMA
            || snapshot.provider.trim().is_empty()
            || !snapshot_seasons.insert(snapshot.season)
        {
            return Err("invalid or duplicate AHL snapshot for context draft".to_owned());
        }
    }
    let latest_season = *snapshot_seasons
        .iter()
        .next_back()
        .expect("validated snapshots");
    let latest_snapshot = snapshots
        .iter()
        .find(|snapshot| snapshot.season == latest_season)
        .expect("latest snapshot exists");
    if affiliations.season != latest_season
        || affiliations.checked_at.trim().is_empty()
        || !(affiliations.source_url.starts_with("https://")
            || affiliations.source_url.starts_with("http://"))
    {
        return Err(format!(
            "affiliation catalog must be sourced and match latest snapshot season {latest_season}"
        ));
    }

    let mut affiliation_by_team = BTreeMap::new();
    for row in affiliations.affiliations {
        if row.nhl_team.trim().is_empty()
            || row.ahl_team.trim().is_empty()
            || affiliation_by_team
                .insert(row.ahl_team, row.nhl_team)
                .is_some()
        {
            return Err("invalid or duplicate AHL team in affiliation catalog".to_owned());
        }
    }

    let mut crosswalk_by_season = BTreeMap::new();
    for league in league_crosswalks {
        let Some(snapshot) = snapshots.iter().find(|row| row.season == league.season) else {
            return Err(format!(
                "AHL league crosswalk season {} has no supplied snapshot",
                league.season
            ));
        };
        if league.schema != AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA
            || league.provider != snapshot.provider
            || league.teams != league.crosswalks.len()
            || crosswalk_by_season
                .insert(league.season, league.crosswalks)
                .is_some()
        {
            return Err("invalid or duplicate AHL league crosswalk".to_owned());
        }
    }
    if !snapshot_seasons
        .iter()
        .all(|season| crosswalk_by_season.contains_key(season))
    {
        return Err("every context-draft snapshot requires a league crosswalk".to_owned());
    }

    #[derive(Default)]
    struct Candidate {
        names: BTreeSet<String>,
        birth_dates: BTreeSet<String>,
        observed_seasons: BTreeSet<u32>,
        latest_teams: BTreeSet<String>,
        latest_active_teams: BTreeSet<String>,
        latest_positions: BTreeMap<String, usize>,
        latest_active_positions: BTreeMap<String, usize>,
        evidence_urls: BTreeSet<String>,
    }

    let mut candidates = BTreeMap::<u32, Candidate>::new();
    for (season, crosswalks) in &crosswalk_by_season {
        let snapshot = snapshots
            .iter()
            .find(|row| row.season == *season)
            .expect("crosswalk snapshot validated");
        for crosswalk in crosswalks {
            if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
                || crosswalk.season != *season
                || crosswalk.provider != snapshot.provider
            {
                return Err("invalid child AHL identity crosswalk".to_owned());
            }
            let Some(team) = snapshot
                .teams
                .iter()
                .find(|team| team.team_name == crosswalk.ahl_team)
            else {
                return Err(format!(
                    "AHL crosswalk team {} is absent from season {} snapshot",
                    crosswalk.ahl_team, season
                ));
            };
            for row in crosswalk
                .rows
                .iter()
                .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
            {
                let (Some(player_id), Some(name)) =
                    (row.nhl_player_id, row.nhl_display_name.as_ref())
                else {
                    return Err("reviewed AHL identity lacks canonical identity".to_owned());
                };
                let candidate = candidates.entry(player_id).or_default();
                candidate.names.insert(name.clone());
                if let Some(birth_date) = row.nhl_birth_date.as_ref() {
                    candidate.birth_dates.insert(birth_date.clone());
                }
                candidate.evidence_urls.extend(row.evidence_urls.clone());
                let skater_rows = team
                    .skaters
                    .iter()
                    .filter(|skater| {
                        skater.provider_player_id == row.provider_player_id
                            && skater.games_played > 0
                    })
                    .collect::<Vec<_>>();
                if !skater_rows.is_empty() {
                    candidate.observed_seasons.insert(*season);
                    if *season == latest_season {
                        candidate.latest_teams.insert(crosswalk.ahl_team.clone());
                        for skater in skater_rows {
                            *candidate
                                .latest_positions
                                .entry(skater.position.trim().to_ascii_uppercase())
                                .or_default() += skater.games_played as usize;
                            if skater.active {
                                candidate
                                    .latest_active_teams
                                    .insert(crosswalk.ahl_team.clone());
                                *candidate
                                    .latest_active_positions
                                    .entry(skater.position.trim().to_ascii_uppercase())
                                    .or_default() += skater.games_played as usize;
                            }
                        }
                    }
                } else {
                    let goalie_rows = team
                        .goalies
                        .iter()
                        .filter(|goalie| {
                            goalie.provider_player_id == row.provider_player_id
                                && goalie.games_played > 0
                        })
                        .collect::<Vec<_>>();
                    if !goalie_rows.is_empty() {
                        candidate.observed_seasons.insert(*season);
                        if *season == latest_season {
                            candidate.latest_teams.insert(crosswalk.ahl_team.clone());
                            for goalie in goalie_rows {
                                *candidate
                                    .latest_positions
                                    .entry("G".to_owned())
                                    .or_default() += goalie.games_played as usize;
                                if goalie.active {
                                    candidate
                                        .latest_active_teams
                                        .insert(crosswalk.ahl_team.clone());
                                    *candidate
                                        .latest_active_positions
                                        .entry("G".to_owned())
                                        .or_default() += goalie.games_played as usize;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut players = Vec::new();
    let mut exclusions = Vec::new();
    for (player_id, candidate) in candidates
        .into_iter()
        .filter(|(_, candidate)| !candidate.latest_teams.is_empty())
    {
        let normalized_names = candidate
            .names
            .iter()
            .map(|name| icelines_core::normalize_name(name))
            .collect::<BTreeSet<_>>();
        if normalized_names.len() != 1 || candidate.birth_dates.len() > 1 {
            return Err(format!(
                "reviewed canonical identity conflicts across seasons for player {player_id}"
            ));
        }
        let latest_teams = if candidate.latest_active_teams.is_empty() {
            &candidate.latest_teams
        } else {
            &candidate.latest_active_teams
        };
        let latest_positions = if candidate.latest_active_positions.is_empty() {
            &candidate.latest_positions
        } else {
            &candidate.latest_active_positions
        };
        let player = candidate
            .names
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| format!("Player {player_id}"));
        let exclude = |reason, detail| ProspectLeagueContextExclusionView {
            player_id,
            player: player.clone(),
            reason,
            detail,
        };
        if candidate.observed_seasons.len() < config.minimum_ahl_seasons {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::FewerThanMinimumAhlSeasons,
                format!(
                    "{} reviewed AHL season(s) observed; {} required.",
                    candidate.observed_seasons.len(),
                    config.minimum_ahl_seasons
                ),
            ));
            continue;
        }
        if latest_positions.is_empty() {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::MissingLatestSkaterStats,
                "No positive-games player row joined in the latest snapshot.".to_owned(),
            ));
            continue;
        }
        let organizations = latest_teams
            .iter()
            .filter_map(|team| affiliation_by_team.get(team).cloned())
            .collect::<BTreeSet<_>>();
        if latest_teams
            .iter()
            .any(|team| !affiliation_by_team.contains_key(team))
        {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::MissingAffiliation,
                format!(
                    "Latest AHL team(s) lack a dated affiliation mapping: {}.",
                    latest_teams.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            ));
            continue;
        }
        if organizations.len() != 1 {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::AmbiguousOrganization,
                "Latest AHL appearances map to more than one NHL organization.".to_owned(),
            ));
            continue;
        }
        let Some(birth_date_text) = candidate.birth_dates.iter().next() else {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::MissingBirthDate,
                "Reviewed canonical identity has no birth date.".to_owned(),
            ));
            continue;
        };
        let Ok(birth_date) = NaiveDate::parse_from_str(birth_date_text, "%Y-%m-%d") else {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::InvalidBirthDate,
                format!("Canonical birth date is invalid: {birth_date_text}."),
            ));
            continue;
        };
        let age = config.as_of_date.year()
            - birth_date.year()
            - i32::from(
                (config.as_of_date.month(), config.as_of_date.day())
                    < (birth_date.month(), birth_date.day()),
            );
        if age < 0 || age > i32::from(u8::MAX) {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::InvalidBirthDate,
                format!("Canonical birth date is implausible: {birth_date_text}."),
            ));
            continue;
        }
        if age > i32::from(config.max_age) {
            exclusions.push(exclude(
                ProspectLeagueContextExclusionReason::AboveMaximumAge,
                format!(
                    "Age {age} exceeds the configured maximum {}.",
                    config.max_age
                ),
            ));
            continue;
        }
        let position = latest_positions
            .iter()
            .max_by(|left, right| left.1.cmp(right.1).then_with(|| right.0.cmp(left.0)))
            .map(|row| row.0.clone())
            .expect("latest position validated");
        let mut evidence = candidate
            .evidence_urls
            .into_iter()
            .map(|source_url| ProspectStudyEvidenceInput {
                label: format!("Reviewed AHL-to-NHL identity evidence for {player}."),
                source_url,
            })
            .collect::<Vec<_>>();
        evidence.push(ProspectStudyEvidenceInput {
            label: format!(
                "Official AHL {latest_season} snapshot records {player} in the selected organization."
            ),
            source_url: latest_snapshot.source_url.clone(),
        });
        players.push(ProspectLeaguePlayerContext {
            player_id,
            player,
            organization: organizations.into_iter().next().expect("one organization"),
            position,
            age: age as u8,
            nhl_games_played: 0,
            opportunity: ProspectOpportunityStatus::None,
            availability: ProspectAvailabilityStatus::Unknown,
            attention_score: 0.5,
            attention_basis: "Neutral machine-generated placeholder; replace with sourced analyst context before using attention-sensitive discovery lanes.".to_owned(),
            evidence,
        });
    }
    players.sort_by(|left, right| {
        left.organization
            .cmp(&right.organization)
            .then_with(|| left.player.cmp(&right.player))
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
    exclusions.sort_by(|left, right| {
        left.player
            .cmp(&right.player)
            .then_with(|| left.player_id.cmp(&right.player_id))
    });
    if players.is_empty() {
        return Err("no eligible players remained in prospect context draft".to_owned());
    }
    Ok(ProspectLeagueContext {
        schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
        authority: ProspectLeagueContextAuthority::ObservedDraft,
        as_of_date: Some(config.as_of_date.to_string()),
        snapshot_seasons: snapshot_seasons.into_iter().collect(),
        players,
        exclusions,
        disclosures: vec![
            "Observed draft includes only reviewed canonical players appearing in the latest supplied AHL snapshot, at or below the configured age ceiling, with the configured minimum observed AHL seasons.".to_owned(),
            "Organization comes from the supplied dated affiliation catalog; missing or conflicting mappings are explicit exclusions.".to_owned(),
            "NHL games are not inferred by the AHL adapter and remain zero placeholders; opportunity is none, availability unknown, and attention neutral until separately sourced enrichment is applied.".to_owned(),
            "Goalies enter the context only when two reviewed AHL seasons exist; their save-percentage, goals-against-average, and workload evidence is scored by the separate goalie adapter.".to_owned(),
        ],
    })
}

pub fn build_prospect_league_discovery(
    mut snapshots: Vec<AhlRosterStatsSnapshot>,
    crosswalks: Vec<AhlIdentityCrosswalkView>,
    context: ProspectLeagueContext,
    config: ProspectDevelopmentStudyConfig,
) -> Result<ProspectLeagueDiscoveryView, String> {
    validate_authorities(&snapshots, &crosswalks, &context)?;
    let context_authority = context.authority;
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
    let mut goalie_studies = Vec::new();
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

        if player.position.eq_ignore_ascii_case("G") {
            let mut season_totals = BTreeMap::<u32, (u32, u32, u32, u32, u64)>::new();
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
                for row in team.goalies.iter().filter(|row| {
                    row.provider_player_id == identity.provider_player_id && row.games_played > 0
                }) {
                    let Some(seconds) = parse_ahl_minutes_seconds(&row.minutes_played) else {
                        return Err(format!(
                            "invalid AHL goalie minutes {} for player {}",
                            row.minutes_played, player.player_id
                        ));
                    };
                    let totals = season_totals.entry(snapshot.season).or_default();
                    totals.0 = totals.0.saturating_add(row.games_played);
                    totals.1 = totals.1.saturating_add(row.shots_against);
                    totals.2 = totals.2.saturating_add(row.saves);
                    totals.3 = totals.3.saturating_add(row.goals_against);
                    totals.4 = totals.4.saturating_add(seconds);
                    snapshot_evidence.insert((
                        format!(
                            "Official AHL {} goalie snapshot includes {} with {}.",
                            snapshot.season, player.player, identity.ahl_team
                        ),
                        snapshot.source_url.clone(),
                    ));
                }
                identity_evidence.extend(identity.evidence_urls.clone());
            }
            if season_totals.is_empty()
                || season_totals
                    .values()
                    .any(|(_, shots, _, _, seconds)| *shots == 0 || *seconds == 0)
            {
                excluded.push(ProspectLeagueExclusionView {
                    player_id: player.player_id,
                    player: player.player,
                    reason: ProspectLeagueExclusionReason::MissingAhlGoalieStats,
                    detail:
                        "Reviewed identity rows did not join to complete AHL goalie season facts."
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
                        "Only {} reviewed AHL goalie season joined; the study requires at least two.",
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
                    |(season, (games_played, shots, saves, goals_against, seconds))| {
                        ProspectGoalieDevelopmentSeasonInput {
                            season,
                            league: "AHL".to_owned(),
                            games_played,
                            save_percentage: f64::from(saves) / f64::from(shots),
                            goals_against_average: f64::from(goals_against) * 3_600.0
                                / seconds as f64,
                        }
                    },
                )
                .collect();
            goalie_studies.push(build_prospect_goalie_development_study(
                ProspectGoalieDevelopmentStudyInput {
                    player_id: player.player_id,
                    player: player.player,
                    organization: player.organization,
                    age: player.age,
                    nhl_games_played: player.nhl_games_played,
                    seasons,
                    opportunity: player.opportunity,
                    availability: player.availability,
                    evidence,
                },
                ProspectGoalieDevelopmentStudyConfig::default(),
            )?);
            continue;
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
    goalie_studies.sort_by_key(|study| study.player_id);
    excluded.sort_by_key(|row| row.player_id);
    if studies.is_empty() {
        return Err("no eligible prospect studies remained after reviewed AHL joins".to_owned());
    }
    let mut board = build_prospect_discovery_board(studies.clone())?;
    if context_authority == ProspectLeagueContextAuthority::ObservedDraft {
        board.hidden_gems.clear();
        board.buyer_beware.clear();
        board.watch.clear();
        board.disclosures.push(
            "Discovery lanes are suppressed because observed-draft context has no sourced public-attention authority; the underlying studies remain available for attention-independent program analysis."
                .to_owned(),
        );
    }
    Ok(ProspectLeagueDiscoveryView {
        schema: PROSPECT_LEAGUE_DISCOVERY_SCHEMA.to_owned(),
        snapshot_seasons,
        context_players: studies.len() + goalie_studies.len() + excluded.len(),
        studies,
        goalie_studies,
        excluded,
        board,
        disclosures: vec![
            "AHL production is joined only through reviewed season/team identity crosswalk rows; provider-local IDs are never treated as NHL IDs.".to_owned(),
            "Organization, position, age, NHL games, opportunity, availability, and public attention remain explicit authored context rather than feed-derived guesses.".to_owned(),
            "Candidates without two joined AHL seasons are reported as exclusions. Goalie studies feed program ranking but remain outside skater-only Hidden Gems and Buyer Beware lanes.".to_owned(),
            "Multiple reviewed team segments in one season are summed; source snapshot and identity evidence remain attached to each study.".to_owned(),
            "Observed-draft context suppresses all discovery-board lanes until public-attention context is separately sourced; neutral placeholders cannot create Hidden Gem, Buyer Beware, or Watch recommendations.".to_owned(),
        ],
    })
}

fn parse_ahl_minutes_seconds(value: &str) -> Option<u64> {
    let (minutes, seconds) = value.trim().split_once(':')?;
    let minutes = minutes.parse::<u64>().ok()?;
    let seconds = seconds.parse::<u64>().ok()?;
    (seconds < 60).then_some(minutes.saturating_mul(60).saturating_add(seconds))
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
    if context.authority == ProspectLeagueContextAuthority::ObservedDraft
        && context.players.iter().any(|player| {
            player.opportunity != ProspectOpportunityStatus::None
                || player.availability != ProspectAvailabilityStatus::Unknown
                || (player.attention_score - 0.5).abs() > f64::EPSILON
        })
    {
        return Err(
            "observed prospect context draft contains non-neutral authored fields".to_owned(),
        );
    }
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
        AhlGoalieSeasonRow, AhlIdentityCrosswalkCounts, AhlIdentityCrosswalkRow,
        AhlIdentityMatchBasis, AhlSkaterSeasonRow, AhlTeamRosterStats, AHL_PROVIDER,
    };

    #[test]
    fn reviewed_two_season_facts_build_board_and_report_exclusions() {
        let context = ProspectLeagueContext {
            schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
            authority: ProspectLeagueContextAuthority::Authored,
            as_of_date: None,
            snapshot_seasons: vec![],
            players: vec![
                player_context(10, "Joined Prospect"),
                player_context(20, "Missing Prospect"),
            ],
            exclusions: vec![],
            disclosures: vec![],
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
    fn observed_context_draft_selects_reviewed_two_season_young_skater() {
        let snapshots = vec![
            snapshot(20242025, 10, 40, 10, 10),
            snapshot(20252026, 10, 40, 20, 20),
        ];
        let leagues = vec![
            league_crosswalk(crosswalk(20242025, 10, "Joined Prospect")),
            league_crosswalk(crosswalk(20252026, 10, "Joined Prospect")),
        ];
        let context = build_prospect_league_context_draft(
            snapshots,
            leagues,
            AhlAffiliationCatalogView {
                schema: AHL_AFFILIATION_CATALOG_SCHEMA.to_owned(),
                season: 20252026,
                checked_at: "2026-07-26".to_owned(),
                source_url: "https://theahl.com/nhl-affiliations".to_owned(),
                affiliations: vec![icelines_core::AhlAffiliationView {
                    nhl_team: "SEA".to_owned(),
                    ahl_team: "Coachella Valley Firebirds".to_owned(),
                }],
            },
            ProspectLeagueContextDraftConfig {
                max_age: 24,
                as_of_date: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
                minimum_ahl_seasons: 2,
            },
        )
        .unwrap();

        assert_eq!(
            context.authority,
            ProspectLeagueContextAuthority::ObservedDraft
        );
        assert_eq!(context.snapshot_seasons, [20242025, 20252026]);
        assert_eq!(context.players.len(), 1);
        assert_eq!(context.players[0].player_id, 10);
        assert_eq!(context.players[0].organization, "SEA");
        assert_eq!(context.players[0].age, 22);
        assert_eq!(
            context.players[0].opportunity,
            ProspectOpportunityStatus::None
        );
        assert!(context.exclusions.is_empty());
    }

    #[test]
    fn observed_context_draft_cannot_create_attention_sensitive_board_lanes() {
        let snapshots = vec![
            snapshot(20242025, 10, 40, 2, 3),
            snapshot(20252026, 10, 40, 1, 2),
        ];
        let crosswalks = vec![
            crosswalk(20242025, 10, "Joined Prospect"),
            crosswalk(20252026, 10, "Joined Prospect"),
        ];
        let context = build_prospect_league_context_draft(
            snapshots.clone(),
            crosswalks.iter().cloned().map(league_crosswalk).collect(),
            AhlAffiliationCatalogView {
                schema: AHL_AFFILIATION_CATALOG_SCHEMA.to_owned(),
                season: 20252026,
                checked_at: "2026-07-26".to_owned(),
                source_url: "https://theahl.com/nhl-affiliations".to_owned(),
                affiliations: vec![icelines_core::AhlAffiliationView {
                    nhl_team: "SEA".to_owned(),
                    ahl_team: "Coachella Valley Firebirds".to_owned(),
                }],
            },
            ProspectLeagueContextDraftConfig {
                max_age: 24,
                as_of_date: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
                minimum_ahl_seasons: 2,
            },
        )
        .unwrap();

        let view = build_prospect_league_discovery(
            snapshots,
            crosswalks,
            context,
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();

        assert_eq!(view.studies.len(), 1);
        assert!(view.board.hidden_gems.is_empty());
        assert!(view.board.buyer_beware.is_empty());
        assert!(view.board.watch.is_empty());
        assert!(view
            .board
            .disclosures
            .iter()
            .any(|row| row.contains("observed-draft context")));
    }

    #[test]
    fn reviewed_goalie_facts_build_native_goalie_study() {
        let mut goalie_context = player_context(30, "Goalie Prospect");
        goalie_context.position = "G".to_owned();
        let view = build_prospect_league_discovery(
            vec![
                mixed_snapshot(20242025, 0.898, 3.15),
                mixed_snapshot(20252026, 0.914, 2.61),
            ],
            vec![mixed_crosswalk(20242025), mixed_crosswalk(20252026)],
            ProspectLeagueContext {
                schema: PROSPECT_LEAGUE_CONTEXT_SCHEMA.to_owned(),
                authority: ProspectLeagueContextAuthority::Authored,
                as_of_date: None,
                snapshot_seasons: vec![],
                players: vec![player_context(10, "Joined Prospect"), goalie_context],
                exclusions: vec![],
                disclosures: vec![],
            },
            ProspectDevelopmentStudyConfig::default(),
        )
        .unwrap();

        assert_eq!(view.studies.len(), 1);
        assert_eq!(view.goalie_studies.len(), 1);
        assert_eq!(view.goalie_studies[0].player_id, 30);
        assert_eq!(
            view.goalie_studies[0].trajectory,
            icelines_core::ProspectTrajectory::Rising
        );
        assert!(view.goalie_studies[0].seasons[1].save_percentage > 0.913);
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
                authority: ProspectLeagueContextAuthority::Authored,
                as_of_date: None,
                snapshot_seasons: vec![],
                players: vec![player_context(10, "Joined Prospect")],
                exclusions: vec![],
                disclosures: vec![],
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
                authority: ProspectLeagueContextAuthority::Authored,
                as_of_date: None,
                snapshot_seasons: vec![],
                players: vec![player_context(10, "Joined Prospect")],
                exclusions: vec![],
                disclosures: vec![],
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

    fn mixed_snapshot(
        season: u32,
        save_percentage: f64,
        goals_against_average: f64,
    ) -> AhlRosterStatsSnapshot {
        let mut snapshot = snapshot(season, 10, 40, 10, 10);
        snapshot.teams[0].goalies.push(AhlGoalieSeasonRow {
            provider: AHL_PROVIDER.to_owned(),
            provider_player_id: "30".to_owned(),
            name: "Goalie Prospect".to_owned(),
            team_code: "CV".to_owned(),
            active: true,
            rookie: false,
            games_played: 30,
            minutes_played: "1800:00".to_owned(),
            wins: 18,
            losses: 10,
            overtime_losses: 2,
            shots_against: 900,
            saves: (900.0 * save_percentage).round() as u32,
            goals_against: (goals_against_average * 30.0).round() as u32,
            shutouts: 2,
            save_percentage,
            goals_against_average,
        });
        snapshot
    }

    fn mixed_crosswalk(season: u32) -> AhlIdentityCrosswalkView {
        let mut crosswalk = crosswalk(season, 10, "Joined Prospect");
        let mut goalie = crosswalk.rows[0].clone();
        goalie.provider_player_id = "30".to_owned();
        goalie.ahl_display_name = "Goalie Prospect".to_owned();
        goalie.nhl_player_id = Some(30);
        goalie.nhl_display_name = Some("Goalie Prospect".to_owned());
        crosswalk.rows.push(goalie);
        crosswalk
    }

    fn league_crosswalk(crosswalk: AhlIdentityCrosswalkView) -> AhlIdentityLeagueCrosswalkView {
        AhlIdentityLeagueCrosswalkView {
            schema: AHL_IDENTITY_LEAGUE_CROSSWALK_SCHEMA.to_owned(),
            season: crosswalk.season,
            provider: crosswalk.provider.clone(),
            roster_fetched_at: crosswalk.roster_fetched_at.clone(),
            candidates_checked_at: crosswalk.candidates_checked_at.clone(),
            teams: 1,
            roster_appearances: crosswalk.rows.len(),
            unique_provider_players: crosswalk.rows.len(),
            crosswalks: vec![crosswalk],
            disclosures: vec![],
        }
    }
}
