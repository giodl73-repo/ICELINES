# IceLines Command Reference

<!-- proof:figure id="commands-table" kind="table.reference" -->
| Command | Description | Data required |
|---------|-------------|---------------|
| `icelines fetch rosters` | Fetch all 32 team rosters (headshots, positions) | Live API |
| `icelines fetch stats` | Fetch skater season stats (G, A, GP, TOI, PP, SH) | Live API |
| `icelines fetch realtime` | Fetch physical stats (hits, blocks, giveaways, takeaways, PIM) | Live API |
| `icelines fetch contracts` | Fetch contract status (expiry year, UFA/RFA/ELC type) | Live API |
| `icelines fetch money-puck` | Download MoneyPuck xG, CF%, FF%, xGF% CSV | Live + MoneyPuck |
| `icelines fetch all` | Run rosters + stats + realtime in one pass | Live API |
| `icelines rank` | Top-N players by pts-pace with optional position filter | Bundled |
| `icelines team <ABBR>` | Team depth chart — 4×3 forwards, 3×2 defense | Bundled |
| `icelines players` | Filtered player list with demographic + stat filters | Bundled |
| `icelines query leaders` | Ranked leaderboard — 30+ sort metrics, all filters | Bundled |
| `icelines query player` | Deep player profile — career arc, percentile, contract | Bundled |
| `icelines query compare` | Head-to-head comparison or Z-score similarity search | Bundled |
| `icelines history` | Season-by-season career statistics | Bundled |
| `icelines project` | Rest-of-season projection (pace / regressed / composite) | Bundled |
| `icelines scouting` | Full 8-section scouting report (terminal or JSON) | Bundled |
| `icelines peers` | Players from same draft class ± 1 year at same position | Bundled |
| `icelines class` | Entire draft class ranked by production | Bundled |
| `icelines compare` | Side-by-side stat comparison | Bundled |
| `icelines mates` | Linemates by shared ice time | Shifts (fetch shifts) |
| `icelines tonight` | Tonight's NHL games with UTC start times | Live API |
| `icelines schedule` | Upcoming schedule (N days) | Live API |
| `icelines trade` | Depth chart impact of a player trade | Bundled |
| `icelines tui` | Interactive Jack Adams dashboard with command bar | Bundled |
| `icelines fantasy league-create` | Create a new fantasy league | Local SQLite |
| `icelines fantasy team-create` | Create a team in the active league | Local SQLite |
| `icelines fantasy team-add` | Add a player to a team | Local SQLite |
| `icelines fantasy team-drop` | Drop a player from a team | Local SQLite |
| `icelines fantasy team-show` | Team roster with per-player fantasy scores | Local SQLite |
| `icelines fantasy standings` | League standings ranked by total score | Local SQLite |
| `icelines fantasy trade` | Trade simulation (before/after) or execution | Local SQLite |
| `icelines fantasy serve` | Start HTTP dashboard server (default port 8080) | Local SQLite |
| `icelines data install` | Download historical season bundle from GitHub Releases | Network |
| `icelines data list` | List installed seasons | Local |
| `icelines scheme list` | List all available fantasy scoring schemes | Built-in |
| `icelines scheme show` | Show scoring weights for a scheme | Built-in |
| `icelines build` | Generate mkdocs site from current snapshot | Bundled |
| `icelines serve` | Build and serve site locally | Bundled |
| `icelines deploy` | Deploy site to GitHub Pages | Git |
