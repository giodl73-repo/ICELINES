# Rinkcraft sheet reconciliation

CHG-142 / WP-RINKCRAFT-18. Owner handoff accepted 2026-11-09.

## Branch decision

Common base: `6402e5e`. The C: branch at `cfc4337` and the isolated F: branch
at `66f3cbe` are alternative implementations. No merge or blanket cherry-pick
was performed. Keep the owner's approved Rinkcraft-category form from the four
F: commits as the active presentation. Keep the C: checkout and its compact
rail implementation intact for provenance. CHG-141 belongs to that alternative.

The accepted layout and category mapping live in
[rinkcraft-character-card.md](rinkcraft-character-card.md). The 650px acceptance
criterion from the other branch does not govern this Letter-format form.

## Semantic contract

Attributes describe proposed capability categories, with a 1–20 display scale:
Skating, Handling, Passing, Shooting, Reading, Defending for skaters; Tracking,
Positioning, Movement, Rebounds, Recovery, Puck Play for goalies. All are Unrated.
No rubric, assessment schema, arithmetic modifier or exceptional-tier threshold
is defined by this presentation. Existing 1–5 descriptive percentile bands must
not be converted into capability ratings.

Keep raw values and percentiles in Measured Record; GP/minutes in Record Counters.
Use the approved Rinkcraft names for tasks, responses, methods, tendencies,
assignments, links, limitations, features, coverage and deployment. Unsupported
tasks remain Unassessed; unsupported duties/history/relationships remain
Undocumented. Class purpose is not a coaching assignment; an open question is
not a known limitation. No HP, death saves or universal proficiency bonus.

## Selective integration

Retain the F: front renderer, native counterpart, multi-column/print styling,
full dossier, focused capability tests and reference comparison. Carry over the
C: branch's explicit per-measure interpretation/peer-count explanation inside the
dossier, preserving the F: disclosure arrangement and full evidence labels.
Do not import the raw-stat attribute rail, its compact-drawer CSS, 650px test or
its parent-disclosure assumptions into this different front/dossier design.

Preserve the class-catalog failure versus Unscouted distinction on the front,
including the footer. Native position selection may fall back to the numerical
source group when no ordinary season record exists, matching the browser's
ability to retain goalie-specific fields. This is UI projection only.

## Validation boundary

Reproduce TypeScript/focused tests, native template compilation, four retained
fixture layouts on each host stylesheet, mobile/Letter exports and the reference
comparison. These fixture snapshots are review artifacts. They do not establish
a rebuilt full native/browser app, full runtime parity or validated capabilities.
No original checkout changes, hosted build, push, merge or publication is implied.

Results: [reconciliation validation](../../docs/rinkcraft-sheet-reconciliation-2026-11-09.md).
