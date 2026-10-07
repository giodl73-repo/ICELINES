# Browser WASM Workbench

**Date:** 2026-10-03
**Status:** Active partial implementation — Pages shell update, live relay access and explicit stats/schedule persistence verified on 2026-10-06. Live report completeness correction, deployed offline/rollback and device/resource gates remain open.
**Scope:** A GitHub Pages application running the shared IceLines Rust engine on the user's device, with release packages, imported files, live refresh, and optional local persistence.
**Review:** [Planning review](../../signals/roles/check/browser-wasm-workbench-roles-check-2026-10-03.md); [implementation handoff review](../../signals/roles/check/browser-wasm-handoff-roles-check-2026-10-04.md) (release needs work).

## Decision summary

Build a browser surface distributed through GitHub Pages, with the shared Rust
engine compiled to WASM and owned by a dedicated worker. Start with season
selection, skater/goalie analysis, player details, filtering, and data management.
Expand other native UI surfaces only after this complete path passes its gates.

| User action | Session behavior | Persistence behavior |
|---|---|---|
| Load catalog package or import a file | Validate and use in memory | No automatic save |
| Refresh latest supported data | Replace only after complete validation | No automatic save by default |
| Save locally | Continue using the active data | Save a validated snapshot in IndexedDB |
| Enable Keep updated locally | Apply successful refreshes | Save replacements for that dataset/context |
| Remove saved copy | Keep the loaded session copy | Delete the saved copy |
| Unload from session | Release resident data | Keep any saved copy |
| Export | Download package or query result | User-managed backup outside browser storage |

Public release archives are build inputs for a versioned, lazy-loaded catalog.
The existing Pages documentation stays at `/ICELINES/`; the browser is composed
under `/ICELINES/workbench/`. Switching legacy Pages publishing to Actions is an
explicit rollout step, with documentation publication coordinated and rollback
acceptance still required; see [rollout](../../icelines-browser/PAGES.md).
LFS inputs use the same conversion path if actual assets are identified. Live
stats and schedule acquisition are separate adapters; their browser access must
be proven from the deployed origin. If direct access fails, record a constrained
relay's hosting and operating owner before implementing it.
User decision on 2026-10-04: choose relay hosting after deployed-origin testing.
No relay provider or operating owner is allocated yet.

The delivery order is engine/contracts → one-season parity → local library →
live refresh → browser UX → Pages release. Current implementation evidence is
listed in section 12; local offline reopening passed in pulse 27. Deployment,
live access, deployed offline/update behavior, mobile, and other release gates
remain open. The roles verdict approves planning
with conditions and is not a release approval.

## 1. Outcome and first release

Users open IceLines from the project website, choose a season and season type,
run shared queries, inspect players and goalies, refresh supported data, and
choose which datasets survive closing the browser. Imported personal data stays
on their device. GitHub Pages distributes the application and selected public
packages; it does not run the Rust engine or a backend.

The first release includes a catalog/library, skater and goalie leaderboards,
player details, shared filtering, JSON/CSV result export, package import/export,
explicit refresh, and local save/remove controls. A complete first slice covers
one season end to end before adding all historical packages. Live season stats
and scores/schedule are the first acquisition families; the release must prove
a working live refresh path, rather than replacing that requirement with static
release downloads.

Later waves extend compare, historical/career queries, team views, workbench
layouts, transactions, advanced metrics, cards, and fantasy workflows. Native
SQLite league-management parity and every existing dashboard route are outside
the first release. Each promoted surface needs its own parity evidence.

## 2. Observed baseline

- `icelines-core` owns the domain model, calculations, view models, and in-memory
  `StatsRepository`. It has some filesystem-backed workbench-layout helpers that
  need a native boundary; a WASM build alone will not make those APIs usable.
- `icelines-query` owns parser/planner/executor and the injected `DataProvider`
  seam. Its pinned `slice-core` dependency currently declares serde/JSON/error
  dependencies only; actual WASM compilation remains a gate.
- `icelines-sources` contains reusable parsing and normalization boundaries.
  `icelines-fetch` couples acquisition and loading to native filesystem, SQLite,
  Tokio, reqwest, and FLETCH. Do not import that entire dependency graph into WASM.
