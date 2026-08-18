---
skill: roles-check
topic: hockey-sprite-profile-cache-plan
date: 2026-08-17
roles_used: 6
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# IceLines roles check: hockey sprite profile and cache plan

## Artifact identification

- Type: hockey-domain profile and reusable film-blueprint proposal
- IceLines responsibility: hockey pose vocabulary, role/handedness/direction semantics, profile selectors, domain validation, and sanitized fixtures
- Explicit non-responsibility: bitmap generation/rendering, player-specific likeness art, team branding, customer casting, or local absolute cache paths
- Integration: emit a REEL-defined generic profile contract that Karts can bind to a REEL sprite library

## Roles selected

1. HART — correct identity and temporal axes for player/equipment/profile state.
2. KEEL — cross-repository boundaries and cache architecture.
3. SCOUT — hockey-valid pose vocabulary and action causality.
4. PACE — measured pose coverage, atlas size, and generation/render budgets.
5. BENCH — deterministic fixtures and failure-catching tests.
6. EDGE — missing, stale, ambiguous, and direction/identity failure modes.

## Review

### HART

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Hockey pose definitions are domain-stable, while handedness and identity are player-stable and team skin/number may be season- or event-stable. | P2 | Model axes | Model these as separate layers; do not collapse them into one player sprite record. |
| 2 | A cache key containing only `player_id` would be wrong for season/event-dependent uniform and number data. | P2 | Cache key | Key casting skins by the declared context axis while keeping stable identity layers separate. |
| 3 | The media profile should not mutate the canonical statistics repository merely to obtain animation metadata. | P3 | Model boundary | Read canonical identity fields through existing views and emit a separate media/profile document. |

### KEEL

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | IceLines should instantiate a generic REEL profile schema rather than invent a competing renderer contract. | P2 | Cross-repo contract | REEL defines mechanics; IceLines publishes hockey vocabulary using that contract. |
| 2 | The configured cache is a media-build persistence tier, not an NHL query fallback or a new StatsRepository tier. | P3 | Architecture | Keep sprite cache resolution out of normal data-query paths and document the asymmetry explicitly. |
| 3 | CLI, skill, and template consumers must resolve the same profile selectors to the same pose requirements. | P3 | Convergence | Centralize profile resolution in one IceLines media module/command rather than duplicating YAML logic. |

### SCOUT

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | The initial profile needs separate skater, goalie, bench, faceoff, contact, recovery, and celebration families. | P2 | Hockey vocabulary | Do not force goalie or off-ice actions into generic forward-skating poses. |
| 2 | Facing alone is insufficient: handedness, attacking direction, body phase, possession state, and puck/stick contact determine whether a pose makes hockey sense. | P2 | Selectors | Make those selectors explicit and validate impossible combinations. |
| 3 | A semantic action must describe anticipation, action/contact, follow-through, and recovery where relevant. | P3 | Choreography | Supply phase mappings so REEL receives intentional pose changes rather than isolated hockey nouns. |

### PACE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | “64 poses” and “512 atlas cells” are planning estimates, not established coverage requirements. | P2 | Quantitative claims | Begin with a measured production corpus and report uncovered selectors before selecting the library size. |
| 2 | On-demand generation reduces upfront cost but needs declared budgets for generation time, bytes, atlas dimensions, and render inputs. | P3 | Budget | Measure and record these values in coverage reports. |
| 3 | Derived direction/depth variants should not be counted as unique hockey actions when reporting semantic coverage. | P3 | Metrics | Report base actions, phases, and derived visual variants separately. |

### BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | A sanitized fixture must prove selector resolution without real player identities or Karts artwork. | P2 | Fixtures | Include generic skater, defender, and goalie cases with known expected pose IDs. |
| 2 | Tests must catch mirrored numbers, missing attachment anchors, ambiguous fallback, wrong handedness, and profile/library hash drift. | P3 | Regression coverage | Add L0 schema/selector tests and an L2 command fixture that emits a path-free packet. |
| 3 | Cache-hit tests must verify bytes and hashes; cache-miss tests must produce a regeneration request rather than silently substituting an asset. | P3 | Cache tests | Treat incomplete or corrupt packs as explicit states. |

### EDGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Direction changes during a play can invalidate one fixed facing for a performer. | P2 | Direction edge cases | Permit facing at pose/keyframe resolution, not only at character level. |
| 2 | Trades, temporary jersey numbers, emergency goalies, duplicate names, and missing handedness can all poison automatic casting. | P3 | Identity edge cases | Prefer player IDs, explicit event context, and declared unknown states; never guess through name-only matching. |
| 3 | Generator/model drift means regeneration may produce an equivalent but not exact result. | P3 | Reproducibility | Distinguish exact cached-output verification from recipe-based equivalent regeneration. |

## Synthesis

Roles reviewed: 6  
P1 blockers: 0 | P2 issues: 8 | P3 notes: 10

Verdict: **APPROVED-WITH-CONDITIONS**

Top finding: IceLines must model hockey selectors on the correct identity/time axes without turning the sprite cache into part of its statistics persistence architecture.

Cross-role consensus: The hockey profile should describe what pose is required and why; it must never own the rendered bitmap, customer likeness, or local cache location.

## Amendments

1. Specify the profile axes and selector vocabulary—role, action, phase, facing, handedness, possession/contact, and event context—before implementation.
2. Measure pose coverage from real animated-moment packets before committing to 64 base poses or 512 derived cells.
3. Add sanitized selector, cache-miss, hash-drift, and direction-change fixtures that emit REEL-compatible path-free results.
