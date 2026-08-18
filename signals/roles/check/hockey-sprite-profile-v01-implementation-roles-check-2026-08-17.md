---
skill: roles-check
topic: hockey-sprite-profile-v01-implementation
date: 2026-08-17
roles_used: 4
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Hockey sprite profile v0.1 implementation review

Artifact type: hockey-domain selector profile and documentation.

## Selected roles

- EDGE — missing combinations, ambiguity, and fallback behavior.
- FORGE — contract boundaries and strict validation.
- SCOUT — hockey meaning and player/action plausibility.
- GLASS — whether selector output can remain readable in visual review.

## EDGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Unknown handedness is modeled explicitly rather than guessed. | P3 | selectors | Keep unknown distinct from left/right. |
| 2 | Only nine reviewed combinations exist; many valid hockey actions correctly fail. | P2 | coverage | Grow coverage from measured plays and test each added binding. |
| 3 | Facing is editorial stage direction, not inferred from undocumented EDGE coordinates. | P3 | boundary | Preserve this distinction in downstream evidence. |

## FORGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | IceLines owns only opaque domain mappings; generic machinery stays in REEL. | P3 | architecture | Keep renderer dependencies one-way. |
| 2 | Exact library hashes prevent profiles silently drifting against poses. | P3 | dependency | Update hashes only with reviewed pose changes. |
| 3 | The YAML profile lacks an IceLines-native CLI wrapper and fixture test. | P2 | tooling | Add a profile audit command when multiple consumers exist. |

## SCOUT

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Skate touch, carry, pass, receive, shot, defend, goalie, and celly states match this play's needs. | P3 | bindings | Treat this as event-derived starter coverage. |
| 2 | Kartye, Miller, and Sheary handedness was checked against local player data. | P3 | selectors | Continue using player IDs, not name matching. |
| 3 | A single defensive-brace pose cannot express every gap-control or stick-lane choice. | P2 | hockey vocabulary | Split defensive states only after reviewed examples demonstrate the distinction. |

## GLASS

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Human-readable selector and binding IDs support contact-sheet labeling. | P3 | naming | Carry these labels into review packets. |
| 2 | Quality and fallback fields can expose compromises without color-only encoding. | P3 | reporting | Render quality as text in coverage reports. |
| 3 | No hockey pose coverage report exists yet. | P2 | review UX | Add a compact role/action/phase matrix when the profile grows. |

## Synthesis

Roles reviewed: 4  
P1 blockers: 0 | P2 issues: 4 | P3 notes: 8

Verdict: **APPROVED-WITH-CONDITIONS**

Top finding: the nine bindings are sufficient for this event but must not be
presented as general hockey coverage.

Cross-role consensus: grow the vocabulary empirically and keep unknown or
unsupported states visible.

## Amendments

1. Add measured actions only with a reviewed fixture.
2. Add an IceLines coverage/audit surface before the profile becomes broad.
3. Keep attacking direction and coordinate interpretation evidence-bound.
