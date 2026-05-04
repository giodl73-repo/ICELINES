# Phase King Clancy — King.1 — `icelines serve` skeleton

**Status**: Draft v0.1 — pending forge/wire/bench re-review sign-off on the spec
**Date**: 2026-05-04
**Spec**: `design/specs/web-dashboard.md` (the King Clancy spec, v2026-05-04)
**Target**: v0.13.0 → King.1 ships as the first stop

---

## Goal

Stand up the `icelines serve` web dashboard skeleton: a working axum server bound to `127.0.0.1:8000`, vendored static assets, a single `/` page, and the staged `serve` rename plumbing. No real analytics surfaces yet — those land in King.2-9.

This phase is intentionally narrow: prove the binary boots, the rename works, the static-assets pipeline works, and the concurrency model compiles. Everything else is the next ten phases.

User-visible after King.1:
```
$ icelines serve
→ http://localhost:8000
[opens browser]
```
And the browser shows a placeholder home page with the active season label and the IceLines logo.

---

## Scope

| In | Out (deferred to later King.N) |
|---|---|
| Top-level `Commands::Serve` clap variant | Real `/leaders` / `/player` / `/api/v1/*` routes |
| `icelines site {build,serve,deploy}` group + hidden deprecated aliases | Filter form / sort picker / preset selector |
| `WebState` struct + concurrency-model decision (RwLock or LocalSet) | Reports overlay + season picker |
| Vendored `static/` (HTMX, CSS, logo) via `include_bytes!` | Live-data routes (scores / schedule / playoffs) |
| `/` placeholder page (askama template, active-season header) | Fantasy fold-in |
| Browser auto-open with `--no-open` opt-out | LAN mode hardening |
| `--port`, `--bind`, `--no-cache` flags wired (no behavior on `--no-cache` yet) | `?partial=` HTMX fragments |
| `WebError` thiserror enum + `IntoResponse` impl | a11y axe-core fence |
| `/static/*` route serving with Cache-Control + ETag | Markdown export `?format=md` |
| `Config::with_root(tempdir)` test seam | KEEL-B1 cross-surface JSON-key fence (no JSON routes yet) |

---

## Pre-King.1 — staged rename (required before King.1 lands)

Rename in code so the new `Commands::Serve` doesn't collide with the existing one:

1. Add new enum: `Commands::Site(SiteSubcommand)` where `SiteSubcommand = Build | Serve | Deploy`.
2. Move existing `Commands::Build/Serve/Deploy` body into `Commands::Site(...)` dispatch.
3. Keep `Commands::Build`, `Commands::Serve`, `Commands::Deploy` as **hidden** top-level variants (`#[command(hide = true)]`) that print a deprecation warning to **stderr**, then dispatch to the new path:
   ```
   WARNING: 'icelines build' moved to 'icelines site build' in v0.13.
            The old alias is removed in v0.14. Run 'icelines site build' instead.
   ```
4. Update `COMMANDS.md` + `--help` long_about for the new `site` group.

L0 fences:
- `l0_deprecated_build_alias_writes_to_stderr_and_dispatches`
- `l0_deprecated_serve_alias_writes_to_stderr_and_dispatches`
- `l0_deprecated_deploy_alias_writes_to_stderr_and_dispatches`

This commit ships first, isolated. THEN King.1 can introduce the new `Commands::Serve`.

---

## Crate layout

New crate: **`icelines-web`** (peer of `icelines-site`).

```
icelines-web/
├── Cargo.toml
└── src/
    ├── lib.rs            # public API: `pub fn router(state: WebState) -> Router`
    ├── state.rs          # WebState struct + builder
    ├── error.rs          # WebError thiserror enum + IntoResponse
    ├── handlers/
    │   ├── mod.rs
    │   └── home.rs       # GET /
    ├── templates/        # askama .html
    │   ├── base.html     # nav + footer + active-season header
    │   └── home.html
    └── static/
        ├── htmx.min.js   # vendored, ~14 KB
        ├── style.css     # ~5 KB hand-rolled
        └── icelines.svg  # logo
```

Dependency chain: `icelines-web` depends on `icelines-core` + `icelines-fetch` (uses `StatsRepository`, `Config`). `icelines-cli::commands::serve` constructs a `WebState` and calls `icelines_web::router()`.

`icelines-cli/src/commands/serve.rs` is a thin entry point: parse flags, build `WebState`, mount router, bind socket, optionally launch browser, run.

---

## Concurrency model — King.1 picks

The spec leaves the choice between (a) `Arc<RwLock<StatsRepository>>` and (b) `LocalSet` + `Rc<RefCell<App>>`. King.1's job is to measure and pick.

**Plan**: try (a) first.
1. Audit `StatsRepository` for `!Send + !Sync` markers (`PhantomData<*const ()>`).
2. If the marker is purely from `LruCache<_, _>` (which is `Send` if its values are), wrap LRU access in `Mutex<LruCache>` interior locking — repo becomes `Send + Sync`.
3. If other markers surface or the audit shows complications, fall back to (b) and document the throughput cap.

