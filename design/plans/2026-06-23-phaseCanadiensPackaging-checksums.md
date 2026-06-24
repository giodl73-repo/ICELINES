# Phase Canadiens Packaging - Checksums

## Status

Closed - 2026-06-23

## Goal

Make release artifacts easier to verify by publishing checksum sidecars from
both local Windows packaging and the canonical GitHub release workflow.

## Scope

- Add SHA-256 sidecar generation to `scripts/package-release.ps1`.
- Record the packaged binary SHA-256 inside `ICELINES-PACKAGE.txt`.
- Generate and verify `.sha256` sidecars for every GitHub release matrix
  archive before upload.
- Include `ICELINES-PACKAGE.txt` in every GitHub release archive and verify its
  membership before upload.
- Publish the `.sha256` files with the GitHub Release assets.
- Add `scripts/verify-release-artifact.ps1` so downloaded `.zip` and `.tar.gz`
  artifacts can be checked against their sidecar, required archive members, and
  manifest-recorded binary SHA-256.
- Document checksum expectations in `README.md`, `COMMANDS.md`, and the release
  checklist.

## Non-Claims

- This does not sign release artifacts.
- This does not replace the GitHub release workflow or add local macOS/Linux
  packaging.
- This does not change binary contents, release versioning, or smoke coverage.

## Validation

```powershell
powershell -ExecutionPolicy Bypass -File scripts\package-release.ps1 -SkipBuild -SkipSmoke
powershell -ExecutionPolicy Bypass -File scripts\verify-release-artifact.ps1 -ArtifactPath dist\release\icelines-windows-x86_64.zip
git diff --check
```
