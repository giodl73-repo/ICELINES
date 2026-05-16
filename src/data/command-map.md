# IceLines Command Map

Complete reference for all `icelines` commands — designed for AI referencing via `md://`.

---

<!-- proof:figure id="tui-screens" kind="table.reference" -->
## TUI Screens

Launch with `icelines tui` (or just `icelines` with no args).

| Screen | Key | Description |
|--------|-----|-------------|
| Home | *(launch)* | 32 teams ranked by aggregate pace score — two columns, color-coded #1–5 green, #6–10 cyan, #28–32 red |
| Team | `Enter` on a team | Depth chart for selected team — 4×3 forward grid + 3×2 defense pairs with fit colors |
| Player | `Enter` on a player | Player profile — stats, pace score, bio, draft info |
| Search | `/` | Fuzzy search all players by name |
| Tonight | `Tab` → Tonight | Live NHL schedule for today (NHL API) |
| Projections | `Tab` → Projections | Rest-of-season projections stub |
| Groups | `Tab` → Groups | Player watchlist management |
| Fetch | `Tab` → Fetch | Data fetch status and controls |
| Help | `?` | Key binding reference overlay |

**TUI navigation keys:**

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move selection |
| `Enter` | Activate selected item |
| `/` | Open search |
| `Tab` | Cycle between screens |
| `Esc` / `Backspace` | Go back |
| `r` | Refresh |
| `?` | Toggle help overlay |
| `q` | Quit |

---

<!-- proof:figure id="query-commands" kind="table.reference" -->
## icelines query

FLETCH handoff note: `icelines fetch fletch-partitions --gate` maps these
query surfaces to durable FLETCH partition and rollup IDs, and
`icelines fetch fletch-quivers --gate` groups those partitions into query
bootstrap/enrichment quiver candidates. These are local reports: ICELINES
sealed snapshots and active pointers remain the query activation evidence.

### query leaders

Ranked leaderboard. All filter flags combine with AND logic.

