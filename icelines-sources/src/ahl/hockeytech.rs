//! Deterministic parser and normalized DTOs for the AHL HockeyTech Statview feed.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const AHL_ROSTER_STATS_SCHEMA: &str = "ahl_roster_stats.v1";
pub const AHL_PROVIDER: &str = "ahl_hockeytech_statview";
pub const AHL_STATS_SOURCE_URL: &str = "https://theahl.com/stats/player-stats";
pub const AHL_ROSTER_SOURCE_URL: &str = "https://theahl.com/stats/roster";

#[derive(Debug, Error)]
pub enum AhlHockeytechError {
    #[error("AHL feed schema changed: {0}")]
    Schema(String),
    #[error("AHL season not found: {0}")]
    SeasonNotFound(String),
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
    pub roster: Vec<AhlRosterPlayer>,
    pub skaters: Vec<AhlSkaterSeasonRow>,
    pub goalies: Vec<AhlGoalieSeasonRow>,
    #[serde(default)]
    pub source_warnings: Vec<String>,
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
pub struct ProviderSeason {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct SeasonsEnvelope {
    seasons: Vec<ProviderSeason>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderTeam {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub nickname: String,
    pub team_code: String,
    #[serde(default)]
    pub division_id: String,
    #[serde(default)]
    pub logo: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TeamsEnvelope {
    teams_no_all: Vec<ProviderTeam>,
}

pub fn parse_jsonp(body: &str) -> Result<Value, AhlHockeytechError> {
    let trimmed = body.trim();
    let json = trimmed
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .ok_or_else(|| {
            AhlHockeytechError::Schema("expected parenthesized JSONP body".to_owned())
        })?;
    serde_json::from_str(json)
        .map_err(|error| AhlHockeytechError::Schema(format!("invalid JSONP payload: {error}")))
}

pub fn season_label(season: u32) -> Result<String, AhlHockeytechError> {
    let text = format!("{season:08}");
    let start: u32 = text[..4]
        .parse()
        .map_err(|_| AhlHockeytechError::SeasonNotFound(text.clone()))?;
    let end: u32 = text[4..]
        .parse()
        .map_err(|_| AhlHockeytechError::SeasonNotFound(text.clone()))?;
    if end != start + 1 {
        return Err(AhlHockeytechError::SeasonNotFound(text));
    }
    Ok(format!("{start}-{:02} Regular Season", end % 100))
}

pub fn resolve_regular_season(
    value: Value,
    target: &str,
) -> Result<ProviderSeason, AhlHockeytechError> {
    let envelope: SeasonsEnvelope = serde_json::from_value(value)
        .map_err(|error| AhlHockeytechError::Schema(format!("season catalog: {error}")))?;
    envelope
        .seasons
        .into_iter()
        .find(|row| row.name == target)
        .ok_or_else(|| AhlHockeytechError::SeasonNotFound(target.to_owned()))
}

pub fn parse_team_catalog(value: Value) -> Result<Vec<ProviderTeam>, AhlHockeytechError> {
    let envelope: TeamsEnvelope = serde_json::from_value(value)
        .map_err(|error| AhlHockeytechError::Schema(format!("team catalog: {error}")))?;
    if envelope.teams_no_all.is_empty() {
        return Err(AhlHockeytechError::Schema(
            "team catalog was empty".to_owned(),
        ));
    }
    Ok(envelope.teams_no_all)
}

pub fn build_team_roster_stats(
    team: ProviderTeam,
    nhl_affiliate: Option<String>,
    roster_value: Value,
    skater_value: Value,
    goalie_value: Value,
) -> Result<AhlTeamRosterStats, AhlHockeytechError> {
    let players = roster_rows(&roster_value)?
        .into_iter()
        .map(|(group, row)| parse_roster_player(group, row))
        .collect::<Result<Vec<_>, _>>()?;
    let (mut roster, mut source_warnings) = deduplicate_roster_players(players, &team.team_code)?;
    let (skater_rows, skater_warnings) =
        team_report_rows(&skater_value, &team.team_code, "skater", true)?;
    let mut skaters = skater_rows
        .into_iter()
        .map(|row| parse_skater(row, &team.team_code))
        .collect::<Result<Vec<_>, _>>()?;
    source_warnings.extend(skater_warnings);
    let (goalie_rows, goalie_warnings) =
        team_report_rows(&goalie_value, &team.team_code, "goalie", false)?;
    let mut goalies = goalie_rows
        .into_iter()
        .map(|row| parse_goalie(row, &team.team_code))
        .collect::<Result<Vec<_>, _>>()?;
    source_warnings.extend(goalie_warnings);
    roster.sort_by(|left, right| {
        left.position_group
            .cmp(&right.position_group)
            .then(left.name.cmp(&right.name))
            .then(left.provider_player_id.cmp(&right.provider_player_id))
    });
    skaters.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.provider_player_id.cmp(&right.provider_player_id))
    });
    goalies.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then(left.provider_player_id.cmp(&right.provider_player_id))
    });
    Ok(AhlTeamRosterStats {
        provider: AHL_PROVIDER.to_owned(),
        provider_team_id: team.id,
        team_code: team.team_code,
        team_name: team.name,
        nickname: team.nickname,
        division_id: team.division_id,
        logo_url: team.logo,
        nhl_affiliate,
        roster,
        skaters,
        goalies,
        source_warnings,
    })
}

