---
name: icelines-wave
description: "Open, advance, and close IceLines backfill waves. Artifacts live in design/waves/{date}-{verb}-the-{object}/ and convert trophy-phase debt into pulse plans."
tags: [icelines, wave, backfill, phases, planning]
---

# icelines-wave

Use this skill when the user wants a new IceLines wave, status on the active
wave, or a closeout.

## Commands

```text
/icelines-wave status
/icelines-wave new <theme>
/icelines-wave next
/icelines-wave close
```

## Artifact Roots

- Wave index: `design/waves/PHASES.md`
- Active wave: first row in `design/waves/PHASES.md` with status `active`
- Wave folder: `design/waves/{YYYY-MM-DD}-{verb}-the-{object}/`
- Roles: `.roles/*.md`
- Phase source plans: `design/plans/`

## Lifecycle

1. Read the active wave `WAVE.md`.
2. Generate or amend pulse plans in `plans/`.
3. Review plans through `.roles`.
4. Materialize forks with `icelines-fork`.
5. Dispatch agents from fork files.
6. Sync completed gates back to plans.
7. Update `WAVE.md` and `PHASES.md`.

## IceLines Gates

Use focused gates first, then broader gates at checkpoints:

```powershell
cargo fmt --check
cargo check --workspace
cargo test -p icelines-core
cargo test -p icelines-cli
cargo test -p icelines-web
powershell -ExecutionPolicy Bypass -File scripts/release-smoke.ps1
```

Prefer existing slice commands when available:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 list
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 quick
```

## Rules

- Do not make agents rely on chat history.
- Do not make one pulse own unrelated surfaces.
- Do not invent surface-local scoring/projection logic; use ViewModels.
- Do not mark a wave closed while pulse gates are unchecked.
