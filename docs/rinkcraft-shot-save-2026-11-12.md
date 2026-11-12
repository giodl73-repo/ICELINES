# Shot, clear-view save and rebound containment

Opt-in explicit-unit rule `0.4.0-playtest` extends controlled reception into
three tasks. Receiver Shooting contests the assigned gap defender's Defending
for an on-target release. Away goalie Tracking contests shooter Shooting for
a fixed clear-view response. After save contact, goalie Rebounds contests
receiver Handling for containment. No extra support bonus applies at these tasks.

The states distinguish controlled release opportunity, on-target flight,
save contact, disrupted release, loose rebound, goalie-held puck and goal.
Save contact is uncontrolled. Failed release/containment ends loose with no
automatic turnover, skater possession or rebound goal. A release tie ends held;
a goalie-response tie ends in unresolved flight; containment tie ends loose.
A goal has `scored_by` and no live controller, flight or loose puck. Exactly one
of held/flight/loose/scored must exist. No restarts, official shot statistics,
match score, clock, fatigue, geometry or empirical probabilities are modeled.
Reusing Shooting in sequential tasks does not assert empirical independence.
Positioning, Movement and Recovery remain untested, pending distinct situations.

The unit studio's **Start shooting scene (0.4)** clears actions and retains unit,
duties, selected hero, seed and support. Default operator setup remains 0.2.
Versions 0.1–0.3 retain exact replay semantics. A frozen 0.3 fixture was copied
before kernel edits from the prior verified fe3784a build, WASM
`f5689504d8edff66797427aed91f8b7bcea4cd3ff2488089483ac442f893e605`.
Unit envelopes remain v1 and the new inner scene is v4. Optional `scored_by`
is omitted from old outputs. Unknown rule versions remain rejected.

Shared Rust owns all scoring, legality, state, seeded dice, exact counts,
validation and studio commands. JavaScript renders rules, legal actions,
flight/goal state and files. Fictional scores never populate NHL assessments.
Rinkcraft decision 0035 owns the authored rule and interpretation.

87 focused Rust tests pass, including exact old fixtures, Shooting/Tracking/
Rebounds contrasts with identical earlier events and dice, unused goalie
capabilities, ties at all three tasks, no extra committed-support bonus,
intermediate import/forgery rejection, new-scene upgrade and 512 seeded paths.
Five studio tests pass again after the final receiver-duty display change.
The first contrast run reused a helper fixed to a synthetic template; its new
helper preserves the fixture's actual template and unaffected scores. No game
rule was changed to fix that test.

Selected contrast seeds are 45 (Shooting), 43 (Tracking) and 286 (Rebounds).
Receiver Handling is 20; other values are 12 except Tracking 20 for the Rebounds
contrast. Each target is varied between 1 and 20. These are selected mechanism
illustrations, not a sample of play or a balance study. The authored packet is
`C:/src/Rinkcraft/game/playtests/2026-11-12-shot-save/`; full local artifacts stay
under ignored `target/shot-save`. The new UI check is
`icelines-browser/e2e/rinkcraft-shot-save.mjs`, using prepared CLI contrast files
and matching native 8772/browser 8773. It does not start servers itself.

All six CLI variants export/verify and step from saved prefixes. Actual native
and offline-browser UI results equal both CLI and WASM, including explicit
upgrade, legal-action gates, no possession at save contact, held/loose/goal
display, unchanged character sheet, invalid-goal rejection and 390-pixel layout.
Independent pypdf confirms two one-page 612 by 792-point Letter character sheets
with identity/attributes and the workbench excluded. Screenshots were inspected.
26 Node tests pass. Browser distribution verification loads the actual WASM and
checks 49 shell assets and 75 supplied packages (810348 bytes gzip for app plus
largest package, below the existing 10 MiB budget).

All seven prior UI suites pass against these builds: receiving, explicit-unit
studio, replay timeline, replay import, plan comparison, possession and game
creator. This includes ten-template creator parity and old 0.1–0.3 behavior.
Validation is local; no hosted build, push, merge or publication was performed.