- `icelines-web` uses Axum routes and Askama/HTMX presentation. A static browser
  application needs new orchestration; existing CSS, tokens, and view contracts
  are reusable. This does not retire `icelines serve`.
- GitHub inventory inspected in this session contains 38 `data-*` season releases,
  1987-88 through 2025-26 excluding the 2004-05 lockout. The inspected 2025-26
  archive is 122,868 bytes, published May 16, 2026, and has a SHA-256 asset digest.
  Its body advertises bios, stats, goalies, playoff files, and a manifest. Archive
  contents and freshness still require validation; the date is not evidence of
  final-season completeness.
- `.github/workflows/data-bundle.yml` currently targets those 38 seasons. Its
  release manifest includes timestamps/counts but not a full browser package
  integrity contract. Older archives can omit goalies/playoffs.
- No LFS rules or tracked LFS files were found in this checkout. LFS support is
  conditional on locating actual assets in another branch/repository.
- The WASM Rust target is installed locally; wasm-bindgen CLI/wasm-pack was not
  found on PATH. Tool versions must be pinned by the implementation pipeline.

## 3. Architecture and ownership

```text
Pages: application shell + WASM + versioned public package catalog
                             |
Browser UI -- async messages -- dedicated worker
                             | Rust engine + query + normalized repository
                             | package decoding / validation / view projection
                             |
                session datasets + saved IndexedDB library

Explicit refresh --> browser fetch --> supported upstream
                                  --> constrained relay if required
Release/repository/LFS inputs --> build-time package publication
User files -------------------> local import / local export
```

Add `icelines-wasm` as a thin Rust binding crate using wasm-bindgen, core, query,
and sources. Use a small TypeScript browser shell in `icelines-browser`, built
with a pinned toolchain. UI state and browser APIs belong there; hockey formulas,
filter semantics, source quality, and projections remain in shared Rust.

Extract portable byte-to-model loading from `stats_loader` into an appropriately
named shared library boundary, with native loaders delegating to it. Do not
create a separate JavaScript stats model or duplicate native normalization. A
new portable data crate is acceptable if extraction would invert current crate
ownership; the build spike must document the chosen DAG before implementation.

The worker exclusively owns `StatsRepository`. Send owned input bytes and owned
view DTOs across the boundary, never borrowed `PlayerView` values or Rust
pointers. Start with one worker and single-threaded WASM; shared-memory threads
and host-specific isolation headers are unnecessary for the first slice.

Browser acquisition is asynchronous. Complete and validate acquisition before
executing a query; the synchronous `DataProvider` reads prepared resident data
and returns explicit missing requirements. Rendering/querying does not silently
fetch or persist. Provide a visible action to load missing catalog packages.

The existing localhost HTMX/no-build-step policy continues for `icelines-web`.
The user-requested static WASM surface has a separate build pipeline and requires
JavaScript; a no-JS visitor receives an explanation and native download link.

## 4. Data identity, selection, and storage

Keep canonical player identity and `(player_id, season, season_type)` stats keys.
Catalog records additionally identify source family, package ID/version, data
schema, content digest, provenance, completeness, observed/fetched timestamps,
and required engine version. Keep persistence policy separate from freshness
and completeness. Download time never becomes the observation time.

Default acquisition/import policy is **session only**. Every dataset displays
either **In memory** or **Saved locally**. **Save locally** persists a validated
snapshot. **Keep updated locally** is an optional per-dataset setting: later
successful refreshes save complete validated replacements. Explicit refresh
does not enable that setting. **Remove saved copy** preserves the session copy;
**Unload from session** preserves any saved copy. A memory eviction never deletes
the saved version. A saved-library manifest is metadata, not a second stats model.

Use IndexedDB for the first implementation, with a versioned application
namespace, package blobs, metadata, and active pointers. OPFS is deferred until
measurements justify file-oriented storage. Local file picking/download export
is the portable baseline; direct file handles are an optional enhancement.

Build replacement data off to the side, then activate it after validation. Failed
refreshes retain the last valid active dataset. Failed saves leave it usable in
memory with a visible unsaved state and preserve the previous saved snapshot.
Incomplete optional source families may be stored as explicitly partial audit
records, but never advertised as a complete dataset.