impl AhlRosterStatsSnapshot {
    pub fn validate(&self) -> Result<(), AhlHockeytechError> {
        if self.schema != AHL_ROSTER_STATS_SCHEMA {
            return Err(AhlHockeytechError::Validation(format!(
                "unexpected schema {}",
                self.schema
            )));
        }
        let mut team_ids = BTreeSet::new();
        let mut team_codes = BTreeSet::new();
        for team in &self.teams {
            if !team_ids.insert(team.provider_team_id.as_str()) {
                return Err(AhlHockeytechError::Validation(format!(
                    "duplicate provider team id {}",
                    team.provider_team_id
                )));
            }
            if !team_codes.insert(team.team_code.as_str()) {
                return Err(AhlHockeytechError::Validation(format!(
                    "duplicate AHL team code {}",
                    team.team_code
                )));
            }
            validate_player_ids(
                team,
                &team
                    .skaters
                    .iter()
                    .map(|player| {
                        (
                            player.provider_player_id.as_str(),
                            player.team_code.as_str(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )?;
            validate_player_ids(
                team,
                &team
                    .goalies
                    .iter()
                    .map(|player| {
                        (
                            player.provider_player_id.as_str(),
                            player.team_code.as_str(),
                        )
                    })
                    .collect::<Vec<_>>(),
            )?;
            let mut roster_ids = BTreeSet::new();
            for player in &team.roster {
                if !roster_ids.insert(player.provider_player_id.as_str()) {
                    return Err(AhlHockeytechError::Validation(format!(
                        "duplicate roster provider player id {} on {}",
                        player.provider_player_id, team.team_code
                    )));
                }
            }
            for player in &team.skaters {
                if player.goals + player.assists != player.points {
                    return Err(AhlHockeytechError::Validation(format!(
                        "{} points do not equal goals plus assists",
                        player.name
                    )));
                }
            }
        }
        Ok(())
    }
}

pub fn report_rows(value: &Value) -> Result<Vec<&Value>, AhlHockeytechError> {
    let reports = value
        .as_array()
        .ok_or_else(|| AhlHockeytechError::Schema("player report root was not an array".into()))?;
    let mut rows = Vec::new();
    for report in reports {
        let sections = report
            .get("sections")
            .and_then(Value::as_array)
            .ok_or_else(|| AhlHockeytechError::Schema("player report sections missing".into()))?;
        for section in sections {
            let data = section
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AhlHockeytechError::Schema("player report data missing".into()))?;
            for item in data {
                let row = item.get("row").ok_or_else(|| {
                    AhlHockeytechError::Schema("player report row missing".into())
                })?;
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
                    return Err(AhlHockeytechError::Schema(format!(
                        "player report row `{label}` had no player_id"
                    )));
                }
            }
        }
    }
    Ok(rows)
}

pub fn team_report_rows<'a>(
    value: &'a Value,
    expected_team: &str,
    report_kind: &str,
    exclude_goalies: bool,
) -> Result<(Vec<&'a Value>, Vec<String>), AhlHockeytechError> {
    let rows = report_rows(value)?;
    let mut retained = Vec::new();
    let mut wrong_team = Vec::new();
    let mut goalie_scoring_rows = Vec::new();
    for row in rows {
        let actual_team = string_field(row, "team_code")?;
        let identity = format!(
            "{} #{}",
            string_field(row, "name")?,
            string_field(row, "player_id")?
        );
        if actual_team != expected_team {
            wrong_team.push(format!("{identity} ({actual_team})"));
        } else if exclude_goalies && string_field(row, "position")? == "G" {
            goalie_scoring_rows.push(identity);
        } else {
            retained.push(row);
        }
    }
    if retained.is_empty() && !wrong_team.is_empty() {
        return Err(AhlHockeytechError::Validation(format!(
            "{report_kind} report for {expected_team} contained only other-team rows: {}",
            wrong_team.join(", ")
        )));
    }
    let mut warnings = Vec::new();
    if !wrong_team.is_empty() {
        warnings.push(format!(
            "Excluded {} other-team row(s) from the {report_kind} report for {expected_team}: {}.",
            wrong_team.len(),
            wrong_team.join(", ")
        ));
    }
    if !goalie_scoring_rows.is_empty() {
        warnings.push(format!(
            "Excluded {} goalie scoring row(s) from the skater report for {expected_team}; typed goalie totals come from the separate goalie report: {}.",
            goalie_scoring_rows.len(),
            goalie_scoring_rows.join(", ")
        ));
    }
    Ok((retained, warnings))
}

fn roster_rows(value: &Value) -> Result<Vec<(&str, &Value)>, AhlHockeytechError> {
    let reports = value
        .get("roster")
        .and_then(Value::as_array)
        .ok_or_else(|| AhlHockeytechError::Schema("roster report missing".into()))?;
    let mut rows = Vec::new();
    for report in reports {
        let sections = report
            .get("sections")
            .and_then(Value::as_array)
            .ok_or_else(|| AhlHockeytechError::Schema("roster sections missing".into()))?;
        for section in sections {
            let title = section
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| AhlHockeytechError::Schema("roster section title missing".into()))?;
            if title == "Team Personnel" {
                continue;
            }
            let data = section
                .get("data")
                .and_then(Value::as_array)
                .ok_or_else(|| AhlHockeytechError::Schema("roster section data missing".into()))?;
            for item in data {
                rows.push((
                    title,
                    item.get("row").ok_or_else(|| {
                        AhlHockeytechError::Schema("roster player row missing".into())
                    })?,
                ));
            }
        }
    }
    Ok(rows)
}

