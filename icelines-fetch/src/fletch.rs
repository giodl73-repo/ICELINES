use crate::{atomic_write::write_bytes_atomic, schema::RawTransaction};
use anyhow::{Context, Result};
use fletch_core::{
    adapter_handoff_report, cache_index_from_manifest, cache_index_gate_report, dry_run_flight,
    fetch_batch_to_cache_best_effort, fetch_batch_to_cache_best_effort_with_delay,
    fetch_paged_json_to_cache, fetch_plan_with_kind, fetch_to_cache, graph_from_registry,
    read_cache_manifest_json, upsert_cache_manifest_entries, validate_registry, CacheEntry,
    CacheIndexGatePolicy, CacheManifest, CachePolicy, DataFormat, FetchOptions, FletchDefinition,
    FletchRegistry, FreshnessPolicy, GraphNodeKind, PagedJsonOptions, SourceKind, SourceSpec,
    FLETCH_CACHE_INDEX_SCHEMA, FLETCH_REGISTRY_SCHEMA,
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
    let window = if season == icelines_core::CURRENT_SEASON_STR {
        "current"
    } else {
        season
    };
    format!("{WEB_BASE}/roster/{team}/{window}")
}

pub fn roster_dataset_id(team: &str, season: &str) -> String {
    if season == icelines_core::CURRENT_SEASON_STR {
        format!("icelines.roster.{season}.current.{team}")
    } else {
        format!("icelines.roster.{season}.{team}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FletchGamecenterArtifact {
    Boxscore,
    PlayByPlay,
}

impl FletchGamecenterArtifact {
    fn path_segment(self) -> &'static str {
        match self {
            Self::Boxscore => "boxscore",
            Self::PlayByPlay => "play-by-play",
        }
    }

    fn id_segment(self) -> &'static str {
        match self {
            Self::Boxscore => "boxscore",
            Self::PlayByPlay => "play-by-play",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FletchPlayerLandingArtifact {
    Landing,
}

impl FletchPlayerLandingArtifact {
    fn id_segment(self) -> &'static str {
        match self {
            Self::Landing => "landing",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FletchTransactionsOutcome {
    pub rows: Vec<RawTransaction>,
    pub dropped_unknown_schema: Vec<String>,
    pub partial: bool,
    pub source_etag: Option<String>,
    pub fetched_at: String,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FletchQueryPartitionRow {
    pub query_surface: String,
    pub partition_id: String,
    pub rollup_id: String,
    pub season: String,
    pub season_type: String,
    pub partition_role: String,
    pub source_fletch_ids: String,
    pub source_handoff_status: String,
    pub activation_evidence: String,
    pub query_examples: String,
    pub icelines_validation_floor: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FletchQueryPartitionReport {
    pub schema_version: String,
    pub generated_by: String,
    pub season: String,
    pub season_type: String,
    pub partition_count: usize,
    pub rollup_count: usize,
    pub adapter_required_count: usize,
    pub rows: Vec<FletchQueryPartitionRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FletchQueryQuiverRow {
    pub quiver_id: String,
    pub season: String,
    pub season_type: String,
    pub bundle_role: String,
    pub member_partition_ids: String,
    pub member_rollup_ids: String,
    pub member_count: usize,
    pub source_ready_partition_count: usize,
    pub adapter_required_partition_count: usize,
    pub activation_evidence: String,
    pub offline_bootstrap_rule: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FletchQueryQuiverReport {
    pub schema_version: String,
    pub generated_by: String,
    pub season: String,
    pub season_type: String,
    pub quiver_count: usize,
    pub member_count: usize,
    pub adapter_required_partition_count: usize,
    pub rows: Vec<FletchQueryQuiverRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FletchCacheIndexRow {
    pub fletch_id: String,
    pub dataset_id: String,
    pub cache_key: String,
    pub sha256: String,
    pub relative_path: String,
    pub bytes: u64,
    pub verified: bool,
    pub evidence_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FletchCacheIndexReport {
    pub schema_version: String,
    pub generated_by: String,
    pub source_schema: String,
    pub season: String,
    pub season_type: String,
    pub fletch_source_count: usize,
    pub indexed_source_count: usize,
    pub missing_source_count: usize,
    pub unexpected_index_count: usize,
    pub unverified_index_count: usize,
    pub byte_count: u64,
    pub rows: Vec<FletchCacheIndexRow>,
}

pub fn fletch_registry_for_season(season: &str, season_type: &str) -> FletchRegistry {
    let mut fletches = Vec::new();
    let mut seen = BTreeSet::new();

    for team in TEAMS {
        push_unique(
            &mut fletches,
            &mut seen,
            source_def(SourceDefInput {
                id: roster_dataset_id(team, season),
                fetch_surface: "rosters",
                season,
                season_type: "regular",
                source_kind: SourceKind::Http,
                source_url: roster_url(team, season),
                cache_targets: format!("snapshots/{season}-<date>-rosters/rosters/{team}.json"),
                media_type: "application/json",
                record_shape: "one team roster JSON",
                acquisition_mode: "generic-http-cacheline",
                activation_rule:
                    "fletch-acquires-source-icelines-owns-snapshot-seal-and-active-pointer",
                validation_floor: "snapshot is valid only after ICELINES parses every roster and seals the rosters tier",
            }),
        );
    }

    for ty in expanded_season_types(season_type) {
        for kind in ReportKind::all()
            .iter()
            .copied()
            .filter(|kind| kind.is_known_working())
        {
            let url_path = kind.url_path();
            push_unique(
                &mut fletches,
                &mut seen,
                source_def(SourceDefInput {
                    id: format!(
                        "icelines.stats.{season}.{ty}.{}",
                        url_path.replace('/', ".")
                    ),
                    fetch_surface: "stats-report",
                    season,
                    season_type: ty,
                    source_kind: SourceKind::Http,
                    source_url: stats_report_url(kind, season, ty),
                    cache_targets: format!(
                        "snapshots/<active-stats>/{season}/{ty}/{}.json",
                        url_path.replace('/', "-")
                    ),
                    media_type: "application/json",
                    record_shape: "paged JSON envelope with data rows and total count",
                    acquisition_mode: "generic-paged-json-cacheline",
                    activation_rule: "FLETCH acquires and caches paged JSON bytes; ICELINES owns report parsing, lock policy, snapshot seal, and active pointer",
                    validation_floor: "download success is not an analytics claim; ICELINES validates report shape, typed rows, chunk manifests, and stat catalog semantics",
                }),
            );
        }
    }

    push_unique(
        &mut fletches,
        &mut seen,
        source_def(SourceDefInput {
            id: format!("icelines.moneypuck.{season}.skaters"),
            fetch_surface: "moneypuck",
            season,
            season_type: "regular",
            source_kind: SourceKind::Http,
            source_url: crate::moneypuck::csv_url(season)
                .unwrap_or_else(|| "icelines-invalid-season://moneypuck".to_string()),
            cache_targets: format!("snapshots/{season}-<date>-moneypuck/moneypuck/moneypuck.json"),
            media_type: "text/csv",
            record_shape: "MoneyPuck season skater CSV",
            acquisition_mode: "generic-http-cacheline",
            activation_rule: "fletch-acquires-source-icelines-owns-csv-parse-json-snapshot-seal",
            validation_floor: "ICELINES validates CSV headers, player rows, derived percentages, snapshot metadata, and UI semantics",
        }),
    );

    for (id_suffix, surface, source_url, target, acquisition, activation, validation) in [
        (
            "ahl",
            "ahl",
            format!("icelines-ahl-hockeytech-batch://season/{season}"),
            format!("snapshots/{season}-<date>-ahl/ahl/ahl-roster-stats.json"),
            "generic-batch-http-cacheline-after-season",
            "FLETCH owns verified source cachelines; ICELINES owns season/team expansion, JSONP parsing, typed validation, snapshot sealing, and affiliation joins",
            "ICELINES validates provider-scoped identities, team codes, roster/stat shapes, counting totals, and current-season affiliation authority",
        ),
        (
            "transactions",
            "transactions",
            format!("icelines-espn-transactions://season/{season}"),
            format!("snapshots/{season}-<date>-transactions/transactions/transactions.json"),
            "generic-window-batch-http-cacheline-after-season",
            "ICELINES owns ESPN monthly window expansion, prose classification, warnings, and stale flags",
            "ICELINES validates ESPN schema fallback, classifier version, unknown-team handling, and stale metadata",
        ),
        (
            "contracts",
            "contracts",
            "icelines-player-landing-batch://contracts/from-active-stats-bios".to_string(),
            format!("snapshots/{season}-<date>-contracts/contracts/contracts.json"),
            "generic-batch-http-cacheline-after-player-set",
            "ICELINES owns active-bios player-set expansion, per-player rate limit, partial skip logging, and snapshot sealing",
            "ICELINES validates player-id coverage and nullable contract field forward-compatibility",
        ),
        (
            "career",
            "career",
            "icelines-player-landing-batch://career/from-active-or-bundled-bios".to_string(),
            "~/.icelines/career_history.json".to_string(),
            "generic-batch-http-cacheline-after-player-set",
            "ICELINES owns active/bundled player-set expansion, merge/upsert semantics, rate limit, and skipped-player reporting",
            "ICELINES validates career landing parsing, multi-league history merge, and preserved existing entries",
        ),
        (
            "boxscore",
            "boxscore",
            "icelines-gamecenter-batch://boxscore/from-schedule-date".to_string(),
            "~/.icelines/data/boxscores/<date>/<game_id>.json + EventStream score rows".to_string(),
            "generic-batch-http-cacheline-after-schedule",
            "ICELINES owns date schedule expansion, favorite filters, event-stream writes, and derived record payloads",
            "ICELINES validates game identity, score/event payload versioning, and favorite-player intersections",
        ),
        (
            "play-by-play",
            "play-by-play",
            "icelines-gamecenter-batch://play-by-play/from-schedule-date".to_string(),
            "~/.icelines/data/play_by_play/<date>/<game_id>.json".to_string(),
            "generic-batch-http-cacheline-after-schedule",
            "ICELINES owns date schedule expansion, favorite filters, raw event persistence, and records eligibility",
            "ICELINES validates event participants, goalie/fight record semantics, and manifest updates",
        ),
    ] {
        push_unique(
            &mut fletches,
            &mut seen,
            source_def(SourceDefInput {
                id: format!("icelines.{id_suffix}.{season}"),
                fetch_surface: surface,
                season,
                season_type,
                source_kind: SourceKind::Adapter,
                source_url,
                cache_targets: target,
                media_type: "application/json",
                record_shape: "dynamic source set",
                acquisition_mode: acquisition,
                activation_rule: activation,
                validation_floor: validation,
            }),
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
            } else if source.is_some_and(|source| source.kind == SourceKind::Adapter)
                && matches!(
                    metadata(definition, "acquisition_mode").as_str(),
                    "generic-batch-http-cacheline-after-schedule"
                        | "generic-batch-http-cacheline-after-player-set"
                        | "generic-batch-http-cacheline-after-season"
                        | "generic-window-batch-http-cacheline-after-season"
                )
            {
                "batch-expansion-ready-after-domain-set"
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

pub fn fletch_query_partition_report(
    season: &str,
    season_type: &str,
) -> FletchQueryPartitionReport {
    let registry = fletch_registry_for_season(season, season_type);
    let source_status = registry
        .fletches
        .iter()
        .map(|definition| {
            let status = definition
                .shafts
                .first()
                .map(|source| match source.kind {
                    SourceKind::Http | SourceKind::File => "source-byte-fletch-ready",
                    SourceKind::Adapter
                        if matches!(
                            metadata(definition, "acquisition_mode").as_str(),
                            "generic-batch-http-cacheline-after-schedule"
                                | "generic-batch-http-cacheline-after-player-set"
                                | "generic-batch-http-cacheline-after-season"
                                | "generic-window-batch-http-cacheline-after-season"
                        ) =>
                    {
                        "batch-source-fletch-ready-after-domain-set"
                    }
                    SourceKind::Adapter => "adapter-required-before-active-partition",
                })
                .unwrap_or("missing-source");
            (definition.id.clone(), status.to_string())
        })
        .collect::<BTreeMap<_, _>>();

    let mut rows = Vec::new();
    for ty in expanded_season_types(season_type) {
        for kind in ReportKind::all()
            .iter()
            .copied()
            .filter(|kind| kind.is_known_working())
        {
            let url_path = kind.url_path();
            let source_id = format!(
                "icelines.stats.{season}.{ty}.{}",
                url_path.replace('/', ".")
            );
            let skater = url_path.starts_with("skater/");
            let goalie = url_path.starts_with("goalie/");
            push_query_partition(
                &mut rows,
                QueryPartitionInput {
                    query_surface: if goalie {
                        "query-goalies"
                    } else {
                        "query-leaders-player-compare"
                    },
                    partition_id: format!(
                        "icelines.partition.{season}.{ty}.{}",
                        url_path.replace('/', ".")
                    ),
                    rollup_id: format!("icelines.rollup.{season}.{ty}.query-stats"),
                    season,
                    season_type: ty,
                    partition_role: if skater {
                        "skater-stats-report"
                    } else {
                        "goalie-stats-report"
                    },
                    source_fletch_ids: source_id,
                    source_status: &source_status,
                    query_examples: if goalie {
                        "icelines query goalies --filter \"save-pct>=0.92\""
                    } else {
                        "icelines query leaders --filter \"g>=50\"; icelines query compare"
                    },
                    validation_floor: "ICELINES validates typed report rows, stat catalog semantics, snapshot integrity, and season/type fences",
                },
            );
        }

        push_query_partition(
            &mut rows,
            QueryPartitionInput {
                query_surface: "query-windowed",
                partition_id: format!("icelines.partition.{season}.{ty}.boxscores.by-date"),
                rollup_id: format!("icelines.rollup.{season}.{ty}.game-lines"),
                season,
                season_type: ty,
                partition_role: "boxscore-date-partition",
                source_fletch_ids: format!("icelines.boxscore.{season}"),
                source_status: &source_status,
                query_examples: "icelines query leaders --week; sliding-window filters",
                validation_floor: "ICELINES validates schedule expansion, game identity, boxscore parsing, player participation, and event payload versions",
            },
        );
    }

    let roster_sources = TEAMS
        .iter()
        .map(|team| roster_dataset_id(team, season))
        .collect::<Vec<_>>();
    push_query_partition(
        &mut rows,
        QueryPartitionInput {
            query_surface: "query-roster-bio",
            partition_id: format!("icelines.partition.{season}.regular.rosters"),
            rollup_id: format!("icelines.rollup.{season}.regular.roster-bios"),
            season,
            season_type: "regular",
            partition_role: "team-roster-set",
            source_fletch_ids: roster_sources.join(";"),
            source_status: &source_status,
            query_examples: "icelines query leaders --team EDM --pos C; icelines query player",
            validation_floor: "ICELINES validates roster JSON shape, player identity joins, positions, active snapshot chain, and seal integrity",
        },
    );

    push_query_partition(
        &mut rows,
        QueryPartitionInput {
            query_surface: "query-advanced-metrics",
            partition_id: format!("icelines.partition.{season}.regular.moneypuck.skaters"),
            rollup_id: format!("icelines.rollup.{season}.regular.advanced-metrics"),
            season,
            season_type: "regular",
            partition_role: "moneypuck-skaters",
            source_fletch_ids: format!("icelines.moneypuck.{season}.skaters"),
            source_status: &source_status,
            query_examples: "icelines query leaders --sort xg; icelines query leaders --sort cf-pct",
            validation_floor: "ICELINES validates CSV headers, player joins, derived percentages, null policy, and snapshot metadata",
        },
    );

    push_query_partition(
        &mut rows,
        QueryPartitionInput {
            query_surface: "query-career",
            partition_id: format!("icelines.partition.{season}.career-history"),
            rollup_id: format!("icelines.rollup.{season}.career"),
            season,
            season_type: "regular",
            partition_role: "career-history-cache",
            source_fletch_ids: format!("icelines.career.{season}"),
            source_status: &source_status,
            query_examples: "icelines query player \"Connor McDavid\" --seasons 38; career league filters",
            validation_floor: "ICELINES validates active/bundled player expansion, multi-league history parsing, merge/upsert semantics, and skipped-player reporting",
        },
    );

    rows.sort_by(|left, right| {
        left.query_surface
            .cmp(&right.query_surface)
            .then(left.season_type.cmp(&right.season_type))
            .then(left.partition_id.cmp(&right.partition_id))
    });
    let rollup_count = rows
        .iter()
        .map(|row| row.rollup_id.clone())
        .collect::<BTreeSet<_>>()
        .len();
    let adapter_required_count = rows
        .iter()
        .filter(|row| {
            row.source_handoff_status
                .contains("adapter-required-before-active-partition")
        })
        .count();

    FletchQueryPartitionReport {
        schema_version: "icelines.fletch-query-partitions.v1".to_string(),
        generated_by: "icelines-fetch".to_string(),
        season: season.to_string(),
        season_type: season_type.to_string(),
        partition_count: rows.len(),
        rollup_count,
        adapter_required_count,
        rows,
    }
}

pub fn fletch_query_quiver_report(season: &str, season_type: &str) -> FletchQueryQuiverReport {
    let partitions = fletch_query_partition_report(season, season_type);
    let mut rows = Vec::new();
    for ty in expanded_season_types(season_type) {
        let members = partitions
            .rows
            .iter()
            .filter(|row| row.season_type == ty)
            .collect::<Vec<_>>();
        push_query_quiver(
            &mut rows,
            format!("icelines.quiver.{season}.{ty}.query"),
            season,
            ty,
            "season-query-bootstrap",
            &members,
        );
    }

    let regular_members = partitions
        .rows
        .iter()
        .filter(|row| {
            row.season_type == "regular"
                && matches!(
                    row.query_surface.as_str(),
                    "query-roster-bio" | "query-advanced-metrics" | "query-career"
                )
        })
        .collect::<Vec<_>>();
    push_query_quiver(
        &mut rows,
        format!("icelines.quiver.{season}.regular.enrichment"),
        season,
        "regular",
        "roster-advanced-career-enrichment",
        &regular_members,
    );

    rows.sort_by(|left, right| left.quiver_id.cmp(&right.quiver_id));
    let member_count = rows.iter().map(|row| row.member_count).sum();
    let adapter_required_partition_count = rows
        .iter()
        .map(|row| row.adapter_required_partition_count)
        .sum();

    FletchQueryQuiverReport {
        schema_version: "icelines.fletch-query-quivers.v1".to_string(),
        generated_by: "icelines-fetch".to_string(),
        season: season.to_string(),
        season_type: season_type.to_string(),
        quiver_count: rows.len(),
        member_count,
        adapter_required_partition_count,
        rows,
    }
}

pub fn fletch_cache_index_report(
    season: &str,
    season_type: &str,
    manifest: &CacheManifest,
) -> FletchCacheIndexReport {
    let registry = fletch_registry_for_season(season, season_type);
    let expected_ids = registry
        .fletches
        .iter()
        .map(|definition| definition.id.clone())
        .collect::<BTreeSet<_>>();
    let index = cache_index_from_manifest(manifest);
    let mut indexed_by_fletch = BTreeMap::<String, Vec<_>>::new();
    let mut unexpected_entries = Vec::new();
    let mut gate_index = index.clone();
    for entry in &index.entries {
        if let Some(fletch_id) =
            fletch_cache_index_registry_id(&entry.dataset_id, season, &expected_ids)
        {
            indexed_by_fletch.entry(fletch_id).or_default().push(entry);
        } else {
            unexpected_entries.push(entry);
        }
    }
    for entry in &mut gate_index.entries {
        if let Some(fletch_id) =
            fletch_cache_index_registry_id(&entry.dataset_id, season, &expected_ids)
        {
            entry.dataset_id = fletch_id;
        }
    }
    let gate = cache_index_gate_report(
        &gate_index,
        &CacheIndexGatePolicy {
            expected_dataset_ids: expected_ids.iter().cloned().collect(),
            require_verified: true,
            allow_missing_expected: true,
        },
    );

    let mut rows = Vec::new();
    for fletch_id in &expected_ids {
        if let Some(entries) = indexed_by_fletch.get(fletch_id) {
            for entry in entries {
                rows.push(FletchCacheIndexRow {
                    fletch_id: fletch_id.clone(),
                    dataset_id: entry.dataset_id.clone(),
                    cache_key: entry.cache_key.clone(),
                    sha256: entry.sha256.clone(),
                    relative_path: entry.relative_path.clone(),
                    bytes: entry.bytes,
                    verified: entry.verified,
                    evidence_status: if entry.verified {
                        "indexed-verified"
                    } else {
                        "indexed-unverified"
                    }
                    .to_string(),
                });
            }
        } else {
            rows.push(FletchCacheIndexRow {
                fletch_id: fletch_id.clone(),
                dataset_id: String::new(),
                cache_key: String::new(),
                sha256: String::new(),
                relative_path: String::new(),
                bytes: 0,
                verified: false,
                evidence_status: "missing-index-row".to_string(),
            });
        }
    }
    for entry in unexpected_entries {
        rows.push(FletchCacheIndexRow {
            fletch_id: entry.dataset_id.clone(),
            dataset_id: entry.dataset_id.clone(),
            cache_key: entry.cache_key.clone(),
            sha256: entry.sha256.clone(),
            relative_path: entry.relative_path.clone(),
            bytes: entry.bytes,
            verified: entry.verified,
            evidence_status: "unexpected-index-row".to_string(),
        });
    }

    rows.sort_by(|left, right| {
        left.evidence_status
            .cmp(&right.evidence_status)
            .then(left.fletch_id.cmp(&right.fletch_id))
    });
    let indexed_source_count = rows
        .iter()
        .filter(|row| {
            matches!(
                row.evidence_status.as_str(),
                "indexed-verified" | "indexed-unverified"
            )
        })
        .count();
    let missing_source_count = rows
        .iter()
        .filter(|row| row.evidence_status == "missing-index-row")
        .count();
    let unexpected_index_count = gate.unexpected_count;
    let unverified_index_count = gate.unverified_count;
    let byte_count = rows
        .iter()
        .filter(|row| row.evidence_status == "indexed-verified")
        .map(|row| row.bytes)
        .sum();

    FletchCacheIndexReport {
        schema_version: "icelines.fletch-cache-index.v1".to_string(),
        generated_by: "icelines-fetch".to_string(),
        source_schema: FLETCH_CACHE_INDEX_SCHEMA.to_string(),
        season: season.to_string(),
        season_type: season_type.to_string(),
        fletch_source_count: expected_ids.len(),
        indexed_source_count,
        missing_source_count,
        unexpected_index_count,
        unverified_index_count,
        byte_count,
        rows,
    }
}

fn fletch_cache_index_registry_id(
    dataset_id: &str,
    season: &str,
    expected_ids: &BTreeSet<String>,
) -> Option<String> {
    if expected_ids.contains(dataset_id) {
        return Some(dataset_id.to_string());
    }

    for (prefix, registered_suffix) in [
        ("icelines.player.contracts.", "contracts"),
        ("icelines.player.career.", "career"),
        ("icelines.gamecenter.boxscore.", "boxscore"),
        ("icelines.gamecenter.play-by-play.", "play-by-play"),
    ] {
        let registered_id = format!("icelines.{registered_suffix}.{season}");
        if dataset_id.starts_with(prefix) && expected_ids.contains(&registered_id) {
            return Some(registered_id);
        }
    }

    let transaction_prefix = format!("icelines.transactions.{season}.");
    let registered_id = format!("icelines.transactions.{season}");
    if dataset_id.starts_with(&transaction_prefix) && expected_ids.contains(&registered_id) {
        return Some(registered_id);
    }

    let ahl_prefix = format!("icelines.ahl.{season}.");
    let registered_id = format!("icelines.ahl.{season}");
    if dataset_id.starts_with(&ahl_prefix) && expected_ids.contains(&registered_id) {
        return Some(registered_id);
    }

    None
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

pub fn write_fletch_query_partitions(
    path: &Path,
    report: &FletchQueryPartitionReport,
) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let file =
        std::fs::File::create(path).with_context(|| format!("writing {}", path.display()))?;
    serde_json::to_writer_pretty(file, report)
        .with_context(|| format!("serializing {}", path.display()))?;
    Ok(())
}

pub fn write_fletch_query_quivers(path: &Path, report: &FletchQueryQuiverReport) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let file =
        std::fs::File::create(path).with_context(|| format!("writing {}", path.display()))?;
    serde_json::to_writer_pretty(file, report)
        .with_context(|| format!("serializing {}", path.display()))?;
    Ok(())
}

pub fn write_fletch_cache_index(path: &Path, report: &FletchCacheIndexReport) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let file =
        std::fs::File::create(path).with_context(|| format!("writing {}", path.display()))?;
    serde_json::to_writer_pretty(file, report)
        .with_context(|| format!("serializing {}", path.display()))?;
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

pub fn fletch_query_partition_gate_failures(report: &FletchQueryPartitionReport) -> Vec<String> {
    let mut failures = Vec::new();
    for row in &report.rows {
        if row.partition_id.is_empty()
            || row.rollup_id.is_empty()
            || row.source_fletch_ids.is_empty()
            || row.activation_evidence.is_empty()
            || row.icelines_validation_floor.is_empty()
        {
            failures.push(format!(
                "{} has incomplete partition metadata",
                row.partition_id
            ));
        }
        if row.source_handoff_status.contains("missing-source") {
            failures.push(format!(
                "{} references missing source(s): {}",
                row.partition_id, row.source_fletch_ids
            ));
        }
    }
    failures
}

pub fn fletch_query_quiver_gate_failures(report: &FletchQueryQuiverReport) -> Vec<String> {
    let mut failures = Vec::new();
    for row in &report.rows {
        if row.quiver_id.is_empty()
            || row.member_count == 0
            || row.member_partition_ids.is_empty()
            || row.member_rollup_ids.is_empty()
            || row.activation_evidence.is_empty()
            || row.offline_bootstrap_rule.is_empty()
        {
            failures.push(format!("{} has incomplete quiver metadata", row.quiver_id));
        }
    }
    failures
}

pub fn fletch_cache_index_gate_failures(report: &FletchCacheIndexReport) -> Vec<String> {
    let mut failures = Vec::new();
    if report.unverified_index_count > 0 {
        failures.push(format!(
            "{} indexed source row(s) are unverified",
            report.unverified_index_count
        ));
    }
    if report.unexpected_index_count > 0 {
        failures.push(format!(
            "{} cache index row(s) do not map to the ICELINES FLETCH registry",
            report.unexpected_index_count
        ));
    }
    failures
}

pub fn fletch_cache_manifest_path(cache_root: &Path) -> std::path::PathBuf {
    cache_root.join("cache-manifest.json")
}

pub fn read_fletch_cache_manifest(path: &Path) -> Result<CacheManifest> {
    read_cache_manifest_json(path)
        .with_context(|| format!("reading FLETCH cache manifest {}", path.display()))
}

fn upsert_fletch_cache_manifest_entries(
    cache_root: &Path,
    entries: impl IntoIterator<Item = CacheEntry>,
) -> Result<CacheManifest> {
    let manifest_path = fletch_cache_manifest_path(cache_root);
    let entries = entries.into_iter().collect::<Vec<_>>();
    if entries.is_empty() {
        return if manifest_path.exists() {
            read_fletch_cache_manifest(&manifest_path)
        } else {
            fletch_core::cache_manifest(cache_root.display().to_string(), Vec::new())
                .context("creating empty FLETCH cache manifest")
        };
    }
    let mut manifest = if manifest_path.exists() {
        read_fletch_cache_manifest(&manifest_path)?
    } else {
        fletch_core::cache_manifest(cache_root.display().to_string(), Vec::new())
            .context("creating empty FLETCH cache manifest")?
    };
    manifest = upsert_cache_manifest_entries(manifest, entries)
        .context("upserting FLETCH cache manifest entries")?;
    let bytes =
        serde_json::to_vec_pretty(&manifest).context("serializing FLETCH cache manifest")?;
    write_bytes_atomic(&manifest_path, &bytes).with_context(|| {
        format!(
            "atomically writing FLETCH cache manifest {}",
            manifest_path.display()
        )
    })?;
    Ok(manifest)
}

fn read_verified_fletch_cache_bytes(cache_root: &Path, fletch_id: &str) -> Result<Option<Vec<u8>>> {
    let manifest_path = fletch_cache_manifest_path(cache_root);
    if !manifest_path.exists() {
        return Ok(None);
    }
    let manifest = read_fletch_cache_manifest(&manifest_path)?;
    let Some(entry) = manifest
        .entries
        .iter()
        .find(|entry| entry.dataset_id == fletch_id && entry.verified)
    else {
        return Ok(None);
    };
    let path = cache_root.join(&entry.relative_path);
    let bytes = std::fs::read(&path)
        .with_context(|| format!("reading verified FLETCH cache object {}", path.display()))?;
    Ok(Some(bytes))
}

pub fn fetch_generic_http_bytes(
    fletch_id: impl Into<String>,
    source_url: impl Into<String>,
    cache_root: &Path,
    force: bool,
) -> Result<Vec<u8>> {
    let (bytes, entry) =
        fetch_generic_http_bytes_unindexed(fletch_id.into(), source_url.into(), cache_root, force)?;
    if let Some(entry) = entry {
        upsert_fletch_cache_manifest_entries(cache_root, [entry])?;
    }
    Ok(bytes)
}

fn fetch_generic_http_bytes_unindexed(
    fletch_id: String,
    source_url: String,
    cache_root: &Path,
    force: bool,
) -> Result<(Vec<u8>, Option<CacheEntry>)> {
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

    let options = FetchOptions::new(cache_root)
        .with_force(force)
        .with_timeout_ms(30_000)
        .with_retry_attempts(5);
    let outcome = match fetch_to_cache(&plan, options) {
        Ok(outcome) => outcome,
        Err(error) if !force => {
            if let Some(bytes) = read_verified_fletch_cache_bytes(cache_root, &fletch_id)
                .with_context(|| format!("reading cached FLETCH object for {fletch_id}"))?
            {
                return Ok((bytes, None));
            }
            return Err(error).with_context(|| format!("fetching {fletch_id} through FLETCH"));
        }
        Err(error) => {
            return Err(error).with_context(|| format!("fetching {fletch_id} through FLETCH"));
        }
    };
    let bytes = std::fs::read(&outcome.path)
        .with_context(|| format!("reading FLETCH cache object {}", outcome.path.display()))?;
    Ok((bytes, Some(outcome.entry)))
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

/// Fetch independent HTTP cachelines concurrently while writing the shared
/// ICELINES FLETCH manifest once. Per-request manifest writes are deliberately
/// deferred so concurrent read-modify-write cycles cannot drop entries.
pub async fn fetch_generic_http_batch_async(
    requests: Vec<(String, String)>,
    cache_root: impl Into<std::path::PathBuf>,
    force: bool,
    max_concurrency: usize,
) -> Vec<(String, Result<Vec<u8>>)> {
    let cache_root = cache_root.into();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrency.max(1)));
    let mut tasks = tokio::task::JoinSet::new();
    for (dataset_id, source_url) in requests {
        let task_id = dataset_id.clone();
        let root = cache_root.clone();
        let semaphore = semaphore.clone();
        tasks.spawn(async move {
            let permit = semaphore
                .acquire_owned()
                .await
                .context("acquiring generic HTTP batch permit")?;
            let outcome = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                fetch_generic_http_bytes_unindexed(dataset_id, source_url, &root, force)
            })
            .await
            .context("joining generic HTTP batch task")?;
            Ok::<_, anyhow::Error>((task_id, outcome))
        });
    }

    let mut fetched = Vec::new();
    let mut entries = Vec::new();
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(Ok((dataset_id, Ok((bytes, entry))))) => {
                if let Some(entry) = entry {
                    entries.push(entry);
                }
                fetched.push((dataset_id, Ok(bytes)));
            }
            Ok(Ok((dataset_id, Err(error)))) => fetched.push((dataset_id, Err(error))),
            Ok(Err(error)) => fetched.push(("unknown".to_owned(), Err(error))),
            Err(error) => fetched.push((
                "unknown".to_owned(),
                Err(anyhow::anyhow!("generic HTTP batch task failed: {error}")),
            )),
        }
    }
    if !entries.is_empty() {
        if let Err(error) = upsert_fletch_cache_manifest_entries(&cache_root, entries) {
            fetched.push(("fletch-manifest".to_owned(), Err(error)));
        }
    }
    fetched.sort_by(|a, b| a.0.cmp(&b.0));
    fetched
}

pub fn stats_report_url(kind: ReportKind, season: &str, season_type: &str) -> String {
    let game_type_id = game_type_id(season_type);
    format!(
        "https://api.nhle.com/stats/rest/en/{}?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D{game_type_id}",
        kind.url_path()
    )
}

pub fn fetch_paged_report_bytes(
    kind: ReportKind,
    season: &str,
    season_type: &str,
    cache_root: &Path,
    force: bool,
) -> Result<Vec<u8>> {
    let fletch_id = format!(
        "icelines.stats.{season}.{season_type}.{}",
        kind.url_path().replace('/', ".")
    );
    let source_url = stats_report_url(kind, season, season_type);
    let mut plan = fetch_plan_with_kind(fletch_id.clone(), source_url, SourceKind::Http)
        .with_context(|| format!("building FLETCH paged fetch plan for {fletch_id}"))?;
    plan.cache_policy = CachePolicy {
        freshness: FreshnessPolicy::AlwaysCheck,
        allow_offline: true,
        resumable: true,
    };
    plan.tags = vec![
        "icelines".to_string(),
        "stats-report".to_string(),
        "generic-paged-json-cacheline".to_string(),
    ];
    plan.metadata
        .insert("adapter".to_string(), "icelines".to_string());

    let outcome = fetch_paged_json_to_cache(
        &plan,
        FetchOptions::new(cache_root)
            .with_force(force)
            .with_timeout_ms(30_000)
            .with_retry_attempts(5),
        PagedJsonOptions::default().with_limit(100),
    )
    .with_context(|| format!("fetching paged report {fletch_id} through FLETCH"))?;
    upsert_fletch_cache_manifest_entries(cache_root, [outcome.outcome.entry.clone()])?;

    std::fs::read(&outcome.outcome.path).with_context(|| {
        format!(
            "reading FLETCH cache object {}",
            outcome.outcome.path.display()
        )
    })
}

pub async fn fetch_paged_report_bytes_async(
    kind: ReportKind,
    season: impl Into<String>,
    season_type: impl Into<String>,
    cache_root: impl Into<std::path::PathBuf>,
    force: bool,
) -> Result<Vec<u8>> {
    let season = season.into();
    let season_type = season_type.into();
    let cache_root = cache_root.into();
    tokio::task::spawn_blocking(move || {
        fetch_paged_report_bytes(kind, &season, &season_type, &cache_root, force)
    })
    .await
    .context("joining FLETCH paged fetch task")?
}

pub fn gamecenter_url(base_web: &str, game_id: u64, artifact: FletchGamecenterArtifact) -> String {
    format!(
        "{base_web}/gamecenter/{game_id}/{}",
        artifact.path_segment()
    )
}

pub fn fetch_gamecenter_batch_bytes_with_base(
    base_web: &str,
    game_ids: &[u64],
    artifact: FletchGamecenterArtifact,
    cache_root: &Path,
    force: bool,
) -> Result<BTreeMap<u64, Vec<u8>>> {
    let plans = game_ids
        .iter()
        .map(|game_id| {
            let fletch_id = format!("icelines.gamecenter.{}.{game_id}", artifact.id_segment());
            let mut plan = fetch_plan_with_kind(
                fletch_id.clone(),
                gamecenter_url(base_web, *game_id, artifact),
                SourceKind::Http,
            )
            .with_context(|| format!("building FLETCH Gamecenter plan for {fletch_id}"))?;
            plan.cache_policy = CachePolicy {
                freshness: FreshnessPolicy::AlwaysCheck,
                allow_offline: true,
                resumable: true,
            };
            plan.tags = vec![
                "icelines".to_string(),
                "gamecenter".to_string(),
                artifact.id_segment().to_string(),
                "generic-batch-http-cacheline".to_string(),
            ];
            plan.metadata
                .insert("adapter".to_string(), "icelines".to_string());
            Ok((*game_id, plan))
        })
        .collect::<Result<Vec<_>>>()?;
    let plan_only = plans
        .iter()
        .map(|(_, plan)| plan.clone())
        .collect::<Vec<_>>();
    let outcome = fetch_batch_to_cache_best_effort(
        &plan_only,
        FetchOptions::new(cache_root)
            .with_force(force)
            .with_timeout_ms(30_000)
            .with_retry_attempts(5),
    )
    .with_context(|| {
        format!(
            "fetching {} Gamecenter artifact(s) through FLETCH",
            artifact.id_segment()
        )
    })?;
    upsert_fletch_cache_manifest_entries(
        cache_root,
        outcome
            .outcomes
            .iter()
            .map(|outcome| outcome.entry.clone())
            .collect::<Vec<_>>(),
    )?;

    let game_by_dataset = plans
        .iter()
        .map(|(game_id, plan)| (plan.dataset_id.clone(), *game_id))
        .collect::<BTreeMap<_, _>>();
    let mut bytes_by_game = BTreeMap::new();
    for outcome in &outcome.outcomes {
        let Some(game_id) = game_by_dataset.get(&outcome.entry.dataset_id) else {
            continue;
        };
        let bytes = std::fs::read(&outcome.path)
            .with_context(|| format!("reading FLETCH cache object {}", outcome.path.display()))?;
        bytes_by_game.insert(*game_id, bytes);
    }
    if !outcome.failures.is_empty() {
        let failures = outcome
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.dataset_id, failure.error))
            .collect::<Vec<_>>()
            .join("; ");
        eprintln!(
            "FLETCH Gamecenter batch skipped {} {} artifact(s): {failures}",
            outcome.failure_count,
            artifact.id_segment()
        );
    }
    Ok(bytes_by_game)
}

pub fn fetch_gamecenter_batch_bytes(
    game_ids: &[u64],
    artifact: FletchGamecenterArtifact,
    cache_root: &Path,
    force: bool,
) -> Result<BTreeMap<u64, Vec<u8>>> {
    fetch_gamecenter_batch_bytes_with_base(WEB_BASE, game_ids, artifact, cache_root, force)
}

pub async fn fetch_gamecenter_batch_bytes_async(
    game_ids: Vec<u64>,
    artifact: FletchGamecenterArtifact,
    cache_root: impl Into<std::path::PathBuf>,
    force: bool,
) -> Result<BTreeMap<u64, Vec<u8>>> {
    let cache_root = cache_root.into();
    tokio::task::spawn_blocking(move || {
        fetch_gamecenter_batch_bytes(&game_ids, artifact, &cache_root, force)
    })
    .await
    .context("joining FLETCH Gamecenter batch fetch task")?
}

pub fn player_landing_url(
    base_web: &str,
    player_id: u32,
    _artifact: FletchPlayerLandingArtifact,
) -> String {
    format!("{base_web}/player/{player_id}/landing")
}

pub fn fetch_player_landing_batch_bytes_with_base(
    base_web: &str,
    player_ids: &[u32],
    artifact: FletchPlayerLandingArtifact,
    cache_root: &Path,
    force: bool,
    delay_between_items_ms: u64,
) -> Result<BTreeMap<u32, Vec<u8>>> {
    let plans = player_ids
        .iter()
        .map(|player_id| {
            let fletch_id = format!("icelines.player.{}.{player_id}", artifact.id_segment());
            let mut plan = fetch_plan_with_kind(
                fletch_id.clone(),
                player_landing_url(base_web, *player_id, artifact),
                SourceKind::Http,
            )
            .with_context(|| format!("building FLETCH player landing plan for {fletch_id}"))?;
            plan.cache_policy = CachePolicy {
                freshness: FreshnessPolicy::AlwaysCheck,
                allow_offline: true,
                resumable: true,
            };
            plan.tags = vec![
                "icelines".to_string(),
                "player-landing".to_string(),
                artifact.id_segment().to_string(),
                "generic-batch-http-cacheline".to_string(),
            ];
            plan.metadata
                .insert("adapter".to_string(), "icelines".to_string());
            Ok((*player_id, plan))
        })
        .collect::<Result<Vec<_>>>()?;
    let plan_only = plans
        .iter()
        .map(|(_, plan)| plan.clone())
        .collect::<Vec<_>>();
    let outcome = fetch_batch_to_cache_best_effort_with_delay(
        &plan_only,
        FetchOptions::new(cache_root)
            .with_force(force)
            .with_timeout_ms(30_000)
            .with_retry_attempts(5),
        delay_between_items_ms,
    )
    .with_context(|| {
        format!(
            "fetching {} player landing artifact(s) through FLETCH",
            artifact.id_segment()
        )
    })?;
    upsert_fletch_cache_manifest_entries(
        cache_root,
        outcome
            .outcomes
            .iter()
            .map(|outcome| outcome.entry.clone())
            .collect::<Vec<_>>(),
    )?;

    let player_by_dataset = plans
        .iter()
        .map(|(player_id, plan)| (plan.dataset_id.clone(), *player_id))
        .collect::<BTreeMap<_, _>>();
    let mut bytes_by_player = BTreeMap::new();
    for outcome in &outcome.outcomes {
        let Some(player_id) = player_by_dataset.get(&outcome.entry.dataset_id) else {
            continue;
        };
        let bytes = std::fs::read(&outcome.path)
            .with_context(|| format!("reading FLETCH cache object {}", outcome.path.display()))?;
        bytes_by_player.insert(*player_id, bytes);
    }
    if !outcome.failures.is_empty() {
        let failures = outcome
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.dataset_id, failure.error))
            .collect::<Vec<_>>()
            .join("; ");
        eprintln!(
            "FLETCH player landing batch skipped {} {} artifact(s): {failures}",
            outcome.failure_count,
            artifact.id_segment()
        );
    }
    Ok(bytes_by_player)
}

pub fn fetch_player_landing_batch_bytes(
    player_ids: &[u32],
    artifact: FletchPlayerLandingArtifact,
    cache_root: &Path,
    force: bool,
    delay_between_items_ms: u64,
) -> Result<BTreeMap<u32, Vec<u8>>> {
    fetch_player_landing_batch_bytes_with_base(
        WEB_BASE,
        player_ids,
        artifact,
        cache_root,
        force,
        delay_between_items_ms,
    )
}

pub async fn fetch_player_landing_batch_bytes_async(
    player_ids: Vec<u32>,
    artifact: FletchPlayerLandingArtifact,
    cache_root: impl Into<std::path::PathBuf>,
    force: bool,
    delay_between_items_ms: u64,
) -> Result<BTreeMap<u32, Vec<u8>>> {
    let cache_root = cache_root.into();
    tokio::task::spawn_blocking(move || {
        fetch_player_landing_batch_bytes(
            &player_ids,
            artifact,
            &cache_root,
            force,
            delay_between_items_ms,
        )
    })
    .await
    .context("joining FLETCH player landing batch fetch task")?
}

pub fn espn_transactions_url(base_url: &str, window: &str) -> String {
    format!("{base_url}/transactions?dates={window}&limit=1000")
}

pub fn fetch_transactions_batch_with_base(
    base_url: &str,
    season: &str,
    cache_root: &Path,
    force: bool,
) -> Result<FletchTransactionsOutcome> {
    let windows = crate::transactions::espn::season_month_windows(season)
        .with_context(|| format!("invalid season ID '{season}' - expected 8 digits"))?;
    let plans = windows
        .iter()
        .map(|window| {
            let fletch_id = format!("icelines.transactions.{season}.{window}");
            let mut plan = fetch_plan_with_kind(
                fletch_id.clone(),
                espn_transactions_url(base_url, window),
                SourceKind::Http,
            )
            .with_context(|| format!("building FLETCH transaction window plan for {fletch_id}"))?;
            plan.cache_policy = CachePolicy {
                freshness: FreshnessPolicy::AlwaysCheck,
                allow_offline: true,
                resumable: true,
            };
            plan.tags = vec![
                "icelines".to_string(),
                "transactions".to_string(),
                "generic-window-batch-http-cacheline".to_string(),
            ];
            plan.metadata
                .insert("adapter".to_string(), "icelines".to_string());
            Ok((window.clone(), plan))
        })
        .collect::<Result<Vec<_>>>()?;
    let plan_only = plans
        .iter()
        .map(|(_, plan)| plan.clone())
        .collect::<Vec<_>>();
    let outcome = fetch_batch_to_cache_best_effort(
        &plan_only,
        FetchOptions::new(cache_root)
            .with_force(force)
            .with_timeout_ms(30_000)
            .with_retry_attempts(5),
    )
    .context("fetching ESPN transaction windows through FLETCH")?;
    upsert_fletch_cache_manifest_entries(
        cache_root,
        outcome
            .outcomes
            .iter()
            .map(|outcome| outcome.entry.clone())
            .collect::<Vec<_>>(),
    )?;

    if outcome.outcomes.is_empty() && !outcome.failures.is_empty() {
        anyhow::bail!(
            "all ESPN transaction windows failed through FLETCH: {}",
            outcome
                .failures
                .iter()
                .map(|failure| format!("{}: {}", failure.dataset_id, failure.error))
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    let window_by_dataset = plans
        .iter()
        .map(|(window, plan)| (plan.dataset_id.clone(), window.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();
    let mut dropped_unknown_schema = Vec::new();
    let mut seen = BTreeSet::new();
    for item in &outcome.outcomes {
        let window = window_by_dataset
            .get(&item.entry.dataset_id)
            .cloned()
            .unwrap_or_else(|| item.entry.dataset_id.clone());
        let bytes = std::fs::read(&item.path)
            .with_context(|| format!("reading FLETCH cache object {}", item.path.display()))?;
        let body = match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(body) => body,
            Err(error) => {
                dropped_unknown_schema.push(format!(
                    "{window}: ESPN body not parseable as JSON: {error}"
                ));
                continue;
            }
        };
        let (window_rows, dropped) = crate::transactions::espn::parse_page_with_fallback(&body);
        for row in window_rows {
            let key = (row.date.clone(), row.description.clone());
            if seen.insert(key) {
                rows.push(row);
            }
        }
        dropped_unknown_schema.extend(dropped.into_iter().map(|path| format!("{window}: {path}")));
    }
    dropped_unknown_schema.extend(
        outcome
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.dataset_id, failure.error)),
    );

    Ok(FletchTransactionsOutcome {
        rows,
        dropped_unknown_schema,
        partial: outcome.failure_count > 0,
        source_etag: None,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub fn fetch_transactions_batch(
    season: &str,
    cache_root: &Path,
    force: bool,
) -> Result<FletchTransactionsOutcome> {
    fetch_transactions_batch_with_base(
        crate::transactions::espn::DEFAULT_ESPN_BASE,
        season,
        cache_root,
        force,
    )
}

pub async fn fetch_transactions_batch_async(
    season: impl Into<String>,
    cache_root: impl Into<std::path::PathBuf>,
    force: bool,
) -> Result<FletchTransactionsOutcome> {
    let season = season.into();
    let cache_root = cache_root.into();
    tokio::task::spawn_blocking(move || fetch_transactions_batch(&season, &cache_root, force))
        .await
        .context("joining FLETCH transaction window batch fetch task")?
}

fn push_query_quiver(
    rows: &mut Vec<FletchQueryQuiverRow>,
    quiver_id: String,
    season: &str,
    season_type: &str,
    bundle_role: &str,
    members: &[&FletchQueryPartitionRow],
) {
    let member_partition_ids = members
        .iter()
        .map(|row| row.partition_id.clone())
        .collect::<Vec<_>>();
    let member_rollup_ids = members
        .iter()
        .map(|row| row.rollup_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let source_ready_partition_count = members
        .iter()
        .filter(|row| {
            matches!(
                row.source_handoff_status.as_str(),
                "source-byte-fletch-ready" | "batch-source-fletch-ready-after-domain-set"
            )
        })
        .count();
    let adapter_required_partition_count = members
        .iter()
        .filter(|row| {
            row.source_handoff_status
                .contains("adapter-required-before-active-partition")
        })
        .count();
    rows.push(FletchQueryQuiverRow {
        quiver_id,
        season: season.to_string(),
        season_type: season_type.to_string(),
        bundle_role: bundle_role.to_string(),
        member_partition_ids: member_partition_ids.join(";"),
        member_rollup_ids: member_rollup_ids.join(";"),
        member_count: members.len(),
        source_ready_partition_count,
        adapter_required_partition_count,
        activation_evidence: "ICELINES sealed snapshots and active pointers decide whether imported quiver members are query-active".to_string(),
        offline_bootstrap_rule: "FLETCH quiver members can stage/cache bytes; ICELINES must parse, validate, seal, and activate snapshots before queries trust them".to_string(),
    });
}

struct QueryPartitionInput<'a> {
    query_surface: &'a str,
    partition_id: String,
    rollup_id: String,
    season: &'a str,
    season_type: &'a str,
    partition_role: &'a str,
    source_fletch_ids: String,
    source_status: &'a BTreeMap<String, String>,
    query_examples: &'a str,
    validation_floor: &'a str,
}

fn push_query_partition(rows: &mut Vec<FletchQueryPartitionRow>, input: QueryPartitionInput<'_>) {
    let statuses = input
        .source_fletch_ids
        .split(';')
        .map(|source_id| {
            input
                .source_status
                .get(source_id)
                .cloned()
                .unwrap_or_else(|| "missing-source".to_string())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(";");
    rows.push(FletchQueryPartitionRow {
        query_surface: input.query_surface.to_string(),
        partition_id: input.partition_id,
        rollup_id: input.rollup_id,
        season: input.season.to_string(),
        season_type: input.season_type.to_string(),
        partition_role: input.partition_role.to_string(),
        source_fletch_ids: input.source_fletch_ids,
        source_handoff_status: statuses,
        activation_evidence: "ICELINES sealed snapshot plus active pointer; FLETCH source cache alone is not active query data".to_string(),
        query_examples: input.query_examples.to_string(),
        icelines_validation_floor: input.validation_floor.to_string(),
    });
}

struct SourceDefInput<'a> {
    id: String,
    fetch_surface: &'a str,
    season: &'a str,
    season_type: &'a str,
    source_kind: SourceKind,
    source_url: String,
    cache_targets: String,
    media_type: &'a str,
    record_shape: &'a str,
    acquisition_mode: &'a str,
    activation_rule: &'a str,
    validation_floor: &'a str,
}

fn source_def(input: SourceDefInput<'_>) -> FletchDefinition {
    FletchDefinition {
        id: input.id,
        node_kind: GraphNodeKind::Fletch,
        shafts: vec![SourceSpec {
            kind: input.source_kind,
            url: input.source_url,
            headers: BTreeMap::new(),
        }],
        edges: Vec::new(),
        format: Some(DataFormat {
            media_type: Some(input.media_type.to_string()),
            encoding: Some("utf-8".to_string()),
            compression: None,
            container: None,
            schema: None,
            record_shape: Some(input.record_shape.to_string()),
            preferred_local: Some(input.cache_targets.clone()),
        }),
        tags: vec![
            "icelines".to_string(),
            input.fetch_surface.to_string(),
            input.season.to_string(),
            input.season_type.to_string(),
        ],
        metadata: BTreeMap::from([
            ("fetch_surface".to_string(), input.fetch_surface.to_string()),
            ("season".to_string(), input.season.to_string()),
            ("season_type".to_string(), input.season_type.to_string()),
            ("cache_targets".to_string(), input.cache_targets),
            (
                "mutation_mode".to_string(),
                input.acquisition_mode.to_string(),
            ),
            (
                "acquisition_mode".to_string(),
                input.acquisition_mode.to_string(),
            ),
            (
                "activation_rule".to_string(),
                input.activation_rule.to_string(),
            ),
            (
                "icelines_validation_floor".to_string(),
                input.validation_floor.to_string(),
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
    use fletch_core::cache_manifest;
    use httpmock::prelude::*;

    fn test_cache_entry(dataset_id: &str, verified: bool) -> CacheEntry {
        CacheEntry {
            dataset_id: dataset_id.to_string(),
            version: None,
            source_url: format!("https://example.test/{dataset_id}.json"),
            cache_key: format!("sha256:{dataset_id}"),
            relative_path: format!("objects/{dataset_id}.json"),
            sha256: format!("sha256:{}", "a".repeat(64)),
            bytes: 10,
            fetched_at_ms: 1,
            verified,
            fetch_attempts: 1,
            retry_count: 0,
            last_retryable_error: None,
        }
    }

    #[test]
    fn registry_marks_rosters_and_moneypuck_generic_http() {
        let report = fletch_source_handoff_report("20252026", "regular");
        assert!(report.registry_valid);
        assert_eq!(
            fletch_source_handoff_gate_failures(&report),
            Vec::<String>::new()
        );
        let requested_roster_id = roster_dataset_id("EDM", "20252026");
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == requested_roster_id
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
    fn current_roster_source_uses_current_endpoint_and_distinct_cache_id() {
        assert_eq!(
            roster_url("NYR", icelines_core::CURRENT_SEASON_STR),
            "https://api-web.nhle.com/v1/roster/NYR/current"
        );
        assert_eq!(
            roster_dataset_id("NYR", icelines_core::CURRENT_SEASON_STR),
            format!(
                "icelines.roster.{}.current.NYR",
                icelines_core::CURRENT_SEASON_STR
            )
        );
        assert_eq!(
            roster_url("NYR", "20242025"),
            "https://api-web.nhle.com/v1/roster/NYR/20242025"
        );
        assert_eq!(
            roster_dataset_id("NYR", "20242025"),
            "icelines.roster.20242025.NYR"
        );
    }

    #[test]
    fn registry_marks_paged_stats_sources_as_fletch_ready() {
        let report = fletch_source_handoff_report("20252026", "regular");
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.stats.20252026.regular.skater.summary"
                && row.source_kind == "http"
                && row.acquisition_mode == "generic-paged-json-cacheline"
                && row.handoff_status == "generic-fetch-ready"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.transactions.20252026"
                && row.source_kind == "adapter"
                && row.acquisition_mode == "generic-window-batch-http-cacheline-after-season"
                && row.handoff_status == "batch-expansion-ready-after-domain-set"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.ahl.20252026"
                && row.source_kind == "adapter"
                && row.acquisition_mode == "generic-batch-http-cacheline-after-season"
                && row.handoff_status == "batch-expansion-ready-after-domain-set"
        }));
    }

    #[test]
    fn registry_marks_player_landing_batches_as_fletch_ready_after_player_set() {
        let report = fletch_source_handoff_report("20252026", "regular");
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.contracts.20252026"
                && row.acquisition_mode == "generic-batch-http-cacheline-after-player-set"
                && row.handoff_status == "batch-expansion-ready-after-domain-set"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.career.20252026"
                && row.acquisition_mode == "generic-batch-http-cacheline-after-player-set"
                && row.handoff_status == "batch-expansion-ready-after-domain-set"
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
    fn query_partition_report_maps_query_surfaces_to_partition_rollups() {
        let report = fletch_query_partition_report("20252026", "regular");
        assert_eq!(report.schema_version, "icelines.fletch-query-partitions.v1");
        assert_eq!(
            fletch_query_partition_gate_failures(&report),
            Vec::<String>::new()
        );
        assert!(report.rows.iter().any(|row| {
            row.partition_id == "icelines.partition.20252026.regular.skater.summary"
                && row.rollup_id == "icelines.rollup.20252026.regular.query-stats"
                && row.source_fletch_ids == "icelines.stats.20252026.regular.skater.summary"
                && row.source_handoff_status == "source-byte-fletch-ready"
        }));
        assert!(report.rows.iter().any(|row| {
            row.partition_id == "icelines.partition.20252026.regular.moneypuck.skaters"
                && row.query_surface == "query-advanced-metrics"
                && row.source_handoff_status == "source-byte-fletch-ready"
        }));
        assert!(report.rows.iter().any(|row| {
            row.partition_id == "icelines.partition.20252026.regular.rosters"
                && row.source_handoff_status == "source-byte-fletch-ready"
        }));
        assert!(report.rows.iter().any(|row| {
            row.partition_id == "icelines.partition.20252026.career-history"
                && row.source_handoff_status == "batch-source-fletch-ready-after-domain-set"
        }));
    }

    #[test]
    fn query_partition_report_expands_both_season_types_for_stats_rollups() {
        let report = fletch_query_partition_report("20252026", "both");
        assert!(report.rows.iter().any(|row| {
            row.partition_id == "icelines.partition.20252026.regular.skater.summary"
        }));
        assert!(report.rows.iter().any(|row| {
            row.partition_id == "icelines.partition.20252026.playoff.skater.summary"
        }));
        assert_eq!(
            report
                .rows
                .iter()
                .filter(|row| row.partition_id
                    == "icelines.partition.20252026.regular.moneypuck.skaters")
                .count(),
            1
        );
    }

    #[test]
    fn query_quiver_report_groups_partition_members_by_season_type() {
        let report = fletch_query_quiver_report("20252026", "both");
        assert_eq!(report.schema_version, "icelines.fletch-query-quivers.v1");
        assert_eq!(
            fletch_query_quiver_gate_failures(&report),
            Vec::<String>::new()
        );
        assert!(report.rows.iter().any(|row| {
            row.quiver_id == "icelines.quiver.20252026.regular.query"
                && row
                    .member_partition_ids
                    .contains("icelines.partition.20252026.regular.skater.summary")
                && row
                    .member_partition_ids
                    .contains("icelines.partition.20252026.regular.moneypuck.skaters")
        }));
        assert!(report.rows.iter().any(|row| {
            row.quiver_id == "icelines.quiver.20252026.playoff.query"
                && row
                    .member_partition_ids
                    .contains("icelines.partition.20252026.playoff.skater.summary")
                && !row.member_partition_ids.contains("moneypuck")
        }));
        assert!(report.rows.iter().any(|row| {
            row.quiver_id == "icelines.quiver.20252026.regular.enrichment"
                && row.member_count == 3
                && row.source_ready_partition_count == 3
        }));
    }

    #[test]
    fn cache_index_report_maps_dynamic_cachelines_to_registered_sources() {
        let manifest = cache_manifest(
            "cache",
            vec![
                test_cache_entry("icelines.transactions.20252026.2025-10-01_2025-10-31", true),
                test_cache_entry("icelines.gamecenter.boxscore.2025020001", true),
                test_cache_entry("icelines.player.contracts.8478402", true),
                test_cache_entry("icelines.ahl.20252026.team.HFD.roster", true),
            ],
        )
        .expect("manifest should build");
        let report = fletch_cache_index_report("20252026", "regular", &manifest);

        assert_eq!(report.unexpected_index_count, 0);
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.transactions.20252026"
                && row.dataset_id == "icelines.transactions.20252026.2025-10-01_2025-10-31"
                && row.evidence_status == "indexed-verified"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.boxscore.20252026"
                && row.dataset_id == "icelines.gamecenter.boxscore.2025020001"
                && row.evidence_status == "indexed-verified"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.contracts.20252026"
                && row.dataset_id == "icelines.player.contracts.8478402"
                && row.evidence_status == "indexed-verified"
        }));
        assert!(report.rows.iter().any(|row| {
            row.fletch_id == "icelines.ahl.20252026"
                && row.dataset_id == "icelines.ahl.20252026.team.HFD.roster"
                && row.evidence_status == "indexed-verified"
        }));
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
        let manifest = read_fletch_cache_manifest(&fletch_cache_manifest_path(dir.path()))
            .expect("ICELINES should persist a FLETCH cache manifest");
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0].dataset_id, "icelines.test.source");
        let index = fletch_cache_index_report("20252026", "regular", &manifest);
        assert_eq!(index.source_schema, FLETCH_CACHE_INDEX_SCHEMA);
        assert_eq!(index.unexpected_index_count, 1);
    }

    #[test]
    fn cache_manifest_upsert_atomically_replaces_existing_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = fletch_cache_manifest_path(dir.path());

        upsert_fletch_cache_manifest_entries(
            dir.path(),
            [test_cache_entry("icelines.test.first", true)],
        )
        .expect("first manifest write should succeed");
        upsert_fletch_cache_manifest_entries(
            dir.path(),
            [test_cache_entry("icelines.test.second", true)],
        )
        .expect("existing manifest should be replaced atomically");

        let manifest = read_fletch_cache_manifest(&manifest_path)
            .expect("replacement manifest should remain readable");
        assert_eq!(manifest.entries.len(), 2);
        assert!(!dir.path().join("cache-manifest.json.tmp").exists());
    }

    #[test]
    fn fetch_generic_http_bytes_uses_cached_object_when_source_unavailable() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/source.csv");
            then.status(200)
                .header("content-type", "text/csv")
                .body("playerId,situation,icetime\n8478402,all,5000\n");
        });
        let dir = tempfile::tempdir().unwrap();
        let fletch_id = "icelines.test.cached-source";

        let fetched =
            fetch_generic_http_bytes(fletch_id, server.url("/source.csv"), dir.path(), false)
                .expect("initial source fetch should populate FLETCH cache");
        assert_eq!(
            fetched,
            b"playerId,situation,icetime\n8478402,all,5000\n".to_vec()
        );
        mock.assert_hits(1);

        let cached = fetch_generic_http_bytes(
            fletch_id,
            "http://127.0.0.1:9/source.csv",
            dir.path(),
            false,
        )
        .expect("offline fallback should reuse the cached object");
        assert_eq!(cached, fetched);

        let forced =
            fetch_generic_http_bytes(fletch_id, "http://127.0.0.1:9/source.csv", dir.path(), true);
        assert!(
            forced.is_err(),
            "forced refresh must not hide unavailable source behind cached bytes"
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

    #[test]
    fn fetch_paged_report_bytes_uses_fletch_paged_cache_object() {
        let server = MockServer::start();
        let first = server.mock(|when, then| {
            when.method(GET)
                .path("/skater/summary")
                .query_param("limit", "100")
                .query_param("start", "0");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":[{"playerId":1},{"playerId":2}],"total":3}"#);
        });
        let second = server.mock(|when, then| {
            when.method(GET)
                .path("/skater/summary")
                .query_param("limit", "100")
                .query_param("start", "100");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":[{"playerId":3}],"total":3}"#);
        });
        let dir = tempfile::tempdir().unwrap();

        let source_url = format!(
            "{}/skater/summary?cayenneExp=seasonId%3D20252026%20and%20gameTypeId%3D2",
            server.base_url()
        );
        let mut plan = fetch_plan_with_kind(
            "icelines.stats.20252026.regular.skater.summary",
            source_url,
            SourceKind::Http,
        )
        .unwrap();
        plan.cache_policy.freshness = FreshnessPolicy::AlwaysCheck;
        let outcome = fetch_paged_json_to_cache(
            &plan,
            FetchOptions::new(dir.path()).with_timeout_ms(1_000),
            PagedJsonOptions::default().with_limit(100),
        )
        .unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(outcome.outcome.path).unwrap()).unwrap();

        assert_eq!(value["data"].as_array().unwrap().len(), 3);
        assert_eq!(value["total"], 3);
        first.assert_hits(1);
        second.assert_hits(1);
    }

    #[test]
    fn fetch_gamecenter_batch_bytes_uses_fletch_batch_cache_objects() {
        let server = MockServer::start();
        let first = server.mock(|when, then| {
            when.method(GET).path("/gamecenter/2025020001/boxscore");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"id":2025020001}"#);
        });
        let second = server.mock(|when, then| {
            when.method(GET).path("/gamecenter/2025020002/boxscore");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"id":2025020002}"#);
        });
        let dir = tempfile::tempdir().unwrap();

        let bytes = fetch_gamecenter_batch_bytes_with_base(
            &server.base_url(),
            &[2025020001, 2025020002],
            FletchGamecenterArtifact::Boxscore,
            dir.path(),
            false,
        )
        .unwrap();

        assert_eq!(bytes.len(), 2);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes[&2025020001]).unwrap()["id"],
            2025020001
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes[&2025020002]).unwrap()["id"],
            2025020002
        );
        first.assert_hits(1);
        second.assert_hits(1);
    }

    #[test]
    fn fetch_player_landing_batch_bytes_uses_fletch_paced_batch_cache_objects() {
        let server = MockServer::start();
        let first = server.mock(|when, then| {
            when.method(GET).path("/player/8478402/landing");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"playerId":8478402,"seasonTotals":[]}"#);
        });
        let second = server.mock(|when, then| {
            when.method(GET).path("/player/8477934/landing");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"playerId":8477934,"seasonTotals":[]}"#);
        });
        let dir = tempfile::tempdir().unwrap();

        let bytes = fetch_player_landing_batch_bytes_with_base(
            &server.base_url(),
            &[8478402, 8477934],
            FletchPlayerLandingArtifact::Landing,
            dir.path(),
            false,
            1,
        )
        .unwrap();

        assert_eq!(bytes.len(), 2);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes[&8478402]).unwrap()["playerId"],
            8478402
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes[&8477934]).unwrap()["playerId"],
            8477934
        );
        first.assert_hits(1);
        second.assert_hits(1);
    }

    #[test]
    fn fetch_transactions_batch_uses_fletch_window_cache_objects() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/transactions");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{
                      "transactions": [
                        {
                          "date": "2026-04-29",
                          "description": "Recalled F Test Player from Bakersfield",
                          "team": { "id": "22", "abbreviation": "EDM", "displayName": "Edmonton Oilers" }
                        }
                      ]
                    }"#,
                );
        });
        let dir = tempfile::tempdir().unwrap();

        let outcome =
            fetch_transactions_batch_with_base(&server.base_url(), "20252026", dir.path(), false)
                .unwrap();

        assert_eq!(outcome.rows.len(), 1);
        assert_eq!(outcome.rows[0].team.as_ref().unwrap().abbreviation, "EDM");
        assert!(!outcome.partial);
        assert!(mock.hits() >= 11);
    }
}
