use anyhow::{Context, Result};
use fletch_core::{
    adapter_handoff_report, dry_run_flight, fetch_plan_with_kind, fetch_to_cache,
    graph_from_registry, validate_registry, CachePolicy, DataFormat, FetchOptions,
    FletchDefinition, FletchRegistry, FreshnessPolicy, GraphNodeKind, SourceKind, SourceSpec,
    FLETCH_REGISTRY_SCHEMA,
};
use icelines_core::stats_catalog::ReportKind;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const REGISTRY_ID: &str = "icelines.fetch-sources";
const WEB_BASE: &str = "https://api-web.nhle.com/v1";
const TEAMS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

pub fn roster_url(team: &str, season: &str) -> String {
    format!("{WEB_BASE}/roster/{team}/{season}")
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FletchSourceHandoffRow {
    pub fetch_surface: String,
    pub fletch_id: String,
    pub season: String,
    pub season_type: String,
    pub source_kind: String,
    pub source_url: String,
    pub cache_targets: String,
    pub mutation_mode: String,
    pub acquisition_mode: String,
    pub activation_rule: String,
    pub icelines_validation_floor: String,
    pub handoff_status: String,
    pub validation_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FletchSourceHandoffReport {
    pub registry_id: String,
    pub registry_valid: bool,
    pub fletch_count: usize,
    pub source_count: usize,
    pub adapter_source_count: usize,
    pub graph_node_count: usize,
    pub graph_edge_count: usize,
    pub flight_step_count: usize,
    pub validation_finding_count: usize,
    pub rows: Vec<FletchSourceHandoffRow>,
}

pub fn fletch_registry_for_season(season: &str, season_type: &str) -> FletchRegistry {
    let mut fletches = Vec::new();
    let mut seen = BTreeSet::new();

    for team in TEAMS {
        push_unique(
            &mut fletches,
            &mut seen,
            source_def(
                format!("icelines.roster.{season}.{team}"),
                "rosters",
                season,
                "regular",
                SourceKind::Http,
                roster_url(team, season),
                format!("snapshots/{season}-<date>-rosters/rosters/{team}.json"),
                "application/json",
                "one team roster JSON",
                "generic-http-cacheline",
                "fletch-acquires-source-icelines-owns-snapshot-seal-and-active-pointer",
                "snapshot is valid only after ICELINES parses every roster and seals the rosters tier",
            ),
        );
    }

    for ty in expanded_season_types(season_type) {
        for kind in ReportKind::all()
            .iter()
            .copied()
            .filter(|kind| kind.is_known_working())
        {
            let url_path = kind.url_path();
            let game_type_id = game_type_id(ty);
            push_unique(
                &mut fletches,
                &mut seen,
                source_def(
                    format!(
                        "icelines.stats.{season}.{ty}.{}",
                        url_path.replace('/', ".")
                    ),
                    "stats-report",
                    season,
                    ty,
                    SourceKind::Adapter,
                    format!(
                        "icelines-nhl-stats-paged://{url_path}?seasonId={season}&gameTypeId={game_type_id}"
                    ),
                    format!("snapshots/<active-stats>/{season}/{ty}/{}.json", url_path.replace('/', "-")),
                    "application/json",
                    "NHL stats API paged report",
                    "adapter-required",
                    "ICELINES currently owns pagination, typed/report-specific parsing, lock policy, and atomic snapshot writes",
                    "download success is not an analytics claim; ICELINES validates report shape, typed rows, chunk manifests, and stat catalog semantics",
                ),
            );
        }
    }

    push_unique(
        &mut fletches,
        &mut seen,
        source_def(
            format!("icelines.moneypuck.{season}.skaters"),
            "moneypuck",
            season,
            "regular",
            SourceKind::Http,
            crate::moneypuck::csv_url(season)
                .unwrap_or_else(|| "icelines-invalid-season://moneypuck".to_string()),
            format!("snapshots/{season}-<date>-moneypuck/moneypuck/moneypuck.json"),
            "text/csv",
            "MoneyPuck season skater CSV",
            "generic-http-cacheline",
            "fletch-acquires-source-icelines-owns-csv-parse-json-snapshot-seal",
            "ICELINES validates CSV headers, player rows, derived percentages, snapshot metadata, and UI semantics",
        ),
    );

    for (id_suffix, surface, source_url, target, activation, validation) in [
        (
            "transactions",
            "transactions",
            format!("icelines-espn-transactions://season/{season}"),
            format!("snapshots/{season}-<date>-transactions/transactions/transactions.json"),
            "ICELINES owns ESPN monthly window expansion, retry/circuit-breaker, prose classification, warnings, and stale flags",
            "ICELINES validates ESPN schema fallback, classifier version, unknown-team handling, and stale metadata",
        ),
        (
            "contracts",
            "contracts",
            "icelines-player-landing-batch://contracts/from-active-stats-bios".to_string(),
            format!("snapshots/{season}-<date>-contracts/contracts/contracts.json"),
            "ICELINES owns active-bios player-set expansion, per-player rate limit, partial skip logging, and snapshot sealing",
            "ICELINES validates player-id coverage and nullable contract field forward-compatibility",
        ),
        (
            "career",
            "career",
            "icelines-player-landing-batch://career/from-active-or-bundled-bios".to_string(),
            "~/.icelines/career_history.json".to_string(),
            "ICELINES owns active/bundled player-set expansion, merge/upsert semantics, rate limit, and skipped-player reporting",
            "ICELINES validates career landing parsing, multi-league history merge, and preserved existing entries",
        ),
        (
            "boxscore",
            "boxscore",
            "icelines-gamecenter-batch://boxscore/from-schedule-date".to_string(),
            "~/.icelines/data/boxscores/<date>/<game_id>.json + EventStream score rows".to_string(),
            "ICELINES owns date schedule expansion, favorite filters, event-stream writes, and derived record payloads",
            "ICELINES validates game identity, score/event payload versioning, and favorite-player intersections",
        ),
        (
            "play-by-play",
            "play-by-play",
            "icelines-gamecenter-batch://play-by-play/from-schedule-date".to_string(),
            "~/.icelines/data/play_by_play/<date>/<game_id>.json".to_string(),
            "ICELINES owns date schedule expansion, favorite filters, raw event persistence, and records eligibility",
            "ICELINES validates event participants, goalie/fight record semantics, and manifest updates",
        ),
    ] {
        push_unique(
            &mut fletches,
            &mut seen,
            source_def(
                format!("icelines.{id_suffix}.{season}"),
                surface,
                season,
                season_type,
                SourceKind::Adapter,
                source_url,
                target,
                "application/json",
                "dynamic source set",
                "adapter-required",
                activation,
                validation,
            ),
        );
    }

    FletchRegistry {
        schema_version: FLETCH_REGISTRY_SCHEMA.to_string(),
        generated_by: "icelines-fetch".to_string(),
        registry_id: REGISTRY_ID.to_string(),
        fletches,
    }
}

pub fn fletch_source_handoff_report(season: &str, season_type: &str) -> FletchSourceHandoffReport {
    let registry = fletch_registry_for_season(season, season_type);
    let requested = registry
        .fletches
        .iter()
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    let validation = validate_registry(&registry);
    let handoff = adapter_handoff_report(&registry, &requested);
    let flight = dry_run_flight(&registry, &requested);
    let graph = graph_from_registry(&registry);

    let mut rows = registry
        .fletches
        .iter()
        .map(|definition| {
            let source = definition.shafts.first();
            let handoff_status = if validation.findings.iter().any(|finding| {
                finding
                    .fletch_id
                    .as_deref()
                    .is_some_and(|id| id == definition.id)
            }) {
                "registry-blocked"
            } else if source.is_some_and(|source| source.kind == SourceKind::Adapter) {
                "adapter-required"
            } else {
                "generic-fetch-ready"
            };
            let validation_status = if required_metadata_present(definition)
                && metadata(definition, "claim_validated_by_download") == "false"
            {
                "pass"
            } else {
                "review"
            };
            FletchSourceHandoffRow {
                fetch_surface: metadata(definition, "fetch_surface"),
                fletch_id: definition.id.clone(),
                season: metadata(definition, "season"),
                season_type: metadata(definition, "season_type"),
                source_kind: source
                    .map(|source| source_kind_label(&source.kind).to_string())
                    .unwrap_or_else(|| "none".to_string()),
                source_url: source.map(|source| source.url.clone()).unwrap_or_default(),
                cache_targets: metadata(definition, "cache_targets"),
                mutation_mode: metadata(definition, "mutation_mode"),
                acquisition_mode: metadata(definition, "acquisition_mode"),
                activation_rule: metadata(definition, "activation_rule"),
                icelines_validation_floor: metadata(definition, "icelines_validation_floor"),
                handoff_status: handoff_status.to_string(),
                validation_status: validation_status.to_string(),
            }
        })
        .collect::<Vec<_>>();

    rows.sort_by(|left, right| {
        left.fetch_surface
            .cmp(&right.fetch_surface)
            .then(left.season_type.cmp(&right.season_type))
            .then(left.fletch_id.cmp(&right.fletch_id))
    });

    FletchSourceHandoffReport {
        registry_id: registry.registry_id.clone(),
        registry_valid: handoff.registry_valid,
        fletch_count: handoff.fletch_count,
        source_count: handoff.source_count,
        adapter_source_count: handoff.adapter_source_count,
        graph_node_count: graph.nodes.len(),
        graph_edge_count: graph.edges.len(),
        flight_step_count: flight.steps.len(),
        validation_finding_count: handoff.validation_finding_count,
        rows,
    }
}

pub fn write_fletch_source_handoff(path: &Path, report: &FletchSourceHandoffReport) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut writer =
        csv::Writer::from_path(path).with_context(|| format!("writing {}", path.display()))?;
    for row in &report.rows {
        writer.serialize(row)?;
    }
    writer.flush()?;
    Ok(())
}

pub fn fletch_source_handoff_gate_failures(report: &FletchSourceHandoffReport) -> Vec<String> {
    let mut failures = Vec::new();
    if !report.registry_valid {
        failures.push("registry validation failed".to_string());
    }
    if report.validation_finding_count > 0 {
        failures.push(format!(
            "{} registry validation finding(s)",
            report.validation_finding_count
        ));
    }
    for row in &report.rows {
        if row.validation_status != "pass" {
            failures.push(format!(
                "{} validation_status={}",
                row.fletch_id, row.validation_status
            ));
        }
        if row.handoff_status == "registry-blocked" {
            failures.push(format!("{} is registry-blocked", row.fletch_id));
        }
    }
    failures
}

pub fn fetch_generic_http_bytes(
    fletch_id: impl Into<String>,
    source_url: impl Into<String>,
    cache_root: &Path,
    force: bool,
) -> Result<Vec<u8>> {
    let fletch_id = fletch_id.into();
    let source_url = source_url.into();
    let mut plan = fetch_plan_with_kind(fletch_id.clone(), source_url, SourceKind::Http)
        .with_context(|| format!("building FLETCH fetch plan for {fletch_id}"))?;
    plan.cache_policy = CachePolicy {
        freshness: FreshnessPolicy::AlwaysCheck,
        allow_offline: true,
        resumable: true,
    };
    plan.tags = vec!["icelines".to_string(), "generic-http-cacheline".to_string()];
    plan.metadata
        .insert("adapter".to_string(), "icelines".to_string());

    let outcome = fetch_to_cache(
        &plan,
        FetchOptions::new(cache_root)
            .with_force(force)
            .with_timeout_ms(30_000)
            .with_retry_attempts(5),
    )
    .with_context(|| format!("fetching {fletch_id} through FLETCH"))?;

    std::fs::read(&outcome.path)
        .with_context(|| format!("reading FLETCH cache object {}", outcome.path.display()))
}

pub async fn fetch_generic_http_bytes_async(
    fletch_id: impl Into<String>,
    source_url: impl Into<String>,
    cache_root: impl Into<std::path::PathBuf>,
    force: bool,
) -> Result<Vec<u8>> {
    let fletch_id = fletch_id.into();
    let source_url = source_url.into();
    let cache_root = cache_root.into();
    tokio::task::spawn_blocking(move || {
        fetch_generic_http_bytes(fletch_id, source_url, &cache_root, force)
    })
    .await
    .context("joining FLETCH fetch task")?
}

fn source_def(
    id: String,
    fetch_surface: &str,
    season: &str,
    season_type: &str,
    source_kind: SourceKind,
    source_url: String,
    cache_targets: String,
    media_type: &str,
    record_shape: &str,
    acquisition_mode: &str,
    activation_rule: &str,
    validation_floor: &str,
) -> FletchDefinition {
    FletchDefinition {
        id,
        node_kind: GraphNodeKind::Fletch,
        shafts: vec![SourceSpec {
            kind: source_kind,
            url: source_url,
            headers: BTreeMap::new(),
        }],
        edges: Vec::new(),
        format: Some(DataFormat {
            media_type: Some(media_type.to_string()),
            encoding: Some("utf-8".to_string()),
            compression: None,
            container: None,
            schema: None,
            record_shape: Some(record_shape.to_string()),
            preferred_local: Some(cache_targets.clone()),
        }),
        tags: vec![
            "icelines".to_string(),
            fetch_surface.to_string(),
            season.to_string(),
            season_type.to_string(),
        ],
        metadata: BTreeMap::from([
            ("fetch_surface".to_string(), fetch_surface.to_string()),
            ("season".to_string(), season.to_string()),
            ("season_type".to_string(), season_type.to_string()),
            ("cache_targets".to_string(), cache_targets),
            ("mutation_mode".to_string(), acquisition_mode.to_string()),
            ("acquisition_mode".to_string(), acquisition_mode.to_string()),
            ("activation_rule".to_string(), activation_rule.to_string()),
            (
                "icelines_validation_floor".to_string(),
                validation_floor.to_string(),
            ),
            (
                "claim_validated_by_download".to_string(),
                "false".to_string(),
            ),
        ]),
    }
}

fn push_unique(
    fletches: &mut Vec<FletchDefinition>,
    seen: &mut BTreeSet<String>,
    fletch: FletchDefinition,
) {
    if seen.insert(fletch.id.clone()) {
        fletches.push(fletch);
    }
}

fn expanded_season_types(season_type: &str) -> Vec<&'static str> {
    match season_type {
        "playoff" => vec!["playoff"],
        "both" => vec!["regular", "playoff"],
        _ => vec!["regular"],
    }
}

