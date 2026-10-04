# Browser WASM pulse 04 — release archive import

Date: 2026-10-03. Parent: WP-BW-01 / REQ-BROWSER-001 / CHG-119.

## Implementation

Local `.tar.gz` imports decompress in the dedicated worker with streaming bounds:
25 MiB compressed, 100 MiB expanded including headers/padding, 32 MiB per file,
128 entries. No paths are extracted to disk. Header checksums, gzip integrity,
safe paths, regular-file/directory allowlists, duplicate basenames, required files,
season identity and selected season type are checked. Links/extended metadata
and nonzero trailing content are refused. Conversion produces the existing raw
SeasonPackage; the shared Rust loader stages and validates it before activation.
Regular/playoff import selection is explicit. Missing goalie reports and unknown
observation timestamps remain absent/unknown. Local filenames are not copied into
public URLs/source metadata. Successful imports remain memory-only until saved.

## Actual release evidence

Inspected/downloaded [data-20242025 release](https://github.com/giodl73-repo/ICELINES/releases/tag/data-20242025)
using GitHub CLI. Asset API ID 422106373; 149,672 bytes; published SHA-256:
`23ab90d4306c18299ac68d35389bc75edea5ce454765c40b6c66c6248b34d8ca`.
Downloaded bytes match that digest. Publication is not evidence of final completeness.

`node scripts/verify-browser-release.mjs target/browser-releases/data-20242025.tar.gz <digest>`
passed actual generated WASM loading and shared queries for both converted types:

| Context | Skaters | Goalies | Leading points |
|---|---:|---:|---|
| Regular | 905 | 103 | Nikita Kucherov 121 |
| Playoff | 332 | 27 | Leon Draisaitl / Connor McDavid 33 |

Outputs/evidence are in `target/browser-release-import/verification.json`.
The Python publication converter also produced two packages from the downloaded
archive with an empty repository-season input. `cargo run -p icelines-wasm
--example verify_catalog --offline --locked -- target/browser-release-catalog/catalog.json`
verified both content hashes and shared-engine loading.

Actual browser file chooser imported that same archive at
`http://127.0.0.1:8049/ICELINES/`. Regular season context was 20242025; the p>=100
Rust query returned six named players, led by Kucherov. UI confirmed imported in
memory and no automatic save. Playoff browser conversion/engine loading passed
in Node WASM; direct playoff file-chooser interaction is not yet verified.

The prior origin's update check did not expose the new control within its wait;
old preview state persisted. Fresh-origin verification does not close that update
gate. Working fresh preview: server session **15620**, browser tab **6**, marked
for continuation. [Browser archive screenshot](browser-pulse-04.png).

## Tests and remaining scope

Initial full Node suite passed 35 tests (25 prior + 10 archive fixtures). Coverage
includes standard release layout, accents, optional reports, season/type mismatch,
paths/links/metadata, duplicates, entry/file bounds, checksums, truncation, end
markers, malformed gzip, compressed bound and lockout/schema rejection.
The final `npm test` build passed **37 tests**, including two additional cases for
the actual 100 MiB streaming expansion boundary and corrupted gzip integrity.
Scoped `git diff --check` passed. The file input now clears after selection so
the same archive can be selected again for another season type; this final UI
change passed compilation but its repeat-selection interaction remains unverified.
No CI test fetches live upstreams.

Full goal remains active. Live deployed path/relay hosting, scores/schedule,
refresh coordination, offline/update proof, crash and storage failure recovery,
sort/rate-floor parity, resource budgets, full browser acceptance and Pages
deployment remain incomplete or unverified. No shipped status is promoted.
