# Explicit-unit receiving gate

Rule version `0.3.0-playtest` separates a pass arriving from possession. In the
unit studio choose **Start receiving scene (0.3)**, set seed **43**, then **Start
new unit scene**. The supplied neutral lineup reaches arrival after outlet,
carry and pass. **Roll unit receive** uses the assigned receiver's Handling
against the assigned away interceptor's Defending, without another support bonus.

An arriving puck has no controller and is not loose. Successful reception gives
the receiver control and completes this bounded sequence; it awards no shot or
goal. Failed reception makes the puck loose, with recovery still assigned to the
declared carrier. A tie ends the opportunity with a loose puck. No geometry or
nearest-player rule is implied. At the sixteen-action cap flight may remain
unresolved. Held, in-flight and loose states are mutually exclusive.

The upgrade starts a new scene and clears actions while retaining characters,
duties, selected hero, seed and support. The default authored setup stays 0.2.
Legacy 0.1 and explicit-unit 0.2 use their frozen resolver behavior and omit the
new optional flight field. Unit input/replay envelopes retain v1; embedded rule
version routes behavior, and the 0.3 inner scene is v3. Unknown rule versions are
rejected. The frozen 0.2 fixture was captured from the prior 93befe3 WASM build.

Shared Rust owns the rule, command, validation, seeded dice and exact counts.
The existing JavaScript panel renders the version, flight and legal receive action.
There is no new rating conversion or NHL assessment. Shooting and five goalie
capabilities remain unexercised by this scene.

Validation: 81 focused Rust tests and 26 Node tests passed. The new
`icelines-browser/e2e/rinkcraft-reception.mjs` exercises ten-template native/WASM
command parity, exact frozen 0.2 import, explicit upgrade, pending-flight export/
resume, invalid-controller rejection, actual native/offline-browser UI, low/high
Handling with identical earlier events/dice, no extra support, carrier recovery,
390-pixel layout and one-page Letter print. Independent pypdf checks confirm both
612 by 792-point pages, current identity/attributes and excluded workbench.

The authored Rinkcraft packet is
`C:/src/Rinkcraft/game/playtests/2026-11-11-receiving/`. Four CLI variants verify
their exports. Seed 43 reception dice are 2 versus 5: Handling 1 totals -3, 12
totals 3 and 20 totals 7 against 6. The first two leave a loose puck; 20 controls.
An unused Shooting change leaves events identical. This selected illustration
demonstrates rule sensitivity, not empirical probabilities or game balance.
Full replays, screenshots, PDFs and test logs remain under ignored
`target/receiving`; the research packet retains compact results and hashes.

The six prior E2E suites also pass against the new builds: explicit-unit studio,
replay timeline, replay import, plan comparison, possession and game creator.
Browser distribution verification loads the actual WASM and verifies 49 shell
assets and 75 supplied public packages. This is local validation only; no hosted
build, push, merge or publication occurred.
