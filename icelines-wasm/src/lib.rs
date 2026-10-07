//! Browser bindings around the shared normalized IceLines engine.
//! This crate takes bytes, never paths or network handles. One worker owns it.
#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet, VecDeque};

use chrono::NaiveDate;
use icelines_core::identity::PlayerId;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::{PlayerView, StatsRepository};
use icelines_data::{
    build_goalie_identity, build_goalie_season_stats, build_identity, build_skater_stats,
};
use icelines_query::{
    DataProvider, EvalCtx, FetchError, FetchEvent, FilterInput, PlanRequirement, StrictMode,
};
use icelines_sources::schema::{GoalieStats, SkaterBio, SkaterStats};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use wasm_bindgen::prelude::*;

mod package_limits;

const MAX_PACKAGE_BYTES: usize = 100 * 1024 * 1024;
const MAX_RESIDENT_WINDOWS: usize = 8;
const MAX_RESIDENT_INPUT_BYTES: usize = 64 * 1024 * 1024;

/// Versioned interchange envelope. Missing optional reports stay unavailable.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeasonPackage {
    pub schema_version: u32,
    pub season: u32,
    pub season_type: SeasonType,
    pub source: String,
    pub observed_at: Option<String>,
    pub fetched_at: Option<String>,
    pub bios: Vec<SkaterBio>,
    pub stats: Vec<SkaterStats>,
    #[serde(default)]
    pub goalies: Vec<GoalieStats>,
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("invalid package: {0}")]
    InvalidPackage(String),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("missing data: {0}")]
    MissingData(String),
    #[error(transparent)]
    Normalize(#[from] icelines_data::NormalizationError),
    #[error(transparent)]
    Repository(#[from] icelines_core::stats_repository::RepoError),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryRequest {
    pub filter: String,
    pub sort: String,
    pub goalies: bool,
    #[serde(default)]
    pub minimum_games: Option<u32>,
    pub today: String,
}

#[derive(Debug, Serialize)]
pub struct PlayerRow {
    pub player_id: u32,
    pub name: String,
    pub team: String,
    pub position: String,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub pace_82: Option<f64>,
    pub save_pct: Option<f64>,
    pub gaa: Option<f64>,
    pub wins: Option<u32>,
    pub hits: Option<u32>,
    pub blocked_shots: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub schema_version: u32,
    pub season: u32,
    pub season_type: SeasonType,
    pub revision: String,
    pub source: String,
    pub observed_at: Option<String>,
    pub fetched_at: Option<String>,
    pub missing_sources: Vec<String>,
    pub minimum_games: u32,
    pub pace_minimum_games: u32,
    pub rows: Vec<PlayerRow>,
}

#[derive(Debug, Serialize)]
struct PackageMetadata<'a> {
    schema_version: u32,
    season: u32,
    season_type: &'a SeasonType,
    source: &'a str,
    observed_at: &'a Option<String>,
    fetched_at: &'a Option<String>,
}

struct PreparedProvider;
impl DataProvider for PreparedProvider {
    fn ensure(&self, _: &PlanRequirement, _: &mut dyn FnMut(FetchEvent)) -> Result<(), FetchError> {
        Err(FetchError::DataMissingNoFetch {
            what: "browser queries require prepared data".into(),
        })
    }
}

struct ActivePackage {
    package: SeasonPackage,
    repository: StatsRepository,
    revision: String,
    input_bytes: usize,
}

#[wasm_bindgen]
pub struct BrowserEngine {
    active: Option<ActivePackage>,
    resident: VecDeque<ActivePackage>,
}

