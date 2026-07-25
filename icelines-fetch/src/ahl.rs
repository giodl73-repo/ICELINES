//! Official AHL roster and season-stat ingestion.
//!
//! The AHL statistics pages are backed by HockeyTech's Statview feed.  The
//! identifiers returned by that feed are provider-local: an AHL `player_id`
//! is never an NHL player id.  This module preserves that boundary explicitly
//! and leaves identity linking to a reviewed crosswalk.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const AHL_ROSTER_STATS_SCHEMA: &str = "ahl_roster_stats.v1";
pub const AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA: &str = "ahl_canonical_identity_catalog.v1";
pub const AHL_IDENTITY_CROSSWALK_SCHEMA: &str = "ahl_identity_crosswalk.v1";
pub const AHL_PROVIDER: &str = "ahl_hockeytech_statview";
pub const AHL_STATS_SOURCE_URL: &str = "https://theahl.com/stats/player-stats";
pub const AHL_ROSTER_SOURCE_URL: &str = "https://theahl.com/stats/roster";
pub const AHL_FEED_BASE_URL: &str = "https://lscluster.hockeytech.com/feed/index.php";
const AHL_FEED_KEY: &str = "ccb91f29d6744675";
const AHL_CLIENT_CODE: &str = "ahl";

#[derive(Debug, Error)]
pub enum AhlFeedError {
    #[error("AHL feed request failed for {url}: {detail}")]
    Request { url: String, detail: String },
    #[error("AHL feed returned HTTP {status} for {url}")]
    Http { status: u16, url: String },
    #[error("AHL feed schema changed: {0}")]
    Schema(String),
    #[error("AHL season not found: {0}")]
    SeasonNotFound(String),
    #[error("unknown AHL team filter(s): {0}")]
    UnknownTeams(String),
    #[error("invalid AHL snapshot: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlRosterStatsSnapshot {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub provider_season_id: String,
    pub provider_season_name: String,
    pub fetched_at: String,
    pub source_url: String,
    pub roster_source_url: String,
    pub identity_note: String,
    pub teams: Vec<AhlTeamRosterStats>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlTeamRosterStats {
    pub provider: String,
    pub provider_team_id: String,
    pub team_code: String,
    pub team_name: String,
    pub nickname: String,
    pub division_id: String,
    pub logo_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nhl_affiliate: Option<String>,
    /// Official season roster. It may be empty before the AHL publishes the
    /// club roster and can include players who appeared earlier in-season.
    pub roster: Vec<AhlRosterPlayer>,
    pub skaters: Vec<AhlSkaterSeasonRow>,
    pub goalies: Vec<AhlGoalieSeasonRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlRosterPlayer {
    pub provider: String,
    pub provider_player_id: String,
    pub name: String,
    pub position_group: String,
    pub position: String,
    pub jersey_number: String,
    pub handedness: String,
    pub height: String,
    pub weight_pounds: String,
    pub birthdate: String,
    pub birthplace: String,
}

/// Explicit bridge from one provider-scoped AHL roster identity to the
/// canonical NHL identity and scenario facts required by the core projection.
/// No AHL id is ever copied into `nhl_player_id` implicitly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlProjectionPlayerEnrichment {
    pub provider_player_id: String,
    pub nhl_player_id: u32,
    pub primary_position: icelines_core::model::Position,
    pub eligible_positions: Vec<icelines_core::model::Position>,
    pub projected_score: f64,
    #[serde(default)]
    pub prospect: bool,
    #[serde(default)]
    pub recall_readiness: Option<f64>,
    #[serde(default)]
    pub professional_games_at_season_start: Option<u32>,
    #[serde(default = "default_true")]
    pub assigned_to_affiliate: bool,
    #[serde(default)]
    pub waiver_required: bool,
}

/// Canonical NHL identity candidates from reviewed NHL roster, draft, or
/// player-profile authorities. This catalog proposes links; it never approves
/// them automatically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlCanonicalIdentityCatalog {
    pub schema: String,
    pub checked_at: String,
    pub candidates: Vec<AhlCanonicalIdentityCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlCanonicalIdentityCandidate {
    pub nhl_player_id: u32,
    pub display_name: String,
    #[serde(default)]
    pub birth_date: Option<String>,
    pub evidence_urls: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityMatchBasis {
    ExactNameAndBirthDate,
    ExactNameOnly,
    BirthDateConflict,
    Ambiguous,
    Unmatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AhlIdentityReviewStatus {
    Pending,
    Reviewed,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityCrosswalkRow {
    pub provider_player_id: String,
    pub ahl_display_name: String,
    pub ahl_birth_date: String,
    pub match_basis: AhlIdentityMatchBasis,
    pub review_status: AhlIdentityReviewStatus,
    #[serde(default)]
    pub nhl_player_id: Option<u32>,
    #[serde(default)]
    pub nhl_display_name: Option<String>,
    #[serde(default)]
    pub nhl_birth_date: Option<String>,
    #[serde(default)]
    pub evidence_urls: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityCrosswalkCounts {
    pub roster_players: usize,
    pub exact_name_and_birth_date: usize,
    pub exact_name_only: usize,
    pub ambiguous: usize,
    pub conflicts: usize,
    pub unmatched: usize,
    pub reviewed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlIdentityCrosswalkView {
    pub schema: String,
    pub season: u32,
    pub provider: String,
    pub ahl_team: String,
    pub nhl_affiliate: Option<String>,
    pub roster_fetched_at: String,
    pub candidates_checked_at: String,
    pub counts: AhlIdentityCrosswalkCounts,
    pub rows: Vec<AhlIdentityCrosswalkRow>,
    pub disclosures: Vec<String>,
}

/// Scenario and player-value facts stay separate from identity review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlProjectionPlayerFacts {
    pub provider_player_id: String,
    pub primary_position: icelines_core::model::Position,
    pub eligible_positions: Vec<icelines_core::model::Position>,
    pub projected_score: f64,
    #[serde(default)]
    pub prospect: bool,
    #[serde(default)]
    pub recall_readiness: Option<f64>,
    #[serde(default)]
    pub professional_games_at_season_start: Option<u32>,
    #[serde(default = "default_true")]
    pub assigned_to_affiliate: bool,
    #[serde(default)]
    pub waiver_required: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AhlSkaterSeasonRow {
    pub provider: String,
    pub provider_player_id: String,
    pub name: String,
    pub team_code: String,
    pub position: String,
    pub active: bool,
    pub rookie: bool,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub plus_minus: i32,
    pub penalty_minutes: u32,
    pub power_play_goals: u32,
    pub short_handed_goals: u32,
    pub shots: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AhlGoalieSeasonRow {
    pub provider: String,
    pub provider_player_id: String,
    pub name: String,
    pub team_code: String,
    pub active: bool,
    pub rookie: bool,
    pub games_played: u32,
    pub minutes_played: String,
    pub wins: u32,
    pub losses: u32,
    pub overtime_losses: u32,
    pub shots_against: u32,
    pub saves: u32,
    pub goals_against: u32,
    pub shutouts: u32,
    pub save_percentage: f64,
    pub goals_against_average: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderSeason {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct SeasonsEnvelope {
    seasons: Vec<ProviderSeason>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderTeam {
    id: String,
    name: String,
    #[serde(default)]
    nickname: String,
    team_code: String,
    #[serde(default)]
    division_id: String,
    #[serde(default)]
    logo: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamsEnvelope {
    teams_no_all: Vec<ProviderTeam>,
}

/// Client for the feed behind the official AHL statistics pages.
#[derive(Debug, Clone)]
pub struct AhlFeedClient {
    client: reqwest::Client,
    base_url: String,
    key: String,
    client_code: String,
    cache: Option<(std::path::PathBuf, bool)>,
}

impl AhlFeedClient {
    pub fn production() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("IceLines/ahl-roster-stats")
            .build()
            .expect("valid AHL HTTP client configuration");
        Self {
            client,
            base_url: AHL_FEED_BASE_URL.to_owned(),
            key: AHL_FEED_KEY.to_owned(),
            client_code: AHL_CLIENT_CODE.to_owned(),
            cache: None,
        }
    }

    /// Production client whose source bytes are acquired through FLETCH's
    /// verified cacheline and shared cache manifest before IceLines parses.
    pub fn production_cached(cache_root: impl Into<std::path::PathBuf>, force: bool) -> Self {
        let mut client = Self::production();
        client.cache = Some((cache_root.into(), force));
        client
    }

    #[cfg(test)]
    fn with_base_url(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            key: AHL_FEED_KEY.to_owned(),
            client_code: AHL_CLIENT_CODE.to_owned(),
            cache: None,
        }
    }

    #[cfg(test)]
    fn with_base_url_and_cache(base_url: String, cache_root: std::path::PathBuf) -> Self {
        let mut client = Self::with_base_url(base_url);
        client.cache = Some((cache_root, false));
        client
    }

    /// Fetch one league snapshot. `team_filters` accepts AHL codes or exact
    /// AHL team names; an empty slice means every team in the provider catalog.
    pub async fn fetch_roster_stats(
        &self,
        season: u32,
        team_filters: &[String],
    ) -> Result<AhlRosterStatsSnapshot, AhlFeedError> {
        let provider_season = self.resolve_regular_season(season).await?;
        let mut teams = self.fetch_teams(season, &provider_season.id).await?;
        teams.sort_by(|a, b| a.name.cmp(&b.name));
        teams = filter_teams(teams, team_filters)?;

        let affiliate_by_name = current_affiliates_for(season);
        let mut output = Vec::with_capacity(teams.len());
        for team in teams {
            let mut roster = self
                .fetch_roster(season, &provider_season.id, &team)
                .await?;
            let mut skaters = self
                .fetch_skaters(season, &provider_season.id, &team)
                .await?;
            let mut goalies = self
                .fetch_goalies(season, &provider_season.id, &team)
                .await?;
            roster.sort_by(|a, b| {
                a.position_group
                    .cmp(&b.position_group)
                    .then(a.name.cmp(&b.name))
                    .then(a.provider_player_id.cmp(&b.provider_player_id))
            });
            skaters.sort_by(|a, b| {
                a.name
                    .cmp(&b.name)
                    .then(a.provider_player_id.cmp(&b.provider_player_id))
            });
            goalies.sort_by(|a, b| {
                a.name
                    .cmp(&b.name)
                    .then(a.provider_player_id.cmp(&b.provider_player_id))
            });
            output.push(AhlTeamRosterStats {
                provider: AHL_PROVIDER.to_owned(),
                provider_team_id: team.id,
                team_code: team.team_code,
                nhl_affiliate: affiliate_by_name.get(&team.name).cloned(),
                team_name: team.name,
                nickname: team.nickname,
                division_id: team.division_id,
                logo_url: team.logo,
                roster,
                skaters,
                goalies,
            });
        }

        let snapshot = AhlRosterStatsSnapshot {
            schema: AHL_ROSTER_STATS_SCHEMA.to_owned(),
            season,
            provider: AHL_PROVIDER.to_owned(),
            provider_season_id: provider_season.id,
            provider_season_name: provider_season.name,
            fetched_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            source_url: AHL_STATS_SOURCE_URL.to_owned(),
            roster_source_url: AHL_ROSTER_SOURCE_URL.to_owned(),
            identity_note: "provider_player_id is an AHL HockeyTech identifier, not an NHL player_id; link only through an explicit crosswalk".to_owned(),
            teams: output,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    async fn resolve_regular_season(&self, season: u32) -> Result<ProviderSeason, AhlFeedError> {
        let target = season_label(season)?;
        let dataset_id = format!("icelines.ahl.{season}.catalog.seasons");
        let value = self
            .get_feed(
                &dataset_id,
                &[("view", "seasonsForLeague"), ("league", "4")],
            )
            .await?;
        let envelope: SeasonsEnvelope = serde_json::from_value(value)
            .map_err(|e| AhlFeedError::Schema(format!("season catalog: {e}")))?;
        envelope
            .seasons
            .into_iter()
            .find(|row| row.name == target)
            .ok_or(AhlFeedError::SeasonNotFound(target))
    }

    async fn fetch_teams(
        &self,
        season: u32,
        provider_season_id: &str,
    ) -> Result<Vec<ProviderTeam>, AhlFeedError> {
        let dataset_id = format!("icelines.ahl.{season}.catalog.teams");
        let value = self
            .get_feed(
                &dataset_id,
                &[("view", "teamsForSeason"), ("season", provider_season_id)],
            )
            .await?;
        let envelope: TeamsEnvelope = serde_json::from_value(value)
            .map_err(|e| AhlFeedError::Schema(format!("team catalog: {e}")))?;
        if envelope.teams_no_all.is_empty() {
            return Err(AhlFeedError::Schema("team catalog was empty".to_owned()));
        }
        Ok(envelope.teams_no_all)
    }

    async fn fetch_skaters(
        &self,
        season: u32,
        provider_season_id: &str,
        team: &ProviderTeam,
    ) -> Result<Vec<AhlSkaterSeasonRow>, AhlFeedError> {
        let value = self
            .fetch_player_report(
                &format!("icelines.ahl.{season}.team.{}.skaters", team.team_code),
                provider_season_id,
                &team.id,
                "skaters",
                "points",
            )
            .await?;
        report_rows(&value)?
            .into_iter()
            .map(|row| parse_skater(row, &team.team_code))
            .collect()
    }

    async fn fetch_roster(
        &self,
        season: u32,
        provider_season_id: &str,
        team: &ProviderTeam,
    ) -> Result<Vec<AhlRosterPlayer>, AhlFeedError> {
        let dataset_id = format!("icelines.ahl.{season}.team.{}.roster", team.team_code);
        let value = self
            .get_feed(
                &dataset_id,
                &[
                    ("view", "roster"),
                    ("team_id", &team.id),
                    ("season_id", provider_season_id),
                    ("rosterstatus", "all"),
                    ("site_id", "0"),
                    ("league_id", "4"),
                    ("lang", "en"),
                ],
            )
            .await?;
        roster_rows(&value)?
            .into_iter()
            .map(|(group, row)| parse_roster_player(group, row))
            .collect()
    }

    async fn fetch_goalies(
        &self,
        season: u32,
        provider_season_id: &str,
        team: &ProviderTeam,
    ) -> Result<Vec<AhlGoalieSeasonRow>, AhlFeedError> {
        let value = self
            .fetch_player_report(
                &format!("icelines.ahl.{season}.team.{}.goalies", team.team_code),
                provider_season_id,
                &team.id,
                "goalies",
                "wins",
            )
            .await?;
        report_rows(&value)?
            .into_iter()
            .map(|row| parse_goalie(row, &team.team_code))
            .collect()
    }

    async fn fetch_player_report(
        &self,
        dataset_id: &str,
        season: &str,
        team: &str,
        position: &str,
        sort: &str,
    ) -> Result<Value, AhlFeedError> {
        self.get_feed(
            dataset_id,
            &[
                ("view", "players"),
                ("season", season),
                ("team", team),
                ("position", position),
                ("rookies", "0"),
                ("statsType", "standard"),
                ("rosterstatus", "all"),
                ("first", "0"),
                ("limit", "500"),
                ("lang", "en"),
                ("sort", sort),
            ],
        )
        .await
    }

    async fn get_feed(
        &self,
        dataset_id: &str,
        params: &[(&str, &str)],
    ) -> Result<Value, AhlFeedError> {
        let mut query = vec![
            ("feed", "statviewfeed"),
            ("key", self.key.as_str()),
            ("client_code", self.client_code.as_str()),
        ];
        query.extend_from_slice(params);
        let request = self.client.get(&self.base_url).query(&query);
        let url = request
            .try_clone()
            .and_then(|r| r.build().ok())
            .map(|r| r.url().to_string())
            .unwrap_or_else(|| self.base_url.clone());
        if let Some((cache_root, force)) = &self.cache {
            let bytes = crate::fletch::fetch_generic_http_bytes_async(
                dataset_id.to_owned(),
                url.clone(),
                cache_root.clone(),
                *force,
            )
            .await
            .map_err(|e| AhlFeedError::Request {
                url,
                detail: format!("FLETCH cache acquisition failed: {e:#}"),
            })?;
            let body = std::str::from_utf8(&bytes).map_err(|e| {
                AhlFeedError::Schema(format!("{dataset_id} returned non-UTF-8 bytes: {e}"))
            })?;
            return parse_jsonp(body);
        }
        let response = request.send().await.map_err(|e| AhlFeedError::Request {
            url: url.clone(),
            detail: e.to_string(),
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(AhlFeedError::Http {
                status: status.as_u16(),
                url,
            });
        }
        let body = response.text().await.map_err(|e| AhlFeedError::Request {
            url,
            detail: e.to_string(),
        })?;
        parse_jsonp(&body)
    }
}

impl AhlRosterStatsSnapshot {
    pub fn validate(&self) -> Result<(), AhlFeedError> {
        if self.schema != AHL_ROSTER_STATS_SCHEMA {
            return Err(AhlFeedError::Validation(format!(
                "unexpected schema {}",
                self.schema
            )));
        }
        let mut team_ids = BTreeSet::new();
        let mut team_codes = BTreeSet::new();
        for team in &self.teams {
            if !team_ids.insert(team.provider_team_id.as_str()) {
                return Err(AhlFeedError::Validation(format!(
                    "duplicate provider team id {}",
                    team.provider_team_id
                )));
            }
            if !team_codes.insert(team.team_code.as_str()) {
                return Err(AhlFeedError::Validation(format!(
                    "duplicate AHL team code {}",
                    team.team_code
                )));
            }
            validate_player_ids(
                team,
                &team
                    .skaters
                    .iter()
                    .map(|p| (p.provider_player_id.as_str(), p.team_code.as_str()))
                    .collect::<Vec<_>>(),
            )?;
            validate_player_ids(
                team,
                &team
                    .goalies
                    .iter()
                    .map(|p| (p.provider_player_id.as_str(), p.team_code.as_str()))
                    .collect::<Vec<_>>(),
            )?;
            let mut roster_ids = BTreeSet::new();
            for player in &team.roster {
                if !roster_ids.insert(player.provider_player_id.as_str()) {
                    return Err(AhlFeedError::Validation(format!(
                        "duplicate roster provider player id {} on {}",
                        player.provider_player_id, team.team_code
                    )));
                }
            }
            for player in &team.skaters {
                if player.goals + player.assists != player.points {
                    return Err(AhlFeedError::Validation(format!(
                        "{} points do not equal goals plus assists",
                        player.name
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Adapt one official AHL team roster into the core affiliate projection
/// contract. Every roster player must have an explicit provider→NHL crosswalk
/// plus the scenario facts that the official feed does not establish.
pub fn affiliate_projection_input_from_snapshot(
    snapshot: &AhlRosterStatsSnapshot,
    nhl_team: &str,
    ahl_team: &str,
    rule: icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput,
    enrichments: &[AhlProjectionPlayerEnrichment],
) -> Result<icelines_core::view_model::ahl_affiliate::AhlAffiliateProjectionInput, AhlFeedError> {
    use icelines_core::view_model::ahl_affiliate::{
        AhlAffiliatePlayerInput, AhlAffiliateProjectionInput,
    };

    snapshot.validate()?;
    let team = snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("AHL snapshot has no team named `{ahl_team}`"))
        })?;
    if team
        .nhl_affiliate
        .as_deref()
        .is_some_and(|affiliate| affiliate != nhl_team)
    {
        return Err(AhlFeedError::Validation(format!(
            "{} snapshot affiliate is {}, not {nhl_team}",
            team.team_name,
            team.nhl_affiliate.as_deref().unwrap_or_default()
        )));
    }

    let mut by_provider_id = BTreeMap::new();
    let mut nhl_ids = BTreeSet::new();
    for enrichment in enrichments {
        if enrichment.provider_player_id.trim().is_empty()
            || enrichment.nhl_player_id == 0
            || !enrichment.projected_score.is_finite()
            || enrichment.recall_readiness.is_some_and(|readiness| {
                !readiness.is_finite() || !(0.0..=1.0).contains(&readiness)
            })
            || !enrichment
                .eligible_positions
                .contains(&enrichment.primary_position)
            || (enrichment.assigned_to_affiliate
                && enrichment.primary_position != icelines_core::model::Position::Goalie
                && enrichment.professional_games_at_season_start.is_none())
            || by_provider_id
                .insert(enrichment.provider_player_id.as_str(), enrichment)
                .is_some()
            || !nhl_ids.insert(enrichment.nhl_player_id)
        {
            return Err(AhlFeedError::Validation(
                "AHL projection crosswalk contains blank or duplicate identities".to_owned(),
            ));
        }
    }

    let official_ids = team
        .roster
        .iter()
        .map(|player| player.provider_player_id.as_str())
        .collect::<BTreeSet<_>>();
    let missing = official_ids
        .iter()
        .filter(|id| !by_provider_id.contains_key(**id))
        .copied()
        .collect::<Vec<_>>();
    let extra = by_provider_id
        .keys()
        .filter(|id| !official_ids.contains(**id))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(AhlFeedError::Validation(format!(
            "AHL projection crosswalk must exactly cover the official roster; missing=[{}], extra=[{}]",
            missing.join(","),
            extra.join(",")
        )));
    }

    let players = team
        .roster
        .iter()
        .map(|official| {
            let enrichment = by_provider_id[official.provider_player_id.as_str()];
            AhlAffiliatePlayerInput {
                player_id: enrichment.nhl_player_id,
                display_name: official.name.clone(),
                primary_position: enrichment.primary_position,
                eligible_positions: enrichment.eligible_positions.clone(),
                projected_score: enrichment.projected_score,
                prospect: enrichment.prospect,
                recall_readiness: enrichment.recall_readiness,
                professional_games_at_season_start: enrichment.professional_games_at_season_start,
                assigned_to_affiliate: enrichment.assigned_to_affiliate,
                waiver_required: enrichment.waiver_required,
                source_league: "AHL".to_owned(),
            }
        })
        .collect();

    let input = AhlAffiliateProjectionInput {
        nhl_team: nhl_team.to_owned(),
        ahl_team: team.team_name.clone(),
        season: snapshot.season,
        rule,
        players,
    };
    Ok(input)
}

/// Build a deterministic review queue. Exact official name and birth-date
/// agreement is a high-confidence proposal, but remains pending until a human
/// changes `review_status` to `reviewed`.
pub fn build_ahl_identity_crosswalk(
    snapshot: &AhlRosterStatsSnapshot,
    ahl_team: &str,
    catalog: &AhlCanonicalIdentityCatalog,
) -> Result<AhlIdentityCrosswalkView, AhlFeedError> {
    snapshot.validate()?;
    validate_identity_catalog(catalog)?;
    let team = snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("AHL snapshot has no team named `{ahl_team}`"))
        })?;
    let mut by_name = BTreeMap::<String, Vec<&AhlCanonicalIdentityCandidate>>::new();
    for candidate in &catalog.candidates {
        by_name
            .entry(icelines_core::normalize_name(&candidate.display_name))
            .or_default()
            .push(candidate);
    }

    let mut rows = team
        .roster
        .iter()
        .map(|official| {
            let candidates = by_name
                .get(&icelines_core::normalize_name(&official.name))
                .map(Vec::as_slice)
                .unwrap_or_default();
            identity_crosswalk_row(official, candidates)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        a.ahl_display_name
            .cmp(&b.ahl_display_name)
            .then_with(|| a.provider_player_id.cmp(&b.provider_player_id))
    });
    let counts = identity_crosswalk_counts(&rows);
    Ok(AhlIdentityCrosswalkView {
        schema: AHL_IDENTITY_CROSSWALK_SCHEMA.to_owned(),
        season: snapshot.season,
        provider: snapshot.provider.clone(),
        ahl_team: team.team_name.clone(),
        nhl_affiliate: team.nhl_affiliate.clone(),
        roster_fetched_at: snapshot.fetched_at.clone(),
        candidates_checked_at: catalog.checked_at.clone(),
        counts,
        rows,
        disclosures: vec![
            "AHL provider_player_id values remain provider-local and are never copied into NHL player IDs.".to_owned(),
            "Even exact normalized-name and birth-date matches are proposals until review_status is explicitly changed to reviewed.".to_owned(),
            "Identity approval does not establish roster assignment, prospect status, professional-game totals, waivers, player value, or recall readiness.".to_owned(),
        ],
    })
}

/// Join a fully reviewed identity artifact to separately authored projection
/// facts and feed the existing exact-coverage snapshot adapter.
pub fn affiliate_projection_input_from_reviewed_crosswalk(
    snapshot: &AhlRosterStatsSnapshot,
    nhl_team: &str,
    ahl_team: &str,
    rule: icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput,
    crosswalk: &AhlIdentityCrosswalkView,
    facts: &[AhlProjectionPlayerFacts],
) -> Result<icelines_core::view_model::ahl_affiliate::AhlAffiliateProjectionInput, AhlFeedError> {
    validate_reviewed_ahl_identity_crosswalk(snapshot, ahl_team, crosswalk)?;
    let identities = crosswalk
        .rows
        .iter()
        .map(|row| {
            (
                row.provider_player_id.as_str(),
                row.nhl_player_id
                    .expect("review validation requires NHL id"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let enrichments = facts
        .iter()
        .map(|fact| {
            let nhl_player_id = identities
                .get(fact.provider_player_id.as_str())
                .copied()
                .ok_or_else(|| {
                    AhlFeedError::Validation(format!(
                        "projection facts reference provider player {} absent from reviewed crosswalk",
                        fact.provider_player_id
                    ))
                })?;
            Ok(AhlProjectionPlayerEnrichment {
                provider_player_id: fact.provider_player_id.clone(),
                nhl_player_id,
                primary_position: fact.primary_position,
                eligible_positions: fact.eligible_positions.clone(),
                projected_score: fact.projected_score,
                prospect: fact.prospect,
                recall_readiness: fact.recall_readiness,
                professional_games_at_season_start: fact.professional_games_at_season_start,
                assigned_to_affiliate: fact.assigned_to_affiliate,
                waiver_required: fact.waiver_required,
            })
        })
        .collect::<Result<Vec<_>, AhlFeedError>>()?;
    affiliate_projection_input_from_snapshot(snapshot, nhl_team, ahl_team, rule, &enrichments)
}

pub fn validate_reviewed_ahl_identity_crosswalk(
    snapshot: &AhlRosterStatsSnapshot,
    ahl_team: &str,
    crosswalk: &AhlIdentityCrosswalkView,
) -> Result<(), AhlFeedError> {
    snapshot.validate()?;
    if crosswalk.schema != AHL_IDENTITY_CROSSWALK_SCHEMA
        || crosswalk.season != snapshot.season
        || crosswalk.provider != snapshot.provider
        || crosswalk.ahl_team != ahl_team
        || crosswalk.roster_fetched_at != snapshot.fetched_at
        || crosswalk.candidates_checked_at.trim().is_empty()
    {
        return Err(AhlFeedError::Validation(
            "identity crosswalk does not match the selected AHL snapshot/team authority".to_owned(),
        ));
    }
    let team = snapshot
        .teams
        .iter()
        .find(|team| team.team_name == ahl_team)
        .ok_or_else(|| {
            AhlFeedError::Validation(format!("AHL snapshot has no team named `{ahl_team}`"))
        })?;
    if team.roster.is_empty() {
        return Err(AhlFeedError::Validation(format!(
            "official AHL roster for {ahl_team} is empty and cannot establish projection identity coverage"
        )));
    }
    if crosswalk.nhl_affiliate != team.nhl_affiliate {
        return Err(AhlFeedError::Validation(
            "identity crosswalk NHL affiliate differs from the snapshot".to_owned(),
        ));
    }
    let official = team
        .roster
        .iter()
        .map(|row| (row.provider_player_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut provider_ids = BTreeSet::new();
    let mut nhl_ids = BTreeSet::new();
    for row in &crosswalk.rows {
        let source = official
            .get(row.provider_player_id.as_str())
            .ok_or_else(|| {
                AhlFeedError::Validation(format!(
                    "identity crosswalk contains extra provider player {}",
                    row.provider_player_id
                ))
            })?;
        if !provider_ids.insert(row.provider_player_id.as_str())
            || row.ahl_display_name != source.name
            || row.ahl_birth_date != source.birthdate
        {
            return Err(AhlFeedError::Validation(format!(
                "identity crosswalk altered or duplicated official AHL identity {}",
                row.provider_player_id
            )));
        }
        if row.review_status != AhlIdentityReviewStatus::Reviewed {
            return Err(AhlFeedError::Validation(format!(
                "identity {} is not reviewed",
                row.provider_player_id
            )));
        }
        let nhl_id = row.nhl_player_id.filter(|id| *id != 0).ok_or_else(|| {
            AhlFeedError::Validation(format!(
                "reviewed identity {} has no NHL player ID",
                row.provider_player_id
            ))
        })?;
        if !nhl_ids.insert(nhl_id)
            || row
                .nhl_display_name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty())
            || row.evidence_urls.is_empty()
            || row.evidence_urls.iter().any(|url| !absolute_http_url(url))
            || row
                .nhl_birth_date
                .as_deref()
                .is_some_and(|date| !source.birthdate.is_empty() && date != source.birthdate)
        {
            return Err(AhlFeedError::Validation(format!(
                "reviewed identity {} has invalid or conflicting NHL evidence",
                row.provider_player_id
            )));
        }
    }
    if provider_ids.len() != official.len() {
        let missing = official
            .keys()
            .filter(|id| !provider_ids.contains(**id))
            .copied()
            .collect::<Vec<_>>();
        return Err(AhlFeedError::Validation(format!(
            "identity crosswalk is missing provider players [{}]",
            missing.join(",")
        )));
    }
    Ok(())
}

fn validate_identity_catalog(catalog: &AhlCanonicalIdentityCatalog) -> Result<(), AhlFeedError> {
    if catalog.schema != AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA
        || catalog.checked_at.trim().is_empty()
    {
        return Err(AhlFeedError::Validation(
            "invalid canonical NHL identity catalog authority".to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    for candidate in &catalog.candidates {
        if candidate.nhl_player_id == 0
            || !ids.insert(candidate.nhl_player_id)
            || icelines_core::normalize_name(&candidate.display_name).is_empty()
            || candidate.evidence_urls.is_empty()
            || candidate
                .evidence_urls
                .iter()
                .any(|url| !absolute_http_url(url))
            || candidate
                .birth_date
                .as_deref()
                .is_some_and(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err())
        {
            return Err(AhlFeedError::Validation(
                "canonical NHL identity catalog contains invalid or duplicate candidates"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

fn identity_crosswalk_row(
    official: &AhlRosterPlayer,
    candidates: &[&AhlCanonicalIdentityCandidate],
) -> AhlIdentityCrosswalkRow {
    let birth_matches = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            candidate
                .birth_date
                .as_deref()
                .is_some_and(|date| !official.birthdate.is_empty() && date == official.birthdate)
        })
        .collect::<Vec<_>>();
    let (basis, candidate, note) = if birth_matches.len() == 1 {
        (
            AhlIdentityMatchBasis::ExactNameAndBirthDate,
            Some(birth_matches[0]),
            "Exact normalized name and birth date; human review still required.",
        )
    } else if candidates.len() == 1 {
        let candidate = candidates[0];
        if candidate.birth_date.is_some()
            && !official.birthdate.is_empty()
            && candidate.birth_date.as_deref() != Some(official.birthdate.as_str())
        {
            (
                AhlIdentityMatchBasis::BirthDateConflict,
                None,
                "Exact normalized name but conflicting birth date.",
            )
        } else {
            (
                AhlIdentityMatchBasis::ExactNameOnly,
                Some(candidate),
                "Exact normalized name with incomplete birth-date corroboration; human review required.",
            )
        }
    } else if candidates.is_empty() {
        (
            AhlIdentityMatchBasis::Unmatched,
            None,
            "No exact normalized-name candidate.",
        )
    } else {
        (
            AhlIdentityMatchBasis::Ambiguous,
            None,
            "Multiple exact normalized-name candidates remain unresolved.",
        )
    };
    AhlIdentityCrosswalkRow {
        provider_player_id: official.provider_player_id.clone(),
        ahl_display_name: official.name.clone(),
        ahl_birth_date: official.birthdate.clone(),
        match_basis: basis,
        review_status: AhlIdentityReviewStatus::Pending,
        nhl_player_id: candidate.map(|candidate| candidate.nhl_player_id),
        nhl_display_name: candidate.map(|candidate| candidate.display_name.clone()),
        nhl_birth_date: candidate.and_then(|candidate| candidate.birth_date.clone()),
        evidence_urls: candidate
            .map(|candidate| candidate.evidence_urls.clone())
            .unwrap_or_default(),
        note: note.to_owned(),
    }
}

fn identity_crosswalk_counts(rows: &[AhlIdentityCrosswalkRow]) -> AhlIdentityCrosswalkCounts {
    AhlIdentityCrosswalkCounts {
        roster_players: rows.len(),
        exact_name_and_birth_date: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::ExactNameAndBirthDate)
            .count(),
        exact_name_only: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::ExactNameOnly)
            .count(),
        ambiguous: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::Ambiguous)
            .count(),
        conflicts: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::BirthDateConflict)
            .count(),
        unmatched: rows
            .iter()
            .filter(|row| row.match_basis == AhlIdentityMatchBasis::Unmatched)
            .count(),
        reviewed: rows
            .iter()
            .filter(|row| row.review_status == AhlIdentityReviewStatus::Reviewed)
            .count(),
    }
}

fn absolute_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn validate_player_ids(
    team: &AhlTeamRosterStats,
    players: &[(&str, &str)],
) -> Result<(), AhlFeedError> {
    let mut ids = BTreeSet::new();
    for (id, code) in players {
        if !ids.insert(*id) {
            return Err(AhlFeedError::Validation(format!(
                "duplicate provider player id {id} on {}",
                team.team_code
            )));
        }
        if *code != team.team_code {
            return Err(AhlFeedError::Validation(format!(
                "player team code {code} does not match {}",
                team.team_code
            )));
        }
    }
    Ok(())
}

fn filter_teams(
    teams: Vec<ProviderTeam>,
    filters: &[String],
) -> Result<Vec<ProviderTeam>, AhlFeedError> {
    if filters.is_empty() {
        return Ok(teams);
    }
    let wanted: BTreeSet<String> = filters
        .iter()
        .map(|s| s.trim().to_ascii_uppercase())
        .collect();
    let selected: Vec<_> = teams
        .into_iter()
        .filter(|team| {
            wanted.contains(&team.team_code.to_ascii_uppercase())
                || wanted.contains(&team.name.to_ascii_uppercase())
        })
        .collect();
    let found: BTreeSet<String> = selected
        .iter()
        .flat_map(|team| {
            [
                team.team_code.to_ascii_uppercase(),
                team.name.to_ascii_uppercase(),
            ]
        })
        .collect();
    let unknown: Vec<_> = wanted.difference(&found).cloned().collect();
    if !unknown.is_empty() {
        return Err(AhlFeedError::UnknownTeams(unknown.join(", ")));
    }
    Ok(selected)
}

fn season_label(season: u32) -> Result<String, AhlFeedError> {
    let text = format!("{season:08}");
    let start: u32 = text[..4]
        .parse()
        .map_err(|_| AhlFeedError::SeasonNotFound(text.clone()))?;
    let end: u32 = text[4..]
        .parse()
        .map_err(|_| AhlFeedError::SeasonNotFound(text.clone()))?;
    if end != start + 1 {
        return Err(AhlFeedError::SeasonNotFound(text));
    }
    Ok(format!("{start}-{:02} Regular Season", end % 100))
}

fn current_affiliates_for(season: u32) -> BTreeMap<String, String> {
    if season != icelines_core::view_model::ahl_affiliate::CURRENT_AHL_AFFILIATION_SEASON {
        return BTreeMap::new();
    }
    icelines_core::view_model::ahl_affiliate::current_ahl_affiliation_catalog()
        .affiliations
        .into_iter()
        .map(|row| (row.ahl_team, row.nhl_team))
        .collect()
}

/// Parse the JSONP wrappers used by Statview (`({...})` and `([...])`).
pub fn parse_jsonp(body: &str) -> Result<Value, AhlFeedError> {
    let trimmed = body.trim();
    let json = trimmed
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| AhlFeedError::Schema("expected parenthesized JSONP body".to_owned()))?;
    serde_json::from_str(json)
        .map_err(|e| AhlFeedError::Schema(format!("invalid JSONP payload: {e}")))
}

fn report_rows(value: &Value) -> Result<Vec<&Value>, AhlFeedError> {
    let reports = value
        .as_array()
        .ok_or_else(|| AhlFeedError::Schema("player report root was not an array".to_owned()))?;
    let mut rows = Vec::new();
    for report in reports {
        let sections = report
            .get("sections")
            .and_then(Value::as_array)
            .ok_or_else(|| AhlFeedError::Schema("player report sections missing".to_owned()))?;
        for section in sections {
            let data = section
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AhlFeedError::Schema("player report data missing".to_owned()))?;
            for item in data {
                let row = item
                    .get("row")
                    .ok_or_else(|| AhlFeedError::Schema("player report row missing".to_owned()))?;
                if row.get("player_id").is_some() {
                    rows.push(row);
                    continue;
                }
                let label = row
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim();
                if !matches!(label, "Empty Net" | "Totals") {
                    return Err(AhlFeedError::Schema(format!(
                        "player report row `{label}` had no player_id"
                    )));
                }
            }
        }
    }
    Ok(rows)
}

fn roster_rows(value: &Value) -> Result<Vec<(&str, &Value)>, AhlFeedError> {
    let reports = value
        .get("roster")
        .and_then(Value::as_array)
        .ok_or_else(|| AhlFeedError::Schema("roster report missing".to_owned()))?;
    let mut rows = Vec::new();
    for report in reports {
        let sections = report
            .get("sections")
            .and_then(Value::as_array)
            .ok_or_else(|| AhlFeedError::Schema("roster sections missing".to_owned()))?;
        for section in sections {
            let title = section
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| AhlFeedError::Schema("roster section title missing".to_owned()))?;
            if title == "Team Personnel" {
                continue;
            }
            let data = section
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AhlFeedError::Schema("roster section data missing".to_owned()))?;
            for item in data {
                let row = item
                    .get("row")
                    .ok_or_else(|| AhlFeedError::Schema("roster player row missing".to_owned()))?;
                rows.push((title, row));
            }
        }
    }
    Ok(rows)
}

fn parse_roster_player(group: &str, row: &Value) -> Result<AhlRosterPlayer, AhlFeedError> {
    let mut handedness = optional_string_field(row, "shoots");
    if handedness.is_empty() {
        handedness = optional_string_field(row, "catches");
    }
    Ok(AhlRosterPlayer {
        provider: AHL_PROVIDER.to_owned(),
        provider_player_id: string_field(row, "player_id")?,
        name: string_field(row, "name")?,
        position_group: group.to_owned(),
        position: string_field(row, "position")?,
        jersey_number: optional_string_field(row, "tp_jersey_number"),
        handedness,
        height: optional_string_field(row, "height_hyphenated"),
        weight_pounds: optional_string_field(row, "w"),
        birthdate: optional_string_field(row, "birthdate"),
        birthplace: optional_string_field(row, "birthplace"),
    })
}

fn parse_skater(row: &Value, expected_team: &str) -> Result<AhlSkaterSeasonRow, AhlFeedError> {
    Ok(AhlSkaterSeasonRow {
        provider: AHL_PROVIDER.to_owned(),
        provider_player_id: string_field(row, "player_id")?,
        name: string_field(row, "name")?,
        team_code: checked_team_code(row, expected_team)?,
        position: string_field(row, "position")?,
        active: bool_field(row, "active")?,
        rookie: bool_field(row, "rookie")?,
        games_played: u32_field(row, "games_played")?,
        goals: u32_field(row, "goals")?,
        assists: u32_field(row, "assists")?,
        points: u32_field(row, "points")?,
        plus_minus: i32_field(row, "plus_minus")?,
        penalty_minutes: u32_field(row, "penalty_minutes")?,
        power_play_goals: u32_field(row, "power_play_goals")?,
        short_handed_goals: u32_field(row, "short_handed_goals")?,
        shots: u32_field(row, "shots")?,
    })
}

fn parse_goalie(row: &Value, expected_team: &str) -> Result<AhlGoalieSeasonRow, AhlFeedError> {
    Ok(AhlGoalieSeasonRow {
        provider: AHL_PROVIDER.to_owned(),
        provider_player_id: string_field(row, "player_id")?,
        name: string_field(row, "name")?,
        team_code: checked_team_code(row, expected_team)?,
        active: bool_field(row, "active")?,
        rookie: bool_field(row, "rookie")?,
        games_played: u32_field(row, "games_played")?,
        minutes_played: string_field(row, "minutes_played")?,
        wins: u32_field(row, "wins")?,
        losses: u32_field(row, "losses")?,
        overtime_losses: u32_field(row, "ot_losses")?,
        shots_against: u32_field(row, "shots")?,
        saves: u32_field(row, "saves")?,
        goals_against: u32_field(row, "goals_against")?,
        shutouts: u32_field(row, "shutouts")?,
        save_percentage: f64_field(row, "save_percentage")?,
        goals_against_average: f64_field(row, "goals_against_average")?,
    })
}

fn checked_team_code(row: &Value, expected: &str) -> Result<String, AhlFeedError> {
    let actual = string_field(row, "team_code")?;
    if actual != expected {
        return Err(AhlFeedError::Validation(format!(
            "feed returned team code {actual} while fetching {expected}"
        )));
    }
    Ok(actual)
}

fn string_field(row: &Value, field: &str) -> Result<String, AhlFeedError> {
    row.get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AhlFeedError::Schema(format!("missing string field `{field}`")))
}

fn optional_string_field(row: &Value, field: &str) -> String {
    row.get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn bool_field(row: &Value, field: &str) -> Result<bool, AhlFeedError> {
    match string_field(row, field)?.as_str() {
        "1" => Ok(true),
        "0" => Ok(false),
        value => Err(AhlFeedError::Schema(format!(
            "invalid boolean `{field}` value {value}"
        ))),
    }
}

fn u32_field(row: &Value, field: &str) -> Result<u32, AhlFeedError> {
    string_field(row, field)?
        .parse()
        .map_err(|e| AhlFeedError::Schema(format!("invalid integer `{field}`: {e}")))
}

fn i32_field(row: &Value, field: &str) -> Result<i32, AhlFeedError> {
    string_field(row, field)?
        .parse()
        .map_err(|e| AhlFeedError::Schema(format!("invalid integer `{field}`: {e}")))
}

fn f64_field(row: &Value, field: &str) -> Result<f64, AhlFeedError> {
    string_field(row, field)?
        .parse()
        .map_err(|e| AhlFeedError::Schema(format!("invalid decimal `{field}`: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    const SKATER_REPORT: &str = r#"([{"sections":[{"data":[{"row":{"player_id":"7669","name":"Trey Fix-Wolansky","active":"1","position":"F","rookie":"0","team_code":"HFD","games_played":"72","goals":"31","assists":"24","points":"55","plus_minus":"-9","penalty_minutes":"121","power_play_goals":"7","short_handed_goals":"0","shots":"214"}}]}]}])"#;
    const GOALIE_REPORT: &str = r#"([{"sections":[{"data":[{"row":{"player_id":"8430","name":"Dylan Garand","active":"0","rookie":"0","team_code":"HFD","games_played":"36","minutes_played":"2013:22","saves":"821","shots":"916","save_percentage":"0.896","goals_against":"95","shutouts":"1","wins":"16","losses":"15","ot_losses":"2","goals_against_average":"2.83"}}]}]}])"#;
    const ROSTER_REPORT: &str = r#"({"roster":[{"sections":[{"title":"Forwards","data":[{"row":{"shoots":"L","birthplace":"Fort Collins, CO","height_hyphenated":"5-11","player_id":"10618","birthdate":"2002-02-18","tp_jersey_number":"6","position":"F","w":"180","name":"Aidan Thompson"}}]},{"title":"Team Personnel","data":[{"row":{"name":"Ryan Martin","role":"General Manager"}}]}]}]})"#;

    #[test]
    fn parses_both_statview_jsonp_shapes() {
        assert!(parse_jsonp("({\"teamsNoAll\":[]})").unwrap().is_object());
        assert!(parse_jsonp(SKATER_REPORT).unwrap().is_array());
        assert!(parse_jsonp("{\"no\":\"wrapper\"}").is_err());
    }

    #[test]
    fn ignores_documented_goalie_summary_rows_but_not_unknown_malformed_rows() {
        let summary = parse_jsonp(
            r#"([{"sections":[{"data":[{"row":{"name":"Empty Net "}},{"row":{"name":"Totals "}}]}]}])"#,
        )
        .unwrap();
        assert!(report_rows(&summary).unwrap().is_empty());
        let malformed =
            parse_jsonp(r#"([{"sections":[{"data":[{"row":{"name":"Mystery Player"}}]}]}])"#)
                .unwrap();
        assert!(report_rows(&malformed).is_err());
    }

    #[test]
    fn parses_provider_ids_without_claiming_nhl_identity() {
        let value = parse_jsonp(SKATER_REPORT).unwrap();
        let player = parse_skater(report_rows(&value).unwrap()[0], "HFD").unwrap();
        assert_eq!(player.provider, AHL_PROVIDER);
        assert_eq!(player.provider_player_id, "7669");
        assert_eq!(player.points, 55);
        assert_eq!(player.plus_minus, -9);
    }

    #[test]
    fn rejects_team_identity_mismatch() {
        let value = parse_jsonp(SKATER_REPORT).unwrap();
        let error = parse_skater(report_rows(&value).unwrap()[0], "CV").unwrap_err();
        assert!(error.to_string().contains("while fetching CV"));
    }

    #[test]
    fn season_labels_are_annual_and_validated() {
        assert_eq!(season_label(20262027).unwrap(), "2026-27 Regular Season");
        assert!(season_label(20262028).is_err());
    }

    #[tokio::test]
    async fn fetches_catalog_and_both_player_shapes() {
        let server = MockServer::start();
        let cache = tempfile::tempdir().unwrap();
        let seasons = server.mock(|when, then| {
            when.method(GET).query_param("view", "seasonsForLeague");
            then.status(200)
                .body("({\"seasons\":[{\"id\":\"94\",\"name\":\"2026-27 Regular Season\"}]})");
        });
        let teams = server.mock(|when, then| {
            when.method(GET).query_param("view", "teamsForSeason");
            then.status(200).body("({\"teamsNoAll\":[{\"id\":\"307\",\"name\":\"Hartford Wolf Pack\",\"nickname\":\"Wolf Pack\",\"team_code\":\"HFD\",\"division_id\":\"15\",\"logo\":\"https://example.test/hfd.png\"}]})");
        });
        let roster = server.mock(|when, then| {
            when.method(GET).query_param("view", "roster");
            then.status(200).body(ROSTER_REPORT);
        });
        let skaters = server.mock(|when, then| {
            when.method(GET).query_param("position", "skaters");
            then.status(200).body(SKATER_REPORT);
        });
        let goalies = server.mock(|when, then| {
            when.method(GET).query_param("position", "goalies");
            then.status(200).body(GOALIE_REPORT);
        });

        let snapshot =
            AhlFeedClient::with_base_url_and_cache(server.url("/feed"), cache.path().to_path_buf())
                .fetch_roster_stats(20262027, &["HFD".to_owned()])
                .await
                .unwrap();
        assert_eq!(snapshot.provider_season_id, "94");
        assert_eq!(snapshot.teams.len(), 1);
        assert_eq!(snapshot.teams[0].nhl_affiliate.as_deref(), Some("NYR"));
        assert_eq!(snapshot.teams[0].roster.len(), 1);
        assert_eq!(snapshot.teams[0].roster[0].provider_player_id, "10618");
        assert_eq!(snapshot.teams[0].skaters.len(), 1);
        assert_eq!(snapshot.teams[0].goalies.len(), 1);
        let enrichment = AhlProjectionPlayerEnrichment {
            provider_player_id: "10618".to_owned(),
            nhl_player_id: 8_480_001,
            primary_position: icelines_core::model::Position::Center,
            eligible_positions: vec![icelines_core::model::Position::Center],
            projected_score: 42.0,
            prospect: true,
            recall_readiness: Some(0.65),
            professional_games_at_season_start: Some(80),
            assigned_to_affiliate: true,
            waiver_required: false,
        };
        let input = affiliate_projection_input_from_snapshot(
            &snapshot,
            "NYR",
            "Hartford Wolf Pack",
            icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput::default(),
            &[enrichment],
        )
        .unwrap();
        assert_eq!(input.players[0].player_id, 8_480_001);
        assert_eq!(input.players[0].display_name, "Aidan Thompson");
        assert!(affiliate_projection_input_from_snapshot(
            &snapshot,
            "NYR",
            "Hartford Wolf Pack",
            icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput::default(),
            &[],
        )
        .unwrap_err()
        .to_string()
        .contains("exactly cover"));
        let cache_manifest = crate::fletch::read_fletch_cache_manifest(
            &crate::fletch::fletch_cache_manifest_path(cache.path()),
        )
        .unwrap();
        assert_eq!(cache_manifest.entries.len(), 5);
        assert!(cache_manifest.entries.iter().all(|entry| entry.verified));
        seasons.assert();
        teams.assert();
        roster.assert();
        skaters.assert();
        goalies.assert();
    }

    fn identity_snapshot() -> AhlRosterStatsSnapshot {
        AhlRosterStatsSnapshot {
            schema: AHL_ROSTER_STATS_SCHEMA.to_owned(),
            season: 20262027,
            provider: AHL_PROVIDER.to_owned(),
            provider_season_id: "94".to_owned(),
            provider_season_name: "2026-27 Regular Season".to_owned(),
            fetched_at: "2026-07-24T12:00:00Z".to_owned(),
            source_url: AHL_STATS_SOURCE_URL.to_owned(),
            roster_source_url: AHL_ROSTER_SOURCE_URL.to_owned(),
            identity_note: "provider-local identity".to_owned(),
            teams: vec![AhlTeamRosterStats {
                provider: AHL_PROVIDER.to_owned(),
                provider_team_id: "307".to_owned(),
                team_code: "HFD".to_owned(),
                team_name: "Hartford Wolf Pack".to_owned(),
                nickname: "Wolf Pack".to_owned(),
                division_id: "15".to_owned(),
                logo_url: "https://example.test/hfd.png".to_owned(),
                nhl_affiliate: Some("NYR".to_owned()),
                roster: vec![AhlRosterPlayer {
                    provider: AHL_PROVIDER.to_owned(),
                    provider_player_id: "10618".to_owned(),
                    name: "Aidan Thompson".to_owned(),
                    position_group: "Forwards".to_owned(),
                    position: "F".to_owned(),
                    jersey_number: "6".to_owned(),
                    handedness: "L".to_owned(),
                    height: "5-11".to_owned(),
                    weight_pounds: "180".to_owned(),
                    birthdate: "2002-02-18".to_owned(),
                    birthplace: "Fort Collins, CO".to_owned(),
                }],
                skaters: Vec::new(),
                goalies: Vec::new(),
            }],
        }
    }

    fn identity_catalog() -> AhlCanonicalIdentityCatalog {
        AhlCanonicalIdentityCatalog {
            schema: AHL_CANONICAL_IDENTITY_CATALOG_SCHEMA.to_owned(),
            checked_at: "2026-07-24".to_owned(),
            candidates: vec![AhlCanonicalIdentityCandidate {
                nhl_player_id: 8_480_001,
                display_name: "Aidan Thompson".to_owned(),
                birth_date: Some("2002-02-18".to_owned()),
                evidence_urls: vec!["https://www.nhl.com/player/8480001".to_owned()],
            }],
        }
    }

    #[test]
    fn exact_identity_match_remains_pending_review() {
        let view = build_ahl_identity_crosswalk(
            &identity_snapshot(),
            "Hartford Wolf Pack",
            &identity_catalog(),
        )
        .unwrap();
        assert_eq!(view.counts.exact_name_and_birth_date, 1);
        assert_eq!(view.counts.reviewed, 0);
        assert_eq!(view.rows[0].nhl_player_id, Some(8_480_001));
        assert_eq!(view.rows[0].review_status, AhlIdentityReviewStatus::Pending);
    }

    #[test]
    fn reviewed_identity_and_separate_facts_build_projection_input() {
        let snapshot = identity_snapshot();
        let mut crosswalk =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        crosswalk.rows[0].review_status = AhlIdentityReviewStatus::Reviewed;
        crosswalk.counts = identity_crosswalk_counts(&crosswalk.rows);
        let facts = AhlProjectionPlayerFacts {
            provider_player_id: "10618".to_owned(),
            primary_position: icelines_core::model::Position::Center,
            eligible_positions: vec![icelines_core::model::Position::Center],
            projected_score: 42.0,
            prospect: true,
            recall_readiness: Some(0.65),
            professional_games_at_season_start: Some(80),
            assigned_to_affiliate: true,
            waiver_required: false,
        };
        let input = affiliate_projection_input_from_reviewed_crosswalk(
            &snapshot,
            "NYR",
            "Hartford Wolf Pack",
            icelines_core::view_model::ahl_affiliate::AhlDevelopmentRuleInput::default(),
            &crosswalk,
            &[facts],
        )
        .unwrap();
        assert_eq!(input.players[0].player_id, 8_480_001);
        assert_eq!(input.players[0].projected_score, 42.0);
    }

    #[test]
    fn pending_review_and_birth_date_conflicts_fail_closed() {
        let snapshot = identity_snapshot();
        let pending =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        assert!(validate_reviewed_ahl_identity_crosswalk(
            &snapshot,
            "Hartford Wolf Pack",
            &pending
        )
        .unwrap_err()
        .to_string()
        .contains("not reviewed"));

        let mut catalog = identity_catalog();
        catalog.candidates[0].birth_date = Some("2001-02-18".to_owned());
        let conflict =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &catalog).unwrap();
        assert_eq!(
            conflict.rows[0].match_basis,
            AhlIdentityMatchBasis::BirthDateConflict
        );
        assert_eq!(conflict.rows[0].nhl_player_id, None);
    }

    #[test]
    fn empty_official_roster_can_be_audited_but_not_certified() {
        let mut snapshot = identity_snapshot();
        snapshot.teams[0].roster.clear();
        let crosswalk =
            build_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &identity_catalog())
                .unwrap();
        assert_eq!(crosswalk.counts.roster_players, 0);
        let error =
            validate_reviewed_ahl_identity_crosswalk(&snapshot, "Hartford Wolf Pack", &crosswalk)
                .unwrap_err();
        assert!(error.to_string().contains("roster") && error.to_string().contains("empty"));
    }
}
