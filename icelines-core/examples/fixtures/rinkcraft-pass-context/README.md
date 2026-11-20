# Declared pressure and open-task difficulty

Draft/episode/result 0.4 adds `pass_with_context`. Both lane and receipt must be
explicitly `open`, `defended` or `unknown`; no missing field defaults to open.
Use the existing CLI `build DRAFT REQUEST`, then `REQUEST RESULT`.

Open means no opposing contest is declared at that stage, not automatic success.
Specify authored `target` 1..40 and a nonempty `basis`. Roll one d20 plus the
selected score-rule term; total >= target succeeds. At half-offset score 10,
target 5 gives 16/20 = 80%; score 16 gives 19/20 = 95%. These targets are game
inputs, not measured NHL difficulty. Direct scoring needs targets appropriate
to its different numerical scale; target 5 is not rule-independent difficulty.

Defended names an actual opposing active skater: Passing vs Reading for delivery,
Handling vs Defending for receipt. Existing opposed rules and strict-win behavior
are unchanged. Open checks record no phantom opponent/profile or second die.
Unknown requires a reason. A reached unknown lane stops before release; a reached
unknown receipt stops in flight before receipt time/contact. The step's delivery/
assessment time is already consumed. An unreached unknown receipt need not stop
a pass whose delivery has already failed.

Character profiles are required for passer/receiver and only named defenders.
Every supplied profile is verified. Pressure is declared, not inferred from
geometry. All named participants must be active and properly sided at delivery;
receipt actors are rechecked at receipt. No substitutions, goalie capability
mapping or spatial feasibility claim is introduced.

Each reached check has a `context_task` audit: open score/rule/term/target/die/
total/exact 20-roll chances or the existing defended 400-pair audit. Conditional
receipt time/RNG, last touch/deliberate play and loose/control semantics remain.
Versions 0.1–0.3 retain their serialized results and RNG paths. Old 0.3 actions
can still be used explicitly within 0.4, but do not silently acquire open stages.

Eight neutral drafts: four pressure combinations, two open-stage capability
upgrades and two unknown-context stops. At the authored target 5, neutral joint
control chances are open/open 64%, defended/open 38%, open/defended 38%, and
defended/defended 22.5625%. Upgrading open Passing or Handling to 16 gives 76%.
Unknown contexts have no inferred probability. Shared seeds do not imply identical
actor dice across one-die and two-die mechanisms. Exact distributions, not one
illustrative outcome, support these comparisons. Admission frequencies and real
pass-completion calibration remain future work.
