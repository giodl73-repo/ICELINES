# Goalie Rebounds after contact

Episode/draft/result 0.6 uses `resolve_shot` with exclusive `rebound_context`.
Omit authored `containment` weights. Version 0.5 keeps its original weighted
policy; no silent migration. `hold_eligible` needs target 1..40 and a basis;
`unknown` needs a reason. Only named verified goalie Rebounds is used after
Tracking has actually produced contact. Unknown stops only when reached.

Roll one d20 plus the selected existing score term; total >= target holds and
whistles. Otherwise the puck remains a loose rebound. This is a hold-eligible
subset, not every aspect of safe/dangerous rebound redirection. Neither branch
invents player possession, official save credit or a new shot. Pending minor
service follows a hold's whistle, not a mere contact/rebound. No extra live time
is added beyond the existing encounter step; this is a bounded same-contact
resolution, not spatial motion or goalie Recovery.

Target 13 with half-offset score 10 gives 8/20 holds (40%), intentionally bridging
the older **authored** neutral hold share, not fitting NHL data. Rebounds16 gives
11/20 (55%). Direct scoring needs a target appropriate to its different scale.
The old weighted choice and new d20 need not pick the same individual outcome;
old episode versions themselves must still replay byte-identically.

Seven drafts: neutral, Tracking16, Rebounds16, both16, harder target17, unknown
eligibility, unmapped Recovery16. Input game rules stay `shot-encounter.0.1`
because the character template/rule definitions are unchanged. The new task
mapping is explicitly event 0.6. Tracking/Rebounds independently affect contact
opportunity and conditional containment. Initial goal chance stays unchanged
when only Rebounds changes; later rebound-shot consequences remain separate.

Illustration selection is explicitly conditional: first seed >=20261115 whose
neutral first shot contest records goalie contact. Seeds 20261115 and 20261116
do not; seed 20261117 does (10 vs 16 dice). Selection is based only on contact,
before examining containment outcomes; all variants use that seed. The PUCK
runner retains the two earlier candidate results as well. These are conditional
demonstrations, not an unconditional frequency estimate. Exact analytic branch
probabilities still include the differing Tracking contact chances.