Selection is explicit per source family and season/type. A session refresh can
override the saved version for that session. On restart use the saved active
version; if none exists, offer the published catalog package. Never combine
overlapping versions of one family by adding cumulative totals. Package selection
does not imply one universal fallback chain for every native source family.

## 5. Live acquisition and freshness

WP-BW-01 must probe browser requests from a deployed/representative Pages origin,
including redirects and preflight where applicable. Probe NHL stats, schedule,
score/game endpoints, and GitHub release-asset delivery separately. Native curl
success is not proof of browser CORS access. Record per-source endpoint, response
schema, CORS result, season coverage, rate limits, and recovery behavior.

Prefer direct fetch for sources that pass. For blocked sources, design an optional
HTTPS relay with fixed source adapters, allowlisted parameters and upstream hosts,
redirect validation, response/timeout limits, and shared caching/rate limits.
It must never be an arbitrary-URL proxy. Hosting and credentials belong to the
relay service, never the Pages bundle. A deployment choice and operating owner
must be recorded before implementing that service. Release-package publication
is useful background distribution, but is not a substitute for on-demand live
refresh. The live release gate cannot pass until direct or relay acquisition
works in a real browser. Do not offer an unverified public proxy as a fallback.

Provide Refresh for the active family/context, and opt-in auto-refresh while the
page is visible. Initial policy: at most two requests concurrently, bounded
retries with exponential backoff/jitter, respect Retry-After, and no overlapping
polls. Choose family-specific TTLs from existing freshness policy, with source
observation time displayed. Pause polling in hidden/offline tabs and coordinate
requests across tabs. An unavailable source remains visibly unavailable/stale;
do not replace missing values with zero or quietly use another season.

## 6. Releases, repository files, and LFS

Generate a versioned browser catalog during the Pages build. Pin input release
asset IDs/digests, validate archives, and publish chosen packages as ordinary
static assets on the Pages origin. This makes runtime package loading independent
of GitHub API quotas and release-download CORS. Keep release provenance in each
entry. Downloads of original release archives remain available for manual import.

Validate advertised files, context, schema, counts, and completeness before
publication. Generate SHA-256 digests for each file plus the whole package.
Legacy missing optional files become explicit missing-source metadata. Legacy
packages without trusted digests need a validated build-time conversion, not a
claim that their original manifest provided integrity assurance.

If actual LFS inputs are identified, the build checks out their real bytes with
`lfs: true`, verifies that no pointer text remains, and packages only necessary
public data. GitHub Pages does not directly serve LFS pointers. Publish a small
starter package and lazy-load historical seasons; enforce Pages size budgets
rather than copying every repository artifact into the website.

## 7. Delivery sequence and gates

| Work package | Deliverable | Exit evidence |
|---|---|---|
| WP-BW-01: feasibility and contracts | WASM dependency/build spike; shared loader extraction design; live/CORS source matrix; catalog and worker contracts; initial VTRACE change proposal | Core/query/source WASM build, one fixture projection, documented native dependency exclusions, real-browser source probes, relay disposition |
| WP-BW-02: engine slice | WASM bindings, worker, shared prepared-data provider, one season loader, leaderboards/player/goalie query | Same fixture input produces native/WASM parity for identity, rows, ordering, context, source state, and missing values |
| WP-BW-03: data library | IndexedDB session/save lifecycle; validated package/import/export; historical catalog | Restart recovery, temporary-data absence, failed-save rollback, quota/schema recovery, archive rejection evidence |
| WP-BW-04: live refresh | Live stats and score/schedule adapters; direct/relay path; staged activation; optional saved refresh | Working deployed-origin refresh, mocked failure suite, stale-state evidence, complete/partial distinction, previous-good preservation |
| WP-BW-05: browser product | Responsive shell, query controls, data panel, accessible states, URL context | Desktop/mobile browser evidence, keyboard flow, deep-link reload, export, worker recovery, storage-loss flow |
| WP-BW-06: Pages distribution | Reproducible build, catalog conversion, preview artifact, publishing workflow | Project-subpath smoke, hashes/MIME/worker loading, reviewed preview, size measurements, previous-version rollback |
| WP-BW-07: additional surfaces | Compare/history/team/cards/fantasy in separately scoped waves | Per-surface contract and native/WASM/browser parity; no blanket full-dashboard claim |

