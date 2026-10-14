---
skill: roles-check
topic: evidence-bytes
date: 2026-10-14
roles_used: [BENCH, WIRE, KEEL, FORGE, EDGE]
p1_count: 0
p2_count: 0
verdict: APPROVED
---

# Validation evidence bytes

Single-agent review through five installed role lenses, not independent reviewers.
Artifact: evidence/.gitattributes and14 report-byte restorations, plus audit records.
Scope is exact archival fidelity; browser acceptance, native behavior, public
assets and local-state schemas are unchanged. Native/hockey-specific lens checks
are not newly proved by this metadata correction.

| Role | Finding | Severity | Evidence / recommendation |
|---|---|---|---|
| BENCH | Before audit exposes14 Git-blob mismatches and48 clean-checkout mismatches. | P3 | Retain failed report; do not call old archive verification passed. |
| BENCH | After audit proves62 explicit sibling file/SHA descriptors with zero worktree/Git/checkout mismatches. | P3 | Keep exact audited scope; unrelated hash fields are excluded. |
| BENCH |227 existing records decode as strict UTF-8 JSON. | P3 | Restoration preserves every JSON value; no benchmark rerun needed. |
| WIRE | Recorded original report hashes remain unchanged. | P3 | Restore bytes to existing hashes rather than replacing hashes to fit normalized files. |
| WIRE | All14 reconstructions match exact original SHA after LF-to-CRLF conversion only. | P3 | A mismatch would stop restoration; no speculative re-encoding. |
| WIRE | Independent failed/current reports remain separate. | P3 | Before and after audits are retained with distinct identities. |
| KEEL | Byte preservation belongs to the evidence directory. | P3 | Nested attributes avoid changing source/config formatting elsewhere. |
| KEEL | Runtime architecture and deployment do not change. | P3 | No app publication/rebuild needed for this metadata fix. |
| KEEL | Newline policy does not rewrite acceptance meaning. | P3 |250 MiB and Android gates remain open. |
| FORGE | Report restoration operates on validated tracked paths inside the evidence directory. | P3 | No recursive deletion/move or main-worktree edits. |
| FORGE | Non-restored files retain existing Git blob bytes. | P3 | Staged diff contains only14 reports plus archival policy and new audit records. |
| FORGE | CRLF archival lines are valid report bytes. | P3 | Attribute permits CR at EOL for whitespace checking; JSON remains readable. |
| EDGE | Mixed LF/CRLF original reporters are supported. | P3 | Unset text conversion preserves both; forcing every report to LF would break14 original hashes. |
| EDGE | Windows autocrlf=true is exercised by checkout-index. | P3 | Raw hashes, not decoded semantic comparison alone, prove checkout fidelity. |
| EDGE | Unknown/external hashes are not covered by this audit. | P3 | Explicitly exclude archive/module/PDF/source hashes without sibling descriptors. |

Roles reviewed:5. P1 blockers:0. P2 issues:0. P3 notes:15.
Verdict: APPROVED for archival correction only.
Top finding: preserve existing hash identities across checkout conversion.
BENCH and WIRE agree byte equality is required; JSON semantic equality alone is
insufficient. Amendments applied: retain before-failure evidence; preserve original
CRLF where exact hashes require it; protect all archival JSON from conversion.
