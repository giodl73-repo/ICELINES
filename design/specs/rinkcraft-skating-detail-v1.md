# Skating Detail from admitted measurements

The Skating Detail box presents recorded inputs alongside the broad 1–20 estimate.
It does not award an ability, feat, class or technique. Response Timing remains
independent; neither burst frequency nor peak speed supplies a reaction-time test.
Goalies retain their separate Movement Detail box.

## Input contract

Read the unique `Skating` score's admitted parts:

| Key | Required unit | Meaning |
|---|---|---|
| `peak-speed` | `mph` | Reported season maximum |
| `bursts` | `/60` | Recorded 20-plus-mph bursts per official all-strength TOI hour |

The parent profile must identify a skater, a season, positive GP and minutes, and
an all-strength scope. There is no ten-game gate for displaying an observation.
Only finite nonnegative measurements with unique matching keys/units are shown.
Missing and measured zero stay distinct. A burst-only record uses the burst rate
as its headline and explicitly says the peak is unavailable; no speed is inferred.

Optional `edge-distance` in the same admitted context adds the already normalized
cumulative skating distance in miles. It is not a speed or quality measure and
does not change the Skating recipe. Peer positions reuse existing part percentiles
and measured-peer counts. They are not recalculated by this display or treated
as new independent evidence.

## Projection and presentation

Rust generates a typed optional `skating_detail`, bound to actor, season, position
family, GP and minutes. Its strings must match a canonical rebuild from the parent
inputs. Native and WASM stat-profile admission produce it automatically. Persisted
catalogs can omit it; old canonical catalog bytes do not need a migration or a new
statistical refresh. No new source fetch, rank, scalar estimate or numerical formula
is introduced. A cached projection with mismatched metadata is withheld.

The front shows the reported peak and burst rate with a recorded/all-strength
caption. Measures & scope expands distance, existing peer positions and measurement
limits. The compact print view retains headline measurements and hides the expander,
including when open. Older profiles preserve the prior supported EDGE/style maximum
fallback. A missing new projection never creates a client-side estimate.

Peak speed is not puck-carrying speed, acceleration, turning or pivoting ability.
Burst rate uses official on-ice minutes, not a demonstrated tracking-time denominator.
All hockey interpretation and formatting run in Rust; browser code binds and renders.