fn game_type_id(season_type: &str) -> u8 {
    match season_type {
        "playoff" => 3,
        _ => 2,
    }
}

fn required_metadata_present(definition: &FletchDefinition) -> bool {
    [
        "fetch_surface",
        "season",
        "season_type",
        "cache_targets",
        "mutation_mode",
        "acquisition_mode",
        "activation_rule",
        "icelines_validation_floor",
    ]
    .iter()
    .all(|key| {
        definition
            .metadata
            .get(*key)
            .is_some_and(|value| !value.is_empty())
    })
}

fn metadata(definition: &FletchDefinition, key: &str) -> String {
    definition.metadata.get(key).cloned().unwrap_or_default()
}

fn source_kind_label(kind: &SourceKind) -> &'static str {
    match kind {
        SourceKind::Http => "http",
        SourceKind::File => "file",
        SourceKind::Adapter => "adapter",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[test]
    fn registry_marks_rosters_and_moneypuck_generic_http() {
        let report = fletch_source_handoff_report("20252026", "regular");
        assert!(report.registry_valid);
        assert_eq!(
            fletch_source_handoff_gate_failures(&report),
            Vec::<String>::new()
        );
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.roster.20252026.EDM"
                && row.source_kind == "http"
                && row.handoff_status == "generic-fetch-ready"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.moneypuck.20252026.skaters"
                && row.source_kind == "http"
                && row.handoff_status == "generic-fetch-ready"
        }));
    }

    #[test]
    fn registry_keeps_paged_and_dynamic_sources_adapter_owned() {
        let report = fletch_source_handoff_report("20252026", "regular");
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.stats.20252026.regular.skater.summary"
                && row.source_kind == "adapter"
                && row.acquisition_mode == "adapter-required"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.transactions.20252026"
                && row.source_kind == "adapter"
                && row.handoff_status == "adapter-required"
        }));
    }

    #[test]
    fn both_season_type_expands_stats_windows_without_duplicate_regular_only_sources() {
        let report = fletch_source_handoff_report("20252026", "both");
        assert!(report
            .rows
            .iter()
            .any(|row| row.fletch_id == "icelines.stats.20252026.regular.skater.summary"));
        assert!(report
            .rows
            .iter()
            .any(|row| row.fletch_id == "icelines.stats.20252026.playoff.skater.summary"));
        assert_eq!(
            report
                .rows
                .iter()
                .filter(|row| row.fletch_id == "icelines.moneypuck.20252026.skaters")
                .count(),
            1
        );
    }

    #[test]
    fn fetch_generic_http_bytes_uses_fletch_cache_object() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/source.csv");
            then.status(200)
                .header("content-type", "text/csv")
                .body("playerId,situation,icetime\n8478402,all,5000\n");
        });
        let dir = tempfile::tempdir().unwrap();

        let bytes = fetch_generic_http_bytes(
            "icelines.test.source",
            server.url("/source.csv"),
            dir.path(),
            false,
        )
        .expect("FLETCH should fetch mock source");

        assert_eq!(
            bytes,
            b"playerId,situation,icetime\n8478402,all,5000\n".to_vec()
        );
        mock.assert_hits(1);
        assert!(
            dir.path().join("objects").join("sha256").exists(),
            "FLETCH object store should be populated"
        );
    }

    #[tokio::test]
    async fn fetch_generic_http_bytes_async_runs_blocking_fetch_off_runtime() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/roster/EDM/20252026");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"forwards":[],"defensemen":[],"goalies":[]}"#);
        });
        let dir = tempfile::tempdir().unwrap();

        let bytes = fetch_generic_http_bytes_async(
            "icelines.roster.20252026.EDM",
            server.url("/roster/EDM/20252026"),
            dir.path().to_path_buf(),
            false,
        )
        .await
        .expect("async wrapper should fetch through spawn_blocking");

        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            serde_json::json!({"forwards":[],"defensemen":[],"goalies":[]})
        );
        mock.assert_hits(1);
    }
}
