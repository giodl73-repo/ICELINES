# IceLines System Invariants

Properties that must always hold. Violation of any invariant is a bug, not a design trade-off.

Invariants are grouped by domain:

- **DI** — Data Invariants: properties of player and team data at any point in the pipeline
- **AI** — Algorithm Invariants: properties of the scoring and classification engine
- **II** — Interface Invariants: properties of the CLI commands and their inputs/outputs
- **SI** — Site Invariants: properties of the generated mkdocs site

---

## Status Table

| ID    | Domain    | Invariant | Status | Notes |
|-------|-----------|-----------|--------|-------|
| DI-01 | Data | Every `Player` in the pipeline has a non-empty `name` and a `team` that is in the canonical 32-team abbreviation list. Players that fail this check are rejected at CSV load time, not silently passed through. | OPEN | `TeamAbbr::parse()` will enforce this; not yet implemented |
| DI-02 | Data | A `Player` with `season_gp = Some(0)` must have `pace_score = None` and `fit_class = None`. A zero-GP player is never assigned a pace projection, even if they have nonzero points in the CSV. | OPEN | Scoring engine not yet implemented |
| DI-03 | Data | A `Player` with `season_gp = Some(n)` where `n < MIN_GP` must have `pace_score = None` and `fit_class = None`. The MIN_GP gate is enforced in the scoring engine, not at the caller. | OPEN | |
| DI-04 | Data | A `Player` with `nhl_id = None` (name resolution failed) must never appear on a lineup card or in a ranked output. Unresolved players are collected into a separate error report at the end of the pipeline. | OPEN | |
| DI-05 | Data | The `name_normalized` field of every `Player` is the result of applying `normalize_name()` to `name`. These two fields are always in sync — `name_normalized` is never set independently. | OPEN | |
| DI-06 | Data | Two `Player` records in the same pipeline run must not share the same (`nhl_id`, `season`) pair. A mid-season trade that produces two CSV rows for the same player must be detected and merged before the scoring engine runs. | OPEN | EDGE DI-06 / Sebastian Aho pattern |
| AI-01 | Algorithm | For any `Player` with `fit_class = Some(FitClass::Elite)`, `pace_score.pace_82` is ≥ the Elite threshold for that player's position group. The fit classification and pace score are always consistent — they are produced by the same function call and stored together. | OPEN | |
| AI-02 | Algorithm | The `sort_by_rank()` output is deterministic: given the same input `Vec<Player>`, the output is always the same ordering. The tiebreaker chain (pace_82 desc → goals_per_game desc → name asc) must be total — no two players can be considered equal by all three criteria simultaneously (names are unique in a pool). | OPEN | |
| AI-03 | Algorithm | A `DepthChart` for any team has at most 4 forward lines (12 forward slots) and at most 3 defense pairs (6 defense slots). A roster with more than 12 forwards in the CSV will have the excess players in `unplaced`, not in an overlong `forward_lines` array. | OPEN | |
| AI-04 | Algorithm | The fit thresholds used in `classify_fit()` are the same values documented in `docs/specs/rust-cli.md`. If the spec changes the threshold values, the implementation must change in the same commit. These cannot diverge. | OPEN | |
| II-01 | Interface | `icelines team <TEAM>` exits with code 0 if and only if the team was found, GP data was available (cached or freshly fetched) for at least one player on the team, and the lineup card was rendered without error. All other cases exit non-zero. | OPEN | |
| II-02 | Interface | `icelines rank` with no `--position` filter includes all positions except goalies. A goalie in the CSV is parsed but never included in ranking output and never placed on a lineup card. | OPEN | |
| II-03 | Interface | `--no-color` is always respected. Any terminal output path that produces ANSI color codes must check the `--no-color` flag first. If stdout is not a TTY, `--no-color` is implied automatically. | OPEN | |
| SI-01 | Site | Every generated team markdown file contains exactly one forward grid section and one defense grid section. A team file with no skaters in the CSV generates a page with empty grid sections and an explicit "No skaters in fantasy pool" message — not a missing section. | OPEN | |
| SI-02 | Site | The CSS fit class applied to a player cell in the site (`fit-elite`, `fit-solid`, `fit-buried`, `fit-stretch`) matches the `FitClass` computed by the scoring engine for that player. The mapping from `FitClass` variant to CSS class name is a constant, not a runtime string construction. | OPEN | |

---

## Adding Invariants

When a new invariant is identified (by any role, in any session), add it to the table above:

1. Assign the next ID in the appropriate domain sequence
2. State the invariant as a property that is either true or false — not a goal or a guideline
3. Set status to **OPEN**
4. Add a test reference once a test enforcing the invariant exists
5. Set status to **VERIFIED** only when the test passes in CI

An invariant with no test is a promise. An invariant with a passing test is a guarantee.

## Status Codes

- **OPEN** — invariant is stated, no enforcement mechanism or test yet
- **ENFORCED** — structural enforcement exists (type system, validation at boundary) but no test
- **VERIFIED** — a test would fail if the invariant were violated, and the test passes
