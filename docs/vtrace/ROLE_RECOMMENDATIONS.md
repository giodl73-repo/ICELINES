# Role Recommendations

## Scope

Repo or feature: `icelines` VTRACE adoption and active implementation pulses,
including WP-001 leaders parity and WP-002 named workbench layout persistence.

ROLES conformance target: indexed local roles through `.roles/ROLE.md`, with
VTRACE role lanes mapped to ICELINES local role files.

## Recommended Panel

| Role | Tier | Required | Trigger | Local File |
|---|---|---|---|---|
| Systems Engineering Steward | parliament | yes | Cross-surface state, package sequencing, VTRACE stage changes | `.roles/keel.md`; `.roles/hart.md` |
| Requirements Traceability Auditor | parliament | yes | Requirement, trace, evidence, or work-package status changes | `.roles/bench.md` |
| Package / Interface Boundary Reviewer | parliament | yes | Shared schema, crate/module boundary, URL or local-state interface | `.roles/keel.md`; `.roles/forge.md`; `.roles/wire.md` |
| Verification and Validation Lead | parliament | yes | L0/L1/L2 checks, validation demos, evidence closure | `.roles/bench.md`; `.roles/edge.md` |
| Software Assurance Guardian | parliament | yes | Rust implementation, error handling, tests, dependency direction | `.roles/forge.md` |
| Security Privacy Guardian | parliament | yes for WP-002; optional for WP-001 | Local files, browser query state, durable user state, or public JSON surface changes | `.roles/crest.md`; `.roles/broadcast.md` |
| Safety Risk Officer | parliament | yes for public or wrong-output risk | User-visible analytical state, misleading restore behavior, or cross-surface identity drift | `.roles/glass.md`; `.roles/tape.md`; `.roles/pace.md` |
| Source Custody Counsel | parliament | yes when source/provenance changes | External data, standards, license, source provenance | `.roles/tape.md`; `.roles/wire.md` |
| Repo Maintainer | stakeholder | yes | Commit scope, child-repo vs TRACKER pointer separation | Jim Gregory / TRACKER policy |
| Future Agent | stakeholder | yes | Resumability, pulse handoff, honest pending work | `context/waves/**/pulses/*.md`; `docs/vtrace/TRACE.md` |

## Local Role Gaps

| Gap | Why It Matters | Proposed Role |
|---|---|---|
| No separate ROLES `parliament/` tree yet. | VTRACE can run with `.roles/ROLE.md`, but future automation would benefit from standard frontmatter and role categories. | Defer until more than three active VTRACE implementation packages are running in parallel. |
| Security/privacy is split between CREST and broadcast rather than a dedicated guardian. | WP-002 touches local files and URL state; WP-001 leaders JSON adds public identity fields but no secrets or auth. | Add a dedicated role only if future work introduces auth, secrets, network exposure, or sensitive roster/user data. |

## Review Order

1. Systems engineering: KEEL, then HART for semantic shape.
2. Requirements traceability: BENCH checks `REQ-*`, `IF-*`, `VAL-*`, `EVID-*`, and `WP-*` coverage.
3. Package/interface boundary: KEEL, FORGE, WIRE check schema, crate, URL, and local-state boundaries.
4. Software assurance/code rigor: FORGE checks Rust ownership, error paths, type safety, and test scope.
5. Security/privacy and safety/risk where required: CREST, broadcast, GLASS, TAPE, and PACE check browser/local-state/user-facing risk.
6. V&V: BENCH and EDGE check L0/L1/L2 evidence and failure modes.
7. Source custody where required: TAPE and WIRE when external source/provenance changes are in scope.
8. Stakeholder/editorial: Jim Gregory and future-agent continuity review before readiness or portfolio pointer updates.
