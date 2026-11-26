# Power-play exposure and assist context v1

Role Coverage gains an expandable **Power play** summary. This separates PP share
of own ice time from PP share of credited assists. It describes observed usage
and production; it does not assign a unit, station, class or performance bonus.

Use the selected skater's existing all-strength NHL context:

- S = PP minutes / all-strength minutes (`pp-share`).
- P = PP assists per 60 PP minutes (`pp-assists`).
- A = all assists per 60 all-strength minutes (`assists`).
- PP assist share = P × S / A, for positive A and compatible components.
- Outside-PP assist rate = (A − P × S) / (1 − S), for positive outside-PP time.

Outside PP includes all remaining situations, not just even strength or five-on-
five. Calculate before rounding; display shares to one decimal and rates to two.
Each displayed rate retains its own minutes denominator. Do not divide the PP
rate directly by the all-strength rate to obtain an assist share.

Require a skater, positive GP and finite positive all-strength minutes. S must be
finite in [0,1]. No S means no record. Missing/nonfinite/negative assist components
prevent composition and outside-PP rates. Reject a PP contribution greater than
the total by more than 1e-8; require zero PP rate when S=0 and agreement between P
and A when S=1. Clamp only tolerance-scale residual noise. Zero assists have no
share; zero strength-specific time has no per-60 rate. Missing PP rate with zero
PP time stays unavailable for composition rather than inventing a credited count.
There is no ten-GP admission gate.

Rust derives the optional record during profile admission from existing refresh
inputs. Catalog validation binds actor, season, group, GP and minutes, and rejects
altered records. Native Askama and TypeScript render Rust strings; older cards can
omit the field. No additional fetch or enrichment command is needed. Keep the
disclosure closed by default and hidden on compact print. Expand for four values,
exposure and interpretation; a nested Calculation disclosure holds the formula.

Opportunity, teammates, systems and finishing influence assists. These measures
are not passing success, PP1/PP2 membership, coaching assignments or isolated
impact. Existing attributes, feats, documentary roles and simulation rules stay
unchanged.
