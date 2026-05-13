# Career and docs parity inventory

Pulse 08 reviewed the Calder/Ted Lindsay career and generated-doc surfaces
against the SCOUT, KEEL, BENCH, TAPE, and WIRE lenses.

## Surface inventory

| Surface | Contract | Status |
|---|---|---|
| `icelines query career --league ...` | Projects local `CareerHistoryStore` rows into `CareerView`, then CLI text/JSON/CSV rows. | Ready; missing local store now uses the shared fetch instruction. |
| `/career` | HTML cohort leaderboard backed by `CareerView` and the shared web page shell. | Ready; missing local store returns a 400 page with the same fetch instruction. |
| `/api/v1/career` | JSON twin for `/career`, stable data/meta or error/meta envelope. | Ready; missing local store returns the same explicit fetch instruction. |
| Dashboard career workspace | Embeds `/career?...` as the canonical web workspace target. | Ready; no separate data path. |
| TUI career affordance | Player cards show bundled NHL career arcs and local pre-NHL rows; MDI command bar parses `career` cohort args. | Handoff-only by design. The command bar flashes the canonical `query career` and `/career` targets instead of duplicating the cohort table. |
| `/docs` | Renders embedded `COMMANDS.md` through `DocsView`. | Ready; docs route now has a fence proving the career fetch prerequisite is visible. |

## TUI board decision

No dedicated cross-league cohort TUI board was added in this pulse. `CareerView`
already exposes the cohort table needed by CLI and web, but the TUI has no
additional long-lived state or player-card fields that would make a separate
board more useful than the canonical one-shot CLI/web surfaces. Per KEEL, this
avoids duplicating renderer logic across surfaces; per TAPE/WIRE, it avoids
implying that cold installs include non-bundled career-history data.

## Verification notes

- The career-history store remains intentionally unbundled.
- Cold installs must show `icelines fetch career --bundled-seasons 5` rather
  than an empty-success cohort.
- `surface-parity.md`, `COMMANDS.md`, and `README.md` now name the CLI/web
  handoff and local-store prerequisite.
