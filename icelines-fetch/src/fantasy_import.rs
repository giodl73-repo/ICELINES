use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context};
use csv::StringRecord;
use icelines_core::name::normalize_name;
use icelines_core::season_stats::SeasonType;
use icelines_core::view_model::{
    FantasyImportMode, FantasyImportRowInput, FantasyImportRowStatus, FantasyImportTeamInput,
    FantasyImportTeamStatus, FantasyImportView, FantasyImportViewInput, SourceKind, ViewWarning,
    WarningKind,
};
use icelines_core::{
    model::{Position, Season},
    RosterShapePlayerInput, RosterShapeStatus, RosterShapeValidationInput,
    RosterShapeValidationView, CURRENT_SEASON,
};

use crate::fantasy_db::{
    resolve_roster_shape, FantasyDb, LeagueRow, TeamRow, DEFAULT_ROSTER_SHAPE,
};

const DEFAULT_SCORING_SCHEME: &str = "yahoo-standard";

const PLAYER_ALIASES: &[&str] = &["Player", "Name", "Player Name"];
const FIRST_NAME_ALIASES: &[&str] = &["First Name", "First"];
const LAST_NAME_ALIASES: &[&str] = &["Last Name", "Last"];
const FANTASY_TEAM_ALIASES: &[&str] = &[
    "Fantasy Team",
    "Team Name",
    "Rostered By",
    "Owner Team",
    "Manager Team",
];
const OWNER_ALIASES: &[&str] = &["Owner", "Manager"];
const NHL_TEAM_ALIASES: &[&str] = &["NHL Team", "Team"];
const POSITION_ALIASES: &[&str] = &["Eligible Positions", "Positions"];

#[derive(Debug, Clone)]
pub struct FantasyRosterImportOptions {
    pub league_name: String,
    pub scoring_scheme: String,
    pub user_team: Option<String>,
    pub mode: FantasyImportMode,
    pub known_player_keys: Option<BTreeSet<String>>,
    pub known_player_positions: Option<BTreeMap<String, Vec<Position>>>,
}

impl FantasyRosterImportOptions {
    pub fn dry_run(league_name: impl Into<String>) -> Self {
        Self {
            league_name: league_name.into(),
            scoring_scheme: DEFAULT_SCORING_SCHEME.to_string(),
            user_team: None,
            mode: FantasyImportMode::DryRun,
            known_player_keys: None,
            known_player_positions: None,
        }
    }

