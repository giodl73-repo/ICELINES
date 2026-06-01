# Wave: VTRACE WP-003 Web Safety

## Goal

Execute `WP-003` as a controlled VTRACE implementation wave: browser route
safety, no-JS navigability, bookmarkable read state, explicit recovery, and
GET-read-only behavior for active Web surfaces.

## Pulse table

| Pulse | Title | Status | Outcome |
|------:|-------|--------|---------|
| 01 | Season-type mutation boundary | closed_with_risk | `/season-type/:kind` moved from GET mutation to POST-backed route; GET now returns method-not-allowed and preserves state. L0/L1 affected evidence passed; broader browser/no-JS inspection remains pending. |
| 02 | Favorites cache read boundary | closed_with_risk | `GET /favorites` now renders favorite player stat lines from existing cached boxscores only. Route tests prove the read path does not create manifest or boxscore cache state; broader browser/no-JS inspection remains pending. |
| 03 | Streaks cache read boundary | closed_with_risk | `GET /player/:id/streaks`, `GET /team/:abbrev/streaks`, and the team JSON twin now render missing-cache empty states without opening the writable data store. Route tests prove the read paths do not create local cache directories; broader browser/no-JS inspection remains pending. |
| 04 | Scoring cache read boundary | closed_with_risk | Selected scoring, outlook, and tonight-intel HTML/JSON GET routes now render missing-source state without opening the writable data store. Route tests prove the read paths do not create local cache directories; broader browser/no-JS inspection remains pending. |
| 05 | Admin data-status cache read boundary | closed_with_risk | `GET /admin` and `GET /api/v1/admin/data-status` now render missing-cache data-status state without opening the writable data store. Route tests prove the read paths do not create local cache directories; broader browser/no-JS inspection remains pending. |
| 06 | Browser shell and recovery navigation | closed_with_risk | The shared HTML shell now exposes explicit no-JS guidance alongside viewport and skip-link affordances. Route tests prove `/dashboard` carries the no-JS/viewport/navigation shell and that unknown routes return recovery/search navigation; launch/host and broader JSON-twin inspection remain pending. |
| 07 | Serve launch safety and closeout | closed_with_risk | CLI serve launch planning now has focused tests for URL-before-open output, `--no-open` browser gating, LAN bind warning copy, and bind resolution. WP-003 closes with accepted live-browser/touch-focus/full-JSON-matrix residual risk routed to WP-008. |

## Success criteria

- `WP-003` stays linked to `REQ-WEB-001`, `REQ-WEB-002`, `IF-WEB-001`,
  `VAL-003`, `EVID-CR-006`, `EVID-CR-014`, and `EVID-CR-018`.
- GET routes remain read-only; browser mutations are POST-backed or explicitly
  deferred.
- Route inventory, templates, and surface-parity docs stay aligned with router
  behavior.
- No-JS browser affordances remain understandable without requiring JavaScript.
- TRACKER submodule pointer updates remain separate from ICELINES child-repo
  implementation commits.

## Gate Status

Current gate: `closed_with_risk` for WP-003 after pulses 01-07.

This wave is closed with risk. Pulses 01-07 close the observed season-type GET
mutation gap, `/favorites` GET cache-write gap, streaks GET cache-directory
creation gap, and selected scoring/outlook/tonight-intel GET cache-directory
creation gap, and selected Admin data-status GET cache-directory creation gap,
plus selected no-JS shell, viewport, skip-link, dashboard navigation, and
unknown-route recovery evidence, plus serve URL-before-open, `--no-open`, LAN
bind warning, and bind resolution evidence. Live browser screenshot/review,
touch/focus interaction, and broader JSON-twin matrix evidence remain accepted
residual risks for `WP-008` rehearsal before readiness claims.
