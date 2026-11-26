# Goalie singleton feat trial v1

Opt-in exploratory comparison; the active 34-definition feat registry remains
unchanged. Split the two measurements already used by **High-save workload** into
separate candidate patterns:

| Candidate | Formula | Placement |
|---|---|---|
| Save percentage | Reported NHL saves / shots against | Task Skills; saving family |
| Shot workload | Reported NHL shots against / appearances | Role Coverage; workload family |

Keep the existing same-season goalie-only reference, minimum 10 GP, minimum 20
measured peers, midranks, and Bronze P60 / Silver P75 / Gold P90 thresholds. Zero
shot exposure has no save percentage. Ineligible and missing observations retain
their states. A zero qualifying result never becomes a padded award.

This is a conjunction-coverage experiment, not a new technical measurement.
Saving outcomes include shot difficulty, team and deployment effects. Workload is
activity/exposure, not saving quality, starts or effort. The two candidate metrics
already occur in the registry; splitting them does not create independent evidence
or new attribute values. No class or simulation bonus is assigned.

## Source and computation boundary

Read complete regular-season NHL goalie summary assemblies. The source adapter
checks identity/season, complete totals, source URL/window, integer counts,
starts <= GP and saves <= shots. Preserve reported goals against separately;
it is not substituted for the reported save numerator. Additional differences
between goals against and shots minus saves are retained as accounting notes,
without explaining or repairing them.

The native source example verifies cached-page checksums and capture timestamps,
assembles strictly contiguous pages with agreeing totals, and rejects duplicate,
overlapping, incomplete or oversized inputs. Original source metadata remains in
the output. It writes only to a new output directory after successful evaluation.
Checksums establish capture integrity, not independent source authentication.

`rinkcraft_goalie_feat_trial::evaluate` compares the existing rules and an explicit
36-rule trial on identical goalie inputs. It checks every existing feat for exact
equality and reports new recognition, tiers and adjacent-season repeats. Annual
references change; repeats are conditional on a next eligible observation. Record
attrition explicitly. Partial current-season observations are not whole-season
prediction tests or evidence of stable goalie traits.

## Review cards

Prepare isolated trial profiles as a direct child of an existing local preview
site. Before preparing them, require same actor/season/GP and exact existing feat
equality with the base cards. Verify all current award objects through the actual
WASM evaluator using real complete report assemblies; verify repertoire parity.

The trial has 78 goalie cards, an explicit experimental banner, an experimental
marker inside every printed card, a link to the active card, and a four-season
summary. Normal profiles, stat scores and curated identity are unchanged. Preparing
an existing output directory is rejected. Trial artifacts are not a hosted release
or automatic promotion of these candidates into the default registry.
