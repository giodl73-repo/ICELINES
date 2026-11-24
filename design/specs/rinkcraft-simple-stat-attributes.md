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
Their six public-stat recipes and the routine refresh command are defined in
[Goalie stat estimates and routine refresh](rinkcraft-goalie-stat-attributes.md).

## Deeper skater records

The optional `skater_context` on an attribute profile supplies 25 readable
measurement rows. The six grades retain the recipes above; these observations
support testing better recipes without quietly changing the existing scale.

| Attribute | Additional record detail |
| --- | --- |
| Skating | Recorded maximum speed, speed bursts/60 and total distance |
| Handling | Giveaways/60, drawn penalties/60 and recorded-action loss share |
| Passing | Primary/secondary assists, primary assists/60, separate EV and PP assist rates |
| Shooting | Shot volume, goalie-in-net xG, goals minus xG, expected goals per attempt, average attempt distance and tracked maximum shot speed |
| Reading | Takeaways, penalties taken and net penalties per60 |
| Defending | Blocks, hits, PK time per appearance and defensive-zone exposure |

Primary/secondary credits come from complete selected-season official goal logs.
They are used only when their sum equals the player's NHL summary assist total;
otherwise they remain unavailable with an explanatory note. These are scoring
credits, not a count of completed passes. Even-strength assist rates use EV ice
time (including formats beyond 5v5); PP rates use PP time. No PP exposure gives
an unavailable rate, rather than a fictitious 0% success rate.

Shot-quality comparisons use MoneyPuck.com's own goals, xG and unblocked attempts
from the same archive, excluding empty nets, shootouts and playoffs. Average
distance includes unblocked misses. They never combine NHL goal totals with a
different MoneyPuck shot denominator. Missing attributed attempts stay unavailable.
The shot capture date is retained in the context note and raw hashes in the
refresh report. MoneyPuck.com is credited on the card and in the expanded record.

Reading and Defending remain limited public-event approximations. Anticipation
and isolated defensive effectiveness are not established by takeaways, blocks,
hits or deployment; the added records expose this distinction rather than adding
an unsupported ability bonus. No coaching assignment or simulation modifier is
inferred from the context.

`profile-refresh` produces this detail with its ordinary sources and optional
`--game-cache-dir DIRECTORY` for the complete official goal-log captures. The
existing `--edge-captures DIRECTORY` can recover retained raw EDGE observations
after the same actor/season/GP/source validation. Fresh matching cache data keeps
precedence. Omit the game cache to leave assist-type details unavailable.

The primary card adds **Attribute Record** in Distinctive Features. Each attribute
opens its own measurements; the full six-part record remains expandable. Existing
catalogs load without the optional field. Native and WASM retain the same strict
player/season/GP/position attachment, and the underlying math lives in Rust.

Math lives in `icelines-core::view_model::rinkcraft_stat_scores`. The native
catalog command is `rinkcraft_stat_score_profiles BASE NEW_DIRECTORY`; the WASM
binding is `stat_attribute_catalog`. Native templates and the shared browser
renderer accept the same score payload. No simulator bonuses are added here.

The projector also writes `rinkcraft-stat-attributes.json`, using
`rinkcraft.stat-attribute-catalog.v1`. Each estimate retains its observed GP and
minutes. Set `ICELINES_RINKCRAFT_STAT_ATTRIBUTES` to that file for the ordinary
native CLI and HTML profiles. For the browser, place it in
`icelines-browser/public/` before building; it is included in the offline shell.
The generated catalog remains a local data artifact, separate from source code.

Shared Rust attaches estimates only for the exact player, regular season, GP and
position family in the selected profile. A selected numerical snapshot supplies
its own GP; canonical position and requested season bounds still apply. Browser
requests stop at the active dataset's season. Estimates work without a numerical
dossier. Missing or invalid optional catalogs leave the other profile facts
usable; invalid supplied catalogs add an availability note. The WASM
`profile_stat_attributes` binding uses the same attachment function as native.

The existing history disclosure includes a newest-first six-attribute season
table, with GP, position peers and small-sample labels. Each score opens its own
season's formula without changing the primary card. Missing scores remain dashes.
These are comparisons with each season's peers, not a fixed-scale claim of skill
growth, and supplied windows are not presented as a verified complete career.

Shared Rust supplies each row's change from the previous supplied season. A
comparison requires the same actor, position group, scope, recipe and ranked
input set. Missing scores or incompatible inputs produce an unavailable delta;
an intervening supplied missing score is not skipped. The table names the
comparison season, so gaps are not presented as annual changes. Zero means an
unchanged estimate; it is distinct from an unavailable comparison. Changes are
estimate points, not measured changes in technical skill.

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
