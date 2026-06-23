# Phase Canadiens Packaging

## Mission

Advance the Canadiens roadmap production-packaging item by adding a local
release-artifact assembly path that mirrors the Windows artifact name from the
GitHub release workflow and verifies archive contents before publication.

## Scope

- Add `scripts/package-release.ps1`.
- Document the local package command in release docs.
- Keep GitHub Actions as the canonical cross-platform release builder.
- Avoid tagging, publishing, or broad release-version changes.

## Pulses

| Pulse | Status | Evidence |
|---|---|---|
| 01 | passed | Local Windows package script added and documented. |

## Closeout

Phase Canadiens Packaging is closed for the local Windows artifact slice. The
production-packaging roadmap still needs future work for seeded demo profiles,
public API docs, installer/update flows, and broader release UX.