pub fn deduplicate_roster_players(
    players: Vec<AhlRosterPlayer>,
    team_code: &str,
) -> Result<(Vec<AhlRosterPlayer>, Vec<String>), AhlHockeytechError> {
    let mut retained: Vec<AhlRosterPlayer> = Vec::new();
    let mut index_by_id = BTreeMap::new();
    let mut warnings = Vec::new();
    for player in players {
        let Some(existing_index) = index_by_id.get(&player.provider_player_id).copied() else {
            index_by_id.insert(player.provider_player_id.clone(), retained.len());
            retained.push(player);
            continue;
        };
        let existing = &mut retained[existing_index];
        let existing_jersey = existing.jersey_number.clone();
        let duplicate_jersey = player.jersey_number.clone();
        let existing_position = existing.position.clone();
        let duplicate_position = player.position.clone();
        let mut comparable_existing = existing.clone();
        let mut comparable_duplicate = player.clone();
        comparable_existing.jersey_number.clear();
        comparable_duplicate.jersey_number.clear();
        comparable_existing.position.clear();
        comparable_duplicate.position.clear();
        if comparable_existing != comparable_duplicate {
            return Err(AhlHockeytechError::Validation(format!(
                "conflicting duplicate roster rows for {} #{} on {team_code}",
                player.name, player.provider_player_id
            )));
        }
        let position_changed = existing_position != duplicate_position;
        if position_changed
            && !(is_forward_roster_position(&existing_position)
                && is_forward_roster_position(&duplicate_position))
        {
            return Err(AhlHockeytechError::Validation(format!(
                "conflicting duplicate roster positions `{existing_position}` and `{duplicate_position}` for {} #{} on {team_code}",
                player.name, player.provider_player_id
            )));
        }
        let jersey_changed = existing_jersey != duplicate_jersey;
        if position_changed || jersey_changed {
            let mut changes = Vec::new();
            if position_changed {
                existing.position = "F".to_owned();
                changes.push(format!(
                    "forward positions `{existing_position}` and `{duplicate_position}` were generalized to `F`"
                ));
            }
            if jersey_changed {
                existing.jersey_number.clear();
                changes.push(format!(
                    "jersey numbers `{existing_jersey}` and `{duplicate_jersey}` were omitted"
                ));
            }
            warnings.push(format!(
                "Collapsed compatible duplicate roster rows for {} #{} on {team_code}; {}.",
                player.name,
                player.provider_player_id,
                changes.join(" and ")
            ));
        } else {
            warnings.push(format!(
                "Collapsed an exact duplicate roster row for {} #{} on {team_code}.",
                player.name, player.provider_player_id
            ));
        }
    }
    Ok((retained, warnings))
}

