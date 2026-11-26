# Observed Deployment and Unit Links v1

Unit Links describes verified shared ice time. It does not establish L1–L4,
D1–D3, PP1/PP2, tactical stations, coach intent or causal chemistry. Assigned
Role remains a separate sourced category. This fills the character's observed
party relationships, independently of Attributes, Task Skills and Signature Methods.

## Population and scope

An optional `unit_links` field on the existing stat profile binds player, season,
position family and selected GP. Source game IDs, first/last dates and paired
source/event fingerprints bind the observed subset. Missing fields remain absent.
Matched appearances require verified positive regulation participation; a backup
goalie's bench roster entry does not become an appearance. Historical trades
retain team identity on each group and partner record.

The first import accepts the same explicitly paired, checksum-verified native
shift manifest as [late-lead exposure](rinkcraft-under-pressure-v1.md). It reads
box-score position and names, reconciles full-game TOI within an authored one-second
tolerance, deduplicates identical intervals and reconstructs regulation segments.
Goalie TOI is also reconciled. Overtime is used for full-game TOI validation and
excluded from these displayed unit totals. HTML-derived shifts remain a separately
declared source category outside this import path.

All active participants on both teams must be verified for a segment to enter
unit statistics. Invalid occupancy, conflicting intervals and unverified active
participants leave the focused player's time unresolved. This is deliberately
stricter than the earlier individual late-lead exposure count: co-deployment and
manpower claims depend on the other participants too. A participant with unusable
own shifts is excluded from that player-game. Unresolved time is retained in the
record; it is not redistributed among verified groups.

## Relations and denominators

| Focus | Main 5v5 relation | 5v4 / 4v5 relation |
|---|---|---|
| Forward | Joint three-forward trio | Joint five-/four-skater group |
| Defenseman | Joint two-defenseman pair | Joint five-/four-skater group |
| Goalie | Joint defense pair in front while that goalie plays | Joint skater group in front |

Ordinary 5v5 trios/pairs require three forwards and two defensemen. Nonstandard
position compositions can contribute verified 5v5 time without being forced into
a standard trio/pair. Empty-net and other manpower remain separate from 5v5,
5v4 and 4v5; they do not inflate PP/PK group counts.

Group share = joint group seconds / the focused player's verified seconds in
that named manpower state and matched window. The focus is omitted from the
displayed companion names, while full group IDs remain in the record. Concurrent
5v5 partner shares overlap and must not be summed to 100%. A goalie uses that
goalie's own on-ice denominator. All formatting and calculations run in Rust.

Keep the leading two groups per state and three frequent 5v5 partners. Preserve
the total number of observed groups. This is a presentation budget; the source
data and command reproduce the remaining combinations. Equal leading seconds
produce an explicit co-leading count, rather than an arbitrary primary assignment.
Stable ID ordering only controls presentation ordering among ties.

## Recent window and display

Recent means the last five matched appearances, or all available when fewer than
five. It is not necessarily the team's last five games. Compare group identity
and team, not spelling changes in display names. A recent/full leading-combination
difference is reported only when both windows have a unique leader. This is an
observed composition difference, not evidence of a promotion, demotion or coach's
permanent reassignment. Exact dates remain visible in the disclosure.

The screen shows a main group in each positive-time state, shared time and share,
with alternatives, partners, recent history and limitations expandable. Deployment
adds the matched subset's 5v5/5v4/4v5 totals below the independent season workload.
The workload caption distinguishes assigned units/stations from observed groups.

The single-page Letter version keeps the primary 5v5 relationship and compact
manpower time totals. Detailed PP/PK personnel and expanded notes remain on screen.
The interactive click-instruction footer is omitted on those printed sheets;
attribute Estimate labels and documentary coverage remain present.

## Refresh and validation

Configured native and WASM profile projection preserve the optional record.
Skater/all refresh can rebind selected GP only against the same admitted parent
event fingerprint, player, season and position family. A changed or unavailable
parent withholds the record until explicit paired import is rerun. A refresh
without a paired parent cannot silently reuse old neighbors.

Validate actor/window identity, game and date scope, sorted unique IDs, positive
group time, member cardinality/positions, self-membership rules, denominator
bounds, ties, source fingerprints and Rust display strings. Recent game IDs are
a bounded subset; formatted percentages cannot override their raw counts.
This record changes no attribute score, feat, method, class or simulation modifier.