Expected outcome: (a) works with a small refactor. King.1 plan ships with the chosen model documented in commit message.

`WebState`:
```rust
pub struct WebState {
    pub repo: Arc<RwLock<StatsRepository>>,
    pub config: Arc<RwLock<Config>>,
    pub fantasy_db: Arc<FantasyDb>,
    pub group_db: Arc<GroupDb>,
    pub cache: Arc<moka::sync::Cache<CacheKey, CachedResponse>>,
}
```

King.1 adds the type but does not exercise the cache yet (no real routes to cache).

---

## Dependencies added

Workspace-deps (`Cargo.toml` `[workspace.dependencies]`):

```toml
axum = "0.7"
tower-http = { version = "0.5", features = ["compression-gzip", "cors", "trace"] }
askama = { version = "0.12", features = ["with-axum"] }
askama_axum = "0.4"
moka = { version = "0.12", features = ["sync"] }
ulid = "1"
open = "5"  # for browser auto-open
```

Already in workspace: `tokio`, `serde`, `serde_json`, `tracing`, `thiserror`, `reqwest` (for tests).

`icelines-web/Cargo.toml`:
```toml
[dependencies]
axum = { workspace = true }
tower-http = { workspace = true }
askama = { workspace = true }
askama_axum = { workspace = true }
moka = { workspace = true }
ulid = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["sync"] }
icelines-core = { path = "../icelines-core" }
icelines-fetch = { path = "../icelines-fetch" }

[dev-dependencies]
reqwest = { workspace = true, features = ["json"] }
insta = { workspace = true }
tempfile = { workspace = true }
```

Compile-time budget: King.1 records the clean-build delta. If >60s additional, surface for re-evaluation.

---

## Tasks

### King.1.0 — Pre-King.1 rename (separate commit, lands first)
- [ ] Add `Commands::Site(SiteSubcommand)` enum
- [ ] Move build/serve/deploy bodies into Site dispatch
- [ ] Add hidden deprecated top-level aliases printing to stderr
- [ ] Update COMMANDS.md (mkdocs section moves under `icelines site`)
- [ ] Update `--help` long_about for `icelines site`
- [ ] L0 fences for the 3 deprecated aliases
- [ ] cargo build / clippy / fmt clean

### King.1.1 — `icelines-web` crate skeleton
- [ ] Create crate at `icelines-web/`
- [ ] Add to `Cargo.toml` workspace members
- [ ] `lib.rs` exports `pub fn router(state: WebState) -> Router`
- [ ] `state.rs` defines `WebState`
- [ ] `error.rs` defines `WebError` thiserror enum (`UnknownStat`, `UnknownSort`, `UnknownSeason`, `UnknownPlayer`, `BadFilter`, `BadParam`, `ConflictingParams`, `NotFound`, `RateLimited`, `Internal`, `CorruptSnapshot`) + `IntoResponse` impl returning the spec error envelope shape

### King.1.2 — Concurrency model decision
- [ ] Audit `StatsRepository` Send/Sync markers
- [ ] Try `Arc<RwLock<StatsRepository>>` path; verify it compiles
- [ ] Fallback to `LocalSet` + `Rc<RefCell>` if blocked
- [ ] Document the choice in this plan file's "Outcomes" section
- [ ] Add `Config::with_root(tempdir)` constructor in `icelines-core` (test seam)

### King.1.3 — Static assets pipeline
- [ ] Vendor `htmx.min.js` (download from htmx.org, pin version in comment header)
- [ ] Write minimal `style.css` (~100 lines covering base + fit classes + score classes)
- [ ] Source / commit `icelines.svg`
- [ ] `static.rs` handler: serves `/static/*` with proper `Content-Type`, `Cache-Control: public, max-age=31536000, immutable`, ETag from `env!("CARGO_PKG_VERSION")`
- [ ] L0 fences: each asset MIME, Cache-Control, ETag

### King.1.4 — Home page
- [ ] `templates/base.html` with nav (placeholder links) + active-season header showing `{season} · {type}` from `WebState.config`
- [ ] `templates/home.html` extending base, single placeholder block "IceLines analytics — pick a tab above"
- [ ] `handlers/home.rs::get_home` returns rendered template
- [ ] L0: `oneshot` fires `GET /` and asserts response is HTML with active-season text

### King.1.5 — `Commands::Serve` wired
- [ ] `cli.rs` adds `Commands::Serve { port, bind, no_open, no_cache, cors_origin }` (long_about explaining web vs `site serve`)
- [ ] `commands/serve.rs::run`: builds `WebState`, mounts router, binds socket, optionally launches browser, awaits Ctrl-C
- [ ] Browser auto-open: print URL FIRST, then `open::that(&url)` honoring `BROWSER` env, swallow errors
- [ ] Port collision handling: `bind` returns `AddrInUse` → `eprintln!` with hint, `process::exit(1)`
- [ ] `--bind ADDR[:PORT]` + `--port N` interaction per spec

