# Phase Tigers - Docs boundary truth gate

## Status

Closed - 2026-06-21

## Goal

Keep Web `/docs` aligned with two active boundary claims:

- persistent report-toggle writes are not a Web admin mutation and remain a TUI
  durable-config handoff
- the removed mkdocs/static-site frontend is not an active CLI or Web docs
  surface

## Work

- Added an L1 `/docs` route assertion for the persistent report-toggle boundary
  copy.
- Added the same route assertion for the removed static-site frontend copy.

## Validation

```powershell
cargo fmt --check
cargo test -p icelines-web --test l1_router docs
git diff --check
```
