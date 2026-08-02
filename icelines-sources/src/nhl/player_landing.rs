use crate::adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdditiveFieldPolicy,
    HistoricalAvailability, SourceAdapter, SourceDescriptor, SourceInput,
};
use chrono::{DateTime, TimeZone, Utc};
use icelines_core::career_history::{CareerGameType, CareerHistory, CareerStint, LeagueAbbrev};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterId, AdapterVersion, EffectivePrecision, EffectiveTime, FactAssertion, FactAuthority,
    FactId, FactSubject, FreshnessClass, OrganizationId, PlayerOrganizationEvent, ProviderId,
    SourceEvidence, SourceFact, SourceUrl,
};
use icelines_core::{PlayerAwardRow, PlayerAwardSeasonRow};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::PlayerContract;

/// Extract contract hints from an official NHL player landing payload. The
/// current public API normally omits these fields; absence remains `None`.
pub fn parse_player_landing_contract(player_id: u32, raw: &Value) -> PlayerContract {
    let expiry_year = raw["currentContract"]["expiryYear"]
        .as_u64()
        .map(|year| year as u16)
        .or_else(|| raw["expiryYear"].as_u64().map(|year| year as u16));
    let expiry_type = raw["currentContract"]["expiryType"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| raw["expiryType"].as_str().map(str::to_owned));
    let salary = raw["currentContract"]["capHit"]
        .as_u64()
        .or_else(|| raw["currentContract"]["salary"].as_u64())
        .or_else(|| raw["capHit"].as_u64());
    PlayerContract {
        player_id,
        expiry_year,
        expiry_type,
        salary,
        ..PlayerContract::default()
    }
}

/// Parse the provider-owned awards portion of a player landing document.
/// View context and player presentation remain consumer responsibilities.
pub fn parse_player_award_rows(raw: &Value) -> Vec<PlayerAwardRow> {
    raw.get("awards")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(parse_award_row).collect())
        .unwrap_or_default()
}

fn parse_award_row(entry: &Value) -> Option<PlayerAwardRow> {
    let trophy = entry
        .get("trophy")
        .and_then(|value| value.get("default"))
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())?
        .to_owned();
    let seasons = entry
        .get("seasons")
        .and_then(Value::as_array)
        .map(|rows| rows.iter().filter_map(parse_award_season).collect())
        .unwrap_or_default();
    Some(PlayerAwardRow { trophy, seasons })
}

fn parse_award_season(entry: &Value) -> Option<PlayerAwardSeasonRow> {
    let season = entry
        .get("seasonId")
        .or_else(|| entry.get("season"))
        .and_then(Value::as_u64)? as u32;
    Some(PlayerAwardSeasonRow {
        season: Season(season),
        game_type_id: entry
            .get("gameTypeId")
            .and_then(Value::as_u64)
            .map(|value| value as u8)
            .unwrap_or(2),
        games_played: u32_field(entry, "gamesPlayed"),
        goals: u32_field(entry, "goals"),
        assists: u32_field(entry, "assists"),
        points: u32_field(entry, "points"),
        plus_minus: i32_field(entry, "plusMinus"),
        pim: u32_field(entry, "pim"),
        hits: u32_field(entry, "hits"),
        blocked_shots: u32_field(entry, "blockedShots"),
    })
}

