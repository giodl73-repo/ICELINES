# AHL affiliate and development-pool projection

Status: implemented core constraint, 32-team affiliation authority,
UI-neutral projection, verified source caching, durable AHL snapshots, and a
reviewed provider-to-NHL identity workflow; professional-game and
historical-affiliation expansion remains.

## Purpose

IceLines evaluates an NHL organization rather than treating an NHL camp cut as
the end of the decision. Each NHL roster branch can feed an associated AHL
assignment scenario, where the player must clear every applicable transaction
gate and compete for an actual affiliate role.

The authoritative 2026-27 catalog covers all 32 organizations, including
Hartford Wolf Pack → New York Rangers and Coachella Valley Firebirds → Seattle
Kraken. The projection contract remains season-scoped for historical seasons.

`ahl_affiliation_catalog.v1` is a dated 32-team association document. The
2026-27 projection validates its NHL/AHL pair against that catalog, while a
historical season remains explicit so later archive catalogs can replace the
current mapping without rewriting projection logic.

## Development rule

The rule is season-scoped input, not a permanent constant. For the 2025-26
official AHL rule carried into the 2026-27 baseline:

- teams dress 18 skaters plus two goaltenders;
- at least 12 dressed skaters must be development players;
- a development player has 260 or fewer NHL, AHL, and European elite
  regular-season professional games at the start of the season;
- goaltenders do not count in the 18-skater calculation.

Source: <https://theahl.com/faq>, checked 2026-07-24.

IceLines therefore allows at most six veteran skaters in a dressed affiliate
lineup. Missing professional-game totals are a no-read and fail closed. AHL
rookie status, NHL waiver eligibility, age, and contract type remain separate
facts.

## Projection contract

`ahl_affiliate_projection.v1` contains:

- NHL and AHL team identity and season;
- roster-pool authority (`official_snapshot`, `preseason_projection`,
  `authored_scenario`, or `unspecified`/no-read), including date, sources, and
  methodology where applicable;
- dated rule authority;
- assigned organizational player pool;
- start-of-season professional-game totals;
- development/veteran classification;
- 12F/6D/2G dressed decision;
- four forward lines, three defense pairs, and goalie tandem;
- an explicitly labeled organizational prospect pool ranked by the supplied
  projection score, with optional recall-readiness and current line role;
- unused veteran capacity and veterans blocked by the development rule;
- waiver-required and not-assigned gates kept separate from the lineup choice.

Prospect status is not inferred from the AHL development rule. A 25-year-old
minor-league regular can satisfy the rule without being an organizational
prospect, while a high-end prospect can be outside the AHL assignment pool due
to college, junior, European, contract, or roster constraints.

## Organizational simulation flow

1. The Cut selects an NHL opening-roster branch.
2. The Bubble identifies available players outside that branch.
3. Contract protection, waivers, consent, injuries, and league-assignment
   rights determine which players can actually reach the affiliate.
4. The affiliate projection combines those assignments with AHL-contracted and
   incumbent affiliate players.
5. The AHL development rule constrains the dressed lineup.
6. The resulting AHL role and line quality feed prospect development,
   recall-readiness, and NHL injury-replacement simulations.

## Reviewed identity bridge

`ahl_identity_crosswalk.v1` binds an official roster snapshot and exact AHL
team to canonical NHL identity candidates with evidence URLs. Matching is
deterministic and review-oriented:

- exact normalized name plus birth date is proposed, never auto-approved;
- name-only matches remain lower-confidence proposals;
- ambiguity, birth-date conflict, and no-match are explicit states;
- all rows begin `pending`; and
- only a complete artifact whose rows are explicitly `reviewed` can feed the
  projection adapter.

An empty preseason roster can still produce a zero-coverage audit artifact,
but it cannot be certified or joined into a projection input.

Identity facts remain separate from scenario facts. A reviewed identity does
not prove AHL assignment, prospect status, professional-game totals, waivers,
projection score, or recall readiness. `affiliate-input` joins those separate
facts only after exact snapshot coverage and evidence validation.

