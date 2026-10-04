# IceLines browser workbench

Implementation follows `design/plans/2026-10-03-browser-wasm-workbench.md`.
The engine lives in `icelines-wasm`; its normalization is shared with native
loaders through `icelines-data`. The browser owns network/file/storage adapters.

The Session library retains up to eight prepared season/type windows in this
tab. Rust applies LRU eviction and a 64 MiB package-input budget; a larger
active package runs alone. Switch resident seasons without another download.
Unloading removes the selected window while others remain available. Refresh
replaces the same window only after validation. Residency never enables saving
and ends on application reload or worker failure. These input limits are not a
total browser memory guarantee.

[Pulse 31](../context/waves/2026-10-03-browser-wasm/pulses/pulse-31.md) records
actual playoff import of the downloaded 2024–25 GitHub release through the file
picker: 332 skaters and 27 goalies, memory-only. Twelve regular/playoff worker
conversions of those verified bytes passed; the slowest conversion was 12 ms
and the initial regular load was 20.7 ms. These measurements exclude file-picker
latency, total peak memory and mobile hardware.

Catalog failure is separate from engine and saved-library startup. The UI shows
an unavailable catalog and a retry action, restores saved data, and keeps imports
available. Retry replaces catalog choices without reloading or replacing the
active dataset/query. Public links needing an unavailable catalog still require
explicit dataset selection; saved private data is not substituted into a public
view. Until a catalog identifies a package, its saved-data links are local-only.
Catalog reads use a 20-second total deadline and 2 MiB response limit. Entries
must have unique IDs and season/type contexts, valid bounded metadata, and an
exact `data/<sha256>.json` path. Package reads refuse redirects and verify byte
size, digest, source, and season/type before shared Rust activation.
[Catalog recovery evidence](../context/waves/2026-10-03-browser-wasm/pulses/pulse-20.md).

Query JSON/CSV exports include the completed query, revision, season/type,
timestamps, missing sources, effective sample floors and methodology. Unsubmitted
control edits do not alter exported result context. CSV has single-field metadata
comment records before the table; readers should skip those records. Potential
spreadsheet formulas in text receive a leading apostrophe, disclosed in metadata;
JSON preserves the original strings. Numbers retain engine precision, null CSV
cells are empty, and row order is shared Rust order. Use package exports for
importable dataset backups. [Export evidence](../context/waves/2026-10-03-browser-wasm/pulses/pulse-17.md).

Tables have named keyboard-focusable scroll regions; the skip link focuses
analysis. Player dialogs are named, start at Close and restore focus on Escape.
Long metadata wraps, context stays visible, and tables scroll within the page.
Selected 360px embedded-layout and desktop keyboard checks are recorded in
[accessibility evidence](../context/waves/2026-10-03-browser-wasm/pulses/pulse-18.md).
Physical mobile and broader assistive-technology acceptance remain open.

Offline readiness checks the actual cached shell assets against their sizes and
SHA-256 digests. An active service worker alone does not establish readiness.
When a cached file is missing or damaged, **Repair offline files** downloads the
exact current build with a 120-second deadline. It stages and verifies all files
before copying replacements into the active cache; a failed download or different
published build leaves existing files untouched. Concurrent tabs share one repair.
Repair never loads or persists datasets, activates an update, or reloads the page.
Cache writes themselves are not atomic; a write failure reports repair failure,
and a later readiness check must verify completeness before an offline claim.

Prepare a disposable localhost recovery fixture with
`node scripts/prepare-browser-offline-preview.mjs`, then serve
`target/browser-offline-preview` on a separate port and open `offline-test.html`.
The fixture removes only its test build's cached stylesheet. Production assets
remain unchanged and the fixture is excluded from publication.
[Recovery evidence](../context/waves/2026-10-03-browser-wasm/pulses/pulse-19.md)
proves missing-file detection and browser repair. Server-stopped reopening failed
in the in-app browser; offline reopening remains an unpassed acceptance gate.

Generate content-addressed public packages:

```powershell
python scripts/build-browser-catalog.py
```

To convert downloaded GitHub data releases, pass `--release-dir <directory>`.
Archives are inspected in memory without extracting arbitrary paths. The catalog
records source-file hashes and package hashes. Legacy observation timestamps
remain unknown; package conversion time does not establish source freshness.

