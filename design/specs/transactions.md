# Transactions — Specification

**Version**: 0.2 (post-review)
**Date**: 2026-04-30
**Status**: Draft — reviewed by WIRE / TAPE / EDGE / GLASS, ready for plan
**Replaces**: nothing (new feature)
**Related plans**: design/plans/INDEX.md (will add `2026-04-30-phaseT-transactions.md`)

---

## Purpose

A unified, league-wide feed of NHL roster moves — trades, waivers (placements,
clears, claims), signings, IR placements, callups/recalls, reassignments —
surfaced as:

- A new **TUI tab "Transactions"** with chronological feed, team filter,
  kind filter, and date scoping.
- A `icelines transactions` **CLI command** with the same flags as other
  reports (`--csv`, `--json`, `--out`, `--team`, `--since`, `--kind`).
- A bundled per-season snapshot (`transactions-{season}.json`) so historical
  views work offline, parallel to `bios.json` / `goalie-stats.json`.

Today: no transactions surface exists. Users have to switch to NHL.com,
Daily Faceoff, Pro Hockey Rumors, etc. to see who moved.

---

## Data source — confirmed and decided

The NHL public API has **no transactions endpoint** (verified
2026-04-30: every plausible URL returns 404; `api.nhle.com/stats/rest/en/config`
authoritatively lists every available report and includes none for
transactions / waivers / signings).

**Decision: use ESPN's site.api as the primary source.**

| Source | URL | Shape | Auth | Cadence | Verdict |
|--------|-----|-------|------|---------|---------|
| **ESPN** (primary) | `site.api.espn.com/apis/site/v2/sports/hockey/nhl/transactions` | Clean JSON: `{transactions: [{date, description, team{id, abbreviation, displayName, logos}}]}` | None | Continuous; ~750/season | Use this |
| **Pro Hockey Rumors** (enrichment, deferred) | `prohockeyrumors.com/waivers/feed` | RSS 2.0, article-shaped | None | Hourly | Defer to v2 |
| NHL official | various | n/a — 404 | n/a | n/a | Ruled out |
| PuckPedia | private | Paid only | API key | n/a | Ruled out |
| CapWages | docs API | No transactions endpoint | None | n/a | Ruled out for this feature |

ESPN's endpoint is **undocumented but stable** — the same shape ships in
ESPN's iOS / Android apps. Treat it as a reverse-engineered public source:
attribute "Data: ESPN" in the TUI title bar and CLI `--help`. Do not
redistribute the raw JSON — we re-shape it before persistence.

### Failure modes (WIRE)

ESPN has no SLA. The fetcher must enumerate and handle:

| Failure | Detection | Action |
|---------|-----------|--------|
| HTTP 429 (rate limit) | status code | Honor `Retry-After`; otherwise exponential backoff with jitter; max 3 retries |
| HTTP 5xx | status code | Same retry policy; surface "ESPN unavailable", keep last-known snapshot |
| HTTP 200 with empty `transactions: []` | array length == 0 | If a non-empty snapshot exists for this season, **refuse to overwrite** and exit non-zero |
| HTTP 200 with HTML body (Cloudflare challenge / endpoint removed) | `Content-Type` ≠ `application/json` | Do not feed to serde; surface "ESPN response shape changed", keep last-known snapshot |
| Schema drift (unknown fields) | serde error | Capture via `serde_json::Value`, log WARN with field names, continue with best-effort row (see "Schema validation" below) |
| Three consecutive non-200s in one run | counter | Circuit-break: abort fetch, keep existing snapshot |
| Missing-team payload | `team: None` | Bucket under synthetic `LEAGUE` team; document `--team LEAGUE` filter |

### Source abstraction

Even though we ship one source in v1, the fetcher lives behind a trait so a
paid PuckPedia, an in-house feed, or a swap to a different free source is a
one-file change:

