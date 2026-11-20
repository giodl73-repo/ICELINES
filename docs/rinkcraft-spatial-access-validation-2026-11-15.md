# Timestamped spatial access — 2026-11-15

Episode/draft/result 0.8 adds a complete authored local-grid snapshot for loose
puck access. Cells/facing/intent/rates determine projected arrivals, then earliest
access and separate open/pressured Handling collection. No ability-to-speed
conversion, persistent moving state or automatic team-system policy is introduced.

95 event tests pass (79 prior plus 16 spatial checks). Tests cover geometry,
turn/reach/hold, unique earliest access without automatic control, multiple
pressure exact counts, two- and three-player access, top-roll ties, roster-order
invariance, stale/incomplete/duplicate snapshots, bad rates/cells/profiles/puck
state, no arrival versus insufficient collection time, first-goalie unknown,
exact eligibility-change boundary, delayed-call control, older action composition
and no hidden Skating-to-speed effect. Replay/causal/one-puck/goal accounting pass.
Existing possession 12, contest 6 and character 8 checks pass:121 distinct tests.

Run `cargo test -p icelines-core --lib rinkcraft_events --locked`, plus neighboring
`rinkcraft_possession`, `rinkcraft_contest`, `rinkcraft_game`. Build with
`cargo build -p icelines-core --example rinkcraft_events --locked`.
Neutral drafts/instructions: `icelines-core/examples/fixtures/rinkcraft-spatial-access/`.

Initial review caught a distinction between arriving before the window ends and
having time to complete collection. Both are now recorded explicitly; a deadline
arrival does not invent control. The extra actor returning at penalty expiry
requires a fresh snapshot. Goalie opportunities remain physically represented
and explicitly unmapped, not assigned skater scores.

The exact maximum-of-defenders probability is authored game arithmetic, not
observed hockey pressure effectiveness. Coordinate resolution, delays, open
target and task dice need separate calibration. Formatter/whitespace pass.
No app/WASM acceptance, hosted build, publication, merge or push is claimed.
