use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

// ── Top-level CLI ─────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "icelines",
    about = "NHL fantasy depth-chart and ranking tool",
    long_about = r#"
icelines — NHL analytics + fantasy CLI with 38 seasons of history bundled in.

QUICK START
  icelines query leaders --top 10                       # current season scorers
  icelines query player "Connor McDavid" --seasons 38   # full bundled career
  icelines query compare "Wayne Gretzky" "Mario Lemieux" --seasons 38
  icelines query goalies --filter "save-pct>=0.92"
  icelines tui                                          # interactive dashboard

MULTI-FILTER PATTERNS
  --filter is repeatable and ANDed. Use canonical cli_keys (goals, points,
  blocked-shots, ...) or short aliases (g, p, blk, gp, ppg, sv%, ...).

  icelines query leaders --age-max 24 --filter "hits>=200" --filter "p>=40"
  icelines query leaders --seasons 3 --filter "g>=60" --filter "a>=60"

DOCS
  icelines docs                  Print the full command reference.
  icelines <subcommand> --help   Per-command help with examples.
  README.md and COMMANDS.md      Single-page references in the source tree.

FLAGS
  --no-live           Disable live NHL API calls (deterministic / offline).
  --no-dashboards     Hide the TUI dashboard side panel.
"#,
    version,
    propagate_version = true
)]
pub struct Cli {
    /// Disable all live NHL API fetches (Phase 8f.1).
    ///
    /// Suppresses Scores / Schedule / Playoffs / boxscore / play-by-play
    /// endpoints and the Scores auto-refresh timer.
    ///
    /// Useful for airplane mode, demos, and CI runs.
    ///
    /// Also settable via `ICELINES_NO_LIVE=1` or `live = false`
    /// in `~/.icelines/config.toml`.
    ///
    /// Precedence: CLI flag > env > config > default (live ON).
    #[arg(long, global = true)]
    pub no_live: bool,

    /// Disable the dashboard side panel on the TUI player card.
    ///
    /// The panel is on by default; pass this flag to suppress it.
    ///
    /// Also settable via `ICELINES_DASHBOARDS=0` or `dashboards =
    /// false` in `~/.icelines/config.toml`.
    ///
    /// Precedence: CLI flag > env > config > default (on).
    #[arg(long, global = true)]
    pub no_dashboards: bool,

    /// Phase Foster.0.8 — skip the auto-setup wizard.
    ///
    /// On first run with no config file, `icelines` opens the setup
    /// wizard. Headless / scripted callers pass this to bypass it.
    #[arg(long, global = true)]
    pub no_setup: bool,

    #[command(subcommand)]
    pub command: Commands,
}

// ── Phase 1 commands ──────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Fetch NHL data from the API or a CSV export.
    #[command(subcommand)]
    Fetch(FetchSubcommand),

    /// Show a team's depth chart.
    Team {
        /// Team abbreviation (e.g. NYR, TOR, SEA).
        team: String,

        /// Color scheme override.
        #[arg(long)]
        scheme: Option<String>,

        /// Disable ANSI color output.
        #[arg(long)]
        no_color: bool,
    },

    /// Show team season performance: record, splits, form, and remaining schedule.
    #[command(name = "team-season")]
    TeamSeason {
        /// Team abbreviation (e.g. EDM, TOR, SEA).
        team: String,

        /// Emit the shared TeamSeasonView as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Show the top-N ranked players by pace score.
    Rank {
        /// Number of players to show.
        #[arg(long, default_value_t = 20)]
        top: usize,

        /// Filter by position abbreviation (C, LW, RW, D).
        #[arg(long)]
        pos: Option<String>,

        /// Color scheme override.
        #[arg(long)]
        scheme: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Find fantasy add/stream/stash candidates from the poacher ViewModel.
    Poach {
        /// Season id, e.g. 20252026. Defaults to configured season.
        #[arg(long)]
        season: Option<String>,

        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,

        /// Fantasy scoring scheme name.
        #[arg(long, default_value = "yahoo-standard")]
        scheme: String,

        /// Comma-separated categories to emphasize, e.g. hits,blocks,shots.
        #[arg(long = "category", value_delimiter = ',')]
        categories: Vec<String>,

        /// Filter by team abbreviation.
        #[arg(long)]
        team: Vec<String>,

        /// Filter by position abbreviation: C, LW, RW, D.
        #[arg(long)]
        pos: Vec<String>,

        /// Filter by availability: any, available, imported-available,
        /// not-on-user-roster, watched, unknown.
        #[arg(long)]
        availability: Option<String>,

        /// Number of candidates to show.
        #[arg(long, default_value_t = 20)]
        top: u16,

        /// Emit the full PoachBoardView as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Generate decision reports from shared ViewModels.
    #[command(
        subcommand,
        long_about = r#"
Generate durable reports and discover the canonical report/export surfaces.

Use `icelines report list` when you are not sure whether a report belongs under
`query`, `x`, `export md`, or `report`. The list command also marks planned
families such as player/team records.

Examples:
  icelines report list
  icelines report list --json
  icelines report poach --category shots --top 10 --out poach.md
  icelines report weekly --league default --category hits,blocks --out weekly.md
"#
    )]
    Report(ReportSubcommand),

    /// Player/team individual records from event-level data.
    #[command(
        subcommand,
        long_about = r#"
Compute individual records from persisted event-level data.

The first available metrics use persisted boxscore goal rows:
  icelines records player "Andre Burakovsky" --metric teams-scored-against
  icelines records player "Andre Burakovsky" --metric goalies-scored-against
  icelines records player "Andre Burakovsky" --metric fight-opponents
  icelines records team EDM --metric players-scored-against-team
  icelines records team EDM --metric goalies-beaten-by-team
  icelines records team EDM --metric fight-opponents-by-team

Run `icelines fetch boxscore --date YYYY-MM-DD` to populate local boxscore
records. `icelines fetch play-by-play --date YYYY-MM-DD` populates the event
participants needed by goalie and fight metrics. Fight-opponent records use
explicit fighting-major participants, not aggregate PIM totals.
"#
    )]
    Records(RecordsSubcommand),

    /// Show a player's official NHL awards / Trophy Case from the landing API.
    #[command(long_about = r#"
Show a player's official NHL awards / Trophy Case.

Awards come from the NHL player landing endpoint (`awards[]`). They are not
inferred from leaderboard finishes, so voted trophies and playoff awards such as
the Conn Smythe stay authoritative.

Examples:
  icelines awards "Connor McDavid"
  icelines awards "Connor McDavid" --json
  icelines awards "Connor McDavid" --csv --out mcdavid-awards.csv
"#)]
    Awards {
        /// Player name or partial name.
        player: String,

        /// Emit the full PlayerAwardsView as JSON.
        #[arg(long)]
        json: bool,

        /// Emit CSV instead of a table.
        #[arg(long)]
        csv: bool,

        /// Write output to a file instead of stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Show a player's scoring and shot streaks from cached game lines.
    #[command(long_about = r#"
Show a player's goal, assist, point, shot-on-goal, and shot-attempt streaks from cached game lines.

Goal/assist/point streaks are computed from persisted boxscore skater rows.
Shot-on-goal and shot-attempt streaks are computed from official play-by-play
rows. Run `icelines fetch boxscore --date YYYY-MM-DD` and
`icelines fetch play-by-play --date YYYY-MM-DD` to populate the local inputs.

Examples:
  icelines streaks "Andre Burakovsky"
  icelines streaks "Connor McDavid" --json
  icelines streaks "Connor McDavid" --csv --out mcdavid-streaks.csv
"#)]
    Streaks {
        /// Player name or partial name.
        player: String,

        /// Emit the full PlayerStreaksView as JSON.
        #[arg(long)]
        json: bool,

        /// Emit CSV instead of a table.
        #[arg(long)]
        csv: bool,

        /// Write output to a file instead of stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Preview fantasy poacher watch rules.
    #[command(subcommand)]
    Watch(WatchSubcommand),

    /// Manage named data snapshots.
    #[command(subcommand)]
    Snapshot(SnapshotSubcommand),

    // ── Web dashboard — Phase King Clancy King.1.5 ────────────────────────────
    /// Open the IceLines web dashboard in your browser.
    ///
    /// Boots a localhost web server on port 8000 (configurable) and
    /// auto-opens your default browser. Same data as `icelines tui` —
    /// just rendered as HTML instead of a terminal UI.
    ///
    /// Not to be confused with the removed mkdocs/static-site preview. The
    /// bare `serve` command is the web dashboard (Phase King Clancy King.1.5).
    #[command(long_about = r#"
icelines serve — open the IceLines web dashboard in your browser.

Boots a localhost web server (axum) and auto-opens your default
browser. Same data as `icelines tui` — just rendered as HTML.

EXAMPLES
  icelines serve                       # 127.0.0.1:8000, auto-open browser
  icelines serve --port 9000           # custom port
  icelines serve --no-open             # print URL, don't auto-open
  icelines serve --bind 0.0.0.0        # LAN-accessible (LAN warning prints)
  icelines serve --no-cache            # bypass response cache (dev mode)

NOT THE SAME AS
  the removed mkdocs/static-site preview commands

DEPRECATIONS
  Pre-v0.13:  icelines serve  → mkdocs preview
  v0.13+:     icelines serve  → web dashboard (this command)
  v0.14:      mkdocs/static-site CLI surface is removed
"#)]
    Serve {
        /// Port to bind. Default 8000. Shorthand for `--bind 127.0.0.1:N`.
        #[arg(long, default_value_t = 8000)]
        port: u16,
        /// Bind address (HOST or HOST:PORT). Default `127.0.0.1`.
        /// Pass `0.0.0.0` for LAN access (prints a security warning).
        #[arg(long)]
        bind: Option<String>,
        /// Don't auto-open the browser. The URL still prints to stdout.
        #[arg(long)]
        no_open: bool,
        /// Bypass the per-request response cache (dev mode). Does NOT
        /// bypass disk-level integrity checks.
        #[arg(long)]
        no_cache: bool,
        /// Allowed CORS origin (only meaningful with `--bind 0.0.0.0`).
        /// Default: no CORS headers (localhost-only is the secure default).
        #[arg(long)]
        cors_origin: Option<String>,
    },

    // mkdocs surface (Commands::Site / Build / Deploy) was removed
    // 2026-05-04 — `icelines serve` is the single web frontend.
    // `icelines-site` crate, `docs/`, and `mkdocs.yml` remain on
    // disk for now in case the markdown-generation logic is
    // repurposed by future King.X handlers; nothing in the CLI
    // surface mounts them.
    /// Show tonight's NHL games.
    Tonight {
        /// Filter to games involving this team.
        #[arg(long)]
        team: Option<String>,
        /// Phase Foster.1 — anchor on a specific date (YYYY-MM-DD).
        /// Defaults to today. Past dates work back through the
        /// historical NHL schedule (verified ≥2014).
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        /// Phase Foster +7 — widen to 7 days from `--date` (or today).
        /// Mutually exclusive with `--month` (last in wins).
        #[arg(long)]
        week: bool,
        /// Phase Foster +7 — widen to ~30 days from `--date` (or today).
        /// Note: NHL gameWeek API returns 7 days per call, so today
        /// `--month` aliases to `--week` until multi-week aggregation
        /// lands.
        #[arg(long)]
        month: bool,
    },
    /// Show a playoff bracket — round / series / winner (LP.2).
    ///
    /// Defaults to the most recent COMPLETED playoff season in the
    /// bundle (so the output isn't empty in the offseason). Override
    /// with `--season YYYYZZZZ` for any of the 38 bundled seasons.
    /// `--round N` (1-4) filters to one round; `--json` / `--csv` for
    /// scripts.
    #[command(long_about = r#"icelines playoffs — bracket as text.

EXAMPLES
    icelines playoffs                  # most recent completed bracket
    icelines playoffs --season 19921993  # 1992-93 — Habs 23rd Cup
    icelines playoffs --round 4        # only the Cup Final
    icelines playoffs --json > bracket.json
"#)]
    Playoffs {
        /// Season in YYYYZZZZ form (e.g. 19921993 for 1992-93).
        /// Default: most recent completed playoff in the bundle.
        #[arg(long, value_name = "YYYYZZZZ")]
        season: Option<String>,
        /// Filter to one round (1=First Round, 2=Second, 3=Conference
        /// Finals, 4=Stanley Cup Final).
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=4))]
        round: Option<u8>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        /// Phase Conn Smythe C.1 — drill into one series's live
        /// momentum view (leader, OT count, last result, next-game
        /// home advantage). Series letter A–H matches the NHL's
        /// bracket convention.
        #[arg(long, value_name = "LETTER")]
        series: Option<String>,
    },

    /// Show the upcoming NHL schedule (Phase Lester Patrick — LP.1).
    ///
    /// Mirrors `icelines tonight`'s pattern but covers a date range.
    /// Defaults to the next 7 days; pass `--days N` (1-14) to widen
    /// or narrow. `--team ABBR` filters to games involving one team.
    /// `--json` / `--csv` emit machine-readable output for scripts.
    #[command(long_about = r#"icelines schedule — upcoming NHL games.