fn is_forward_roster_position(position: &str) -> bool {
    matches!(position, "F" | "C" | "LW" | "RW")
}

fn parse_roster_player(group: &str, row: &Value) -> Result<AhlRosterPlayer, AhlHockeytechError> {
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

pub fn parse_skater(
    row: &Value,
    expected_team: &str,
) -> Result<AhlSkaterSeasonRow, AhlHockeytechError> {
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

fn parse_goalie(
    row: &Value,
    expected_team: &str,
) -> Result<AhlGoalieSeasonRow, AhlHockeytechError> {
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

fn checked_team_code(row: &Value, expected: &str) -> Result<String, AhlHockeytechError> {
    let actual = string_field(row, "team_code")?;
    if actual != expected {
        return Err(AhlHockeytechError::Validation(format!(
            "feed returned team code {actual} while fetching {expected}"
        )));
    }
    Ok(actual)
}

pub fn string_field(row: &Value, field: &str) -> Result<String, AhlHockeytechError> {
    row.get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AhlHockeytechError::Schema(format!("missing string field `{field}`")))
}

fn optional_string_field(row: &Value, field: &str) -> String {
    row.get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn bool_field(row: &Value, field: &str) -> Result<bool, AhlHockeytechError> {
    match string_field(row, field)?.as_str() {
        "1" => Ok(true),
        "0" => Ok(false),
        value => Err(AhlHockeytechError::Schema(format!(
            "invalid boolean `{field}` value {value}"
        ))),
    }
}

fn u32_field(row: &Value, field: &str) -> Result<u32, AhlHockeytechError> {
    string_field(row, field)?
        .parse()
        .map_err(|error| AhlHockeytechError::Schema(format!("invalid integer `{field}`: {error}")))
}

fn i32_field(row: &Value, field: &str) -> Result<i32, AhlHockeytechError> {
    string_field(row, field)?
        .parse()
        .map_err(|error| AhlHockeytechError::Schema(format!("invalid integer `{field}`: {error}")))
}

fn f64_field(row: &Value, field: &str) -> Result<f64, AhlHockeytechError> {
    string_field(row, field)?
        .parse()
        .map_err(|error| AhlHockeytechError::Schema(format!("invalid decimal `{field}`: {error}")))
}

fn validate_player_ids(
    team: &AhlTeamRosterStats,
    players: &[(&str, &str)],
) -> Result<(), AhlHockeytechError> {
    let mut ids = BTreeSet::new();
    for (id, code) in players {
        if !ids.insert(*id) {
            return Err(AhlHockeytechError::Validation(format!(
                "duplicate provider player id {id} on {}",
                team.team_code
            )));
        }
        if *code != team.team_code {
            return Err(AhlHockeytechError::Validation(format!(
                "player team code {code} does not match {}",
                team.team_code
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{build_team_roster_stats, parse_jsonp, ProviderTeam};

    #[test]
    fn parses_jsonp_and_builds_typed_team_stats() {
        let roster = parse_jsonp(r#"({"roster":[{"sections":[{"title":"Forwards","data":[{"row":{"shoots":"L","player_id":"1","name":"A Player","position":"C"}}]}]}]})"#).unwrap();
        let skaters = parse_jsonp(r#"([{"sections":[{"data":[{"row":{"player_id":"1","name":"A Player","active":"1","position":"C","rookie":"1","team_code":"HFD","games_played":"1","goals":"1","assists":"0","points":"1","plus_minus":"0","penalty_minutes":"0","power_play_goals":"0","short_handed_goals":"0","shots":"1"}}]}]}])"#).unwrap();
        let goalies = parse_jsonp("([])").unwrap();
        let team = build_team_roster_stats(
            ProviderTeam {
                id: "10".into(),
                name: "Hartford Wolf Pack".into(),
                nickname: "Wolf Pack".into(),
                team_code: "HFD".into(),
                division_id: "1".into(),
                logo: String::new(),
            },
            Some("NYR".into()),
            roster,
            skaters,
            goalies,
        )
        .unwrap();
        assert_eq!(team.roster.len(), 1);
        assert_eq!(team.skaters[0].points, 1);
    }
}
