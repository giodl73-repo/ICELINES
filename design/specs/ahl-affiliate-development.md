# AHL affiliate and development-pool projection

Status: implemented core constraint, 32-team affiliation authority,
UI-neutral projection, verified source caching, and durable AHL snapshots;
professional-game and historical-affiliation expansion remains.

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

## Data work remaining

- populate reviewed provider-to-NHL identity crosswalks and projection
  enrichments for official affiliate rosters;
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