Official discovery is a review-queue accelerator, not an approval mechanism.
For each AHL roster name, IceLines acquires the official NHL player-search
response through FLETCH, retains only exact normalized-name results, and then
acquires the corresponding official NHL player landing document. Landing
player ID/name conflicts fail closed; a valid landing birth date upgrades the
proposal to exact-name-and-birth-date evidence. Identity normalization is
scoped to this bridge: diacritics are folded and
hyphens become word boundaries while apostrophes and periods are ignored, so
provider punctuation variants compare equally. Birth dates and canonical IDs
remain separate gates; the global IceLines name-search normalizer is unchanged.
The rule is comparison-only; established official search queries and FLETCH
dataset keys remain unchanged. Discovery additionally acquires a
straight-apostrophe query variant for curly-apostrophe provider names because
the official search index can distinguish the forms.
All discovered rows remain
`pending` until explicitly reviewed. An authored catalog may be merged by NHL
player ID; conflicting names or birth dates are rejected.
Non-refresh discovery reads hash-verified search and landing cachelines without
revalidating every source, then acquires only missing objects. `--refresh`
retains the explicit full-revalidation path. League acquisition checkpoints
bounded search and landing batches into the shared manifest, so an interrupted
run resumes from completed chunks rather than restarting a league-sized batch.

`affiliate-identities-league` scales that acquisition boundary to a complete
season snapshot. It deduplicates official search by normalized AHL roster name
across teams and player landing acquisition by NHL ID, then emits
`ahl_identity_league_crosswalk.v1` with one unchanged review queue per club.
When distinct roster-name searches return the same NHL player, their compatible
candidate evidence is merged by NHL ID before the strict canonical catalog is
validated; conflicting names or birth dates still fail closed.
The envelope distinguishes roster appearances from unique AHL provider IDs and
does not introduce league-level approval authority.

Approval uses a separate `ahl_identity_review_decisions.v1` authority rather
than editing generated rows in place. IceLines can draft `accept_proposal`
decisions for exact name-and-birth proposals, but the draft is non-applicable
until a reviewer inspects the evidence, sets `draft=false`, and supplies their
identity and an RFC3339 review timestamp. Final application supports accepting
the retained proposal, setting a different evidence-linked NHL identity for an
alias/remap, or rejecting the proposal. The batch is bound to the exact
season/provider/team/roster fetch; partial batches leave untouched rows
unchanged, and duplicate provider or resulting NHL identities fail closed.
Applied rows preserve reviewer, timestamp, action, note, and conflicting source
dates in the resulting crosswalk.

League exception drafting preserves that boundary. The
`ahl_identity_league_review_draft.v1` envelope contains only `draft=true`
per-team batches, counts pending rows that have no proposal, and never mutates
the source league crosswalk. Birth-date conflicts can be included for explicit
inspection; unmatched and ambiguous identities require new evidence or a
separate evidence-backed rejection.

`affiliate-review-exact` is the narrow bulk-review surface for an explicit
reviewer. It creates an applicable, timestamp-bound decision batch only for
pending `exact_name_and_birth_date` rows after rechecking normalized names,
equal provider/NHL birth dates, canonical NHL IDs, and retained absolute source
URLs. It then applies that batch through the same canonical decision function.
Alias, conflict, ambiguous, unmatched, rejected, and already-reviewed rows are
never included. The optional decision output preserves the exact applied
authority for audit and replay.

`affiliate-review-aliases` provides the equally narrow sourced-alias batch: it
requires a distinct full name, equal normalized surname and birth date,
canonical NHL ID, retained absolute evidence, reviewer, and timestamp, and
applies the result as an explicit identity override. `affiliate-review-reject`
handles selected pending exceptions without conflating rejection of an NHL
mapping with rejection of the AHL person. Its required note carries the
evidence-backed AHL-only, non-player, or other exclusion rationale into the
crosswalk, while repeatable absolute evidence URLs remain structured on the
rejected row for every renderer and downstream audit.

