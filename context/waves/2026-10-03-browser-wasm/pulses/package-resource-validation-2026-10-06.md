# Browser package resource validation — 2026-10-06

The deployed PR #75 engine accepted a 60 MiB player-name string and allocated
372.625 MiB of WASM memory. [Before](../evidence/resident-stress-before-20261006.json)
records clean source 4ae78a8c and exact WASM identity. This contradicts the
provisional 250 MiB total target even without JavaScript/browser overhead.

The separate `codex/browser-package-resource-validation` implementation adds a
streaming Rust JSON resource preflight in the binding crate before owned
SeasonPackage decoding. Hockey normalization remains in icelines-data. Limits:
1 KiB per decoded string, 8 MiB total text including keys, 10,000 array items,
500,000 nodes including keys. An allocation-free encoded-token scan bounds
serde_json escape scratch to 6 KiB per string. Schema/context checks still follow;
validated exact-revision reuse remains available. No accepted text is truncated.

[Catalog metrics](../evidence/package-resource-catalog-metrics-20261006.json)
measure maxima across all 75 packages: 29 bytes/string, 637,984 total text bytes,
1,004 array items and 106,012 nodes. This is measured headroom for the limits, not
proof that arbitrary future imports fit them. Existing custom or saved packages
with oversized text can now be refused; documented browser limits are part of
the interchange acceptance contract. Native loader policy is unchanged.

Validation: 17 Rust tests pass, including previous-active/revision/order retention,
UTF-8 and escape boundaries, aggregate/array/node limits and malformed JSON.
Clippy with warnings denied passes. The pinned offline build verifies 22 shell
assets, count goldens, eight-window residency and all 75 packages using generated
WASM; largest compressed app plus one package is 406,332 bytes.

[After](../evidence/resident-stress-after-20261006.json) is an actual desktop
Chromium 154 worker/client regression. It rejects the identical 63,824,258-byte
package in 382.5 ms and preserves the entire prior 905-skater query DTO. Allocated
WASM is 70,254,592 bytes (67 MiB). Build-info discloses dirty local source based on
ca0494b4 and WASM digest c5fd6e9448cfcb3016525fb87586fce3bc10f8f01be63cb9393aff9a3cfb2d57.
This is local implementation evidence, not a clean CI artifact or deployment.

The valid whitespace-padded archive test is separate: 96.26 MiB expanded tar
became a 907,753-byte model, converted in 218.4 ms, loaded in 20.8 ms, and returned
905 skaters/103 goalies. Warm-query p95 across 20 samples was 1.5 ms. It does not
prove near-limit model memory. Full browser peak/staging memory, phone testing,
export download, deployed offline reopening and remote rollback remain open.
The guard fixes the reproduced model amplification, not every resource gate.
## Staging ownership follow-up

The main thread previously copied a staging buffer and then structured-cloned it
into the worker. Load/schedule requests now explicitly transfer that staging copy,
while the raw backup remains attached. Archive input is consumed as temporary
compressed bytes; its converted output is transferred back. Default client calls
without a transfer list retain the previous borrowed-input behavior.

Session/restored metadata is reduced to six header fields, so it does not retain
another full decoded bios/stats/goalie tree. Digest-verified raw bytes remain the
backup. Existing saved records, including legacy duplicate metadata arrays, are
accepted through the same envelope/header checks; no schema migration or deletion
occurs. Library tests check report data remains intact in the raw package.

All 133 browser tests pass; TypeScript check and the actual WASM 75-package build
pass. [Browser ownership report](../evidence/owned-buffers-20261006.json) records
real desktop worker input detachment, intact backup digest, header-only metadata,
archive sender detachment and correct 905-skater/103-goalie projections. This is
local dirty-source evidence based on dd547229, with the disclosed unchanged guard
WASM identity. It demonstrates ownership semantics, not total heap/peak memory.
Current-head clean CI and whole-browser staging/resource acceptance remain open.

## Validated header and acceptance reconciliation

Rust now serializes the active package's six metadata fields in the worker load
response. UI activation consumes that header instead of decoding raw JSON again.
A native regression verifies initial absence, regular/playoff resident selection,
rejected replacement preservation and unload. 18 Rust tests, 133 browser tests,
TypeScript and Clippy pass. Rebuilt actual WASM verifies all 75 packages. The local
worker probe matches the header and revision, detaches staging, keeps 908625 backup
bytes and queries 905 skaters. UI activation also displays the correct 2024-25
context and 905 rows with Kucherov first. This evidence is a dirty local build over
2bcf3aaf (WASM 5b407c14f0a727855114091d66c5ddf363659c36aaa308bf1fc3950c841253de).

The prior 2bcf3aaf browser CI 37558731398 passed and its clean synthetic preview
93b043a2 verified 102 files/75 packages. New header source needs fresh CI.

Deployed PR75 exports and actual file-picker round-trip now pass. Package bytes
and complete goalie JSON/CSV reexports remain identical; all eight goalie rows and
49 schedule games match. Blob observer timeouts did not mean missing files:
filesystem/content verification confirms all diagnostic Blob downloads too.
See deployed-export-verification and deployed-file-roundtrip evidence. Physical
phone/picker latency, deployed offline/rollback, new PR publication and full peak
memory remain open. The 100 MiB whitespace probe allocated 105.25 MiB WASM; it is
not a main-UI or whole-browser peak measurement.
