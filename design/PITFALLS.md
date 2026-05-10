# IceLines Pitfalls Collection

Every way the IceLines system has tried to fail, organized by domain. The collection grows every
session. Pitfalls are never removed — when a pitfall is SOLVED, it stays here marked SOLVED with
the structural solution and test reference. The collection is institutional memory.

Pitfall domains:

- **DP** — Data Pitfalls: problems in player/team data, CSV parsing, or API data
- **AP** — Algorithm Pitfalls: problems in scoring, classification, or depth chart logic
- **PP** — Pipeline Pitfalls: problems in the fetch/cache/build execution chain
- **SP** — Site Pitfalls: problems in site generation or visual rendering

---

## Status Codes

- **OPEN** — known failure mode, no structural solution yet
- **MITIGATED** — we are careful about it, but a code change could reintroduce it
- **SOLVED** — structural solution exists AND a test proves it cannot happen

---

## Data Pitfalls

### DP-01 — Diacritic Mismatch: Slafkovský / Slafkovsky

**Description**: Juraj Slafkovský's name appears in Yahoo Fantasy CSV exports without the diacritic
('Slafkovsky') in some export formats, while the NHL API returns 'Slafkovský'. A naive string
equality join on `name` silently fails — the player is reported as unresolved rather than matched.
This is not unique to Slafkovský: Tomas Kämpf, Erik Björk, and others trigger the same problem.

**Status**: OPEN

**Known affected players (2023-24+)**: Juraj Slafkovský (MTL), Tomas Kämpf (CHI), Erik Björk
(multiple teams)

**Structural solution required**: `normalize_name()` in `icelines-core` must produce a canonical
lowercase ASCII form. The resolver must attempt: (1) exact match, (2) normalized match. The
normalized match must be verified unique — if two different players normalize to the same string
(Sebastian Aho case, DP-02), the match is ambiguous and must be rejected.

**Test required**: `test_slafkovsky_normalization()` — input "Slafkovsky", expected resolved ID = 8482078

---

### DP-02 — Name Collision After Normalization: The Sebastian Aho Problem

**Description**: In the 2019-20 NHL season, two active NHL players shared the name "Sebastian Aho"
— one on the Carolina Hurricanes, one on the NY Islanders. A name-based lookup returns two
player IDs. Any system that resolves player ID by name alone cannot distinguish them without
additional context (team, position). The normalized name is identical for both.

**Status**: OPEN

**Structural solution required**: When name normalization produces multiple API matches, the
resolver must use team context from the CSV to disambiguate. If disambiguation fails (wrong team
abbreviation in CSV, traded player), the pitfall escalates to DP-04.

**Test required**: `test_name_collision_disambiguation()` — two players with identical normalized
name in fixture data, resolver uses team column to return correct ID for each.

---

### DP-03 — Trade Deadline Split Rows

**Description**: After a mid-season trade, Yahoo Fantasy CSV exports sometimes include two rows
for the same player — one with stats accumulated for their old team, one for the new team. The
pipeline sees two `Player` records with the same `nhl_id` and different teams. If not detected,
the player is ranked twice (once per row) and appears on two teams' lineup cards.

**Status**: OPEN

**Structural solution required**: After player ID resolution, `csv_loader` must detect duplicate
`nhl_id` values. When a duplicate is found, merge the stats rows (sum points, sum goals, use the
most recent team) and produce a single `Player` record. Log the merge at INFO level.

**Test required**: `test_trade_split_deduplication()` — fixture CSV with two rows for same player
ID, assert output `Vec<Player>` has exactly one record with summed stats and current team.

---

### DP-04 — Stale Team Assignment After Trade

**Description**: A player is traded between the Yahoo CSV export and the current moment. The CSV
reflects their old team. The pipeline places them on the old team's lineup card. The NHL API may
confirm their new team, but only if we compare the API team to the CSV team — which the current
design does not do.

**Status**: OPEN

**Structural solution required**: After fetching player data from the NHL API, compare the API
`currentTeam.abbreviation` to the CSV `team` field. If they differ by more than 24 hours since
the CSV export (i.e., not a same-day export), emit a warning: "Player X was traded from OLD to
NEW — update your CSV or pass --refresh to use current team."

