---
skill: roles-check
topic: tls-trust-patch
date: 2026-10-21
roles_used: [FORGE, WIRE, BENCH]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# TLS trust dependency patch review

Single-agent installed-role review, not independent reviewers. FORGE reviews locked
dependency scope; WIRE reviews certificate trust; BENCH reviews the observed audit
failure and native request/handler regression checks. No advisory exception.

|Role|Finding|Severity|Evidence / condition|
|---|---|---|---|
|FORGE|Only two dependency versions/checksums change|P3|rustls-webpki0.103.15→0.103.16; webpki-roots1.0.7→1.0.10|
|FORGE|No manifest/features/profile/API changes|P3|Precise updates, dependency lists unchanged|
|FORGE|Borrowed core/query/browser source stays unchanged|P3|No Rust/JS implementation edits|
|FORGE|Active WASM runtime graph excludes the patched TLS packages|P3|Locked wasm32 normal-dependency graph checked separately;174 lines retained|
|FORGE|Existing broad dependency PR is separate|P3|PR114 does not remediate these versions; avoid unrelated upgrades|
|WIRE|CI identifies certificate-verification advisory|P3|RUSTSEC-2026-0382 against rustls-webpki0.103.15|
|WIRE|CI identifies root-bundle advisory|P3|RUSTSEC-2026-0381 against webpki-roots1.0.7|
|WIRE|Patched versions match authoritative RustSec ranges|P3|Official advisory DB e6f94f3760d5e6c54c1b05f204002263bc025524|
|WIRE|Refreshed audit reports zero vulnerabilities|P3|1348 advisories; no ignore-list or TLS-verification bypass|
|WIRE|Two existing maintenance warnings remain|P3|async-std/basic-toml remain visible; add missing compile-time Askama parser ledger entry, no vulnerability exception|
|BENCH|The failure is reproducible audit evidence|P3|Master run39534734373 / job119008647385; exact failed log retained|
|BENCH|Native HTTP integration is the affected surface|P3|Require locked icelines-fetch/icelines-web tests before merge|
|BENCH|A successful audit is not complete application validation|P3|Full hosted CI/browser checks tracked separately; do not claim pending checks pass|
|BENCH|No new tests duplicating upstream certificate logic|P3|Use existing native regressions and refreshed advisory audit|
|BENCH|Memory/mobile/private-MISER gates remain distinct|P3|This patch does not close250MiB/500ms/physical Android/private lab acceptance|

Roles3; P1:0; P2:0; P3:15. Conditional approval after native regressions and
target-specific graph verification complete, with refreshed audit already clear.
Full CI remains required evidence for the combined integration and must retain its
actual state. Amendments: isolate precise lock changes, preserve maintenance-warning
scope, and test affected native consumers without weakening certificate checks.

Primary RustSec sources:
[rustls-webpki](https://github.com/RustSec/advisory-db/blob/e6f94f3760d5e6c54c1b05f204002263bc025524/crates/rustls-webpki/RUSTSEC-2026-0382.md),
[webpki-roots](https://github.com/RustSec/advisory-db/blob/e6f94f3760d5e6c54c1b05f204002263bc025524/crates/webpki-roots/RUSTSEC-2026-0381.md).
