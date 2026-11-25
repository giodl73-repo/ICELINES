# Earned task repertoire — 2026-11-24

Task Skills now presents a selection of earned feats as a compact repertoire:
short task name, existing tier, readable prerequisite and the player's measured
result. This changes presentation selection, not qualification or ability scores.
Under Pressure remains a separately sourced challenge/response repertoire.

## Shared selection

`rinkcraft_task_repertoire::select` runs in Rust for native profiles and WASM.
Newly evaluated or refreshed achievement profiles carry an optional
`rinkcraft.task-repertoire.v1` projection bound to player, season, competition,
position group and GP. Older browser profiles retain the existing feat list.
Browser guards reject stale metadata or references to non-earned feats.

The selector only considers earned Task Skills feats with an explicit task
mapping. It greedily prefers the largest number of additional metric keys, then
tier, condition count, weakest percentile and stable feat ID. Each task family
appears at most once; a selection must introduce an additional metric key. It
stops at five or when no complementary task remains. Input order cannot affect
the result. The cap and diversity policy are presentation choices, not validated
measures of hockey skill or independent evidence.

Families: scoring, creation, contact, blocking, disruption, penalty drawing,
power-play assists and goalie saving workload. Combined feats retain all their
original conditions and tier. For example, “Shoot & assist” still requires both
the original shot and assist conditions. It does not establish a passing method.

Every original earned, locked, unavailable or unrecognized feat remains in the
category library. Adding a new mapped task requires a deliberate family/name
decision. The selector never invents feats to reach five, changes class, changes
an attribute, infers a coaching role or awards a simulation bonus.

## Frozen local population

The existing 2026–27 local public-source snapshot contains 787 skater profiles.
The new projection was applied twice per profile with identical results; all
original achievement fields and attribute score histories were preserved.

| Player | Selected repertoire |
|---|---|
| Nathan MacKinnon | Shoot & assist; Shoot & score; Draw penalties; PP assists |
| Sebastian Aho | Shoot & assist; Disrupt & score; Goal production; PP assists; Draw penalties |
| Adam Fox | Disrupt & score; Shoot & assist; PP assists; Score & block; Shoot & score |
| Alex Killorn | Draw penalties |

Distribution by selected count: 0: 209; 1: 142; 2: 136; 3: 135; 4: 74; 5: 91.
Mean: 1.995 across the full snapshot, including players below the ten-game gate.
Of 618 eligible skaters, the mean is 2.54. Of the 209 empty repertoires, 169 are
below the GP gate and 40 meet it but have no earned Task Skills feat.
Killorn's single selected task exposes limited coverage of the current task
catalog; it does not mean he has only one useful hockey skill.

## Reproduction and validation

Retained ignored helper: `target/task-repertoire-study/project.mjs`. It calls the
actual WASM `BrowserEngine.task_repertoire` API against each retained profile,
checks deterministic output and preservation of qualifications and scores, then
updates the local 8780 preview. No new source fetch is involved.

```powershell
node target/task-repertoire-study/project.mjs
node icelines-browser/e2e/rinkcraft-task-repertoire.mjs
```

Four live cards passed selected ID/name order, measured values, full-library
retention, rulebook access, actual WASM selection parity, mobile overflow and
no page errors. All four single-page Letter PDFs were independently checked with
pypdf for every selected task. Desktop screenshot inspected. Browser suite:
246 tests passed. Three Rust selection tests and four existing achievement tests
passed, covering unchanged qualifications, deterministic ordering, family/cap
rules, missing/locked feats, historical identity and goalie selection. Native CLI
compilation includes the Askama templates. No new hosted build or publication.

Preview: <http://127.0.0.1:8780/?player=8479323>.
