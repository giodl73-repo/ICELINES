# Phase Lester Patrick — persona pass + closeout (2026-05-05)

End-of-phase walkthrough recorded per `design/plans/2026-05-05-phaseLesterPatrick-cli-parity.md` § LP.6.

## Per-command observations

### LP.1 — `icelines schedule`

Existing command gained `--json`, `--csv`, comfy-table default, and `--days` range constraint (1..=14). 4 L0 tests cover: team filter, days-cap is per-distinct-day not per-game, no-match path, CSV column order. Verified manually against the live NHL API.

**No follow-ups filed.**

### LP.2 — `icelines playoffs`

Brand-new. Default season picks the most recent COMPLETED bracket so the offseason output isn't empty. `--season` overrides; `--round 1..=4` filters. JSON envelope carries champion + Conn Smythe at the top level. CSV has 8 columns.

Verified manually:

```
$ icelines playoffs --round 4
PLAYOFFS — 1993-94  ·  Champion: NYR
Conn Smythe: Brian Leetch

+-------------------+--------+-----+--------+--------+--------+
| Round             | Series | Top | Bottom | Result | Winner |
| Stanley Cup Final |        | NYR | VAN    | 4-3    | NYR    |
+-------------------+--------+-----+--------+--------+--------+
```

4 L0 tests against the bundled 1993-94 fixture (NYR/VAN, 7-game Cup Final).

**Follow-up filed**: bundle more historical playoffs. Today only `19931994` is embedded (`BUNDLED_PLAYOFFS` in `bundled.rs`); installed seasons take precedence so users can refresh via `icelines data install <season>`. Bundling more would let the default-season picker walk further back than 1993-94 in cold offline scenarios. Tracked as **LP-followup**.

### LP.3 — `icelines transactions`

**Already complete** from Phase Selke (2026-04-30). The pre-existing `commands/transactions.rs` covers everything LP.3 planned (`--team`, `--since`/`--until`, `--kind`, `--search`, `--player`, `--season`, `--top`, `--json`, `--csv`, `--out`) plus extras: `validate_iso_date` for clean error messages, ESPN-archive guard for pre-2021-22 seasons, classifier-version awareness, and 6 L0 tests.

**The IceLines.md portfolio matrix was wrong about this row** — it claimed CLI ❌ when CLI was already ✅. Fixed in LP.5.

**No code changes.** Phase Lester Patrick's transactions deliverable was a no-op; the matrix correction is the only visible change.

### LP.4 — In-TUI docs overlay

**Deferred to a follow-up phase.** Pulldown-cmark → ratatui span conversion + scroll handling + new `Screen::Docs` variant + render path is substantial work that can land independently after Lester Patrick closes. The CLI surface portfolio is now complete without it; the docs gap is "in-TUI only" — `icelines docs` and `/docs` cover the other two surfaces.

## Numbers

- **Tests added across the phase**: 8 (4 LP.1 + 4 LP.2). LP.3 contributed 0 because the pre-existing implementation covered scope.
- **Total CLI bin tests**: 561 (up from 553 post-Lady Byng).
- **Net code added**: ~430 LoC across `commands/playoffs.rs` (new) + extensions to `commands/tonight.rs` and `cli.rs`.
- **Commits**: LP.1 + LP.2 + LP.5 + LP.6. LP.3 is a no-op; LP.4 is deferred.

## Surface portfolio status post-LP

Per `design/IceLines.md` § "Feature × surface portfolio":
- All three CLI ❌ rows from the post-Lady-Byng matrix flipped to ✅.
- The docs ❌ row in the TUI column remains (LP.4 deferred).
- Superseded by Selke/Ted Lindsay follow-up: fantasy read/product web routes
  now ship through `/fantasy` and `/api/v1/fantasy/*`; local CLI remains the
  primary mutation surface.

## What's next

At the time of this note, the portfolio was ~95% complete. The two remaining
gaps were:
1. **In-TUI docs overlay (LP.4)** — flip TUI ❌ to ✅. Future LP-followup phase.
2. **Fantasy web read/product parity** — superseded by later Selke/Ted Lindsay
   work; `/fantasy`, `/api/v1/fantasy/gaps`, and
   `/api/v1/fantasy/simulate` now exist.

After this phase the next-highest-value work, in rough order:
- Persona Wave 5 — exercise the new LP.1 / LP.2 commands with realistic scripts.
- Pre-existing follow-ups from LB.7 — Ctrl-C handler in menu, web option `W` `AddrInUse` introspection.
- LP.4 if/when in-TUI docs becomes a real ask.

Phase Lester Patrick: **complete** (LP.4 deferred). Bumping to v0.14.0 with Lady Byng + Lester Patrick bundled.