EXAMPLES
    icelines schedule                  # next 7 days
    icelines schedule --team EDM       # next 7 days for Edmonton
    icelines schedule --days 14        # next 14 days, all teams
    icelines schedule --team TOR --json # JSON for scripting
    icelines schedule --csv > games.csv # Excel-friendly export

Output columns (default text + CSV + JSON):
    date         YYYY-MM-DD
    away         3-letter abbrev
    home         3-letter abbrev
    time_et      "7:00 PM ET" (UTC offset by 4 hours, year-round
                 approximation — daylight-saving precision is a
                 future polish)
    status       pre / live / final / off
"#)]
    Schedule {
        /// 3-letter team abbrev to filter games to (e.g. EDM, TOR).
        /// Case-insensitive.
        #[arg(long)]
        team: Option<String>,
        /// Number of days forward to include (1-14, default 7).
        #[arg(long, default_value_t = 7, value_parser = clap::value_parser!(u32).range(1..=14))]
        days: u32,
        /// Emit JSON (one envelope wrapping a `games` array).
        #[arg(long)]
        json: bool,
        /// Emit CSV (one row per game, header line first).
        #[arg(long)]
        csv: bool,
        /// Phase Foster.1 — anchor date (YYYY-MM-DD). Defaults to today.
        /// Replaces the older `--start`; `--start` is kept as an alias
        /// for one minor version.
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        /// Deprecated alias for `--date`. Will be removed in v0.15.
        #[arg(long, value_name = "YYYY-MM-DD", hide = true)]
        start: Option<String>,
    },
    /// Evaluate a trade — depth chart before/after.
    Trade {
        /// Player leaving (partial name OK).
        player_out: String,
        /// Literal "for".
        #[arg(value_name = "for")]
        _for: String,
        /// Player arriving.
        player_in: String,
        /// Team perspective [default: player_out's team].
        #[arg(long)]
        team: Option<String>,
    },
    /// Project rest-of-season performance.
    Project {
        /// Player name (partial match OK). Omit to use --team.
        player: Option<String>,
        /// Project all skaters on a team.
        #[arg(long)]
        team: Option<String>,
        /// Projection mode: pace | regressed | composite [default: regressed]
        #[arg(long, default_value = "regressed")]
        mode: String,
        /// Override remaining games (default: auto from schedule).
        #[arg(long)]
        games: Option<u32>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Launch the interactive TUI.
    #[command(long_about = r#"Launch the interactive ratatui TUI.

By default opens the Jack Adams dashboard on the League workspace.
Two ways to jump to a specific workspace — sugar subcommand or --start flag.

EXAMPLES
    icelines tui                       Dashboard on League (default).
    icelines tui goalies               Dashboard with Goalies workspace.
    icelines tui scores                Dashboard with tonight's scores.
    icelines tui --classic             Older tabbed single-document UI.
    icelines tui --start goalies       Same as `icelines tui goalies`.
    icelines tui --start tonight       Alias accepted (= scores).

Recognized canonical slugs:
    league         32-team rankings (default)
    depth          Cross-team depth chart
    stats          Interactive query/filter builder
    goalies        Goalie leaderboard
    scores         Tonight's games + boxscores
    schedule       Weekly + season schedule
    transactions   League-wide moves feed
    playoffs       Bracket + series detail

Aliases also accepted on --start: queries (= stats), tonight (= scores),
moves (= transactions). All slugs are case-insensitive.

By default the TUI opens the Jack Adams dashboard: scores ribbon,
side panes, central workspace, and command bar. Use --classic for
the older tabbed single-document UI. Press y for the season picker;
q (or Esc on a non-tab screen) to quit.
"#)]
    Tui {
        /// Optional sugar subcommand for one of the 8 nav surfaces.
        /// `icelines tui goalies` is shorthand for `--start goalies`.
        #[command(subcommand)]
        surface: Option<TuiSurface>,
        /// Surface to start on. Optional — default is `league`. Same
        /// slug grammar as the sugar subcommands. See --help.
        #[arg(long, value_name = "SLUG")]
        start: Option<String>,
        /// Restore a named dashboard layout saved by `icelines layout save`.
        #[arg(long, value_name = "NAME")]
        layout: Option<String>,
        /// Phase Masterton.3 — lock the TUI to the chosen surface.
        /// Tab/Shift+Tab become no-ops; the tab strip is hidden.
        /// Useful for a focused single-screen experience.
        ///
        /// Example:
        ///   icelines tui goalies --standalone
        #[arg(long, global = true)]
        standalone: bool,
        /// Phase Jack Adams — explicitly launch the TUI in dashboard
        /// mode. This is now the default for `icelines tui`; the flag
        /// remains for scripts and discoverability.
        ///
        /// Mutually exclusive with --standalone and --classic.
        ///
        /// Example:
        ///   icelines tui stats --mdi
        #[arg(
            long,
            global = true,
            conflicts_with_all = ["standalone", "classic"]
        )]
        mdi: bool,
        /// Launch the older tabbed single-document TUI instead of
        /// the Jack Adams dashboard. This keeps the pre-dashboard
        /// workflow available while making the dashboard the normal
        /// product entry point.
        #[arg(
            long,
            alias = "sdi",
            global = true,
            conflicts_with_all = ["standalone", "mdi"]
        )]
        classic: bool,
        #[arg(long, hide = true)]
        render_leaders_active_filter_snapshot: bool,
    },

    /// Manage durable named workbench layouts.
    #[command(subcommand)]
    Layout(LayoutSubcommand),

    /// Print the full command reference (embedded COMMANDS.md). No
    /// internet required — the doc ships inside the binary.
    Docs,

    /// Friendly entry point — print a numbered menu and dispatch to
    /// the chosen surface (web dashboard, TUI, docs). Loops back to
    /// the menu after each surface quits; Q is the only way out.
    /// Skip the menu by running `icelines tui <slug>` /
    /// `icelines serve` / `icelines docs` directly.
    #[command(long_about = r#"icelines menu — interactive looping launcher.

Prints a numbered menu and reads a choice; dispatches to the matching
surface; loops back to the menu when the surface quits. Q (or q) is
the only way out — Ctrl-C will currently exit non-zero (130 on Unix).

EXAMPLES
    icelines menu                          # boot the launcher
    echo 1 | icelines menu                 # NOT supported — see below

NON-INTERACTIVE INVOCATION
    `icelines menu` requires a real TTY on stdin. When stdin is piped
    or redirected (e.g. `icelines menu < /dev/null`) it exits 0 with a
    one-line redirect message; for scripted use call
    `icelines tui --start <slug>` directly.

ENTRY POINTS COVERED
    1-8                Nav-tab surfaces (League, Stats, Goalies, ...).
    P / T / G / C      Drill-downs (Player / Team / Goalie / Comps).
    W                  Web dashboard (port 8000).
    D                  Print COMMANDS.md.
    Q                  Quit the menu.