`affiliate-review-reject-league` is the atomic league counterpart. It selects
unique provider IDs, closes every pending team occurrence of each selected ID,
and emits the same team-bound decision batches inside a league audit. A missing
or non-pending selected ID fails the entire transformation. Rejection remains
mapping-only: the official AHL player, club appearance, and season facts are
preserved, so later canonical evidence can support a new reviewed remap rather
than reconstructing deleted source data.

Birth-date conflicts use a separate targeted league authority rather than the
routine exact or alias lanes. A reviewer selects proposed NHL IDs and supplies
new absolute evidence plus a timestamped rationale. IceLines rechecks that
every selected row is pending with `birth_date_conflict`, copies the proposed
canonical ID/name/date into an explicit `set_identity` decision, unions the
retained and new evidence, and records both provider dates in the note. Every
requested NHL ID must match at least one eligible team row or the entire league
operation fails without returning a partially reviewed envelope.
Generic `accept_proposal` decisions cannot apply birth-date conflicts, even if
an inspection draft is manually finalized.

The league exact, alias, targeted conflict, birth-date-correction, and
collision-remap commands apply atomically to the
league acquisition envelope. Their `ahl_identity_league_review_decisions.v1`
audit retains the original team-bound decision batches, reviewer, timestamp,
eligible-team count, skipped teams, and total applied decisions. A bad child
fails the whole transformation; a team with no eligible rows is a recorded
skip rather than an error.

Birth-date correction is also distinct from accepting the NHL source date.
`affiliate-review-birth-date-league` preserves the proposed NHL identity but
replaces its canonical birth date only when the normalized AHL/NHL names are
exact, the supplied canonical date equals the AHL date and differs from the NHL
proposal, and novel absolute evidence plus reviewer authority are present. The
displaced NHL date remains in the decision note and a failed child leaves the
league envelope unchanged.

Collision remediation is distinct from accepting a disputed source date or
rejecting an AHL person. `affiliate-review-collision-league` replaces one
same-name NHL proposal across all affected team-season rows only when the
displaced proposal is at least 1,460 days from the AHL birth date, the supplied
canonical identity has the same surname and exact AHL birth date, and novel
absolute evidence plus reviewer authority are present. Every decision retains
the displaced identity, both dates, date delta, canonical identity, and unioned
evidence. A validation failure leaves the league envelope unchanged.

`affiliate-review-league` owns the cross-team expansion view. Its
`ahl_identity_league_review.v1` contract aggregates snapshot-bound crosswalks,
recomputes team and league coverage, flags stale declared counts, and groups
every pending or rejected appearance into a deterministic exception queue.
Resolved basis points include explicit mapping rejections; canonical-identity
basis points count only reviewed NHL mappings. The aggregate is read-only and
cannot turn coverage reporting into review authority.

`affiliate-review-board` is the UI-neutral triage projection over that queue.
Each row carries a deterministic priority score, recommended action,
occurrence count, distinct seasons and teams, retained evidence, and structured
conflicting date pairs. Routine exact and alias wins receive the highest base
scores, with collision-scale date conflicts receiving a dedicated
investigation action ahead of ordinary birth conflicts, ambiguity, missing
canonical evidence, and rejected-mapping audits. An absolute date delta of at
least 1,460 days is collision-scale. Recurrence adds bounded appearance,
season, and team leverage; a conflict with a canonical ID, date pair, and two
retained sources receives an evidence-readiness bonus. The board never creates
review authority.

Preseason rollover binds two club identities when an affiliation changes:
`prior_ahl_team` selects the official historical roster and its reviewed
crosswalk, while `ahl_team` names the target-season affiliate. The prior field
is optional and defaults to the target for backward compatibility. Outputs
disclose both names. Historical players are never relabeled as members of a
new or relocated club merely to make the join succeed.

## Data work remaining

- continue resolving the three-season historical evidence queue; retain Conor McCollum as
  pending unless stronger date authority emerges;
- add dated historical NHL/AHL affiliation catalogs rather than applying the
  current association map to old seasons;
- build a sourced start-of-season professional-game ledger across NHL, AHL,
  and European elite leagues;
