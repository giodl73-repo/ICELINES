# Conditional failed-transport recovery fixtures

Explicit authored draft/episode/result0.10, fictional manual scores of 10, seed 20261115.
The existing shot-encounter.0.1 character rules are unchanged. All twelve actors
appear in each authored retrieval scene. Positions/rates are projections, not NHL
tracking observations or capability-to-speed conversions.

Target 1 deliberately guarantees ordinary branch traversal; target 40 deliberately
fails the named task. These are accounting cases, not hockey completion rates.

- denied-exit-counter: failure at x=-8 gives B offensive control after collection.
- denied-entry-counter: failure at x=3 gives B neutral control, then westward entry.
- failed-delivery-counter: puck stays at departure x=3; no completed flight crossing.
- failed-outlet-receipt-counter: receipt fails at x=-3 after the outlet crosses.
- failed-entry-receipt-counter: receipt fails at x=6; B retrieves in its defensive end and must exit.
- own-neutral-recovery: A recovers but cannot retain the failed neutral-progress marker; repeats are bounded.
- no-arrival: no actor pursues the loose puck; window ends without an owner.
- unknown-context: unresolved pressure is not a failed task and does not admit recovery.

Optional carry `on_failure` and pass `on_delivery_failure`/`on_receipt_failure`
policies each name a scene and a bounded basis. Absent policies preserve the old
loose stop. A successful recovery never means that the denied task succeeded.
Unknown context/ruling and clock/eligibility boundaries cannot manufacture retries.
Goalie-first access/pressure remains explicit and unmapped. No new shot is implied.

Rust tests additionally cover pressured recovery denial, absent policy, unknown
receipt, deadline, goalie access, penalty expiry, validation of unreached branches,
old-version semantics and exact replay. Dated reproduction/custody is in PUCK's
`scenarios/nhl/2026-27/rinkcraft-transition-recovery/`.