"#)]
    Menu,
    /// Export analytical output as markdown tables (Phase 8d).
    /// Bridges to proof's DASHBOARD-SPEC compiler — see
    /// `design/specs/export-markdown.md`.
    #[command(subcommand)]
    Export(ExportSubcommand),
    /// Quick CSV/JSON export of any report — `icelines x leaders --top 10`.
    /// Default format is CSV, default destination is stdout. Use --out to
    /// write to a file you can open in Excel directly.
    #[command(name = "x", alias = "xport")]
    X {
        /// Report shape to export. Run `icelines x --help` for the list.
        shape: ExportShape,
        /// Player name (for shapes that target a single player).
        #[arg(long)]
        player: Option<String>,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        pos: Option<String>,
        #[arg(long)]
        year: Option<u16>,
        #[arg(long, default_value_t = 25)]
        top: usize,
        /// Number of seasons (history shape).
        #[arg(long, default_value_t = 5)]
        seasons: usize,
        /// Emit JSON instead of CSV. CSV is the default — Excel-friendly.
        #[arg(long)]
        json: bool,
        /// Write to a file instead of stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Filter and list players with rich criteria.
    Players {
        #[arg(long)]
        pos: Option<String>,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        age_max: Option<u8>,
        #[arg(long)]
        age_min: Option<u8>,
        #[arg(long)]
        nationality: Option<String>,
        #[arg(long)]
        draft_year: Option<u16>,
        #[arg(long)]
        draft_round: Option<u8>,
        #[arg(long)]
        ppg_min: Option<f64>,
        #[arg(long)]
        gp_min: Option<u32>,
        #[arg(long, default_value_t = 25)]
        top: usize,
        #[arg(long)]
        json: bool,
        /// Emit RFC-4180 CSV (one header row + data rows). Mutually exclusive with --json.
        #[arg(long)]
        csv: bool,
        /// Write the report to a file instead of stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Show a draft class — all players from a given draft year.
    Class {
        year: u16,
        #[arg(long)]
        pos: Option<String>,
        #[arg(long)]
        top: Option<usize>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Find statistical peers for a player (same draft era and position).
    Peers {
        player: String,
        #[arg(long, default_value_t = 10)]
        size: usize,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Head-to-head player comparison.
    Compare {
        player1: String,
        player2: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Show a player's historical season stats.
    History {
        player: String,
        /// Number of seasons to show (default: 5).
        #[arg(long, default_value_t = 5)]
        seasons: usize,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Find line-mates for a player.
    Mates {
        /// Player name (fuzzy matched).
        player: String,
        /// Number of top linemates to display.
        #[arg(long, default_value_t = 5)]
        top: usize,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// League-wide transactions feed: trades, waivers, signings, IR, recalls,
    /// reassignments. Sourced from ESPN's site.api. Phase T.4.
    Transactions {
        /// Filter to one team (canonical NHL abbrev — TBL, EDM, …).
        /// Use `LEAGUE` for league-wide / teamless rows.
        #[arg(long)]
        team: Option<String>,
        /// Only show transactions on/after this date (YYYY-MM-DD).
        #[arg(long)]
        since: Option<String>,
        /// Only show transactions on/before this date (YYYY-MM-DD).
        #[arg(long)]
        until: Option<String>,
        /// Filter by kind: trade, signing, recall, reassignment, ir,
        /// waiver (or waiver_placement / waiver_clear / waiver_claim), other.
        #[arg(long)]
        kind: Option<String>,
        /// Free-form substring search against the description (case-insensitive,
        /// diacritic-stripped). Works on player names, team names, anything.
        #[arg(long)]
        search: Option<String>,
        /// Show only transactions mentioning this player by last name.
        /// Combine with --team to disambiguate shared last names.
        #[arg(long)]
        player: Option<String>,
        /// Use a bundled or installed historical season.
        #[arg(long)]
        season: Option<String>,
        /// Limit to first N rows (default: all).
        #[arg(long)]
        top: Option<usize>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        csv: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Manage player watchlists and custom groups.
    #[command(subcommand)]
    Group(GroupSubcommand),
    /// Favorites dashboard (Phase Foster.2).
    ///
    /// Renders today's games + stat lines for your favorited
    /// players and teams.
    ///
    /// Works offline from cached boxscores; falls back to the
    /// live NHL API when `live_feeds = on`.
    Favorites {
        /// Anchor date (`YYYY-MM-DD`). Default: today.
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        /// Aggregate over a window: `day` (default), `week`, `month`.
        #[arg(long, value_name = "WINDOW")]
        range: Option<String>,
        /// Group name to read favorites from. Default: `Favorites`.
        #[arg(long, default_value = "Favorites")]
        group: String,
        /// Emit JSON envelope instead of the default text table.
        #[arg(long)]
        json: bool,
    },

    /// Inspect the on-disk data manifest (Phase Foster +2).
    ///
    /// Pretty-prints what's in `~/.icelines/data/manifest/`: the
    /// per-kind shards, their backing files, freshness, and source
    /// (Bundle / Setup / Live / DataInstall / Manual).
    DataStatus {
        /// Filter to one DataKind (case-insensitive). Recognized:
        /// bios, stats, goalie_stats, transactions, boxscore,
        /// career_history, schedule, score, playoff_bracket.
        #[arg(long, value_name = "KIND")]
        shard: Option<String>,
        /// List only entries that `fetch sync` would refresh.
        #[arg(long)]
        stale_only: bool,
    },

    /// First-run setup wizard (Phase Foster.0.8).
    ///
    /// Three-question flow that writes capability matrix defaults to
    /// `~/.icelines/config.toml`. Auto-runs on first invocation when
    /// no config file exists; pass `--no-setup` (top-level) to skip.
    /// In scripted contexts pass `--accept-defaults` to write the
    /// defaults non-interactively. Existing config files are left
    /// unchanged unless `--reset` is passed.
    Setup {
        /// Skip the prompts; write the spec defaults and exit.
        /// Useful for headless / CI / persona-test contexts.
        #[arg(long)]
        accept_defaults: bool,
        /// Print what setup would do without writing config.toml.
        #[arg(long)]
        dry_run: bool,
        /// Re-run setup even if config.toml already exists.
        #[arg(long)]
        reset: bool,
    },

    /// Read or update a configuration value (Phase Foster.0.7).
    ///
    /// Configurable keys live under `sync.*`:
    ///   sync.policy                          eager | lazy | off
    ///   sync.banner                          summary | silent | verbose
    ///   sync.season_transition               prompt | auto | ignore
    ///   sync.capabilities.stats              off | favorites | league
    ///   sync.capabilities.scores_schedule    off | favorites | league
    ///   sync.capabilities.transactions       off | favorites | league
    ///   sync.capabilities.boxscores          off | favorites | league
    ///   sync.capabilities.shifts             off (only — per-shift parsing not implemented)
    ///   sync.capabilities.career_history     off | favorites | league
    #[command(subcommand)]
    Config(ConfigSubcommand),
    /// Track NHL games you attended in person.
    #[command(subcommand)]
    Games(GamesSubcommand),
    /// Full 8-section scouting report for a player.
    Scouting {
        player: String,
        #[arg(long, default_value = "terminal")]
        format: String,
    },
    /// Show a player's descriptive Signals (derived metrics).
    #[command(long_about = r#"
Render a player's IceLines Signals — descriptive derived metrics built from
existing stat inputs (Phase Hurricane / WP-010).

Signals are NOT predictions, betting edges, injury signals, deployment
recommendations, or autonomous coaching decisions. Missing or partial evidence
renders as `unavailable`, never as a 0.0 player value.

Current signals (all per-60):
  Physical Engagement Rate  (=)  (hits + blocked shots) per 60
  Puck Management Differential (↑)  (takeaways − giveaways) per 60
  Penalty Drag Rate         (↓)  penalty minutes per 60

Legend: ↑ higher is better · ↓ lower is better · = neutral

Examples:
  icelines signals "Connor McDavid"
  icelines signals "McDavid" --json
  icelines signals "Wayne Gretzky" --season 19881989
"#)]
    Signals {
        /// Player name (partial match works, e.g. "McDavid").
        player: String,
        /// Season id (YYYYZZZZ). Defaults to the configured / current season.
        #[arg(long)]
        season: Option<String>,
        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Emit the PlayerSignalsView as a frozen `signals.v1` JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Show a team-scoped Signals roster matrix without leaderboard promotion.
    #[command(
        name = "signals-roster",
        long_about = r#"
Render a team-scoped IceLines Signals roster matrix.

This is a discovery aid for finding player Signals cards to inspect. It is not a
Signal leaderboard, StatId promotion, filter key, cache metric family, prediction,
betting edge, injury signal, deployment recommendation, player-quality grade, or
autonomous coaching decision.

Examples:
  icelines signals-roster --team NYR
  icelines signals-roster --team NYR --json
"#
    )]
    SignalsRoster {
        /// Team abbreviation (e.g. NYR, EDM, TOR).
        #[arg(long)]
        team: String,
        /// Season id (YYYYZZZZ). Defaults to the configured / current season.
        #[arg(long)]
        season: Option<String>,
        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Emit a `signals-roster.v1` JSON envelope.
        #[arg(long)]
        json: bool,
    },
    /// Manage fantasy scoring schemes.
    #[command(subcommand)]
    Scheme(SchemeSubcommand),
    /// Open the league dashboard (alias for `icelines tui`).
    Dashboard, // → launches TUI

    /// Download and manage additional season data bundles.
    #[command(subcommand)]
    Data(DataSubcommand),

    /// Advanced query engine — leaderboards, player profiles, similarity search.
    #[command(subcommand)]
    Query(QuerySubcommand),

    /// Fantasy league management — teams, scoring, imports, trades, server.
    #[command(
        subcommand,
        long_about = r#"
Manage local fantasy leagues in `~/.icelines/icelines.db`.

Yahoo roster CSV import is a local setup helper: preview with `--dry-run`, then
apply league/team/roster membership to FantasyDb. NHL API/bundled data remains
authoritative for player identity, current NHL teams, stats, and photos.

Examples:
  icelines fantasy league-create "My League" --scheme yahoo-standard
  icelines fantasy team-create "My Team" --owner "Gio"
  icelines fantasy import-yahoo --file rosters.csv --league "My League" --dry-run
  icelines fantasy import-yahoo --file rosters.csv --league "My League" --my-team "My Team"
  icelines fantasy roster-shape
  icelines fantasy roster-shape-validate --team "My Team"
  icelines fantasy gaps --category hits,blocks,shots
  icelines fantasy daily --date 2026-01-15 --json
"#
    )]
    Fantasy(FantasySubcommand),
}

// ── Tui sub-commands (LB.2 sugar) ─────────────────────────────────────────────

/// Phase Lady Byng (LB.2) — sugar subcommands for `icelines tui`.
///
/// Each variant boots the TUI directly on the matching nav surface.
/// `icelines tui goalies` is shorthand for `icelines tui --start goalies`.
/// LB.3 will extend this enum with parameterized drill-downs (Player,
/// Team, Goalie, Comps).
#[derive(Debug, Subcommand)]
pub enum TuiSurface {
    /// 32-team rankings (default).
    League,
    /// Cross-team depth chart.
    Depth,
    /// Interactive query/filter builder.
    Stats,
    /// Goalie leaderboard.
    Goalies,
    /// Fantasy poacher board.
    Poach,
    /// Local fantasy poacher watchlist group.
    Watchlist,
    /// Tonight's games + boxscores.
    Scores,
    /// Weekly + season schedule.
    Schedule,
    /// League-wide moves feed.
    Transactions,
    /// Bracket + series detail.
    Playoffs,

    // ── LB.3 — Drill-down sugar ────────────────────────────────────────
    /// Open a player card directly. Accepts a name (substring match)
    /// or an explicit pid.
    Player {
        /// Player name (e.g. "Bedard", "Connor McDavid") or pid (e.g. 8478402).
        needle: String,
    },
    /// Open a team's depth chart directly. 3-letter abbrev (e.g. EDM, TOR).
    Team {
        /// 3-letter team abbreviation. Case-insensitive.
        abbrev: String,
    },
    /// Open a goalie card directly. Accepts a name or pid.
    Goalie {
        /// Goalie name or pid.
        needle: String,
    },
    /// Open the comps screen for a player. Accepts a name or pid.
    Comps {
        /// Player name or pid.
        needle: String,
    },
}

impl TuiSurface {
    /// Map sugar variant → ScreenSpec. Nav-tab variants wrap the
    /// matching NavSpec; LB.3 drill-down variants build parameterized
    /// ScreenSpec values that need resolution.
    pub fn into_screen_spec(self) -> crate::start_slug::ScreenSpec {
        use crate::start_slug::{NavSpec, Needle, ScreenSpec};
        match self {
            TuiSurface::League => ScreenSpec::Nav(NavSpec::Home),
            TuiSurface::Depth => ScreenSpec::Nav(NavSpec::Depth),
            TuiSurface::Stats => ScreenSpec::Nav(NavSpec::Queries),
            TuiSurface::Goalies => ScreenSpec::Nav(NavSpec::Goalies),
            TuiSurface::Poach => ScreenSpec::Nav(NavSpec::Poach),
            TuiSurface::Watchlist => ScreenSpec::Nav(NavSpec::Watchlist),
            TuiSurface::Scores => ScreenSpec::Nav(NavSpec::Tonight),
            TuiSurface::Schedule => ScreenSpec::Nav(NavSpec::Schedule),
            TuiSurface::Transactions => ScreenSpec::Nav(NavSpec::Transactions),
            TuiSurface::Playoffs => ScreenSpec::Nav(NavSpec::Playoffs),
            // LB.3 — drill-down sugar. The needle is parsed-but-not-
            // resolved; main.rs calls into_screen() to resolve.
            TuiSurface::Player { needle } => ScreenSpec::Player(Needle::from_arg(&needle)),
            TuiSurface::Team { abbrev } => ScreenSpec::Team(abbrev),
            TuiSurface::Goalie { needle } => ScreenSpec::Goalie(Needle::from_arg(&needle)),
            TuiSurface::Comps { needle } => ScreenSpec::Comps(Needle::from_arg(&needle)),
        }
    }
}

#[cfg(test)]
mod tui_surface_tests {
    use super::*;
    use crate::start_slug::{parse_start_slug, NavSpec, ScreenSpec};
    use clap::Parser;

    /// LB.2 / l0_sugar_each_nav_tab_parses
    /// — Every TuiSurface variant parses cleanly via clap and resolves
    ///   to the same ScreenSpec as the corresponding `--start <slug>`.
    ///   This locks sugar↔flag parity: if a future rename breaks one
    ///   path, this test fails before users hit it.
    #[test]
    fn l0_sugar_each_nav_tab_parses() {
        let cases = [
            ("league", ScreenSpec::Nav(NavSpec::Home)),
            ("depth", ScreenSpec::Nav(NavSpec::Depth)),
            ("stats", ScreenSpec::Nav(NavSpec::Queries)),
            ("goalies", ScreenSpec::Nav(NavSpec::Goalies)),
            ("poach", ScreenSpec::Nav(NavSpec::Poach)),
            ("watchlist", ScreenSpec::Nav(NavSpec::Watchlist)),
            ("scores", ScreenSpec::Nav(NavSpec::Tonight)),
            ("schedule", ScreenSpec::Nav(NavSpec::Schedule)),
            ("transactions", ScreenSpec::Nav(NavSpec::Transactions)),
            ("playoffs", ScreenSpec::Nav(NavSpec::Playoffs)),
        ];
        for (slug, expected) in cases {
            // Sugar form: `icelines tui <slug>`
            let cli = Cli::try_parse_from(["icelines", "tui", slug]).expect("sugar should parse");
            let sugar_spec = match cli.command {
                Commands::Tui {
                    surface: Some(s),
                    start: None,
                    layout: None,
                    standalone: false,
                    mdi: false,
                    classic: false,
                    render_leaders_active_filter_snapshot: false,
                } => s.into_screen_spec(),
                other => panic!("expected Tui {{ surface: Some(_), start: None }}, got {other:?}"),
            };
            assert_eq!(sugar_spec, expected, "sugar slug={slug}");

            // Flag form: `icelines tui --start <slug>`
            let cli = Cli::try_parse_from(["icelines", "tui", "--start", slug])
                .expect("--start should parse");
            let flag_spec = match cli.command {
                Commands::Tui {
                    surface: None,
                    start: Some(s),
                    layout: None,
                    standalone: false,
                    mdi: false,
                    classic: false,
                    render_leaders_active_filter_snapshot: false,
                } => parse_start_slug(&s).expect("known slug"),
                other => panic!("expected Tui {{ surface: None, start: Some(_) }}, got {other:?}"),
            };
            assert_eq!(flag_spec, expected, "flag slug={slug}");

            // Parity: both produce the same ScreenSpec.
            assert_eq!(sugar_spec, flag_spec, "sugar↔flag drift for {slug}");
        }
    }

    /// LB.2 / l0_bare_tui_has_no_surface_or_start
    /// — `icelines tui` with no args produces None/None — main.rs
    ///   dispatches that to Screen::Home (default).
    #[test]
    fn l0_bare_tui_has_no_surface_or_start() {
        let cli = Cli::try_parse_from(["icelines", "tui"]).unwrap();
        match cli.command {
            Commands::Tui {
                surface,
                start,
                layout,
                standalone,
                mdi,
                classic,
                render_leaders_active_filter_snapshot,
            } => {
                assert!(surface.is_none());
                assert!(!standalone, "bare tui must default standalone=false");
                assert!(!mdi, "bare tui should not require explicit --mdi");
                assert!(!classic, "bare tui must default classic=false");
                assert!(
                    !render_leaders_active_filter_snapshot,
                    "bare tui must not render a diagnostic snapshot"
                );
                let _ = standalone; // silence unused warning
                let _ = mdi;
                let _ = classic;
                assert!(start.is_none());
                assert!(layout.is_none());
            }
            other => panic!("expected Tui, got {other:?}"),
        }
    }

    #[test]
    fn l0_tui_classic_flag_parses_as_sdi_escape_hatch() {
        let cli = Cli::try_parse_from(["icelines", "tui", "--classic"]).unwrap();
        match cli.command {
            Commands::Tui {
                surface,
                start,
                layout,
                standalone,
                mdi,
                classic,
                render_leaders_active_filter_snapshot,
            } => {
                assert!(surface.is_none());
                assert!(start.is_none());
                assert!(layout.is_none());
                assert!(!standalone);
                assert!(!mdi);
                assert!(classic);
                assert!(!render_leaders_active_filter_snapshot);
            }
            other => panic!("expected Tui, got {other:?}"),
        }
    }

    #[test]
    fn l0_tui_mode_flags_parse_after_surface_for_documented_examples() {
        let cases = [
            (["icelines", "tui", "goalies", "--standalone"], "standalone"),
            (["icelines", "tui", "goalies", "--mdi"], "mdi"),
            (["icelines", "tui", "goalies", "--classic"], "classic"),
        ];
        for (args, expected_mode) in cases {
            let cli = Cli::try_parse_from(args).unwrap();
            match cli.command {
                Commands::Tui {
                    surface: Some(TuiSurface::Goalies),
                    standalone,
                    mdi,
                    classic,
                    ..
                } => match expected_mode {
                    "standalone" => assert!(standalone),
                    "mdi" => assert!(mdi),
                    "classic" => assert!(classic),
                    other => panic!("unexpected mode {other}"),
                },
                other => panic!("expected Tui goalies for {args:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn l0_tui_mode_flags_conflict() {
        for args in [
            ["icelines", "tui", "--mdi", "--standalone"],
            ["icelines", "tui", "--mdi", "--classic"],
            ["icelines", "tui", "--classic", "--standalone"],
        ] {
            let result = Cli::try_parse_from(args);
            assert!(result.is_err(), "expected conflict for {args:?}");
        }
    }

    /// LB.2 / l0_sugar_with_alias_slug_rejected
    /// — Alias slugs (`queries`, `tonight`, `moves`) are NOT sugar
    ///   subcommands — they only exist on `--start`. clap rejects
    ///   `icelines tui tonight` as an unknown subcommand. This is
    ///   intentional: aliases shouldn't bloat the `--help` output.
    #[test]
    fn l0_sugar_with_alias_slug_rejected() {
        for alias in ["queries", "tonight", "moves"] {
            let result = Cli::try_parse_from(["icelines", "tui", alias]);
            assert!(
                result.is_err(),
                "alias '{alias}' should not work as sugar subcommand"
            );
        }
        // But the same aliases work on --start:
        for alias in ["queries", "tonight", "moves"] {
            let result = Cli::try_parse_from(["icelines", "tui", "--start", alias]);
            assert!(
                result.is_ok(),
                "alias '{alias}' should work on --start flag"
            );
        }
    }

    #[test]
    fn l0_fantasy_simulate_clap_surface_parses() {
        let cli = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "simulate",
            "--weeks",
            "6",
            "--add",
            "Block Helper",
            "--drop",
            "Bench Forward",
            "--json",
        ])
        .expect("fantasy simulate should parse");

        match cli.command {
            Commands::Fantasy(FantasySubcommand::Simulate {
                weeks,
                add_player,
                drop_player,
                json,
                ..
            }) => {
                assert_eq!(weeks, 6);
                assert_eq!(add_player.as_deref(), Some("Block Helper"));
                assert_eq!(drop_player.as_deref(), Some("Bench Forward"));
                assert!(json);
            }
            other => panic!("expected fantasy simulate, got {other:?}"),
        }
    }

    #[test]
    fn l0_fantasy_daily_clap_surface_parses() {
        let cli = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "daily",
            "--date",
            "2026-01-15",
            "--league",
            "Daily League",
            "--json",
        ])
        .expect("fantasy daily should parse");

        match cli.command {
            Commands::Fantasy(FantasySubcommand::Daily {
                date,
                league,
                season,
                season_type,
                json,
            }) => {
                assert_eq!(date.to_string(), "2026-01-15");
                assert_eq!(league.as_deref(), Some("Daily League"));
                assert_eq!(season, icelines_core::CURRENT_SEASON);
                assert_eq!(season_type, QuerySeasonType::Regular);
                assert!(json);
            }
            other => panic!("expected fantasy daily, got {other:?}"),
        }
    }

    #[test]
    fn l0_fantasy_matchup_clap_surfaces_parse() {
        let read = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "matchup",
            "--date",
            "2026-01-15",
            "--league",
            "Matchup League",
            "--json",
        ])
        .expect("fantasy matchup should parse");
        match read.command {
            Commands::Fantasy(FantasySubcommand::Matchup {
                date, league, json, ..
            }) => {
                assert_eq!(date.to_string(), "2026-01-15");
                assert_eq!(league.as_deref(), Some("Matchup League"));
                assert!(json);
            }
            other => panic!("expected fantasy matchup, got {other:?}"),
        }

        let setup = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "matchup-set",
            "--week",
            "2026-01-15",
            "--home",
            "My Team",
            "--away",
            "Rival",
        ])
        .expect("fantasy matchup-set should parse");
        match setup.command {
            Commands::Fantasy(FantasySubcommand::MatchupSet {
                week, home, away, ..
            }) => {
                assert_eq!(week.to_string(), "2026-01-15");
                assert_eq!(home, "My Team");
                assert_eq!(away.as_deref(), Some("Rival"));
            }
            other => panic!("expected fantasy matchup-set, got {other:?}"),
        }
    }

    #[test]
    fn l0_fantasy_import_yahoo_clap_surface_parses() {
        let cli = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "import-yahoo",
            "--file",
            "rosters.csv",
            "--league",
            "Office Pool",
            "--my-team",
            "My Team",
            "--dry-run",
            "--json",
        ])
        .expect("fantasy import-yahoo should parse");

        match cli.command {
            Commands::Fantasy(FantasySubcommand::ImportYahoo {
                file,
                league,
                my_team,
                dry_run,
                json,
            }) => {
                assert_eq!(file, std::path::PathBuf::from("rosters.csv"));
                assert_eq!(league, "Office Pool");
                assert_eq!(my_team.as_deref(), Some("My Team"));
                assert!(dry_run);
                assert!(json);
            }
            other => panic!("expected fantasy import-yahoo, got {other:?}"),
        }
    }

    #[test]
    fn l0_fantasy_roster_shape_clap_surfaces_parse() {
        let show = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "roster-shape",
            "--league",
            "Office Pool",
            "--json",
        ])
        .expect("fantasy roster-shape should parse");
        match show.command {
            Commands::Fantasy(FantasySubcommand::RosterShape { league, json }) => {
                assert_eq!(league.as_deref(), Some("Office Pool"));
                assert!(json);
            }
            other => panic!("expected fantasy roster-shape, got {other:?}"),
        }

        let set = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "roster-shape-set",
            "yahoo-standard",
            "--league",
            "Office Pool",
        ])
        .expect("fantasy roster-shape-set should parse");
        match set.command {
            Commands::Fantasy(FantasySubcommand::RosterShapeSet { shape, league }) => {
                assert_eq!(shape, "yahoo-standard");
                assert_eq!(league.as_deref(), Some("Office Pool"));
            }
            other => panic!("expected fantasy roster-shape-set, got {other:?}"),
        }

        let validate = Cli::try_parse_from([
            "icelines",
            "fantasy",
            "roster-shape-validate",
            "--league",
            "Office Pool",
            "--team",
            "My Team",
            "--json",
        ])
        .expect("fantasy roster-shape-validate should parse");
        match validate.command {
            Commands::Fantasy(FantasySubcommand::RosterShapeValidate { league, team, json }) => {
                assert_eq!(league.as_deref(), Some("Office Pool"));
                assert_eq!(team.as_deref(), Some("My Team"));
                assert!(json);
            }
            other => panic!("expected fantasy roster-shape-validate, got {other:?}"),
        }
    }
}

