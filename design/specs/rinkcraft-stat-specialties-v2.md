# Earned task selection and stat specialties v2

This presentation policy supersedes the novelty-first task selection described
in the [v1 validation](../../docs/rinkcraft-task-repertoire-2026-11-24.md).
Qualification, tier thresholds, attribute recipes and documentary classification
are unchanged. The shared Rust projection is `rinkcraft.task-repertoire.v2`.
Older v1 task lists still render; absent v2 highlights produce no identity strip.

## Task selection

Use only earned feats placed in Task Skills and explicitly mapped to a short
task name/family. Candidates must add a metric key not already represented, and
each family appears at most once. Rank by earned tier first (Gold/Silver/Bronze),
then additional metric keys, condition count, weakest condition percentile and
stable feat ID. Stop at five or when no complementary task remains.

The important correction is that a Gold single-stat specialty can precede a
weaker compound task. The old novelty-first ordering could hide Beniers' Gold
Takeaways behind Bronze Disrupt & score. The full feat library keeps every entry,
including earned feats omitted from the compact selection.

## Identity highlights

Add up to two **Stat specialties** immediately below the documentary identity.
Each chip contains a short mapped name and its existing tier. Label the strip
with the feat season and observed GP; this retains its own capture window when
attribute/event snapshots differ. Curated Class, Specialty, Secondary, Development
and Experience keep their existing values and evidence dates.

Candidates are explicitly mapped earned positive feats from task, tendency,
distinctive-feature or role-coverage categories. Only Bronze/Silver/Gold qualify.
Rank by tier, weakest condition percentile, condition count and stable ID. Select
different families with disjoint metric keys. A second highlight is optional.

Penalty/giveaway cautions, eventful-contact penalties, short-handed goal/point
outcomes and unknown IDs are excluded from this identity strip. They keep their
original category/library positions and awards. The SH exclusion is an authored
presentation choice after the exploratory projection let sparse SH credits lead
ordinary identity summaries; it is not a new exposure gate or finding about value.

Disjoint keys reduce literal repetition, not statistical dependence: assists and
PP assists, for example, remain related observations. These are descriptions of
earned statistical patterns, not technical skills, class assignments, coaching
roles, permanent traits or simulation bonuses. No qualifying highlight means no
strip; never manufacture a specialty to fill the space.

## Boundaries and rendering

The optional `highlights` vector lives in the existing repertoire projection.
Normal feat evaluation/refresh and `BrowserEngine.task_repertoire` call the same
Rust selector. Native and browser retain actor/season/competition/group/GP checks.
The browser additionally requires each referenced feat to remain earned with the
same tier and rejects risk flags. Native uses the same matched-achievement guard
for identity and category rendering.

Keep the strip compact on screen and on Letter print. Print-specific spacing and
name size accommodate the new row; no feat, attribute or documentary coverage
content is clipped or removed to obtain a single page.
