# IceLines Sources S1 — Crate Scaffold and Player-Landing Extraction

**Date:** 2026-07-31
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally; repository CI target matrix remains the merge gate

## Delivered

- Added `icelines-sources` as a workspace crate depending on `icelines-core`
  plus deterministic parsing libraries only.
- Added validated source/adapter/version/hash identifiers, caller-supplied
  `SourceInput`, `SourceDescriptor`, a categorized `AdapterError`, and the
  `SourceAdapter` trait without introducing S2 fact/package semantics.
- Moved NHL player-landing career and dated organization parsing to
  `icelines_sources::nhl::player_landing`.
- Preserved `icelines_fetch::career_landing::{parse_career_history,
  parse_official_nhl_organization_fact, CareerParseError,
  OfficialNhlOrganizationFact}` through compatibility re-exports.
- Kept HTTP/FLETCH acquisition and the career-history filesystem store in
  `icelines-fetch`.
- Corrected `design/ARCHITECTURE.md` to the real workspace dependency edges,
  including the existing fetch-to-query dependency.

No serialized artifact or cache identity changed. The frozen landing input hash
remains guarded by the S0 compatibility test, while the existing 20 fetch
career-landing tests now execute the parser through the compatibility facade.

## Dependency boundary

```text
icelines-sources -> icelines-core
icelines-fetch   -> icelines-core, icelines-query, icelines-sources
```

The source-crate architecture test rejects direct network, async runtime,
FLETCH, SQLite, Web/TUI/CLI, fetch, or query dependencies.

## Verification

```text
cargo test -p icelines-sources
6 passed; 0 failed

cargo test -p icelines-fetch career_landing --lib
20 passed; 0 failed

cargo test -p icelines-fetch --test source_module_inventory
3 passed; 0 failed

cargo clippy -p icelines-sources -p icelines-fetch --all-targets -- -D warnings
passed

cargo check --workspace
passed
```

S2 is next and remains a separate semantic slice: reviewed fact assertions,
identity proposals/decisions, run manifests, freshness, coverage, disclosures,
and the first source-package schema.
