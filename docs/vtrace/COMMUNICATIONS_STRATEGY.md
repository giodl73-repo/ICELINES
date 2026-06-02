# IceLines Communications Strategy

## Purpose

This artifact maps accepted IceLines VTRACE intent to user-facing docs surfaces.
The docs package explains the NHL analytics workbench, shared query/ViewModel
model, source-state honesty, reports, data-depth commands, fantasy/cache/signal
limits, and code-rigor posture without overstating prediction or coaching
authority.

## Surface Plan

| Surface ID | Source IDs | Audience | User Question | Generated Docs | Cadence | Owner | Status |
|---|---|---|---|---|---|---|---|
| COMMS-ICE-README-001 | REQ-WB-001 / REQ-WB-002 / SPEC-ICE-001 | NHL analyst / future agent | Where do I start, and what workbench surfaces are active? | `docs/index.md`, `docs/README.md` | every docs wave | IceLines maintainer | planned |
| COMMS-ICE-QUERY-001 | REQ-QUERY-001 / REQ-PARITY-001 / SPEC-ICE-002 / WP-001 | CLI/TUI/Web operator | How do I ask the same hockey question across surfaces? | `docs/guides/01-query.md`, `docs/concepts/query-viewmodel-parity.md` | when query or ViewModel semantics change | Art Ross / Campbell owner | planned |
| COMMS-ICE-REPORT-001 | REQ-STAT-001 / REQ-STAT-002 / REQ-REPORT-001 / SPEC-ICE-003 / WP-004 | analyst / public post author | What does stat-in-perspective mean, and what must reports disclose? | `docs/how-to/share-stat-in-perspective.md` | when report/export guardrails change | PACE / SCOUT owner | planned |
| COMMS-ICE-WEB-001 | REQ-WEB-001 / REQ-WEB-002 / SPEC-ICE-004 / WP-003 | web user / reviewer | What can the web dashboard prove, and what recovery states exist? | `docs/guides/06-tui.md`, `docs/how-to/review-web-dashboard.md` | when web route behavior changes | Web surface owner | planned |
| COMMS-ICE-DATA-001 | REQ-OFFLINE-001 / REQ-DATA-DEPTH-001 / REQ-FRESH-001 / SPEC-ICE-005 / WP-005 | data operator / maintainer | How do I install, refresh, verify, and interpret source-state data? | `docs/guides/04-data.md`, `docs/how-to/review-source-state.md` | when source/fetch/cache behavior changes | HART / WIRE owner | planned |
| COMMS-ICE-FANTASY-001 | REQ-FANTASY-001 / REQ-CACHE-001 / REQ-SIGNAL-001 / SPEC-ICE-006 / WP-006 / WP-009 / WP-010 | fantasy manager / coach workflow reviewer | Which fantasy, cache, and signal claims are supported or partial? | `docs/guides/03-fantasy.md`, `docs/concepts/analytics-cache-and-signals.md` | when fantasy/cache/signal evidence changes | Selke / Signals owner | planned |
| COMMS-ICE-CORPUS-001 | REQ-CODE-001 / SPEC-ICE-007 / REVIEW.md | docs owner / future agent | Who owns docs updates and claim-limit wording? | `docs/CORPUS.md` | every docs wave | IceLines docs owner | planned |

## Review Checklist

| Item | Required | Decision | Evidence / Rationale |
|---|---|---|---|
| Docs claims trace to controlled source IDs. | yes | accepted | Rows cite requirements, specs, work packages, and review posture. |
| Concepts/tutorials/examples do not overclaim unvalidated behavior. | yes | accepted | Prediction, betting, coaching, and complete-world claims remain excluded. |
| Public interfaces have expected usage or expected output docs. | if applicable | accepted | Query, web, data, report, fantasy, cache, and signal surfaces are mapped. |
| `docs/CORPUS.md` names ownership and update obligations. | if multiple surfaces exist | planned | COMMS-ICE-CORPUS-001 records the corpus surface. |