**Test required**: `test_stale_team_warning()` — mock API returns team "SEA" for player whose CSV
team is "PIT", assert warning is emitted.

---

### DP-05 — AHL Call-Up with Zero NHL GP

**Description**: A player in a fantasy pool may be on an NHL roster in the CSV (they were added
speculatively by a fantasy manager) but has played zero NHL games this season. The CSV may show
0 points and the NHL API may return GP=0. This player is not the same as an injured player
(who has 0 GP due to injury after starting the season) — the AHL call-up has never played.

In the current design, both cases produce `season_gp = Some(0)` and `pace_score = None`. The
lineup card correctly excludes them. But the `icelines rank` command should not silently drop
them — it should report them separately: "5 players below MIN_GP (including 0-GP players)".

**Status**: OPEN

**Structural solution required**: Distinguish `season_gp = Some(0)` from `season_gp = Some(n)`
where `1 ≤ n < MIN_GP`. Add a `GpStatus` enum: `Zero`, `BelowThreshold(u32)`, `Eligible(u32)`.
The rank command reports each category separately.

**Test required**: `test_gp_zero_vs_below_threshold()` — fixture with one GP=0 player and one
GP=5 player, assert rank output reports them in separate categories.

---

### DP-06 — CSV Column Name Drift

**Description**: Yahoo Fantasy has changed their CSV export column names in past seasons. A
CSV parser that accesses columns by index (column 0, column 3) fails silently on format changes.
A parser that accesses by name fails loudly — but only if the expected column names are documented
and validated.

**Status**: OPEN

**Structural solution required**: `csv_loader` must validate that all required columns are present
by name before processing any rows. Required columns: `['Name', 'Team', 'Pos', 'GP', 'PTS', 'G']`
(exact names to be confirmed against the current Yahoo export format). If any required column is
missing, return a `CsvParse(MissingColumn { name: "PTS" })` error before processing any rows.

**Test required**: `test_csv_missing_column()` — fixture CSV with "Points" instead of "PTS",
assert error is `MissingColumn { name: "PTS" }`.

---

## Algorithm Pitfalls

### AP-01 — Forward Threshold Applied to Defensemen

**Description**: The scoring engine's `classify_fit()` function takes a `Position` argument that
must be used to select the appropriate threshold set (forward vs. defense). If the position
argument is ignored, or if a defenseman's `Position::Defense` is not matched, the forward
thresholds are applied to defensemen. Devon Toews with a pace projection of 48 pts/82 is
classified as Solid (forward threshold: ≥40) instead of Elite (defense threshold: ≥45).

**Status**: OPEN

**Structural solution required**: The threshold selection in `classify_fit()` must be an exhaustive
match on `Position`. If a new position variant is added (e.g., a phantom `Utility` for future
features), the match must fail to compile unless the new variant is handled.

**Test required**: `test_classify_fit_defenseman_thresholds()` — a `Player` with `position =
Position::Defense` and `pace_82 = 46.0` must classify as Elite, not Solid.

---

### AP-02 — Tiebreaker Not Deterministic for Identical Players

**Description**: The `sort_by_rank()` function's tiebreaker chain is: pace_82 desc → goals_per_game
desc → name asc. If two players have the same pace_82, same goals_per_game, AND the same name
(impossible in real data but possible in test fixtures with synthetic names), the sort is
non-deterministic. A non-deterministic sort produces non-reproducible ranked output.

**Status**: OPEN

**Structural solution required**: Add a final tiebreaker: `nhl_id` ascending. NHL player IDs are
unique; this makes the sort total. The name tiebreaker alone is not sufficient in all possible
inputs.

**Test required**: `test_rank_sort_deterministic()` — sort the same Vec<Player> twice, assert
identical ordering both times. Use a fixture with two players sharing pace_82 and goals_per_game.

---

### AP-03 — Depth Chart Overflow: More Than 12 Forwards

**Description**: A team with many forwards in the fantasy pool (12+ skaters listed as C/LW/RW)
will overflow the depth chart's `[[Option<Player>; 3]; 4]` array. The builder must not panic or
silently drop players — it must place the top 12 by pace projection in the grid and move the rest
to `unplaced`.

**Status**: OPEN

