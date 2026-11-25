# Under Pressure: observed challenge/response catalog v1

This section describes responses to named, observable opportunities. It is
separate from capability Attributes, earned Task Skills, documentary methods and
coaching assignments. It awards no tier, feat, class, modifier or simulation bonus.

| Challenge | Opportunity | Response counted | Source |
|---|---|---|---|
| Contested draws | Credited faceoff contest, all manpower situations | NHL credited win | NHL play-by-play |
| Late-close draws | Regulation third period, 15:00 or later, pre-draw score gap ≤1 | NHL credited win | NHL play-by-play |
| Rapid follow-ups | Provider-tagged rebound on goal, 0–1 second after prior event | Save | MoneyPuck.com shot archive |
| Cross-slot rebounds | Provider royal-road rebound flag, attempt on goal | Save | MoneyPuck.com shot archive |
| Rebound saves | Provider rebound flag, attempt on goal | Save | MoneyPuck.com shot archive |

The late-close boundary is an authored Rinkcraft definition, not an established
psychological pressure measure. Contested draws supply a descriptive baseline.
The goalie buckets overlap and must not be added as independent trials.

## What each entry carries

Short name; challenge; response; successes; opportunities; a Rust-formatted
count-first result; exact definition; source credit; capture timestamp; coverage
description; and an interpretation limit. Parent actor, season, group and selected
GP bind the card record. Raw faceoff samples also carry their own season, and
goalie archive URLs must match the card season before responses are admitted. GP is not an event-outcome denominator. Event/shot samples
retain their own capture windows and can differ from season-summary snapshots.

Positive opportunities permit an outcome rate. Zero opportunities show “No
observed opportunities”, not 0%. Missing samples and missing tracking are named
gaps. Small samples remain visible without a mastery rank or confidence claim.

## Measurement gaps

- Press resistance needs pressure-tagged possession opportunities and responses.
- Contested retrievals need observed contest opportunities, including failures.
- Entry denial needs a complete defended-entry sample and responsible defender.
- Goalie traffic tracking needs screen/sightline opportunities.

Giveaways, hits, blocks, source quotes, raw skating maxima and goals-only geometry
do not supply those denominators. A defenseman without a contextual sample keeps
the gap; the system does not assign zero ability or substitute faceoffs for defense.

## Evidence interpretation

Faceoff credits are not retained possession or downstream value. The primary
research explicitly studies how clean wins, direction and context affect what
follows a draw: [Czuzoj-Shulman et al., *Winning Is Not Everything* (2019)](https://arxiv.org/abs/1902.02397).
This pilot does not measure those additional factors or adopt that paper's model.

The public [MoneyPuck.com data description](https://moneypuck.com/data.htm)
documents shot-event data and links its dictionary. This catalog reuses the
already parsed provider sequence flags. Misses are excluded from save
opportunities; some follow-ups originate after misses, so a rebound response is
not automatically a goalie-created failure. Neither elapsed event time nor the
royal-road flag measures goalie travel, reset speed or isolated technique.

## Admission and display

The Rust parser requires finalized regular-season games, matching game/season,
unique event identity/order, valid opposing roster actors, valid clocks and a
reconciled scoring sequence. Late context uses the score before the faceoff.
Shootout scoring is excluded from regulation/OT goal counts; overtime draws are
excluded from the late-regulation bucket. Offline cache loading verifies URL keys,
HTTP status, timestamps, byte limits and body SHA-256 before producing output.

The primary sheet shows short names and count-first results. Definitions and
sample context expand on demand. Existing documentary responses remain available
separately. Native and browser render the same Rust record; no client hockey
calculation is involved. Historical/older catalogs can omit the optional field.

## Before turning outcomes into gameplay

Test reliability across seasons, opportunity volume, opponents and deployment;
separate overlapping buckets; evaluate shrinkage and held-out prediction; and
define what a simulated success actually changes. Until then, these are observed
samples, not calibrated future success probabilities or “clutch” ratings.
