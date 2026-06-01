# Requirements

## Scope

Repo or feature: `icelines` repo-baseline VTRACE adoption.

These requirements derive from `MISSION.md` and the ten `CONOPS.md` scenarios.
They are traceability requirements for the current IceLines platform and for
explicit targets called out by the mission. A status of `target` means the
requirement is intentionally not claimed as met today.

## Requirement Table

| ID | Requirement | Parent Need / Scenario | Rationale | Priority | Owner | Verification Method | Status |
|---|---|---|---|---|---|---|---|
| REQ-WB-001 | The TUI workbench shall let an analyst launch the default workbench, switch to the Stats workspace, apply a shared `icelines-query` filter, cycle left/right panes, time-travel seasons, and open a player card without leaving keyboard flow. | CON-001; power user / analyst | The core loop is ask -> see -> reshape -> act -> repeat. | must | Jack Adams / Messier | demonstration + L2 TUI checks | accepted |
| REQ-WB-002 | Every interactive analytical surface shall keep the active `(season, season_type)` visible when it affects the result. | MISSION Constraints; CON-001; CON-003 | Silent time-travel makes every statistic ambiguous. | must | GLASS / Campbell | inspection + surface tests | accepted |
| REQ-WB-003 | Users shall be able to name, save, restore, and update custom workbench layouts across TUI and Web without storing semantic hockey state only in a renderer-local format. | MISSION Success Criteria; CON-001; CON-003 | "Personalized workbench layouts" is a mission target and must not remain prose-only. | should | Jack Adams / GLASS | layout persistence demo + compatibility inspection | target |
| REQ-QUERY-001 | CLI flags, TUI cmdbar input, web query params, and AI fallback output shall lower through the deterministic Art Ross query parser/planner before execution. | Contract 2; CON-001; CON-004 | Cross-surface parity requires one typed intent, not renderer-local parsing. | must | Art Ross / Campbell | test + inspection | accepted |
| REQ-STAT-001 | Historical streak/stat answers shall report historical ranking or context and shall not present "perspective" as era-adjusted, predictive, betting, deployment-adjusted, or linemate-adjusted analysis. | MISSION Need; CON-002 | Public claims must be citable and honest about what the tool computed. | must | PACE / SCOUT | demonstration + export inspection | accepted |
| REQ-STAT-002 | Stat-in-perspective validation shall cover, as separate expected observations, 2004-05 lockout omission, October `CURRENT_SEASON` rollover, ambiguous player names, intra-season trade continuity, active streak "ongoing" labeling, and skeleton/snapshot-only completeness disclosure. | CON-002; R2-CONOPS-05 | One broad social-post scenario is too large to verify as one assertion. | must | EDGE / TAPE / BENCH | fixture analysis + demonstration | accepted |
| REQ-REPORT-001 | Markdown, JSON, CSV, and public report/export artifacts shall disclose source/completeness state near the top and shall avoid language that implies era normalization, betting prediction, or deployment-quality adjustment. | CON-002; Contract 5; R2-CONOPS-07 | The limitation must survive from mission wording into the artifact a user shares. | must | SCOUT / PACE / Jim Gregory | inspection + report snapshot tests | accepted |
| REQ-WEB-001 | The web dashboard shall cold-start to a no-JS-readable shell where a new user can find leaders, scores, a player card, and playoffs; controlled workspace/pane state shall be bookmarkable in the URL. | CON-003 | New users need discoverability without CLI knowledge. | must | Ted Lindsay / CREST | browser demonstration + route tests | accepted |
| REQ-WEB-002 | Browser validation shall include visible active context above the fold, color-not-alone state, narrow viewport behavior, unknown-route recovery, overconstrained-filter empty-state recovery, and a warning when binding to `0.0.0.0`. | CON-003; R2-CONOPS-06 | First-impression quality and safety must be observable, not taste-only review. | should | CREST / broadcast / GLASS | browser inspection + route tests | accepted |
| REQ-PARITY-001 | Any capability rendered by more than one surface shall compare one canonical ViewModel/envelope across CLI text/JSON, TUI, web HTML, web JSON, and export/static artifacts where applicable; row identity, applied filters/sort, warnings, and source/completeness state shall match. | Contract 4; CON-004; R2-CONOPS-02 | "Same answer everywhere" requires a controlled comparison artifact. | must | Campbell / Ted Lindsay / BENCH | automated parity tests + inspection | accepted |
| REQ-DATA-001 | All analytical results shall carry `ViewContext`-equivalent source state with season, season type, completeness, provenance, freshness when known, and typed missing-source/unavailable state instead of silent zeroes. | Contract 1; CON-005; CON-008; R2-CONOPS-03 | Trustworthy data is the core safety value. | must | HART / KEEL / WIRE | unit tests + interface inspection | accepted |
| REQ-OFFLINE-001 | The default workspace and bundled-backed historical queries shall run with zero network calls; snapshot-only/live-only domains shall render explicit `MissingSource` or unavailable state when offline. | CON-005 | Offline-first must not become wrong-but-confident output. | must | Foster / WIRE | offline smoke test + demonstration | accepted |
| REQ-DATA-DEPTH-001 | Data-depth commands shall install seasons, fetch boxscore-level detail for selected scopes, show freshness/provenance by kind, verify snapshot integrity, resume partial fetches, and refuse locked shift-level tracking with an explicit message. | CON-006 | The product must be honest about boxscore depth versus shift-level targets. | must | Foster / BENCH | command tests + demonstration | accepted |
| REQ-FANTASY-001 | Fantasy poach, gaps, import, and simulation read flows shall render shared ViewModels across CLI/TUI/Web where available; deferred web mutations shall route users to CLI/TUI instead of mutating through GET. | CON-007 | Fantasy decisions need repeatable shared read models and safe mutation boundaries. | should | Selke / Ted Lindsay | L2 tests + inspection | accepted |
| REQ-FRESH-001 | Fresh-data refresh shall validate cache integrity before deserialization, fail loud on schema drift or newer bundle schemas, honor retry/backoff signals, and surface absent MoneyPuck/Realtime/Contracts sources as typed missing state. | CON-008 | Upstream failure is normal and must degrade safely. | must | WIRE / HART | integration tests + code inspection | accepted |
| REQ-DEP-001 | The workspace shall have no FLETCH or SLICE path/git dependencies before standalone compliance is claimed; any removed command surface shall have a documented replacement, refusal, or rollback path. | CON-009; R2-CONOPS-04 | Current `Cargo.toml` still contains cross-repo dependencies, so the mission target must stay traceable. | must | KEEL / FORGE | dependency inspection | target |
| REQ-LEAN-001 | A lean build target shall compile and run an offline CLI with `--no-default-features --features cli`, excluding web, TUI, and network crates unless explicitly opted in. | CON-009; MISSION Success Criteria | Dependency-minimal delivery is a target, not a present-tense fact. | should | FORGE / Jim Gregory | build smoke | target |
| REQ-CACHE-001 | The platform shall define a versioned major analytics cache record that names scope, source window, producer version, provenance, freshness/staleness, quality/completeness, warnings, invalidation keys, and consumer contract version for every cached result. | CON-010; Coach / analyst | Future hockey front ends need one trusted evidence layer rather than per-screen recomputation. | must | HART / Campbell / BENCH | design inspection + fixture tests | target |
| REQ-CACHE-002 | Cache build paths shall read only explicit local/bundled/snapshot source state, shall not perform query-time live fetches, and shall refuse missing, stale, partial, schema-incompatible, or unsupported metric inputs with typed states instead of zero-filled success. | CON-010; Contract 1; CON-005; CON-008 | A major cache amplifies wrong data unless freshness, missing-source, and rebuild semantics are controlled. | must | WIRE / TAPE / HART | source-state tests + cache build fixtures | target |
| REQ-CACHE-003 | Cache consumers shall read canonical analytics envelopes and disclosure fields without recomputing ranking, confidence, source-state, or hockey semantics locally. | CON-010; Contract 3; Contract 4 | Coach dashboards, scout reports, player cards, line explorers, goalie views, practice reports, and postgame reports must agree on prepared evidence. | must | Campbell / Ted Lindsay / Jim Gregory | consumer contract tests + review | target |
| REQ-CACHE-004 | Cache-driven product copy shall present analytics as decision support only and shall not claim autonomous coaching authority, prediction accuracy, betting value, injury certainty, line-chemistry causality, or complete-world truth unless a later controlled requirement and validation evidence authorizes the claim. | CON-010; MISSION Non-Goals; SCOUT/PACE limits | Hockey decision screens must help humans reason without overstating model authority. | must | SCOUT / PACE / BENCH | text review + validation demo | target |
| REQ-CODE-001 | Repo gates shall remain green for the affected slice: formatting, clippy with warnings denied, build, and L0/L1/L2 tests appropriate to changed surfaces. | MISSION Constraints; CON-009 | Traceability is useful only if code rigor remains visible. | must | Jim Gregory / BENCH | command evidence | accepted |

## Requirement Quality Checklist

- [x] Each requirement is clear.
- [x] Each requirement is feasible.
- [x] Each requirement is verifiable.
- [x] Each requirement has an owner.
- [x] Each requirement links to a mission need or CONOPS scenario.
- [x] Each requirement avoids implementation detail unless the detail is itself required.

## Deferred Requirements

| ID | Reason Deferred | Revisit Trigger |
|---|---|---|
| REQ-WB-003 | Named layout persistence is a mission target, but no storage schema, migration rule, or cross-surface restore evidence has been recorded. | Layout schema/design and TUI/Web restore implementation are ready for review. |
| REQ-DEP-001 | FLETCH/SLICE dependencies still exist in `Cargo.toml`; removal is a product target, not a completed fact. | Dependency replacement or removal PR is ready for review. |
| REQ-LEAN-001 | Feature boundaries for `cli`, `web`, `tui`, and `net` are not yet implemented. | Cargo feature surgery begins or a release claims lean/offline CLI support. |
| REQ-CACHE-001..004 | Major analytics cache is a newly accepted product direction and specification baseline, not implemented cache behavior. | WP-009 cache design/implementation begins or any coach/scout/report surface claims to consume cached analytics. |
