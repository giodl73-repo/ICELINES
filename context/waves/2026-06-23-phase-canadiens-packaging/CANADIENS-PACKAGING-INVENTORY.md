# Phase Canadiens Packaging Inventory

## Surfaces

| Surface | File | Result |
|---|---|---|
| Local package script | `scripts/package-release.ps1` | Builds or reuses `target\release\icelines.exe`, creates `dist\release\icelines-windows-x86_64.zip`, writes `ICELINES-PACKAGE.txt`, verifies archive membership, and can run release smoke. |
| Release checklist | `design/release-checklist.md` | Documents the local packaging command and artifact check. |
| Command reference | `COMMANDS.md` | Lists the local package helper with release smoke commands. |
| Plan index | `design/plans/INDEX.md` | Records this as a closed Canadiens packaging slice under the active major-stats roadmap. |
| Wave index | `design/waves/PHASES.md` | Records repo-local execution evidence for the slice. |

## Non-Claims

- GitHub Actions remains the cross-platform release builder.
- The local script currently packages the Windows x64 zip only.
- No tag, GitHub Release, version bump, or changelog release heading is created.
- Seeded demo profiles, installer/update UX, public API docs, and freshness
  diagnostics remain future packaging roadmap work.

## Validation

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1 -SkipBuild -SkipSmoke
git diff --check
```