impl Default for BrowserEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl BrowserEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            active: None,
            resident: VecDeque::new(),
        }
    }

    /// Stage and validate before replacing the active repository.
    pub fn load_package(&mut self, bytes: &[u8]) -> Result<String, JsValue> {
        self.load_bytes(bytes)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn query(&self, request: &str) -> Result<String, JsValue> {
        let request: QueryRequest = serde_json::from_str(request)
            .map_err(|e| JsValue::from_str(&format!("invalid query: {e}")))?;
        let result = self
            .execute(&request)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Return the validated active header without decoding the raw package again.
    pub fn package_metadata(&self) -> Result<String, JsValue> {
        let metadata = self
            .active_metadata()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&metadata).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn export_package(&self) -> Result<Vec<u8>, JsValue> {
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| JsValue::from_str("missing data: load a package"))?;
        serde_json::to_vec(&active.package).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn current_season(&self) -> u32 {
        icelines_core::CURRENT_SEASON
    }

    /// Select an already validated window by exact revision; never fetch or save.
    pub fn select_resident(&mut self, revision: &str) -> Result<(), JsValue> {
        self.select_revision(revision)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn resident_revisions(&self) -> String {
        serde_json::to_string(
            &self
                .resident
                .iter()
                .chain(self.active.iter())
                .map(|window| &window.revision)
                .collect::<Vec<_>>(),
        )
        .expect("revision strings serialize")
    }

    pub fn unload_active(&mut self) {
        self.active = None;
    }

    /// Stateless projection: schedule acquisition never replaces season stats.
    pub fn schedule_week(&self, bytes: &[u8]) -> Result<String, JsValue> {
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(JsValue::from_str("Schedule response exceeds 2 MiB"));
        }
        let raw: serde_json::Value = serde_json::from_slice(bytes)
            .map_err(|e| JsValue::from_str(&format!("invalid schedule: {e}")))?;
        let games = icelines_sources::nhl::schedule::parse_game_week(&raw)
            .map_err(|e| JsValue::from_str(&e))?;
        serde_json::to_string(&games).map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn player_detail(&self, player_id: u32) -> Result<String, JsValue> {
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| JsValue::from_str("missing data: load a package"))?;
        let view = active
            .repository
            .league(Season(active.package.season), active.package.season_type)
            .find(|view| view.id().0 == player_id)
            .ok_or_else(|| JsValue::from_str("missing data: player absent in this season"))?;
        serde_json::to_string(&serde_json::json!({
            "schema_version": 1, "revision": active.revision,
            "season": active.package.season, "season_type": active.package.season_type,
            "identity": view.identity, "stats": view.stats,
        }))
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl BrowserEngine {
    fn active_metadata(&self) -> Result<PackageMetadata<'_>, EngineError> {
        let package = &self
            .active
            .as_ref()
            .ok_or_else(|| EngineError::MissingData("load a package".into()))?
            .package;
        Ok(PackageMetadata {
            schema_version: package.schema_version,
            season: package.season,
            season_type: &package.season_type,
            source: &package.source,
            observed_at: &package.observed_at,
            fetched_at: &package.fetched_at,
        })
    }

    pub fn load_bytes(&mut self, bytes: &[u8]) -> Result<String, EngineError> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(EngineError::InvalidPackage(
                "expanded package exceeds 100 MiB".into(),
            ));
        }
        let revision = format!("{:x}", Sha256::digest(bytes));
        if self.select_revision(&revision).is_ok() {
            return Ok(revision);
        }
        package_limits::validate(bytes).map_err(EngineError::InvalidPackage)?;
        let package: SeasonPackage = serde_json::from_slice(bytes)
            .map_err(|e| EngineError::InvalidPackage(e.to_string()))?;
        if package.schema_version != 1 {
            return Err(EngineError::InvalidPackage(
                "unsupported schema version".into(),
            ));
        }
        let season = Season::try_new(package.season)
            .map_err(|e| EngineError::InvalidPackage(e.to_string()))?;
        if package.source.trim().is_empty()
            || (package.bios.is_empty() && package.goalies.is_empty())
        {
            return Err(EngineError::InvalidPackage(
                "source and player records are required".into(),
            ));
        }
        for timestamp in [&package.observed_at, &package.fetched_at]
            .into_iter()
            .flatten()
        {
            chrono::DateTime::parse_from_rfc3339(timestamp)
                .map_err(|e| EngineError::InvalidPackage(format!("invalid timestamp: {e}")))?;
        }
        let mismatch = package
            .bios
            .iter()
            .filter_map(|r| r.season_id)
            .chain(package.stats.iter().filter_map(|r| r.season_id))
            .chain(package.goalies.iter().map(|r| r.season_id))
            .find(|s| *s != season.0);
        if let Some(found) = mismatch {
            return Err(EngineError::InvalidPackage(format!(
                "requested season {}, found {found}",
                season.0
            )));
        }
        let stats: HashMap<u32, &SkaterStats> =
            package.stats.iter().map(|s| (s.player_id, s)).collect();
        let mut repo = StatsRepository::with_lru_cap(8);
        let mut seen = HashSet::new();
        for bio in package
            .bios
            .iter()
            .rev()
            .filter(|b| seen.insert(b.player_id))
        {
            let Some(position) = Position::from_api_code(&bio.position_code) else {
                continue;
            };
            if position == Position::Goalie {
                continue;
            }
            let id = PlayerId(bio.player_id);
            repo.upsert_identity(build_identity(id, bio))?;
            repo.upsert_stats(build_skater_stats(
                id,
                season,
                package.season_type,
                position,
                bio,
                stats.get(&bio.player_id).copied(),
                None,
                None,
                None,
            )?)?;
        }
        for goalie in &package.goalies {
            let id = PlayerId(goalie.player_id);
            if repo.identity(id).is_none() {
                repo.upsert_identity(build_goalie_identity(goalie))?;
            }
            repo.upsert_stats(build_goalie_season_stats(
                id,
                season,
                package.season_type,
                goalie,
            )?)?;
        }
        let next = ActivePackage {
            package,
            repository: repo,
            revision: revision.clone(),
            input_bytes: bytes.len(),
        };
        // One revision per season/type window. A refresh replaces that window
        // only after complete normalization, preserving the last good revision
        // and its LRU order on invalid input.
        self.resident
            .retain(|window| !Self::same_window(window, &next));
        if let Some(previous) = self.active.take() {
            if !Self::same_window(&previous, &next) {
                self.resident.push_back(previous);
            }
        }
        self.active = Some(next);
        self.trim_resident();
        Ok(revision)
    }

    fn same_window(left: &ActivePackage, right: &ActivePackage) -> bool {
        left.package.season == right.package.season
            && left.package.season_type == right.package.season_type
    }

    fn trim_resident(&mut self) {
        while !self.resident.is_empty()
            && (self.resident.len() + usize::from(self.active.is_some()) > MAX_RESIDENT_WINDOWS
                || self
                    .resident
                    .iter()
                    .chain(self.active.iter())
                    .map(|window| window.input_bytes)
                    .sum::<usize>()
                    > MAX_RESIDENT_INPUT_BYTES)
        {
            self.resident.pop_front();
        }
    }

    fn select_revision(&mut self, revision: &str) -> Result<(), EngineError> {
        if self
            .active
            .as_ref()
            .is_some_and(|window| window.revision == revision)
        {
            return Ok(());
        }
        let index = self
            .resident
            .iter()
            .position(|window| window.revision == revision)
            .ok_or_else(|| {
                EngineError::MissingData("season window was evicted; reload its package".into())
            })?;
        let next = self
            .resident
            .remove(index)
            .expect("located resident window");
        if let Some(previous) = self.active.take() {
            self.resident.push_back(previous);
        }
        self.active = Some(next);
        self.trim_resident();
        Ok(())
    }

    pub fn execute(&self, request: &QueryRequest) -> Result<QueryResult, EngineError> {
        let active = self
            .active
            .as_ref()
            .ok_or_else(|| EngineError::MissingData("load a season package".into()))?;
        let package = &active.package;
        let today = NaiveDate::parse_from_str(&request.today, "%Y-%m-%d")
            .map_err(|e| EngineError::InvalidQuery(e.to_string()))?;
        let plan = if request.filter.trim().is_empty() {
            None
        } else {
            Some(
                icelines_query::parse_query(FilterInput::Form(request.filter.clone()))
                    .map_err(|e| EngineError::InvalidQuery(format!("{e:?}")))?,
            )
        };
        if plan.as_ref().is_some_and(|p| p.root.needs_provider()) {
            return Err(EngineError::MissingData(
                "this query requires game logs or career data; season packages do not contain them"
                    .into(),
            ));
        }
        if ![
            "points", "goals", "assists", "gp", "pace", "wins", "save_pct", "gaa",
        ]
        .contains(&request.sort.as_str())
        {
            return Err(EngineError::InvalidQuery("unsupported sort".into()));
        }
        if (request.goalies
            && ["points", "goals", "assists", "pace"].contains(&request.sort.as_str()))
            || (!request.goalies && ["wins", "save_pct", "gaa"].contains(&request.sort.as_str()))
        {
            return Err(EngineError::InvalidQuery(
                "sort does not apply to this player type".into(),
            ));
        }
        let provider = PreparedProvider;
        let ctx = EvalCtx::new(&provider, StrictMode::Off, true, today, package.season);
        let minimum_games = request.minimum_games.unwrap_or_else(|| {
            if request.goalies {
                icelines_core::view_model::goalies::qualified_goalie_games(package.season_type)
            } else {
                0
            }
        });
        let views: Vec<PlayerView<'_>> = if request.goalies {
            active
                .repository
                .goalies(Season(package.season), package.season_type)
                .collect()
        } else {
            active
                .repository
                .skaters(Season(package.season), package.season_type)
                .collect()
        };
        let mut views: Vec<PlayerView<'_>> = views
            .into_iter()
            .filter(|view| view.gp() >= minimum_games)
            .filter(|view| plan.as_ref().is_none_or(|p| p.root.matches(view, &ctx)))
            .collect();
        if request.goalies {
            let sort = icelines_core::GoalieLeaderboardSort::from_key(&request.sort)
                .ok_or_else(|| EngineError::InvalidQuery("unsupported goalie sort".into()))?;
            views.sort_by(|a, b| sort.compare_player_views(a, b));
        } else if request.sort == "pace" {
            icelines_core::sort_views_by_pace(&mut views);
        } else {
            let metric = icelines_core::stats_catalog::StatId::from_cli_key(&request.sort)
                .ok_or_else(|| EngineError::InvalidQuery("unsupported skater sort".into()))?;
            views.sort_by(|a, b| metric.sort_cmp(a, b));
        }
        let rows: Vec<PlayerRow> = views
            .into_iter()
            .map(|view| {
                let goalie = view.stats.goalie.as_ref();
                PlayerRow {
                    player_id: view.identity.id.0,
                    name: view.identity.full_name.clone(),
                    team: view.team_display().to_owned(),
                    position: format!("{:?}", view.position()),
                    gp: view.gp(),
                    goals: view.goals(),
                    assists: view.assists(),
                    points: view.points(),
                    pace_82: view.pace_82(),
                    save_pct: goalie.and_then(|g| g.save_pct).map(f64::from),
                    gaa: goalie.and_then(|g| g.goals_against_average).map(f64::from),
                    wins: goalie.map(|g| g.wins),
                    hits: view.hits(),
                    blocked_shots: view.blocked_shots(),
                }
            })
            .collect();
        Ok(QueryResult {
            schema_version: 1,
            season: package.season,
            season_type: package.season_type,
            revision: active.revision.clone(),
            source: package.source.clone(),
            observed_at: package.observed_at.clone(),
            fetched_at: package.fetched_at.clone(),
            missing_sources: vec![
                "realtime".into(),
                "MoneyPuck".into(),
                "contracts".into(),
                "time on ice reports".into(),
            ],
            minimum_games,
            pace_minimum_games: icelines_core::model::MIN_GP,
            rows,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package() -> serde_json::Value {
        // Hand-audited 82 GP, 20 G + 30 A = 50 P, 50 P/82 pace.
        // Bio's current team differs from the season stats on purpose.
        serde_json::json!({
            "schema_version": 1, "season": 20252026, "season_type": "regular",
            "source": "fixture", "observed_at": null, "fetched_at": null,
            "bios": [{"playerId": 1, "skaterFullName": "Juraj Slafkovský",
                "lastName": "Slafkovský", "positionCode": "L", "currentTeamAbbrev": "MTL",
                "gamesPlayed": 82, "goals": 20, "assists": 30, "points": 50}],
            "stats": [{"playerId": 1, "seasonId": 20252026, "teamAbbrevs": "SEA,NYR",
                "gamesPlayed": 82, "goals": 20, "assists": 30, "points": 50,
                "pointsPerGame": 0.609756, "ppGoals": 0, "ppPoints": 0,
                "shGoals": 0, "shPoints": 0, "gameWinningGoals": 0,
                "otGoals": 0, "shots": 0, "plusMinus": 0}], "goalies": []
        })
    }

    fn season_package(season: u32) -> serde_json::Value {
        let mut value = package();
        value["season"] = season.into();
        value["stats"][0]["seasonId"] = season.into();
        value
    }

    #[test]
    fn resident_windows_are_lru_bounded_and_missing_selection_preserves_active() {
        let mut engine = BrowserEngine::new();
        let mut revisions = Vec::new();
        for year in 2010..2019 {
            revisions.push(load(&mut engine, &season_package(year * 10000 + year + 1)).unwrap());
        }
        assert_eq!(engine.resident.len(), 7);
        assert!(engine.select_revision(&revisions[0]).is_err());
        assert_eq!(engine.execute(&request("")).unwrap().revision, revisions[8]);
        engine.select_revision(&revisions[1]).unwrap();
        assert_eq!(engine.execute(&request("")).unwrap().season, 20112012);
        load(&mut engine, &season_package(20192020)).unwrap();
        assert!(engine.select_revision(&revisions[2]).is_err());
        engine.select_revision(&revisions[1]).unwrap();
        assert_eq!(engine.execute(&request("")).unwrap().rows[0].points, 50);
    }

    #[test]
    fn resident_refresh_replaces_only_same_window_after_validation() {
        let mut engine = BrowserEngine::new();
        let regular = load(&mut engine, &package()).unwrap();
        let mut playoff_package = package();
        playoff_package["season_type"] = "playoff".into();
        let playoff = load(&mut engine, &playoff_package).unwrap();
        let before = engine.resident_revisions();
        let mut invalid = package();
        invalid["schema_version"] = 2.into();
        assert!(load(&mut engine, &invalid).is_err());
        assert_eq!(engine.resident_revisions(), before);
        let mut refresh = package();
        refresh["fetched_at"] = "2026-10-04T00:00:00Z".into();
        let refreshed = load(&mut engine, &refresh).unwrap();
        assert_ne!(refreshed, regular);
        assert!(engine.select_revision(&regular).is_err());
        engine.select_revision(&playoff).unwrap();
        assert_eq!(
            engine.execute(&request("")).unwrap().season_type,
            SeasonType::Playoff
        );
        assert_eq!(engine.resident.len(), 1);
    }

    #[test]
    fn unload_removes_selected_window_but_keeps_other_resident_seasons() {
        let mut engine = BrowserEngine::new();
        let first = load(&mut engine, &season_package(20232024)).unwrap();
        let second = load(&mut engine, &season_package(20242025)).unwrap();
        engine.unload_active();
        assert!(engine.execute(&request("")).is_err());
        assert!(engine.select_revision(&second).is_err());
        engine.select_revision(&first).unwrap();
        assert_eq!(engine.execute(&request("")).unwrap().season, 20232024);
    }

    #[test]
    fn resident_input_budget_evicts_older_windows_and_keeps_large_active_alone() {
        let mut engine = BrowserEngine::new();
        load(&mut engine, &season_package(20232024)).unwrap();
        load(&mut engine, &season_package(20242025)).unwrap();
        // Exercise accounting at the boundary without allocating giant JSON.
        engine.resident[0].input_bytes = MAX_RESIDENT_INPUT_BYTES;
        engine.trim_resident();
        assert!(engine.resident.is_empty());
        load(&mut engine, &season_package(20252026)).unwrap();
        engine.active.as_mut().unwrap().input_bytes = MAX_RESIDENT_INPUT_BYTES + 1;
        engine.trim_resident();
        assert!(engine.resident.is_empty());
        assert_eq!(engine.execute(&request("")).unwrap().season, 20252026);
    }

    #[test]
    fn resource_rejection_preserves_active_data_and_resident_order() {
        let mut engine = BrowserEngine::new();
        load(&mut engine, &season_package(20232024)).unwrap();
        load(&mut engine, &season_package(20242025)).unwrap();
        let revisions = engine.resident_revisions();
        let before = serde_json::to_value(engine.execute(&request("")).unwrap()).unwrap();
        let mut oversized_name = package();
        oversized_name["bios"][0]["skaterFullName"] = "x".repeat(1025).into();
        let mut oversized_metadata = package();
        oversized_metadata["source"] = "x".repeat(1025).into();
        let mut oversized_rows = package();
        oversized_rows["bios"] = vec![package()["bios"][0].clone(); 10_001].into();
        for invalid in [oversized_name, oversized_metadata, oversized_rows] {
            let error = load(&mut engine, &invalid).unwrap_err().to_string();
            assert!(
                error.contains("package string exceeds") || error.contains("package array exceeds"),
                "{error}"
            );
            assert_eq!(engine.resident_revisions(), revisions);
            assert_eq!(
                serde_json::to_value(engine.execute(&request("")).unwrap()).unwrap(),
                before
            );
        }
    }
    fn request(filter: &str) -> QueryRequest {
        QueryRequest {
            filter: filter.into(),
            sort: "points".into(),
            goalies: false,
            minimum_games: Some(0),
            today: "2026-10-03".into(),
        }
    }

    fn load(engine: &mut BrowserEngine, value: &serde_json::Value) -> Result<String, EngineError> {
        engine.load_bytes(&serde_json::to_vec(value).unwrap())
    }

    #[test]
    fn metadata_tracks_validated_selection_without_player_arrays() {
        let mut engine = BrowserEngine::new();
        assert!(engine.active_metadata().is_err());
        let original = package();
        let first = load(&mut engine, &original).unwrap();
        let expected = |package: &serde_json::Value| {
            let mut header = package.clone();
            for key in ["bios", "stats", "goalies"] {
                header.as_object_mut().unwrap().remove(key);
            }
            header
        };
        assert_eq!(
            serde_json::to_value(engine.active_metadata().unwrap()).unwrap(),
            expected(&original)
        );
        let mut replacement = package();
        replacement["source"] = "Replacement source".into();
        replacement["season_type"] = "playoff".into();
        load(&mut engine, &replacement).unwrap();
        assert_eq!(
            serde_json::to_value(engine.active_metadata().unwrap()).unwrap(),
            expected(&replacement)
        );
        let mut invalid = replacement.clone();
        invalid["schema_version"] = 2.into();
        assert!(load(&mut engine, &invalid).is_err());
        assert_eq!(
            serde_json::to_value(engine.active_metadata().unwrap()).unwrap(),
            expected(&replacement)
        );
        engine.select_revision(&first).unwrap();
        assert_eq!(
            serde_json::to_value(engine.active_metadata().unwrap()).unwrap(),
            expected(&original)
        );
        engine.unload_active();
        assert!(engine.active_metadata().is_err());
    }

    #[test]
    fn shared_filter_normalization_and_missing_values() {
        let mut engine = BrowserEngine::new();
        load(&mut engine, &package()).unwrap();
        let result = engine.execute(&request("p>=50 AND gp=82")).unwrap();
        assert_eq!(result.rows.len(), 1);
        let row = &result.rows[0];
        assert_eq!(row.name, "Juraj Slafkovský");
        assert_eq!(row.team, "SEA/NYR");
        assert_eq!(row.points, 50);
        assert_eq!(row.pace_82, Some(50.0));
        assert_eq!(row.hits, None);
        assert_eq!(row.save_pct, None);
        assert!(engine.execute(&request("p>50")).unwrap().rows.is_empty());
    }

    #[test]
    fn count_sorts_use_identity_ties_without_secondary_points_or_names() {
        let mut fixture = package();
        let bio = fixture["bios"][0].clone();
        let stat = fixture["stats"][0].clone();
        let mut bios = Vec::new();
        let mut stats = Vec::new();
        for (id, name, gp, goals, assists) in [
            (2, "Alpha", 82, 20, 30),
            (1, "Zed", 82, 20, 30),
            (3, "Beta", 80, 20, 50),
        ] {
            let mut b = bio.clone();
            let mut s = stat.clone();
            for row in [&mut b, &mut s] {
                row["playerId"] = id.into();
                row["gamesPlayed"] = gp.into();
                row["goals"] = goals.into();
                row["assists"] = assists.into();
                row["points"] = (goals + assists).into();
            }
            b["skaterFullName"] = name.into();
            bios.push(b);
            stats.push(s);
        }
        fixture["bios"] = bios.into();
        fixture["stats"] = stats.into();
        let mut engine = BrowserEngine::new();
        load(&mut engine, &fixture).unwrap();
        for (sort, expected) in [
            ("points", vec![3, 1, 2]),
            ("goals", vec![1, 2, 3]),
            ("assists", vec![3, 1, 2]),
            ("gp", vec![1, 2, 3]),
        ] {
            let mut query = request("");
            query.sort = sort.into();
            assert_eq!(
                engine
                    .execute(&query)
                    .unwrap()
                    .rows
                    .iter()
                    .map(|row| row.player_id)
                    .collect::<Vec<_>>(),
                expected,
                "sort={sort}"
            );
        }
    }

    #[test]
    fn failed_replacement_keeps_previous_revision() {
        let mut engine = BrowserEngine::new();
        let revision = load(&mut engine, &package()).unwrap();
        let mut invalid = package();
        invalid["stats"][0]["seasonId"] = serde_json::json!(20242025);
        assert!(load(&mut engine, &invalid).is_err());
        assert_eq!(engine.execute(&request("")).unwrap().revision, revision);
        invalid = package();
        invalid["schema_version"] = serde_json::json!(99);
        assert!(load(&mut engine, &invalid).is_err());
        assert_eq!(engine.execute(&request("")).unwrap().revision, revision);
    }

    #[test]
    fn game_logs_are_required_instead_of_silent_empty_results() {
        let mut engine = BrowserEngine::new();
        load(&mut engine, &package()).unwrap();
        assert!(matches!(
            engine.execute(&request("g.last5g>=1")),
            Err(EngineError::MissingData(_))
        ));
    }

    #[test]
    fn season_type_and_zero_games_survive_normalization() {
        let mut engine = BrowserEngine::new();
        let mut value = package();
        value["season_type"] = serde_json::json!("playoff");
        value["stats"][0]["gamesPlayed"] = serde_json::json!(0);
        value["stats"][0]["goals"] = serde_json::json!(0);
        value["stats"][0]["assists"] = serde_json::json!(0);
        value["stats"][0]["points"] = serde_json::json!(0);
        load(&mut engine, &value).unwrap();
        let result = engine.execute(&request("")).unwrap();
        assert_eq!(result.season_type, SeasonType::Playoff);
        assert_eq!(result.rows[0].gp, 0);
        assert_eq!(result.rows[0].pace_82, None);
    }

    #[test]
    fn pace_uses_shared_goal_tiebreak_and_ten_game_boundary() {
        let mut value = package();
        let bio = value["bios"][0].clone();
        let stats = value["stats"][0].clone();
        let mut bios = Vec::new();
        let mut reports = Vec::new();
        // Equal pace at 10 GP: more goals wins, then stable ID. A 9-GP
        // high scorer stays present but its guarded pace is unavailable.
        for (id, gp, goals, assists) in
            [(1, 10, 2, 8), (2, 10, 6, 4), (3, 10, 6, 4), (4, 9, 20, 20)]
        {
            let mut b = bio.clone();
            b["playerId"] = serde_json::json!(id);
            let mut s = stats.clone();
            s["playerId"] = serde_json::json!(id);
            s["gamesPlayed"] = serde_json::json!(gp);
            s["goals"] = serde_json::json!(goals);
            s["assists"] = serde_json::json!(assists);
            s["points"] = serde_json::json!(goals + assists);
            bios.push(b);
            reports.push(s);
        }
        value["bios"] = serde_json::json!(bios);
        value["stats"] = serde_json::json!(reports);
        let mut engine = BrowserEngine::new();
        load(&mut engine, &value).unwrap();
        let mut query = request("");
        query.sort = "pace".into();
        let result = engine.execute(&query).unwrap();
        assert_eq!(
            result.rows.iter().map(|r| r.player_id).collect::<Vec<_>>(),
            vec![2, 3, 1, 4]
        );
        assert_eq!(result.rows[0].pace_82, Some(82.0));
        assert_eq!(result.rows[3].pace_82, None);
        assert_eq!(result.pace_minimum_games, 10);
        query.minimum_games = Some(10);
        assert_eq!(engine.execute(&query).unwrap().rows.len(), 3);
    }

    #[test]
    fn goalie_default_floor_is_contextual_and_zero_is_an_override() {
        let mut engine = BrowserEngine::new();
        let mut value = package();
        let mut query = request("");
        query.goalies = true;
        query.sort = "save_pct".into();
        query.minimum_games = None;
        load(&mut engine, &value).unwrap();
        assert_eq!(engine.execute(&query).unwrap().minimum_games, 5);
        value["season_type"] = serde_json::json!("playoff");
        load(&mut engine, &value).unwrap();
        assert_eq!(engine.execute(&query).unwrap().minimum_games, 1);
        query.minimum_games = Some(0);
        assert_eq!(engine.execute(&query).unwrap().minimum_games, 0);
        let mut wire = serde_json::to_value(&query).unwrap();
        assert_eq!(
            serde_json::from_value::<QueryRequest>(wire.clone())
                .unwrap()
                .minimum_games,
            Some(0)
        );
        wire.as_object_mut().unwrap().remove("minimum_games");
        assert_eq!(
            serde_json::from_value::<QueryRequest>(wire.clone())
                .unwrap()
                .minimum_games,
            None
        );
        wire["minimum_games"] = serde_json::Value::Null;
        assert_eq!(
            serde_json::from_value::<QueryRequest>(wire)
                .unwrap()
                .minimum_games,
            None
        );
    }

    #[test]
    fn nullable_timestamps_are_unknown_and_invalid_timestamps_fail() {
        let mut engine = BrowserEngine::new();
        let mut value = package();
        load(&mut engine, &value).unwrap();
        assert_eq!(engine.execute(&request("")).unwrap().observed_at, None);
        value["observed_at"] = serde_json::json!("yesterday");
        assert!(load(&mut engine, &value).is_err());
    }
}
