# Phase Foster — Data Architecture (Foster.0 + Foster.4)

**Parent**: `foster-overview.md`
**Plan**: `design/plans/2026-05-06-phaseFoster-data.md`
**Status**: Spec — ready for implementation

---

## Goal

Replace today's *bundle / snapshot / `data install`* triad with one
cache, one manifest, one set of read paths. Bundle becomes the first
cache layer; everything else augments through the same `DataStore`
API. Sync is opt-in, capability-scoped, non-blocking.

## Core types

### `EntityRef` (icelines-core/src/entity.rs — new module)

Stringly-typed enum, single encoding everywhere — JSON envelopes,
SQLite columns, URL query params, CLI args. Mirrors `LeagueAbbrev`'s
freeform-string pattern proven in Calder.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityRef {
    Player(PlayerId),
    Team(TeamAbbr),
    Game(GameId),
    // Coach, Conference, Division — defer; add when consumers need them.
}

impl Display for EntityRef { /* "player:8478402" / "team:EDM" / "game:2025020001" */ }
impl FromStr for EntityRef {
    type Err = EntityRefError;
    /* parses ^(player|team|game):[A-Za-z0-9]+$ */
}

// Serde delegates to FromStr/Display so the wire form is always the string.
impl Serialize for EntityRef { /* via Display */ }
impl Deserialize for EntityRef { /* via FromStr */ }

