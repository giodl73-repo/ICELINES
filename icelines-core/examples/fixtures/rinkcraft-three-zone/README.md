# Three-zone route fixtures

Authored fictional event/draft/result 0.9 cases, seed 20261115, one illustration
per case. Character rules remain shot-encounter.0.1; all manual scores are 10.
These are branch-accounting fixtures, not estimated hockey rates. Open target 1
ensures the neutral builds exercise the named route. Contested entry instead
uses Handling versus Defending, neutral strict-win 190/400.

- carry-route: defensive retrieval, controlled exit, neutral carry, entry.
- pass-route: outlet to p1; entry pass to p2; flight and receipt are distinct.
- clear-counter: clearance reaches neutral; B collects and attacks west.
- dump-recovered: A dumps east; p1 must actually secure the loose puck.
- dump-lost-counter: B retrieves east, exits its defensive end and attacks west.
- regroup: A passes backward, exits again, then advances and enters.
- unknown-entry: lawful exit/progression; unknown entry ruling stops in neutral.
- reverse-direction: A attacks west in a declared period-2 frame.
- contested-entry: entry can succeed or leave loose play; no automatic turnover.
- window-stop: entry cannot fit the remaining maximum live-time budget.

Cells are abstract -20..20; blue lines -4/+4, on-line is neutral by game convention.
Coordinates do not implement NHL offside, icing or goalie-area rules. Legal/unknown
crossing rulings are explicit. Scene offsets project a complete roster around a
landing point; they are authored hypotheses, not observations or continuous paths.
Goalies remain in every scene, with their own domain; goalie-first access remains
explicitly unmapped. No roster-position-to-F1/F2/F3 or blame mapping is inferred.

Build and run with the existing Rust `rinkcraft_events` example. Dated manifests,
replay checks, old-version comparisons and findings live in PUCK's
`scenarios/nhl/2026-27/rinkcraft-three-zone-transition/`.
