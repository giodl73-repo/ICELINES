# Goalie puck-event evidence

The six estimated goalie attributes already work from public season statistics.
Puck Play currently uses assists/60. This ledger adds event detail without
claiming that official game logs record every retrieval, pass or puck touch.

## Run

```powershell
icelines profile-puck-play --season 20262027 --cache-dir target/puck-cache --output-dir target/puck-run
# A bounded sample; games involving either club, deduplicated.
icelines profile-puck-play --season 20262027 --team NYR,SEA --cache-dir target/puck-cache --output-dir target/puck-sample
# Exact captured-source replay into a different output directory.
icelines --no-live profile-puck-play --season 20262027 --team NYR,SEA --cache-dir target/puck-cache --output-dir target/puck-replay
```

The default selects all NHL teams. Regular-season finalized games are discovered
from their official season schedules. Play-by-play downloads have four workers.
The report is written only after every selected game has been captured and parsed;
an unavailable game fails the run rather than becoming zero events. Existing
output directories are preserved. Finalized game captures are reused; a fresh
cache requests a new snapshot if later official corrections are needed.

## What counts

| Record | Official actor field | Interpretation |
| --- | --- | --- |
| Goal | scoringPlayerId | Credited goalie goal |
| Primary assist | assist1PlayerId | Credited first assist |
| Secondary assist | assist2PlayerId | Credited second assist |
| Giveaway | playerId | Officially credited giveaway |
| Takeaway | playerId | Officially credited takeaway |
| Penalty | committedByPlayerId | Goalie committed it; retain exact description |

Actors must appear in that game's roster with position G and agree with the event
team. Being the goalie facing a goal does not make the goalie an offensive actor.
Drawn and served penalties are not committed penalties. Team penalties without
a named committing player are not assigned to a goalie. Shootout plays are excluded.

For a giveaway, scan forward in official sort order for the first opposing shot
on goal or goal within ten seconds in the same period. Stop at a stoppage,
faceoff, assessed penalty, goal or period/game end. Store its event ID and elapsed
seconds. This is temporal context, **not** a claim that the giveaway caused the
attempt or that the opponent possessed the puck throughout that interval.
Missed and blocked shots are not counted as shots on goal.

## Profile use

The player card now has a compact **Puck Record** inside Distinctive Features:
goals, primary/secondary assists, giveaways, takeaways and committed penalties.
Expand **Games & penalties** for penalty descriptions, game/period/clock,
official event-source links and the following-attempt context. Zero remains zero;
no matching captured ledger means no block. These are actual recorded counts,
separate from the Estimated Record's season pace.

The shared Rust projection recomputes counts from game-level events rather than
trusting the cached summary. It requires complete selected-game coverage and a
matching goalie/regular-season window. Skater and playoff cards do not receive
these records. Each block states its own capture date and selected-game coverage;
that sample is not the player's appearances or exposure denominator.

Native configuration:

```powershell
$env:ICELINES_RINKCRAFT_GOALIE_PUCK='C:/path/to/goalie-puck-events.json'
```

For the browser, stage the same report as
`icelines-browser/public/rinkcraft-goalie-puck-events.json` before the normal
local build, with WASM bindings generated from the matching source revision.
The file is optional and joins the offline shell when present. No network fetch
of game logs happens when opening a card: the worker projects the captured report.
The selected source snapshot remains explicit even when season statistics have a
different observation date.

These are measured event counts that can sit below Puck Play or support a future
specialty. There is no new 1–20 rating, feat unlock, pass-completion percentage or
giveaway-per-touch rate. A zero means no credited event in the captured games.
It does not mean no puck handling. Roster-game counts include backups and must
never become GP, minutes or a touch-opportunity denominator.

In particular, fewer giveaways/60 cannot by itself mean better handling: a goalie
who rarely plays the puck can have fewer giveaways than an active outlet passer.
Primary and secondary assists distinguish credited scoring involvement; neither
measures all completed passes. Keep penalty descriptions separate instead of
classifying every goalie penalty as a puck-handling error.

## Public source contract

Official endpoints:

- [NHL club season schedule example](https://api-web.nhle.com/v1/club-schedule-season/NYR/20262027)
- [NHL game play-by-play example](https://api-web.nhle.com/v1/gamecenter/2026020001/play-by-play)

The JSON report records source URLs, capture times and SHA-256 hashes; each event
retains game/event/player identities, period and clock. It reports expected and
captured game counts and the selected teams. Counts from a team sample are scoped
to those games, even when an opposing goalie appears in the output.

Technical handling measurements remain a later extension requiring recorded
touch/pass opportunities, outcomes and context. The public-event ledger makes
those missing denominators explicit without delaying the working profile.
