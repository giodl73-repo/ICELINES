---
skill: roles-check
topic: wasm-gzip-delivery
date: 2026-10-13
roles_used: 5
p1_count: 2
verdict: NEEDS-WORK
---

# Gzip delivery prototype review

Artifact: prototype code at 0bc5388f546590fe5269e3172f21d0406fdeb1ba and retained
`target/gzip-delivery-artifact`. One agent applies five installed role lenses;
this is not independent multi-agent review. Selected from the installed role set:
BENCH (measurement/verification), WIRE (transport reliability), KEEL (revision and
compatibility architecture), FORGE (resource/ownership boundaries), EDGE
(adversarial failure handling). No domain/scoring or presentation redesign.

Full validation verdict remains NEEDS-WORK. The local optimization is a promising
prototype, conditional on reviewed clean
PR/CI, retained master artifact and public transfer/update/rollback verification.
Existing incremental publication authorization is unchanged; this review does
not impose another user permission flow.

| Role / ID | Finding | Severity | Evidence / section | Recommendation |
|---|---|---|---|---|
| BENCH B1 | Whole-browser250 MiB acceptance still fails; first candidate405.8 MiB | P1 | gzip-delivery-first-candidate-memory report | Continue optimization; no baseline waiver or workload reduction |
| BENCH B2 | Physical Android remains deferred and unmeasured | P1 | PHONE-ACCEPTANCE.md and user instruction | Keep mobile gate open; test when user connects phone |
| BENCH B3 | No reviewed PR/CI/master publication artifact exists for this prototype | P2 | Clean implementation branch prepared; CI pending | Isolate implementation from local diagnostic history; validate exact reviewed/master artifacts before adoption |
| WIRE W1 | Public Pages gzip MIME/bytes/content-encoding behavior remains untested | P2 | Local HTTP evidence only | Verify all served bytes plus actual app acquisition on public origin, with previous release retained for rollback |
| WIRE W2 | Both envelope and expanded raw bytes are bounded and SHA-authenticated | P3 | package-download.ts, catalog transport validation;9 artifact tests | Preserve dual integrity and exact content-addressed paths |
| WIRE W3 | Existing foreground reader retains bounded time/retries; decode failures do not downgrade silently | P3 | network.ts unchanged; deadline/cancellation unit cases | Preserve terminal corruption errors and foreground cancellation |
| KEEL K1 | Raw JSON remains canonical revision, saving and package export source | P3 | gzip expand-byte equality, refusal browser exact package export | Keep wire hash separate from dataset revision |
| KEEL K2 | Optional descriptor supports old catalogs and unsupported gzip browsers; raw files remain shipped | P3 | legacy catalog and no-DecompressionStream unit cases | Retain both representations for cross-version compatibility |
| KEEL K3 | Actual WASM binary/domain/query code unchanged | P3 | candidate/control WASM hash, all75 actual engine loads | Avoid duplicating hockey normalization/filtering in delivery helper |
| FORGE F1 | Expanded allocation is bounded before aggregation into final buffer | P3 | expandPackage loop; expansion bomb test | Keep exact advertised-size ceiling and100 MiB global maximum |
| FORGE F2 | Reader, abort listener and deadline timer are cleaned in finally | P3 | package-download.ts | Preserve cancellation cleanup on read/parse/size failures |
| FORGE F3 | No new native dependency or engine storage representation is introduced | P3 | Cargo/runtime diff; gzip is build-time node:zlib and browser-native decoder | Keep native engine ownership isolated in existing worker |
| EDGE E1 | Corrupt transport preserves previous active context/full result | P3 | gzip-delivery-refusal report; successful905-player retry | Retain real-browser refusal regression and old-good export byte equality |
| EDGE E2 | Malformed/truncated gzip and incorrect expanded digest/length are refused | P3 | package-download.test.mjs | Preserve explicit corruption and bomb fixtures |
| EDGE E3 | Complete result and offline process reopen still work | P3 | all933 ranks/names,77 goalies, AX/print/keyboard; saved933/fresh835 offline | Do not replace complete results with mounted-row count |

Roles reviewed:5. P1 blockers:2; P2 issues:2; P3 notes:11.
Top finding:250 MiB whole-browser requirement remains unsatisfied.
Cross-role consensus:BENCH/WIRE require exact production artifact and origin proof;
KEEL/WIRE/EDGE require byte-authenticated compatibility and previous-good retention.

Amendments:
1. Reverse-order comparison is complete:405.8 versus449.6 MiB and389.5 versus434.6 MiB. Retain both raw pairs and their measured-scope limitations.
2. If adopted, prepare one clean implementation PR, run required CI, verify retained
   master artifact and public gzip bytes/execution/update/rollback.
3. Keep resource and physical-device gates explicitly open until actual evidence
   satisfies their original scope. Do not label bandwidth saving full acceptance.
