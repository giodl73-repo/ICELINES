# Shared seasonal-record engine

The two observational families now use shared Rust rather than separate export-only
classification logic. Native callers use `evaluate(&Input)` or `evaluate_json(bytes)`
in `icelines_core::view_model::rinkcraft_seasonal_records`. The native example
`rinkcraft_seasonal_records` verifies a transport checksum and writes a new output.
The existing skating/shooting export helpers delegate their classification to the
shared function; shooting also delegates conversion and checked midrank arithmetic.

`BrowserEngine.seasonal_record_catalog` exposes the same byte function. The
`seasonalRecords` worker operation accepts an owned ArrayBuffer and returns the
parsed catalog through the existing context-generation/error protocol. No package
must be resident; no package is loaded, selected, mutated or persisted by this call.

## Input and output contract

The versioned input is a regular-season/all-strength supplied cohort. Each record
carries identity, season, position group, GP, minutes, nullable official goals/SOG,
an official source hash and optional typed EDGE evidence. Tracking identity, scope,
GP, URL, hash, capture time, used units and domains must match the official record.
The envelope denies unknown fields. The byte limit is 32 MiB, with 10,000 records.
Goalies are outside these two skater families and are rejected, not compared against
skaters. Missing tracking remains unknown; observed zero counts stay zero.

Metric values are skating maximum, burst count, bursts/60, shot-speed maximum,
SOG/60 and raw conversion. Official TOI is not equivalent task opportunity time;
maxima and activity are not capability ratings. Conversion is diagnostic only,
with the requested minimum-SOG gate and no finishing award. Missing, zero-SOG or
goals-above-SOG conversion stays unknown.

References are separate by season, position and metric. The input is sorted by
actor/window before building references and the canonical input fingerprint.
Every metric carries its source lineage, peer count and reference hash. Reference
hashes bind the pool key, GP/TOI gates, relevant conversion gate, sorted members,
values and sources. Changing only conversion exposure must not change a volume
or skating reference hash. One initial test caught that coupling; the corrected
hash includes conversion settings only for conversion.

The output explicitly says **caller-supplied cohort, not certified NHL-wide**.
A source hash is a fingerprint, not source authentication or proof of population
completeness. Acquisition, source verification and manifest completeness remain
caller responsibilities. No network or filesystem access occurs in the core or
WASM byte function.

P90, minimum 20 measured peers and the exclusive family definitions remain the
version-1 authored hypotheses. Each family emits at most one candidate, with
assessed, unassessed and insufficient-exposure states. Neither family supplies a
capability score, production feat award, documentary class or simulation bonus.
The earlier public profile fields, native profile response and snapshot sources
are not automatically rewritten by this API.

## Validation and reproduction

Five core tests passed: boundaries/exclusivity, null/zero, source/identity/unit
binding, independent season/position references, ordering, sample-gate independence
and invalid cohorts. The two legacy skating and three legacy shooting tests also
passed. Both complete legacy exports remain byte-identical after delegation.

The prepared manifest verifies 3,941 source profile hashes and supplies 3,565 skaters.
Native Rust 1.98.0 and actual WASM built with pinned Rust 1.95.0 / wasm-bindgen
0.2.118 produced byte-identical catalogs for five settings: 5/10/20 GP at 25 SOG,
and 10 GP at 10/50 SOG (all with 100 minutes). The independent frozen-export
comparison checked 66,640 legacy ranks and 28,500 legacy decisions across those
settings. Default labels remain 446 skating and 506 shooting across four seasons.

The test executes the generated WASM binding, not a JavaScript reimplementation.
It also invokes the real browser worker through EngineClient: transfer detachment,
full default-catalog equivalence, resident revisions unchanged, error recovery,
current-season operation and zero page errors all passed. Nine malformed/oversize
cases were rejected. The existing 16 EngineClient lifecycle tests and TypeScript
compilation passed. A native `cargo check -p icelines-wasm --locked` also passed.
The WASM witness is a development build, not a production/browser-release build.

```text
cargo test -p icelines-core --lib rinkcraft_seasonal_records --locked
cargo test -p icelines-sources --example rinkcraft_edge_feat_candidates --locked
cargo test -p icelines-sources --example rinkcraft_shooting_records --locked
cargo build -p icelines-sources --example rinkcraft_seasonal_records --locked
cargo check -p icelines-wasm --locked
python tools/verify_rinkcraft_seasonal_records.py prepare BASE_DIRECTORY EXPOSURE_JSON NEW_INPUT_JSON NEW_MANIFEST_JSON
cargo run -p icelines-sources --example rinkcraft_seasonal_records --locked -- INPUT_JSON SHA256 NEW_OUTPUT_JSON
python tools/verify_rinkcraft_seasonal_records.py check INPUT_JSON OUTPUT_JSON EXPOSURE_JSON SKATING_JSON SHOOTING_JSON NEW_RECEIPT_JSON
```

For the actual WASM witness, build `icelines-wasm` for wasm32-unknown-unknown with
Rust 1.95.0 into an isolated target directory; invoke wasm-bindgen 0.2.118 with
`--target web`. Compile TypeScript, then set `ICELINES_SEASONAL_NATIVE` to the built
native executable and `ICELINES_SEASONAL_PKG` to the isolated bindings directory
before running `node icelines-browser/e2e/rinkcraft-seasonal-engine.mjs`.
The acceptance harness requires unused output filenames. Its optional evidence
directory is `ICELINES_SEASONAL_EVIDENCE`; the retained fixture paths are documented
in the dated provenance. Build inputs, witness artifacts and checks stay outside Git.

No public Site, main branch, hosted CI workflow or existing IceLines Pages release
changed. This adds a reusable calculation API and worker access; automatic profile
assembly, browser source acquisition and production deployment remain separate work.