**Structural solution required**: `DepthChartBuilder::build()` sorts eligible forwards by
`pace_82` descending before filling line slots. Slots are filled in order: Line 1 LW, Line 1 C,
Line 1 RW, Line 2 LW, ..., Line 4 RW. Any forward beyond position 12 in the sorted list goes to
`unplaced`.

**Test required**: `test_depth_chart_forward_overflow()` — fixture team with 15 forwards, assert
`forward_lines` is fully populated (12 players) and `unplaced.len() == 3`.

---

### AP-04 — MIN_GP Boundary: Exactly 10 GP

**Description**: The MIN_GP threshold is 10 games. A player with exactly 10 GP must be included
in ranking and fit classification. A player with 9 GP must not. An off-by-one in the comparison
operator (`< MIN_GP` vs. `<= MIN_GP`) silently excludes the 10-GP player or includes the 9-GP
player.

**Status**: OPEN

**Structural solution required**: The comparison in `compute_pace_score()` must be `gp < MIN_GP`,
not `gp <= MIN_GP`. MIN_GP is a constant (value: 10) in `icelines-core::scoring`. The comparison
is `if gp < MIN_GP_THRESHOLD { return None; }`.

**Test required**: Two tests — `test_min_gp_exactly_at_threshold()` asserts Some for GP=10;
`test_min_gp_below_threshold()` asserts None for GP=9.

---

### AP-05 - PoachScore Magic Number Without Reasons

**Description**: A fantasy-poacher board can look precise while hiding why a
player was recommended. If a row exposes only `PoachScore`, users cannot tell
whether the score came from measured category fit, estimated deployment,
schedule edge, stale availability, or a deferred component.

**Status**: VERIFIED (Selke core fixture tests)

**Structural solution required**: `PoachPlayerRow` must include explanation rows
for component impact, status, source, freshness, and risk. Renderers may
summarize these rows, but may not drop them from JSON/report output.

**Test coverage**: `poach_fixture_satisfies_contract_invariants` asserts every
fixture row has explanations, and
`poach_contract_fixture_serializes_context_score_and_explanations` asserts
component status/source/explanation fields survive JSON projection.

---

### AP-06 - Missing Deployment Data Treated As Negative Evidence

**Description**: Line, PP, PK, and shift data may be unavailable even when a
player is a valid add. Treating unknown deployment as "not promoted" creates
false negatives and makes the poacher punish data gaps.

**Status**: VERIFIED (Selke core fixture tests)

**Structural solution required**: `DeploymentSignal::Unknown` contributes no
negative score by itself. Estimated deployment must name the proxy used, such as
TOI trend, shot trend, goalie-start proxy, game-log usage, or manual watch note.

**Test coverage**: `unknown_deployment_and_availability_are_not_negative_evidence`
asserts unknown deployment and availability contribute zero negative score by
themselves.

---

### AP-07 - Schedule Overfitting

**Description**: A player with extra games can outrank a clearly better category
fit if schedule value is overweighted or not clamped. This turns a useful weekly
edge into noisy add/drop churn.

**Status**: OPEN

**Structural solution required**: `schedule_value` is clamped separately and
cannot exceed the spec weight without a same-commit spec update and fixture
change. Reports must show schedule edge beside category fit and risk.

**Test required**: Known-value fixture where a weak player with many games does
not outrank a stronger category-fit player solely due to schedule.

---

## Pipeline Pitfalls

### PP-01 — Partial Fetch Producing Silent Gaps

**Description**: If `icelines fetch` processes 480 players and the NHL API returns an error on
player 200, the current design (without cache-resume) would either abort entirely or continue and
silently exclude the failed player. The `icelines build` command would then produce lineup cards
with the failed player missing — no error, no warning, just a gap in the card.

**Status**: OPEN

**Structural solution required**: `icelines fetch` must track fetch status per player in a
per-run manifest file (`~/.icelines/cache/fetch_manifest_{date}.json`). If a player's fetch
fails, the manifest records `{ "player_id": X, "status": "failed", "reason": "HTTP 503" }`. The
`icelines build` command reads the manifest before building and reports all failed players at the
top of its output. It does not silently proceed with gaps.

**Test required**: `test_fetch_partial_failure()` — mock server returns 503 for one player ID,
assert manifest contains failed entry and build command reports the failure.