// ── Fetch sub-commands ────────────────────────────────────────────────────────

/// Hart.6.5 — `--type` flag value for `fetch stats|goalies|all`.
/// `Regular` (default) fetches the regular-season dataset.
/// `Playoff` fetches `gameTypeId=3` and writes co-located
/// `playoff-bios.json` / `playoff-stats.json` / `playoff-goalie-stats.json`
/// alongside the regular files. `Both` runs the regular pass first
/// (full pipeline including realtime + moneypuck + contracts) then
/// the playoff trio (bios + stats + goalies only — playoff has no
/// realtime/moneypuck per Hart.6 D6).
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum, Default)]
pub enum FetchSeasonType {
    #[default]
    #[clap(name = "regular")]
    Regular,
    #[clap(name = "playoff")]
    Playoff,
    #[clap(name = "both")]
    Both,
}

/// Hart.6.9 — `--type` flag for read-side query commands. Unlike
/// `FetchSeasonType` there's no `Both`: a query operates on a single
/// `(season, season_type)` window at a time. Maps cleanly to
/// `icelines_core::season_stats::SeasonType`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum, Default)]
pub enum QuerySeasonType {
    #[default]
    #[clap(name = "regular")]
    Regular,
    #[clap(name = "playoff")]
    Playoff,
}

