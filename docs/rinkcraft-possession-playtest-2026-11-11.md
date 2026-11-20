# Rinkcraft possession playtest

The creator's **Try a possession** control runs a small fictional scene in native
and browser apps. Shared Rust owns unit construction, legality, seeded dice,
compressed arithmetic, support/cover, control and transitions. A shared JS module
renders both surfaces. NHL profiles and character creation scores do not change.

`rinkcraft_possession::from_json(character_rules, possession_rules, request)`
validates the entire fictional export against its rules, constructs twelve unique
F1/F2/F3/D1/D2/G slots and replays the action history. Failed attempts become loose
pucks; recovery can open a counter. Ties end an opportunity; sixteen steps bound
the scene. Classes add no bonuses. This scene's local slots are not NHL assignments.

Arithmetic: `d20 + floor((attribute - 10) / 2)`, with external support adjustment
0–2. Committing F3 supports home outlet/carry/pass and vacates cover; holding F3
supports home counter defense. No dual benefit. The response includes exact counts
out of 400 die pairs and inspectable traces. No win becomes a shot or goal.

Native GET `/api/v1/rinkcraft/possession-rules` and POST
`/api/v1/rinkcraft/possession` use the usual envelope. POST body:
`{character_rules,rules,request}`; request: `{hero,seed,commit_support,actions}`.
The WASM `possession_session` method delegates to the same core. There is no
server-side session mutation. Native rules path: `ICELINES_RINKCRAFT_ENCOUNTER_RULES`.
Optional browser asset: `public/rinkcraft-possession-rules.json`. Authored rules
belong to Rinkcraft. Missing encounter rules preserve character creation; Start retries.

Replay download exports rules, request and session under
`rinkcraft.possession-replay.v1`. Replaying its request reconstructs it; no replay
file-import control is claimed. Character export/import remains separate.

Validation: five possession tests, eight creation regressions, six arithmetic
tests, 26 focused Node tests; actual native/WASM parity over ten templates and
four sequences including a defense hero; illegal pass/post-terminal rejection;
complete/recovery/counter branches; a cover-dependent outcome; real desktop/mobile,
export, offline UI checks; unchanged NHL JSON; one-page character printing;
existing creator E2E; and 46-asset browser distribution verification. Screenshots
were inspected. An intermediate offline run detected stale files after bare tsc;
the complete matching bundle rebuilt successfully without weakening integrity checks.

Replay with existing owned 8772/8773 servers, external rules and pinned dependencies:

```powershell
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-core rinkcraft_possession --lib --locked
cargo test -p icelines-core rinkcraft_game --lib --locked
cargo test -p icelines-core rinkcraft_contest --lib --locked
node icelines-browser/build.mjs
node scripts/verify-browser-dist.mjs
node icelines-browser/e2e/rinkcraft-possession.mjs
node icelines-browser/e2e/rinkcraft-game-studio.mjs
```

Default Conductor/standard array: seed 7 completes the sequence; seed 92 gives a
counter that holding cover stops and committing support allows to advance, on
identical dice. Selected examples are not sampled strategy performance or balance
evidence. Lanes/opposition are scripted. Shots, saves, fatigue, shifts, clock,
competition infractions and full matches remain outside this scene. Local only.