```rust
// icelines-fetch/src/transactions/mod.rs
pub trait TransactionSource {
    async fn fetch_season(&self, season: &str) -> Result<FetchOutcome, FetchError>;
}

/// Rich return type so callers can react to partial / degraded fetches
/// (WIRE: never paper over reliability).
pub struct FetchOutcome {
    pub rows: Vec<RawTransaction>,
    /// Rows we received but couldn't fully parse (unknown fields captured
    /// via serde_json::Value fallback; row still extracted best-effort).
    pub dropped_unknown_schema: usize,
    /// True when the source signaled partial data (e.g. circuit-break
    /// triggered before completion). Caller must NOT overwrite a richer
    /// snapshot with a partial one.
    pub partial: bool,
    /// ETag / Last-Modified for conditional re-fetch when supported.
    pub source_etag: Option<String>,
    /// Wall-clock at fetch time. Persisted in the snapshot as `fetched_at`.
    pub fetched_at: String,
}

pub struct EspnSource { client: reqwest::Client, base: String }
impl TransactionSource for EspnSource { /* ... */ }
```

Aggregator dedup tie-break (when two sources disagree on `kind` for the
same `(date, team, description)`): primary source wins.

---

## Data model

### `icelines-fetch::schema::RawTransaction`

The JSON we receive from ESPN, parsed with the **WIRE-compliant** policy:
**`deny_unknown_fields` is ON**, but on unknown-field error the fetcher
falls back to a `serde_json::Value` capture and best-effort extraction
of known keys. This gives loud-on-change without losing the run.

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawTransaction {
    /// Raw timestamp from ESPN — typically date-only, sometimes ISO 8601.
    pub date:        String,
    /// Free-form English description. Always populated.
    pub description: String,
    /// Team that initiated / received the move. Some rows lack a team.
    #[serde(default)]
    pub team:        Option<RawTransactionTeam>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawTransactionTeam {
    pub id:           String,
    pub abbreviation: String,
    pub display_name: String,
}
```

### `icelines-core::model::Transaction`

After classification + sanitization (the parsed, persisted shape):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Calendar date YYYY-MM-DD parsed from ESPN's timestamp, converted
    /// to America/New_York (NHL's operational TZ) before bucketing —
    /// avoids "midnight UTC" rows landing on the wrong day.
    pub date: String,
    /// Primary team in canonical NHL form (TBL not TB; SJS not SJ).
    /// `None` for league-wide rows; surfaced as synthetic `LEAGUE` bucket.
    pub team: Option<TeamAbbr>,
    /// Classified kind. `Other` is a real outcome — never bail when a
    /// description doesn't match a known pattern.
    pub kind: TransactionKind,
    /// Sanitized prose: control characters stripped, whitespace normalized.
    /// Kept verbatim otherwise so we can re-classify when we improve the
    /// regex set (see "Classifier versioning").
    pub description: String,
    /// Stable hash over (date, team, description) for dedup / idempotency.
    pub id: String,
    /// Trade-mirror grouping. Set when the row appears to be one side of
    /// a multi-team move (see "Trade grouping" below). UI collapses rows
    /// sharing a non-None group_id.
    pub trade_group_id: Option<String>,
    /// Classifier version that produced `kind`. On load, if this is less
    /// than `CURRENT_CLASSIFIER_VERSION`, re-run `classify()` against
    /// `description` so bundled snapshots don't fossilize stale classes.
    pub classifier_version: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionKind {
    Trade,
    WaiverPlacement,    // "Placed on waivers"
    WaiverClear,        // "Cleared waivers"
    WaiverClaim,        // "Claimed off waivers from X"
    Signing,            // "Signed F X to a Y-year, $Z contract"
    Recall,             // "Recalled F X from AHL"
    Reassignment,       // "Reassigned F X to AHL" (incl. "Returned", "Loaned")
    InjuryReserve,      // "Placed on IR" / "Activated from IR"
    Other,              // unknown — preserves the row
}
```

### Classifier (TAPE-revised)

Pure function in `icelines_core::transactions::classify`. Uses **anchored
regexes** with priority ordering, not naive substring contains. Each
fixture below is real ESPN prose and **must be tested** in L0:

| Pattern (regex, case-insensitive) | Kind | Fixture |
|---|---|---|
| `\bclaimed off waivers\b` | WaiverClaim | "Claimed F X off waivers from BOS" |
| `\bcleared waivers\b` | WaiverClear | "Cleared waivers; assigned to AHL" |
| `\bplaced .* on waivers\b` | WaiverPlacement | "Placed F X on waivers" |
| `\bacquired (the rights to|negotiating rights)\b` | Other | "Acquired the rights to RFA F X" — NOT a roster trade |
| `\b(traded|acquired)\b` | Trade | "Acquired D X from NYR in exchange for…" |
| `\bsigned .* to a (PTO|professional tryout)\b` | Other | "Signed F X to a PTO" — NOT a roster signing |
| `\b(signed|re-signed)\b` | Signing | "Signed F X to a 2-year, $5M extension" |
| `\bemergency recall(ed)?\b` or `\brecalled\b` | Recall | "Recalled F X from Bakersfield (AHL)" |
| `\b(reassigned|loaned to (the )?AHL|sent down|optioned|returned to)\b.*\b(AHL|junior|college)\b` | Reassignment | "Returned F X to Bakersfield" |
| `\bloaned .* (for|to) (IIHF|World)` | Other | International loan — not an AHL move |
| `\b(placed on|activated from) (injured reserve|IR\b|LTIR)\b` | InjuryReserve | "Placed F X on LTIR retroactive to…" |
| _no match_ | Other | catchall; observability metric must alert if `>5%` |

Order matters: `Other`-promoting rules (PTO, rights-only, international
loan) check **before** the broader Signing/Trade/Reassignment rules so a
PTO doesn't get classified as Signing.

#### Classifier versioning

`pub const CURRENT_CLASSIFIER_VERSION: u16 = 1;` lives in
`icelines_core::transactions`. Bump on any change to the regex set.
`load_transactions_with_fallback()` re-runs `classify()` on any row
whose `classifier_version < CURRENT_CLASSIFIER_VERSION` so bundled
snapshots and live data never disagree on `kind`.

#### Observability

After every fetch, log:
```
classified: 700 rows
  trade: 42 · signing: 180 · recall: 220 · reassignment: 195 · ir: 51
  waiver_placement: 8 · waiver_clear: 4 · waiver_claim: 0 · other: 0
```
CI fixture asserts `other_rate < 5%`. If ESPN swaps "Acquired" → "Obtained"
and 100% of trades collapse to `Other`, the test fires.

### Team mapping (TAPE+EDGE+WIRE-revised)

ESPN abbrevs sometimes differ from NHL API form. Resolved via
**season-aware** `espn_to_nhl_abbrev(abbrev: &str, season: &str) -> Option<TeamAbbr>`:

1. **Static map**: known divergences. `"TB" → "TBL"`, `"SJ" → "SJS"`,
   plus historical `"PHX"` and `"ARI"` → themselves for pre-2024-25
   seasons (Coyotes existed), `"ATL"` for pre-2011 Thrashers, `"MOJ"`
   (Mighty Ducks legacy abbrev) → `"ANA"`.
2. **Season boundary**: `"ARI"` at season `>= 20242025` → `"UTA"` (relocation);
   at `< 20242025` → `"ARI"` preserved verbatim.
3. **Whitelist passthrough**: only if `abbrev ∈ ALL_NHL_TEAMS` AND was
   recognized as canonical for this season.
4. **Else**: return `None` AND emit `WARN: unmapped ESPN abbrev '{abbrev}'
   for season {season}`. Surface row as teamless (`LEAGUE` bucket).

L1 test: assert mapping is exhaustive against a fixture of every distinct
ESPN abbrev seen in the bundled seasons.

### Trade grouping (TAPE+EDGE)

ESPN reports trades **twice** — once per team, with prose and team flipped:
- `(2026-04-29, "TBL", "Acquired D Ryan McDonagh from NSH for…")`
- `(2026-04-29, "NSH", "Traded D Ryan McDonagh to TBL for…")`

Different `(date, team, description)` → different dedup hash → both rows
correctly persisted. UI groups them via `trade_group_id`:

```rust
fn trade_group_id(date: &str, players: &[NormalizedName], teams: &[TeamAbbr]) -> String {
    // sorted to make the id permutation-invariant
    let mut t = teams.to_vec(); t.sort();
    let mut p = players.to_vec(); p.sort();
    sha256(format!("{date}|{}|{}", t.join(","), p.join(",")))
}
```

`players` extracted from descriptions via the player linker (next section).
Set only when at least one player AND ≥2 distinct teams are detected
across the date-window. TUI collapses grouped rows under the highest-team
row with `(+1 mirror)` suffix; CLI emits all rows separately (the data is
the data).

### Player linking (TAPE+EDGE-revised)

Used by trade-grouping AND by the "Enter on row → open player card" TUI
action. Pure function `link_players(description, team) -> Vec<&Player>`:

1. Tokenize after the position prefix. Regex:
   `(?i)\b(F|D|G|LW|RW|C)?\s+([A-ZÀ-ÿ][\w'\-̀-ͯ]+(?:\s+[A-ZÀ-ÿ][\w'\-̀-ͯ]+)+)`
2. NFD-normalize both the captured name and the candidate `Player.full_name`
   (strip combining marks → "Hörnqvist" matches "Hornqvist").
3. Strip suffixes (`Jr.`, `Sr.`, `III`) and punctuation (apostrophes,
   periods).
4. Apply alias table: `Mike↔Michael`, `Tom↔Thomas`, `Alex↔Alexander/Aleksander`,
   `Mat↔Matthew`, `Sam↔Samuel`, `Nick↔Nicholas`, `JT↔J.T.`,
   `Kirill↔Kiril`, `Evgeny↔Evgeni`. Lives in
   `icelines_core::name::aliases`.
5. Score candidate Players by Levenshtein on (first_name, last_name);
   require ≥0.85.
6. **Disambiguate by team**: when the row has a non-None team, reject any
   candidate whose `Player.team != row.team`. Resolves the Sebastian-Aho
   collision.
7. Below threshold OR team mismatch → no link. **Never wrong-link.**

L0 fixtures must include: D'Pinto, Hörnqvist, Van Riemsdyk, MacKinnon,
de Haan, St. Louis, J.T. Miller, Tom (vs Thomas) Wilson, Mike (vs Michael)
Matheson, Alex (vs Alexander) Ovechkin, both Sebastian Ahos.

### Description sanitization (EDGE)

Before persisting, every `description` runs through:
```rust
fn sanitize(s: &str) -> String {
    s.chars()
     .filter(|c| !c.is_control() || matches!(c, '\t' | '\n')) // keep no whitespace controls
     .filter(|c| *c != '\t' && *c != '\n')                     // strip those too
     .collect::<String>()
     .split_whitespace()
     .collect::<Vec<_>>()
     .join(" ")
}
```
Prevents ratatui from breaking layout on stray `\n` / `\t`.

### Filter-flag validation (EDGE)

CLI must reject malformed input with helpful errors:
- `--kind unknown` → `error: unknown kind 'unknown'. valid: trade, waiver, signing, recall, reassignment, ir, other`
- `--since 2026-13-40` → `error: --since is not a valid date (YYYY-MM-DD): 2026-13-40`
- `--since 2026-04-30 --until 2026-04-01` → `error: --since is after --until`

L2 tests assert each error path.

---

## Storage

### Persistence layout

Snapshot file per season includes provenance:

```json
{
  "season":              "20252026",
  "source":              "espn",
  "fetched_at":          "2026-04-30T14:32:11-04:00",
  "classifier_version":  1,
  "rows":                [ /* Vec<Transaction> */ ]
}
```

`fetched_at` exposes staleness in the TUI footer (red if `> 7 days` mid-season).

### Cache + snapshot are different (WIRE)

Cache-First Protocol applies. We persist BOTH:

- **Cache**: `~/.icelines/cache/transactions/{season}_{fetched_at}.json` —
  raw ESPN response, written **before** classification. Lets us re-classify
  without re-fetching when we improve the regex set.
- **Snapshot**: `~/.icelines/snapshots/{season}/transactions.json` —
  classified, sanitized, ready for UI. Written **after** classification.
  Atomic-rename pattern: write to `transactions.json.tmp` then `rename()`.
  If load fails, try `transactions.json.bak` (previous successful write).

### Bundled per-season

Same pattern as goalies. Each bundled season directory gets a new file:

```
data/seasons/20252026/
├── bios.json
├── stats.json
├── goalie-stats.json
└── transactions.json     ← new (with the provenance envelope above)
```

Embedded via `include_bytes!` in `icelines-fetch::bundled`.
`load_transactions_with_fallback()` mirrors `load_goalies_with_fallback()`:

1. Try chunked snapshot tier (Phase 8h) — not used initially
2. Try legacy snapshot file
3. Fall back to embedded bundle
4. Fall back to installed-season tarball
5. Else error with the standard "run `icelines fetch transactions`" hint

### Live fetch

`icelines fetch transactions` is added as a new sub-command (parallel to
`fetch goalies`):

1. Hits ESPN with `?limit=1000&season={season}`; paginates if `pageCount > 1`.
2. **Refuses to overwrite a non-empty snapshot with an empty result**
   (WIRE). Empty result + no prior snapshot is fine for pre-season.
3. Classifies + sanitizes + sets `trade_group_id`.
4. Atomic-rename write to snapshot (with `.bak` of the prior).
5. Logs the observability summary (per-kind counts + `other_rate`).

### `fetch all` integration (WIRE+EDGE-revised)

Wired into `icelines fetch all` as best-effort BUT with a **failure flag**:

- Failure logs WARN AND sets a `transactions_stale` bit in
  `~/.icelines/snapshots/{season}/_meta.json`.
- Next `icelines transactions` invocation reads `_meta.json` and prints
  `WARN: transactions snapshot is N days stale (last fetch failed)` until
  a successful run clears the flag.
- TUI title bar shows `Transactions · ESPN · as of 2 days ago` (red text
  if `> 7 days`).

This converts silent staleness into a visible UI contract.

### Historical availability (EDGE)

ESPN's archive likely doesn't reach back to 1995-96. T.2 must verify which
seasons return data; the result drives a constant:

```rust
pub const TRANSACTIONS_EARLIEST_SEASON: &str = "20182019"; // verified in T.2
```

When the user time-travels to a season `< TRANSACTIONS_EARLIEST_SEASON`,
both CLI and TUI surface:
> "Transactions data begins 2018-19. The {season_label} season is not
>  covered by ESPN's archive."

Not "run `icelines fetch transactions`" — that would 404.

---

## CLI surface

### `icelines transactions`

```
icelines transactions
icelines transactions --team EDM
icelines transactions --team LEAGUE                # league-wide / teamless rows
icelines transactions --since 2026-03-01 --kind trade
icelines transactions --csv --out trades.csv
icelines transactions --season 20242025
```

| Flag | Description |
|------|-------------|
| `--team ABBREV` | Filter by canonical NHL team abbrev. Use `LEAGUE` for teamless rows. |
| `--since YYYY-MM-DD` | Only show transactions on/after this date. |
| `--until YYYY-MM-DD` | Only show transactions on/before this date. Must be ≥ `--since`. |
| `--kind KIND` | Filter: trade, waiver, signing, recall, reassignment, ir, other. |
| `--season YYYYZZZZ` | Use a bundled / installed historical season. |
| `--csv` | Excel-friendly output. |
| `--json` | Object array. |
| `--out PATH` | Write to file. |
| `--top N` | Limit to first N rows (default: all). |
| `--no-group` | Disable trade-mirror collapsing in CSV/JSON (default: collapsed). |

Routes through `commands::output::Format` — gets CSV escaping, JSON typing,
and `--out` for free.

### `icelines x transactions`

Same data via the unified `x` shortcut, defaulting to CSV with the standard
`--team`, `--since`, `--kind` args.

---

## TUI surface (GLASS-revised)

### Tab placement — slot 7 (between Groups and Playoffs)

Tab order becomes:
`League · Stats · Goalies · Scores · Schedule · Groups · Transactions · Playoffs`

GLASS rationale: Transactions is a *news/people* surface, conceptually
adjacent to Groups (also "people"). Splitting the time cluster
(Scores/Schedule/Playoffs) was wrong; keep them adjacent. Number shortcut
becomes `7` (Transactions), `8` (Playoffs) — single-digit, no chord.

### Layout (revised — column order, color contract, detail pane)

```
┌ Transactions · ESPN · as of 2 hours ago · 472 in 25-26 ─── Esc:back ┐
│                                                                     │
│  TBL   ⇄ Trade       2026-04-29  Acquired D Ryan McDonagh from NS…  │
│  EDM   ↑ Recall      2026-04-29  Recalled F Vasily Podkolzin from…  │
│  CHI   $ Signing     2026-04-28  Signed F Connor Bedard to a 8-ye…  │
│  BOS   ↻ WaiverClear 2026-04-28  Cleared waivers; assigned to Pro…  │
│  FLA   ✚ IR          2026-04-27  Placed F Sam Reinhart on IR (low…  │
│  LEAGUE ◇ Other      2026-04-27  League-wide reassignment deadline…  │
│  ...                                                                │
│                                                                     │
│  /:filter  T:team  k:kind  d:date  Enter:detail  q:quit             │
└─────────────────────────────────────────────────────────────────────┘
```

GLASS-mandated changes from v0.1:
- **Column order**: TEAM (identity) → KIND (with glyph) → DATE → DESCRIPTION
  (ellipsis-truncated, never wrapped).
- **Glyph prefix on every kind** so color is supplementary, not primary
  (deuteranopia-safe + WCAG 1.4.1 compliant):
  `⇄ Trade  $ Signing  ↑ Recall  ↓ Reassign  ↻ WaiverClear  ⊘ WaiverPlace
  + WaiverClaim  ✚ IR  ◇ Other`.
- **Color the KIND token only** (not the whole row), bold. Defaults
  elsewhere. Drop Magenta (collapses with Cyan under protanopia); use
  Blue+Bold for WaiverClaim instead.
- **Title bar** carries provenance (`ESPN · as of {fetched_at}`); footer
  carries keybindings only.
- **`T` for team filter** (not lowercase `t`, which conflicts with the
  Schedule "today" key — muscle-memory clash).
- **`Enter` opens a right-side detail pane** showing full description,
  parsed player link (if any), classified kind with glyph, source URL,
  and trade-group siblings. `Esc` closes the pane.

### Empty state — rendered card, not single line

When no transactions are bundled OR the season is pre-`TRANSACTIONS_EARLIEST_SEASON`:

```
   ╭──────────────────────────────────────────────╮
   │   No transactions for 1995-96.               │
   │                                              │
   │   ⇄ Trade   $ Signing   ↑ Recall  ↓ Reassign │
   │   ✚ IR      ⊘ Waiver    ↻ Cleared + Claim    │
   │                                              │
   │   Coverage begins 2018-19.                   │
   ╰──────────────────────────────────────────────╯
```

Doubles as a legend so users learn the glyph set while waiting / browsing
historical seasons.

### Stale-data marker

When `_meta.json:transactions_stale == true` OR the snapshot's
`fetched_at` is `> 7 days` old during the active season:
- Title bar prefixed with `[STALE]` in red.
- Footer adds `r:refresh-from-disk` (re-reads the snapshot; no live fetch
  from inside the TUI).

---

## Tests

| Tier | Coverage |
|------|----------|
| **L0** | `classify()` against fixture descriptions for every kind (≥3 real ESPN strings per kind, including PTO/rights/international-loan/emergency-recall negative cases); ESPN→NHL season-aware abbrev mapper (incl. ARI 2023-24 vs UTA 2024-25 boundary); date parser (UTC midnight → ET boundary); dedup hash stability; trade-group permutation invariance; player-link aliases (Mike↔Michael, Hörnqvist NFD, Sebastian Aho disambiguation); description sanitization (control chars stripped); description sanitization (newlines stripped); classifier versioning re-run. |
| **L1** | Deserialize a captured ESPN response fixture (`tests/fixtures/espn_transactions.json`); `EspnSource::fetch_season` against `httpmock` (200/429/5xx/empty/HTML response paths); `load_transactions_with_fallback` chain; atomic-rename + `.bak` recovery on corrupted snapshot; circuit-breaker after 3 consecutive 5xx. |
| **L2** | `icelines transactions` exits 0; `--csv` emits header + ≥1 row; `--team EDM` filters; `--team LEAGUE` returns teamless rows; `--kind trade` filters; `--out` writes file; `--since 2026-13-40` exits non-zero with helpful error; `--since 2026-04-30 --until 2026-04-01` exits non-zero; `icelines x transactions` defaults to CSV. |
| **L1 (TUI)** | Transactions tab renders correctly when bundle empty / loaded / filtered; stale marker appears when `fetched_at > 7 days`; legend card renders for pre-2018 season. |

Same "no live network in tests" rule the rest of the codebase follows.

---

## Phasing

Sub-phases (each independently shippable):

- **T.1** — Schema + classifier + alias table + abbrev map (icelines-core +
  icelines-fetch); no network. L0 tests pass against fixture descriptions.
- **T.2** — `EspnSource` fetcher + L1 mock test + verify historical
  coverage (set `TRANSACTIONS_EARLIEST_SEASON`). Standalone, no CLI yet.
- **T.3** — `icelines fetch transactions` CLI command + atomic-rename
  snapshot writer + provenance envelope + `_meta.json` stale-flag.
  Bundle one season's transactions.json into `data/seasons/20252026/`.
- **T.4** — `icelines transactions` CLI command + filter validation +
  L2 tests + `x transactions` shortcut.
- **T.5** — TUI Transactions tab + glyph color contract + filter/date
  controls + detail pane + empty-state legend card + stale marker.
- **T.6** — Backfill bundled transactions for 24-25, 23-24, 22-23, 21-22
  (provided ESPN's archive covers those seasons).

T.1–T.4 ship as the v0.11.0 cut. T.5 + T.6 follow up.

---

## Non-goals

- **Real-time push**: no websocket, no polling refresh inside the TUI. The
  `r` refresh key re-reads the snapshot. A future `--watch` flag could
  poll ESPN every N minutes; not in scope.
- **Cap impact**: no integration with contract data. Trades and signings
  show prose, not cap deltas.
- **Notifications**: no toast / system notification when a new transaction
  involves a watched player. Possible follow-up tied to the existing
  groups feature.
- **Editing / hiding**: read-only view. No "mark as read" or "mute team"
  state.
- **Multi-source aggregation in v1**: PHR RSS as a second source defers
  to a v2.

---

## Resolved review questions

| Q | Resolution |
|---|------------|
| `deny_unknown_fields`? | **ON** — but with `serde_json::Value` capture-and-log fallback, not silent drop. (WIRE) |
| Cache vs snapshot? | **Both.** Cache pre-classification, snapshot post-classification. (WIRE) |
| Trade dedup safety? | Keep both mirror rows; collapse in UI via `trade_group_id`. (TAPE+EDGE) |
| Team-map silent passthrough? | **Whitelist only**; otherwise `None` + WARN. (WIRE) |
| Time-travel pre-coverage? | Explicit `TRANSACTIONS_EARLIEST_SEASON` constant + dedicated empty-state copy. (EDGE) |
| ARI / UTA mid-season? | Season-aware abbrev mapper. (TAPE+EDGE) |
| Player-link disambiguation? | Pass team into `link_players`; reject team-mismatched candidates. (EDGE — Sebastian Aho) |
| Color encoding accessibility? | Glyph prefix on every kind; color the kind token only; drop Magenta. (GLASS) |
| `fetch all` silent failure? | `_meta.json:transactions_stale` flag surfaced in CLI + TUI. (WIRE+EDGE) |
| Classifier ossification? | `classifier_version` per row; re-run on load when stale. (TAPE) |