#[derive(Debug, Subcommand)]
pub enum ReportSubcommand {
    /// List canonical report/export surfaces and planned record reports.
    List {
        /// Emit the report catalog as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Generate a fantasy poacher report from PoachReportView.
    Poach {
        /// Season id, e.g. 20252026. Defaults to configured season.
        #[arg(long)]
        season: Option<String>,

        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,

        /// Fantasy scoring scheme name.
        #[arg(long, default_value = "yahoo-standard")]
        scheme: String,

        /// Comma-separated categories to emphasize, e.g. hits,blocks,shots.
        #[arg(long = "category", value_delimiter = ',')]
        categories: Vec<String>,

        /// Filter by team abbreviation.
        #[arg(long)]
        team: Vec<String>,

        /// Filter by position abbreviation: C, LW, RW, D.
        #[arg(long)]
        pos: Vec<String>,

        /// Filter by availability: any, available, imported-available,
        /// not-on-user-roster, watched, unknown.
        #[arg(long)]
        availability: Option<String>,

        /// Number of candidates to include.
        #[arg(long, default_value_t = 20)]
        top: u16,

        /// Emit the PoachReportView as JSON instead of markdown.
        #[arg(long)]
        json: bool,

        /// Write report to path. Pass '-' or omit for stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Generate a weekly fantasy prep report from PoachReportView.
    Weekly {
        /// Season id, e.g. 20252026. Defaults to configured season.
        #[arg(long)]
        season: Option<String>,

        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,

        /// Fantasy scoring scheme name.
        #[arg(long, default_value = "yahoo-standard")]
        scheme: String,

        /// League/profile label to include in the report id.
        #[arg(long, default_value = "default")]
        league: String,

        /// Comma-separated categories to emphasize, e.g. hits,blocks,shots.
        #[arg(long = "category", value_delimiter = ',')]
        categories: Vec<String>,

        /// Filter by team abbreviation.
        #[arg(long)]
        team: Vec<String>,

        /// Filter by position abbreviation: C, LW, RW, D.
        #[arg(long)]
        pos: Vec<String>,

        /// Filter by availability: any, available, imported-available,
        /// not-on-user-roster, watched, unknown.
        #[arg(long)]
        availability: Option<String>,

        /// Number of candidates to include per populated section.
        #[arg(long, default_value_t = 20)]
        top: u16,

        /// Emit the PoachReportView as JSON instead of markdown.
        #[arg(long)]
        json: bool,

        /// Write report to path. Pass '-' or omit for stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum RecordsSubcommand {
    /// Show records for one player.
    Player {
        /// Player name or partial name.
        player: String,

        /// Record metric to compute.
        #[arg(long, value_enum, default_value_t = RecordsMetric::TeamsScoredAgainst)]
        metric: RecordsMetric,

        /// Emit JSON instead of a table.
        #[arg(long)]
        json: bool,

        /// Emit CSV instead of a table.
        #[arg(long)]
        csv: bool,

        /// Write output to a file instead of stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Show records against one team.
    Team {
        /// Team abbreviation, e.g. EDM.
        team: String,

        /// Record metric to compute.
        #[arg(long, value_enum, default_value_t = RecordsMetric::PlayersScoredAgainstTeam)]
        metric: RecordsMetric,

        /// Emit JSON instead of a table.
        #[arg(long)]
        json: bool,

        /// Emit CSV instead of a table.
        #[arg(long)]
        csv: bool,

        /// Write output to a file instead of stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum RecordsMetric {
    /// NHL teams a player has scored against.
    TeamsScoredAgainst,
    /// Goalies a player has scored against.
    GoaliesScoredAgainst,
    /// Players a player has fought.
    FightOpponents,
    /// Players who scored against a team.
    PlayersScoredAgainstTeam,
    /// Goalies a team has scored against.
    GoaliesBeatenByTeam,
    /// Opposing players fought by players on a team.
    FightOpponentsByTeam,
}

#[derive(Debug, Subcommand)]
pub enum WatchSubcommand {
    /// List the local fantasy poacher watchlist with stored reasons.
    List {
        /// Emit watchlist rows as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Add/update a manual reason for a watched player.
    Note {
        /// Player name to watch.
        player: String,

        /// Freeform reason. Multiple words do not need quotes.
        #[arg(required = true, num_args = 1..)]
        reason: Vec<String>,

        /// Emit the updated watch row as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Show default poacher watch rules.
    Rules {
        /// Season id, e.g. 20252026. Defaults to configured season.
        #[arg(long)]
        season: Option<String>,

        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,

        /// Emit WatchRulesView as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Enable a persisted poacher watch rule by id.
    Enable {
        /// Persisted watch rule id, e.g. player-matthew-knies.
        id: String,

        /// Emit the updated rule as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Disable a persisted poacher watch rule by id.
    Disable {
        /// Persisted watch rule id, e.g. player-matthew-knies.
        id: String,

        /// Emit the updated rule as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Record that a persisted watch rule fired.
    Fire {
        /// Persisted watch rule id, e.g. player-matthew-knies.
        id: String,

        /// Optional player name connected to the alert.
        #[arg(long)]
        player: Option<String>,

        /// Human-readable alert message.
        #[arg(required = true, num_args = 1..)]
        message: Vec<String>,

        /// Emit the recorded event as JSON.
        #[arg(long)]
        json: bool,
    },

    /// List local fired-alert history for persisted watch rules.
    History {
        /// Maximum events to show.
        #[arg(long, default_value_t = 20)]
        limit: u16,

        /// Emit watch rule events as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Evaluate current fantasy watch alerts without persisting events.
    Alerts {
        /// Season id, e.g. 20252026. Defaults to configured season.
        #[arg(long)]
        season: Option<String>,

        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,

        /// Number of poach candidates to evaluate.
        #[arg(long, default_value_t = 200)]
        top: u16,

        /// Persist new alerts to watch history after evaluating.
        #[arg(long)]
        save: bool,

        /// Emit WatchAlertsView as JSON.
        #[arg(long)]
        json: bool,
    },

    /// Preview a player-specific watch rule.
    Player {
        /// Player name to watch.
        player: String,

        /// Trigger label, e.g. pp1, top-six, available.
        #[arg(long = "when", default_value = "promotion")]
        when: String,

        /// Season id, e.g. 20252026. Defaults to configured season.
        #[arg(long)]
        season: Option<String>,

        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,

        /// Emit WatchRulesView as JSON.
        #[arg(long)]
        json: bool,

        /// Persist this rule locally after previewing it.
        #[arg(long)]
        save: bool,
    },

    /// Preview a team deployment watch rule.
    Deployment {
        /// Team abbreviation to watch.
        #[arg(long)]
        team: Option<String>,

        /// Watch for line-change events.
        #[arg(long = "line-change")]
        line_change: bool,

        /// Season id, e.g. 20252026. Defaults to configured season.
        #[arg(long)]
        season: Option<String>,

        /// Regular season or playoffs.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,

        /// Emit WatchRulesView as JSON.
        #[arg(long)]
        json: bool,

        /// Persist this rule locally after previewing it.
        #[arg(long)]
        save: bool,
    },
}

impl QuerySeasonType {
    pub fn to_core(self) -> icelines_core::season_stats::SeasonType {
        match self {
            Self::Regular => icelines_core::season_stats::SeasonType::Regular,
            Self::Playoff => icelines_core::season_stats::SeasonType::Playoff,
        }
    }
}

// SiteSubcommand removed 2026-05-04 alongside the mkdocs-frontend cut.

#[derive(Debug, Subcommand)]
pub enum FetchSubcommand {
    /// Inventory ICELINES source surfaces for FLETCH handoff/gating.
    #[command(name = "fletch-sources")]
    FletchSources {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Season type: regular, playoff, or both.
        #[arg(long = "type", value_enum, default_value_t = FetchSeasonType::Regular)]
        season_type: FetchSeasonType,
        /// Write the handoff CSV to this path.
        #[arg(
            long,
            value_name = "PATH",
            default_value = "data/fletch-source-handoff.csv"
        )]
        out: PathBuf,
        /// Fail when registry validation or ICELINES handoff review checks fail.
        #[arg(long)]
        gate: bool,
    },
    /// Map ICELINES query surfaces onto FLETCH partition and rollup IDs.
    #[command(name = "fletch-partitions")]
    FletchPartitions {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Season type: regular, playoff, or both.
        #[arg(long = "type", value_enum, default_value_t = FetchSeasonType::Regular)]
        season_type: FetchSeasonType,
        /// Write the partition report JSON to this path.
        #[arg(
            long,
            value_name = "PATH",
            default_value = "data/fletch-query-partitions.json"
        )]
        out: PathBuf,
        /// Fail when partition metadata references missing FLETCH source IDs.
        #[arg(long)]
        gate: bool,
    },
    /// Group FLETCH query partitions into quiver handoff bundles.
    #[command(name = "fletch-quivers")]
    FletchQuivers {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Season type: regular, playoff, or both.
        #[arg(long = "type", value_enum, default_value_t = FetchSeasonType::Regular)]
        season_type: FetchSeasonType,
        /// Write the quiver handoff report JSON to this path.
        #[arg(
            long,
            value_name = "PATH",
            default_value = "data/fletch-query-quivers.json"
        )]
        out: PathBuf,
        /// Fail when quiver metadata is incomplete.
        #[arg(long)]
        gate: bool,
    },
    /// Map the ICELINES FLETCH cache manifest to cache-index evidence.
    #[command(name = "fletch-cache-index")]
    FletchCacheIndex {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Season type: regular, playoff, or both.
        #[arg(long = "type", value_enum, default_value_t = FetchSeasonType::Regular)]
        season_type: FetchSeasonType,
        /// Read this FLETCH cache manifest. Defaults to the ICELINES FLETCH manifest.
        #[arg(long, value_name = "PATH")]
        manifest: Option<PathBuf>,
        /// Write the cache-index evidence report JSON to this path.
        #[arg(
            long,
            value_name = "PATH",
            default_value = "data/fletch-cache-index.json"
        )]
        out: PathBuf,
        /// Fail when indexed rows are unverified or outside the ICELINES FLETCH registry.
        #[arg(long)]
        gate: bool,
    },
    /// Fetch all 32 team rosters (headshots, positions, bios).
    Rosters {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch skater season stats (G, A, GP, TOI).
    Stats {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
        /// Write per-player chunks (Phase 8h) instead of single bios.json
        /// and stats.json files. Daily-cadence storage shrinks ~10–15× via
        /// content-addressed dedup; readers fall back transparently.
        #[arg(long)]
        chunked: bool,
        /// Season type: regular (default), playoff, or both.
        #[arg(long = "type", value_enum, default_value_t = FetchSeasonType::Regular)]
        season_type: FetchSeasonType,
    },
    /// Fetch rosters then stats in one pass.
    All {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
        /// Same as `fetch stats --chunked` — applies to the stats step.
        #[arg(long)]
        chunked: bool,
        /// Season type: regular (default), playoff, or both. `playoff`
        /// runs bios+stats+goalies only (skips realtime/moneypuck/
        /// contracts — they're regular-season concepts). `both` runs
        /// the full regular pipe first, then the playoff trio.
        #[arg(long = "type", value_enum, default_value_t = FetchSeasonType::Regular)]
        season_type: FetchSeasonType,
    },
    /// Refresh position eligibility from boxscore data (Phase 2).
    Positions {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch NHL realtime stats (hits, blocks, giveaways, takeaways) — bundled with stats fetch.
    Realtime {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Download MoneyPuck xG, CF%, FF%, xGF% data (free public CSV).
    MoneyPuck {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Fetch this season plus N-1 prior regular seasons.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=38))]
        seasons: u8,
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch NHL contract data (expiry type, expiry year) from the player landing API.
    Contracts {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Phase Calder.2 — Fetch multi-league career history (AHL, OHL,
    /// NCAA, KHL, junior, international) for every player in the
    /// active stats snapshot. Writes to `~/.icelines/career_history.json`.
    ///
    /// `--bundled-seasons N` widens the source set: instead of the
    /// active snapshot, walks the last N bundled seasons' bios and
    /// unions the pids. Used to refresh the shipped bundle.
    Career {
        #[arg(long)]
        dry_run: bool,
        /// Walk the last N bundled seasons' bios (skaters + goalies)
        /// and union the pids. 0 (default) = use active snapshot.
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=38))]
        bundled_seasons: u8,
    },
    /// Fetch boxscores for one date and write score events to the
    /// EventStream (Phase Foster.3). With --for-favorites, only
    /// games involving favorited teams are persisted.
    Boxscore {
        /// Date in `YYYY-MM-DD` form. Defaults to today.
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        /// Restrict to games involving favorited teams (Foster.2 group).
        #[arg(long)]
        for_favorites: bool,
        /// Print what would be fetched without writing to disk / db.
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch play-by-play event JSON for one date. Needed for event-backed
    /// records and Rocket Richard scoring-event reports.
    #[command(name = "play-by-play", alias = "pbp")]
    PlayByPlay {
        /// Date in `YYYY-MM-DD` form. Defaults to today.
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        /// Restrict to games involving favorited teams.
        #[arg(long)]
        for_favorites: bool,
        /// Print what would be fetched without writing to disk.
        #[arg(long)]
        dry_run: bool,
    },
    /// Refresh every stale entry in the manifest (Phase Foster.4).
    /// Walks the manifest, filters by `Freshness::is_stale`, and
    /// re-fetches via the configured Fetcher. Non-blocking when
    /// invoked from the TUI; blocking from this CLI surface.
    Sync {
        /// List entries that would be refreshed without fetching.
        #[arg(long)]
        dry_run: bool,
        /// Override Static TTL — re-fetch even DataInstall-pinned
        /// entries. Use sparingly; intended for "user explicitly
        /// said refresh everything".
        #[arg(long)]
        force: bool,
    },
    /// Fetch goalie season stats (W/L/SV%/GAA/SO) — Phase G.2.
    /// Writes goalie-stats.json (or playoff-goalie-stats.json) into the
    /// active snapshot store.
    Goalies {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
        /// Season type: regular (default), playoff, or both.
        #[arg(long = "type", value_enum, default_value_t = FetchSeasonType::Regular)]
        season_type: FetchSeasonType,
    },
    /// Fetch league-wide transactions from ESPN — Phase T.3.
    /// Trades, waivers, signings, IR, recalls, reassignments. Writes
    /// transactions.json into the active snapshot store and updates the
    /// per-season `_meta.json` stale flag.
    Transactions {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Phase Lindsay L.1.6 — fetch one Tier-1 stats report per the
    /// catalog dispatch.
    ///
    /// Generic wrapper around the 9 Tier-1 NHL stats endpoints (skater
    /// summary/bios/realtime/timeonice/goalsForAgainst, goalie
    /// summary/bios/advanced/savesByStrength). Tier-2 endpoints are
    /// listed as `--kind` values for completeness but the CLI errors
    /// (Tier-2 lands in L.6 with the runtime `extra_reports` cache).
    ///
    /// Writes to `<snapshot_root>/<season>/<season_type>/<filename>`.
    /// Concurrent invocations are serialized via the fetch lock at
    /// `<icelines_home>/.fetch.lock` (TAPE-R3 rate-limit policy).
    Report {
        /// Which report to fetch (camelCase variant of the L.2 catalog).
        #[arg(long, value_enum)]
        kind: ReportKindArg,
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Single window — no `Both`. Use two `fetch report` invocations
        /// with `--type regular` and `--type playoff` for both windows.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Skip the fs lock at `<icelines_home>/.fetch.lock`. Use only
        /// when you accept the rate-limit risk of concurrent invocations
        /// (TAPE-R3 follow-up: error message references this flag).
        #[arg(long)]
        no_lock: bool,
        /// Print the URL + report kind WITHOUT issuing the HTTP call or
        /// writing to disk. Useful for verifying dispatch + path joins.
        #[arg(long)]
        dry_run: bool,
    },
}

/// CLI surface for `icelines-core::stats_catalog::ReportKind`. Mirror
/// of the core enum — clap derive `ValueEnum` lives on this side so the
/// core crate stays clap-independent. The mapping below is 1:1 by
/// variant name (camelCase via `clap(name = ...)`).
#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum ReportKindArg {
    // Tier 1
    #[clap(name = "skater-summary")]
    SkaterSummary,
    #[clap(name = "skater-bios")]
    SkaterBios,
    #[clap(name = "skater-realtime")]
    SkaterRealtime,
    #[clap(name = "skater-timeonice")]
    SkaterTimeOnIce,
    #[clap(name = "skater-goals-for-against")]
    SkaterGoalsForAgainst,
    #[clap(name = "goalie-summary")]
    GoalieSummary,
    #[clap(name = "goalie-bios")]
    GoalieBios,
    #[clap(name = "goalie-advanced")]
    GoalieAdvanced,
    #[clap(name = "goalie-saves-by-strength")]
    GoalieSavesByStrength,
    // Tier 2 (L.6)
    #[clap(name = "skater-puck-possessions")]
    SkaterPuckPossessions,
    #[clap(name = "skater-scoring-rates")]
    SkaterScoringRates,
    #[clap(name = "skater-summary-shooting")]
    SkaterSummaryShooting,
    #[clap(name = "skater-power-play")]
    SkaterPowerPlay,
    #[clap(name = "skater-penalty-kill")]
    SkaterPenaltyKill,
    #[clap(name = "skater-penalties")]
    SkaterPenalties,
    #[clap(name = "skater-faceoff-wins")]
    SkaterFaceoffWins,
    #[clap(name = "skater-faceoff-percentages")]
    SkaterFaceoffPercentages,
    #[clap(name = "skater-shot-type")]
    SkaterShotType,
    #[clap(name = "skater-scoring-per-game")]
    SkaterScoringPerGame,
    #[clap(name = "goalie-started-vs-relieved")]
    GoalieStartedVsRelieved,
    #[clap(name = "goalie-days-rest")]
    GoalieDaysRest,
    #[clap(name = "goalie-penalty-shots")]
    GoaliePenaltyShots,
    #[clap(name = "goalie-shootout")]
    GoalieShootout,
}

impl ReportKindArg {
    pub fn to_core(self) -> icelines_core::stats_catalog::ReportKind {
        use icelines_core::stats_catalog::ReportKind as R;
        match self {
            Self::SkaterSummary => R::SkaterSummary,
            Self::SkaterBios => R::SkaterBios,
            Self::SkaterRealtime => R::SkaterRealtime,
            Self::SkaterTimeOnIce => R::SkaterTimeOnIce,
            Self::SkaterGoalsForAgainst => R::SkaterGoalsForAgainst,
            Self::GoalieSummary => R::GoalieSummary,
            Self::GoalieBios => R::GoalieBios,
            Self::GoalieAdvanced => R::GoalieAdvanced,
            Self::GoalieSavesByStrength => R::GoalieSavesByStrength,
            Self::SkaterPuckPossessions => R::SkaterPuckPossessions,
            Self::SkaterScoringRates => R::SkaterScoringRates,
            Self::SkaterSummaryShooting => R::SkaterSummaryShooting,
            Self::SkaterPowerPlay => R::SkaterPowerPlay,
            Self::SkaterPenaltyKill => R::SkaterPenaltyKill,
            Self::SkaterPenalties => R::SkaterPenalties,
            Self::SkaterFaceoffWins => R::SkaterFaceoffWins,
            Self::SkaterFaceoffPercentages => R::SkaterFaceoffPercentages,
            Self::SkaterShotType => R::SkaterShotType,
            Self::SkaterScoringPerGame => R::SkaterScoringPerGame,
            Self::GoalieStartedVsRelieved => R::GoalieStartedVsRelieved,
            Self::GoalieDaysRest => R::GoalieDaysRest,
            Self::GoaliePenaltyShots => R::GoaliePenaltyShots,
            Self::GoalieShootout => R::GoalieShootout,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum GroupSubcommand {
    /// Create a new group.
    Create {
        name: String,
        #[arg(long)]
        desc: Option<String>,
    },
    /// Add a player to a group.
    Add { group: String, player: String },
    /// Remove a player from a group.
    Remove { group: String, player: String },
    /// List all groups.
    List,
    /// Show members of a group with current stats.
    Show { name: String },
    /// Delete a group.
    Delete { name: String },
    /// Export a group (members + metadata) to JSON.
    Export {
        name: String,
        /// Output path. Use `-` to write to stdout. Default: stdout.
        #[arg(long, default_value = "-")]
        out: String,
    },
    /// Import a group from a previously exported JSON file.
    Import {
        path: String,
        /// Override the group name from the file.
        #[arg(long = "as", value_name = "NAME")]
        as_name: Option<String>,
    },
    /// Rename an existing group (members carry over).
    Rename { old: String, new: String },
}

#[derive(Debug, Subcommand)]
pub enum GamesSubcommand {
    /// Record an NHL game you attended in person.
    /// `game_id` is the 10-digit NHL ID (visible in the Scores tab).
    Add {
        game_id: u64,
        /// Freeform note — "took my dad", "Ovechkin's 800th", etc.
        #[arg(long, default_value = "")]
        note: String,
    },
    /// Remove a game from your attended list.
    Remove { game_id: u64 },
    /// List every game you've recorded as attended.
    List,
    /// Export the attended-games list to CSV or JSON (default: stdout, CSV).
    Export {
        /// Output path. Use `-` to write to stdout.
        #[arg(long, default_value = "-")]
        out: String,
        /// Emit JSON instead of CSV.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SchemeSubcommand {
    /// List all available schemes (built-in + user-defined).
    List,
    /// Show scoring weights for a named scheme.
    Show {
        name: String,
        /// Print the scheme as JSON instead of the human-readable table.
        /// Useful for copy/paste, diff, or piping into another tool.
        #[arg(long)]
        source: bool,
    },
    /// Detect scoreable stats from a fantasy CSV export and create a scheme
    /// template. Auto-detects the platform (Yahoo / ESPN / Sleeper / Fantrax)
    /// from header columns; pass `--platform` to override.
    FromCsv {
        path: String,
        #[arg(long)]
        name: Option<String>,
        /// Force a specific platform: yahoo, espn, sleeper, or fantrax.
        /// Bypasses auto-detection.
        #[arg(long)]
        platform: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum DataSubcommand {
    /// Fetch season data from original sources for local use.
    Install {
        /// Fetch last N seasons [default: 1 = current season refresh].
        #[arg(long, default_value_t = 1)]
        seasons: u8,
        /// Fetch a specific season (e.g. 20212022).
        #[arg(long)]
        season: Option<String>,
        /// Re-fetch even if the season already has local source snapshots.
        #[arg(long)]
        force: bool,
    },
    /// List installed season bundles.
    List,
    /// Remove an installed season bundle.
    Remove { season: String },
    /// Verify SHA-256 hashes of an installed bundle's files against the
    /// manifest written at install time. Detects partial downloads and
    /// post-install tampering.
    Verify {
        /// Season to verify (e.g. 20242025). Conflicts with `--all`.
        season: Option<String>,
        /// Verify every installed bundle.
        #[arg(long)]
        all: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SnapshotSubcommand {
    /// List all snapshots with tier, date, and sealed status.
    List,
    /// Show full detail for a named snapshot (files, integrity, parent chain).
    Show { name: String },
    /// Set the active snapshot (must be sealed).
    Use { name: String },
    /// Re-verify integrity hashes for a snapshot (or the active one).
    Verify { name: Option<String> },
    /// Delete a named snapshot (cannot delete the active one).
    Delete { name: String },
    /// Convert a legacy snapshot to the chunked layout (Phase 8h).
    /// Idempotent — already-chunked snapshots are a no-op.
    Rebuild {
        name: String,
        /// Use the chunked storage format (Phase 8h). Required flag for
        /// forward-compat with future rebuild modes.
        #[arg(long)]
        chunked: bool,
    },
    /// Sweep zero-ref chunks from the chunk store (Phase 8h).
    Gc {
        /// Report what would be removed without touching disk.
        #[arg(long)]
        dry_run: bool,
    },
    /// Keep the newest N sealed snapshots per tier; delete the rest
    /// (Phase 8f.2). Active snapshot is always preserved. Drafts are
    /// excluded from the keep count. Pair with `snapshot gc` to free
    /// chunks newly orphaned by the prune.
    Prune {
        #[arg(long)]
        keep: usize,
        /// Report what would be deleted without touching disk.
        #[arg(long)]
        dry_run: bool,
    },
    /// Compare two chunked snapshots — reports added / removed players
    /// plus bios/stats hash mismatches (Phase 8f.3).
    Diff {
        /// First snapshot name.
        a: String,
        /// Second snapshot name.
        b: String,
    },
}

// ── Phase 5 query subcommands ─────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum QuerySubcommand {
    /// Ranked leaderboard — filter by any combination of demographics and stats.
    #[command(long_about = r#"
icelines query leaders — top-N players by any of 30+ sort metrics.

EXAMPLES
  icelines query leaders --top 20
  icelines query leaders --pos C --sort ppg --top 15
  icelines query leaders --season 19921993 --filter "g>=50"
  icelines query leaders --age-max 24 --filter "hits>=200" --filter "p>=40"
  icelines query leaders --seasons 3 --filter "g>=60" --filter "a>=60"
  icelines query leaders --json | jq '.[0]'

FILTER GRAMMAR (each --filter is a full boolean expression)
  <expr>     := <or-expr>
  <or-expr>  := <and-expr> ( OR  <and-expr> )*
  <and-expr> := <unary>    ( AND <unary>    )*
  <unary>    := NOT <unary> | <primary>
  <primary>  := '(' <expr> ')' | <atom>
  <atom>     := <stat> <op> <value>

  ops: >=  <=  >  <  ==
  Precedence: NOT > AND > OR (standard). Keywords case-insensitive.
  Multiple --filter flags ANDed at top level (so --filter "a>=20 OR b>=20"
  --filter "gp>=70" means "(a OR b) AND gp>=70").

  Stats: any cli_key from the catalog (108 stats) or a short alias:
    g  → goals       a  → assists      p, pts → points     gp → games
    s  → shots       blk → blocked-shots  ppg → points-per-game
    +/- → plus-minus  pim → pim         tk / gv → takeaways/giveaways
  Filter keys are case-insensitive (HITS works the same as hits).
  Note: `age` is NOT a stat; use --age-min N / --age-max N flags below.

  Examples:
    --filter "g>=50 OR a>=80"
    --filter "(g>=30 AND a>=30) OR p>=80"
    --filter "NOT pim>=100"

SORTS (use canonical cli_key or alias)
  pts-pace (default), ppg, pts/p, goals/g, assists/a, gp, hits, blocks,
  pp-pts-pace, pp-g-pace, sh-g-pace, gwg-pace, shots-pace, shooting-pct,
  plus-minus, toi, fo-pct, takeaways, giveaways, pim, xg, xg-per-60,
  cf-pct, ff-pct, xgf-pct, improvement (Y/Y PPG delta).

OUTPUT
  --json / --csv to stdout, or --out PATH to a file.
"#)]
    Leaders {
        /// Position filter: C, LW, RW, D, F (all forwards), G
        #[arg(long)]
        pos: Option<String>,
        /// Filter to players on this team (e.g. SEA, NYR)
        #[arg(long)]
        team: Option<String>,
        /// Minimum age (inclusive)
        #[arg(long)]
        age_min: Option<u8>,
        /// Maximum age (inclusive)
        #[arg(long)]
        age_max: Option<u8>,
        /// ISO-3166 alpha-3 nationality code(s), comma-separated (e.g. FIN, SWE)
        #[arg(long)]
        nationality: Option<String>,
        /// Draft year (e.g. 2022)
        #[arg(long)]
        draft_year: Option<u16>,
        /// Draft round (1–7)
        #[arg(long = "draft-round")]
        round: Option<u8>,
        /// Maximum overall draft pick number
        #[arg(long)]
        draft_pick_max: Option<u16>,
        /// Only undrafted players
        #[arg(long)]
        undrafted: bool,
        /// Only rookies (first NHL season)
        #[arg(long)]
        rookie: bool,
        /// Handedness: L or R
        #[arg(long)]
        handedness: Option<String>,
        /// Minimum points per game (e.g. 0.80)
        #[arg(long)]
        ppg_min: Option<f64>,
        /// Minimum games played
        #[arg(long)]
        gp_min: Option<u32>,
        /// Maximum games played
        #[arg(long)]
        gp_max: Option<u32>,
        /// Minimum average TOI per game in minutes (e.g. 18.5)
        #[arg(long)]
        toi_min: Option<f32>,
        /// Minimum plus/minus rating (e.g. 5 or -5)
        #[arg(long)]
        plus_minus_min: Option<i32>,
        /// Minimum shots per game (e.g. 3.0)
        #[arg(long)]
        shots_pg_min: Option<f32>,
        /// Birth province/state filter, comma-separated (e.g. ON,QC for Ontario/Quebec)
        #[arg(long)]
        birth_province: Option<String>,
        /// Aggregate across N seasons (1–5, default 1 = current season only)
        #[arg(long, default_value_t = 1)]
        seasons: u8,
        /// Phase Foster +26 — aggregate the last 7 days of game logs
        /// instead of season totals. Reads per-game stats from the
        /// boxscore manifest (Foster +3 persistence). Mutually
        /// exclusive with `--month`; both override `--seasons`.
        #[arg(long)]
        week: bool,
        /// Phase Foster +26 — aggregate the calendar month containing
        /// today's date.
        #[arg(long)]
        month: bool,
        /// Phase Conn Smythe C.2 — Cup-run leaderboard. Aggregates
        /// every persisted boxscore with `gameType=3` (playoffs).
        /// Mutually exclusive with `--week` / `--month`.
        #[arg(long)]
        playoff: bool,
        /// Phase Art Ross A.5 — print the parsed query plan
        /// (constraint tree + data requirements) and exit
        /// without running the query. Useful for debugging
        /// complex filters and seeing how the planner routes
        /// atoms across legacy / sliding-window / career-
        /// aggregate / cross-league sub-evaluators. Pair with
        /// `--json` for the `explain.v1` envelope shape.
        #[arg(long)]
        explain: bool,
        /// Query a specific historical season (e.g. 20242025). Conflicts with
        /// --seasons N. Must match a bundled season — see icelines-fetch::BUNDLED_SEASONS.
        #[arg(long)]
        season: Option<String>,
        /// Season type: regular (default) or playoff. `playoff` requires
        /// the season's playoff data to be bundled or installed.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Sort metric: pts-pace (default), ppg, g-pace, gpg, pts, goals, assists, gp,
        ///   pp-pts-pace, pp-g-pace, pp-pts, pp-g, sh-g-pace, sh-g, gwg-pace, gwg,
        ///   shots-pace, shots, sh-pct, plus-minus, toi, fo-pct,
        ///   hits-pace, hits, blocks-pace, blocks, takeaways, giveaways, pim,
        ///   xg, xg-per-60, cf-pct, ff-pct, xgf-pct, improvement
        #[arg(long, default_value = "pts-pace")]
        sort: String,
        /// Number of results to show
        #[arg(long, default_value_t = 20)]
        top: usize,
        /// Show per-game rates instead of per-82 projections
        #[arg(long)]
        rate: bool,
        /// Show league percentile rank for each result
        #[arg(long)]
        percentiles: bool,
        /// Export as JSON
        #[arg(long)]
        json: bool,
        /// Export a Web-compatible JSON envelope with data and meta.
        #[arg(long)]
        json_envelope: bool,
        /// Export as CSV
        #[arg(long)]
        csv: bool,
        /// Only players on UFA contracts (unrestricted free agents)
        #[arg(long)]
        ufa: bool,
        /// Only players on RFA contracts (restricted free agents)
        #[arg(long)]
        rfa: bool,
        /// Only players on entry-level contracts
        #[arg(long)]
        elc: bool,
        /// Filter by contract expiry year (e.g. 2026)
        #[arg(long)]
        expiry_year: Option<u16>,
        /// Phase Lindsay L.3.1 — generic stat filter. Repeatable; multiple
        /// --filter flags accumulate (implicit AND). Grammar:
        /// `<stat-key><op><value>` where op is `>=` `<=` `==` `=`.
        ///
        /// Examples:
        ///   --filter "hits>=20" --filter "blocks>=10"
        ///   --filter "save-pct>=.910" --filter "gp>=15"
        ///
        /// Stat keys: any catalog cli_key (run `--help` for the full list,
        /// or see `design/specs/stat-catalog.md`).
        #[arg(long = "filter", value_name = "STAT-KEY OP VALUE")]
        filters: Vec<String>,
    },

    /// Deep dive on a single player — career arc, league rank, percentiles.
    /// Deep dive on a single player — career arc, league rank, percentiles.
    #[command(long_about = r#"
icelines query player NAME — full profile for a single player.

Searches both skater AND goalie bios; historical players resolve without
--season via cross-bundled name lookup.

EXAMPLES
  icelines query player "Connor McDavid"                       # current season
  icelines query player "Connor McDavid" --seasons 38          # full bundled career
  icelines query player "McDavid" --percentiles
  icelines query player "McDavid" --rank-by g --percentiles    # rank by goals not pts
  icelines query player "McDavid" --filter "gp>=60"            # narrow peer pool
  icelines query player "Wayne Gretzky"                        # historical (no --season needed)
  icelines query player "Patrick Roy" --season 19951996        # historical goalie

NOTES
  - --seasons N (default 38) controls how many bundled seasons land in the
    career arc (1 = current only, 38 = full bundled history).
  - --filter narrows the percentile peer pool (e.g. only score-vs-similar-GP).
  - --rank-by overrides the default Pts/82 ranking with any cli_key.
"#)]
    Player {
        /// Player name (partial match OK)
        name: String,
        /// Breakdown mode: career (default) | situation
        #[arg(long, default_value = "career")]
        breakdown: String,
        /// Show league percentile rank
        #[arg(long)]
        percentiles: bool,
        /// Last N games rolling (requires Phase 5C game-log data)
        #[arg(long)]
        last_n: Option<u32>,
        /// Query a specific historical season (e.g. 20242025).
        #[arg(long)]
        season: Option<String>,
        /// Season type: regular (default) or playoff.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Phase Lindsay L.5.3b — override the percentile/rank metric
        /// to any catalog `cli_key` (or any legacy `--sort` alias).
        /// Default behavior (omit flag) ranks by Pts/82.
        /// Example: `--rank-by goals-per-60`.
        #[arg(long)]
        rank_by: Option<String>,
        /// Phase Lindsay D1b — narrow the percentile peer pool. Same
        /// grammar as `query leaders --filter`. Repeatable. Example:
        /// `--filter "gp>=20"` ranks the target only against same-pos
        /// peers with ≥20 GP (filters out call-ups distorting the
        /// curve).
        #[arg(long = "filter")]
        filters: Vec<String>,
        /// Gaps.2 — number of bundled seasons to include in the career
        /// arc (newest-first). Default 38 = full history. Set to 5 for
        /// the pre-L.7b "modern era" view.
        #[arg(long, default_value_t = 38)]
        seasons: u8,
    },

    /// Side-by-side comparison or similarity search.
    #[command(long_about = r#"
icelines query compare PLAYER1 [PLAYER2] — head-to-head or similarity search.

EXAMPLES
  icelines query compare "Connor McDavid" "Sidney Crosby"
  icelines query compare "Wayne Gretzky" "Mario Lemieux" --seasons 38
  icelines query compare "Matty Beniers" --similar 8
  icelines query compare "McDavid" --similar 5 --filter "gp>=20"

MODES
  Two players → head-to-head table (per-stat side-by-side).
  --similar N → N most-similar peers via Z-score distance (Phase Lindsay L.5.2).

NOTES
  - --seasons N prints each player's career arc after the head-to-head.
  - --filter narrows the similarity cohort (only when --similar is set).
  - Historical players resolve without --season (cross-bundled name lookup).
"#)]
    Compare {
        /// First player name (partial match OK)
        player1: String,
        /// Second player for head-to-head (omit to use --similar)
        player2: Option<String>,
        /// Find N most similar players using Z-score distance
        #[arg(long)]
        similar: Option<usize>,
        /// Similarity metric: ppg (default) | career-arc
        #[arg(long, default_value = "ppg")]
        by: String,
        /// Query a specific historical season (e.g. 20242025).
        #[arg(long)]
        season: Option<String>,
        /// Season type: regular (default) or playoff.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Phase Lindsay D1b — narrow the similarity cohort (only
        /// applies when `--similar N` is set). Same grammar as `query
        /// leaders --filter`. Repeatable. Example: `--filter "age>=22"
        /// --filter "gp>=10"` finds similar-aged peers with enough
        /// games for a stable comparison.
        #[arg(long = "filter")]
        filters: Vec<String>,
        /// Gaps.3 — number of bundled seasons to include in each
        /// player's career line for head-to-head context. Default 38 =
        /// full history. Active only on head-to-head (player2 set);
        /// `--similar` is single-season Z-score and ignores this.
        #[arg(long, default_value_t = 38)]
        seasons: u8,
    },

    /// Goalie leaderboard — Phase G.5.
    /// `sv-pct` (default) | `gaa` | `wins` | `gp` | `saves` | `so`.
    #[command(long_about = r#"
icelines query goalies — top-N goalies by save % / GAA / wins / etc.

EXAMPLES
  icelines query goalies --top 10
  icelines query goalies --filter "gp>=30" --filter "save-pct>=0.92"
  icelines query goalies --filter "wins>=30" --filter "so>=4"
  icelines query goalies --season 19981999 --top 10                  # Hasek era
  icelines query goalies --json

GOALIE FILTER REWRITE (Gaps.4)
  Skater-context keys auto-rewrite: `gp` → `goalie-games`, `starts` →
  `goalie-starts`. So you can type `gp>=15` naturally without knowing
  the goalie cli_key namespace.

ALIASES
  w  → wins        l  → losses        ot → ot-losses     so → shutouts
  sv → saves       sa → shots-against ga → goals-against
  sv%, save%       → save-pct

OUTPUT
  --json / --csv to stdout, --out PATH to a file.
"#)]
    Goalies {
        /// Number of goalies to show.
        #[arg(long, default_value_t = 20)]
        top: usize,
        /// Sort metric: sv-pct | gaa | wins | gp | saves | so.
        #[arg(long, default_value = "sv-pct")]
        sort: String,
        /// Filter to one team (e.g. WPG).
        #[arg(long)]
        team: Option<String>,
        /// Minimum GP threshold. Default 15 (NHL leaderboard convention).
        #[arg(long, default_value_t = 15)]
        min_gp: u32,
        /// Query a specific historical season (e.g. 20242025).
        #[arg(long)]
        season: Option<String>,
        /// Season type: regular (default) or playoff.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Export as JSON.
        #[arg(long)]
        json: bool,
        /// Export as CSV.
        #[arg(long)]
        csv: bool,
        /// Phase Lindsay L.5b post-fix (II-06 partial roll) — generic
        /// stat filter, repeatable. Same grammar as `query leaders
        /// --filter`. Example: `--filter "save-pct>=0.910" --filter
        /// "goalie-games>=20"`.
        #[arg(long = "filter")]
        filters: Vec<String>,
    },
    /// Phase Calder.4 — cross-league cohort leaderboard.
    ///
    /// Lists top scorers from a non-NHL league/season window, drawn
    /// from the local career-history store populated by
    /// `icelines fetch career`. Useful for "OHL leaders 2014-15" or
    /// "AHL goal-scorers 2024-25" queries the existing
    /// `query leaders` (NHL-only) can't answer.
    #[command(long_about = r#"
icelines query career — top-N players in a non-NHL league/season.

EXAMPLES
  icelines query career --league OHL --season 20142015
  icelines query career --league AHL --season 20242025 --top 30
  icelines query career --league NCAA --season 20132014 --sort goals
  icelines query career --league WHL --json | jq '.data[0]'

NOTES
  - Source: ~/.icelines/career_history.json (run `icelines fetch
    career` first to populate the store).
  - Cohort scope: only players who have an NHL roster appearance in
    the last 5 bundled seasons (the fetch target). Career history
    for players who never reached the NHL is not in scope.
  - --season defaults to the most-recent season for the chosen
    league when omitted.
"#)]
    Career {
        /// League abbreviation (e.g. OHL, WHL, QMJHL, AHL, NCAA, KHL,
        /// SHL, Liiga). Case-insensitive.
        #[arg(long)]
        league: String,
        /// Season in YYYYZZZZ form (e.g. 20142015). Defaults to the
        /// most recent season for that league in the store.
        #[arg(long)]
        season: Option<String>,
        /// Number of rows to show.
        #[arg(long, default_value_t = 20)]
        top: usize,
        /// Sort metric: points (default) | goals | assists | gp | ppg.
        #[arg(long, default_value = "points")]
        sort: String,
        /// Emit JSON envelope (King.2.4 shape).
        #[arg(long)]
        json: bool,
        /// Emit CSV (one row per player).
        #[arg(long)]
        csv: bool,
        /// Phase Foster.5 EDGE B2 — `--week` / `--month` on `query
        /// career` is intentionally rejected (junior seasons aren't
        /// aligned with NHL week boundaries). Flag is hidden + present
        /// only so the rejection error message is helpful.
        #[arg(long, hide = true)]
        week: bool,
        #[arg(long, hide = true)]
        month: bool,
        /// Phase Art Ross — narrow the cohort with a Phase Art Ross
        /// filter. Same grammar as `query leaders --filter`. Each
        /// filter is AND-joined.
        ///
        /// Bio atoms work cleanly here (`country=CAN`, `pos=C`,
        /// `age<=18`, `draft-round<=2`). Stat atoms (`g>=10`,
        /// `g.last10g>=5`, `p.career>=500`) evaluate against the
        /// player's NHL career, not their non-NHL league stats —
        /// useful for "OHL leaders who later became NHL 30-goal
        /// scorers" but not for "OHL leaders with 80+ OHL points"
        /// (use `--sort` for that).
        ///
        /// Examples:
        ///   --filter "country=CAN AND pos=C"
        ///   --filter "age<=18"
        ///   --filter "draft-round<=2"
        #[arg(long = "filter")]
        filters: Vec<String>,
    },
}

// ── Fantasy sub-commands ──────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum FantasySubcommand {
    // League management
    /// Create a new fantasy league.
    LeagueCreate {
        name: String,
        #[arg(long, default_value = "yahoo-standard")]
        scheme: String,
    },
    /// List all fantasy leagues.
    LeagueList,
    /// Set the active fantasy league (alias: league-switch).
    LeagueUse { name: String },
    /// Switch to a different league (alias for league-use).
    LeagueSwitch { name: String },
    /// Delete a fantasy league and all its teams.
    LeagueDelete { name: String },

    // Team management
    /// Create a new fantasy team in the active league.
    TeamCreate {
        name: String,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        league: Option<String>,
    },
    /// List all teams in the active league.
    TeamList {
        #[arg(long)]
        league: Option<String>,
    },
    /// Mark a team as your roster in the active league.
    TeamUse {
        name: String,
        #[arg(long)]
        league: Option<String>,
    },
    /// Show a team's roster with current stats and fantasy score.
    TeamShow {
        name: String,
        #[arg(long)]
        league: Option<String>,
    },
    /// Add a player to a fantasy team.
    TeamAdd {
        team: String,
        player: String,
        #[arg(long)]
        league: Option<String>,
    },
    /// Drop a player from a fantasy team.
    TeamDrop {
        team: String,
        player: String,
        #[arg(long)]
        league: Option<String>,
    },

    // Scoring
    /// Show fantasy standings for the active league.
    Standings {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        scheme: Option<String>,
    },

    /// Show category gaps for your marked team against available skaters.
    Gaps {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        scheme: Option<String>,
        /// Comma-separated categories to inspect, e.g. hits,blocks,shots.
        #[arg(long = "category", value_delimiter = ',')]
        categories: Vec<String>,
        #[arg(long, default_value_t = 8)]
        top: usize,
        #[arg(long)]
        json: bool,
    },

    /// Simulate fantasy team standings and optional add/drop scenarios.
    Simulate {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        scheme: Option<String>,
        /// Projection horizon in weeks.
        #[arg(long, default_value_t = 4)]
        weeks: u8,
        /// Candidate player to add in the scenario.
        #[arg(long = "add")]
        add_player: Option<String>,
        /// Player to drop in the scenario.
        #[arg(long = "drop")]
        drop_player: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Show fantasy points earned on a date from cached finalized boxscores.
    Daily {
        /// Date to score, in YYYY-MM-DD format.
        #[arg(long)]
        date: chrono::NaiveDate,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        #[arg(long)]
        json: bool,
    },

    /// Show fantasy head-to-head matchups for the week containing a date.
    Matchup {
        /// Date inside the matchup week, in YYYY-MM-DD format.
        #[arg(long)]
        date: chrono::NaiveDate,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        #[arg(long)]
        json: bool,
    },

    /// Add a local fantasy matchup schedule row for a week.
    MatchupSet {
        /// Date inside the matchup week, in YYYY-MM-DD format.
        #[arg(long)]
        week: chrono::NaiveDate,
        #[arg(long)]
        home: String,
        /// Opponent team. Omit for a bye.
        #[arg(long)]
        away: Option<String>,
        #[arg(long)]
        league: Option<String>,
    },

    /// Import a Yahoo roster CSV into the local FantasyDb.
    ImportYahoo {
        /// Yahoo roster CSV export to parse.
        #[arg(long)]
        file: PathBuf,
        /// Fantasy league name to preview or create/update.
        #[arg(long)]
        league: String,
        /// Mark this fantasy team as your roster after import.
        #[arg(long = "my-team")]
        my_team: Option<String>,
        /// Preview diagnostics without writing FantasyDb changes.
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },

    /// Show the active league roster-shape preset and built-ins.
    RosterShape {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Set the roster-shape preset for a league.
    RosterShapeSet {
        shape: String,
        #[arg(long)]
        league: Option<String>,
    },

    /// Validate one team or every team against the persisted roster shape.
    RosterShapeValidate {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        json: bool,
    },

    // Trade
    /// Evaluate or execute a player trade between two teams.
    Trade {
        player1: String,
        #[arg(long)]
        to_team: String,
        #[arg(long = "for-player")]
        for_player: String,
        #[arg(long)]
        execute: bool,
        #[arg(long)]
        league: Option<String>,
    },

    // Server
    /// Start a fantasy league HTTP dashboard server.
    Serve {
        #[arg(long, default_value_t = 8080)]
        port: u16,
        #[arg(long)]
        league: Option<String>,
    },
}

// ── Phase Foster.0.7: capability + sync config ───────────────────────────────

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Print the value at `key`.
    Get { key: String },
    /// Set `key` to `value`. Validates against the typed schema and
    /// rejects unknown keys / invalid values with a clear error.
    Set { key: String, value: String },
    /// Print every settable key + its current value.
    List,
    /// Reset a section to defaults. Recognized: `sync`,
    /// `sync.capabilities`.
    Reset { key: String },
}

#[derive(Debug, Subcommand)]
pub enum LayoutSubcommand {
    /// List saved layouts.
    List,
    /// Show one saved layout as JSON.
    Show {
        /// Layout name.
        name: String,
    },
    /// Save or replace a named layout.
    Save {
        /// Layout name.
        name: String,
        /// Center workbench slug, e.g. league, scores, stats.
        #[arg(long)]
        center: String,
        /// Left pane binding slug, e.g. favorites-left.
        #[arg(long)]
        left: String,
        /// Right pane binding slug, e.g. schedule-right.
        #[arg(long)]
        right: String,
        /// Optional experience slug, e.g. tonight-bench.
        #[arg(long)]
        experience: Option<String>,
    },
    /// Update an existing layout, or create it if missing.
    Update {
        /// Layout name.
        name: String,
        /// Center workbench slug, e.g. league, scores, stats.
        #[arg(long)]
        center: String,
        /// Left pane binding slug, e.g. favorites-left.
        #[arg(long)]
        left: String,
        /// Right pane binding slug, e.g. schedule-right.
        #[arg(long)]
        right: String,
        /// Optional experience slug, e.g. tonight-bench.
        #[arg(long)]
        experience: Option<String>,
    },
    /// Delete a saved layout.
    Delete {
        /// Layout name.
        name: String,
    },
}

// ── Phase 8d: markdown export ────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum ExportSubcommand {
    /// Write a markdown table for the given shape. Output is deterministic
    /// and proof-DASHBOARD-SPEC ready.
    Md {
        /// What to export. See `design/specs/export-markdown.md`.
        #[arg(value_enum)]
        shape: MdShape,
        /// Output path. Default: `~/.icelines/reports/{shape}.md`.
        /// Pass `-` for stdout.
        #[arg(long)]
        out: Option<String>,

        // ── Filters per shape ────────────────────────────────────────────
        /// Position filter for `leaders` / `roster` (`C`, `LW`, `RW`, `D`, `F`, `G`).
        #[arg(long)]
        pos: Option<String>,
        /// Team abbrev for `team` / `team-season` (e.g. `SEA`).
        #[arg(long)]
        team: Option<String>,
        /// Top-N for `leaders`.
        #[arg(long, default_value_t = 25)]
        top: usize,
        /// Season for `leaders`, e.g. 20242025. Defaults to current season.
        #[arg(long)]
        season: Option<String>,
        /// Season type for `leaders`.
        #[arg(long = "type", value_enum, default_value_t = QuerySeasonType::Regular)]
        season_type: QuerySeasonType,
        /// Sort metric for `leaders`. Defaults to `pts-pace`.
        #[arg(long, default_value = "pts-pace")]
        sort: String,
        /// Min-GP filter for `leaders` (default = current MIN_GP).
        #[arg(long)]
        gp_min: Option<u32>,
        /// Free-form filter for `leaders`. Repeatable; uses the same grammar as
        /// `query leaders --filter`.
        #[arg(long = "filter")]
        filters: Vec<String>,
        /// Phase Lindsay L.5.4 — comma-separated `StatId::cli_key` list
        /// for the `leaders` shape. Replaces the canonical column set.
        /// Example: `--columns "goals,assists,points,hits,blocked-shots"`.
        /// Unknown keys exit non-zero with a list of valid keys.
        #[arg(long)]
        columns: Option<String>,
        /// First player for `compare`.
        #[arg(long)]
        p1: Option<String>,
        /// Second player for `compare`.
        #[arg(long)]
        p2: Option<String>,
        /// Player for `signals`.
        #[arg(long)]
        player: Option<String>,
        /// Series letter for `series` (e.g. `A`).
        #[arg(long)]
        series: Option<String>,

        // ── Render hints (forwarded to YAML front-matter) ────────────────
        /// Suggested terminal width — proof reflows to fit.
        #[arg(long, default_value_t = 100)]
        width: u16,
        /// Suggested terminal height — proof truncates if needed.
        #[arg(long, default_value_t = 30)]
        height: u16,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum MdShape {
    /// Top-N leaderboard (mirrors `query leaders`).
    Leaders,
    /// Single team's lineup card.
    Team,
    /// Single team's season-performance report.
    TeamSeason,
    /// Cross-team line-value rankings.
    Depth,
    /// Active fantasy league standings.
    Fantasy,
    /// Two-player head-to-head.
    Compare,
    /// Single-player Signals report.
    Signals,
    /// One playoff series — game log + scorers.
    Series,
    /// All teams' rosters in one big table.
    Roster,
}

impl MdShape {
    /// Lowercase label used in default output filenames + front-matter.
    pub fn label(&self) -> &'static str {
        match self {
            MdShape::Leaders => "leaders",
            MdShape::Team => "team",
            MdShape::TeamSeason => "team-season",
            MdShape::Depth => "depth",
            MdShape::Fantasy => "fantasy",
            MdShape::Compare => "compare",
            MdShape::Signals => "signals",
            MdShape::Series => "series",
            MdShape::Roster => "roster",
        }
    }
}

/// Report shapes accepted by the unified `icelines x <shape>` command.
/// Each maps to an existing report and emits CSV by default.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExportShape {
    /// Top-N league rank by pace score (mirrors `icelines rank`).
    Rank,
    /// League-wide pace leaderboard (mirrors `icelines query leaders`).
    Leaders,
    /// Goalie leaderboard (mirrors `icelines query goalies`).
    Goalies,
    /// Filter players by criteria (mirrors `icelines players`).
    Players,
    /// Draft class for a year (mirrors `icelines class`).
    Class,
    /// Career season-by-season log for one player (mirrors `icelines history`).
    History,
    /// Statistical peers for one player (mirrors `icelines peers`).
    Peers,
    /// Two-player comparison (mirrors `icelines compare`).
    Compare,
    /// League-wide transactions feed (mirrors `icelines transactions`).
    Transactions,
}