Dependencies: 01 precedes 02; 02 precedes 03 and 04; 03/04 precede the complete
05/06 release gate. Shared native-loader refactors must keep native consumers
building at every step. A browser-first Rust framework is not needed to prove
this slice; revisit only if concrete UI complexity justifies it.

## 8. Verification and release acceptance

L0 Rust fixtures lock identity, multi-stint sum invariants, regular/playoff
separation, nullable fields, filter semantics, ranking thresholds, and schema
refusal. L1 fixture-based native/WASM comparisons exercise real loaders and
shared projections. Browser automation uses mocked requests, never live upstream
APIs, for acquisition failures, persistence, exports, and UI behavior. Manual
deployed-origin live checks are separate environment evidence, not unit tests.

Release acceptance requires: open the Pages subpath; load a published season;
filter leaders and inspect a goalie/player; refresh supported data into memory;
save selected data; reload to restore saved data; confirm session-only data was
not retained; import/export a package; and use saved data when the network fails.
Installable/offline-shell support needs a separately verified service-worker
asset strategy; cached data alone does not prove the application opens offline.

Record toolchain/build IDs and command results. Run affected native tests,
WASM/browser checks, formatting and lint gates; inspect keyboard/mobile and
source-state screens. Measure download size, peak memory, decompression cost,
query latency and worker recovery on representative desktop/mobile devices.
Performance numbers are evidence to collect, not current capability claims.

## 9. Baseline integration and planning status

Parent constraints include REQ-WB-002, REQ-QUERY-001, REQ-PARITY-001,
REQ-DATA-001, REQ-FRESH-001, REQ-REPORT-001 and IF-DATA-001, IF-VIEW-001,
IF-QUERY-001, IF-FETCH-001, IF-BUILD-001. Before feature code, allocate noncolliding
CHG/WP IDs in VTRACE for the new browser surface and local-state schema, and link
requirements, interfaces, trace, work packages, verification and validation.
Do not reinterpret existing native lean/standalone targets as passed by WASM.

The foundation is allocated to REQ-BROWSER-001 and WP-BW-01 in VTRACE. Broader
portfolio promotion remains a separate roadmap decision. The plan records
authorized implementation and a reviewable sequence without advancing
shipped-capability statuses.

## 10. Sources