Build from the repository root using the versions in `toolchain.json` (install
the Rust WASM target first). The shared entry point installs the matching
wasm-bindgen CLI into workspace output, generates packages, builds a clean
distribution, and verifies every public package with the actual WASM engine:

```powershell
npm ci --ignore-scripts --prefix icelines-browser
node scripts/build-browser.mjs --check
python -m http.server 8047 --bind 127.0.0.1 --directory icelines-browser/dist
```

Open `http://127.0.0.1:8047/`. The module worker runs the shared Rust engine.
Use `--offline` with the build entry point when Cargo dependencies and the CLI
are already cached. Tests use `target/browser-test-tmp` for temporary files.
`build-info.json` records actual tool versions, the HEAD commit, whether the
working tree differs from it, and the WASM digest. A dirty build's commit alone
does not identify all its source inputs.

`.github/workflows/browser-pages.yml` runs the same checks on relevant pull
requests and master changes, checking out LFS objects when present. It retains
the distribution as a preview artifact for 30 days. Publication requires a
manual dispatch with `publish` enabled on master, after release gates are met;
the `github-pages` environment can enforce repository-configured review.
The workflow is prepared but has not yet been executed on GitHub. Retained
artifacts support recovery; a verified rollback deployment remains a gate.
Publication preserves the existing documentation site and adds `/workbench/`.
The repository currently uses legacy branch Pages publishing; switching to
Actions and coordinating documentation publication are explicit rollout steps.
See [Pages rollout](PAGES.md).
Load a published package or import a browser-format JSON package or local release
`.tar.gz` archive, query skaters
or goalies, inspect details, and export results or the original package. Data
starts in memory. **Save locally** writes it to the IndexedDB library; **Unload**
only unloads memory; **Remove saved copy** only removes local persistence.
Local saves can be deleted by browser settings; exported packages are portable
backups. No imported personal dataset is uploaded.

Scores and schedule use the shared Rust NHL game-week projection. Refresh a
chosen date into memory, then use **Save schedule locally** to persist it.
**Keep refreshed schedule saved locally** opts that saved date into complete
replacement saves after explicit refresh. New dates remain session-only.
Schedule unload/removal and active pointers are independent from season stats.
Export/import schedule packages for backup; importing never enables the refresh
save policy. Startup restores the saved active schedule without network access
and revalidates its raw payload through Rust. Live CORS access remains unproven.

The deployment-path namespace remains `icelines-v1:`. IndexedDB schema version 2
adds `schedules` alongside the existing `datasets` and `settings` stores.
Its additive transaction preserves version-one season records and pointers;
an interrupted upgrade rolls back. Schedule interchange schema 1 fingerprints
its context, timestamps and raw NHL payload. Unsupported future database/package
versions are refused without deletion. Older builds cannot open the upgraded
database; use the current build or exported backups when rolling back the app.

On worker failure, the app terminates the failed engine, invalidates outstanding
requests and results, and attempts one restart. It restores the same dataset's
saved revision when available. Session-only data needs loading/importing again;
its original bytes remain exportable as a recovery copy in the current tab.
Closing or reloading the tab discards that unsaved recovery copy. A repeated
failure stops automatic recovery and exposes **Restart engine** for manual retry.
Worker construction, message decoding, malformed responses, initialization,
WASM traps and a two-minute response watchdog enter this recovery path.

For local failure acceptance, `node scripts/prepare-browser-recovery-preview.mjs`
copies the built app into `target/browser-recovery-preview` and adds test-only
fault controls. Serve that directory on a fresh localhost port. The harness
terminates a real WASM worker and dispatches a synthetic error signal, and can
block worker construction to verify retry behavior. It is excluded from the
publication distribution. Its modified index intentionally fails offline asset
integrity verification; use the normal distribution for offline acceptance.

Minimum GP can be left blank for the default: all skaters, goalies with at least
5 regular-season games or 1 playoff game. An explicit 0 includes all players.
The query evidence reports the applied floor. Pace is unavailable below the
shared 10-GP guard; qualifying players sort by the shared pace score, including
its goals-per-82 tie-breaker and final player ID tie-breaker. Goalie ordering uses
the same comparator as native goalie views. Count sorts use descending values
and player ID, consistent with CLI queries; the native web skater route currently
uses additional points/name tie-breakers. Cross-surface reconciliation remains
open rather than claiming complete ordering parity with every native surface.

