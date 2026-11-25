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

Late-close draws can include an optional score-state breakdown: tied, leading by
one and trailing by one. These refer to the player's team before the draw, not
the winner's team by default. Taken and won counts must partition the combined
late-close counts exactly. The breakdown stays inside the existing expander;
it is not three additional feats or a context-adjusted skill rating. An older
aggregate sample can lack the breakdown without implying zero opportunities.

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

### Context actions without success denominators

The separate **Context actions · counts only** group can show **Protecting lead**:
credited opponent-shot blocks and takeaways in regulation period three at 15:00
or later, with the actor's team leading by exactly one before the event. These
are named-player event counts, not opportunities/successes. They cannot yield a
success percentage, entry-denial grade, possession-retention inference or clutch
rating. The all-situations counts appear inside the disclosure as a baseline.

The shooter owns a blocked-shot event, while the opposing blocker receives the
defensive credit. Same-team blocks and provider `teammate-blocked` events are
excluded. An `other-block` with no blocker has no player credit and is excluded;
other missing/unknown actors or inconsistent owner teams fail admission. Goalies
do not receive skater action observations. Overtime and shootout events do not
enter the late-lead counts.

Raw optional `action_sample` stores actor, season, counts, roster-game coverage,
cached-game coverage, capture time and source fingerprint. Derived optional
`actions` carries display text. Old records omit both. The same Rust builder and
validator serve offline enrichment, routine refresh, native and WASM projection.
Zero counts mean no credited actions in the supplied sample; they do not show
whether the player was deployed while protecting a lead. Roster games are not
exposure minutes. Missing samples produce no action row. Existing defensive
measurement gaps remain, even when an action count is present.

### Optional matched shift exposure

An action sample can carry optional `exposure`: sorted unique matched game IDs,
observed late-lead ice seconds, unresolved team-context clock seconds, late-lead
blocks/takeaways from those same games, and a paired-source fingerprint. The
disclosure renders this separately from the larger headline action sample.
Missing exposure stays absent. No percentage, per60 rate or new ability follows.

The explicit offline `profile-pressure-exposure` import requires checksum-verified
finalized event captures, matching box scores and complete native NHL shift
responses. HTML-derived shift products are a separate source category and are
not admitted by this first import path. Full-game player shift totals, including
overtime, must match the box score within an authored one-second tolerance;
overlapping intervals or larger discrepancies exclude that player-game, rather
than creating zero exposure. Exact duplicate intervals count once.

The existing regulation participation reconstructor supplies half-open segments.
Goal timestamps split the final-five-minute window: the post-goal score applies
to the next duration interval. Same-second event order still controls credited
action context, but does not establish on-ice event responsibility at a change.
Overtime is excluded from late-regulation exposure. All manpower situations and
empty-net conditions are included; this is not a five-on-five or PK grade.

Conflicting shift spans and invalid occupancy censor time. Reported unresolved
seconds describe team clock while leading, not additional player ice time.
Observed player seconds are therefore conservative where clock is censored.
Matched-game action counts are not divided by the retained seconds: an action
could occur at a second-resolution boundary or inside censored clock. Nor does
ice time enumerate attempted/available blocks or takeaways.

Reprojection preserves the paired study. Routine refresh and offline record
rebuilds retain it only if the parent action sample is identical, including its
source fingerprint. Changed captures drop the study; rerun the explicit import
against those captures. Generic shift synchronization stays disabled.

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
