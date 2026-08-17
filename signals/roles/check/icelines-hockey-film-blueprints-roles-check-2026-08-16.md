---
skill: roles-check
topic: icelines-hockey-film-blueprints
date: 2026-08-16
roles_used: [scout, tape, wire, bench, crest]
p1_count: 0
verdict: APPROVED
---

# IceLines hockey-film blueprints roles check

## Artifact identification

**Type:** hockey-domain editorial templates and reusable animation skill.

**Scope:** player-history, team-hype, and animated-moment blueprints; verified
event evidence schema; REEL camera and production-package handoff guidance.

## Role selection

- **SCOUT:** hockey causality, deployment, team context, and rink correctness.
- **TAPE:** fact/source identity and verified-versus-inferred observations.
- **WIRE:** schema boundaries and failure on missing or drifting evidence.
- **BENCH:** deterministic validation and regression conditions.
- **CREST:** visual grammar, ensemble variety, and animation taste.

## Findings

### SCOUT

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Player history requires development, team relationships, role context, breakthrough, and continued work rather than a goal-only highlight string. | P3 | player-history beats | Preserve community and role context. |
| 2 | Team hype rotates forwards, defense, goalies, coaches, officials, benches, and crowd through varied hockey states. | P3 | team-hype rules | Keep position and state balance explicit. |
| 3 | Animated moments preserve puck possession, pass order, attacking direction, result, and reaction. | P3 | animated-moment | Reject hockey-impossible reconstructions. |
| 4 | Helmets, handedness, rink, bench, penalty-box, goalie, and official geometry are explicit gates. | P3 | hockey geometry | Retain these as blocking review conditions. |

### TAPE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Event facts and video observations have separate roles: feeds establish identity while footage establishes visible geometry. | P3 | evidence workflow | Preserve source-role separation. |
| 2 | Observations must be labeled verified, visible, inferred, or unknown. | P3 | moment schema | Never upgrade inference silently. |
| 3 | Career chronology and retrospective interview repositioning require attribution. | P3 | player-history | Keep chronology audits before editorial approval. |

### WIRE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Each blueprint declares required and optional hockey-data inputs rather than assuming every source exists. | P3 | data requirements | Fail visibly when required identities are missing. |
| 2 | The animation skill prefers game and event IDs but has a bounded fallback for clip-only identification. | P3 | moment identification | Preserve unresolved fields as unknown. |
| 3 | REEL handoff names generic contracts without embedding customer paths or assets. | P3 | ownership boundary | Keep KARTS-specific choices downstream. |

### BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | All three YAML blueprints parse and share the expected template version, production-binding rule, and six quality gates. | P3 | template validation | Add a Rust validator only when templates become a CLI surface. |
| 2 | The animation skill passes the skill-creator structural validator. | P3 | skill validation | Keep metadata synchronized with workflow changes. |
| 3 | Frame-budget rules and exact duration sums give animation timing a deterministic check. | P3 | timing budget | Preserve equality rather than visual approximation. |

### CREST

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The templates prefer cause-and-effect multi-player action over rows of isolated portraits. | P3 | visual grammar | Preserve ensemble composition. |
| 2 | Kinetic cel devices are described generically without copying named characters, frames, or signature compositions. | P3 | style boundary | Keep influences translated into primitives. |
| 3 | Sprite versus full-cel choice follows composition and perspective needs instead of an arbitrary realism target. | P3 | animation mode | Continue choosing the cheapest readable form. |
| 4 | Camera actions share verified play beats and require crop proof at each delivery aspect. | P3 | camera grammar | Preserve dual-aspect review. |

## Synthesis

Roles reviewed: 5
P1 blockers: 0 | P2 issues: 0 | P3 notes: 17

**Verdict: APPROVED**

**Top finding:** IceLines now owns reusable hockey story and play causality while
REEL remains the generic timing, rendering, lineage, and release-integrity
engine.

**Cross-role consensus:** SCOUT and TAPE agree that animation must preserve the
verified play rather than dramatize a different one; WIRE and BENCH agree that
unknown evidence stays explicit; CREST agrees that those constraints still
permit strong original visual grammar.

## Amend

No blocking amendment. Future non-blocking additions:

1. Add a CLI validator if blueprint instantiation becomes a public IceLines command.
2. Add a goalie-centered history variant only after a real project proves the
   current player-history grammar insufficient.
3. Add a square-delivery default only when a target platform requires it.

This simulated review does not establish hockey fact, rights clearance,
artistic approval, or authority to publish.