Hash links restore season/type, skaters or goalies, sort and the default/explicit
minimum GP. An explicit public link takes precedence over a previously active
saved dataset. The public package is checksum-verified before activation.
Ordinary bookmarks omit filter text; **Share this view** can include an applied
filter only after explicitly marking it public. Incoming public filters survive
reload; subsequent ordinary queries return to filter-free bookmarks. Links use
only the current deployment base and allowlisted view parameters.

Imported or live revisions use `data=local` rather than package names, IDs, hashes,
contents or filters. A local link restores a compatible local saved copy, or
shows a load/import recovery message if missing or ambiguous. It never silently
substitutes a public catalog package. For goalie filters use `goalie-games`; the
shared engine treats skater-specific stat atoms as inapplicable to goalie rows.
The Minimum GP control qualifies either player type.

For archives, choose **Archive season type** before selecting the file. Standard
tar regular files/directories are accepted; links, extended tar metadata, unsafe
paths, duplicate basenames and unknown files are refused. Limits are 25 MiB
compressed, 100 MiB expanded including tar overhead, 32 MiB per file and 128
entries. Decompression runs in the engine worker, and shared Rust validates the
result before replacing the active dataset. Archive publication/bundle timestamps
do not establish observation freshness. Older missing goalie reports remain absent.

For manual real-release verification after a build, download a public archive
and obtain its published SHA-256 digest, then run:

```powershell
node scripts/verify-browser-release.mjs <archive-path> <published-sha256>
```

This checks the archive digest, converts regular/playoff packages through the
browser importer and loads/queries them using actual generated WASM bindings.
It contacts no network itself and does not replace fixture-based CI.

Live season refresh uses fixed public NHL endpoints, a shared 120-second deadline,
20-second requests, at most three retries and two concurrent requests. Required
report failure leaves the prior active package intact. The saved-copy policy
is explicit, and failed saving leaves fresh data usable in memory. Direct live
access failed in the local browser preview; deployed-origin access or a hosted
constrained relay is still required before claiming a working live capability.

Automatic refresh is opt-in and tab-only: season stats every 24 hours, scores
and schedule every minute, with positive jitter and completion-based intervals.
Hidden/offline tabs pause and cancel automatic acquisition; context changes
stop it. Checks never enable local saving. Deployment-scoped Web Locks serialize
live acquisition across tabs; the refresh deadline includes waiting. Browsers
without Web Locks support manual refresh only. Coordination shares no response
data or preferences, and saved revision conflicts still require explicit choice.
See [polling evidence](../context/waves/2026-10-03-browser-wasm/pulses/pulse-16.md).

The build generates `sw.js` and `shell-manifest.json` from the exact UI, worker,
WASM and public catalog assets. The cache identity also includes the worker
template, so worker-only changes create a new cache. Installation verifies each
SHA-256 before caching.
Dataset and live routes are excluded. The cache is namespaced by deployment path.
Updates wait for consent from each open app tab before activating and reloading;
save/export in-memory work first. Tests cover these rules. The embedded browser
installed the shell and retained a saved dataset across an update, but failed
the earlier server-off reload test. Pulse 27 below records a corrected worker
and successful local offline reopening; deployed offline/update coverage remains open.
Service-worker lifecycle follows the [MDN guidance](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API/Using_Service_Workers).

The first slice remains under construction: deployed live access and offline/update proof,
comprehensive browser lifecycle
tests and Pages deployment are pending. Node tests use fixture responses and
an IndexedDB test implementation; they do not establish deployed reachability or
browser quota/migration behavior. Generated packages/bindings are build artifacts,
not committed data.

Published package downloads use bounded safe reads: 20-second attempts including
response bodies, three retries within a 120-second total deadline, advertised
size limits, and explicit cancellation. [Pulse 21](../context/waves/2026-10-03-browser-wasm/pulses/pulse-21.md)
records browser retention of saved data after cancellation and recovery from two
HTTP 503 responses without automatically saving the new season.

[Pulse 22](../context/waves/2026-10-03-browser-wasm/pulses/pulse-22.md) verifies
actual 2023-24 package and query downloads, byte-identical package reimport through
the file picker after unload, and JSON/CSV field agreement. This is evidence for
one regular-season slice; broader file-picker and release acceptance remain open.

