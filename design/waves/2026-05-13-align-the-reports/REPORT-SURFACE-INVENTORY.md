# Report Surface Inventory

## Current report doors

| Door | Best for | Current formats |
|---|---|---|
| `query` | Asking stat/filter questions interactively. | table, JSON, CSV depending on subcommand |
| `x` | Fast CSV/JSON exports for Excel and scripts. | CSV default, JSON, `--out` |
| `export md` | Durable markdown report packets. | Markdown, `--out` |
| `report` | Durable fantasy decision reports and report discovery. | Markdown, JSON, `--out`, catalog JSON |
| direct commands | First-class workflows (`team-season`, `transactions`, `schedule`). | varies by command |

## Available canonical families

| Family | Canonical command(s) | Screen alignment |
|---|---|---|
| leaders | `query leaders`, `x leaders`, `export md leaders` | TUI Stats, web leaders |
| goalies | `query goalies`, `x goalies` | TUI Goalies |
| player/history | `query player`, `history`, `x history` | player card |
| compare | `query compare`, `x compare`, `export md compare` | compare handoff/web |
| team | `team`, `export md team` | team/depth screens |
| team-season | `team-season`, `export md team-season` | team season screen |
| fantasy poach | `poach`, `report poach`, `export md fantasy` | TUI Poach, web poach |
| weekly fantasy | `report weekly` | web weekly report |
| transactions | `transactions`, `x transactions` | TUI/web Transactions |

## Planned records family

The user-requested individual records should become a first-class `records`
family rather than ad-hoc filters. Candidate player records:

- NHL teams a player has scored against.
- Goalies a player has scored against.
- Players a player has fought.
- Opponents involved in symmetric head-to-head events.

Candidate team records:

- Players who scored against a team.
- Goalies beaten by a team.
- Fight opponents by team.
- Head-to-head opponent counts.

Data warning: teams-scored-against can come from game/goal records; goalie
scored-against and fight opponents need event-level boxscore/play-by-play data
with goalie-on-ice and penalty/fighting participants. This should be modeled in
core/fetch before exposing CLI/TUI/web renderers.
