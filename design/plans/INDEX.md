# IceLines Plans Index

Plans track work items from idea to completion.  
**Status**: Draft → Active → Implemented → Closed

---

## Active

| Plan | Status | Summary |
|------|--------|---------|
| [Phase 8 — Spec Delta Catch-Up](2026-04-28-spec-delta-catchup.md) | Draft | 7 sub-phases (8a–8g) closing spec → code gaps after Phase 7 |
| [Phase 5 — Query Engine](2026-04-26-phase5-query-engine.md) | Active | `icelines query` leaders/player/compare, --seasons N, improvement sort |
| [Phase 6 — Export & Dashboard](2026-04-26-phase6-export-dashboard.md) | Draft | `icelines export md`, proof DASHBOARD-SPEC integration |

---

## Implemented

| Plan | Completed | Summary |
|------|-----------|---------|
| [Phase 1 — Rust CLI Foundation](2026-04-25-rust-cli-foundation.md) | 2026-04-25 | 4-crate workspace, fetch, team, rank, bundled data |
| [Phase 2 — Site & Analysis](2026-04-25-phase2-site-analysis.md) | 2026-04-25 | mkdocs site, scheme engine, snapshot store, players command |
| [Phase 3 — TUI & Projections](2026-04-25-phase3-tui-projections.md) | 2026-04-25 | ratatui TUI, projections, career history, scouting |
| [Phase 4 — Data, History & Polish](2026-04-26-phase4-data-history-polish.md) | 2026-04-26 | Multi-season bundled data, L0/L1/L2 test coverage, CI |
| [Phase 7 — TUI v2 Redesign](2026-04-28-phase7-tui-v2-redesign.md) | 2026-04-28 | 6-tab nav, season time-travel, Scores/Schedule/Playoffs (7a–7e) |

---

## Backlog (not yet planned)

| Item | Value | Blocked on |
|------|-------|------------|
| NHL Edge skating speed stats | ⭐⭐ | Nothing — same API pattern |
| `icelines export md` | ⭐⭐⭐ | Nothing — pre-work for dashboard |
| Fantasy daily delta scoring | ⭐⭐ | Nothing |
| Historical season queries (`--season 20242025`) | ⭐⭐ | Nothing |
| Data bundle GitHub Releases (#42) | ⭐⭐ | CI trigger |
| CI: cargo fmt + cargo audit | ⭐⭐ | Nothing |
| proof DASHBOARD-SPEC integration | ⭐⭐⭐ | proof implementing org tree + chart generation |
| TUI as proof dashboard renderer | ⭐⭐⭐ | proof DASHBOARD-SPEC |
| MoneyPuck historical xG | ⭐ | Nothing |
| Strength-state 5v5/PP/PK splits | ⭐ | Play-by-play + shift data (large lift) |