### King.1.6 — DNS rebinding + LAN warning
- [ ] `tower-http` Host header validation middleware: accept only `localhost`, `127.0.0.1`, the bind addr; reject 421
- [ ] When `--bind 0.0.0.0` (or non-loopback): print `WARNING: LAN mode — no auth, no TLS.` banner

### King.1.7 — Tests (King.1 floor: 10 L0 + 5 L1)

L0 (10):
1. `l0_serve_prints_url_before_open`
2. `l0_serve_continues_on_open_failure`
3. `l0_static_htmx_correct_mime_and_cache_control`
4. `l0_static_css_correct_mime_and_cache_control`
5. `l0_static_svg_correct_mime_and_cache_control`
6. `l0_home_renders_active_season_header`
7. `l0_web_error_unknown_stat_returns_400_with_envelope`
8. `l0_web_error_internal_returns_500_with_request_id`
9. `l0_deprecated_build_alias_writes_to_stderr_and_dispatches` (King.1.0)
10. `l0_deprecated_serve_alias_writes_to_stderr_and_dispatches` (King.1.0)

L1 (5):
1. `l1_serve_cold_start_under_500ms` — boot, time-to-listening, assert <500ms
2. `l1_get_home_returns_html_with_active_season` — full reqwest round-trip
3. `l1_get_static_htmx_returns_immutable_cache_control_with_etag`
4. `l1_dns_rebinding_rejected_when_localhost_bind` — fire `Host: evil.example` against `127.0.0.1` server, assert 421
5. `l1_lan_mode_prints_warning_banner` — capture stderr on `--bind 0.0.0.0` start

Total: 15 (above King.1 floor of 10+5 = 15 ✓)

### King.1.8 — DoD
- [ ] cargo build / clippy / fmt clean
- [ ] All 10 L0 + 5 L1 pass
- [ ] COMMANDS.md updated with `icelines serve` + `icelines site {build,serve,deploy}`
- [ ] `--help` long_about for `serve` leads with "Opens the web dashboard at http://localhost:8000 in your browser."
- [ ] Compile-time delta recorded in commit message
- [ ] Concurrency model decision (RwLock vs LocalSet) recorded in commit message
- [ ] Smoke test: `cargo run --release -- serve` + open `http://localhost:8000` in a real browser, see placeholder + active-season header

---

## Outcomes (filled in at close)

- **Concurrency model chosen** (King.1.2 — 2026-05-04): `Arc<RwLock<StatsRepository>>` via the new `send-sync` cargo feature on `icelines-core`. The Phase Hart `PhantomData<*const ()>` marker was a soft-lint enforcing "wrap me in `Arc<RwLock<_>>` at the call site" — but the marker itself prevented that exact wrapping. King.1.2 gates the marker behind `#[cfg(not(feature = "send-sync"))]`. `icelines-web` enables the feature; CLI/TUI consumers default to the original `!Send + !Sync` lint. Audit confirmed all `StatsRepository` fields (`HashMap`, `BTreeMap`, `VecDeque`, `usize`, `serde_json::Value`) are naturally Send+Sync — the marker was the only blocker. No `LocalSet`+`Rc<RefCell>` fallback was needed. L0 fence `state::l0_web_state_is_send_sync` proves `Arc<RwLock<StatsRepository>>` IS Send+Sync now.
- **Compile-time delta**: TBD (measured at King.1.5 close, after `commands::serve` driver + browser auto-open ship)
- **Binary size delta**: TBD (King.1.10 close)
- **Cold-start latency measured**: TBD (King.1.5 close, target <500ms)

---

## Risks for King.1 specifically

- `StatsRepository` `!Send` audit could surface non-trivial refactor → fallback to LocalSet adds plan-file effort but doesn't block
- Vendoring HTMX from htmx.org needs a one-time manual download (no Cargo crate); record SHA in a comment header so future updates are checksummed
- `open` crate behavior on Windows/WSL/headless varies — `l0_serve_continues_on_open_failure` catches the failure mode; manual smoke on each platform during PR review
- Active-season header relies on `Config::active_season_label()` — may need a small helper added to `icelines-core::Config` if not already present

---

## What King.2 starts with

After King.1 lands, King.2 has:
- A working `icelines serve` that boots and serves `/` + `/static/*`
- `WebState` ready to add `repo`-using handlers
- `WebError` ready to return error envelopes
- Test infrastructure (`Config::with_root(tempdir)`, `OnceLock<WebState>` shared fixture pattern)
- Migration warnings already in place

King.2's task: real `/leaders` HTML + `/api/v1/leaders` JSON with the full filter/sort/pagination contract from the spec.
