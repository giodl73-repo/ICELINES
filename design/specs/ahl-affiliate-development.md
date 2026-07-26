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
proposal to exact-name-and-birth-date evidence. All discovered rows remain
`pending` until explicitly reviewed. An authored catalog may be merged by NHL
player ID; conflicting names or birth dates are rejected.

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

`affiliate-review-exact` is the narrow bulk-review surface for an explicit
reviewer. It creates an applicable, timestamp-bound decision batch only for
pending `exact_name_and_birth_date` rows after rechecking normalized names,
equal provider/NHL birth dates, canonical NHL IDs, and retained absolute source
URLs. It then applies that batch through the same canonical decision function.
Alias, conflict, ambiguous, unmatched, rejected, and already-reviewed rows are
never included. The optional decision output preserves the exact applied
authority for audit and replay.

## Data work remaining

- complete the manual exception review after exact-only Hartford and Coachella
  Valley historical pilot batches, then expand the same coverage league-wide;
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

## Official AHL ingestion

`icelines fetch ahl` resolves `YYYYYYYY` to the matching regular season in the
official AHL HockeyTech feed, discovers that season's team catalog, and fetches
three separate shapes per selected team: the season roster, skater totals, and
goalie totals. The output is the UI-neutral `ahl_roster_stats.v1` contract.

The parser recognizes the feed's non-player goalie aggregate rows (`Empty Net`
and `Totals`) but rejects any other row missing provider identity. Team-code
mismatches, duplicate team/player IDs, malformed counting stats, and skater
point totals that do not equal goals plus assists fail closed before an atomic
write. `provider_player_id` is explicitly AHL-scoped and is never treated as an
NHL `player_id`.

Team-filtered acquisitions receive a deterministic team-code suffix on their
snapshot name. This preserves the canonical full-league season snapshot when a
user later requests a smaller affiliate-only view on the same date.

Source bytes are acquired as season/team-specific FLETCH cachelines and entered
in the shared verified cache manifest under the registered
`icelines.ahl.<season>` adapter family. IceLines then parses those immutable
bytes, writes `ahl/ahl-roster-stats.json`, and seals a first-class `Ahl`
snapshot tier. The previous active snapshot is retained as its parent, so an
AHL side-fetch does not cut NHL roster/stat reads out of the active chain.

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