    pub fn apply(league_name: impl Into<String>) -> Self {
        Self {
            mode: FantasyImportMode::Apply,
            ..Self::dry_run(league_name)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YahooRosterCsvRow {
    pub row_number: u32,
    pub player_name: String,
    pub fantasy_team: String,
    pub owner: Option<String>,
    pub nhl_team_hint: Option<String>,
    pub position_hint: Option<String>,
}

pub fn parse_yahoo_roster_csv(path: &Path) -> anyhow::Result<Vec<YahooRosterCsvRow>> {
    let raw = read_file_strip_bom(path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(raw.as_bytes());
    let headers = rdr
        .headers()
        .with_context(|| format!("read CSV headers from {}", path.display()))?
        .clone();
    let selection = HeaderSelection::from_headers(&headers)?;
    let mut rows = Vec::new();
    for (idx, record) in rdr.records().enumerate() {
        let record = record.with_context(|| format!("parse CSV row {}", idx + 2))?;
        rows.push(YahooRosterCsvRow {
            row_number: (idx + 2) as u32,
            player_name: selection.player_name(&record),
            fantasy_team: field(&record, Some(selection.fantasy_team)).to_string(),
            owner: optional_field(&record, selection.owner),
            nhl_team_hint: optional_field(&record, selection.nhl_team),
            position_hint: optional_field(&record, selection.position),
        });
    }
    Ok(rows)
}

pub fn import_yahoo_roster_csv(
    db: &FantasyDb,
    path: &Path,
    options: FantasyRosterImportOptions,
) -> anyhow::Result<FantasyImportView> {
    let csv_rows = parse_yahoo_roster_csv(path)?;
    import_yahoo_roster_rows(db, csv_rows, options)
}

pub fn import_yahoo_roster_rows(
    db: &FantasyDb,
    csv_rows: Vec<YahooRosterCsvRow>,
    options: FantasyRosterImportOptions,
) -> anyhow::Result<FantasyImportView> {
    let existing_league = find_league(db, &options.league_name)?;
    let existing_teams = existing_teams_by_name(db, existing_league.as_ref())?;
    let existing_rosters = existing_rosters_by_team(db, existing_league.as_ref(), &existing_teams)?;
    let existing_owner_by_player = existing_owner_by_player(&existing_rosters);
    let duplicate_team_keys = duplicate_player_keys_across_teams(&csv_rows);
    let mut seen_player_team = BTreeSet::<(String, String)>::new();

    let rows = csv_rows
        .iter()
        .map(|row| {
            row_input(
                row,
                &options,
                &duplicate_team_keys,
                &existing_owner_by_player,
                &mut seen_player_team,
            )
        })
        .collect::<Vec<_>>();
    let teams = team_inputs(
        &csv_rows,
        &rows,
        &options,
        &existing_teams,
        &existing_rosters,
    );
    let warnings = import_warnings(
        &options,
        existing_league.as_ref(),
        &teams,
        &rows,
        &existing_rosters,
    )?;

    if options.mode == FantasyImportMode::Apply {
        apply_import(db, &options, existing_league.as_ref(), &teams, &rows)?;
    }

    Ok(FantasyImportView::from_input(FantasyImportViewInput {
        season: Season(CURRENT_SEASON),
        season_type: SeasonType::Regular,
        league: options.league_name,
        mode: options.mode,
        teams,
        rows,
        source_state: Vec::new(),
        warnings,
    }))
}

#[derive(Debug, Clone, Copy)]
enum PlayerHeader {
    Combined(usize),
    Split { first: usize, last: usize },
}

#[derive(Debug, Clone, Copy)]
struct HeaderSelection {
    player: PlayerHeader,
    fantasy_team: usize,
    owner: Option<usize>,
    nhl_team: Option<usize>,
    position: Option<usize>,
}

impl HeaderSelection {
    fn from_headers(headers: &StringRecord) -> anyhow::Result<Self> {
        let player = find_header(headers, PLAYER_ALIASES)
            .map(PlayerHeader::Combined)
            .or_else(|| {
                let first = find_header(headers, FIRST_NAME_ALIASES)?;
                let last = find_header(headers, LAST_NAME_ALIASES)?;
                Some(PlayerHeader::Split { first, last })
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "missing required CSV column for player name; expected one of Player, Name, Player Name, or First Name + Last Name"
                )
            })?;
        let fantasy_team = find_header(headers, FANTASY_TEAM_ALIASES).ok_or_else(|| {
            anyhow::anyhow!(
                "missing required CSV column for fantasy team; expected one of Fantasy Team, Team Name, Rostered By, Owner Team, or Manager Team"
            )
        })?;

        Ok(Self {
            player,
            fantasy_team,
            owner: find_header(headers, OWNER_ALIASES),
            nhl_team: find_header(headers, NHL_TEAM_ALIASES),
            position: find_header(headers, POSITION_ALIASES),
        })
    }

    fn player_name(self, record: &StringRecord) -> String {
        match self.player {
            PlayerHeader::Combined(idx) => field(record, Some(idx)).to_string(),
            PlayerHeader::Split { first, last } => {
                let first = field(record, Some(first));
                let last = field(record, Some(last));
                format!("{first} {last}").trim().to_string()
            }
        }
    }
}

fn find_header(headers: &StringRecord, aliases: &[&str]) -> Option<usize> {
    headers.iter().position(|header| {
        aliases
            .iter()
            .any(|alias| header.trim().eq_ignore_ascii_case(alias))
    })
}

fn field(record: &StringRecord, idx: Option<usize>) -> &str {
    idx.and_then(|idx| record.get(idx)).unwrap_or("").trim()
}

fn optional_field(record: &StringRecord, idx: Option<usize>) -> Option<String> {
    let value = field(record, idx);
    (!value.is_empty()).then(|| value.to_string())
}

fn read_file_strip_bom(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .with_context(|| format!("read {}", path.display()))?;
    let stripped = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        &bytes
    };
    Ok(String::from_utf8_lossy(stripped).into_owned())
}

fn find_league(db: &FantasyDb, league_name: &str) -> anyhow::Result<Option<LeagueRow>> {
    Ok(db
        .list_leagues()?
        .into_iter()
        .find(|league| league.name == league_name))
}

fn existing_teams_by_name(
    db: &FantasyDb,
    league: Option<&LeagueRow>,
) -> anyhow::Result<BTreeMap<String, TeamRow>> {
    let Some(league) = league else {
        return Ok(BTreeMap::new());
    };
    Ok(db
        .list_teams(&league.id)?
        .into_iter()
        .map(|team| (team.name.clone(), team))
        .collect())
}

fn existing_rosters_by_team(
    db: &FantasyDb,
    league: Option<&LeagueRow>,
    teams: &BTreeMap<String, TeamRow>,
) -> anyhow::Result<BTreeMap<String, BTreeSet<String>>> {
    if league.is_none() {
        return Ok(BTreeMap::new());
    }
    teams
        .values()
        .map(|team| {
            let roster = db.list_roster(&team.id)?.into_iter().collect();
            Ok((team.name.clone(), roster))
        })
        .collect()
}

fn existing_owner_by_player(
    rosters: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeMap<String, String> {
    let mut owner_by_player = BTreeMap::new();
    for (team, roster) in rosters {
        for player in roster {
            owner_by_player
                .entry(player.clone())
                .or_insert_with(|| team.clone());
        }
    }
    owner_by_player
}

fn duplicate_player_keys_across_teams(rows: &[YahooRosterCsvRow]) -> BTreeSet<String> {
    let mut teams_by_player = BTreeMap::<String, BTreeSet<String>>::new();
    for row in rows {
        if row.player_name.trim().is_empty() || row.fantasy_team.trim().is_empty() {
            continue;
        }
        teams_by_player
            .entry(normalize_name(&row.player_name))
            .or_default()
            .insert(row.fantasy_team.trim().to_string());
    }
    teams_by_player
        .into_iter()
        .filter_map(|(player, teams)| (teams.len() > 1).then_some(player))
        .collect()
}

fn row_input(
    row: &YahooRosterCsvRow,
    options: &FantasyRosterImportOptions,
    duplicate_team_keys: &BTreeSet<String>,
    existing_owner_by_player: &BTreeMap<String, String>,
    seen_player_team: &mut BTreeSet<(String, String)>,
) -> FantasyImportRowInput {
    let player_name = row.player_name.trim().to_string();
    let fantasy_team = row.fantasy_team.trim().to_string();
    let normalized_name = (!player_name.is_empty()).then(|| normalize_name(&player_name));
    let mut status = FantasyImportRowStatus::Imported;
    let mut message = None;

    if player_name.is_empty() {
        status = FantasyImportRowStatus::Error;
        message = Some(format!("row {} is missing a player name", row.row_number));
    } else if fantasy_team.is_empty() {
        status = FantasyImportRowStatus::Error;
        message = Some(format!("row {} is missing a fantasy team", row.row_number));
    } else if let Some(normalized) = normalized_name.as_ref() {
        let player_team = (normalized.clone(), fantasy_team.clone());
        if duplicate_team_keys.contains(normalized) {
            status = FantasyImportRowStatus::Duplicate;
            message = Some(format!(
                "'{player_name}' appears on multiple fantasy teams in this CSV"
            ));
        } else if options
            .known_player_keys
            .as_ref()
            .is_some_and(|known| !known.contains(normalized))
        {
            status = FantasyImportRowStatus::Unresolved;
            message = Some(format!(
                "'{player_name}' was not found in the active player pool"
            ));
        } else if !seen_player_team.insert(player_team) {
            status = FantasyImportRowStatus::Skipped;
            message = Some(format!(
                "'{player_name}' appears more than once on '{fantasy_team}' in this CSV"
            ));
        } else if let Some(existing_team) = existing_owner_by_player.get(normalized) {
            if existing_team != &fantasy_team {
                status = FantasyImportRowStatus::Duplicate;
                message = Some(format!(
                    "'{player_name}' is already rostered by '{existing_team}'"
                ));
            }
        }
    }

    FantasyImportRowInput {
        row_number: row.row_number,
        player_name,
        normalized_name,
        fantasy_team: (!fantasy_team.is_empty()).then_some(fantasy_team),
        owner: row.owner.clone(),
        nhl_team_hint: row.nhl_team_hint.clone(),
        position_hint: row.position_hint.clone(),
        status,
        message,
    }
}

fn team_inputs(
    csv_rows: &[YahooRosterCsvRow],
    rows: &[FantasyImportRowInput],
    options: &FantasyRosterImportOptions,
    existing_teams: &BTreeMap<String, TeamRow>,
    existing_rosters: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<FantasyImportTeamInput> {
    let mut owners = BTreeMap::<String, Option<String>>::new();
    for row in csv_rows {
        let team = row.fantasy_team.trim();
        if team.is_empty() {
            continue;
        }
        owners
            .entry(team.to_string())
            .or_insert_with(|| row.owner.clone());
    }

    owners
        .into_iter()
        .map(|(team, owner)| {
            let imported = rows
                .iter()
                .filter(|row| {
                    row.fantasy_team.as_deref() == Some(team.as_str())
                        && row.status == FantasyImportRowStatus::Imported
                })
                .filter_map(|row| row.normalized_name.as_ref())
                .cloned()
                .collect::<BTreeSet<_>>();
            let existing = existing_rosters.get(&team).cloned().unwrap_or_default();
            let existing_team = existing_teams.get(&team);
            let status = if imported.is_empty() && existing_team.is_none() {
                FantasyImportTeamStatus::Error
            } else if existing_team.is_none() {
                FantasyImportTeamStatus::Created
            } else if imported.iter().any(|player| !existing.contains(player)) {
                FantasyImportTeamStatus::Updated
            } else {
                FantasyImportTeamStatus::Unchanged
            };
            let rostered_players_after =
                (!imported.is_empty() || existing_team.is_some()).then(|| {
                    existing
                        .union(&imported)
                        .count()
                        .try_into()
                        .unwrap_or(u16::MAX)
                });

            FantasyImportTeamInput {
                team: team.clone(),
                owner,
                is_user_team: options
                    .user_team
                    .as_ref()
                    .map(|user_team| user_team == &team)
                    .unwrap_or_else(|| existing_team.is_some_and(|team| team.is_user_team)),
                status,
                rostered_players_after,
            }
        })
        .collect()
}

fn import_warnings(
    options: &FantasyRosterImportOptions,
    existing_league: Option<&LeagueRow>,
    teams: &[FantasyImportTeamInput],
    rows: &[FantasyImportRowInput],
    existing_rosters: &BTreeMap<String, BTreeSet<String>>,
) -> anyhow::Result<Vec<ViewWarning>> {
    let mut warnings = Vec::new();
    if let Some(user_team) = options.user_team.as_ref() {
        if !teams.iter().any(|team| &team.team == user_team) {
            warnings.push(ViewWarning {
                kind: WarningKind::PartialSource,
                source: Some(SourceKind::FantasyImport),
                message: format!("user team '{user_team}' was not present in the import rows"),
                recovery: Vec::new(),
            });
        }
    }

    if let Some(player_positions) = options.known_player_positions.as_ref() {
        let shape_name = existing_league
            .map(|league| league.roster_shape.as_str())
            .unwrap_or(DEFAULT_ROSTER_SHAPE);
        let shape = resolve_roster_shape(shape_name)?;
        for validation in validate_import_roster_shapes(
            &options.league_name,
            &shape,
            teams,
            rows,
            existing_rosters,
            player_positions,
        ) {
            if validation.status == RosterShapeStatus::Invalid {
                warnings.push(roster_shape_warning(&validation));
            }
        }
    }

    Ok(warnings)
}

fn validate_import_roster_shapes(
    league_name: &str,
    shape: &icelines_core::RosterShape,
    teams: &[FantasyImportTeamInput],
    rows: &[FantasyImportRowInput],
    existing_rosters: &BTreeMap<String, BTreeSet<String>>,
    player_positions: &BTreeMap<String, Vec<Position>>,
) -> Vec<RosterShapeValidationView> {
    teams
        .iter()
        .filter(|team| team.status != FantasyImportTeamStatus::Error)
        .map(|team| {
            let mut roster_keys = existing_rosters
                .get(&team.team)
                .cloned()
                .unwrap_or_default();
            rows.iter()
                .filter(|row| {
                    row.fantasy_team.as_deref() == Some(team.team.as_str())
                        && row.status == FantasyImportRowStatus::Imported
                })
                .filter_map(|row| row.normalized_name.as_ref())
                .for_each(|player| {
                    roster_keys.insert(player.clone());
                });
            let display_by_key = rows
                .iter()
                .filter_map(|row| {
                    row.normalized_name
                        .as_ref()
                        .map(|key| (key.clone(), row.player_name.clone()))
                })
                .collect::<BTreeMap<_, _>>();
            let players = roster_keys
                .into_iter()
                .map(|player_key| {
                    let display_name = display_by_key
                        .get(&player_key)
                        .cloned()
                        .unwrap_or_else(|| player_key.clone());
                    let positions = player_positions
                        .get(&player_key)
                        .cloned()
                        .unwrap_or_default();
                    if positions.is_empty() {
                        RosterShapePlayerInput::unknown(player_key, display_name)
                    } else {
                        RosterShapePlayerInput::known(player_key, display_name, positions)
                    }
                })
                .collect();
            RosterShapeValidationView::validate(RosterShapeValidationInput {
                league: league_name.to_string(),
                team: team.team.clone(),
                shape: shape.clone(),
                players,
            })
        })
        .collect()
}

fn roster_shape_warning(validation: &RosterShapeValidationView) -> ViewWarning {
    let mut fragments = Vec::new();
    if validation.summary.missing_slots > 0 {
        fragments.push(format!(
            "{} missing slot groups",
            validation.summary.missing_slots
        ));
    }
    if validation.summary.overflow_slots > 0 {
        fragments.push(format!(
            "{} over-cap slot groups",
            validation.summary.overflow_slots
        ));
    }
    if validation.summary.unknown_players > 0 {
        fragments.push(format!(
            "{} unknown players",
            validation.summary.unknown_players
        ));
    }
    if validation.summary.ineligible_players > 0 {
        fragments.push(format!(
            "{} ineligible players",
            validation.summary.ineligible_players
        ));
    }
    ViewWarning {
        kind: WarningKind::PartialSource,
        source: Some(SourceKind::FantasyImport),
        message: format!(
            "roster shape '{}' is not satisfied for '{}': {}",
            validation.shape_name,
            validation.team,
            fragments.join(", ")
        ),
        recovery: Vec::new(),
    }
}

fn apply_import(
    db: &FantasyDb,
    options: &FantasyRosterImportOptions,
    existing_league: Option<&LeagueRow>,
    teams: &[FantasyImportTeamInput],
    rows: &[FantasyImportRowInput],
) -> anyhow::Result<()> {
    let has_importable_changes = teams
        .iter()
        .any(|team| team.status == FantasyImportTeamStatus::Created)
        || rows
            .iter()
            .any(|row| row.status == FantasyImportRowStatus::Imported);
    if existing_league.is_none() && !has_importable_changes {
        return Ok(());
    }

    let league_id = if let Some(league) = existing_league {
        league.id.clone()
    } else {
        let id = db.create_league(&options.league_name, &options.scoring_scheme)?;
        db.set_active_league(&options.league_name)?;
        id
    };

    for team in teams {
        if team.status == FantasyImportTeamStatus::Created {
            db.create_team(&league_id, &team.team, team.owner.as_deref().unwrap_or(""))?;
        }
    }
    if let Some(user_team) = options.user_team.as_ref() {
        if teams.iter().any(|team| &team.team == user_team) {
            db.set_user_team(&league_id, user_team)?;
        }
    }

    for row in rows
        .iter()
        .filter(|row| row.status == FantasyImportRowStatus::Imported)
    {
        let Some(team_name) = row.fantasy_team.as_ref() else {
            bail!("import row {} is missing a fantasy team", row.row_number);
        };
        let Some(player_key) = row.normalized_name.as_ref() else {
            bail!(
                "import row {} is missing a normalized player key",
                row.row_number
            );
        };
        let team = db
            .get_team_by_name(&league_id, team_name)?
            .ok_or_else(|| anyhow::anyhow!("team '{team_name}' not found during import apply"))?;
        db.add_player(&team.id, player_key)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("create temp csv");
        file.write_all(content.as_bytes()).expect("write csv");
        file
    }

    fn known(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| normalize_name(name)).collect()
    }

    fn known_positions(names: &[(&str, Vec<Position>)]) -> BTreeMap<String, Vec<Position>> {
        names
            .iter()
            .map(|(name, positions)| (normalize_name(name), positions.clone()))
            .collect()
    }

    #[test]
    fn l1_fantasy_import_parses_bom_and_header_aliases_with_diacritics() {
        let mut content = vec![0xEF, 0xBB, 0xBF];
        content.extend_from_slice(
            "Name,Team Name,Manager,NHL Team,Positions\nJuraj Slafkovský,Alpha,Alice,MTL,LW\n"
                .as_bytes(),
        );
        let mut file = NamedTempFile::new().expect("create temp csv");
        file.write_all(&content).expect("write csv");

        let rows = parse_yahoo_roster_csv(file.path()).expect("parse roster csv");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].player_name, "Juraj Slafkovský");
        assert_eq!(rows[0].fantasy_team, "Alpha");
        assert_eq!(rows[0].owner.as_deref(), Some("Alice"));
        assert_eq!(rows[0].position_hint.as_deref(), Some("LW"));
    }

    #[test]
    fn l1_fantasy_import_dry_run_does_not_mutate_db() {
        let file = write_csv("Player,Fantasy Team,Owner\nConnor McDavid,Alpha,Alice\n");
        let db = FantasyDb::open_in_memory().expect("open db");

        let view = import_yahoo_roster_csv(
            &db,
            file.path(),
            FantasyRosterImportOptions {
                known_player_keys: Some(known(&["Connor McDavid"])),
                ..FantasyRosterImportOptions::dry_run("Office League")
            },
        )
        .expect("dry-run import");

        assert_eq!(view.mode, FantasyImportMode::DryRun);
        assert_eq!(view.summary.teams_created, 1);
        assert_eq!(view.summary.players_imported, 1);
        assert!(
            db.list_leagues().expect("list leagues").is_empty(),
            "dry-run must not create a league"
        );
    }

    #[test]
    fn l1_fantasy_import_apply_creates_league_team_roster_and_user_team() {
        let file = write_csv("Player,Fantasy Team,Owner\nConnor McDavid,Alpha,Alice\n");
        let db = FantasyDb::open_in_memory().expect("open db");

        let mut options = FantasyRosterImportOptions::apply("Office League");
        options.user_team = Some("Alpha".to_string());
        options.known_player_keys = Some(known(&["Connor McDavid"]));
        let view = import_yahoo_roster_csv(&db, file.path(), options).expect("apply import");

        assert_eq!(view.mode, FantasyImportMode::Apply);
        assert_eq!(view.summary.teams_created, 1);
        assert_eq!(view.summary.players_imported, 1);

        let league = db
            .get_active_league()
            .expect("active league query")
            .expect("created league is active");
        let team = db
            .get_team_by_name(&league.id, "Alpha")
            .expect("team query")
            .expect("team exists");
        assert!(team.is_user_team);
        assert_eq!(
            db.list_roster(&team.id).expect("list roster"),
            vec![normalize_name("Connor McDavid")]
        );
    }

    #[test]
    fn l1_fantasy_import_roster_shape_validation_uses_canonical_positions() {
        let file = write_csv("Player,Fantasy Team,Owner,Positions\nConnor McDavid,Alpha,Alice,G\n");
        let db = FantasyDb::open_in_memory().expect("open db");

        let view = import_yahoo_roster_csv(
            &db,
            file.path(),
            FantasyRosterImportOptions {
                known_player_keys: Some(known(&["Connor McDavid"])),
                known_player_positions: Some(known_positions(&[(
                    "Connor McDavid",
                    vec![Position::Center],
                )])),
                ..FantasyRosterImportOptions::dry_run("Office League")
            },
        )
        .expect("dry-run import");

        assert_eq!(view.summary.players_imported, 1);
        assert!(view.warnings.iter().any(|warning| {
            warning.message.contains("roster shape 'yahoo-standard'")
                && warning.message.contains("'Alpha'")
                && warning.message.contains("missing slot groups")
        }));
        assert!(
            !view
                .warnings
                .iter()
                .any(|warning| warning.message.contains("G")),
            "Yahoo CSV position hints must not be treated as canonical validation input"
        );
        assert!(
            db.list_leagues().expect("list leagues").is_empty(),
            "dry-run validation must not create a league"
        );
    }

    #[test]
    fn l1_fantasy_import_roster_shape_validation_includes_existing_rosters() {
        let db = FantasyDb::open_in_memory().expect("open db");
        let league_id = db
            .create_league("Office League", "yahoo-standard")
            .expect("create league");
        let team_id = db
            .create_team(&league_id, "Alpha", "Alice")
            .expect("create team");
        db.add_player(&team_id, "existing_goalie")
            .expect("add existing player");
        let file = write_csv("Player,Fantasy Team,Owner\nConnor McDavid,Alpha,Alice\n");

        let view = import_yahoo_roster_csv(
            &db,
            file.path(),
            FantasyRosterImportOptions {
                known_player_keys: Some(known(&["Connor McDavid"])),
                known_player_positions: Some(known_positions(&[(
                    "Connor McDavid",
                    vec![Position::Center],
                )])),
                ..FantasyRosterImportOptions::dry_run("Office League")
            },
        )
        .expect("dry-run import");

        assert!(
            view.warnings.iter().any(|warning| {
                warning.message.contains("'Alpha'") && warning.message.contains("unknown players")
            }),
            "existing roster members without canonical positions should be surfaced"
        );
    }

    #[test]
    fn l1_fantasy_import_missing_required_column_errors_with_header_name() {
        let file = write_csv("Player,Owner\nConnor McDavid,Alice\n");
        let err = parse_yahoo_roster_csv(file.path()).expect_err("missing fantasy team column");
        assert!(
            err.to_string().contains("fantasy team"),
            "error should name the missing logical header: {err}"
        );
    }

    #[test]
    fn l1_fantasy_import_duplicate_ownership_is_diagnostic_not_mutation() {
        let file = write_csv(
            "Player,Fantasy Team,Owner\nConnor McDavid,Alpha,Alice\nConnor McDavid,Bravo,Bob\n",
        );
        let db = FantasyDb::open_in_memory().expect("open db");

        let view = import_yahoo_roster_csv(
            &db,
            file.path(),
            FantasyRosterImportOptions {
                known_player_keys: Some(known(&["Connor McDavid"])),
                mode: FantasyImportMode::Apply,
                ..FantasyRosterImportOptions::dry_run("Office League")
            },
        )
        .expect("duplicate import returns diagnostics");

        assert_eq!(view.summary.players_duplicate, 2);
        assert_eq!(view.summary.players_imported, 0);
        assert_eq!(view.summary.teams_error, 2);
        assert!(
            db.get_active_league()
                .expect("active league query")
                .is_none(),
            "all-diagnostic apply must not create an empty league"
        );
    }

    #[test]
    fn l1_fantasy_import_unresolved_and_same_team_duplicate_rows_are_diagnostics() {
        let file = write_csv(
            "Player,Fantasy Team,Owner\nUnknown Player,Alpha,Alice\nConnor McDavid,Alpha,Alice\nConnor McDavid,Alpha,Alice\n",
        );
        let db = FantasyDb::open_in_memory().expect("open db");

        let view = import_yahoo_roster_csv(
            &db,
            file.path(),
            FantasyRosterImportOptions {
                known_player_keys: Some(known(&["Connor McDavid"])),
                ..FantasyRosterImportOptions::dry_run("Office League")
            },
        )
        .expect("import diagnostics");

        assert_eq!(view.summary.players_unresolved, 1);
        assert_eq!(view.summary.players_skipped, 1);
        assert_eq!(view.summary.players_imported, 1);
        assert!(view.rows.iter().any(|row| {
            row.player_name == "Unknown Player" && row.status == FantasyImportRowStatus::Unresolved
        }));
        assert!(view.rows.iter().any(|row| {
            row.player_name == "Connor McDavid" && row.status == FantasyImportRowStatus::Skipped
        }));
    }
}