---

### PP-02 — Cache Entry from Wrong Season

**Description**: The cache key `{player_id}.json` is not season-specific. If IceLines is run for
the 2023-24 season and the 2024-25 season in the same calendar year, the cache for a player may
contain 2023-24 data when 2024-25 data is requested. The `--season` flag passed to `icelines fetch`
must be part of the cache key.

**Status**: OPEN

**Structural solution required**: Cache key format is `{player_id}_{season}.json`. The `season`
value is always required — `NhlApiClient::fetch_player_gp(player_id, season)` takes season as a
parameter and the cache key is derived from both values.

**Test required**: `test_cache_key_season_isolation()` — two cache writes for same player_id,
different seasons; read each; assert no cross-contamination.

---

### PP-03 — mkdocs Subprocess Failure Hiding Build Error

**Description**: `icelines build` invokes `mkdocs build` as a subprocess. If mkdocs fails (bad
template, missing CSS, markdown syntax error), the subprocess exits non-zero. If the CLI checks
`process.success()` but does not capture stderr, the user sees only "mkdocs build failed" with
no actionable detail.

**Status**: OPEN

**Structural solution required**: When the mkdocs subprocess exits non-zero, capture and display
its stderr output verbatim to the user. The error message from `icelines` must include: "mkdocs
build failed — see output above." Do not swallow subprocess stderr.

**Test required**: `test_mkdocs_failure_output()` — mock subprocess that exits 1 with a known
stderr string; assert the CLI output contains that string.

---

## Site Pitfalls

### SP-01 — CSS Class Name Mismatch Between Rust Enum and Template

**Description**: The `FitClass` enum variants in Rust (`Elite`, `Solid`, `Buried`, `Stretch`)
must map to CSS class names in the template (`fit-elite`, `fit-solid`, `fit-buried`, `fit-stretch`).
If the Rust-to-CSS mapping is a runtime string construction (e.g., `format!("fit-{}", name.to_lowercase())`),
a renamed enum variant will produce a wrong CSS class that applies no styling — silently.

**Status**: OPEN

**Structural solution required**: The CSS class name for each `FitClass` variant is a `const &str`
in a match expression in `icelines-site`. Adding a new `FitClass` variant causes a compile error
if the match is not updated. The CSS class name is never constructed from the variant name at runtime.

**Test required**: `test_fit_class_css_mapping()` — assert each `FitClass` variant maps to the
expected CSS class string (4 assertions, one per variant).

---

### SP-02 — Empty Team Page for Teams with No Fantasy Players

**Description**: If an NHL team (e.g., the Utah Hockey Club after a trade deadline) has no
players in the Yahoo Fantasy CSV, `icelines build` must still generate a page for that team.
The page should not be empty — it should display the team name, season, and an explicit message:
"No skaters from this team appear in the current fantasy pool."

An absent team page means a broken link in the generated site's navigation.

**Status**: OPEN

**Structural solution required**: The build command iterates over all 32 canonical team
abbreviations from `CANONICAL_TEAMS`, not over teams present in the CSV. For teams with zero
players, it renders the empty-state template. The site always has exactly 32 team pages.

**Test required**: `test_build_generates_all_32_teams()` — fixture CSV with players for only 5
teams; assert build produces 32 markdown files, 27 of which contain the empty-state message.

---

### SP-03 — Name Truncation Breaking Player Identity

**Description**: Long player names are truncated to 20 characters for display in the lineup card.
A 20-character truncation of "Alexandar Georgiev" (18 characters, fine) vs. "Alexis Lafrenière"
(17 characters with accent, fine) vs. "Pierre-Luc Dubois" (17 characters, fine) is safe. But
"Alexander Radulov" (17) and "Alexander Barabanov" (19) truncate safely. The risk is a
truncation that makes two different players appear identical in the card — e.g., two players
whose names share the first 20 characters.

**Status**: OPEN

**Structural solution required**: If two players on the same lineup card would produce identical
20-character truncated display names, append the first letter of their team abbreviation. This
collision is rare enough to handle at render time rather than at the data model level.

**Test required**: `test_name_truncation_collision()` — two players whose names share the first
20 characters; assert display names are distinct in the rendered output.
