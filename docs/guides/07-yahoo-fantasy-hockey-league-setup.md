# Set Up a Yahoo Fantasy Hockey League

This is a quick commissioner guide for creating a friendly, competitive Yahoo
Fantasy Hockey league. The custom settings below reproduce the **14-team
head-to-head points format used by the existing Kraken league**. IceLines is
only hosting this shareable guide.

Use a desktop browser if possible. Yahoo exposes more commissioner and draft
controls there than it does in the mobile app.

## 1. Create the league

1. Sign in at [Yahoo Fantasy Hockey](https://hockey.fantasysports.yahoo.com/).
2. Select **Create a league**.
3. Choose **Private League** and **Start from scratch**.
4. Give the league a name.
5. Choose **Head-to-Head Points** as the scoring type.
6. Choose **Live Standard Draft**.
7. Open **Customize** before creating the league.

Yahoo's official flow is also documented in
[Create and customize a Private League](https://help.yahoo.com/kb/fantasy-hockey/create-customize-private-league-sln25711.html).

## 2. Enter the league settings

Under **Commissioner → League Settings → Edit League Settings**, use:

| Setting | Recommended value |
|---|---|
| Maximum teams | **14** |
| Roster changes | **Daily – Today** |
| Maximum acquisitions per week | **4** |
| Maximum acquisitions per season | **No maximum** |
| Waiver mode | **Standard** |
| Waiver time | **2 days** |
| Waiver priority | **Continual rolling list** |
| Post-draft players | **Follow waiver rules** |
| Trade review | **League votes** |
| Trade rejection time | **2 days** |
| Can't Cut List | **Yahoo Sports** |
| Playoff teams | **6** |
| Playoff weeks | **Weeks 23–25** |
| Playoff reseeding | **Enabled** |
| Playoff tiebreaker | **Higher seed wins** |

Leave the trade deadline at Yahoo's current-season default unless the league
agrees on a different date. Ending playoffs before the final NHL weeks helps
avoid late-season rest and shutdown chaos.

## 3. Set the roster positions

Under **Commissioner → Rosters & Scoring → Edit Roster Positions**, enter:

| Position | Slots |
|---|---:|
| Center (C) | 2 |
| Left Wing (LW) | 2 |
| Right Wing (RW) | 2 |
| Defense (D) | 3 |
| Utility (Util) | 1 |
| Goalie (G) | 2 |
| Bench (BN) | 4 |
| Injured Reserve (IR) | 2 |
| Injured Reserve Plus (IR+) | 2 |

That creates **12 active slots plus four bench spots**. IR and IR+ are injury
storage and are not ordinary draft targets.

Double-check this before the draft. Yahoo allows some post-draft roster changes,
but removing or restructuring slots can require resetting the draft. See
[Yahoo's roster-position warning](https://help.yahoo.com/kb/fantasy-hockey/sln6941.html).

## 4. Enter the points scoring

First, make a league decision:

- **Use the Kraken league's custom weights:** preserves the established format
  and makes hits, blocks, special-teams production, and goalie volume matter in
  its particular way. Use the tables below.
- **Use Yahoo's standard Head-to-Head Points scoring:** simpler for new managers
  and better aligned with Yahoo's general rankings and advice. Leave Yahoo's
  default scoring weights unchanged and skip the custom tables below.

Announce this choice before managers prepare for the draft. If the league does
not strongly prefer the established Kraken format, Yahoo standard is the
simpler commissioner choice.

Under **Commissioner → Rosters & Scoring → Scoring Settings**, use these exact
weights only when choosing the custom Kraken format.

### Skaters

| Statistic | Points |
|---|---:|
| Goal (G) | **3.25** |
| Assist (A) | **2.25** |
| Power-Play Goal (PPG) | **3.00** |
| Power-Play Assist (PPA) | **2.00** |
| Short-Handed Goal (SHG) | **1.00** |
| Short-Handed Assist (SHA) | **1.00** |
| Game-Winning Goal (GWG) | **1.00** |
| Hit (HIT) | **0.50** |
| Block (BLK) | **0.50** |

### Goalies

| Statistic | Points |
|---|---:|
| Win (W) | **3.00** |
| Loss (L) | **-0.50** |
| Save (SV) | **0.20** |
| Goal Against (GA) | **-0.25** |
| Shutout (SHO) | **3.00** |

Remove or set to zero any statistic not listed above. Yahoo adds bonuses to the
base event: for example, a power-play goal earns both the Goal and Power-Play
Goal values.

### Important: custom scoring and Yahoo rankings

Custom weights change which players are actually most valuable. Yahoo displays
several different ranking fields, and they should not be treated as
interchangeable:

- Yahoo says its **Expert Rank** uses the game's default scoring and determines
  autopick order.
- Yahoo separately describes a pre-draft **Rank** as league-aware, but also says
  its **Default Rank** is unavailable in Head-to-Head Points leagues.
- Therefore, managers in the custom Kraken format should not assume Yahoo's
  visible player order or autopick order perfectly reflects these weights.

For a custom-scoring league, tell every manager to use a cheat sheet calculated
with the league's weights and to set personal pre-draft rankings, especially if
they may miss picks. For a low-preparation league where everyone expects to
follow Yahoo's rankings, use Yahoo's standard scoring instead.

See [Yahoo's explanation of player ranks](https://help.yahoo.com/kb/SLN6287.html)
and [how to set personal pre-draft rankings](https://help.yahoo.com/kb/fantasy-hockey/autopick-draft-sln6163.html).

## 5. Configure the draft

Under **Commissioner → Draft & Keepers → Edit Draft Settings**:

| Setting | Recommended value |
|---|---|
| Draft type | **Live Standard** |
| Round rotation | **Snaking** |
| Pick time | **60 seconds**; use **45 seconds** for an experienced, fast-moving league |
| Draft order | Random, or a method announced before the draft |

Pick a date when all managers can attend and state the time zone in the league
message. Wait until all 14 managers have joined before finalizing the team list
and draft order.

**League discussion point:** agree on either 45 or 60 seconds before draft day.
Forty-five seconds keeps the draft moving but gives managers less recovery time
after a surprise pick or connection problem. Sixty seconds is the safer default
for a 14-team league without making the draft unnecessarily slow.

In a 14-team snake, the manager drafting 14th also drafts 15th, then 42nd and
43rd, then 70th and 71st. The same turn pattern continues throughout the draft.

Yahoo's commissioner controls for timing, order, pausing, and undoing picks are
covered in [Manage your Private League's draft](https://help.yahoo.com/kb/fantasy-football/manage-private-leagues-draft-sln6086.html).

## 6. Invite everyone

1. Open **Commissioner → League Settings → Send Invites**.
2. Invite each manager using the email address connected to their Yahoo account.
3. Ask everyone to set a team name and verify that they can open the league.
4. Send a reminder several days before the draft.
5. Finalize the team list only after everyone has joined.

## Final commissioner checklist

Before draft day, verify:

- [ ] There are exactly 14 teams.
- [ ] Scoring type is Head-to-Head Points.
- [ ] The league chose either Yahoo standard scoring or the custom Kraken
      scoring before managers prepared for the draft.
- [ ] If using custom scoring, every scoring weight matches the tables above.
- [ ] If using custom scoring, managers were warned about Yahoo's ranking and
      autopick limitations and received time to create personal rankings.
- [ ] The active roster is 2 C / 2 LW / 2 RW / 3 D / 1 Util / 2 G.
- [ ] There are four bench, two IR, and two IR+ slots.
- [ ] Managers get four acquisitions per week.
- [ ] Waivers last two days and use a continual rolling priority.
- [ ] The draft is Live Standard and snaking, with the league's agreed 45- or
      60-second pick timer.
- [ ] The draft date, time, and time zone are visible to everyone.
- [ ] All managers have joined before the team list is finalized.
- [ ] Playoff settings and tiebreakers are visible in the league rules.

After creating the league, send everyone the Yahoo league invitation plus a
link to the league's **Settings** page. That gives managers one place to verify
the rules before drafting.

## Yahoo references

- [Create and customize a Private League](https://help.yahoo.com/kb/fantasy-hockey/create-customize-private-league-sln25711.html)
- [Commissioner tools and settings](https://ca.help.yahoo.com/kb/fantasy-football/league-settings-sln7834.html)
- [Default Fantasy Hockey settings](https://help.yahoo.com/kb/SLN6815.html)
- [About player ranks in Yahoo Fantasy](https://help.yahoo.com/kb/SLN6287.html)
- [Set personal pre-draft rankings](https://help.yahoo.com/kb/fantasy-hockey/autopick-draft-sln6163.html)
- [Fantasy Hockey playoff settings](https://help.yahoo.com/kb/SLN6833.html)