#[derive(Debug, thiserror::Error)]
pub enum CareerParseError {
    #[error("missing or invalid `seasonTotals` array")]
    MissingSeasonTotals,
    #[error("official NHL landing organization fact is invalid: {0}")]
    InvalidOrganizationFact(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialNhlOrganizationFact {
    pub player_id: u32,
    #[serde(default)]
    pub current_team_abbrev: Option<String>,
    #[serde(default)]
    pub current_team_id: Option<u32>,
    #[serde(default)]
    pub is_active: Option<bool>,
    pub observed_at: String,
    pub source_url: String,
}

/// Parses the official NHL player landing document into an immutable draft
/// assertion. A missing `draftDetails` object is an authoritative empty result,
/// not a parse failure and not evidence of current organization control.
#[derive(Debug, Clone)]
pub struct OfficialNhlDraftAdapter {
    player_id: PlayerId,
    captured_at: DateTime<Utc>,
}

impl OfficialNhlDraftAdapter {
    pub fn new(player_id: u32, captured_at: DateTime<Utc>) -> Result<Self, CareerParseError> {
        let player_id = PlayerId::try_new(player_id).map_err(|_| {
            CareerParseError::InvalidOrganizationFact("player id must be non-zero".to_owned())
        })?;
        Ok(Self {
            player_id,
            captured_at,
        })
    }
}

impl SourceAdapter for OfficialNhlDraftAdapter {
    type Output = Option<FactAssertion<SourceFact>>;

    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            source_id: icelines_core::source_facts::SourceId::try_new(format!(
                "nhl-player-landing:{}",
                self.player_id.0
            ))
            .expect("non-zero numeric player id is a valid source id"),
            provider: ProviderId::try_new("official_nhl_api").expect("static provider id is valid"),
            adapter_id: AdapterId::try_new("nhl.player_landing.draft")
                .expect("static adapter id is valid"),
            adapter_version: AdapterVersion::try_new("v1")
                .expect("static adapter version is valid"),
            payload_family: "official_nhl_player_landing",
            supported_layouts: &["nhl_player_landing.v1"],
            required_identity_keys: &["playerId"],
            additive_field_policy: AdditiveFieldPolicy::IgnoreReviewed,
            freshness_class: FreshnessClass::Static,
            historical_availability: HistoricalAvailability::CallerSuppliedArchive,
            absence_semantics: AbsenceSemantics::AuthoritativeEmpty,
            output_fact_families: &["player_organization.drafted"],
        }
    }

    fn parse(&self, input: SourceInput<'_>) -> Result<Self::Output, AdapterError> {
        let descriptor = self.descriptor();
        let raw: Value = serde_json::from_slice(input.bytes()).map_err(|error| AdapterError {
            source_id: input.source_id().clone(),
            adapter_id: descriptor.adapter_id.clone(),
            input_hash: input.content_hash().clone(),
            category: AdapterErrorCategory::UnsupportedLayout,
            disposition: AdapterDisposition::FatalSource,
            message: format!("invalid JSON: {error}"),
        })?;
        let found_player_id = raw.get("playerId").and_then(Value::as_u64);
        if found_player_id != Some(u64::from(self.player_id.0)) {
            return Err(AdapterError {
                source_id: input.source_id().clone(),
                adapter_id: descriptor.adapter_id.clone(),
                input_hash: input.content_hash().clone(),
                category: AdapterErrorCategory::SemanticValidation,
                disposition: AdapterDisposition::FatalSource,
                message: format!(
                    "landing playerId {:?} does not match requested player {}",
                    found_player_id, self.player_id.0
                ),
            });
        }
        let Some(draft) = raw.get("draftDetails") else {
            return Ok(None);
        };
        let parsed = parse_draft_details(draft).map_err(|message| AdapterError {
            source_id: input.source_id().clone(),
            adapter_id: descriptor.adapter_id.clone(),
            input_hash: input.content_hash().clone(),
            category: AdapterErrorCategory::MalformedRecord,
            disposition: AdapterDisposition::QuarantinedRecord,
            message,
        })?;
        let source_url = SourceUrl::try_new(format!(
            "https://api-web.nhle.com/v1/player/{}/landing",
            self.player_id.0
        ))
        .expect("official NHL player URL is valid");
        let evidence = SourceEvidence::new(
            input.source_id().clone(),
            source_url,
            ProviderId::try_new("official_nhl_api").expect("static provider id is valid"),
            self.captured_at,
            input.content_hash().clone(),
            descriptor.adapter_version,
        );
        let occurred_at = EffectiveTime::new(
            Utc.with_ymd_and_hms(i32::from(parsed.year), 1, 1, 0, 0, 0)
                .single()
                .expect("valid draft year"),
            None,
            EffectivePrecision::Season,
        )
        .expect("single-ended effective time is valid");
        FactAssertion::new(
            FactId::try_new(format!("nhl-draft:{}:{}", self.player_id.0, parsed.year))
                .expect("numeric draft fact id is valid"),
            format!("player:{}:draft", self.player_id.0),
            FactSubject::Player(self.player_id),
            occurred_at,
            FactAuthority::Draft,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                by: parsed.organization,
                year: parsed.year,
                round: parsed.round,
                overall: parsed.overall,
            }),
            vec![evidence],
        )
        .map(Some)
        .map_err(|error| AdapterError {
            source_id: input.source_id().clone(),
            adapter_id: descriptor.adapter_id,
            input_hash: input.content_hash().clone(),
            category: AdapterErrorCategory::SemanticValidation,
            disposition: AdapterDisposition::QuarantinedRecord,
            message: error.to_string(),
        })
    }
}

