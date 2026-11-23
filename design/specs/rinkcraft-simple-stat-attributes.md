# Simple stat attributes

Owner direction: use easy, feat-like stat scoring as the best approximation.
This replaces the all-facets-or-unrated requirement for the first usable skater
profile. The detailed research battery remains available separately.

| Attribute | Favorable stat percentiles and weights |
| --- | --- |
| Skating | 50% top speed + 50% 20+ mph bursts/60 |
| Handling | 50% lower giveaway share + 50% penalties drawn/60 |
| Passing | 100% assists/60 |
| Shooting | 50% goals/60 + 30% shots/60 + 20% goals/shots |
| Reading | 50% takeaways/60 + 30% assists/60 + 20% fewer penalties taken/60 |
| Defending | 50% blocks/60 + 30% takeaways/60 + 20% fewer penalties taken/60 |

Giveaway share = giveaways / (shots + assists + giveaways). This is a recorded
action proxy, not a measured touch or possession denominator. All-strength rates
use official ice time. Ranks compare the same season and Forwards/Defense group.
Lower-is-better stats use 100 minus their raw percentile. Ties use midranks.
Ranking rounds measurements to nine decimal places to prevent floating-point
unit-conversion noise from splitting ties; displayed source values are retained.

Average the favorable percentiles with the listed weights, divide by5, round,
and clamp to1–20. Fifty stat points gives10; ninety gives18. The weighted average
is a stat-point index, not the percentile rank of the composite itself.

Reference players have at least10 GP and100 minutes, with20 measured peers per
stat. Smaller player samples still receive scores and show **Small sample**.
Missing stats are omitted and the remaining weights scale to100%; no available
stats means no score. Genuine zero remains a measured value.

The compact sheet labels these **Estimates**. Clicking one opens its raw stats,
favorable percentiles, effective weights and formula. No research receipts are
needed in this flow. These are practical game/profile approximations, without
claims of technical skill measurement or inferred coaching assignments. Goalies
keep their separate attributes; these skater recipes are never applied to them.

Math lives in `icelines-core::view_model::rinkcraft_stat_scores`. The native
catalog command is `rinkcraft_stat_score_profiles BASE NEW_DIRECTORY`; the WASM
binding is `stat_attribute_catalog`. Native templates and the shared browser
renderer accept the same score payload. No simulator bonuses are added here.

`observed_player` joins official counts, deployment and optional EDGE observations
in shared Rust. Player, season, competition, group and GP must match. Per-60 rates
use the deployment's all-strength total, independently of the evidence card's
5v5 or other display scope. Conflicting counts, duplicate measurements and wrong
units are rejected; missing measurements remain missing.

The catalog command also accepts an optional third argument naming a retained
EDGE capture directory. It can recover valid observations using the original
capture date and exact player/window. Invalid shot-location data is withheld
without erasing independent valid skating observations; the EDGE notes explain
the partial coverage. No live fetch or cross-season substitution is performed.

Feat summaries show actual stat cutoffs (for example, takeaways >0.30/game),
with the player's own value below. Rust derives exact raw boundaries for the
existing Bronze/Silver/Gold rules, including ties and strict versus inclusive
operators. The awards do not change. Percentiles live in the optional cutoff
explanation instead of the main feat text.

Stat feats use a common 10 GP minimum for players and reference cohorts, for
skaters and goalies alike. At least 20 measured peers are required per stat.
There is no further 20-game confirmation gate; observed GP stays visible.

Estimated Record shows current-rate season totals with the per-game rate below.
The loaded2026–27 window uses84 games; the loaded earlier windows use82. A supplied
`max_possible_games` can replace that target, never below games already played.
For current single-team players, the preview estimates remaining team games from
the highest GP among that team's single-team players in the same snapshot. The
ceiling is player GP plus those remaining games. Multi-team players use standard
pace unless a limit is supplied. This is an estimated ceiling, not expected starts.
Ratios such as save percentage remain rates. The heading says **game pace**:
this is straight extrapolation, without an injury or lineup-selection model.