- model AHL contracts, NHL assignment rights, junior/college/European return
  restrictions, injuries, recalls, and paper transactions;
- aggregate affiliate results across NHL camp branches rather than only an
  explicit assignment scenario;
- learn affiliate coaching deployment, special teams, matchups, and line
  changes from game and shift evidence where available.

Official affiliation authority: <https://theahl.com/nhl-affiliations>.

Historical replay uses season-dated catalogs rather than applying the current
affiliate map retroactively. `examples/ahl-affiliations-2021-22.json` preserves
the official 2021-22 shared Charlotte affiliation as two relationship rows
(Florida and Seattle); consumers must not silently collapse shared custody to
one organization. Prospect cohort construction attributes organization from
the latest season in its frozen observation window, while retaining earlier
affiliations as provenance.


## Official AHL ingestion

`icelines fetch ahl` resolves `YYYYYYYY` to the matching regular season in the
official AHL HockeyTech feed, discovers that season's team catalog, and fetches
three separate shapes per selected team: the season roster, skater totals, and
goalie totals. The output is the UI-neutral `ahl_roster_stats.v1` contract.

The parser recognizes the feed's non-player goalie aggregate rows (`Empty Net`
and `Totals`) but rejects any other row missing provider identity. HockeyTech
can include a small number of conditioning-loan or other-team rows in a
team-filtered report and includes goalie scoring rows in its skater report.
IceLines excludes those rows from the typed team collections and retains an
auditable reason in `source_warnings`; separate goalie totals remain canonical.
A report containing only other-team players still fails closed. Duplicate
team IDs, conflicting duplicate player identities, malformed counting stats,
and skater point totals that do not equal goals plus assists also fail before
an atomic write. Compatible duplicate roster rows caused by in-season
jersey-number or forward-position changes collapse to one player, omit an
ambiguous jersey number, generalize conflicting forward sides to `F`, and
retain those decisions in `source_warnings`.
`provider_player_id` is explicitly AHL-scoped and is never treated as an NHL
`player_id`.

Team-filtered acquisitions receive a deterministic team-code suffix on their
snapshot name. This preserves the canonical full-league season snapshot when a
user later requests a smaller affiliate-only view on the same date.

Source bytes are acquired as season/team-specific FLETCH cachelines and entered
in the shared verified cache manifest under the registered
`icelines.ahl.<season>` adapter family. IceLines then parses those immutable
bytes, writes `ahl/ahl-roster-stats.json`, and seals a first-class `Ahl`
snapshot tier. The previous active snapshot is retained as its parent, so an
AHL side-fetch does not cut NHL roster/stat reads out of the active chain.
Full-league acquisitions fetch independent team cachelines in bounded batches
of six and commit their shared FLETCH manifest once. The returned verified byte
map is parsed directly; it is not discarded and reacquired once per report.
Team roster/skater/goalie assembly remains atomic, and the final snapshot is
sorted by team name before validation so concurrency cannot alter the artifact.

`affiliate_projection_input_from_snapshot` is the fail-closed bridge into the
core projection contract. Its enrichment set must exactly cover the selected
official roster and explicitly map every `provider_player_id` to a canonical
NHL player ID while supplying position, projection score, professional-games,
prospect, assignment, recall, and waiver facts. Missing or extra mappings,
duplicate identities, and current-affiliation disagreement are rejected.

Official surfaces: <https://theahl.com/stats/roster> and
<https://theahl.com/stats/player-stats>.

## Preseason pool authority

Before the official AHL roster publishes, IceLines may forecast from a
`preseason_projection` pool assembled from camp candidates, prior affiliate
incumbents, and sourced organization changes. That authority requires an as-of
date, at least one absolute source URL, and a methodology note. Its disclosure
must state that it is not an official AHL roster.

An `official_snapshot` pool is assigned only by the reviewed snapshot adapter.
An `authored_scenario` requires an explicit scenario note. Older documents
without authority metadata deserialize as `unspecified` and render as no-read;
they are never silently promoted to official evidence.
