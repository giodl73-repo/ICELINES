# Getting Started with IceLines

IceLines is a Rust CLI for NHL analytics — depth charts, pace-adjusted rankings,
query engine, and fantasy league management. Five seasons of data ship in the
binary: no fetch required to start.

---

## Install

```bash
git clone https://github.com/giodl73-repo/ICELINES.git
cd ICELINES/src
cargo build --release
# Binary is at src/target/release/icelines (or icelines.exe on Windows)
```

Verify:

```bash
icelines --version
# icelines 0.1.0
```

---

## Your first query (30 seconds)

No fetch required — bundled data works immediately.

```bash
# Top 10 players by pts/82
icelines rank --top 10

# Top U23 centers by PPG
icelines query leaders --pos C --age-max 23 --sort ppg --top 10

# Team depth chart
icelines team EDM
```

---

## Data sources

IceLines uses the NHL public API exclusively:

```
api-web.nhle.com      — rosters, headshots, schedule
api.nhle.com/stats    — bios, stats, realtime, skating
moneypuck.com         — xG, CF%, FF%, xGF% (optional)
```

No account, no API key, no rate limit concerns for normal use.

---

## Fetch fresh data

The bundled data refreshes weekly in CI. To get the latest:

```bash
icelines fetch all          # rosters + stats (5-10 minutes)
icelines fetch realtime     # hits, blocks, giveaways (adds to stats)
icelines fetch money-puck   # MoneyPuck xG/CF% (optional, free)
```

After fetching, all queries automatically use the snapshot data instead of
the bundled data — no flag needed.

---

## Historical data

38 seasons are available — back to 1987-88 (Gretzky's first LA season):

```bash
icelines data install --seasons 5    # last 5 seasons
icelines data install --seasons 38   # full history 1987–2025
icelines data install --season 19931994   # specific season
```

Once installed, historical data is available for multi-season queries:

```bash
icelines query leaders --seasons 10 --pos C --sort pts-pace --top 10
```

---

## The interactive TUI

```bash
icelines tui
```

Default dashboard: scores ribbon, Favorites/watchlist pane, central workspace,
Schedule/context pane, and command bar. Use `icelines tui --classic` for the
older tabbed UI.

- `:` opens the command bar; try `stats`, `goalies`, `team EDM`,
  `query age<=24 AND p>=80`, `gaps cats=hits,blocks,shots top=8`,
  `poach rw cats=hits,blocks free top=12`, or `simulate add=Connor_McDavid
  drop=Bench_Forward weeks=3`.
- `Ctrl+H` toggles Favorites; `Ctrl+L` toggles Schedule.
- `y` opens the season picker for time-travel; `g`/`f` add players to
  groups; `?` shows the full key map.

Full walkthrough in [The Interactive TUI](06-tui.md).

---

## Next

- [Query Engine](01-query.md) — `query leaders`, `query player`, `query compare`
- [Team Depth Charts](02-team-depth.md) — fit classification, lineup cards
- [Fantasy League](03-fantasy.md) — leagues, scoring, trades
- [Data & History](04-data.md) — 38 seasons, MoneyPuck, contracts
- [The Interactive TUI](06-tui.md) — six tabs, season time-travel, live scores, schedule search
