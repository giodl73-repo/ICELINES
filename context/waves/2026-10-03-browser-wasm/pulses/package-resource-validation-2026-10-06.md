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