```
icelines query leaders [flags]
```

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--sort` | string | `pts-pace` | Sort metric (see sort metrics table) |
| `--top` | int | 20 | Number of results |
| `--seasons` | int | 1 | Aggregate across N bundled seasons |
| `--pos` | string | — | C, LW, RW, D, F (all fwd), G |
| `--team` | string | — | Team abbreviation (EDM, SEA, …) |
| `--age-min` | int | — | Minimum age |
| `--age-max` | int | — | Maximum age |
| `--nationality` | string | — | ISO-3166 alpha-3 (FIN, SWE, CAN, …) |
| `--birth-province` | string | — | Province/state codes, comma-separated |
| `--draft-year` | int | — | Draft year |
| `--draft-round` | int | — | Draft round (1–7) |
| `--draft-pick-max` | int | — | Max overall pick number |
| `--undrafted` | flag | — | Only undrafted players |
| `--rookie` | flag | — | Only first NHL season |
| `--handedness` | L or R | — | Shooting hand |
| `--ppg-min` | float | — | Min PPG per game (e.g. 0.80) |
| `--gp-min` | int | — | Min games played |
| `--gp-max` | int | — | Max games played |
| `--toi-min` | float | — | Min TOI/game in minutes |
| `--plus-minus-min` | int | — | Min +/- rating |
| `--shots-pg-min` | float | — | Min shots per game |
| `--ufa` | flag | — | UFA contracts only (requires fetch contracts) |
| `--rfa` | flag | — | RFA contracts only |
| `--elc` | flag | — | Entry-level contracts only |
| `--expiry-year` | int | — | Contract expiring this year |
| `--percentiles` | flag | — | Show league percentile per result |
| `--rate` | flag | — | Show per-game rates instead of per-82 |
| `--json` | flag | — | Export as JSON array |
| `--csv` | flag | — | Export as CSV |

### query player

```
icelines query player <name> [--breakdown career|situation] [--percentiles] [--last-n N]
```

Shows current season stats (G, A, PP, SH, GWG, shots, SH%, +/-, TOI, FO%), league rank, career arc.

### query compare

```
icelines query compare <player1> [player2]   # head-to-head
icelines query compare <player1> --similar N  # Z-score similarity search
```

---

<!-- proof:figure id="sort-metrics-map" kind="table.reference" -->
## Sort Metrics

All available values for `--sort`:

| Key | Description | Data required |
|-----|-------------|---------------|
| `pts-pace` | Points per 82 games *(default)* | Bundled |
| `ppg` | Points per game | Bundled |
| `g-pace` | Goals per 82 | Bundled |
| `gpg` | Goals per game | Bundled |
| `pts` | Raw season points | Bundled |
| `goals` | Raw season goals | Bundled |
| `assists` | Raw season assists | Bundled |
| `gp` | Games played | Bundled |
| `pp-pts-pace` | PP points per 82 | Bundled |
| `pp-g-pace` | PP goals per 82 | Bundled |
| `pp-pts` | PP points (total) | Bundled |
| `pp-g` | PP goals (total) | Bundled |
| `sh-g-pace` | SH goals per 82 | Bundled |
| `sh-g` | SH goals (total) | Bundled |
| `gwg-pace` | Game-winning goals per 82 | Bundled |
| `gwg` | GWG (total) | Bundled |
| `shots-pace` | Shots per 82 | Bundled |
| `shots` | Shots (total) | Bundled |
| `sh-pct` | Shooting percentage | Bundled |
| `plus-minus` | +/- rating | Bundled |
| `toi` | TOI per game | Bundled |
| `fo-pct` | Faceoff win % | Bundled |
| `hits-pace` | Hits per 82 | fetch realtime |
| `hits` | Hits (total) | fetch realtime |
| `blocks-pace` | Blocked shots per 82 | fetch realtime |
| `blocks` | Blocked shots (total) | fetch realtime |
| `takeaways` | Takeaways | fetch realtime |
| `giveaways` | Giveaways | fetch realtime |
| `pim` | Penalty minutes | fetch realtime |
| `xg` | Individual expected goals | fetch money-puck |
| `xg-per-60` | ixG per 60 min | fetch money-puck |
| `cf-pct` | Corsi For % at 5v5 | fetch money-puck |
| `ff-pct` | Fenwick For % at 5v5 | fetch money-puck |
| `xgf-pct` | xGoals For % at 5v5 | fetch money-puck |
| `improvement` | Y/Y PPG delta vs prior season | Bundled (2 seasons) |

---

<!-- proof:figure id="fetch-commands" kind="table.reference" -->
## icelines fetch

| Command | Description |
|---------|-------------|
| `fetch all` | Fetch rosters + stats in one pass |
| `fetch rosters [--season S] [--refresh] [--dry-run]` | All 32 team rosters; source bytes acquired through FLETCH |
| `fetch stats [--season S] [--refresh] [--dry-run]` | Bios + summary stats |
| `fetch realtime [--season S] [--dry-run]` | Hits, blocks, giveaways, takeaways, PIM |
| `fetch contracts [--dry-run]` | UFA/RFA/ELC expiry data |
| `fetch money-puck [--season S] [--dry-run]` | xG, CF%, FF%, xGF% CSV; source bytes acquired through FLETCH |
| `fetch fletch-sources [--season S] [--type regular\|playoff\|both] [--out PATH] [--gate]` | FLETCH handoff inventory and gate |
| `fetch positions [--season S] [--dry-run]` | Boxscore-derived position eligibility |

---

<!-- proof:figure id="team-commands" kind="table.reference" -->
## Team and player analysis

| Command | Description |
|---------|-------------|
| `team <ABBR> [--no-color]` | Depth chart — 4×3 forward grid, 3×2 defense pairs, fit colors |
| `rank [--top N] [--pos P] [--scheme S]` | Top-N by pts-pace with color fit labels |
| `players [--pos P] [--team T] [--age-max N] [--ppg-min F] [--gp-min N] [--top N]` | Filtered player list |
| `history <player> [--json]` | Season-by-season career stats |
| `project <player\|--team T> [--mode pace\|regressed\|composite] [--games N]` | Rest-of-season projection |
| `scouting <player> [--format terminal\|markdown\|json]` | 8-section scouting report |
| `peers <player> [--size N] [--json]` | Draft class ± 1 year peers, ranked |
| `class <year> [--pos P] [--top N] [--json]` | Full draft class ranked by production |
| `compare <player1> <player2> [--json]` | Side-by-side stat comparison |
| `mates <player> [--top N]` | Linemate-style roster fallback; shift bundles parked |
| `trade <player_out> for <player_in> [--team T]` | Depth chart before/after a trade |
| `tonight [--team T]` | Tonight's NHL games (live API) |
| `schedule [--team T] [--days N]` | Upcoming schedule |

---

<!-- proof:figure id="fantasy-commands" kind="table.reference" -->
## icelines fantasy

### League management

| Command | Description |
|---------|-------------|
| `fantasy league-create <name> [--scheme yahoo-standard\|espn-standard\|simple-pts]` | Create league (auto-activates) |
| `fantasy league-list` | List all leagues, mark active |
| `fantasy league-use <name>` | Switch active league |
| `fantasy league-switch <name>` | Alias for league-use |
| `fantasy league-delete <name>` | Delete league + all teams (cascade) |

### Team management

| Command | Description |
|---------|-------------|
| `fantasy team-create <name> [--owner O] [--league L]` | Create team in active league |
| `fantasy team-list [--league L]` | List teams in active league |
| `fantasy team-show <name> [--league L]` | Roster with per-player fantasy scores |
| `fantasy team-add <team> <player> [--league L]` | Add player (fuzzy match) |
| `fantasy team-drop <team> <player> [--league L]` | Drop player |

### Scoring and trades

| Command | Description |
|---------|-------------|
| `fantasy standings [--league L] [--scheme S]` | All teams ranked by score |
| `fantasy trade <player1> --to-team <team2> --for-player <player2> [--execute] [--league L]` | Simulate or execute trade |

### Web server

| Command | Description |
|---------|-------------|
| `fantasy serve [--port 8080] [--league L]` | HTTP dashboard server |

**API routes:** `GET /` HTML · `GET /api/standings` · `GET /api/teams` · `GET /api/team/:name/roster` · `POST /api/team/:name/add` · `POST /api/team/:name/drop` · `POST /api/trade`

---

<!-- proof:figure id="data-commands" kind="table.reference" -->
## icelines data

| Command | Description |
|---------|-------------|
| `data install [--seasons N] [--season YYYYZZZZ] [--force]` | Download season bundle(s) from GitHub Releases |
| `data list` | Show installed seasons with sizes and player counts |
| `data remove <season>` | Uninstall a season |

**38 seasons available:** 19871988 → 20252026 (skip 20042005 lockout)

---

<!-- proof:figure id="utility-commands" kind="table.reference" -->
## Utility commands

| Command | Description |
|---------|-------------|
| `group create <name> [--desc D]` | Create player watchlist |
| `group add <group> <player>` | Add player to group |
| `group remove <group> <player>` | Remove player from group |
| `group list` | List all groups |
| `group show <name>` | Show members with stats |
| `group delete <name>` | Delete group |
| `scheme list` | List built-in + custom scoring schemes |
| `scheme show <name>` | Show scheme weights |
| `snapshot list` | All snapshots with tier, date, sealed status |
| `snapshot show <name>` | Full detail for a named snapshot |
| `snapshot verify [name]` | Re-verify SHA-256 integrity |
| `snapshot use <name>` | Switch to a named snapshot |
| `snapshot delete <name>` | Delete a snapshot |
| `build [--no-site]` | Generate mkdocs site from snapshot |
| `serve [--port 8000]` | Serve site locally |
| `deploy [--remote origin]` | Deploy to GitHub Pages |
| `tui` | Launch interactive terminal UI |
| `dashboard` | Alias for tui |
