# ICELINES FLETCH Source Orchestration Role Review

## Boundary

- FLETCH owns cacheline identity and generic HTTP acquisition for stable public source objects.
- ICELINES owns hockey semantics: snapshots, manifests, freshness, locks, API pagination, batch expansion, player/game source discovery, parsing, and validation.

## Role findings

- **HART:** Source identities must respect the `(player_id, season, season_type)` model axis. FLETCH cacheline IDs may include season/type context, but ICELINES must keep the canonical `StatsRepository` and snapshot key semantics.
- **KEEL:** Live APIs are write paths, not query-time fallback tiers. FLETCH can feed snapshot writes, but ICELINES still owns the per-source persistence chain and all four-surface convergence.
- **TAPE:** Downloaded bytes are not trustworthy analytics rows. ICELINES must still enforce player identity, season/type filtering, snapshot integrity, ESPN team-abbrev mapping, and MoneyPuck CSV interpretation.
- **FORGE:** The dependency graph stays `icelines-core` free of I/O. FLETCH integration belongs in `icelines-fetch`, with `icelines-cli` only invoking the fetch/handoff API and adding CLI context.
- **PACE:** No FLETCH doc should imply changed scoring, thresholds, or complexity claims. This migration changes acquisition only.
- **BENCH:** L0 coverage is sufficient for registry/handoff shape; execution migration needs tempdir + mocked HTTP before any live fetch path is changed.
- **EDGE:** Keep season-boundary and playoff leakage cases visible. Paged stats, player-landing batches, gamecenter expansion, ESPN windows, and schedule-derived batches stay adapter-required until FLETCH has explicit primitives for those edge cases.
- **WIRE:** Preserve endpoint-specific retry/schema behavior. Generic FLETCH HTTP is safe first for stable single-object sources; paged NHL stats and ESPN windows are not generic yet.
- **SCOUT:** No hockey-domain claim changes. The migration cannot alter line assignment, roster interpretation, goalie discrimination, or transaction semantics.

## Decision

Start with handoff/gate plus generic-ready roster and MoneyPuck source inventory. The first execution slice moves those stable single-object HTTP acquisitions through FLETCH after mocked HTTP coverage; ICELINES still parses and seals the resulting snapshots. Keep paged stats and dynamic batches adapter-required until FLETCH models pagination, source-set expansion, and rate-limit behavior explicitly.
