# Rinkcraft theme studio

Local opt-in design playground, not a default application redesign or rating
release. Six treatments share the existing compact profile renderer:

| Treatment | Structural and typographic direction |
|---|---|
| Ice Sheet | Current compact form, rink rules, vertical attribute rail |
| Guild Ledger | Serif type, parchment, double borders, shield fields |
| Scouting Bureau | Monospaced records, numbered panels, ruled form and stamp |
| Tactical Board | Chalk/grid treatment and horizontal desktop attribute strip |
| Broadcast | Condensed athletic masthead, jersey stripes, cut-corner blocks |
| Editorial | Serif masthead/numerals, fine rules, mostly unboxed sections |

Theme cards, previous/next, a native range dial and keyboard arrows/brackets
select a look. Normal input/select/button keyboard behavior takes precedence.
The theme and selected example persist in the query string, so reloads and
copied links reproduce the selection. No automatic cycling or external fonts.

## Data boundary

The example selector offers explicit retained NHL fixtures and two authored
fictional design specimens, a defender and goalie. The default fictional card
fills ratings, task fields, counters, methods, tendencies, deployment and record
panels to exercise the design with dense data. Every number, percentile, duty
and relationship is illustrative; no reference cohort or calibrated game value
is implied. No real player name/ID is attached to those values.

Fictional status is visible in a banner, the card header, scope, attribute
capsules and footer; the card retains the label on printouts. Its distinct
`data-profile-kind="fictional_design_sample"` and `illustrative` rating state
do not enter the NHL renderer's assessment logic. No observation importer,
scorer, simulator parameter or research registry is changed. The real examples
retain their supplied statistics/documents and all six unrated capabilities.

This is browser presentation work, so CSS/TypeScript handle it; no Rust model
changes are needed. Shared profile/evidence calculations remain Rust.

## Local preparation

```text
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node tools/prepare_rinkcraft_theme_studio.mjs PROFILE_FIXTURES_JSON OUTPUT_DIRECTORY
node icelines-browser/e2e/rinkcraft-theme-studio.mjs
```

The builder takes an explicit local JSON input and writes an explicit output
directory. It records the input SHA256 and copies local render modules/assets;
no fetch, data refresh, app-server restart or hosted publication. The browser
loads only these local files. `ICELINES_THEME_URL` can select the served preview.

Acceptance covers six themes × seven samples (42 desktop cases), 18 mobile
cases, all profile text preserved during theme changes, six correct rating
states, horizontal-versus-vertical attribute geometry, controls and query reload.
Print normalizes ink/background and compacts type to retain a Letter sheet.
Independent PDF checks must verify page count and the fictional label.

The retained NHL fixtures keep their own provenance and historic dates. New
theme screenshots/exports and provenance belong to PUCK's private local artifact
packet. The authored fictional specimens remain neutral design fixtures here.