#[derive(Debug, thiserror::Error)]
pub enum EntityRefError {
    #[error("malformed entity ref '{0}' — expected 'player:ID' / 'team:ABBR' / 'game:ID'")]
    Malformed(String),
    #[error("unknown entity kind '{0}' — expected one of: player, team, game")]
    UnknownKind(String),
    #[error("invalid {kind} key '{key}': {reason}")]
    BadKey { kind: &'static str, key: String, reason: String },
}
```

`GameId` is a new newtype `pub struct GameId(pub u64)` mirroring
`PlayerId(u32)`; defined in `icelines-core/src/identity.rs`.

**Why stringly-typed everywhere**: SQLite `TEXT` round-trips without
per-row JSON parse cost; URL `%3A`-encodes cleanly; no two-encodings
drift between JSON envelopes and SQL rows; `LeagueAbbrev` precedent
proves the pattern.

### `Freshness` (icelines-core/src/freshness.rs — new)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Freshness {
    pub fetched_at: DateTime<Utc>,
    pub source: FetchSource,
    pub ttl: Ttl,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]   // FORGE H2 — adding RSS / Peer / etc. is non-breaking
pub enum FetchSource {
    Bundle,        // shipped in the binary
    Setup,         // pulled by the setup wizard
    Live,          // lazy-fetched on read miss
    DataInstall,   // installed via `icelines data install` (frozen)
    Manual,        // explicit `fetch X` command
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Ttl {
    /// Frozen — historical season totals, finalized boxscores, etc.
    /// Sync skips these unless `--force`.
    Static,
    /// Refresh after this duration. `(fetched_at + after) < now()` → stale.
    After(Duration),
}

impl Freshness {
    pub fn is_stale(&self, now: &dyn Clock) -> bool {
        match self.ttl {
            Ttl::Static => false,
            Ttl::After(d) => now.utc_now() > self.fetched_at + d,
        }
    }
}

// Clock injection — production = SystemClock; tests = MockClock.
pub trait Clock: Send + Sync {
    fn utc_now(&self) -> DateTime<Utc>;
}
```

`DataInstall` source enforces `Ttl::Static` at construction — user
explicitly chose to install, doesn't get auto-refreshed (TAPE M3).

### Persistence model — manifest is mutable + self-validating (Round 3 Impl B4)

**Resolves the F.0.3 ↔ F.0.5 contradiction** the implementation-readiness
review caught. The manifest is **persisted**, **mutable**, and on each
process open performs a **fast self-validation pass**:

1. Read each shard JSON (HashMap-indexed via OnceLock).
2. For each entry, check `path` exists on disk.
3. Drop entries whose backing file is missing (logged via
   `tracing::warn!`, not user-visible).
4. Walk `~/.icelines/snapshots/<active>/` + bundle for entries the
   manifest doesn't know about; add them with `source: DataInstall`
   (snapshots) or `source: Bundle` (embedded).

The validation pass is **mtime-cached** (Round 3 Risk #1): if every
source dir's mtime ≤ the manifest's `last_validated_at` field, skip
the validation walk. Only re-validate when something on disk changed.
Reduces CLI cold-start from ~200-500 ms to ~5 ms after first run.

**Lazy-fetch path appends to the manifest**, persists immediately via
the atomic-write pattern below. Mutations are not in tension with
the rebuild story — rebuild is reconciliation, not replacement.

**Recovery**: delete `~/.icelines/data/manifest/`. Next open
re-validates from scratch.

### `Manifest` (icelines-fetch/src/manifest.rs — new)

Sharded by kind so `query leaders` deserializes ~50 entries (bios+stats),
not the full 50k including boxscores.

```
~/.icelines/data/manifest/
├── version.json          { schema_version: 1, min_reader_version: 1 }
├── bios.json             { datasets: [{season, fetched_at, source, ttl, path}, ...] }
├── stats.json            same shape
├── goalie_stats.json
├── transactions.json
├── boxscores.json        large; loaded only when boxscore consumer runs
├── career_history.json   single-blob entry pointing at the global file
└── …
```

Loaded into `OnceLock<HashMap<(DataKind, DataKey), Freshness>>` per
shard on first access. O(1) lookup after.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DataKind {
    Bios,
    Stats,
    GoalieStats,
    Transactions,
    Boxscore,
    CareerHistory,
    Schedule,
    Score,
    PlayoffBracket,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataKey {
    Season(Season),
    SeasonType(Season, SeasonType),
    Game(GameId),
    Date(NaiveDate),
    Player(PlayerId),       // career_history per-player
    Global,                 // career_history single-blob form
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DataError {
    #[error("data not installed for {kind:?} / {key:?} — \
             run `icelines setup` or `icelines fetch …`")]
    NotInstalled { kind: DataKind, key: DataKey },

    #[error("manifest entry exists but file missing on disk: {path}")]
    BackingFileMissing { path: PathBuf },

    #[error("network error fetching {url}: {source}")]
    Network { url: String, source: reqwest::Error },

    #[error("HTTP {status} from {url}")]
    Http5xx { url: String, status: u16 },

    #[error("schema drift parsing {url}: {detail}")]
    SchemaDrift { url: String, detail: String },

    #[error("manifest version {found} requires reader v{min_supported} \
             (we are v{our_version})")]
    VersionTooNew { found: u32, min_supported: u32, our_version: u32 },

    #[error("corrupt JSON in {path}: {source}")]
    Corrupt { path: PathBuf, source: serde_json::Error },

    #[error("io error: {source}")]
    Io { #[from] source: std::io::Error },
}
```

**Atomic writes**: every shard mutation goes through `tmp + rename`
(mirrors `CareerHistoryStore::save`). Concurrent processes use
`fs2::FileExt::lock_exclusive` on `manifest/.lock`; readers take a
shared lock. Spec invariant: **manifest mutations are serialized;
readers never observe a half-written shard.**

**Versioning** (WIRE H2):
- `version.json` carries `schema_version` (current writer's) +
  `min_reader_version` (refuse-to-read floor).
- Reader compiled-in `MAX_SUPPORTED = 1`; aborts with a clear error
  if `min_reader_version > MAX_SUPPORTED`.
- Unknown top-level keys preserved on rewrite (forward-compat).
- Unknown `DataKind` values logged + skipped (don't fail).

### `DataStore` (icelines-fetch/src/datastore.rs — new)

Bytes + manifest layer. Returns parsed-but-uncached domain types or
raw bytes; **does NOT cache loaded data** (FORGE H3 layering rule).
`StatsRepository` remains the session-cached domain layer; it calls
`DataStore` on misses.

```rust
pub struct DataStore {
    root: PathBuf,                              // ~/.icelines/data/
    manifest: ManifestSet,                      // sharded; OnceLock per kind
    clock: Arc<dyn Clock>,
    live_feeds: bool,
    test_mode: bool,                            // skips lazy-fetch in tests
}

impl DataStore {
    pub fn load_bios(&self, season: Season) -> Result<Vec<SkaterBio>, DataError>;
    pub fn load_stats(&self, season: Season, type_: SeasonType) -> Result<Vec<SkaterStats>, DataError>;
    pub fn load_career_history(&self, pid: PlayerId) -> Option<CareerHistory>;
    pub fn load_boxscore(&self, game: GameId) -> Result<BoxscoreData, DataError>;
    pub fn freshness(&self, kind: DataKind, key: DataKey) -> Option<Freshness>;
    pub fn list_seasons(&self, kind: DataKind) -> Vec<Season>;

    // Read order: manifest data dir → bundled → lazy fetch (if allowed).
}
```

**Read priority**:
1. Look up `(kind, key)` in manifest. Hit → load from `data/...` path.
2. Miss → check bundle (`BUNDLED_*`). Hit → return. (No manifest entry
   written — bundle is implicit.)
3. Miss + `live_feeds && !test_mode` → lazy-fetch from NHL API, write
   to data/, append manifest entry, return. **Print one-line stderr
   banner** ("fetching season 2008-2009 from NHL API…" — TAPE H2).
4. Miss + offline → return `DataError::NotInstalled` with remediation
   pointing at `icelines setup --season YYYYZZZZ` (WIRE M6's 409 path).

**Bundle stays as-is** (locked: 38 seasons). DataStore reads the
embedded bytes via the existing `BUNDLED_*` constants when manifest
misses but bundle hits. No bundle reduction; no `BUNDLED_SEASONS`
cull. The architecture is a wrapper, not a replacement.

**Bundle freshness**: `freshness(kind, key)` for a bundle-served
season synthesizes `Some(Freshness { source: Bundle, ttl: Static })`
at query time. Sync engine's `enumerate_stale` skips Bundle-source
entries (Static TTL never reports stale). No manifest entry written
for bundle hits.

**Tie-breaker for shim collisions** (Round 3 Impl H5): if both
`~/.icelines/data/seasons/<S>/bios.json` and
`~/.icelines/snapshots/<active>/bios.json` exist for season S, the
**`data/seasons/` path wins**. Snapshots are legacy; modern path
takes precedence.

**`live_feeds` flag source**: read from `Config` (default `true`),
overridden by `--no-live` CLI flag, overridden by
`ICELINES_NO_LIVE=1` env var. Independent of `ICELINES_TEST_MODE=1`,
which is test-runner-only and short-circuits both lazy-fetch and
sync engine.

### `BoxscoreData` type (avoids two-Boxscores collision — Round 3 Impl B2)

The workspace already has two types named `Boxscore`:
- `icelines-fetch/src/nhl_api.rs:696` — wire-shape from the API
- `icelines-fetch/src/shift_profile.rs:40` — `BoxscoreData` (the
  domain shape used by `aggregate_profiles`)

Foster's `DataStore::load_boxscore` returns the existing
**`BoxscoreData`** (the domain shape). `nhl_api::Boxscore` stays as
the wire-only type used during `fetch_boxscore`. Foster.3's fetcher
maps `nhl_api::Boxscore` → `BoxscoreData` + extracted events,
persists both. No new type introduced.

### `DatasetStore` trait — retrofit existing stores (Round 3 Coherence H2 / FORGE M2)

```rust
pub trait DatasetStore: Send + Sync {
    type Key;
    type Value;
    type Error;

    fn load(&self, key: &Self::Key) -> Result<Self::Value, Self::Error>;
    fn save(&self, key: &Self::Key, value: &Self::Value) -> Result<(), Self::Error>;
    fn freshness(&self, key: &Self::Key) -> Option<Freshness>;
}
```

Foster.0 retrofits `CareerHistoryStore` (career_landing.rs) onto this
trait in the same commit it's introduced. New stores (boxscore JSON,
event stream sidecars) implement it. Atomic-write helper extracts to
`icelines-fetch/src/atomic_write.rs` so `CareerHistoryStore` and
`Manifest` share one implementation rather than two copies.

### Snapshot read-shim (Foster.0.7)

`~/.icelines/snapshots/` is **immutable read-only input** (FORGE B1).
On every DataStore open, the manifest is rebuilt by walking:

1. Embedded bundle (always present)
2. `~/.icelines/data/seasons/...` (modern path)
3. `~/.icelines/snapshots/<active>/...` (legacy; recovered as
   `Freshness { source: DataInstall, ttl: Static }`)

Translation table SnapshotTier → DataKind (TAPE B1):

| `SnapshotTier` | `DataKind` |
|---|---|
| `Stats` | `Bios`, `Stats` (one snapshot dir → two manifest entries) |
| `Realtime` | (folded into `Stats` — realtime is annotated bios fields) |
| `MoneyPuck` | (`Stats` — same shape) |
| `Contracts` | `Stats` (extends bios) |
| `Goalies` | `GoalieStats` |
| `Positions` | (out of scope — boxscore-derived; not needed in Foster) |
| `Rosters` | (out of scope — same) |
| `Derived` | (out of scope) |

Manifest is **rebuilt fresh on every open**, never mutated to track
snapshots. Recovery from a corrupt manifest = "delete manifest dir,
restart" — DataStore rebuilds.

## Capability matrix (Foster.0 + Foster.4)

```toml
[sync]
policy = "eager"     # eager | lazy | off
banner = "summary"   # summary | silent | verbose
season_transition = "prompt"   # prompt | auto | ignore

[sync.capabilities]
stats           = "league"      # base — always on
scores_schedule = "league"      # default ON for everyone
transactions    = "favorites"   # opt-in to "league"
boxscores       = "favorites"
shifts          = "off"         # only "off" valid until shift parsing exists
career_history  = "favorites"
```

Each capability has three modes: `off` / `favorites` / `league`. The
sync engine reads the matrix on each refresh tick to decide what to
fetch. Setup wizard maps three user questions to the matrix:

```
1. Track all NHL transactions, or just your favorites?
   [favorites]  league  off
2. Pull deeper stats for your favorites? (advanced metrics, ~5 MB/wk)
   [yes]  no                          → maps to boxscores=favorites|off
3. Refresh on app launch?
   [eager]  lazy  off
```

`shifts` capability is reserved in the matrix but enforced as
`off`-only until per-shift parsing ships in a future phase. Setting
to `favorites`/`league` returns this **literal error message**:

```
error: capability `shifts` cannot be set to `<chosen>` yet —
       per-shift parsing isn't implemented. Reserved for a future
       phase. Allowed values today: off
```

Capability matrix tests (24) assert this exact string for `shifts`
mode-honored cases.

### `icelines config` CLI surface (Round 3 Persona Blocker #1)

Capability matrix is configurable post-setup without hand-editing
TOML:

```bash
icelines config get sync.capabilities.transactions      # prints "favorites"
icelines config set sync.capabilities.transactions league
icelines config set sync.policy lazy
icelines config list                                      # all keys
icelines config reset sync.capabilities                  # reset to defaults
```

Implemented in `icelines-cli/src/commands/config.rs` (~120 lines).
Validates against the typed config schema; rejects unknown keys
with a "did you mean" suggestion (Levenshtein-1, mirrors LB.1
pattern). 6 L0 tests for the parser, 4 L2 for the subprocess
surface (in F.0.7's 24-test capability matrix file).

### EntityRef as filter atom (Round 3 Coherence H1 / EDGE H2)

`parse_filter_expr` (icelines-core/src/stats_catalog.rs) gains
`team=ABBR` and `opponent=ABBR` as recognized bio_keys, routed
through `extract_bio` in icelines-query. Ten lines of code; saves
a breaking change later when users naturally type
`--filter "team=EDM"`.

### Capability mode bleed-through on filters (Round 3 Coherence H1 / EDGE H3)

When `transactions = favorites`, an explicit query for a non-favorited
player (e.g. `query transactions --player Ovechkin`) **proceeds with a
warning**:

```
warning: 'Ovechkin' is not in your favorites; fetching ad-hoc.
         Use --pin to install permanently, or
         `icelines config set sync.capabilities.transactions league` to
         widen the default scope.
```

Allow + warn (not reject + not silent auto-cache). User sees the
result; the data isn't persisted unless `--pin` is set.

## Sync engine (Foster.4)

**Non-blocking background refresh** (PACE B1 — current `get_json` is
sequential with 50ms inter-call sleeps; eager refresh would block
the alt-screen for 10-15 sec typical / 2.5 min worst case).

Architecture:

```rust
pub fn launch_eager_sync(store: Arc<DataStore>) -> mpsc::Receiver<SyncEvent> {
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(async move {
        let stale = store.enumerate_stale().await;
        let mut refreshed = 0;
        for entry in stale {
            match store.refresh(entry).await {
                Ok(_) => { refreshed += 1; let _ = tx.send(SyncEvent::Refreshed(...)).await; }
                Err(e) => { let _ = tx.send(SyncEvent::Failed(...)).await; }
            }
        }
        let _ = tx.send(SyncEvent::Done { refreshed, elapsed: ... }).await;
    });
    rx
}

pub enum SyncEvent {
    Refreshed { kind: DataKind, key: DataKey },
    Failed { kind: DataKind, key: DataKey, error: String },
    Done { refreshed: usize, elapsed: Duration },
}
```

CLI `icelines tui` and `icelines query …` both spawn this on launch
when `policy = eager`. Banner state lives in a `SyncStatus` widget
(TUI status bar) that drains the channel and renders progress.
Process exits don't await pending refresh.

**Test mode**: `ICELINES_TEST_MODE=1` env var skips the spawn entirely
(BENCH B3). `MockClock` injected via `Arc<dyn Clock>` so `Freshness::is_stale`
returns deterministic values.

**Per-kind TTL defaults**:

| Kind | TTL | Why |
|---|---|---|
| Bios (current season) | `After(24h)` | Roster trades, healthy scratches |
| Stats (current season) | `After(6h)` | Stats change every game day |
| Bios/Stats (historical) | `Static` | Frozen forever |
| Schedule (today/future) | `After(2h)` | Live scores during the day |
| Schedule (past) | `Static` | Boxscores immutable post-game |
| Score (today) | `After(15min)` | Live games |
| Score (past) | `Static` | |
| Transactions (current season) | `After(12h)` | New trades land throughout the year |
| Transactions (historical) | `Static` | |
| Boxscore (any) | `Static` | Frozen once final |
| CareerHistory (any) | `After(7days)` | Adds slowly |

`DataInstall` source overrides any `After(...)` to `Static` —
explicit user choice.

## Migration 006 — groups: kind → entity_ref (FORGE B2)

Current schema (after migration 005):

```sql
group_members (
  group_name TEXT, player_normalized TEXT, added_at TEXT,
  kind TEXT NOT NULL DEFAULT 'player',
  PRIMARY KEY (group_name, player_normalized)
)
```

Migration 006:

```sql
-- Add new column
ALTER TABLE group_members ADD COLUMN entity_ref TEXT;
-- Backfill from existing kind + player_normalized
UPDATE group_members
   SET entity_ref = CASE kind
       WHEN 'team' THEN 'team:' || player_normalized
       ELSE 'player:' || player_normalized
   END
WHERE entity_ref IS NULL;
-- Replace PK (sqlite: rebuild table)
CREATE TABLE group_members_new (
  group_name TEXT NOT NULL,
  entity_ref TEXT NOT NULL,
  added_at TEXT NOT NULL,
  PRIMARY KEY (group_name, entity_ref),
  FOREIGN KEY (group_name) REFERENCES groups(name) ON DELETE CASCADE
);
INSERT INTO group_members_new (group_name, entity_ref, added_at)
  SELECT group_name, entity_ref, added_at FROM group_members;
DROP TABLE group_members;
ALTER TABLE group_members_new RENAME TO group_members;
```

`MemberKind` becomes a thin `From<&EntityRef>` view that derives
the player/team discriminator from the entity_ref prefix. One
source of truth.

## Test plan

**Foster.0 = 35 tests** (BENCH B1):

- EntityRef serde (8): stringly + struct round-trips, all 3 variants,
  hash equality, error cases (malformed/unknown-kind/bad-key)
- Freshness/TTL (6): stale/fresh/never-expire/clock-skew/Static-source-pinned/test-mode-bypass
- Manifest (8): add/remove/list/atomic-save/concurrent-writer-lock/schema-bump/missing-file/corrupt-JSON
- DataStore routing (8): bundled-hit, manifest-hit, lazy-fetch-hit,
  lazy-fetch-disabled, lazy-fetch-network-fail, lazy-fetch-5xx,
  lazy-fetch-schema-drift, fallback-order
- Migrations (5): 006 round-trip with pre-migration fixture, idempotent
  re-run, mixed kind→entity_ref backfill, FK cascade, partial-rollback

**Capability matrix = 24 tests** in new `foster_capability_matrix.rs`:

- 18 mode-honored: 6 capabilities × 3 modes each. Each test asserts
  that toggling the mode changes the data fetched/stored as expected.
- 6 interaction tests: transactions=favorites + boxscores=off →
  graceful degrade; shifts=off blocks fetch_shifts; career_history=
  favorites filters non-fav lazy fan-out; sync=off short-circuits
  Foster.4; banner=summary vs off; season_transition=prompt blocks
  in test mode without prompt UI.

**Foster.4 = 15 tests** (BENCH H2):

- Background spawn lifecycle (spawn → drain → done event)
- Banner rendering (shown / suppressed / summary-vs-off / age-formatting)
- Per-capability staleness gating (5)
- Stale-cache + offline → banner with "offline — last refreshed Nh ago"
- `ICELINES_TEST_MODE=1` skip
- `MockClock` time-travel
- One-shot channel close on process exit
- Banner verbosity modes (3)

**Total Foster.0 + .4 = 74 tests.**

## Files added

```
icelines-core/src/entity.rs            (~150 lines)
icelines-core/src/freshness.rs         (~100 lines)
icelines-fetch/src/manifest.rs         (~250 lines)
icelines-fetch/src/datastore.rs        (~300 lines)
icelines-fetch/src/sync_engine.rs      (~150 lines, Foster.4)
icelines-cli/src/commands/setup.rs     (~150 lines)
icelines-cli/src/commands/data_status.rs (~80 lines)
icelines-cli/tests/foster_capability_matrix.rs (~400 lines)
```

## Open items (still unresolved)

1. **`fs2` vs `fd-lock` vs no-lib** — Rust ecosystem has both, neither
   is in the workspace today. `fs2` works on Windows; pick during
   Foster.0.4 implementation.
2. **Boxscore retention/pruning** (TAPE M1) — `data prune --before
   YYYY-MM-DD` with default 90-day retention for non-favorited games.
   Spec'd here but implementation deferred to Foster.3 (where
   boxscores actually start landing).
