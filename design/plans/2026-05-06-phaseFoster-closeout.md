# Phase Foster — Closeout plan (Foster.6)

**Specs**: all four foster-*.md
**Test budget**: 4 unit + **30 personas** = 34 tests

---

## F.6.1 — Setup wizard polish

- Default-run smoothness (`icelines tui` from empty `~/.icelines/`)
- Helpful error messages when network unreachable during setup
- `--no-setup` flag for headless/scripted use
- `icelines setup --reset` clears manifest + re-prompts
- **Tests (1 L2)**: setup --no-setup skips wizard

## F.6.2 — `icelines data status`

- Pretty-print the manifest:
  ```
  DATA STATUS — ~/.icelines/data/
  ─────────────────────────────────────────────
  Source        Kind            Items   Freshness
  Bundle        bios            38      static
  Bundle        stats           38      static
  Setup         boxscore        14      4h ago
  Live          career_history  1655    7d (5 stale)
  DataInstall   bios            2       static (pinned)
  ```
- `--shard <kind>` to filter
- `--stale-only` to list what `fetch sync` would refresh
- **Tests (3 L0)**: empty manifest, populated, stale-only filter

## F.6.3 — Docs refresh

- COMMANDS.md — new `favorites` / `setup` / `data status` sections; new
  `--date` / `--range` / `--week` flags on existing commands;
  capability matrix in the "Data and history" section
- README.md — Foster headline ("Favorites dashboard, time-travel,
  unified data layer")
- CLAUDE.md — Phase Foster bullet under "What's been built"; capability
  matrix as a config table; non-blocking sync banner pattern
- design/specs/event-stream-payloads.md — frozen v1 schemas (referenced
  by foster-favorites-dashboard.md)

## F.6.4 — Persona pass (30 scenarios)

Mirroring `persona_wave3.rs` density. Distribution:

- **Setup-from-scratch (6)**: each capability mode × first-run path
- **Favorites flows (6)**: add player / add team / mid-day trade / goalie pull / DNP / multi-group selector (out-of-scope but probe)
- **Time-travel (6)**: past-date scores / past-date schedule / past-date playoffs / week-aggregate / month-aggregate / season-rollup
- **Sync engine (6)**: eager refresh / lazy stale-banner / off mode / network failure / season transition / `MockClock` deterministic refresh
- **Data layer (6)**: lazy fetch happy path / lazy fetch offline / lazy fetch 5xx / capability mode honored / migration 006 round-trip / `data status` empty/populated/stale

## Files modified

```
COMMANDS.md                                          +120 lines
README.md                                            +40 lines
CLAUDE.md                                            +60 lines (Foster bullet + matrix)
design/specs/event-stream-payloads.md                ~200 lines (new sibling spec)
icelines-cli/tests/persona_foster_v6.rs              ~600 lines (30 personas)
```

## Acceptance for Foster.6

- All four Foster specs pass a final read-through with no broken
  internal references
- COMMANDS.md sections renumber cleanly
- 30-persona suite passes; no flakes across 3 consecutive runs
- 8-role closeout review identifies no new blockers
- `cargo test --workspace` green
- Tag `v0.16.0` (Phase Foster) cut

## Roles closeout

After Foster.6 lands, run a final 8-role review on the full phase
output. The goal is "did we forget anything" — not finding new
blockers (those should be impossible at this point) but capturing
lessons for future phases.
