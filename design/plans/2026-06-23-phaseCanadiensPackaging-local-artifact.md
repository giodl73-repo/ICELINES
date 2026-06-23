# Phase Canadiens Packaging - Local artifact

## Status

Closed - 2026-06-23

## Goal

Make local Windows release artifact assembly inspectable without relying only on
the tag-triggered GitHub Actions release workflow.

## Scope

- Add a local packaging script that builds or reuses the release binary.
- Create the canonical Windows artifact name, `icelines-windows-x86_64.zip`.
- Include the binary plus a compact package manifest with version, commit, and
  build timestamp.
- Verify the archive contains the expected binary and manifest.
- Optionally run the existing release smoke against the packaged binary.

## Non-Claims

- This does not replace the GitHub release workflow.
- This does not build macOS or Linux artifacts locally.
- This does not tag or publish a release.

## Validation

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-release.ps1 -SkipBuild -SkipSmoke
git diff --check
```