`node scripts/verify-browser-native-parity.mjs --offline` compares the actual
distributed WASM with full native-loader results for 270 regular/playoff sort,
floor and filter cases. It also runs in the checked build pipeline.
[Pulse 23](../context/waves/2026-10-03-browser-wasm/pulses/pulse-23.md) records
33,613 row comparisons and 90 empty results; broader route/corpus parity remains open.

[Pulse 25](../context/waves/2026-10-03-browser-wasm/pulses/pulse-25.md) records a
passing full local checked build and actual `/ICELINES/workbench/` loading,
public-filter bookmark reload and saved-season restoration alongside preserved
root documentation. Server-stopped reopening still failed in the in-app browser;
offline and deployed HTTPS acceptance remain open.

[Pulse 26](../context/waves/2026-10-03-browser-wasm/pulses/pulse-26.md) adds a
manual retained-artifact rollback workflow and seven integrity/source tests.
It restores selected clean master-build bytes under workbench/ while preserving
docs; actual remote rollback and saved-data compatibility remain unverified.

[Pulse 27](../context/waves/2026-10-03-browser-wasm/pulses/pulse-27.md) fixes
fragment-bearing bookmarks missing the shell cache allowlist. All 119 browser
tests passed. With the local server stopped and connection refusal confirmed,
the application reopened under `/ICELINES/workbench/`, restored 905 saved skaters,
and ran a fresh six-row WASM query. This supersedes the earlier local offline
failure; deployed HTTPS, other devices and upgrade/rollback acceptance remain open.

[Pulse 28](../context/waves/2026-10-03-browser-wasm/pulses/pulse-28.md) fixes
the original installer tab skipping later application-update reloads. All 121
browser tests passed. Both tabs required consent and restored their saved season
when switching to retained prior bytes and back to the current build; offline
reopening and a new WASM query passed afterward. These builds share schema 2;
remote rollback, browser migration and broader device acceptance remain open.

[Pulse 29](../context/waves/2026-10-03-browser-wasm/pulses/pulse-29.md) verifies
the actual browser's schema-one to schema-two library migration. A held legacy
connection blocked migration with a recovery message; closing it and retrying
preserved original season bytes and the active pointer. An older schema-one
open was refused without changing records. Reload restored 905 skaters and a
fresh six-row WASM query passed. Other browser/device and migration-fault
acceptance remain open.

[Pulse 32](../context/waves/2026-10-03-browser-wasm/pulses/pulse-32.md) fixes
long source names overflowing the session library. With eight windows in a
measured 360px iframe, document client/scroll widths both stayed 345px. Table
arrow scrolling and player-dialog Enter/Escape focus passed; all 121 browser
tests and distribution verification passed. Physical mobile, screen-reader,
deployed live access and remote rollback acceptance remain open.

[Pulse 33](../context/waves/2026-10-03-browser-wasm/pulses/pulse-33.md) fixes
cross-tab removal/policy revocation leaving refresh saving enabled. Library
notifications revoke consent without granting it to session-only tabs. Actual
two-tab removal kept the season queryable in memory, and an in-flight schedule
fixture preserved the old saved revision. All 124 browser tests passed.

[Pulse 34](../context/waves/2026-10-03-browser-wasm/pulses/pulse-34.md) adds
transaction-level consent checks to refresh saves, so missed notifications
cannot recreate removed copies or overwrite revoked consent. Two regressions
failed before correction; all 126 tests then passed. Real browser IndexedDB and
production controller/WASM verified these cases with local fixtures.

[Pulse 35](../context/waves/2026-10-03-browser-wasm/pulses/pulse-35.md) adds
revision checks to saved-copy removal. A stale action preserves another tab's
replacement and active pointer; reload/select the current version to remove it.
All 128 tests passed, with actual IndexedDB fixture evidence for both families.

[Pulse 37](../context/waves/2026-10-03-browser-wasm/pulses/pulse-37.md) makes
analysis primary with expandable Data Library, application options and methodology.
Memory/source state remains visible; season loading closes the library and moves
focus to analysis. Desktop and measured 360px keyboard/layout checks passed;
physical mobile and the complete state-composition matrix remain open.
