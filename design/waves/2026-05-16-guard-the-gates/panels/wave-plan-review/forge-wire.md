# FORGE/WIRE Review - Guard the Gates

## Findings

- Tool bootstrap is part of the contract. Do not assume `cargo-audit` is present
  on `windows-latest` or a developer machine.
- RustSec advisory data changes independently of this repository, so ignored
  advisories need explicit rationale and expiry/removal conditions.
- Avoid broad shell rewrites in the CI workflow. Add the smallest install/run
  path that preserves the current split matrix behavior.

- forge + wire