- [GitHub Pages architecture](https://docs.github.com/en/pages/getting-started-with-github-pages/what-is-github-pages)
- [Inspected season release](https://github.com/giodl73-repo/ICELINES/releases/tag/data-20252026)
- [Git LFS and Pages limitations](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-git-large-file-storage)
- [Actions checkout LFS option](https://github.com/actions/checkout)
- [Browser CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS)
- [IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API)
- [WASM worker bindings](https://github.com/wasm-bindgen/wasm-bindgen/blob/main/guide/src/examples/wasm-in-web-worker.md)

## 11. Roles review amendments — delivery contract

The 12-role review is approved with conditions for planning. The following
refinements are part of this plan; all runtime gates remain unverified.

### A. Shared engine, revisions, and worker protocol

- Before WP-BW-02, choose and document the portable loader's crate ownership.
  Native `load_into_repo` must delegate to the same normalization functions used
  by WASM. Test traded-player stints, goalie discrimination, identity merges,
  accented names, and missing optional sources against hand-audited fixtures.
- Use versioned worker envelopes with `request_id`, `context_generation`,
  operation, and payload. Responses repeat those identifiers and contain either
  an owned view envelope or typed errors: invalid input, missing data, unsupported
  schema, cancelled, source unavailable, storage unavailable/quota, or engine
  failure. Do not transfer Rust pointers or borrows.
- Identify an active data state by an ordered fingerprint of selected source
  family revisions, season/type and engine model version. Derived-result keys
  also include query/sort/window signature. Activation increments generation and
  invalidates all dependent result/detail/roster/source-state caches.
- Assemble and validate in staging; apply replacement between executions.
  Cancel obsolete acquisitions; reject late activation as well as late response
  rendering when context/request generations differ. On worker failure rebuild
  from saved records and explain that session-only data needs reloading.
- Preserve current-roster observation authority separately from historical
  stints. Source-family timestamps remain distinct. Missing or context-incompatible
  optional inputs are unavailable, not zero or implicitly synchronized.
- Audit cargo features/transitive dependencies and native I/O before the WASM
  release build. Inject browser time through the shared clock boundary and keep
  wasm-bindgen crate/CLI versions matched. Do not expand native `send-sync` solely
  to make a worker boundary compile.

### B. Acquisition, imports, and persistence failure rules

- Initial configurable import limits: 25 MiB compressed, 100 MiB total expanded,
  32 MiB per expanded file, and 128 archive entries. These are safety bounds,
  not measured optimal budgets. Enforce during expansion; reject absolute paths,
  parent traversal, symlinks/hardlinks, duplicate normalized names, unknown
  required files, and inconsistent declared sizes. Check metadata/digests before
  model activation; bound deserialization as well as download buffering.
- Runtime source adapters use fixed endpoints, validated season/type and
  pagination parameters, 20-second request timeout, at most three retries per
  request, and at most two concurrent requests. Respect Retry-After; if it exceeds
  the refresh deadline, stop and show when retry is allowed. Set a 120-second
  foreground-refresh deadline. Retry only safe reads; cancel promptly; no
  recursive/repeated retries after that deadline.
- Required-family partial acquisition never replaces the last complete active
  family. Keep its prior observation timestamp and show failed-refresh detail.
  Optional families can expose independently partial/unavailable status. Failed
  Refresh does not change persistence policy.
- Namespace IndexedDB and shell caches by application plus deployment base path;
  same GitHub Pages origin does not provide repository-path isolation. Save
  blobs/metadata/active pointers in one transaction with a revision check.
  Cross-tab notifications invalidate dependent state; concurrent conflicts need
  explicit selection rather than silent last-writer replacement.
- Future unsupported major package/protocol versions are refused. Validate
  required fields, units, hashes and context; supported saved-state migrations
  preserve original package bytes and commit only after validation. Coordinate
  migrations through database version-change handling and notify other tabs to
  reload. Denied/quota storage preserves the in-memory view and old saved copy.
- Render imported names/text as text, never trusted HTML. No credentials, private
  package names, personal contents or tokens in shared URLs, public catalogs or
  telemetry. No personal dataset upload. A relay receives only the minimum public
  acquisition parameters, validates upstream redirects, and has its own request
  budget/host owner. No relay secret is shipped in browser code.
- Browser storage can be removed independently of the app. Offer backup exports
  and recover by reimport/download; any persistent-storage request is best effort.
  Do not describe local saving as a guaranteed permanent backup.

### C. Evidence, performance, and browser acceptance

- Fixed parity corpus: traded player with sum-preserving stints; GP=0 and exact
  rate-floor boundaries; Slafkovsky/Slafkovský round-trip; both Sebastian Ahos;
  goalie discriminator/null values; regular versus playoff; wrong upstream
  season; empty playoffs; October rollover; lockout omission; partial optional
  reports. Use canonical thresholds/tiebreakers and full-precision ordering.
- Browser tests add denied storage, quota exhaustion, interrupted save/migration,
  conflicting tabs, save/unload/remove independence, future schemas, export
  round-trip, stale fetch completion after context switch, 429/503/malformed
  responses, worker restart and application upgrade with old saved data.
- Use hash routes and allowlisted URL query state for season/type, filter, sort
  and public view. Resolve WASM/worker/catalog paths relative to the deployed base.
  Never include private contents in a bookmark; missing local data has a visible
  load/import recovery action. Test deep-link reload under `/ICELINES/`.
- For the offline-open claim, add a versioned service worker caching only the
  shell/engine and public static assets. Do not cache live responses/session-only
  datasets through Cache API, HTTP cache or the service worker; use no-store
  acquisition and exclude those routes from interception. Saved packages live
  in IndexedDB under the explicit policy. Coordinate UI/worker/WASM versions;
  expose an update/reload action, retaining compatible saved data and the prior
  published build for rollback.
- Start with eight resident season/type windows and bounded query preparation;
  never silently answer a multi-season query with evicted inputs.
  Resident package inputs are additionally bounded to 64 MiB; a validated
  larger package may run alone. This byte accounting bounds retained inputs,
  not total Rust/JavaScript/browser memory or staging peaks. Record total
  compressed/uncompressed application size, staging plus resident peak memory,
  startup/import/query latency and all-season stress behavior. Initial measurement
  targets: starter application at most 10 MiB compressed, 250 MiB desktop peak
  memory, and 500 ms warm single-season filter latency. These are provisional
  acceptance targets; benchmark mobile separately and revise with evidence.
- Keep context and a compact memory/saved/source summary visible. Data Library
  is a deliberate drilldown; analysis is the first screen. Use shared IceLines
  visual tokens, semantic tables, keyboard focus, aria-live progress, cancelled
  refresh recovery, color-plus-text states and contained horizontal scrolling.
  Capture 360px mobile and desktop cold/loading/ready/stale/partial/unavailable/
  unsaved screenshots; review accessibility and composition before WP-BW-05 closes.
- Exports carry selected revision fingerprint, context, missing-source state,
  timestamps, implicit rate floors and shared methodology disclosures. Historical
  comparisons do not imply era adjustment; live freshness does not imply forecast
  certainty. Native fantasy mutation features remain separately scoped.
- Record deployed-origin live proof separately from mocked CI: date, browser,
  exact origin, endpoint, redirects, response schema, and result. WP-BW-04 cannot
  close from native request success, a release timestamp, or a disabled refresh
  button. If a relay is needed, its hosting disposition is a gate before coding it.

## 12. Implementation evidence

Foundation execution and remaining gates: [pulse 01](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-01.md).
Portable normalization, native delegation, engine/query fixtures and repository
package conversion are implemented. Worker/UI/live/storage/Pages gates remain open.
Browser slice and acquisition evidence: [pulse 02](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-02.md).
Offline shell and remaining runtime gate: [pulse 03](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-03.md).
Real release conversion and browser archive import: [pulse 04](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-04.md).
Storage failure recovery and transactional rollback: [pulse 09](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-09.md).
Saved-package integrity and schema refusal: [pulse 10](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-10.md).
Shared count-order alignment and rebuilt WASM evidence: [pulse 11](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-11.md).
Measured browser engine/transfer budgets and remaining memory scope: [pulse 12](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-12.md).
Bounded saved-library retention and lazy selection: [pulse 13](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-13.md).
Shared scores/schedule projection and refresh lifecycle: [pulse 14](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-14.md).
Independent schedule persistence, migration and save failure recovery: [pulse 15](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-15.md).
Opt-in polling and cross-tab acquisition coordination: [pulse 16](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-16.md).
Completed-query export metadata and safe CSV text: [pulse 17](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-17.md).
Responsive table containment and selected keyboard accessibility: [pulse 18](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-18.md).
Verified shell readiness and explicit cache repair; failed offline-reopen observation: [pulse 19](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-19.md).
Catalog validation and saved-data startup during catalog failure: [pulse 20](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-20.md).
Bounded package downloads, cancellation and browser retry evidence: [pulse 21](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-21.md).
Actual downloaded package reimport and JSON/CSV file verification: [pulse 22](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-22.md).
Distributed-WASM/native-loader full-row matrix: [pulse 23](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-23.md).
Existing-site preservation and Pages rollout preparation: [pulse 24](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-24.md).
Full local checked build and composed project-subpath browser evidence: [pulse 25](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-25.md).
Retained-artifact verification and prepared rollback workflow: [pulse 26](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-26.md).
Fragment-route cache correction and successful local offline reopening/query: [pulse 27](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-27.md).
Installer-tab reload correction and two-tab saved-season update/rollback rehearsal: [pulse 28](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-28.md).
Real schema-one library migration, blocked-connection recovery and preserved saved selection: [pulse 29](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-29.md).
Eight-window residency, eviction/unload independence and resident resource measurements: [pulse 30](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-30.md).
Actual playoff release file-picker import and regular/playoff archive conversion measurements: [pulse 31](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-31.md).
Session-library long-source containment and measured narrow-frame keyboard/dialog flow: [pulse 32](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-32.md).
Cross-tab refresh-save consent revocation and in-flight schedule regression: [pulse 33](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-33.md).
Transactional refresh consent despite missed notifications, with actual browser storage evidence: [pulse 34](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-34.md).
Revision-checked season/schedule removal and actual IndexedDB replacement preservation: [pulse 35](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-35.md).
Implementation handoff roles review, corrected rollout ordering and reconciled validation status: [pulse 36](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-36.md).
Analysis-first composition, expandable data/methodology controls and selected desktop/narrow evidence: [pulse 37](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-37.md).
Fresh full checked build and isolated browser review branch (independent checks running): [pulse 38](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-38.md).
Scoped staged-blob inventory and complete staged whitespace audit (independent check still running): [pulse 39](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-39.md).
Passing isolated minimal-lock checked build and scoped checkpoint readiness: [pulse 40](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-40.md).
Clean committed artifact and draft PR #74 (remote CI in progress): [pulse 41](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-41.md).
Remote toolchain/audit failures diagnosed; scoped fixes under verification: [pulse 42](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-42.md).
Pinned-toolchain/TLS fixes passed full local checked build; fresh remote CI required: [pulse 43](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-43.md).
Additional native fixture/date-boundary failures diagnosed; focused corrections under verification: [pulse 44](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-44.md).
Focused date/provenance test corrections passed; remote validation remains live: [pulse 45](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-45.md).
Current-head clean artifact identity verified; remote runs and full fetch slice live: [pulse 46](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-46.md).
Successful remote PR artifact verified, full fetch slice passed and selected failure composition captured: [pulse 47](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-47.md).
Current-head cold/loading/ready composition and cancelled late-response recovery captured: [pulse 48](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-48.md).
Current-head remote artifact verified; stale-refresh fixture still under observation: [pulse 49](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-49.md).
Refresh reporting bug corrected and checked; UI-boundary failure preserves results and retry detail: [pulse 50](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-50.md).
Terminal refresh cancellation reporting corrected and checked; full 79adf7e6 remote matrix green: [pulse 51](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-51.md).
Narrow UI-boundary failure/cancellation captures and all-role disposition reconciliation: [pulse 52](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-52.md).
Dated fixture selected through the actual picker; failed refresh preserves old observation and unsaved state: [pulse 53](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-53.md).
Latest implementation CI and retained PR artifact verified; dated desktop import/failure preserves rows and observation: [pulse 54](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-54.md).
Current docs composed with the verified artifact; environment's gh-pages-only deployment policy identified and rollout amended: [pulse 55](../../context/waves/2026-10-03-browser-wasm/pulses/pulse-55.md).


## 2026-10-06 deployed validation and pagination correction

PR 75 merged at `4ae78a8c`; the verified 102-file artifact was published in
`gh-pages` commit `ae53f8bc`, Pages run 37550820648 succeeded, and all 24 shell
HTTP assets match. Existing documentation bytes are unchanged. The normal
application update restored the previously saved season. Live stats and schedule
refreshes and explicit saving/restoration passed on the actual Pages origin.

Full coverage did not pass: live skater counts varied, and an unsorted 940-row
report yielded only 924 unique IDs. The follow-up correction requests stable
player-ID order, checks cross-page identity order and compares bios/summary
coverage before activation. Direct sorted NHL probes returned all 940 skaters
and 98 goalies. See `context/waves/2026-10-03-browser-wasm/evidence/` and the
stable-NHL-pagination role review. Production correction validation and PR CI
remain separate from this direct-source evidence. No full release acceptance
is claimed; deployed offline, rollback and representative device/resource
requirements remain open.


Relay correction runtime reconciliation: Worker `7f92df73` passed full sorted
acquisition and Rust WASM; the existing Pages app refreshed 940 skaters and all
eight 100-point leaders, then explicitly saved the complete snapshot. Browser
coverage guards are prepared but require follow-up PR merge and publication.
