# Organization Window real-scenario evidence — 2026-27

This evidence set exercises The Window with saved IceLines authorities rather
than synthetic profile values. It is modeled preseason evidence, not an
observed future result or a calibrated probability claim.

## Artifacts

- `organization-window-board-multisource-baseline-2026-27.json` is the sealed
  32-team baseline built from the existing baseline team-season forecast, NYR
  and SEA lineup projections, and the complete league training-camp forecast.
- `organization-window-board-multisource-nyr-development-2026-27.json` replaces
  the baseline team-season forecast with the saved NYR development/downturn
  scenario while holding the other source documents constant.
- `organization-window-impact-nyr-development-2026-27.json` is the typed
  deterministic comparison. It retains eight event authorities and records
  direct raw-input, cohort, and unchanged attribution across all 32 teams.
- `icecast-nyr-development-isolated-impact-2026-27.json` is the compact output
  of paired same-seed 1,000-trial event isolation. Its canonical input
  fingerprint is
  `b5a0128708edfe453d99814b2711960e66721cb1473aaed6ef73400796af779f`.
- `organization-window-scenario-distribution-input-nyr-sea-2026-27.json`
  combines those eight isolated NYR conditional point effects with Oscar
  Fisker Mølgaard's SEA camp realization. The SEA shock is mean-centered around
  the upstream make probability rather than treated as free upside.
- `organization-window-scenario-distribution-nyr-sea-2026-27.json` is the
  seeded 1,000-trial full-cohort result.

The baseline board fingerprint is
`c5b3debaea84c3b45f2aecf3daaf8a29607ee5ff12e59637a993105ea5d5838a`;
the deterministic scenario board fingerprint is
`3fd0e33117753baf7ef32fae5497b6224396bc33e677b9f421625e8a344d4a86`;
and the distribution fingerprint is
`72983785f6f1b9c7968099e0aeb215d266aaaf861281bf4c725195f4ecd5d3bd`.

## Reproduction

```text
icelines icecast window-build --season 20262027 --as-of 2026-07-27 --generated-at 2026-07-28T12:00:00-07:00 --team-season-forecast examples/icecast-nyr-line-baseline-season.json --team-lineup examples/team-lineup-nyr-2026-27.json --team-lineup examples/team-lineup-sea-2026-27.json --training-camp examples/icecast-league-training-camp-2026-27.json --out examples/organization-window-board-multisource-baseline-2026-27.json

icelines icecast window-scenario --baseline examples/organization-window-board-multisource-baseline-2026-27.json --scenario examples/organization-window-board-multisource-nyr-development-2026-27.json --scenario-id nyr-development-variance --team-season-authority examples/icecast-nyr-development-variance-10000-result.json --out examples/organization-window-impact-nyr-development-2026-27.json

icelines icecast window-scenario-distribute --baseline examples/organization-window-board-multisource-baseline-2026-27.json --input examples/organization-window-scenario-distribution-input-nyr-sea-2026-27.json --out examples/organization-window-scenario-distribution-nyr-sea-2026-27.json
```

## Result and limits

At seed `20260728`, NYR's overall Window delta has mean `-2.728`, P10
`-10.264`, median `0`, P90 `4.106`, positive probability `0.232`, and negative
probability `0.470`. SEA's mean is `0.284`, median `0`, P90 `1.173`, and
positive probability `0.242`.

The deterministic NYR scenario changes expected points by `-0.3575` but leaves
the normalized score and overall Window unchanged because NYR does not cross a
league percentile boundary. That is disclosed model behavior, not a missing
delta. Required source gaps still withhold all-team ranks and unavailable pane
distributions; no missing score is replaced with zero. Trial noise, source
uncertainty, and future season variation remain distinct limitations.
