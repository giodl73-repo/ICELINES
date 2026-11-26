# Recorded shot repertoire v1

Signature Methods combines an observed repertoire with separately curated scouted
methods. Recorded types describe attempts, not mastery, class, intent, a feat, or
a rating bonus. Classes and existing sourced methods retain their assignment rules.

## Observation contract

- Scope: selected player and season, in checksum-verified finalized regular-season
  play-by-play. Include goals, saved shots and misses; exclude blocked shots,
  shootouts and goalie attempts. Goals are included in on-goal counts.
- Supported provider labels: wrist, snap, slap, backhand, tip-in, deflected,
  wrap-around, bat, poke, between-legs and cradle. Missing or unsupported labels
  remain unknown; they do not become a guessed technique.
- Share is attempts of one supported type divided by all supported typed attempts.
  Unknowns stay in total attempts and coverage, outside that denominator. This is
  not a share of official season shots on goal or an ice-time rate.
- Capture time, source fingerprint, cached-game count and player roster-game count
  accompany the sample. Roster presence is not deployment. There is no ten-GP gate.
- Zero attempts, unknown-only attempts and absent samples are distinct. No empty
  sample yields a zero-percent skill or a zero-denominator fraction.

The raw sample is optional catalog data. Rust validates actor, season, position,
counts, source metadata and bounds, then derives the display record during profile
admission. Native and WASM use that same record. TypeScript only renders it.
Routine refresh and the offline `profile-shot-methods` command collect the same
sample. Existing attributes, feats and classifications do not change.

## Presentation

Display the three most-used types, sorted by descending attempts with stable ID
tie-breaking. Each has a short name, Attempts/Goals, and typed share. Expand for
all types, on-goal/miss counts and scope. Preserve prior documentary techniques
under **Scouted methods**. Older catalogs and goalies retain the existing method
view. Print retains the first two observed types and hides expanded details.

These events do not establish quick release, one-timers, deception, deliberate
deflection, puck handling or goalie technique. Those need other observations.