struct ParsedDraftDetails {
    organization: OrganizationId,
    year: u16,
    round: u8,
    overall: u16,
}

fn parse_draft_details(raw: &Value) -> Result<ParsedDraftDetails, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "draftDetails must be an object".to_owned())?;
    let number = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("draftDetails.{key} must be an unsigned integer"))
    };
    let year = u16::try_from(number("year")?)
        .map_err(|_| "draftDetails.year is outside u16 range".to_owned())?;
    if !(1917..=2200).contains(&year) {
        return Err("draftDetails.year is outside the supported NHL range".to_owned());
    }
    let round = u8::try_from(number("round")?)
        .map_err(|_| "draftDetails.round is outside u8 range".to_owned())?;
    let overall = u16::try_from(number("overallPick")?)
        .map_err(|_| "draftDetails.overallPick is outside u16 range".to_owned())?;
    if round == 0 || overall == 0 {
        return Err("draft round and overall pick must be non-zero".to_owned());
    }
    let team = object
        .get("teamAbbrev")
        .and_then(Value::as_str)
        .ok_or_else(|| "draftDetails.teamAbbrev must be a string".to_owned())?
        .trim()
        .to_ascii_uppercase();
    if !(2..=4).contains(&team.len()) || !team.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err("draftDetails.teamAbbrev must be a 2-4 letter abbreviation".to_owned());
    }
    Ok(ParsedDraftDetails {
        organization: OrganizationId::try_new(team).map_err(|error| error.to_string())?,
        year,
        round,
        overall,
    })
}

pub fn parse_official_nhl_organization_fact(
    player_id: u32,
    observed_at: impl Into<String>,
    raw: &Value,
) -> Result<OfficialNhlOrganizationFact, CareerParseError> {
    let observed_at = observed_at.into();
    if chrono::DateTime::parse_from_rfc3339(&observed_at).is_err() {
        return Err(CareerParseError::InvalidOrganizationFact(
            "observed_at must be RFC 3339".to_owned(),
        ));
    }
    if raw
        .get("playerId")
        .and_then(Value::as_u64)
        .is_some_and(|found| found != u64::from(player_id))
    {
        return Err(CareerParseError::InvalidOrganizationFact(format!(
            "landing playerId does not match requested player {player_id}"
        )));
    }
    let current_team_abbrev = raw
        .get("currentTeamAbbrev")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_uppercase);
    if current_team_abbrev.as_deref().is_some_and(|team| {
        !(2..=4).contains(&team.len()) || !team.bytes().all(|byte| byte.is_ascii_uppercase())
    }) {
        return Err(CareerParseError::InvalidOrganizationFact(
            "currentTeamAbbrev must be a 2-4 character uppercase abbreviation".to_owned(),
        ));
    }
    Ok(OfficialNhlOrganizationFact {
        player_id,
        current_team_abbrev,
        current_team_id: raw
            .get("currentTeamId")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
        is_active: raw.get("isActive").and_then(Value::as_bool),
        observed_at,
        source_url: format!("https://api-web.nhle.com/v1/player/{player_id}/landing"),
    })
}

