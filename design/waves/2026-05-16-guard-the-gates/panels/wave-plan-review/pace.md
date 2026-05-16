# PACE Review - Guard the Gates

## Findings

- Audit is a security gate, not a performance benchmark. Its runtime cost is
  acceptable if it is isolated from the fast compile/test slices and documented
  in the CI/local slice map.
- Do not add cargo-audit to every developer quick path unless the measured cost
  is small. `ci-audit` plus the full `ci` sequence is enough for this wave.

- pace