pub fn parse_career_history(
    player_id: u32,
    raw: &Value,
) -> Result<CareerHistory, CareerParseError> {
    let totals = raw
        .get("seasonTotals")
        .and_then(Value::as_array)
        .ok_or(CareerParseError::MissingSeasonTotals)?;
    let mut stints = Vec::with_capacity(totals.len());
    for entry in totals {
        let Some(season) = entry.get("season").and_then(Value::as_u64) else {
            continue;
        };
        let Some(game_type_id) = entry.get("gameTypeId").and_then(Value::as_u64) else {
            continue;
        };
        let Some(game_type) = CareerGameType::from_api_id(game_type_id as u32) else {
            continue;
        };
        let Some(league) = entry.get("leagueAbbrev").and_then(Value::as_str) else {
            continue;
        };
        let Some(gp) = entry.get("gamesPlayed").and_then(Value::as_u64) else {
            continue;
        };
        let team = entry
            .get("teamName")
            .and_then(|value| value.get("default"))
            .and_then(Value::as_str)
            .or_else(|| {
                entry
                    .get("teamCommonName")
                    .and_then(|value| value.get("default"))
                    .and_then(Value::as_str)
            })
            .unwrap_or("")
            .to_owned();
        let sequence = entry
            .get("sequence")
            .and_then(Value::as_u64)
            .map(|value| value as u8)
            .unwrap_or(1);
        stints.push(CareerStint {
            season: Season(season as u32),
            league: LeagueAbbrev::new(league),
            team,
            game_type,
            sequence,
            gp: gp as u32,
            goals: u32_field(entry, "goals"),
            assists: u32_field(entry, "assists"),
            points: u32_field(entry, "points"),
            pim: u32_field(entry, "pim"),
            plus_minus: i32_field(entry, "plusMinus"),
            power_play_goals: u32_field(entry, "powerPlayGoals"),
            power_play_points: u32_field(entry, "powerPlayPoints"),
            shorthanded_goals: u32_field(entry, "shorthandedGoals"),
            shorthanded_points: u32_field(entry, "shorthandedPoints"),
            game_winning_goals: u32_field(entry, "gameWinningGoals"),
            ot_goals: u32_field(entry, "otGoals"),
            shots: u32_field(entry, "shots"),
            shooting_pct: f32_field(entry, "shootingPctg"),
            avg_toi_sec: toi_field(entry, "avgToi"),
            faceoff_win_pct: f32_field(entry, "faceoffWinningPctg"),
            games_started: u32_field(entry, "gamesStarted"),
            wins: u32_field(entry, "wins"),
            losses: u32_field(entry, "losses"),
            ot_losses: u32_field(entry, "otLosses"),
            goals_against: u32_field(entry, "goalsAgainst"),
            goals_against_avg: f32_field(entry, "goalsAgainstAvg"),
            save_pct: f32_field(entry, "savePctg"),
            shots_against: u32_field(entry, "shotsAgainst"),
            shutouts: u32_field(entry, "shutouts"),
            time_on_ice_sec: toi_field(entry, "timeOnIce"),
        });
    }
    let mut history = CareerHistory { player_id, stints };
    history.sort_for_display();
    Ok(history)
}

fn u32_field(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value as u32)
}

fn i32_field(value: &Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|value| value as i32)
}

fn f32_field(value: &Value, key: &str) -> Option<f32> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .map(|value| value as f32)
}

fn toi_field(value: &Value, key: &str) -> Option<u32> {
    let raw = value.get(key)?.as_str()?;
    let mut parts = raw.splitn(2, ':');
    let minutes: u32 = parts.next()?.parse().ok()?;
    let seconds: u32 = parts.next()?.parse().ok()?;
    Some(minutes * 60 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_hints_preserve_absence_and_reviewed_future_fields() {
        let absent = parse_player_landing_contract(8478402, &serde_json::json!({}));
        assert_eq!(absent.player_id, 8478402);
        assert!(absent.expiry_year.is_none());
        assert!(absent.salary.is_none());

        let present = parse_player_landing_contract(
            8478402,
            &serde_json::json!({
                "currentContract": {
                    "expiryYear": 2031,
                    "expiryType": "UFA",
                    "capHit": 18_000_000
                }
            }),
        );
        assert_eq!(present.expiry_year, Some(2031));
        assert_eq!(present.expiry_type.as_deref(), Some("UFA"));
        assert_eq!(present.salary, Some(18_000_000));
    }

    #[test]
    fn awards_parser_keeps_provider_rows_ui_neutral() {
        let rows = parse_player_award_rows(&serde_json::json!({
            "awards": [{
                "trophy": {"default": "Art Ross Trophy"},
                "seasons": [{"seasonId": 20242025, "points": 100}]
            }]
        }));
        assert_eq!(rows[0].trophy, "Art Ross Trophy");
        assert_eq!(rows[0].seasons[0].points, Some(100));
    }
}
