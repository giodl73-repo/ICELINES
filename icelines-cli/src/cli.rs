use std::path::PathBuf;

use crate::commands::signals::SignalEvidenceFilter;
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
    /// Without a config file, interactive runs open setup. Headless or
    /// scripted callers pass this flag to bypass it.
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

    /// Curated stathead/editorial query starter packs.
    #[command(long_about = r#"
Curated stathead/editorial query starter packs.

This command does not introduce new metric semantics; it prints reusable query
recipes for existing IceLines surfaces. Use it when you want a starting point
for historical leaderboards, young-player lists, playoff runs, or goalie notes.

Examples:
  icelines stathead
  icelines stathead young-stars
  icelines stathead era-leaders --json
  icelines stathead goalie-notebook --markdown --out goalie-notebook.md
  icelines stathead young-stars --commands
  icelines stathead fantasy-prep --commands --read-only
  icelines stathead --commands --writes-only

Output modes:
  --json                      machine-readable pack metadata
  --markdown [--out PATH]     Markdown report artifact
  --commands [--out PATH]     runnable commands only
  --commands --read-only      omit recipes that write files
  --commands --writes-only    show only recipes that write files
"#)]
    Stathead {
        /// Pack slug to show. Omit to list available packs.
        pack: Option<String>,

        /// Emit pack metadata and commands as JSON.
        #[arg(long, conflicts_with_all = ["markdown", "commands_only"])]
        json: bool,

        /// Render the selected pack as Markdown.
        #[arg(long, conflicts_with = "commands_only")]
        markdown: bool,

        /// Print only runnable commands, one per line.
        #[arg(long = "commands")]
        commands_only: bool,

        /// With --commands, omit recipes that write files.
        #[arg(long, requires = "commands_only", conflicts_with = "writes_only")]
        read_only: bool,

        /// With --commands, emit only recipes that write files.
        #[arg(long, requires = "commands_only")]
        writes_only: bool,

        /// Write Markdown or commands output to a file.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

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
        /// Emit the shared DataStatusView JSON envelope.
        #[arg(long)]
        json: bool,
    },

    /// First-run setup wizard (Phase Foster.0.8).
    ///
    /// Three-question flow that writes capability matrix defaults to
    /// `~/.icelines/config.toml`. Auto-runs on first interactive
    /// terminal invocation when no config file exists; pass
    /// `--no-setup` (top-level) to skip. In scripted contexts pass
    /// `--accept-defaults` to write the defaults non-interactively.
    /// Existing config files are left unchanged unless `--reset` is
    /// passed.
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
  icelines signals-roster --team NYR --evidence partial
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
        /// Filter rows by Signal evidence coverage without changing name sorting.
        #[arg(long, value_enum, default_value_t = SignalEvidenceFilter::All)]
        evidence: SignalEvidenceFilter,
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

    /// Forecast NHL games and seasons from The Goal Line.
    #[command(subcommand)]
    Icecast(IceCastSubcommand),
}

#[derive(Debug, Subcommand)]
pub enum IceCastSubcommand {
    /// Gather and rank three seasons of all-team management behavior evidence.
    #[command(name = "behavior-rankings")]
    BehaviorRankings {
        /// Season being forecast; completed evidence seasons precede it.
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        target_season: u32,
        /// Number of completed seasons to gather.
        #[arg(long, default_value_t = 3)]
        window: u8,
        #[arg(long)]
        json: bool,
        /// Write the full UI-neutral evidence and ranking document as JSON.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply citation-backed GM/coach research to one calibrated team profile.
    #[command(name = "behavior-research")]
    BehaviorResearch {
        /// UI-neutral output from `icecast behavior-rankings`.
        #[arg(long, value_name = "PATH")]
        rankings: PathBuf,
        /// `team_behavior_research.v1` leadership timeline and marker document.
        #[arg(long, value_name = "PATH")]
        research: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Simulate training-camp cuts and opening-roster probabilities in The Cut.
    Camp {
        /// `training_camp_simulation_input.v1`-compatible JSON document.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Override the input trial count.
        #[arg(long)]
        trials: Option<u32>,
        /// Override the input simulation seed.
        #[arg(long)]
        seed: Option<u64>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Write reusable probabilistic Cut-to-Blender lineup branches.
        #[arg(long, value_name = "PATH")]
        lineup_set_out: Option<PathBuf>,
        /// Number of most-common camp roster branches to retain.
        #[arg(long, default_value_t = 5)]
        max_lineup_branches: usize,
        /// Score retained camp branches through The Blender.
        #[arg(long, value_name = "PATH")]
        blender_set_out: Option<PathBuf>,
        /// Write an IceCast scenario that samples one camp roster per season trial.
        #[arg(long, value_name = "PATH")]
        season_scenario_out: Option<PathBuf>,
        /// Camp outcomes scored into the compact season scenario.
        #[arg(long, default_value_t = 3000)]
        season_max_roster_branches: usize,
        /// Maximum Blender candidates evaluated per retained camp branch.
        #[arg(long, default_value_t = 24)]
        camp_max_candidates: usize,
    },
    /// Run The Cut across every team from current roster and prior-season evidence.
    CampLeague {
        /// Current roster identity map written by `icelines fetch rosters`.
        #[arg(long, default_value = "data/rosters.json", value_name = "PATH")]
        rosters: PathBuf,
        /// Completed prior-season skater bios used only to fill incomplete camp pools.
        #[arg(
            long,
            default_value = "data/seasons/20252026/bios.json",
            value_name = "PATH"
        )]
        bios: PathBuf,
        /// Completed prior-season skater statistics.
        #[arg(
            long,
            default_value = "data/seasons/20252026/stats.json",
            value_name = "PATH"
        )]
        stats: PathBuf,
        /// Completed prior-season goalie statistics.
        #[arg(
            long,
            default_value = "data/seasons/20252026/goalie-stats.json",
            value_name = "PATH"
        )]
        goalie_stats: PathBuf,
        /// Sourced organizational candidates not present in the current NHL roster snapshot.
        #[arg(long, value_name = "PATH")]
        candidate_overlay: Option<PathBuf>,
        /// Authored team camp inputs that replace automatically inferred pools.
        #[arg(long, value_name = "PATH")]
        authored_input: Vec<PathBuf>,
        #[arg(long, default_value_t = 20262027)]
        season: u32,
        #[arg(long, default_value_t = 1000)]
        trials: u32,
        #[arg(long, default_value_t = 20262027)]
        seed: u64,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Rank each team's roster, waiver, scratch, and prospect pressure in The Bubble.
    Bubble {
        /// UI-neutral `training_camp_league_forecast.v1` JSON document.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Optional sourced contract, protection, and waiver context.
        #[arg(long, value_name = "PATH")]
        transaction_context: Option<PathBuf>,
        /// Players retained in each team's exposure ranking.
        #[arg(long, default_value_t = 5)]
        top: usize,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Show the dated NHL-to-AHL affiliation map used by organization projections.
    #[command(name = "affiliate-map")]
    AffiliateMap {
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Project an NHL organization's AHL affiliate lineup under the development rule.
    Affiliate {
        /// UI-neutral `ahl_affiliate_projection` input JSON.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Propose a reviewable AHL-provider to NHL-player identity crosswalk.
    #[command(name = "affiliate-identities")]
    AffiliateIdentities {
        /// UI-neutral `ahl_roster_stats.v1` snapshot.
        #[arg(long, value_name = "PATH")]
        snapshot: PathBuf,
        /// Exact AHL team name from the snapshot.
        #[arg(long, value_name = "AHL_TEAM")]
        team: String,
        /// Canonical identity catalog or sourced league camp candidate overlay to merge.
        #[arg(
            long,
            value_name = "PATH",
            required_unless_present = "discover_official"
        )]
        candidates: Option<PathBuf>,
        /// Discover exact-name proposals through official NHL search and player landing records.
        #[arg(long)]
        discover_official: bool,
        /// Refresh official NHL discovery cachelines instead of accepting cached bytes.
        #[arg(long, requires = "discover_official")]
        refresh: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Propose identity crosswalks for every team in an AHL season snapshot.
    #[command(name = "affiliate-identities-league")]
    AffiliateIdentitiesLeague {
        #[arg(long, value_name = "PATH")]
        snapshot: PathBuf,
        /// Canonical identity catalog or sourced league camp candidate overlay to merge.
        #[arg(
            long,
            value_name = "PATH",
            required_unless_present = "discover_official"
        )]
        candidates: Option<PathBuf>,
        /// Discover proposals through deduplicated official NHL search and landing records.
        #[arg(long)]
        discover_official: bool,
        #[arg(long, requires = "discover_official")]
        refresh: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate a non-applicable review draft for exact AHL identity proposals.
    #[command(name = "affiliate-review-draft")]
    AffiliateReviewDraft {
        /// Generated `ahl_identity_crosswalk.v1` proposal queue.
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        /// Add sourced surname-and-birth alias remap proposals to the draft.
        #[arg(long)]
        include_aliases: bool,
        /// Add exact-name birth-conflict proposals for explicit review.
        #[arg(long)]
        include_conflicts: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate non-applicable review drafts across a league identity envelope.
    #[command(name = "affiliate-review-draft-league")]
    AffiliateReviewDraftLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        /// Add sourced surname-and-birth alias remap proposals to the draft.
        #[arg(long)]
        include_aliases: bool,
        /// Add exact-name birth-conflict proposals for explicit review.
        #[arg(long)]
        include_conflicts: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Accept only sourced exact-name-and-birth-date AHL identity proposals.
    #[command(name = "affiliate-review-exact")]
    AffiliateReviewExact {
        /// Generated `ahl_identity_crosswalk.v1` proposal queue.
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        /// Human or process authority recorded on every applied decision.
        #[arg(long)]
        reviewer: String,
        /// RFC3339 review timestamp.
        #[arg(long)]
        reviewed_at: String,
        /// Optionally retain the applicable exact-only decision document.
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        /// Reviewed crosswalk output; stdout is used when omitted.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Accept exact identity proposals across every eligible team in a league envelope.
    #[command(name = "affiliate-review-exact-league")]
    AffiliateReviewExactLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        reviewed_at: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Approve sourced surname-and-birth-date AHL identity aliases as remaps.
    #[command(name = "affiliate-review-aliases")]
    AffiliateReviewAliases {
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        #[arg(long)]
        reviewer: String,
        /// RFC3339 review timestamp.
        #[arg(long)]
        reviewed_at: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Approve sourced aliases across every eligible team in a league envelope.
    #[command(name = "affiliate-review-aliases-league")]
    AffiliateReviewAliasesLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        reviewed_at: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Resolve selected league birth-date conflicts with explicit sourced overrides.
    #[command(name = "affiliate-review-conflicts-league")]
    AffiliateReviewConflictsLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        /// Proposed canonical NHL player ID; repeat to resolve multiple identities.
        #[arg(long, value_name = "ID", required = true)]
        nhl_player_id: Vec<u32>,
        /// Additional absolute source URL supporting the override; repeat as needed.
        #[arg(long = "evidence-url", value_name = "URL", required = true)]
        evidence_urls: Vec<String>,
        #[arg(long)]
        reviewer: String,
        /// RFC3339 review timestamp.
        #[arg(long)]
        reviewed_at: String,
        /// Evidence-backed explanation for selecting the canonical NHL identity/date.
        #[arg(long)]
        note: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Correct a canonical NHL birth date while preserving the NHL identity.
    #[command(name = "affiliate-review-birth-date-league")]
    AffiliateReviewBirthDateLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        #[arg(long, value_name = "ID")]
        nhl_player_id: u32,
        #[arg(long)]
        canonical_birth_date: String,
        /// Additional absolute source URL supporting the corrected date; repeat as needed.
        #[arg(long = "evidence-url", value_name = "URL", required = true)]
        evidence_urls: Vec<String>,
        #[arg(long)]
        reviewer: String,
        /// RFC3339 review timestamp.
        #[arg(long)]
        reviewed_at: String,
        #[arg(long)]
        note: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Replace a probable same-name collision with the sourced canonical NHL identity.
    #[command(name = "affiliate-review-collision-league")]
    AffiliateReviewCollisionLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        /// Incorrect NHL proposal selected by the exception board.
        #[arg(long, value_name = "ID")]
        proposed_nhl_player_id: u32,
        #[arg(long, value_name = "ID")]
        canonical_nhl_player_id: u32,
        #[arg(long)]
        canonical_name: String,
        #[arg(long)]
        canonical_birth_date: String,
        /// Additional absolute source URL supporting the remap; repeat as needed.
        #[arg(long = "evidence-url", value_name = "URL", required = true)]
        evidence_urls: Vec<String>,
        #[arg(long)]
        reviewer: String,
        /// RFC3339 review timestamp.
        #[arg(long)]
        reviewed_at: String,
        #[arg(long)]
        note: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Reject selected pending NHL identity mappings with retained review authority.
    #[command(name = "affiliate-review-reject")]
    AffiliateReviewReject {
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        /// AHL provider player ID to reject; repeat for multiple rows sharing one rationale.
        #[arg(long, value_name = "ID", required = true)]
        provider_player_id: Vec<String>,
        /// Absolute source URL supporting the rejection; repeat as needed.
        #[arg(long = "evidence-url", value_name = "URL")]
        evidence_urls: Vec<String>,
        #[arg(long)]
        reviewer: String,
        /// RFC3339 review timestamp.
        #[arg(long)]
        reviewed_at: String,
        /// Evidence-backed explanation; rejection applies only to the NHL identity mapping.
        #[arg(long)]
        note: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Reject selected pending mappings across a league identity envelope.
    #[command(name = "affiliate-review-reject-league")]
    AffiliateReviewRejectLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        /// AHL provider player ID to reject; repeat for multiple identities.
        #[arg(long, value_name = "ID", required = true)]
        provider_player_id: Vec<String>,
        #[arg(long = "evidence-url", value_name = "URL")]
        evidence_urls: Vec<String>,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        reviewed_at: String,
        /// Evidence-backed explanation; AHL player facts remain retained.
        #[arg(long)]
        note: String,
        #[arg(long, value_name = "PATH")]
        decisions_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Aggregate team-season crosswalks into league coverage and exception groups.
    #[command(name = "affiliate-review-league")]
    AffiliateReviewLeague {
        /// Repeat for each reviewed or in-progress team-season crosswalk.
        #[arg(long = "crosswalk", value_name = "PATH")]
        crosswalks: Vec<PathBuf>,
        /// Repeat for each league crosswalk envelope; child team queues are flattened.
        #[arg(long = "league-crosswalk", value_name = "PATH")]
        league_crosswalks: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Rank the read-only league identity exception queue by review leverage.
    #[command(name = "affiliate-review-board")]
    AffiliateReviewBoard {
        #[arg(long, value_name = "PATH")]
        review: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Inspect an existing AHL identity crosswalk without changing review state.
    #[command(name = "affiliate-review-show")]
    AffiliateReviewShow {
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        /// Show only non-routine rows needing reviewer attention.
        #[arg(long)]
        attention_only: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply a finalized, reviewer-authored identity decision batch.
    #[command(name = "affiliate-review-apply")]
    AffiliateReviewApply {
        /// Generated `ahl_identity_crosswalk.v1` proposal queue.
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        /// Finalized `ahl_identity_review_decisions.v1` document.
        #[arg(long, value_name = "PATH")]
        decisions: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate a non-applicable prior-affiliate organization-status draft.
    #[command(name = "affiliate-status-draft")]
    AffiliateStatusDraft {
        #[arg(long, value_name = "PATH")]
        prior_snapshot: PathBuf,
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        #[arg(long, value_name = "PATH")]
        camp: PathBuf,
        #[arg(long, value_name = "NHL_TEAM")]
        nhl_team: String,
        /// Prior-snapshot AHL team; target affiliate is supplied by rollover config.
        #[arg(long, value_name = "PRIOR_AHL_TEAM")]
        ahl_team: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate the non-applicable organization-status draft for every camp team.
    #[command(name = "affiliate-status-draft-league")]
    AffiliateStatusDraftLeague {
        #[arg(long, value_name = "PATH")]
        prior_snapshot: PathBuf,
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        #[arg(long, value_name = "PATH")]
        camp_forecast: PathBuf,
        #[arg(long, value_name = "PATH")]
        config: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Resolve organization status from dated official NHL current-team facts.
    #[command(name = "affiliate-status-evidence")]
    AffiliateStatusEvidence {
        /// Exact all-team organization-status review draft.
        #[arg(long, value_name = "PATH")]
        review: PathBuf,
        /// Official NHL landing career-history cache with organization facts.
        #[arg(long, value_name = "PATH")]
        career_history: PathBuf,
        /// Evaluation cutoff in RFC 3339 form.
        #[arg(long, value_name = "RFC3339")]
        as_of: String,
        /// Maximum accepted age of each official landing fact.
        #[arg(long, default_value_t = 14)]
        maximum_fact_age_days: u32,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Prefill a draft review with exact resolved organization-status evidence.
    #[command(name = "affiliate-status-evidence-apply")]
    AffiliateStatusEvidenceApply {
        #[arg(long, value_name = "PATH")]
        review: PathBuf,
        #[arg(long, value_name = "PATH")]
        ledger: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Interpret explicit official AHL ADD/DEL events as cutoff roster state.
    #[command(name = "affiliate-transaction-state")]
    AffiliateTransactionState {
        /// Sealed `ahl_transaction_snapshot.v1` for the target season.
        #[arg(long, value_name = "PATH")]
        transactions: PathBuf,
        /// Reviewed current or prior AHL provider-to-NHL identity envelope.
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        /// Dated target-season NHL-to-AHL affiliation catalog.
        #[arg(long, value_name = "PATH")]
        affiliations: PathBuf,
        /// Include only source events on or before this YYYY-MM-DD date.
        #[arg(long, value_name = "YYYY-MM-DD")]
        cutoff: String,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply only canonical, unambiguous transaction state to an exact workboard.
    #[command(name = "affiliate-transaction-state-apply")]
    AffiliateTransactionStateApply {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        ledger: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate the exact pending waiver-result queue for a preseason workboard.
    #[command(name = "affiliate-waivers-draft")]
    AffiliateWaiversDraft {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "YYYY-MM-DD")]
        cutoff: String,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Finalize sourced cleared/claimed decisions against an exact waiver draft.
    #[command(name = "affiliate-waivers-finalize")]
    AffiliateWaiversFinalize {
        #[arg(long, value_name = "PATH")]
        draft: PathBuf,
        #[arg(long, value_name = "PATH")]
        decisions: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply a finalized exact waiver review to its source workboard.
    #[command(name = "affiliate-waivers-apply")]
    AffiliateWaiversApply {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        review: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Inspect an existing affiliate organization-status review artifact.
    #[command(name = "affiliate-status-show")]
    AffiliateStatusShow {
        #[arg(long, value_name = "PATH")]
        review: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply finalized retained/departed/other-league decisions to rollover config.
    #[command(name = "affiliate-status-apply")]
    AffiliateStatusApply {
        #[arg(long, value_name = "PATH")]
        prior_snapshot: PathBuf,
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        #[arg(long, value_name = "PATH")]
        camp: PathBuf,
        #[arg(long, value_name = "PATH")]
        review: PathBuf,
        #[arg(long, value_name = "PATH")]
        config: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Atomically apply finalized league organization-status review children.
    #[command(name = "affiliate-status-apply-league")]
    AffiliateStatusApplyLeague {
        #[arg(long, value_name = "PATH")]
        prior_snapshot: PathBuf,
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        #[arg(long, value_name = "PATH")]
        camp_forecast: PathBuf,
        #[arg(long, value_name = "PATH")]
        review: PathBuf,
        #[arg(long, value_name = "PATH")]
        config: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build a fail-closed start-of-season professional-game ledger.
    #[command(name = "affiliate-professional-games")]
    AffiliateProfessionalGames {
        /// Fully reviewed all-league AHL identity envelope.
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        /// Official NHL landing career-history cache.
        #[arg(long, value_name = "PATH")]
        career_history: PathBuf,
        /// Reviewed league-inclusion policy for the target AHL season.
        #[arg(long, value_name = "PATH")]
        policy: PathBuf,
        /// Complete target-season camp pool used to add canonical candidates absent from prior AHL rosters.
        #[arg(long, value_name = "PATH")]
        camp_forecast: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build confidence-weighted prior-season AHL player values.
    #[command(name = "affiliate-values")]
    AffiliateValues {
        /// Official prior-season all-league AHL roster/stats snapshot.
        #[arg(long, value_name = "PATH")]
        snapshot: PathBuf,
        /// Fully reviewed all-league AHL identity envelope.
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        /// Versioned AHL player-value policy.
        #[arg(long, value_name = "PATH")]
        policy: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Fill only missing workboard scores from an exact AHL value ledger.
    #[command(name = "affiliate-values-apply")]
    AffiliateValuesApply {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        ledger: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Fit paired career translations and fill only values missing direct AHL evidence.
    #[command(name = "affiliate-values-cross-league")]
    AffiliateValuesCrossLeague {
        /// A raw workboard or prior machine-application artifact containing one.
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        /// Official NHL landing career-history cache used for calibration and player evidence.
        #[arg(long, value_name = "PATH")]
        career_history: PathBuf,
        /// Versioned cross-league calibration and shrinkage policy.
        #[arg(long, value_name = "PATH")]
        policy: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply a sealed cross-league value ledger to its exact source workboard.
    #[command(name = "affiliate-values-cross-league-apply")]
    AffiliateValuesCrossLeagueApply {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        ledger: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Classify operational organizational-prospect status from official age and NHL workload.
    #[command(name = "affiliate-prospects")]
    AffiliateProspects {
        /// A raw workboard or a prior machine-application artifact containing one.
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        /// Official NHL landing career-history cache.
        #[arg(long, value_name = "PATH")]
        career_history: PathBuf,
        /// Versioned organizational-prospect population policy.
        #[arg(long, value_name = "PATH")]
        policy: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Fill only missing prospect-status facts from an exact prospect ledger.
    #[command(name = "affiliate-prospects-apply")]
    AffiliateProspectsApply {
        /// The exact workboard used to construct the prospect ledger.
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        ledger: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Estimate confidence-aware AHL-to-NHL recall readiness.
    #[command(name = "affiliate-readiness")]
    AffiliateReadiness {
        /// A raw workboard or a prior machine-application artifact containing one.
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        /// Official NHL landing career-history cache.
        #[arg(long, value_name = "PATH")]
        career_history: PathBuf,
        /// Sealed all-32 training-camp league forecast.
        #[arg(long, value_name = "PATH")]
        camp_forecast: PathBuf,
        /// Versioned recall-readiness evaluation policy.
        #[arg(long, value_name = "PATH")]
        policy: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Fill only missing recall-readiness facts from an exact readiness ledger.
    #[command(name = "affiliate-readiness-apply")]
    AffiliateReadinessApply {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        ledger: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Compose a league-wide preseason affiliate facts-readiness workboard.
    #[command(name = "affiliate-facts-board")]
    AffiliateFactsBoard {
        /// Complete `ahl_preseason_league_rollover.v1` artifact.
        #[arg(long, value_name = "PATH")]
        rollover: PathBuf,
        /// Matching `ahl_professional_game_ledger.v2` artifact.
        #[arg(long, value_name = "PATH")]
        professional_games: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Inspect any raw or nested preseason facts workboard and optionally gate readiness.
    #[command(name = "affiliate-facts-status")]
    AffiliateFactsStatus {
        /// Raw workboard or any machine application containing `workboard`.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Fail unless every remaining candidate is facts-ready with no blockers.
        #[arg(long)]
        require_ready: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate a non-applicable player-facts review draft from a sealed workboard.
    #[command(name = "affiliate-facts-draft")]
    AffiliateFactsDraft {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply finalized, source-backed player facts to an exact preseason workboard.
    #[command(name = "affiliate-facts-apply")]
    AffiliateFactsApply {
        #[arg(long, value_name = "PATH")]
        workboard: PathBuf,
        #[arg(long, value_name = "PATH")]
        overlay: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Lower facts-ready teams into canonical preseason affiliate projection inputs.
    #[command(name = "affiliate-inputs-league")]
    AffiliateInputsLeague {
        #[arg(long, value_name = "PATH")]
        application: PathBuf,
        /// Final target-season dressed-roster development rule authority.
        #[arg(long, value_name = "PATH")]
        rule: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Apply a final professional-game ledger to separate team projection facts.
    #[command(name = "affiliate-professional-games-apply")]
    AffiliateProfessionalGamesApply {
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        #[arg(long, value_name = "PATH")]
        ledger: PathBuf,
        /// JSON array of projection facts keyed by AHL provider player ID.
        #[arg(long, value_name = "PATH")]
        facts: PathBuf,
        #[arg(long, value_name = "NHL_TEAM")]
        nhl_team: String,
        #[arg(long, value_name = "AHL_TEAM")]
        ahl_team: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Join reviewed AHL identities to separate projection facts.
    #[command(name = "affiliate-input")]
    AffiliateInput {
        /// UI-neutral `ahl_roster_stats.v1` snapshot.
        #[arg(long, value_name = "PATH")]
        snapshot: PathBuf,
        /// Fully reviewed `ahl_identity_crosswalk.v1` document.
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        /// JSON array of projection facts, or a final professional-game facts application.
        #[arg(long, value_name = "PATH")]
        facts: PathBuf,
        #[arg(long, value_name = "NHL_TEAM")]
        nhl_team: String,
        #[arg(long, value_name = "AHL_TEAM")]
        ahl_team: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Reconcile a prior affiliate roster with the current NHL camp pool.
    #[command(name = "affiliate-rollover")]
    AffiliateRollover {
        /// Prior-season UI-neutral `ahl_roster_stats.v1` snapshot.
        #[arg(long, value_name = "PATH")]
        prior_snapshot: PathBuf,
        /// Snapshot-bound `ahl_identity_crosswalk.v1`, which may retain pending rows.
        #[arg(long, value_name = "PATH")]
        crosswalk: PathBuf,
        /// Current `training_camp_simulation` input.
        #[arg(long, value_name = "PATH")]
        camp: PathBuf,
        /// Matching `training_camp_forecast.v1` output.
        #[arg(long, value_name = "PATH")]
        camp_forecast: PathBuf,
        /// Target-season rollover authority, sources, and prior-player decisions.
        #[arg(long, value_name = "PATH")]
        config: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Draft exact prior/target affiliate bindings for every camp team.
    #[command(name = "affiliate-rollover-config-league")]
    AffiliateRolloverConfigLeague {
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        #[arg(long, value_name = "PATH")]
        camp_forecast: PathBuf,
        #[arg(long, value_name = "PATH")]
        prior_affiliations: PathBuf,
        #[arg(long, value_name = "PATH")]
        affiliations: PathBuf,
        #[arg(long)]
        as_of: String,
        #[arg(long = "source-url", value_name = "URL", required = true)]
        source_urls: Vec<String>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Reconcile every sealed league camp forecast with reviewed prior affiliates.
    #[command(name = "affiliate-rollover-league")]
    AffiliateRolloverLeague {
        #[arg(long, value_name = "PATH")]
        prior_snapshot: PathBuf,
        #[arg(long, value_name = "PATH")]
        league_crosswalk: PathBuf,
        #[arg(long, value_name = "PATH")]
        camp_forecast: PathBuf,
        /// League config with one explicit target/prior affiliation per camp team.
        #[arg(long, value_name = "PATH")]
        config: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Combine an NHL lineup and its AHL affiliate into The System.
    Organization {
        /// UI-neutral input containing `nhl_lineup` and `ahl_affiliate` documents.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Rank lineup alternatives in The Blender and write a reusable Bench scenario.
    Blender {
        /// UI-neutral `team_lineup_projection.v1` JSON document.
        #[arg(long, value_name = "PATH")]
        lineup: PathBuf,
        /// Optional JSON array of explicitly labeled pair-evidence inputs.
        #[arg(long, value_name = "PATH", conflicts_with = "shift_season")]
        pair_evidence: Option<PathBuf>,
        /// Fetch/cache official interval shifts from this completed season.
        #[arg(long, conflicts_with = "pair_evidence")]
        shift_season: Option<u32>,
        /// Refresh official shift-chart cache for `--shift-season`.
        #[arg(long, requires = "shift_season")]
        refresh_shifts: bool,
        /// Write the full official pair/trio deployment overlap report.
        #[arg(long, value_name = "PATH", requires = "shift_season")]
        shift_report_out: Option<PathBuf>,
        #[arg(long, default_value_t = 24)]
        max_candidates: usize,
        /// Allow wings to switch sides and natural centers to fill wing vacancies.
        #[arg(long)]
        allow_off_wing: bool,
        /// Number of team games in each result review window.
        #[arg(long, default_value_t = 6)]
        review_games: u8,
        /// Keep the active lineup when its window points percentage meets this value.
        #[arg(long, default_value_t = 0.5)]
        minimum_points_percentage: f64,
        #[arg(long, default_value_t = 2)]
        max_changes: u8,
        /// Baseline plus this many total ranked choices in The Bench policy.
        #[arg(long, default_value_t = 3)]
        max_choices: usize,
        /// Write a `team_season_scenario.v1` accepted by `icecast season --scenario`.
        #[arg(long, value_name = "PATH")]
        scenario_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Author opponent-specific plans and a season scenario from sealed evidence.
    Bench {
        /// UI-neutral `team_game_forecast.v1` written by `icecast season --game-forecast-out`.
        #[arg(long, value_name = "PATH")]
        forecast: PathBuf,
        /// UI-neutral `team_lineup_projection.v1` for one team.
        #[arg(long, value_name = "PATH")]
        lineup: PathBuf,
        /// `team_decision_profile.v1` for the team's current leadership.
        #[arg(long, value_name = "PATH")]
        profile: PathBuf,
        /// JSON array of sealed `opponent_style_evidence.v1` rows.
        #[arg(long, value_name = "PATH")]
        style_evidence: PathBuf,
        /// Completed stats window used for current-roster player role evidence.
        #[arg(long, default_value_t = 20252026)]
        stats_season: u32,
        /// Write the simulation-ready `team_season_scenario.v1` separately.
        #[arg(long, value_name = "PATH")]
        scenario_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build baseline forecasts for every game in a season.
    Season {
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        /// Completed player-stat season used by roster/depth strength.
        #[arg(long, default_value_t = 20252026)]
        stats_season: u32,
        /// Repeat to focus text output; JSON always retains the full league run.
        #[arg(long = "team")]
        teams: Vec<String>,
        /// Number of seeded chronological league simulations.
        #[arg(long, default_value_t = 10_000)]
        trials: u32,
        /// Reproducible simulation seed.
        #[arg(long, default_value_t = 20_262_027)]
        seed: u64,
        /// JSON file containing dated injury, goalie, trade, return, or form events.
        #[arg(long, value_name = "PATH", conflicts_with = "scenario_id")]
        scenario: Option<PathBuf>,
        /// Stable ID from the local scenario registry.
        #[arg(long, value_name = "ID", conflicts_with = "scenario")]
        scenario_id: Option<String>,
        /// Run paired baseline/one-event attribution for every scenario event.
        #[arg(long)]
        isolated_impacts: bool,
        /// Generate seeded injury and goalie-availability events from player records.
        #[arg(long)]
        auto_personnel: bool,
        /// Simulated trade market: off or plausible.
        #[arg(long, default_value = "off", value_parser = ["off", "plausible"])]
        trade_mode: String,
        /// Forecast evidence mode: off freezes roster strength; rolling uses only earlier results.
        #[arg(long, default_value = "off", value_parser = ["off", "rolling"])]
        replay_mode: String,
        /// Evaluation counterfactual: omit personnel evidence strictly after this date.
        #[arg(long)]
        ignore_replay_personnel_after: Option<chrono::NaiveDate>,
        /// Condition a rolling replay on final results through this date, then simulate the remainder.
        #[arg(long)]
        through: Option<chrono::NaiveDate>,
        /// Use each team's official first-game dressed lineup as retrospective evaluation evidence.
        #[arg(long)]
        retrospective_opening_lineups: bool,
        #[arg(long)]
        all_games: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Write the underlying UI-neutral per-game forecast for The Bench and other consumers.
        #[arg(long, value_name = "PATH")]
        game_forecast_out: Option<PathBuf>,
    },
    /// Apply dated roster, goalie, xG, special-teams, and matchup evidence to a game forecast.
    Edge {
        /// UI-neutral `team_game_forecast.v1` baseline.
        #[arg(long, value_name = "PATH")]
        forecast: PathBuf,
        /// Sealed `game_prediction_edge_evidence_package.v1` document.
        #[arg(long, value_name = "PATH")]
        evidence: PathBuf,
        /// Optional trained `TeamGamePredictionModel`; evaluation weights are used when omitted.
        #[arg(long, value_name = "PATH")]
        model: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Write the enhanced `team_game_forecast.v1` for direct season simulation.
        #[arg(long, value_name = "PATH")]
        enhanced_forecast_out: Option<PathBuf>,
    },
    /// Project one game's sealed forecast vintages into a UI-neutral card.
    #[command(name = "edge-card")]
    EdgeCard {
        /// Repeat for preseason, game-morning, and pregame-confirmed edge documents.
        #[arg(long = "input", required = true, value_name = "PATH")]
        inputs: Vec<PathBuf>,
        #[arg(long)]
        game_id: u64,
        /// Team perspective used for probabilities, deltas, and theme.
        #[arg(long)]
        team: String,
        #[arg(long)]
        team_name: Option<String>,
        /// Optional evidence timestamp for deterministic output.
        #[arg(long)]
        generated_at: Option<String>,
        /// Optional sealed closing-market benchmark JSON; never used as a model feature.
        #[arg(long, value_name = "PATH")]
        market_benchmark: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Simulate a season from an existing baseline or edge-enhanced game forecast.
    #[command(name = "season-simulate")]
    SeasonSimulate {
        /// UI-neutral `team_game_forecast.v1`, including `edge --enhanced-forecast-out`.
        #[arg(long, value_name = "PATH")]
        forecast: PathBuf,
        #[arg(long, default_value_t = 10_000)]
        trials: u32,
        #[arg(long, default_value_t = 20_262_027)]
        seed: u64,
        #[arg(long, value_name = "PATH")]
        scenario: Option<PathBuf>,
        /// Fix final results through this date and simulate only the remainder.
        #[arg(long)]
        through: Option<chrono::NaiveDate>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Assemble and seal a dated evidence package from shared IceLines primitives.
    #[command(name = "edge-evidence")]
    EdgeEvidence {
        /// Exact `team_game_forecast.v1` baseline the package will bind.
        #[arg(long, value_name = "PATH")]
        forecast: PathBuf,
        /// `GamePredictionEvidencePackageBuildInput` JSON with lineup, goalie, and form inputs.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Join later final outcomes to frozen edge documents for historical training.
    #[command(name = "edge-observe")]
    EdgeObserve {
        /// Repeat for each sealed `team_game_prediction_edge.v1` season document.
        #[arg(long = "edge", required = true, value_name = "PATH")]
        edges: Vec<PathBuf>,
        /// Repeat for each sealed official outcome set (or raw outcome array).
        #[arg(long = "outcomes", required = true, value_name = "PATH")]
        outcomes: Vec<PathBuf>,
        /// RFC 3339 time at which the observation set is sealed.
        #[arg(long)]
        created_at: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Seal official NHL final results for later outcome joining.
    #[command(name = "edge-outcomes")]
    EdgeOutcomes {
        #[arg(long)]
        season: u32,
        /// RFC 3339 timestamp of the official result snapshot.
        #[arg(long)]
        captured_at: String,
        /// Refresh official NHL schedules before sealing results.
        #[arg(long)]
        refresh: bool,
        /// Allow an in-progress season instead of requiring every game final.
        #[arg(long)]
        allow_partial: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Reconstruct a historical xG/special-teams challenger from MoneyPuck team files.
    #[command(name = "edge-replay-xg")]
    EdgeReplayXg {
        #[arg(long, value_name = "PATH")]
        forecast: PathBuf,
        /// Directory containing one MoneyPuck career game-by-game CSV per team (`NYR.csv`).
        #[arg(long, value_name = "DIR")]
        moneypuck_dir: PathBuf,
        /// RFC 3339 retrieval time for the exact CSV bytes.
        #[arg(long)]
        retrieved_at: String,
        #[arg(long, default_value_t = 10)]
        trailing_games: usize,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Reconstruct confirmed starters and dressed lineups from official boxscores.
    #[command(name = "edge-replay-confirmed")]
    EdgeReplayConfirmed {
        #[arg(long, value_name = "PATH")]
        forecast: PathBuf,
        /// Sealed game-morning evidence package to enrich.
        #[arg(long, value_name = "PATH")]
        morning_evidence: PathBuf,
        /// Cache directory containing `{game_id}.json` official boxscores.
        #[arg(long, value_name = "DIR")]
        boxscore_dir: PathBuf,
        /// RFC 3339 retrieval time assigned to the exact cached source bytes.
        #[arg(long)]
        retrieved_at: String,
        /// Fetch missing boxscores (or replace cached files) from the official NHL API.
        #[arg(long)]
        refresh: bool,
        #[arg(long, default_value_t = 8)]
        concurrency: usize,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Enrich confirmed historical starters with point-in-time goalie form and workload.
    #[command(name = "edge-replay-goalies")]
    EdgeReplayGoalies {
        #[arg(long, value_name = "PATH")]
        forecast: PathBuf,
        /// Sealed pregame-confirmed evidence package containing starter player IDs.
        #[arg(long, value_name = "PATH")]
        confirmed_evidence: PathBuf,
        /// Cache directory containing one MoneyPuck career CSV per goalie (`{player_id}.csv`).
        #[arg(long, value_name = "DIR")]
        goalie_dir: PathBuf,
        /// RFC 3339 retrieval time assigned to the exact cached source bytes.
        #[arg(long)]
        retrieved_at: String,
        #[arg(long, default_value_t = 5)]
        trailing_appearances: usize,
        /// Fetch missing goalie files (or replace cached files) from MoneyPuck.
        #[arg(long)]
        refresh: bool,
        #[arg(long, default_value_t = 1)]
        concurrency: usize,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Preregister an untouched season before forecasts or outcomes exist.
    #[command(name = "edge-register-holdout")]
    EdgeRegisterHoldout {
        #[arg(long)]
        season: u32,
        #[arg(long)]
        registered_at: String,
        #[arg(long)]
        outcome_not_before: String,
        /// Optional exact training configuration to seal; defaults to the standard config.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Train or rolling-origin validate the leakage-safe game-prediction ensemble.
    #[command(name = "edge-train")]
    EdgeTrain {
        /// JSON array of frozen `TeamGamePredictionTrainingObservation` rows.
        #[arg(long, value_name = "PATH")]
        observations: PathBuf,
        /// Optional `TeamGamePredictionTrainingConfig` JSON.
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// Sealed prospective registration required for production authority.
        #[arg(long, value_name = "PATH")]
        registration: Option<PathBuf>,
        /// Run season-forward holdouts and the promotion gate.
        #[arg(long)]
        validate: bool,
        /// Write the fitted model as a directly reusable model document.
        #[arg(long, value_name = "PATH")]
        model_out: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Project one team from a sealed forecast history into `card_document.v1`.
    #[command(name = "history-card")]
    HistoryCard {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        team: String,
        #[arg(long)]
        team_name: Option<String>,
        /// Evidence timestamp for deterministic output.
        #[arg(long)]
        generated_at: Option<String>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Inspect a sealed all-team organization Window board or one team detail.
    Window {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Focus one team while retaining the sealed league artifact as authority.
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        json: bool,
        /// Export a durable Markdown report from the sealed board.
        #[arg(long, conflicts_with = "json")]
        markdown: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build a sealed standing profile history from one or more validated Window boards.
    #[command(name = "window-profile-history-build")]
    WindowProfileHistoryBuild {
        #[arg(long = "board", value_name = "PATH", required = true)]
        boards: Vec<PathBuf>,
        #[arg(long)]
        history_id: String,
        #[arg(long)]
        created_at: String,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Backfill standing history from sealed point-in-time historical origins.
    #[command(name = "window-profile-history-backfill")]
    WindowProfileHistoryBackfill {
        #[arg(long = "origin", value_name = "PATH", required = true)]
        origins: Vec<PathBuf>,
        #[arg(long)]
        history_id: String,
        #[arg(long)]
        created_at: String,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Derive the prior-season AHL depth baseline from observed affiliate appearances.
    #[command(name = "window-profile-history-baseline")]
    WindowProfileHistoryBaseline {
        #[arg(long, value_name = "PATH")]
        source_package: PathBuf,
        #[arg(long, value_name = "PATH")]
        ahl_workboard: PathBuf,
        #[arg(long)]
        history_id: String,
        #[arg(long)]
        created_at: String,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Audit every registered profile at every standing-history checkpoint.
    #[command(name = "window-profile-history-audit")]
    WindowProfileHistoryAudit {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        generated_at: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Compare exact profile methods across two standing-history checkpoints.
    #[command(name = "window-profile-history-delta")]
    WindowProfileHistoryDelta {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        earlier_season: u32,
        #[arg(long)]
        earlier_as_of: chrono::NaiveDate,
        #[arg(long)]
        later_season: u32,
        #[arg(long)]
        later_as_of: chrono::NaiveDate,
        #[arg(long, default_value = "one_year")]
        horizon: String,
        #[arg(long)]
        generated_at: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Project one team's standing-history delta into a UI-neutral card.
    #[command(name = "window-profile-history-card")]
    WindowProfileHistoryCard {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        team: String,
        #[arg(long)]
        team_name: Option<String>,
        #[arg(long)]
        generated_at: Option<String>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build the official balanced Window from sealed IceLines source documents.
    #[command(name = "window-build")]
    WindowBuild {
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(long)]
        as_of: chrono::NaiveDate,
        #[arg(long)]
        generated_at: String,
        /// Read one sealed, portable Window source package instead of loose documents.
        #[arg(long, value_name = "PATH", conflicts_with_all = ["team_season_forecast", "team_game_forecast", "team_lineups", "ahl_affiliates", "organization_lineups", "prospect_program", "prospect_conversion", "training_camp", "schedule_rest", "profile_history"])]
        source_package: Option<PathBuf>,
        #[arg(long, value_name = "PATH", conflicts_with = "source_package")]
        team_season_forecast: Option<PathBuf>,
        /// Derive league schedule-fatigue profiles from a sealed game forecast.
        #[arg(long, value_name = "PATH", conflicts_with = "source_package")]
        team_game_forecast: Option<PathBuf>,
        #[arg(
            long = "team-lineup",
            value_name = "PATH",
            conflicts_with = "source_package"
        )]
        team_lineups: Vec<PathBuf>,
        /// Compose organization lineups in core from sealed AHL projections.
        #[arg(
            long = "ahl-affiliate",
            value_name = "PATH",
            conflicts_with = "source_package"
        )]
        ahl_affiliates: Vec<PathBuf>,
        #[arg(
            long = "organization-lineup",
            value_name = "PATH",
            conflicts_with = "source_package"
        )]
        organization_lineups: Vec<PathBuf>,
        #[arg(long, value_name = "PATH", conflicts_with = "source_package")]
        prospect_program: Option<PathBuf>,
        #[arg(long, value_name = "PATH", conflicts_with = "source_package")]
        prospect_conversion: Option<PathBuf>,
        #[arg(long, value_name = "PATH", conflicts_with = "source_package")]
        training_camp: Option<PathBuf>,
        #[arg(
            long = "schedule-rest",
            value_name = "PATH",
            conflicts_with = "source_package"
        )]
        schedule_rest: Vec<PathBuf>,
        /// Fill only missing organization/recall profiles from the latest eligible prior season.
        #[arg(long, value_name = "PATH", conflicts_with = "source_package")]
        profile_history: Option<PathBuf>,
        /// Refuse to write unless every organization has an eligible rank.
        #[arg(long)]
        require_ranked: bool,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Seal loose Window authorities into one replayable, cache-friendly package.
    #[command(name = "window-source-package")]
    WindowSourcePackage {
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(long)]
        as_of: chrono::NaiveDate,
        #[arg(long, value_name = "PATH")]
        team_season_forecast: Option<PathBuf>,
        /// Derive league schedule-fatigue profiles from a sealed game forecast.
        #[arg(long, value_name = "PATH")]
        team_game_forecast: Option<PathBuf>,
        /// Build all 32 team lineups from the configured snapshot cache.
        #[arg(long, conflicts_with = "team_lineups")]
        cache_team_lineups: bool,
        /// Completed production season used by cache-built player scores.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(
            long = "team-lineup",
            value_name = "PATH",
            conflicts_with = "cache_team_lineups"
        )]
        team_lineups: Vec<PathBuf>,
        /// Compose organization lineups in core from sealed AHL projections.
        #[arg(long = "ahl-affiliate", value_name = "PATH")]
        ahl_affiliates: Vec<PathBuf>,
        /// Build all affiliates from one fully reviewed league projection-input artifact.
        #[arg(long, value_name = "PATH", conflicts_with = "ahl_affiliates")]
        ahl_projection_inputs: Option<PathBuf>,
        #[arg(long = "organization-lineup", value_name = "PATH")]
        organization_lineups: Vec<PathBuf>,
        #[arg(long, value_name = "PATH")]
        prospect_program: Option<PathBuf>,
        /// Build the prospect program from training camp and the configured career cache.
        #[arg(long, conflicts_with = "prospect_program", requires = "training_camp")]
        cache_prospect_program: bool,
        /// Override the official career cache for camp-completed goalies or prospects.
        #[arg(long, value_name = "PATH")]
        career_history: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        prospect_conversion: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        training_camp: Option<PathBuf>,
        #[arg(long = "schedule-rest", value_name = "PATH")]
        schedule_rest: Vec<PathBuf>,
        /// Standing point-in-time profile history used by the preseason fallback adapter.
        #[arg(long, value_name = "PATH")]
        profile_history: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Refresh cache-built NHL lineups inside an existing sealed Window package.
    #[command(name = "window-source-refresh-lineups")]
    WindowSourceRefreshLineups {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Completed production season used by cache-built player scores.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        /// Override the package's training-camp authority.
        #[arg(long, value_name = "PATH")]
        training_camp: Option<PathBuf>,
        /// Override the configured official NHL landing career cache.
        #[arg(long, value_name = "PATH")]
        career_history: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Refresh reviewed AHL affiliates inside an existing sealed Window package.
    #[command(name = "window-source-refresh-affiliates")]
    WindowSourceRefreshAffiliates {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        /// Complete league artifact emitted by `affiliate-preseason-projection-inputs`.
        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "ahl_facts_application",
            required_unless_present = "ahl_facts_application"
        )]
        ahl_projection_inputs: Option<PathBuf>,
        /// Final reviewed facts application to lower and refresh atomically.
        #[arg(
            long,
            value_name = "PATH",
            conflicts_with = "ahl_projection_inputs",
            requires = "ahl_development_rule"
        )]
        ahl_facts_application: Option<PathBuf>,
        /// Final target-season AHL development rule used to lower reviewed facts.
        #[arg(long, value_name = "PATH", requires = "ahl_facts_application")]
        ahl_development_rule: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Attach standing profile history to an existing sealed Window package.
    #[command(name = "window-source-refresh-history")]
    WindowSourceRefreshHistory {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_name = "PATH")]
        profile_history: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Audit all 17 balanced profiles across the canonical league source package.
    #[command(name = "window-source-audit")]
    WindowSourceAudit {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        generated_at: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Project one team from a sealed Window board into `card_document.v1`.
    #[command(name = "window-card")]
    WindowCard {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        team: String,
        #[arg(long)]
        team_name: Option<String>,
        #[arg(long)]
        generated_at: Option<String>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Compare two sealed, method-compatible Window checkpoints.
    #[command(name = "window-movement")]
    WindowMovement {
        #[arg(long, value_name = "PATH")]
        earlier: PathBuf,
        #[arg(long, value_name = "PATH")]
        later: PathBuf,
        /// Reviewed bridge for an intentional manifest or method upgrade.
        #[arg(long, value_name = "PATH")]
        bridge: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Attribute checkpoint movement to dated personnel evidence through a sealed scenario.
    #[command(name = "window-personnel-attribution")]
    WindowPersonnelAttribution {
        #[arg(long, value_name = "PATH")]
        earlier: PathBuf,
        #[arg(long, value_name = "PATH")]
        later: PathBuf,
        #[arg(long, value_name = "PATH")]
        movement: PathBuf,
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build a later-checkpoint personnel counterfactual input from paired IceReplay evidence.
    #[command(name = "window-personnel-input-build")]
    WindowPersonnelInputBuild {
        #[arg(long, value_name = "PATH")]
        actual_forecast: PathBuf,
        #[arg(long, value_name = "PATH")]
        counterfactual_board: PathBuf,
        #[arg(long)]
        earlier_as_of: chrono::NaiveDate,
        #[arg(long)]
        later_as_of: chrono::NaiveDate,
        #[arg(long)]
        attribution_id: String,
        #[arg(long)]
        scenario_id: String,
        #[arg(long)]
        rationale: String,
        #[arg(long, value_name = "PATH")]
        out: PathBuf,
    },
    /// Summarize a typed personnel movement artifact for durable evidence review.
    #[command(name = "window-personnel-summary")]
    WindowPersonnelSummary {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Rebuild a sealed Window board under a reviewed target manifest.
    #[command(name = "window-rebase")]
    WindowRebase {
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_name = "PATH")]
        target_manifest: PathBuf,
        #[arg(long, value_name = "PATH")]
        bridge: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build Window history from two or more comparable checkpoints.
    #[command(name = "window-history")]
    WindowHistory {
        #[arg(long = "input", required = true, value_name = "PATH")]
        inputs: Vec<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Compare a same-context baseline and scenario Window board.
    #[command(name = "window-scenario")]
    WindowScenario {
        #[arg(long, value_name = "PATH")]
        baseline: PathBuf,
        #[arg(long, value_name = "PATH")]
        scenario: PathBuf,
        #[arg(long)]
        scenario_id: String,
        /// Typed upstream authority document; repeat for combined scenarios.
        #[arg(long = "authority", value_name = "PATH")]
        authorities: Vec<PathBuf>,
        /// Derive typed authorities from a team-season forecast; repeat as needed.
        #[arg(long = "team-season-authority", value_name = "PATH")]
        team_season_authorities: Vec<PathBuf>,
        /// Derive typed authorities from a training-camp league forecast.
        #[arg(long = "training-camp-authority", value_name = "PATH")]
        training_camp_authorities: Vec<PathBuf>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Run a seeded raw-profile scenario distribution through the full Window scorer.
    #[command(name = "window-scenario-distribute")]
    WindowScenarioDistribute {
        #[arg(long, value_name = "PATH")]
        baseline: PathBuf,
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Calibrate one Frame across frozen rolling-origin inputs.
    #[command(name = "window-calibrate")]
    WindowCalibrate {
        #[arg(long)]
        target: String,
        #[arg(long = "origin", required = true, value_name = "PATH")]
        origins: Vec<PathBuf>,
        #[arg(long, default_value_t = 3)]
        minimum_origins: usize,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Evaluate frozen training, validation, and retrospective-holdout origins.
    #[command(name = "window-evaluate")]
    WindowEvaluate {
        #[arg(long)]
        target: String,
        /// Labeled origin document; repeat in any order.
        #[arg(long = "origin", required = true, value_name = "PATH")]
        origins: Vec<PathBuf>,
        #[arg(long, default_value_t = 2)]
        minimum_training_origins: usize,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Fetch and seal an official final-standings outcome snapshot.
    #[command(name = "window-standings")]
    WindowStandings {
        #[arg(long)]
        target_season: u32,
        #[arg(long)]
        date: String,
        #[arg(long)]
        captured_at: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build one point-in-time historical Window origin from bundled facts.
    #[command(name = "window-origin-build")]
    WindowOriginBuild {
        #[arg(long)]
        source_season: u32,
        #[arg(long)]
        target_season: u32,
        #[arg(long)]
        as_of: String,
        #[arg(long)]
        generated_at: String,
        /// One of: training, validation, retrospective_holdout.
        #[arg(long)]
        role: String,
        #[arg(long, value_name = "PATH")]
        standings: PathBuf,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Freeze an outcome-free future Window holdout before results are observable.
    #[command(name = "window-holdout-register")]
    WindowHoldoutRegister {
        #[arg(long)]
        source_season: u32,
        #[arg(long)]
        target_season: u32,
        #[arg(long)]
        feature_cutoff: String,
        #[arg(long)]
        outcome_not_before: String,
        #[arg(long)]
        registered_at: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Score the exact registered future Window holdout after outcomes are eligible.
    #[command(name = "window-holdout-score")]
    WindowHoldoutScore {
        #[arg(long, value_name = "PATH")]
        registration: PathBuf,
        #[arg(long, value_name = "PATH")]
        standings: PathBuf,
        #[arg(long)]
        scored_at: String,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Prove The Window's production-source and future-holdout closeout gates together.
    #[command(name = "window-completion-status")]
    WindowCompletionStatus {
        /// Fresh output from `window-source-audit`.
        #[arg(long, value_name = "PATH")]
        source_audit: PathBuf,
        /// Exact preregistered future holdout.
        #[arg(long, value_name = "PATH")]
        holdout_registration: PathBuf,
        /// Scored holdout result, supplied only after the registered eligibility date.
        #[arg(long, value_name = "PATH")]
        holdout_result: Option<PathBuf>,
        /// RFC 3339 instant at which completion is evaluated.
        #[arg(long)]
        evaluated_at: String,
        /// Return a failing exit code after writing status unless both gates are complete.
        #[arg(long)]
        require_complete: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Project one team from a sealed league forecast into `card_document.v1`.
    #[command(name = "season-card")]
    SeasonCard {
        /// Full `icecast season --json` artifact; filtering happens only after it is fingerprinted.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        team: String,
        #[arg(long)]
        team_name: Option<String>,
        /// Evidence timestamp for deterministic output.
        #[arg(long)]
        generated_at: Option<String>,
        #[arg(long)]
        calendar_fingerprint: Option<String>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Compare two sealed league runs and report how each team's outlook moved.
    Movement {
        #[arg(long, value_name = "PATH")]
        earlier: PathBuf,
        #[arg(long, value_name = "PATH")]
        later: PathBuf,
        /// Repeat to focus text output; JSON always retains every team.
        #[arg(long = "team")]
        teams: Vec<String>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Project one team from a sealed movement artifact into `card_document.v1`.
    #[command(name = "movement-card")]
    MovementCard {
        /// Full `icecast movement --json` artifact; filtering happens after it is loaded.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        team: String,
        #[arg(long)]
        team_name: Option<String>,
        /// Evidence timestamp for deterministic output.
        #[arg(long)]
        generated_at: Option<String>,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Build a chronological trend from two or more sealed point-in-time forecasts.
    History {
        /// Repeat in chronological order for each `icecast season --through ... --json` artifact.
        #[arg(long = "input", required = true, value_name = "PATH")]
        inputs: Vec<PathBuf>,
        /// Repeat to focus text output; JSON always retains every team.
        #[arg(long = "team")]
        teams: Vec<String>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Cross-validate Elo blends from three or more season forecast JSON files.
    Backtest {
        /// Repeat for each `icecast season --json` artifact.
        #[arg(long = "input", required = true)]
        inputs: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Calibrate player breakout and downturn rates from consecutive completed seasons.
    #[command(name = "calibrate-development")]
    CalibrateDevelopment {
        #[arg(long, default_value_t = 20052006)]
        start_season: u32,
        #[arg(long, default_value_t = 20252026)]
        end_season: u32,
        /// Team-strength gain that defines a breakout.
        #[arg(long, default_value_t = 2.0)]
        breakout_threshold: f64,
        /// Team-strength loss that defines a downturn (must be negative).
        #[arg(long, default_value_t = -2.0, allow_hyphen_values = true)]
        downturn_threshold: f64,
        /// Global pseudo-observations used to stabilize small cohorts.
        #[arg(long, default_value_t = 20.0)]
        prior_sample_size: f64,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Explain a prospect's development, opportunity, injury, and attention gap.
    #[command(name = "prospect-study")]
    ProspectStudy {
        /// Authored `ProspectDevelopmentStudyInput` JSON with sourced context.
        #[arg(long, value_name = "PATH")]
        input: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate a conservative all-team AHL prospect context from reviewed identities.
    #[command(name = "prospect-context")]
    ProspectContext {
        /// Repeat for each official `ahl_roster_stats.v1` season snapshot.
        #[arg(long = "snapshot", required = true, value_name = "PATH")]
        snapshots: Vec<PathBuf>,
        /// Repeat for each reviewed `ahl_identity_league_crosswalk.v1` season envelope.
        #[arg(long = "league-crosswalk", required = true, value_name = "PATH")]
        league_crosswalks: Vec<PathBuf>,
        /// Dated `ahl_affiliation_catalog.v1` mapping the latest AHL clubs to NHL organizations.
        #[arg(long, value_name = "PATH")]
        affiliations: PathBuf,
        /// Age calculation date in YYYY-MM-DD form.
        #[arg(long, default_value = "2026-09-15")]
        as_of: String,
        /// Oldest player retained in the observed prospect draft.
        #[arg(long, default_value_t = 24)]
        max_age: u8,
        /// Minimum number of joined AHL seasons required.
        #[arg(long, default_value_t = 2)]
        minimum_ahl_seasons: usize,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Join reviewed AHL season facts into prospect studies and a discovery board.
    #[command(name = "prospect-league")]
    ProspectLeague {
        /// Repeat for each official `ahl_roster_stats.v1` season snapshot.
        #[arg(long = "snapshot", required = true, value_name = "PATH")]
        snapshots: Vec<PathBuf>,
        /// Repeat for each reviewed team crosswalk or league crosswalk envelope.
        #[arg(long = "crosswalk", required = true, value_name = "PATH")]
        crosswalks: Vec<PathBuf>,
        /// `prospect_league_context.v1` with separately authored non-feed facts.
        #[arg(long, value_name = "PATH")]
        context: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Generate neutral multi-league prospect context from the league camp pool.
    #[command(name = "prospect-career-context")]
    ProspectCareerContext {
        /// `training_camp_league_forecast.v1` selecting the prospect pool.
        #[arg(long = "camp-forecast", value_name = "PATH")]
        camp_forecast: PathBuf,
        /// Current canonical roster identity map containing birth dates.
        #[arg(long, value_name = "PATH")]
        rosters: PathBuf,
        /// Season bios used to cover camp fallback candidates.
        #[arg(long, value_name = "PATH")]
        bios: PathBuf,
        /// Optional sourced camp-candidate overlay used by the camp forecast.
        #[arg(long = "candidate-overlay", value_name = "PATH")]
        candidate_overlay: Option<PathBuf>,
        /// Optional career cache whose official landing birth dates fill identity gaps.
        #[arg(long = "career-history", value_name = "PATH")]
        career_history: Option<PathBuf>,
        #[arg(long, default_value = "2026-09-15")]
        as_of: String,
        #[arg(long, default_value_t = 24)]
        max_age: u8,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Adapt cached official multi-league career totals into prospect studies.
    #[command(name = "prospect-career")]
    ProspectCareer {
        /// `prospect_league_context.v1` with authored player context.
        #[arg(long, value_name = "PATH")]
        context: PathBuf,
        /// Cached `career_history.json` populated from official NHL player landing feeds.
        #[arg(long = "career-history", value_name = "PATH")]
        career_history: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Rank organizations by observed prospect pool, development, and pipeline signals.
    #[command(name = "prospect-program")]
    ProspectProgram {
        /// Repeat for each `prospect_league_discovery.v1` artifact.
        #[arg(long = "league-discovery", value_name = "PATH")]
        league_discoveries: Vec<PathBuf>,
        /// Repeat for each `prospect_career_discovery.v1` CHL/NCAA/Europe artifact.
        #[arg(long = "career-discovery", value_name = "PATH")]
        career_discoveries: Vec<PathBuf>,
        /// Add a canonical `prospect_development_study.v1` from another adapter.
        #[arg(long = "study", value_name = "PATH")]
        studies: Vec<PathBuf>,
        /// Optional prior `prospect_program_board.v2` used only for rank deltas.
        #[arg(long, value_name = "PATH")]
        prior_board: Option<PathBuf>,
        /// Maximum regular-season NHL GP retained in reserve-system scoring.
        /// Higher-GP players remain visible as graduates.
        #[arg(long, default_value_t = 50)]
        maximum_nhl_games: u32,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Measure organization rank sensitivity across prospect graduation boundaries.
    #[command(name = "prospect-program-sensitivity")]
    ProspectProgramSensitivity {
        #[arg(long = "league-discovery", value_name = "PATH")]
        league_discoveries: Vec<PathBuf>,
        #[arg(long = "career-discovery", value_name = "PATH")]
        career_discoveries: Vec<PathBuf>,
        #[arg(long = "study", value_name = "PATH")]
        studies: Vec<PathBuf>,
        /// NHL-GP graduation boundaries to compare.
        #[arg(long, value_delimiter = ',', default_value = "25,50,82")]
        thresholds: Vec<u32>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Recompute adjacent and first-to-latest trends from comparable annual program boards.
    #[command(name = "prospect-program-history")]
    ProspectProgramHistory {
        /// Repeat for each dated `prospect_program_board.v2` artifact.
        #[arg(long = "board", required = true, value_name = "PATH")]
        boards: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Measure frozen prospect cohorts against later official NHL outcomes.
    #[command(name = "prospect-conversion")]
    ProspectConversion {
        /// Repeat for each frozen `prospect_league_discovery.v1` artifact.
        #[arg(long = "league-discovery", value_name = "PATH")]
        league_discoveries: Vec<PathBuf>,
        /// Repeat for each frozen `prospect_career_discovery.v1` artifact.
        #[arg(long = "career-discovery", value_name = "PATH")]
        career_discoveries: Vec<PathBuf>,
        /// Add a frozen canonical `prospect_development_study.v1` artifact.
        #[arg(long = "study", value_name = "PATH")]
        studies: Vec<PathBuf>,
        /// Cached official NHL player landing career histories.
        #[arg(long = "career-history", value_name = "PATH")]
        career_history: PathBuf,
        /// Season represented by the frozen study cohort.
        #[arg(long = "baseline-season")]
        baseline_season: u32,
        /// Last NHL season included in realized outcomes.
        #[arg(long = "through-season")]
        through_season: u32,
        /// Optional `prospect_conversion_performance.v2` canonical NHL scores.
        /// When omitted, IceLines derives them from the official career cache.
        #[arg(long, value_name = "PATH")]
        performance: Option<PathBuf>,
        /// Write the supplied or derived NHL-performance authority for audit/reuse.
        #[arg(long = "performance-out", value_name = "PATH")]
        performance_out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Rank validated prospect studies into Hidden Gems, Buyer Beware, and Watch.
    #[command(name = "prospect-board")]
    ProspectBoard {
        /// Repeat for each `ProspectDevelopmentStudyView` JSON artifact.
        #[arg(long = "study", required = true, value_name = "PATH")]
        studies: Vec<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Import complete, timestamped Internet Archive captures of official NHL opening rosters.
    #[command(name = "import-opening-rosters")]
    ImportOpeningRosters {
        /// Provenance manifest with one immutable official NHL archive URL per season team.
        #[arg(long, value_name = "PATH")]
        manifest: PathBuf,
        /// Validate provenance and team coverage without downloading or writing a snapshot.
        #[arg(long)]
        dry_run: bool,
        /// Permit a sealed evaluation-only snapshot with less than full league coverage.
        #[arg(long)]
        allow_partial_evaluation: bool,
    },
    /// Discover pre-opening official NHL roster captures in the Internet Archive.
    #[command(name = "discover-opening-rosters")]
    DiscoverOpeningRosters {
        #[arg(long)]
        season: u32,
        /// Write the complete coverage report; stdout is used when omitted.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Write an importer-ready manifest only when every season team is covered.
        #[arg(long, value_name = "PATH")]
        manifest_out: Option<PathBuf>,
        /// Write a partial evaluation manifest whenever at least one team is covered.
        #[arg(long, value_name = "PATH")]
        partial_manifest_out: Option<PathBuf>,
        /// Re-evaluate cached CDX responses without contacting the Internet Archive.
        #[arg(long)]
        cache_only: bool,
    },
    /// Import, inspect, and list stable IceCast scenarios.
    Scenario {
        #[command(subcommand)]
        command: IceCastScenarioSubcommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum IceCastScenarioSubcommand {
    /// Import a CLI-authored JSON scenario into the stable local registry.
    Import {
        #[arg(long, value_name = "ID")]
        id: String,
        #[arg(long, value_name = "PATH")]
        path: PathBuf,
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(
            long,
            default_value = "estimated",
            value_parser = ["confirmed", "reported", "estimated", "simulated", "under-review", "no-read"]
        )]
        evidence: String,
        #[arg(long)]
        json: bool,
    },
    /// List registered scenario IDs and immutable hashes.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Show one registered scenario and its metadata.
    Show {
        id: String,
        #[arg(long)]
        json: bool,
    },
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
    /// Open the sealed IceCast prognosis card for NYR or SEA.
    TeamCard {
        /// Showcase team abbreviation (`NYR` or `SEA`).
        team: String,
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
            TuiSurface::TeamCard { team } => ScreenSpec::TeamCard(team),
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

    fn with_large_stack(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .name("clap-surface-test".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(test)
            .expect("spawn clap surface test")
            .join()
            .expect("clap surface test");
    }

    #[test]
    fn l0_window_bridge_commands_parse() {
        with_large_stack(|| {
            let history_audit = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-profile-history-audit",
                "--input",
                "profile-history.json",
                "--generated-at",
                "2026-07-30T12:00:00Z",
                "--out",
                "profile-history-coverage.json",
            ])
            .expect("Window profile history audit should parse");
            assert!(matches!(
                history_audit.command,
                Commands::Icecast(IceCastSubcommand::WindowProfileHistoryAudit { .. })
            ));

            let source_package = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-package",
                "--season",
                "20262027",
                "--as-of",
                "2026-10-01",
                "--team-game-forecast",
                "games.json",
                "--cache-team-lineups",
                "--stats-season",
                "20252026",
                "--ahl-affiliate",
                "hartford.json",
                "--out",
                "window-sources.json",
            ])
            .expect("Window source package should parse");
            assert!(matches!(
                source_package.command,
                Commands::Icecast(IceCastSubcommand::WindowSourcePackage { .. })
            ));

            let league_affiliates = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-package",
                "--season",
                "20262027",
                "--as-of",
                "2026-10-01",
                "--ahl-projection-inputs",
                "ahl-league-inputs.json",
                "--out",
                "window-sources.json",
            ])
            .expect("league AHL projection inputs should parse");
            assert!(matches!(
                league_affiliates.command,
                Commands::Icecast(IceCastSubcommand::WindowSourcePackage {
                    ahl_projection_inputs: Some(_),
                    ..
                })
            ));
            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-package",
                "--as-of",
                "2026-10-01",
                "--ahl-affiliate",
                "hartford.json",
                "--ahl-projection-inputs",
                "ahl-league-inputs.json",
                "--out",
                "window-sources.json",
            ])
            .is_err());

            let refresh_lineups = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-refresh-lineups",
                "--input",
                "window-sources.json",
                "--stats-season",
                "20252026",
                "--training-camp",
                "camp.json",
                "--career-history",
                "career.json",
                "--out",
                "window-refreshed.json",
            ])
            .expect("Window lineup refresh should parse");
            assert!(matches!(
                refresh_lineups.command,
                Commands::Icecast(IceCastSubcommand::WindowSourceRefreshLineups { .. })
            ));

            let refresh_history = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-refresh-history",
                "--input",
                "window-sources.json",
                "--profile-history",
                "profile-history.json",
                "--out",
                "window-history.json",
            ])
            .expect("Window history refresh should parse");
            assert!(matches!(
                refresh_history.command,
                Commands::Icecast(IceCastSubcommand::WindowSourceRefreshHistory { .. })
            ));

            let refresh_affiliates = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-refresh-affiliates",
                "--input",
                "window-sources.json",
                "--ahl-projection-inputs",
                "ahl-league-inputs.json",
                "--out",
                "window-affiliates.json",
            ])
            .expect("Window affiliate refresh should parse");
            assert!(matches!(
                refresh_affiliates.command,
                Commands::Icecast(IceCastSubcommand::WindowSourceRefreshAffiliates {
                    ahl_projection_inputs: Some(_),
                    ahl_facts_application: None,
                    ..
                })
            ));

            let refresh_affiliates_from_facts = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-refresh-affiliates",
                "--input",
                "window-sources.json",
                "--ahl-facts-application",
                "ahl-facts.json",
                "--ahl-development-rule",
                "ahl-rule.json",
                "--out",
                "window-affiliates.json",
            ])
            .expect("direct Window affiliate facts refresh should parse");
            assert!(matches!(
                refresh_affiliates_from_facts.command,
                Commands::Icecast(IceCastSubcommand::WindowSourceRefreshAffiliates {
                    ahl_projection_inputs: None,
                    ahl_facts_application: Some(_),
                    ahl_development_rule: Some(_),
                    ..
                })
            ));
            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-refresh-affiliates",
                "--input",
                "window-sources.json",
                "--ahl-facts-application",
                "ahl-facts.json",
                "--out",
                "window-affiliates.json",
            ])
            .is_err());
            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-refresh-affiliates",
                "--input",
                "window-sources.json",
                "--ahl-projection-inputs",
                "ahl-league-inputs.json",
                "--ahl-facts-application",
                "ahl-facts.json",
                "--ahl-development-rule",
                "ahl-rule.json",
                "--out",
                "window-affiliates.json",
            ])
            .is_err());

            let cache_prospects = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-package",
                "--season",
                "20262027",
                "--as-of",
                "2026-07-28",
                "--training-camp",
                "camp.json",
                "--cache-prospect-program",
                "--career-history",
                "career.json",
                "--out",
                "window-sources.json",
            ])
            .expect("cache-native prospect package should parse");
            assert!(matches!(
                cache_prospects.command,
                Commands::Icecast(IceCastSubcommand::WindowSourcePackage {
                    cache_prospect_program: true,
                    career_history: Some(_),
                    ..
                })
            ));
            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-package",
                "--as-of",
                "2026-07-28",
                "--cache-prospect-program",
                "--out",
                "window-sources.json",
            ])
            .is_err());
            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-package",
                "--as-of",
                "2026-07-28",
                "--training-camp",
                "camp.json",
                "--cache-prospect-program",
                "--prospect-program",
                "program.json",
                "--out",
                "window-sources.json",
            ])
            .is_err());

            let source_audit = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-source-audit",
                "--input",
                "window-sources.json",
                "--generated-at",
                "2026-10-01T12:00:00Z",
            ])
            .expect("Window source audit should parse");
            assert!(matches!(
                source_audit.command,
                Commands::Icecast(IceCastSubcommand::WindowSourceAudit { .. })
            ));

            let package_build = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-build",
                "--season",
                "20262027",
                "--as-of",
                "2026-10-01",
                "--generated-at",
                "2026-10-01T12:00:00Z",
                "--source-package",
                "window-sources.json",
                "--require-ranked",
                "--out",
                "window.json",
            ])
            .expect("packaged ranked Window build should parse");
            assert!(matches!(
                package_build.command,
                Commands::Icecast(IceCastSubcommand::WindowBuild {
                    source_package: Some(_),
                    require_ranked: true,
                    ..
                })
            ));

            let movement = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-movement",
                "--earlier",
                "october.json",
                "--later",
                "january.json",
                "--bridge",
                "v1-to-v2.json",
            ])
            .expect("bridged Window movement should parse");
            assert!(matches!(
                movement.command,
                Commands::Icecast(IceCastSubcommand::WindowMovement {
                    bridge: Some(_),
                    ..
                })
            ));

            let attribution = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-personnel-attribution",
                "--earlier",
                "october.json",
                "--later",
                "january.json",
                "--movement",
                "movement.json",
                "--input",
                "personnel.json",
            ])
            .expect("Window personnel attribution should parse");
            assert!(matches!(
                attribution.command,
                Commands::Icecast(IceCastSubcommand::WindowPersonnelAttribution { .. })
            ));

            let personnel_input = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-personnel-input-build",
                "--actual-forecast",
                "actual.json",
                "--counterfactual-board",
                "counterfactual.json",
                "--earlier-as-of",
                "2025-01-31",
                "--later-as-of",
                "2025-02-28",
                "--attribution-id",
                "january-february",
                "--scenario-id",
                "paired-replay",
                "--rationale",
                "Paired replay evidence",
                "--out",
                "personnel-input.json",
            ])
            .expect("Window personnel input builder should parse");
            assert!(matches!(
                personnel_input.command,
                Commands::Icecast(IceCastSubcommand::WindowPersonnelInputBuild { .. })
            ));

            let personnel_summary = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-personnel-summary",
                "--input",
                "attributed-movement.json",
            ])
            .expect("Window personnel summary should parse");
            assert!(matches!(
                personnel_summary.command,
                Commands::Icecast(IceCastSubcommand::WindowPersonnelSummary { .. })
            ));

            let rebase = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-rebase",
                "--input",
                "october.json",
                "--target-manifest",
                "balanced-v2.json",
                "--bridge",
                "v1-to-v2.json",
            ])
            .expect("Window rebase should parse");
            assert!(matches!(
                rebase.command,
                Commands::Icecast(IceCastSubcommand::WindowRebase { .. })
            ));

            let scenario = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-scenario",
                "--baseline",
                "baseline.json",
                "--scenario",
                "trade.json",
                "--scenario-id",
                "deadline-addition",
                "--authority",
                "trade-authority.json",
                "--team-season-authority",
                "season-scenario.json",
                "--training-camp-authority",
                "camp-scenario.json",
            ])
            .expect("typed Window scenario should parse");
            assert!(matches!(
                scenario.command,
                Commands::Icecast(IceCastSubcommand::WindowScenario {
                    authorities,
                    team_season_authorities,
                    training_camp_authorities,
                    ..
                }) if authorities.len() == 1
                    && team_season_authorities.len() == 1
                    && training_camp_authorities.len() == 1
            ));

            let distribution = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-scenario-distribute",
                "--baseline",
                "baseline.json",
                "--input",
                "distribution.json",
                "--out",
                "result.json",
            ])
            .expect("Window scenario distribution should parse");
            assert!(matches!(
                distribution.command,
                Commands::Icecast(IceCastSubcommand::WindowScenarioDistribute { .. })
            ));

            let calibration = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-calibrate",
                "--target",
                "next-season-value",
                "--origin",
                "2023.json",
                "--origin",
                "2024.json",
                "--origin",
                "2025.json",
            ])
            .expect("rolling Window calibration should parse");
            assert!(matches!(
                calibration.command,
                Commands::Icecast(IceCastSubcommand::WindowCalibrate {
                    origins,
                    minimum_origins: 3,
                    ..
                }) if origins.len() == 3
            ));

            let evaluation = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-evaluate",
                "--target",
                "next-season-value",
                "--origin",
                "2022-train.json",
                "--origin",
                "2023-train.json",
                "--origin",
                "2024-validation.json",
                "--origin",
                "2025-holdout.json",
            ])
            .expect("split Window evaluation should parse");
            assert!(matches!(
                evaluation.command,
                Commands::Icecast(IceCastSubcommand::WindowEvaluate {
                    origins,
                    minimum_training_origins: 2,
                    ..
                }) if origins.len() == 4
            ));

            let standings = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-standings",
                "--target-season",
                "20252026",
                "--date",
                "2026-04-17",
                "--captured-at",
                "2026-07-28T08:00:00Z",
                "--out",
                "standings.json",
            ])
            .expect("historical Window standings should parse");
            assert!(matches!(
                standings.command,
                Commands::Icecast(IceCastSubcommand::WindowStandings {
                    target_season: 20252026,
                    ..
                })
            ));

            let origin = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-origin-build",
                "--source-season",
                "20242025",
                "--target-season",
                "20252026",
                "--as-of",
                "2025-06-30",
                "--generated-at",
                "2026-07-28T08:00:00Z",
                "--role",
                "retrospective_holdout",
                "--standings",
                "standings.json",
            ])
            .expect("historical Window origin should parse");
            assert!(matches!(
                origin.command,
                Commands::Icecast(IceCastSubcommand::WindowOriginBuild {
                    source_season: 20242025,
                    target_season: 20252026,
                    ..
                })
            ));

            let holdout = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-holdout-register",
                "--source-season",
                "20252026",
                "--target-season",
                "20262027",
                "--feature-cutoff",
                "2026-06-30",
                "--outcome-not-before",
                "2027-04-11",
                "--registered-at",
                "2026-07-29T12:00:00Z",
                "--out",
                "future-holdout.json",
            ])
            .expect("future Window holdout registration should parse");
            assert!(matches!(
                holdout.command,
                Commands::Icecast(IceCastSubcommand::WindowHoldoutRegister {
                    source_season: 20252026,
                    target_season: 20262027,
                    ..
                })
            ));

            let holdout_score = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-holdout-score",
                "--registration",
                "future-holdout.json",
                "--standings",
                "standings-2026-27.json",
                "--scored-at",
                "2027-04-11T13:00:00Z",
                "--out",
                "future-holdout-result.json",
            ])
            .expect("future Window holdout scoring should parse");
            assert!(matches!(
                holdout_score.command,
                Commands::Icecast(IceCastSubcommand::WindowHoldoutScore { .. })
            ));

            let completion = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-completion-status",
                "--source-audit",
                "source-audit.json",
                "--holdout-registration",
                "future-holdout.json",
                "--evaluated-at",
                "2026-07-30T12:00:00Z",
                "--require-complete",
                "--out",
                "window-completion.json",
            ])
            .expect("Window completion status should parse");
            assert!(matches!(
                completion.command,
                Commands::Icecast(IceCastSubcommand::WindowCompletionStatus {
                    require_complete: true,
                    ..
                })
            ));

            let report = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window",
                "--input",
                "window.json",
                "--team",
                "NYR",
                "--markdown",
            ])
            .expect("Window Markdown report should parse");
            assert!(matches!(
                report.command,
                Commands::Icecast(IceCastSubcommand::Window { markdown: true, .. })
            ));
            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "window",
                "--input",
                "window.json",
                "--json",
                "--markdown",
            ])
            .is_err());
        });
    }

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
        with_large_stack(|| {
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
                    assert!(start.is_none());
                    assert!(layout.is_none());
                }
                other => panic!("expected Tui, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_tui_team_card_surface_parses_and_resolves() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from(["icelines", "tui", "team-card", "sea"])
                .expect("team-card sugar parses");
            let spec = match cli.command {
                Commands::Tui {
                    surface: Some(surface),
                    ..
                } => surface.into_screen_spec(),
                other => panic!("expected Tui team-card, got {other:?}"),
            };
            assert_eq!(spec, ScreenSpec::TeamCard("sea".to_string()));
            assert_eq!(
                spec.into_screen().unwrap(),
                crate::tui::app::Screen::TeamCard {
                    team: "SEA".to_string(),
                    compare: false,
                }
            );
        });
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
    fn l0_icecast_season_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "season",
                "--season",
                "20262027",
                "--stats-season",
                "20252026",
                "--team",
                "NYR",
                "--team",
                "SEA",
                "--trials",
                "25000",
                "--seed",
                "17",
                "--scenario",
                "scenario.json",
                "--isolated-impacts",
                "--auto-personnel",
                "--trade-mode",
                "plausible",
                "--replay-mode",
                "rolling",
                "--ignore-replay-personnel-after",
                "2026-12-31",
                "--through",
                "2027-01-15",
                "--retrospective-opening-lineups",
                "--all-games",
                "--game-forecast-out",
                "games.json",
                "--json",
            ])
            .expect("IceCast season command should parse");

            match cli.command {
                Commands::Icecast(IceCastSubcommand::Season {
                    season,
                    stats_season,
                    teams,
                    trials,
                    seed,
                    scenario,
                    scenario_id,
                    isolated_impacts,
                    auto_personnel,
                    trade_mode,
                    replay_mode,
                    ignore_replay_personnel_after,
                    through,
                    retrospective_opening_lineups,
                    all_games,
                    game_forecast_out,
                    json,
                    ..
                }) => {
                    assert_eq!(season, 20262027);
                    assert_eq!(stats_season, 20252026);
                    assert_eq!(teams, ["NYR", "SEA"]);
                    assert_eq!(trials, 25_000);
                    assert_eq!(seed, 17);
                    assert_eq!(scenario, Some(PathBuf::from("scenario.json")));
                    assert_eq!(scenario_id, None);
                    assert!(isolated_impacts);
                    assert!(auto_personnel);
                    assert_eq!(trade_mode, "plausible");
                    assert_eq!(replay_mode, "rolling");
                    assert_eq!(
                        ignore_replay_personnel_after.unwrap().to_string(),
                        "2026-12-31"
                    );
                    assert_eq!(through.unwrap().to_string(), "2027-01-15");
                    assert!(retrospective_opening_lineups);
                    assert!(all_games);
                    assert_eq!(game_forecast_out, Some(PathBuf::from("games.json")));
                    assert!(json);
                }
                other => panic!("expected IceCast season command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_blender_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "blender",
                "--lineup",
                "lineup.json",
                "--pair-evidence",
                "pairs.json",
                "--review-games",
                "8",
                "--minimum-points-percentage",
                "0.55",
                "--max-changes",
                "2",
                "--max-choices",
                "4",
                "--scenario-out",
                "bench.json",
                "--json",
            ])
            .expect("IceCast Blender command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Blender {
                    lineup,
                    pair_evidence,
                    review_games,
                    minimum_points_percentage,
                    max_changes,
                    max_choices,
                    scenario_out,
                    json,
                    ..
                }) => {
                    assert_eq!(lineup, PathBuf::from("lineup.json"));
                    assert_eq!(pair_evidence, Some(PathBuf::from("pairs.json")));
                    assert_eq!(review_games, 8);
                    assert_eq!(minimum_points_percentage, 0.55);
                    assert_eq!(max_changes, 2);
                    assert_eq!(max_choices, 4);
                    assert_eq!(scenario_out, Some(PathBuf::from("bench.json")));
                    assert!(json);
                }
                other => panic!("expected IceCast Blender command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_bench_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "bench",
                "--forecast",
                "games.json",
                "--lineup",
                "lineup.json",
                "--profile",
                "manager.json",
                "--style-evidence",
                "styles.json",
                "--stats-season",
                "20252026",
                "--scenario-out",
                "scenario.json",
                "--json",
            ])
            .expect("IceCast Bench command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Bench {
                    forecast,
                    lineup,
                    profile,
                    style_evidence,
                    stats_season,
                    scenario_out,
                    json,
                    ..
                }) => {
                    assert_eq!(forecast, PathBuf::from("games.json"));
                    assert_eq!(lineup, PathBuf::from("lineup.json"));
                    assert_eq!(profile, PathBuf::from("manager.json"));
                    assert_eq!(style_evidence, PathBuf::from("styles.json"));
                    assert_eq!(stats_season, 20252026);
                    assert_eq!(scenario_out, Some(PathBuf::from("scenario.json")));
                    assert!(json);
                }
                other => panic!("expected IceCast Bench command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_camp_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "camp",
                "--input",
                "camp.json",
                "--trials",
                "5000",
                "--seed",
                "27",
                "--lineup-set-out",
                "lineups.json",
                "--max-lineup-branches",
                "3",
                "--blender-set-out",
                "blenders.json",
                "--season-scenario-out",
                "camp-scenario.json",
                "--season-max-roster-branches",
                "2500",
                "--json",
            ])
            .expect("IceCast camp command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Camp {
                    input,
                    trials,
                    seed,
                    json,
                    lineup_set_out,
                    max_lineup_branches,
                    blender_set_out,
                    season_scenario_out,
                    season_max_roster_branches,
                    ..
                }) => {
                    assert_eq!(input, PathBuf::from("camp.json"));
                    assert_eq!(trials, Some(5000));
                    assert_eq!(seed, Some(27));
                    assert_eq!(lineup_set_out, Some(PathBuf::from("lineups.json")));
                    assert_eq!(max_lineup_branches, 3);
                    assert_eq!(blender_set_out, Some(PathBuf::from("blenders.json")));
                    assert_eq!(
                        season_scenario_out,
                        Some(PathBuf::from("camp-scenario.json"))
                    );
                    assert_eq!(season_max_roster_branches, 2500);
                    assert!(json);
                }
                other => panic!("expected IceCast camp command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_blender_shift_surface_parses_and_conflicts_with_manual_pairs() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "blender",
                "--lineup",
                "lineup.json",
                "--shift-season",
                "20252026",
                "--shift-report-out",
                "shifts.json",
                "--allow-off-wing",
            ])
            .expect("IceCast Blender shift command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Blender {
                    shift_season,
                    shift_report_out,
                    allow_off_wing,
                    ..
                }) => {
                    assert_eq!(shift_season, Some(20252026));
                    assert_eq!(shift_report_out, Some(PathBuf::from("shifts.json")));
                    assert!(allow_off_wing);
                }
                other => panic!("expected IceCast Blender command, got {other:?}"),
            }
            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "blender",
                "--lineup",
                "lineup.json",
                "--shift-season",
                "20252026",
                "--pair-evidence",
                "pairs.json",
            ])
            .is_err());
        });
    }

    #[test]
    fn l0_icecast_season_card_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "season-card",
                "--input",
                "league-run.json",
                "--team",
                "NYR",
                "--team-name",
                "New York Rangers",
                "--generated-at",
                "2026-07-22T12:00:00Z",
                "--calendar-fingerprint",
                "schedule-2026-27",
                "--out",
                "nyr-card.json",
            ])
            .expect("IceCast season-card command should parse");

            match cli.command {
                Commands::Icecast(IceCastSubcommand::SeasonCard {
                    input,
                    team,
                    team_name,
                    generated_at,
                    calendar_fingerprint,
                    out,
                }) => {
                    assert_eq!(input, PathBuf::from("league-run.json"));
                    assert_eq!(team, "NYR");
                    assert_eq!(team_name.as_deref(), Some("New York Rangers"));
                    assert_eq!(generated_at.as_deref(), Some("2026-07-22T12:00:00Z"));
                    assert_eq!(calendar_fingerprint.as_deref(), Some("schedule-2026-27"));
                    assert_eq!(out, Some(PathBuf::from("nyr-card.json")));
                }
                other => panic!("expected IceCast season-card command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_window_profile_history_backfill_and_delta_surfaces_parse() {
        with_large_stack(|| {
            let backfill = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-profile-history-backfill",
                "--origin",
                "origin-a.json",
                "--origin",
                "origin-b.json",
                "--history-id",
                "observed-history",
                "--created-at",
                "2026-07-30T18:00:00Z",
                "--out",
                "history.json",
            ])
            .expect("profile history backfill should parse");
            assert!(matches!(
                backfill.command,
                Commands::Icecast(IceCastSubcommand::WindowProfileHistoryBackfill {
                    origins,
                    ..
                }) if origins.len() == 2
            ));

            let delta = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-profile-history-delta",
                "--input",
                "history.json",
                "--earlier-season",
                "20242025",
                "--earlier-as-of",
                "2024-06-30",
                "--later-season",
                "20252026",
                "--later-as-of",
                "2025-06-30",
                "--generated-at",
                "2026-07-30T18:00:00Z",
            ])
            .expect("profile history delta should parse");
            assert!(matches!(
                delta.command,
                Commands::Icecast(IceCastSubcommand::WindowProfileHistoryDelta {
                    earlier_season: 20242025,
                    later_season: 20252026,
                    ..
                })
            ));

            let card = Cli::try_parse_from([
                "icelines",
                "icecast",
                "window-profile-history-card",
                "--input",
                "delta.json",
                "--team",
                "NYR",
                "--out",
                "nyr-history-card.json",
            ])
            .expect("profile history card should parse");
            assert!(matches!(
                card.command,
                Commands::Icecast(IceCastSubcommand::WindowProfileHistoryCard {
                    team,
                    ..
                }) if team == "NYR"
            ));
        });
    }

    #[test]
    fn l0_icecast_movement_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "movement",
                "--earlier",
                "jan.json",
                "--later",
                "feb.json",
                "--team",
                "NYR",
                "--team",
                "SEA",
                "--json",
                "--out",
                "movement.json",
            ])
            .expect("IceCast movement command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Movement {
                    earlier,
                    later,
                    teams,
                    json,
                    out,
                }) => {
                    assert_eq!(earlier, PathBuf::from("jan.json"));
                    assert_eq!(later, PathBuf::from("feb.json"));
                    assert_eq!(teams, ["NYR", "SEA"]);
                    assert!(json);
                    assert_eq!(out, Some(PathBuf::from("movement.json")));
                }
                other => panic!("expected IceCast movement command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_movement_card_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "movement-card",
                "--input",
                "movement.json",
                "--team",
                "NYR",
                "--team-name",
                "New York Rangers",
                "--generated-at",
                "2027-02-15T12:00:00Z",
                "--out",
                "nyr-movement-card.json",
            ])
            .expect("IceCast movement-card command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::MovementCard {
                    input,
                    team,
                    team_name,
                    generated_at,
                    out,
                }) => {
                    assert_eq!(input, PathBuf::from("movement.json"));
                    assert_eq!(team, "NYR");
                    assert_eq!(team_name.as_deref(), Some("New York Rangers"));
                    assert_eq!(generated_at.as_deref(), Some("2027-02-15T12:00:00Z"));
                    assert_eq!(out, Some(PathBuf::from("nyr-movement-card.json")));
                }
                other => panic!("expected IceCast movement-card command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_history_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "history",
                "--input",
                "jan.json",
                "--input",
                "feb.json",
                "--team",
                "NYR",
                "--json",
                "--out",
                "history.json",
            ])
            .expect("IceCast history command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::History {
                    inputs,
                    teams,
                    json,
                    out,
                }) => {
                    assert_eq!(
                        inputs,
                        [PathBuf::from("jan.json"), PathBuf::from("feb.json")]
                    );
                    assert_eq!(teams, ["NYR"]);
                    assert!(json);
                    assert_eq!(out, Some(PathBuf::from("history.json")));
                }
                other => panic!("expected IceCast history command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_history_card_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "history-card",
                "--input",
                "history.json",
                "--team",
                "SEA",
                "--team-name",
                "Seattle Kraken",
                "--generated-at",
                "2025-03-01T12:00:00Z",
                "--out",
                "sea-history-card.json",
            ])
            .expect("IceCast history-card command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::HistoryCard {
                    input,
                    team,
                    team_name,
                    generated_at,
                    out,
                }) => {
                    assert_eq!(input, PathBuf::from("history.json"));
                    assert_eq!(team, "SEA");
                    assert_eq!(team_name.as_deref(), Some("Seattle Kraken"));
                    assert_eq!(generated_at.as_deref(), Some("2025-03-01T12:00:00Z"));
                    assert_eq!(out, Some(PathBuf::from("sea-history-card.json")));
                }
                other => panic!("expected IceCast history-card command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_scenario_registry_surfaces_parse_and_paths_conflict() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "season",
                "--scenario-id",
                "nyr-development-variance",
            ])
            .expect("registered scenario ID should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Season {
                    scenario,
                    scenario_id,
                    ..
                }) => {
                    assert_eq!(scenario, None);
                    assert_eq!(scenario_id.as_deref(), Some("nyr-development-variance"));
                }
                other => panic!("expected IceCast season command, got {other:?}"),
            }

            assert!(Cli::try_parse_from([
                "icelines",
                "icecast",
                "season",
                "--scenario",
                "scenario.json",
                "--scenario-id",
                "nyr-development-variance",
            ])
            .is_err());

            for args in [
                vec![
                    "icelines",
                    "icecast",
                    "scenario",
                    "import",
                    "--id",
                    "nyr-development-variance",
                    "--path",
                    "scenario.json",
                    "--season",
                    "20262027",
                    "--evidence",
                    "estimated",
                    "--json",
                ],
                vec!["icelines", "icecast", "scenario", "list", "--json"],
                vec![
                    "icelines",
                    "icecast",
                    "scenario",
                    "show",
                    "nyr-development-variance",
                    "--json",
                ],
            ] {
                Cli::try_parse_from(args).expect("scenario registry command should parse");
            }
        });
    }

    #[test]
    fn l0_icecast_backtest_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "backtest",
                "--input",
                "2021.json",
                "--input",
                "2022.json",
                "--input",
                "2023.json",
                "--json",
            ])
            .expect("IceCast backtest command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Backtest { inputs, json, .. }) => {
                    assert_eq!(inputs.len(), 3);
                    assert!(json);
                }
                other => panic!("expected IceCast backtest command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_team_lineup_report_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "report",
                "team-lineup",
                "--team",
                "NYR",
                "--roster-season",
                "20262027",
                "--stats-season",
                "20252026",
                "--json",
            ])
            .expect("team lineup report should parse");
            match cli.command {
                Commands::Report(ReportSubcommand::TeamLineup {
                    roster_season,
                    stats_season,
                    team,
                    json,
                    ..
                }) => {
                    assert_eq!(roster_season, "20262027");
                    assert_eq!(stats_season, "20252026");
                    assert_eq!(team, "NYR");
                    assert!(json);
                }
                other => panic!("expected team lineup report, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_organization_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "organization",
                "--input",
                "organization.json",
                "--json",
                "--out",
                "the-system.json",
            ])
            .expect("organization lineup command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::Organization { input, json, out }) => {
                    assert_eq!(input, PathBuf::from("organization.json"));
                    assert!(json);
                    assert_eq!(out, Some(PathBuf::from("the-system.json")));
                }
                other => panic!("expected IceCast organization command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_affiliate_identity_surfaces_parse() {
        with_large_stack(|| {
            let review = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-identities",
                "--snapshot",
                "ahl.json",
                "--team",
                "Hartford Wolf Pack",
                "--candidates",
                "candidates.json",
                "--json",
                "--out",
                "review.json",
            ])
            .expect("affiliate identity review command should parse");
            assert!(matches!(
                review.command,
                Commands::Icecast(IceCastSubcommand::AffiliateIdentities { json: true, .. })
            ));

            let discovered = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-identities",
                "--snapshot",
                "ahl.json",
                "--team",
                "Hartford Wolf Pack",
                "--discover-official",
                "--refresh",
                "--json",
            ])
            .expect("official affiliate identity discovery should parse without a catalog");
            assert!(matches!(
                discovered.command,
                Commands::Icecast(IceCastSubcommand::AffiliateIdentities {
                    candidates: None,
                    discover_official: true,
                    refresh: true,
                    ..
                })
            ));

            let league_discovered = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-identities-league",
                "--snapshot",
                "ahl-season.json",
                "--discover-official",
                "--refresh",
                "--json",
                "--out",
                "ahl-league-crosswalk.json",
            ])
            .expect("official league affiliate identity discovery should parse");
            assert!(matches!(
                league_discovered.command,
                Commands::Icecast(IceCastSubcommand::AffiliateIdentitiesLeague {
                    candidates: None,
                    discover_official: true,
                    refresh: true,
                    json: true,
                    ..
                })
            ));

            let draft = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-draft",
                "--crosswalk",
                "review.json",
                "--include-aliases",
                "--include-conflicts",
                "--out",
                "decisions-draft.json",
            ])
            .expect("affiliate identity review draft should parse");
            assert!(matches!(
                draft.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewDraft {
                    include_aliases: true,
                    include_conflicts: true,
                    ..
                })
            ));

            let league_draft = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-draft-league",
                "--league-crosswalk",
                "ahl-reviewed.json",
                "--include-conflicts",
                "--out",
                "ahl-exception-drafts.json",
            ])
            .expect("league affiliate identity review draft should parse");
            assert!(matches!(
                league_draft.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewDraftLeague {
                    include_aliases: false,
                    include_conflicts: true,
                    ..
                })
            ));

            let exact = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-exact",
                "--crosswalk",
                "review.json",
                "--reviewer",
                "identity-pilot",
                "--reviewed-at",
                "2026-07-25T12:00:00Z",
                "--decisions-out",
                "exact-decisions.json",
                "--json",
                "--out",
                "exact-reviewed.json",
            ])
            .expect("exact affiliate identity review should parse");
            assert!(matches!(
                exact.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewExact {
                    reviewer,
                    json: true,
                    ..
                }) if reviewer == "identity-pilot"
            ));

            let exact_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-exact-league",
                "--league-crosswalk",
                "ahl-league.json",
                "--reviewer",
                "league-exact-pilot",
                "--reviewed-at",
                "2026-07-25T12:30:00Z",
                "--decisions-out",
                "league-exact-decisions.json",
                "--json",
                "--out",
                "league-exact-reviewed.json",
            ])
            .expect("exact league affiliate identity review should parse");
            assert!(matches!(
                exact_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewExactLeague {
                    reviewer,
                    json: true,
                    ..
                }) if reviewer == "league-exact-pilot"
            ));

            let aliases = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-aliases",
                "--crosswalk",
                "exact-reviewed.json",
                "--reviewer",
                "alias-pilot",
                "--reviewed-at",
                "2026-07-25T13:00:00Z",
                "--decisions-out",
                "alias-decisions.json",
                "--json",
                "--out",
                "alias-reviewed.json",
            ])
            .expect("alias affiliate identity review should parse");
            assert!(matches!(
                aliases.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewAliases {
                    reviewer,
                    json: true,
                    ..
                }) if reviewer == "alias-pilot"
            ));

            let aliases_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-aliases-league",
                "--league-crosswalk",
                "league-exact-reviewed.json",
                "--reviewer",
                "league-alias-pilot",
                "--reviewed-at",
                "2026-07-25T13:30:00Z",
                "--decisions-out",
                "league-alias-decisions.json",
                "--json",
                "--out",
                "league-alias-reviewed.json",
            ])
            .expect("alias league affiliate identity review should parse");
            assert!(matches!(
                aliases_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewAliasesLeague {
                    reviewer,
                    json: true,
                    ..
                }) if reviewer == "league-alias-pilot"
            ));

            let conflicts_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-conflicts-league",
                "--league-crosswalk",
                "league-alias-reviewed.json",
                "--nhl-player-id",
                "8482739",
                "--evidence-url",
                "https://example.test/ahl/brett-harrison",
                "--evidence-url",
                "https://example.test/nhl/brett-harrison",
                "--reviewer",
                "league-conflict-pilot",
                "--reviewed-at",
                "2026-07-26T20:10:00Z",
                "--note",
                "official NHL club evidence controls the canonical date",
                "--decisions-out",
                "league-conflict-decisions.json",
                "--json",
                "--out",
                "league-conflict-reviewed.json",
            ])
            .expect("conflict league affiliate identity review should parse");
            assert!(matches!(
                conflicts_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewConflictsLeague {
                    nhl_player_id,
                    evidence_urls,
                    reviewer,
                    json: true,
                    ..
                }) if nhl_player_id == [8_482_739]
                    && evidence_urls.len() == 2
                    && reviewer == "league-conflict-pilot"
            ));

            let birth_date_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-birth-date-league",
                "--league-crosswalk",
                "league-conflict-reviewed.json",
                "--nhl-player-id",
                "8484115",
                "--canonical-birth-date",
                "1999-04-17",
                "--evidence-url",
                "https://www.iowawild.com/players/detail/zmolek-1",
                "--reviewer",
                "league-date-pilot",
                "--reviewed-at",
                "2026-07-26T22:50:00Z",
                "--note",
                "official club and college sources agree with the provider date",
                "--decisions-out",
                "league-date-decisions.json",
                "--json",
                "--out",
                "league-date-reviewed.json",
            ])
            .expect("birth-date league affiliate identity review should parse");
            assert!(matches!(
                birth_date_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewBirthDateLeague {
                    nhl_player_id: 8_484_115,
                    canonical_birth_date,
                    reviewer,
                    json: true,
                    ..
                }) if canonical_birth_date == "1999-04-17"
                    && reviewer == "league-date-pilot"
            ));

            let collision_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-collision-league",
                "--league-crosswalk",
                "league-conflict-reviewed.json",
                "--proposed-nhl-player-id",
                "8475366",
                "--canonical-nhl-player-id",
                "8484302",
                "--canonical-name",
                "Matt Brown",
                "--canonical-birth-date",
                "1999-08-09",
                "--evidence-url",
                "https://api-web.nhle.com/v1/player/8484302/landing",
                "--reviewer",
                "league-collision-pilot",
                "--reviewed-at",
                "2026-07-26T21:10:00Z",
                "--note",
                "official landing record identifies the younger same-name player",
                "--decisions-out",
                "league-collision-decisions.json",
                "--json",
                "--out",
                "league-collision-reviewed.json",
            ])
            .expect("collision league affiliate identity review should parse");
            assert!(matches!(
                collision_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewCollisionLeague {
                    proposed_nhl_player_id: 8_475_366,
                    canonical_nhl_player_id: 8_484_302,
                    reviewer,
                    json: true,
                    ..
                }) if reviewer == "league-collision-pilot"
            ));

            let reject = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-reject",
                "--crosswalk",
                "alias-reviewed.json",
                "--provider-player-id",
                "11069",
                "--evidence-url",
                "https://example.test/stalletti",
                "--reviewer",
                "exception-pilot",
                "--reviewed-at",
                "2026-07-25T14:00:00Z",
                "--note",
                "official team evidence identifies non-player personnel",
                "--decisions-out",
                "reject-decisions.json",
                "--json",
                "--out",
                "rejection-reviewed.json",
            ])
            .expect("affiliate identity rejection review should parse");
            assert!(matches!(
                reject.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewReject {
                    provider_player_id,
                    evidence_urls,
                    reviewer,
                    json: true,
                    ..
                }) if provider_player_id == ["11069"]
                    && evidence_urls == ["https://example.test/stalletti"]
                    && reviewer == "exception-pilot"
            ));

            let reject_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-reject-league",
                "--league-crosswalk",
                "ahl-league.json",
                "--provider-player-id",
                "11069",
                "--provider-player-id",
                "10646",
                "--evidence-url",
                "https://example.test/ahl-exceptions",
                "--reviewer",
                "league-exception-pilot",
                "--reviewed-at",
                "2026-07-28T23:00:00Z",
                "--note",
                "AHL facts retained; no unique canonical NHL mapping",
                "--decisions-out",
                "league-reject-decisions.json",
                "--json",
                "--out",
                "league-rejection-reviewed.json",
            ])
            .expect("league affiliate identity rejection review should parse");
            assert!(matches!(
                reject_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewRejectLeague {
                    provider_player_id,
                    reviewer,
                    json: true,
                    ..
                }) if provider_player_id == ["11069", "10646"]
                    && reviewer == "league-exception-pilot"
            ));

            let config = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-rollover-config-league",
                "--league-crosswalk",
                "league-identities.json",
                "--camp-forecast",
                "league-camp.json",
                "--prior-affiliations",
                "ahl-affiliations-2025-26.json",
                "--affiliations",
                "ahl-affiliations.json",
                "--as-of",
                "2026-07-28",
                "--source-url",
                "https://theahl.com/nhl-affiliations",
                "--out",
                "league-rollover-config.json",
            ])
            .expect("league affiliate rollover config command should parse");
            assert!(matches!(
                config.command,
                Commands::Icecast(IceCastSubcommand::AffiliateRolloverConfigLeague { .. })
            ));

            let league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-league",
                "--crosswalk",
                "hartford-2025.json",
                "--crosswalk",
                "coachella-2025.json",
                "--json",
                "--out",
                "league-review.json",
            ])
            .expect("affiliate league identity review should parse");
            assert!(matches!(
                league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewLeague {
                    crosswalks,
                    league_crosswalks,
                    json: true,
                    ..
                }) if crosswalks.len() == 2 && league_crosswalks.is_empty()
            ));

            let league_envelopes = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-league",
                "--league-crosswalk",
                "ahl-2024.json",
                "--league-crosswalk",
                "ahl-2025.json",
                "--json",
            ])
            .expect("affiliate league envelopes should parse");
            assert!(matches!(
                league_envelopes.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewLeague {
                    crosswalks,
                    league_crosswalks,
                    json: true,
                    ..
                }) if crosswalks.is_empty() && league_crosswalks.len() == 2
            ));

            let board = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-board",
                "--review",
                "league-review.json",
                "--json",
                "--out",
                "league-exception-board.json",
            ])
            .expect("affiliate identity exception board should parse");
            assert!(matches!(
                board.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewBoard {
                    review,
                    json: true,
                    ..
                }) if review == PathBuf::from("league-review.json")
            ));

            let show = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-show",
                "--crosswalk",
                "review.json",
                "--attention-only",
                "--json",
                "--out",
                "review-attention.json",
            ])
            .expect("affiliate identity review inspection should parse");
            assert!(matches!(
                show.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewShow {
                    attention_only: true,
                    json: true,
                    ..
                })
            ));

            let apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-review-apply",
                "--crosswalk",
                "review.json",
                "--decisions",
                "decisions.json",
                "--json",
                "--out",
                "reviewed.json",
            ])
            .expect("affiliate identity review application should parse");
            assert!(matches!(
                apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReviewApply { json: true, .. })
            ));

            let status_draft = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-status-draft",
                "--prior-snapshot",
                "prior-ahl.json",
                "--crosswalk",
                "reviewed.json",
                "--camp",
                "camp.json",
                "--nhl-team",
                "NYR",
                "--ahl-team",
                "Hartford Wolf Pack",
                "--out",
                "status-draft.json",
            ])
            .expect("affiliate organization-status review draft should parse");
            assert!(matches!(
                status_draft.command,
                Commands::Icecast(IceCastSubcommand::AffiliateStatusDraft { .. })
            ));

            let status_draft_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-status-draft-league",
                "--prior-snapshot",
                "prior-ahl.json",
                "--league-crosswalk",
                "league-reviewed.json",
                "--camp-forecast",
                "league-camp.json",
                "--config",
                "league-rollover-config.json",
                "--json",
                "--out",
                "league-status-draft.json",
            ])
            .expect("league organization-status review draft should parse");
            assert!(matches!(
                status_draft_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateStatusDraftLeague { json: true, .. })
            ));

            let status_evidence = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-status-evidence",
                "--review",
                "league-status-draft.json",
                "--career-history",
                "career.json",
                "--as-of",
                "2026-07-28T12:00:00Z",
                "--maximum-fact-age-days",
                "21",
                "--json",
            ])
            .expect("official organization-status evidence should parse");
            assert!(matches!(
                status_evidence.command,
                Commands::Icecast(IceCastSubcommand::AffiliateStatusEvidence {
                    maximum_fact_age_days: 21,
                    json: true,
                    ..
                })
            ));

            let status_evidence_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-status-evidence-apply",
                "--review",
                "league-status-draft.json",
                "--ledger",
                "status-evidence.json",
                "--json",
            ])
            .expect("organization-status evidence application should parse");
            assert!(matches!(
                status_evidence_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateStatusEvidenceApply {
                    json: true,
                    ..
                })
            ));

            let transaction_state = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-transaction-state",
                "--transactions",
                "transactions.json",
                "--league-crosswalk",
                "league-reviewed.json",
                "--affiliations",
                "affiliations.json",
                "--cutoff",
                "2026-07-28",
                "--json",
                "--out",
                "transaction-state.json",
            ])
            .expect("affiliate transaction-state ledger should parse");
            assert!(matches!(
                transaction_state.command,
                Commands::Icecast(IceCastSubcommand::AffiliateTransactionState { json: true, .. })
            ));

            let transaction_state_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-transaction-state-apply",
                "--workboard",
                "workboard.json",
                "--ledger",
                "transaction-state.json",
                "--json",
                "--out",
                "transaction-state-application.json",
            ])
            .expect("affiliate transaction-state application should parse");
            assert!(matches!(
                transaction_state_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateTransactionStateApply {
                    json: true,
                    ..
                })
            ));

            let waivers_draft = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-waivers-draft",
                "--workboard",
                "workboard.json",
                "--cutoff",
                "2026-09-30",
                "--json",
            ])
            .expect("affiliate waiver draft should parse");
            assert!(matches!(
                waivers_draft.command,
                Commands::Icecast(IceCastSubcommand::AffiliateWaiversDraft { json: true, .. })
            ));

            let waivers_finalize = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-waivers-finalize",
                "--draft",
                "waiver-draft.json",
                "--decisions",
                "waiver-decisions.json",
                "--json",
            ])
            .expect("affiliate waiver finalization should parse");
            assert!(matches!(
                waivers_finalize.command,
                Commands::Icecast(IceCastSubcommand::AffiliateWaiversFinalize { json: true, .. })
            ));

            let waivers_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-waivers-apply",
                "--workboard",
                "workboard.json",
                "--review",
                "waiver-review.json",
                "--json",
            ])
            .expect("affiliate waiver application should parse");
            assert!(matches!(
                waivers_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateWaiversApply { json: true, .. })
            ));

            let status_show = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-status-show",
                "--review",
                "status-review.json",
                "--json",
                "--out",
                "status-review-copy.json",
            ])
            .expect("affiliate organization-status review inspection should parse");
            assert!(matches!(
                status_show.command,
                Commands::Icecast(IceCastSubcommand::AffiliateStatusShow { json: true, .. })
            ));

            let status_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-status-apply",
                "--prior-snapshot",
                "prior-ahl.json",
                "--crosswalk",
                "reviewed.json",
                "--camp",
                "camp.json",
                "--review",
                "status-review.json",
                "--config",
                "rollover-base.json",
                "--out",
                "rollover-config.json",
            ])
            .expect("affiliate organization-status review application should parse");
            assert!(matches!(
                status_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateStatusApply { .. })
            ));

            let status_apply_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-status-apply-league",
                "--prior-snapshot",
                "prior-ahl.json",
                "--league-crosswalk",
                "league-reviewed.json",
                "--camp-forecast",
                "league-camp.json",
                "--review",
                "league-status-review.json",
                "--config",
                "league-rollover-base.json",
                "--out",
                "league-rollover-reviewed.json",
            ])
            .expect("league organization-status review application should parse");
            assert!(matches!(
                status_apply_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateStatusApplyLeague { .. })
            ));

            let professional_games = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-professional-games",
                "--league-crosswalk",
                "league-reviewed.json",
                "--career-history",
                "career-history.json",
                "--policy",
                "professional-game-policy.json",
                "--camp-forecast",
                "league-camp.json",
                "--json",
                "--out",
                "professional-game-ledger.json",
            ])
            .expect("affiliate professional-game ledger should parse");
            assert!(matches!(
                professional_games.command,
                Commands::Icecast(IceCastSubcommand::AffiliateProfessionalGames {
                    camp_forecast: Some(_),
                    json: true,
                    ..
                })
            ));

            let values = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-values",
                "--snapshot",
                "ahl.json",
                "--league-crosswalk",
                "league-reviewed.json",
                "--policy",
                "ahl-value-policy.json",
                "--json",
                "--out",
                "ahl-values.json",
            ])
            .expect("affiliate player-value ledger should parse");
            assert!(matches!(
                values.command,
                Commands::Icecast(IceCastSubcommand::AffiliateValues { json: true, .. })
            ));

            let values_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-values-apply",
                "--workboard",
                "affiliate-facts-board.json",
                "--ledger",
                "ahl-values.json",
                "--json",
                "--out",
                "affiliate-values-application.json",
            ])
            .expect("affiliate player-value application should parse");
            assert!(matches!(
                values_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateValuesApply { json: true, .. })
            ));

            let cross_league_values = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-values-cross-league",
                "--workboard",
                "affiliate-values-application.json",
                "--career-history",
                "career-history.json",
                "--policy",
                "cross-league-policy.json",
                "--json",
                "--out",
                "cross-league-values.json",
            ])
            .expect("affiliate cross-league value ledger should parse");
            assert!(matches!(
                cross_league_values.command,
                Commands::Icecast(IceCastSubcommand::AffiliateValuesCrossLeague { json: true, .. })
            ));

            let cross_league_values_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-values-cross-league-apply",
                "--workboard",
                "affiliate-values-application.json",
                "--ledger",
                "cross-league-values.json",
                "--json",
                "--out",
                "cross-league-application.json",
            ])
            .expect("affiliate cross-league value application should parse");
            assert!(matches!(
                cross_league_values_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateValuesCrossLeagueApply {
                    json: true,
                    ..
                })
            ));

            let prospects = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-prospects",
                "--workboard",
                "affiliate-values-application.json",
                "--career-history",
                "career-history.json",
                "--policy",
                "organizational-prospect-policy.json",
                "--json",
                "--out",
                "ahl-prospects.json",
            ])
            .expect("affiliate prospect-status ledger should parse");
            assert!(matches!(
                prospects.command,
                Commands::Icecast(IceCastSubcommand::AffiliateProspects { json: true, .. })
            ));

            let prospects_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-prospects-apply",
                "--workboard",
                "affiliate-values-application.json",
                "--ledger",
                "ahl-prospects.json",
                "--json",
                "--out",
                "affiliate-prospects-application.json",
            ])
            .expect("affiliate prospect-status application should parse");
            assert!(matches!(
                prospects_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateProspectsApply { json: true, .. })
            ));

            let readiness = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-readiness",
                "--workboard",
                "affiliate-prospects-application.json",
                "--career-history",
                "career-history.json",
                "--camp-forecast",
                "camp.json",
                "--policy",
                "readiness-policy.json",
                "--json",
                "--out",
                "readiness.json",
            ])
            .expect("affiliate recall-readiness ledger should parse");
            assert!(matches!(
                readiness.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReadiness { json: true, .. })
            ));

            let readiness_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-readiness-apply",
                "--workboard",
                "affiliate-prospects-application.json",
                "--ledger",
                "readiness.json",
                "--json",
                "--out",
                "affiliate-readiness-application.json",
            ])
            .expect("affiliate recall-readiness application should parse");
            assert!(matches!(
                readiness_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateReadinessApply { json: true, .. })
            ));

            let facts_board = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-facts-board",
                "--rollover",
                "league-rollover.json",
                "--professional-games",
                "professional-game-ledger.json",
                "--json",
                "--out",
                "affiliate-facts-board.json",
            ])
            .expect("affiliate preseason facts workboard should parse");
            assert!(matches!(
                facts_board.command,
                Commands::Icecast(IceCastSubcommand::AffiliateFactsBoard { json: true, .. })
            ));

            let facts_status = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-facts-status",
                "--input",
                "affiliate-readiness-application.json",
                "--require-ready",
                "--json",
            ])
            .expect("affiliate preseason facts status should parse");
            assert!(matches!(
                facts_status.command,
                Commands::Icecast(IceCastSubcommand::AffiliateFactsStatus {
                    require_ready: true,
                    json: true,
                    ..
                })
            ));

            let facts_draft = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-facts-draft",
                "--workboard",
                "affiliate-facts-board.json",
                "--out",
                "affiliate-facts-overlay-draft.json",
            ])
            .expect("affiliate preseason facts draft should parse");
            assert!(matches!(
                facts_draft.command,
                Commands::Icecast(IceCastSubcommand::AffiliateFactsDraft { .. })
            ));

            let facts_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-facts-apply",
                "--workboard",
                "affiliate-facts-board.json",
                "--overlay",
                "affiliate-facts-overlay.json",
                "--json",
                "--out",
                "affiliate-facts-application.json",
            ])
            .expect("affiliate preseason facts application should parse");
            assert!(matches!(
                facts_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateFactsApply { json: true, .. })
            ));

            let inputs_league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-inputs-league",
                "--application",
                "affiliate-facts-application.json",
                "--rule",
                "ahl-development-rule.json",
                "--json",
                "--out",
                "affiliate-inputs-league.json",
            ])
            .expect("affiliate preseason league inputs should parse");
            assert!(matches!(
                inputs_league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateInputsLeague { json: true, .. })
            ));

            let professional_games_apply = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-professional-games-apply",
                "--crosswalk",
                "hartford-reviewed.json",
                "--ledger",
                "professional-game-ledger.json",
                "--facts",
                "hartford-facts.json",
                "--nhl-team",
                "NYR",
                "--ahl-team",
                "Hartford Wolf Pack",
                "--out",
                "hartford-final-facts.json",
            ])
            .expect("affiliate professional-game facts application should parse");
            assert!(matches!(
                professional_games_apply.command,
                Commands::Icecast(IceCastSubcommand::AffiliateProfessionalGamesApply { .. })
            ));

            let input = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-input",
                "--snapshot",
                "ahl.json",
                "--crosswalk",
                "reviewed.json",
                "--facts",
                "facts.json",
                "--nhl-team",
                "NYR",
                "--ahl-team",
                "Hartford Wolf Pack",
                "--out",
                "affiliate-input.json",
            ])
            .expect("reviewed affiliate input command should parse");
            assert!(matches!(
                input.command,
                Commands::Icecast(IceCastSubcommand::AffiliateInput { .. })
            ));
        });
    }

    #[test]
    fn l0_icecast_affiliate_rollover_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-rollover",
                "--prior-snapshot",
                "prior-ahl.json",
                "--crosswalk",
                "prior-identities.json",
                "--camp",
                "camp.json",
                "--camp-forecast",
                "camp-forecast.json",
                "--config",
                "rollover-config.json",
                "--json",
                "--out",
                "rollover.json",
            ])
            .expect("affiliate rollover command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::AffiliateRollover { json: true, .. })
            ));

            let league = Cli::try_parse_from([
                "icelines",
                "icecast",
                "affiliate-rollover-league",
                "--prior-snapshot",
                "prior-ahl.json",
                "--league-crosswalk",
                "league-identities.json",
                "--camp-forecast",
                "league-camp.json",
                "--config",
                "league-rollover-config.json",
                "--json",
                "--out",
                "league-rollover.json",
            ])
            .expect("league affiliate rollover command should parse");
            assert!(matches!(
                league.command,
                Commands::Icecast(IceCastSubcommand::AffiliateRolloverLeague { json: true, .. })
            ));
        });
    }

    #[test]
    fn l0_team_card_report_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "report",
                "team-card",
                "--team",
                "NYR",
                "--scenario-id",
                "nyr-development-variance",
                "--scenario-comparison-key",
                "development-variance",
                "--trials",
                "25000",
                "--seed",
                "73",
                "--generated-at",
                "2026-07-22T12:00:00Z",
                "--json",
            ])
            .expect("team card report should parse");
            match cli.command {
                Commands::Report(ReportSubcommand::TeamCard {
                    team,
                    scenario_id,
                    scenario_comparison_key,
                    trials,
                    seed,
                    generated_at,
                    json,
                    ..
                }) => {
                    assert_eq!(team, "NYR");
                    assert_eq!(scenario_id, "nyr-development-variance");
                    assert_eq!(
                        scenario_comparison_key.as_deref(),
                        Some("development-variance")
                    );
                    assert_eq!(trials, 25_000);
                    assert_eq!(seed, 73);
                    assert_eq!(generated_at.as_deref(), Some("2026-07-22T12:00:00Z"));
                    assert!(json);
                }
                other => panic!("expected team card report, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_development_calibration_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "calibrate-development",
                "--start-season",
                "20152016",
                "--end-season",
                "20252026",
                "--breakout-threshold",
                "2.5",
                "--downturn-threshold",
                "-2.5",
                "--prior-sample-size",
                "30",
                "--json",
            ])
            .expect("IceCast development calibration command should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::CalibrateDevelopment {
                    start_season,
                    end_season,
                    breakout_threshold,
                    downturn_threshold,
                    prior_sample_size,
                    json,
                    ..
                }) => {
                    assert_eq!(start_season, 20152016);
                    assert_eq!(end_season, 20252026);
                    assert_eq!(breakout_threshold, 2.5);
                    assert_eq!(downturn_threshold, -2.5);
                    assert_eq!(prior_sample_size, 30.0);
                    assert!(json);
                }
                other => panic!("expected development calibration command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_prospect_study_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-study",
                "--input",
                "firkus.json",
                "--json",
                "--out",
                "firkus-study.json",
            ])
            .expect("IceCast prospect study command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectStudy {
                    input,
                    json: true,
                    out: Some(out),
                }) if input == PathBuf::from("firkus.json") && out == PathBuf::from("firkus-study.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_board_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-board",
                "--study",
                "firkus-study.json",
                "--study",
                "another-study.json",
                "--json",
                "--out",
                "prospect-board.json",
            ])
            .expect("IceCast prospect board command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectBoard {
                    studies,
                    json: true,
                    out: Some(out),
                }) if studies == vec![PathBuf::from("firkus-study.json"), PathBuf::from("another-study.json")]
                    && out == PathBuf::from("prospect-board.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_league_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-league",
                "--snapshot",
                "ahl-2024.json",
                "--snapshot",
                "ahl-2025.json",
                "--crosswalk",
                "reviewed-2024.json",
                "--crosswalk",
                "reviewed-2025.json",
                "--context",
                "prospects.json",
                "--json",
                "--out",
                "league-discovery.json",
            ])
            .expect("IceCast prospect league command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectLeague {
                    snapshots,
                    crosswalks,
                    context,
                    json: true,
                    out: Some(out),
                }) if snapshots.len() == 2
                    && crosswalks.len() == 2
                    && context == PathBuf::from("prospects.json")
                    && out == PathBuf::from("league-discovery.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_context_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-context",
                "--snapshot",
                "ahl-2024.json",
                "--snapshot",
                "ahl-2025.json",
                "--league-crosswalk",
                "league-2024.json",
                "--league-crosswalk",
                "league-2025.json",
                "--affiliations",
                "affiliations.json",
                "--as-of",
                "2026-09-15",
                "--max-age",
                "24",
                "--minimum-ahl-seasons",
                "2",
                "--json",
                "--out",
                "prospect-context.json",
            ])
            .expect("IceCast prospect context command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectContext {
                    snapshots,
                    league_crosswalks,
                    affiliations,
                    as_of,
                    max_age: 24,
                    minimum_ahl_seasons: 2,
                    json: true,
                    out: Some(out),
                }) if snapshots.len() == 2
                    && league_crosswalks.len() == 2
                    && affiliations == PathBuf::from("affiliations.json")
                    && as_of == "2026-09-15"
                    && out == PathBuf::from("prospect-context.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_program_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-program",
                "--league-discovery",
                "league-discovery.json",
                "--career-discovery",
                "career-discovery.json",
                "--study",
                "college-study.json",
                "--prior-board",
                "prior-program-board.json",
                "--json",
                "--out",
                "program-board.json",
            ])
            .expect("IceCast prospect program command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectProgram {
                    league_discoveries,
                    career_discoveries,
                    studies,
                    prior_board: Some(prior),
                    maximum_nhl_games: 50,
                    json: true,
                    out: Some(out),
                }) if league_discoveries == vec![PathBuf::from("league-discovery.json")]
                    && career_discoveries == vec![PathBuf::from("career-discovery.json")]
                    && studies == vec![PathBuf::from("college-study.json")]
                    && prior == PathBuf::from("prior-program-board.json")
                    && out == PathBuf::from("program-board.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_program_sensitivity_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-program-sensitivity",
                "--league-discovery",
                "league.json",
                "--career-discovery",
                "career.json",
                "--thresholds",
                "25,50,82",
                "--json",
                "--out",
                "sensitivity.json",
            ])
            .expect("IceCast prospect program sensitivity command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectProgramSensitivity {
                    league_discoveries,
                    career_discoveries,
                    studies,
                    thresholds,
                    json: true,
                    out: Some(out),
                }) if league_discoveries == vec![PathBuf::from("league.json")]
                    && career_discoveries == vec![PathBuf::from("career.json")]
                    && studies.is_empty()
                    && thresholds == vec![25, 50, 82]
                    && out == PathBuf::from("sensitivity.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_program_history_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-program-history",
                "--board",
                "2024.json",
                "--board",
                "2025.json",
                "--json",
                "--out",
                "history.json",
            ])
            .expect("IceCast prospect program history command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectProgramHistory {
                    boards,
                    json: true,
                    out: Some(out),
                }) if boards == vec![PathBuf::from("2024.json"), PathBuf::from("2025.json")]
                    && out == PathBuf::from("history.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_conversion_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-conversion",
                "--league-discovery",
                "frozen-league.json",
                "--career-history",
                "career_history.json",
                "--baseline-season",
                "20222023",
                "--through-season",
                "20252026",
                "--performance",
                "performance.json",
                "--performance-out",
                "derived-performance.json",
                "--json",
                "--out",
                "conversion.json",
            ])
            .expect("IceCast prospect conversion command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectConversion {
                    league_discoveries,
                    career_discoveries,
                    studies,
                    career_history,
                    baseline_season: 20222023,
                    through_season: 20252026,
                    performance: Some(performance),
                    performance_out: Some(performance_out),
                    json: true,
                    out: Some(out),
                }) if league_discoveries == vec![PathBuf::from("frozen-league.json")]
                    && career_discoveries.is_empty()
                    && studies.is_empty()
                    && career_history == PathBuf::from("career_history.json")
                    && performance == PathBuf::from("performance.json")
                    && performance_out == PathBuf::from("derived-performance.json")
                    && out == PathBuf::from("conversion.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_career_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-career",
                "--context",
                "prospect-context.json",
                "--career-history",
                "career_history.json",
                "--json",
                "--out",
                "career-discovery.json",
            ])
            .expect("IceCast prospect career command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectCareer {
                    context,
                    career_history,
                    json: true,
                    out: Some(out),
                }) if context == PathBuf::from("prospect-context.json")
                    && career_history == PathBuf::from("career_history.json")
                    && out == PathBuf::from("career-discovery.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_prospect_career_context_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "prospect-career-context",
                "--camp-forecast",
                "camp.json",
                "--rosters",
                "rosters.json",
                "--bios",
                "bios.json",
                "--career-history",
                "career.json",
                "--json",
                "--out",
                "context.json",
            ])
            .expect("IceCast prospect career context command should parse");
            assert!(matches!(
                cli.command,
                Commands::Icecast(IceCastSubcommand::ProspectCareerContext {
                    camp_forecast,
                    rosters,
                    bios,
                    candidate_overlay: None,
                    career_history: Some(career_history),
                    json: true,
                    out: Some(out),
                    ..
                }) if camp_forecast == PathBuf::from("camp.json")
                    && rosters == PathBuf::from("rosters.json")
                    && bios == PathBuf::from("bios.json")
                    && career_history == PathBuf::from("career.json")
                    && out == PathBuf::from("context.json")
            ));
        });
    }

    #[test]
    fn l0_fetch_career_camp_target_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fetch",
                "career",
                "--camp-forecast",
                "camp.json",
                "--dry-run",
            ])
            .expect("fetch career camp target should parse");
            assert!(matches!(
                cli.command,
                Commands::Fetch(FetchSubcommand::Career {
                    dry_run: true,
                    bundled_seasons: 0,
                    prospect_context: None,
                    camp_forecast: Some(path),
                    league_crosswalk: None,
                    affiliate_workboard: None,
                }) if path == PathBuf::from("camp.json")
            ));

            let league = Cli::try_parse_from([
                "icelines",
                "fetch",
                "career",
                "--league-crosswalk",
                "ahl-identities.json",
                "--dry-run",
            ])
            .expect("fetch career league crosswalk target should parse");
            assert!(matches!(
                league.command,
                Commands::Fetch(FetchSubcommand::Career {
                    dry_run: true,
                    bundled_seasons: 0,
                    prospect_context: None,
                    camp_forecast: None,
                    league_crosswalk: Some(path),
                    affiliate_workboard: None,
                }) if path == PathBuf::from("ahl-identities.json")
            ));

            let workboard = Cli::try_parse_from([
                "icelines",
                "fetch",
                "career",
                "--affiliate-workboard",
                "affiliate-prospects-application.json",
                "--dry-run",
            ])
            .expect("fetch career affiliate workboard target should parse");
            assert!(matches!(
                workboard.command,
                Commands::Fetch(FetchSubcommand::Career {
                    dry_run: true,
                    bundled_seasons: 0,
                    prospect_context: None,
                    camp_forecast: None,
                    league_crosswalk: None,
                    affiliate_workboard: Some(path),
                }) if path == PathBuf::from("affiliate-prospects-application.json")
            ));
        });
    }

    #[test]
    fn l0_fetch_ahl_transactions_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fetch",
                "ahl-transactions",
                "--season",
                "20252026",
                "--refresh",
                "--out",
                "ahl-transactions.json",
            ])
            .expect("AHL transaction acquisition should parse");
            assert!(matches!(
                cli.command,
                Commands::Fetch(FetchSubcommand::AhlTransactions {
                    season,
                    out: Some(path),
                    refresh: true,
                    dry_run: false,
                }) if season == "20252026" && path == PathBuf::from("ahl-transactions.json")
            ));
        });
    }

    #[test]
    fn l0_icecast_opening_roster_archive_import_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "import-opening-rosters",
                "--manifest",
                "opening-rosters.json",
                "--dry-run",
                "--allow-partial-evaluation",
            ])
            .expect("IceCast opening-roster archive import should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::ImportOpeningRosters {
                    manifest,
                    dry_run,
                    allow_partial_evaluation,
                }) => {
                    assert_eq!(manifest, PathBuf::from("opening-rosters.json"));
                    assert!(dry_run);
                    assert!(allow_partial_evaluation);
                }
                other => panic!("expected IceCast archive import command, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_icecast_opening_roster_discovery_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "icecast",
                "discover-opening-rosters",
                "--season",
                "20242025",
                "--out",
                "coverage.json",
                "--manifest-out",
                "import.json",
                "--partial-manifest-out",
                "partial.json",
                "--cache-only",
            ])
            .expect("IceCast opening-roster discovery should parse");
            match cli.command {
                Commands::Icecast(IceCastSubcommand::DiscoverOpeningRosters {
                    season,
                    out,
                    manifest_out,
                    partial_manifest_out,
                    cache_only,
                }) => {
                    assert_eq!(season, 20242025);
                    assert_eq!(out, Some(PathBuf::from("coverage.json")));
                    assert_eq!(manifest_out, Some(PathBuf::from("import.json")));
                    assert_eq!(partial_manifest_out, Some(PathBuf::from("partial.json")));
                    assert!(cache_only);
                }
                other => panic!("expected IceCast archive discovery command, got {other:?}"),
            }
        });
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
    fn l0_fantasy_roster_card_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "roster-card",
                "--date",
                "2026-10-05",
                "--league",
                "Ice League",
                "--classes",
                "6",
                "--json",
            ])
            .expect("fantasy roster-card should parse");

            match cli.command {
                Commands::Fantasy(FantasySubcommand::RosterCard {
                    date,
                    league,
                    classes,
                    json,
                    ..
                }) => {
                    assert_eq!(date.as_deref(), Some("2026-10-05"));
                    assert_eq!(league.as_deref(), Some("Ice League"));
                    assert_eq!(classes, 6);
                    assert!(json);
                }
                other => panic!("expected fantasy roster-card, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_fantasy_morning_card_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "morning-card",
                "--date",
                "2026-10-08",
                "--at",
                "2026-10-08T14:00:00Z",
                "--league",
                "Ice League",
                "--current-goalie-appearances",
                "2.0",
                "--json",
            ])
            .expect("fantasy morning-card should parse");

            match cli.command {
                Commands::Fantasy(FantasySubcommand::MorningCard {
                    date,
                    at,
                    league,
                    current_goalie_appearances,
                    json,
                    ..
                }) => {
                    assert_eq!(date.as_deref(), Some("2026-10-08"));
                    assert_eq!(at.as_deref(), Some("2026-10-08T14:00:00Z"));
                    assert_eq!(league.as_deref(), Some("Ice League"));
                    assert_eq!(current_goalie_appearances, 2.0);
                    assert!(json);
                }
                other => panic!("expected fantasy morning-card, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_fantasy_trade_card_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "trade-card",
                "Adam Fox",
                "--to-team",
                "Blue Line Bandits",
                "--for-player",
                "Mikko Rantanen",
                "--json",
            ])
            .expect("fantasy trade-card should parse");

            match cli.command {
                Commands::Fantasy(FantasySubcommand::TradeCard {
                    player1,
                    to_team,
                    for_player,
                    json,
                    ..
                }) => {
                    assert_eq!(player1, "Adam Fox");
                    assert_eq!(to_team, "Blue Line Bandits");
                    assert_eq!(for_player, "Mikko Rantanen");
                    assert!(json);
                }
                other => panic!("expected fantasy trade-card, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_fantasy_draft_card_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "draft-card",
                "--taken-file",
                "taken.txt",
                "--pick",
                "Nathan MacKinnon",
                "--top",
                "8",
                "--json",
            ])
            .expect("fantasy draft-card should parse");

            match cli.command {
                Commands::Fantasy(FantasySubcommand::DraftCard {
                    taken_file,
                    pick,
                    top,
                    json,
                    ..
                }) => {
                    assert_eq!(
                        taken_file.as_deref(),
                        Some(std::path::Path::new("taken.txt"))
                    );
                    assert_eq!(pick.as_deref(), Some("Nathan MacKinnon"));
                    assert_eq!(top, 8);
                    assert!(json);
                }
                other => panic!("expected fantasy draft-card, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_fantasy_matchup_clap_surface_parses() {
        with_large_stack(|| {
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
        });
    }

    #[test]
    fn l0_fantasy_matchup_set_clap_surface_parses() {
        with_large_stack(|| {
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
        });
    }

    #[test]
    fn l0_fantasy_matchup_plan_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "matchup-plan",
                "--week",
                "2026-10-08",
                "--opponent",
                "Rival",
                "--team",
                "My Team",
                "--strategy",
                "floor",
                "--through",
                "2026-10-07",
                "--user-current",
                "42.5",
                "--opponent-current",
                "39.0",
                "--candidates",
                "40",
                "--json",
            ])
            .expect("fantasy matchup-plan should parse");
            assert!(matches!(
                cli.command,
                Commands::Fantasy(FantasySubcommand::MatchupPlan {
                    week,
                    team: Some(team),
                    opponent: Some(opponent),
                    strategy,
                    through: Some(through),
                    user_current: Some(user_current),
                    opponent_current: Some(opponent_current),
                    candidates: 40,
                    json: true,
                    ..
                }) if week.to_string() == "2026-10-08"
                    && team == "My Team"
                    && opponent == "Rival"
                    && strategy == "floor"
                    && through.to_string() == "2026-10-07"
                    && user_current == 42.5
                    && opponent_current == 39.0
            ));
        });
    }

    #[test]
    fn l0_fantasy_playoff_portfolio_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "playoff-portfolio",
                "--rounds",
                "3",
                "--start",
                "2027-03-15",
                "--team",
                "Dexter's Dawgs",
                "--season",
                "20262027",
                "--json",
            ])
            .expect("fantasy playoff-portfolio should parse");
            assert!(matches!(
                cli.command,
                Commands::Fantasy(FantasySubcommand::PlayoffPortfolio {
                    rounds: Some(3),
                    start: Some(start),
                    team: Some(team),
                    season: 20262027,
                    candidates: 25,
                    top: 10,
                    json: true,
                    ..
                }) if team == "Dexter's Dawgs" && start.to_string() == "2027-03-15"
            ));
        });
    }

    #[test]
    fn l0_fantasy_playoff_calendar_set_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "playoff-calendar-set",
                "--start",
                "2027-03-15",
                "--rounds",
                "3",
                "--league",
                "My League",
                "--json",
            ])
            .expect("fantasy playoff-calendar-set should parse");
            assert!(matches!(
                cli.command,
                Commands::Fantasy(FantasySubcommand::PlayoffCalendarSet {
                    start,
                    rounds: 3,
                    league: Some(league),
                    json: true,
                }) if start.to_string() == "2027-03-15" && league == "My League"
            ));
        });
    }

    #[test]
    fn l0_fantasy_competition_set_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "competition-set",
                "--mode",
                "categories",
                "--category",
                "goals:higher:sum",
                "--category",
                "save_percentage:higher:ratio:0.0001",
                "--minimum-goalie-appearances",
                "3",
            ])
            .expect("fantasy competition-set should parse");
            assert!(matches!(
                cli.command,
                Commands::Fantasy(FantasySubcommand::CompetitionSet {
                    mode,
                    categories,
                    minimum_goalie_appearances: 3,
                    ..
                }) if mode == "categories" && categories.len() == 2
            ));
        });
    }

    #[test]
    fn l0_fantasy_matchup_plan_category_snapshot_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "matchup-plan",
                "--week",
                "2026-10-08",
                "--opponent",
                "Rival",
                "--category-snapshot",
                "-",
            ])
            .expect("category snapshot should parse");
            assert!(matches!(
                cli.command,
                Commands::Fantasy(FantasySubcommand::MatchupPlan {
                    category_snapshot: Some(path),
                    ..
                }) if path.as_os_str() == "-"
            ));
        });
    }

    #[test]
    fn l0_fantasy_import_yahoo_clap_surface_parses() {
        with_large_stack(|| {
            let cli = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "import-yahoo",
                "--file",
                "-",
                "--league",
                "Office Pool",
                "--my-team",
                "My Team",
                "--dry-run",
                "--replace",
                "--json",
            ])
            .expect("fantasy import-yahoo should parse");

            match cli.command {
                Commands::Fantasy(FantasySubcommand::ImportYahoo {
                    file,
                    league,
                    my_team,
                    dry_run,
                    replace,
                    json,
                }) => {
                    assert_eq!(file, std::path::PathBuf::from("-"));
                    assert_eq!(league, "Office Pool");
                    assert_eq!(my_team.as_deref(), Some("My Team"));
                    assert!(dry_run);
                    assert!(replace);
                    assert!(json);
                }
                other => panic!("expected fantasy import-yahoo, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_fantasy_trade_readiness_and_complete_gate_parse() {
        with_large_stack(|| {
            let readiness = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "trade-readiness",
                "--league",
                "Office Pool",
                "--team",
                "My Team",
                "--stats-season",
                "20252026",
                "--json",
            ])
            .expect("fantasy trade-readiness should parse");
            assert!(matches!(
                readiness.command,
                Commands::Fantasy(FantasySubcommand::TradeReadiness {
                    team: Some(team),
                    json: true,
                    ..
                }) if team == "My Team"
            ));

            let finder = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "trade-finder",
                "--team",
                "My Team",
                "--require-complete",
            ])
            .expect("fantasy trade-finder complete gate should parse");
            assert!(matches!(
                finder.command,
                Commands::Fantasy(FantasySubcommand::TradeFinder {
                    require_complete: true,
                    ..
                })
            ));

            let history = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "trade-history",
                "--league",
                "Office Pool",
                "--limit",
                "25",
                "--json",
            ])
            .expect("fantasy trade-history should parse");
            assert!(matches!(
                history.command,
                Commands::Fantasy(FantasySubcommand::TradeHistory {
                    limit: 25,
                    json: true,
                    ..
                })
            ));

            let offers = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "trade-offers",
                "--status",
                "pending",
                "--limit",
                "25",
                "--actionable-only",
            ])
            .expect("fantasy trade-offers should parse");
            assert!(matches!(
                offers.command,
                Commands::Fantasy(FantasySubcommand::TradeOffers {
                    status: Some(status),
                    limit: 25,
                    actionable_only: true,
                    ..
                }) if status == "pending"
            ));

            let save = Cli::try_parse_from([
                "icelines",
                "fantasy",
                "trade",
                "Bouchard",
                "--to-team",
                "Rival",
                "--for-player",
                "Werenski",
                "--save-offer",
            ])
            .expect("fantasy trade --save-offer should parse");
            assert!(matches!(
                save.command,
                Commands::Fantasy(FantasySubcommand::Trade {
                    save_offer: true,
                    execute: false,
                    ..
                })
            ));
            assert!(Cli::try_parse_from([
                "icelines",
                "fantasy",
                "trade",
                "Bouchard",
                "--to-team",
                "Rival",
                "--for-player",
                "Werenski",
                "--save-offer",
                "--execute",
            ])
            .is_err());
        });
    }

    #[test]
    fn l0_fantasy_roster_shape_clap_surfaces_parse() {
        with_large_stack(|| {
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
                Commands::Fantasy(FantasySubcommand::RosterShapeValidate {
                    league,
                    team,
                    json,
                }) => {
                    assert_eq!(league.as_deref(), Some("Office Pool"));
                    assert_eq!(team.as_deref(), Some("My Team"));
                    assert!(json);
                }
                other => panic!("expected fantasy roster-shape-validate, got {other:?}"),
            }
        });
    }

    #[test]
    fn l0_fantasy_goalie_evidence_and_plan_commands_parse() {
        with_large_stack(|| {
            for args in [
                vec![
                    "icelines",
                    "fantasy",
                    "goalie-start-record",
                    "Igor Shesterkin",
                    "--date",
                    "2026-11-12",
                    "--state",
                    "confirmed-starting",
                    "--source",
                    "team reporter",
                ],
                vec![
                    "icelines",
                    "fantasy",
                    "goalie-start-show",
                    "--week",
                    "2026-11-09",
                    "--max-age-minutes",
                    "180",
                ],
                vec![
                    "icelines",
                    "fantasy",
                    "goalie-start-import",
                    "--file",
                    "-",
                    "--source",
                    "daily goalie report",
                ],
                vec![
                    "icelines",
                    "fantasy",
                    "goalie-start-template",
                    "--date",
                    "2026-11-12",
                    "--top-streams",
                    "5",
                    "--out",
                    "-",
                ],
                vec![
                    "icelines",
                    "fantasy",
                    "goalie-plan",
                    "--date",
                    "2026-11-12",
                    "--strategy",
                    "floor",
                    "--current-appearances",
                    "2",
                    "--json",
                ],
            ] {
                Cli::try_parse_from(args).expect("fantasy goalie command should parse");
            }
        });
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

/// Contract data provider. Third-party providers are always opt-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ContractSource {
    /// NHL player landing API (currently carries no salary values).
    Nhl,
    /// Licensed CapWages API; requires CAPWAGES_API_KEY.
    CapWages,
    /// Validated local CSV overlay with per-row provenance.
    Csv,
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

    /// Project current-roster market cost and cap pressure across future seasons.
    CapForecast {
        /// First forecast season, e.g. 20262027.
        #[arg(long, default_value = "20262027")]
        season: String,

        /// Number of seasons to project (1-10).
        #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u8).range(1..=10))]
        years: u8,

        /// Annual cap growth after announced limits, as a percentage.
        #[arg(long, default_value_t = 5.0, allow_hyphen_values = true)]
        growth_pct: f64,

        /// Limit output to one current NHL roster.
        #[arg(long)]
        team: Option<String>,

        /// Emit cap_projection.v1 JSON instead of the text report.
        #[arg(long)]
        json: bool,

        /// Write report to path. Pass '-' or omit for stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Compare 2026-27 current-roster ceiling with the prior-season roster.
    TeamCeiling {
        /// Current roster authority season.
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        roster_season: String,

        /// Completed production season used to rate players and form the baseline.
        #[arg(long, default_value = "20252026")]
        stats_season: String,

        /// Limit rendered output to one team while preserving league normalization.
        #[arg(long)]
        team: Option<String>,

        /// Emit team_ceiling.v1 JSON instead of text.
        #[arg(long)]
        json: bool,

        /// Write report to path. Pass '-' or omit for stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Project one team's four lines, three pairs, goalies, extras, faces, and IceLines scores.
    TeamLineup {
        /// Current roster authority season.
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        roster_season: String,

        /// Completed production season used by the player score.
        #[arg(long, default_value = "20252026")]
        stats_season: String,

        /// Current NHL roster to project.
        #[arg(long)]
        team: String,

        /// Emit team_lineup_projection.v1 JSON instead of text.
        #[arg(long)]
        json: bool,

        /// Write report to path. Pass '-' or omit for stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },

    /// Build the two-page UI-neutral team prognosis card from IceLines sources.
    TeamCard {
        /// Current roster authority season.
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        roster_season: String,

        /// Completed production season used by player scores.
        #[arg(long, default_value = "20252026")]
        stats_season: String,

        /// NHL team abbreviation.
        #[arg(long)]
        team: String,

        /// Stable scenario ID from the local IceCast registry.
        #[arg(long)]
        scenario_id: String,

        /// Explicit cross-team scenario dimension for safe side-by-side deltas.
        #[arg(long)]
        scenario_comparison_key: Option<String>,

        /// Paired simulation trials.
        #[arg(long, default_value_t = 10_000)]
        trials: u32,

        /// Deterministic simulation seed.
        #[arg(long, default_value_t = 20_262_027)]
        seed: u64,

        /// Fixed RFC 3339 generation/evidence timestamp for reproducible documents.
        #[arg(long)]
        generated_at: Option<String>,

        /// Emit the complete card_document.v1 JSON instead of compact text.
        #[arg(long)]
        json: bool,

        /// Write report to path. Pass '-' or omit for stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
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
    /// Fetch contract data from the NHL API or an opt-in licensed provider.
    Contracts {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Contract value season; defaults to --season.
        #[arg(long, value_name = "YYYYYYYY")]
        valuation_season: Option<String>,
        #[arg(long, value_enum, default_value_t = ContractSource::Nhl)]
        source: ContractSource,
        /// Local contract overlay; required with `--source csv`.
        #[arg(long, value_name = "PATH")]
        input: Option<PathBuf>,
        /// Salary-cap upper limit in dollars, used for team cap-share output.
        #[arg(long)]
        cap_limit: Option<u64>,
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
        /// Restrict acquisition to player IDs in a `prospect_league_context.v1` file.
        #[arg(long = "prospect-context", value_name = "PATH")]
        prospect_context: Option<PathBuf>,
        /// Restrict acquisition to prospects in a `training_camp_league_forecast.v1` file.
        #[arg(long = "camp-forecast", value_name = "PATH")]
        camp_forecast: Option<PathBuf>,
        /// Restrict acquisition to canonical players in a reviewed all-league AHL crosswalk.
        #[arg(long = "league-crosswalk", value_name = "PATH")]
        league_crosswalk: Option<PathBuf>,
        /// Restrict acquisition to canonical candidates in an AHL preseason workboard or application.
        #[arg(long = "affiliate-workboard", value_name = "PATH")]
        affiliate_workboard: Option<PathBuf>,
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
    /// Fetch official AHL team rosters plus skater and goalie season stats.
    /// Provider player IDs remain explicitly separate from NHL player IDs.
    Ahl {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Restrict to an AHL team code or exact name. Repeat for multiple teams.
        /// With no filter, fetches every team in the provider season catalog.
        #[arg(long = "team", value_name = "AHL_CODE_OR_NAME")]
        teams: Vec<String>,
        /// Also export the UI-neutral snapshot to this path. The canonical
        /// copy is always sealed in the IceLines snapshot store.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Revalidate source bytes instead of accepting a verified FLETCH cacheline.
        #[arg(long)]
        refresh: bool,
        /// Print the planned fetch without making network requests or writing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch the complete official AHL ADD/DEL transaction stream for a season.
    #[command(name = "ahl-transactions")]
    AhlTransactions {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        /// Also export the UI-neutral transaction snapshot to this path.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Revalidate source pages instead of accepting verified FLETCH cachelines.
        #[arg(long)]
        refresh: bool,
        /// Print the planned fetch without network requests or writes.
        #[arg(long)]
        dry_run: bool,
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
        /// Write JSON/CSV output to this path instead of stdout.
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
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
    /// Change the scoring scheme for an existing fantasy league.
    LeagueSchemeSet {
        scheme: String,
        #[arg(long)]
        league: Option<String>,
    },
    /// Show the active league's exact points/category competition contract.
    #[command(name = "competition-show")]
    CompetitionShow {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Set points mode or exact category rules for a league.
    #[command(name = "competition-set")]
    CompetitionSet {
        /// points or categories.
        #[arg(long)]
        mode: String,
        /// KEY:DIRECTION:AGGREGATION[:TIE_EPSILON], repeat for every category.
        #[arg(long = "category")]
        categories: Vec<String>,
        #[arg(long, default_value_t = 0)]
        minimum_goalie_appearances: u8,
        /// tie or higher_seed_wins.
        #[arg(long, default_value = "tie")]
        tie_policy: String,
        #[arg(long)]
        league: Option<String>,
    },

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
        #[arg(long)]
        stats_season: Option<String>,
    },
    /// Add a player to a fantasy team.
    TeamAdd {
        team: String,
        player: String,
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        stats_season: Option<String>,
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

    /// Stress-test a complete fantasy season with injuries, pickups, and trades.
    #[command(name = "season-sim")]
    SeasonSim {
        #[arg(long)]
        league: Option<String>,
        /// Fantasy team whose current partial or complete roster is locked into team one.
        #[arg(long)]
        team: Option<String>,
        #[arg(long, default_value_t = 12)]
        teams: usize,
        #[arg(long, default_value_t = 6)]
        playoff_teams: usize,
        #[arg(long, default_value_t = 100)]
        trials: usize,
        #[arg(long, default_value_t = 20_262_027)]
        seed: u64,
        /// Per-player probability of a new injury on each scheduled game day.
        #[arg(long, default_value_t = 0.0015)]
        injury_rate: f64,
        /// Probability that each fantasy team attempts a trade each week.
        #[arg(long, default_value_t = 0.10)]
        trade_probability: f64,
        /// Chance an opponent selects the top projected weekly pickup; team one always does.
        #[arg(long, default_value_t = 1.0)]
        opponent_pickup_accuracy: f64,
        /// Team-one acquisitions held back through Friday for injury replacements.
        #[arg(long, default_value_t = 1)]
        pickup_reserve: u8,
        /// Minimum simulated net value required to spend the protected final move.
        #[arg(long, default_value_t = 6.0)]
        exceptional_reserve_min_value: f64,
        /// Minimum seven-day game-volume gain required to spend the protected move.
        #[arg(long, default_value_t = 3)]
        exceptional_reserve_min_games: i8,
        /// Disable the exceptional-value escape hatch and keep a strict reserve.
        #[arg(long)]
        strict_pickup_reserve: bool,
        /// Compare clean, baseline, and high-chaos environments with the same seed.
        #[arg(long)]
        scenario_matrix: bool,
        /// Compare 100%, 85%, and 70% opponent pickup accuracy with the same seed.
        #[arg(long)]
        manager_matrix: bool,
        /// Compare all-in, strict-reserve, and adaptive-reserve acquisition policies.
        #[arg(long)]
        reserve_matrix: bool,
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long)]
        json: bool,
    },

    /// Find weekly game volume, quiet-slate teams, and schedule-diverse draft fits.
    #[command(name = "schedule-edge")]
    ScheduleEdge {
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        /// Show only the Monday-Sunday week containing this date.
        #[arg(long)]
        week: Option<chrono::NaiveDate>,
        /// Explicit NHL teams to analyze as a roster (overrides the active fantasy roster).
        #[arg(long = "teams", value_delimiter = ',')]
        teams: Vec<String>,
        /// Fantasy league used to resolve the marked user roster.
        #[arg(long)]
        league: Option<String>,
        /// A game is an off-night opportunity when the NHL slate has at most this many games.
        #[arg(long, default_value_t = 4)]
        off_night_max_games: usize,
        /// Number of exact-date schedule equivalence classes.
        #[arg(long, default_value_t = 8)]
        classes: usize,
        /// Ignore the locally cached schedule and reload all teams from the official NHL API.
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },

    /// Rank the marked roster's legal usable starts across the final playoff weeks.
    #[command(name = "playoff-portfolio")]
    PlayoffPortfolio {
        /// Number of final Monday-Sunday fantasy playoff rounds to inspect.
        #[arg(long)]
        rounds: Option<usize>,
        /// Monday starting the first fantasy playoff round; defaults to the final season weeks.
        #[arg(long)]
        start: Option<chrono::NaiveDate>,
        /// Team perspective; otherwise use the marked user team.
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        /// A quiet-slate start occurs on a date with at most this many NHL games.
        #[arg(long, default_value_t = 4)]
        off_night_max_games: usize,
        /// Maximum unrostered completed-season value leaders evaluated as add candidates.
        #[arg(long, default_value_t = 25)]
        candidates: usize,
        /// Maximum candidate add/drop fits returned.
        #[arg(long, default_value_t = 10)]
        top: usize,
        #[arg(long)]
        json: bool,
    },

    /// Persist the league's first fantasy playoff Monday and number of rounds.
    #[command(name = "playoff-calendar-set")]
    PlayoffCalendarSet {
        /// Monday starting the first fantasy playoff round.
        #[arg(long)]
        start: chrono::NaiveDate,
        #[arg(long, default_value_t = 3)]
        rounds: u8,
        #[arg(long)]
        league: Option<String>,
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

    /// Project a points-mode weekly matchup using legal daily lineups.
    #[command(name = "matchup-plan")]
    MatchupPlan {
        /// Date inside the Monday-Sunday matchup week.
        #[arg(long)]
        week: chrono::NaiveDate,
        /// Team perspective; otherwise use the marked user team.
        #[arg(long)]
        team: Option<String>,
        /// Opponent override; otherwise use the saved matchup schedule.
        #[arg(long)]
        opponent: Option<String>,
        /// Risk posture: floor, balanced, or upside.
        #[arg(long, default_value = "balanced")]
        strategy: String,
        /// Whether your team owns the higher-seed tiebreak (required by that policy).
        #[arg(long)]
        user_higher_seed: Option<bool>,
        /// JSON category totals snapshot, or - to read pasted JSON from stdin.
        #[arg(long)]
        category_snapshot: Option<PathBuf>,
        /// Last matchup date already included in both current point totals.
        #[arg(long)]
        through: Option<chrono::NaiveDate>,
        /// Your platform matchup points through --through.
        #[arg(long, requires = "through")]
        user_current: Option<f64>,
        /// Opponent platform matchup points through --through.
        #[arg(long, requires = "through")]
        opponent_current: Option<f64>,
        /// Human-readable authority for supplied current totals.
        #[arg(long, default_value = "manual platform entry")]
        current_source: String,
        /// Maximum age of saved status evidence before it becomes advisory only.
        #[arg(long, default_value_t = 360)]
        status_max_age_minutes: i64,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        /// Maximum free agents considered for the one-move swing.
        #[arg(long, default_value_t = 75)]
        candidates: usize,
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
        /// Yahoo roster CSV export to parse, or `-` to read pasted CSV from stdin.
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
        /// Replace each included team's saved roster instead of only adding rows.
        #[arg(long)]
        replace: bool,
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

    /// Verify that saved rosters are complete enough for actionable trade search.
    #[command(name = "trade-readiness")]
    TradeReadiness {
        #[arg(long)]
        league: Option<String>,
        /// Limit the report to one team.
        #[arg(long)]
        team: Option<String>,
        /// Completed season used to resolve canonical player positions.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long)]
        json: bool,
    },

    /// Persist the configured 2026 draft/daily assistant roster and transaction rules.
    AssistantSetup {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Show the active league's persisted draft/daily assistant rules.
    AssistantRules {
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Rank the best available players for the next draft pick.
    DraftBoard {
        /// Newline or CSV list of drafted players; use `-` to read stdin.
        #[arg(long)]
        taken_file: Option<PathBuf>,
        /// Yahoo/player-pool CSV containing player names and eligible positions.
        #[arg(long)]
        eligibility_file: Option<PathBuf>,
        /// Preview the recommendation after hypothetically drafting this player.
        #[arg(long)]
        pick: Option<String>,
        #[arg(long)]
        league: Option<String>,
        /// Completed stats season evaluated under the active league scheme.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 15)]
        top: usize,
        #[arg(long)]
        json: bool,
    },

    /// Build the sealed two-page fantasy draft card from the live draft board.
    #[command(name = "draft-card")]
    DraftCard {
        /// Newline or CSV list of drafted players; use `-` to read stdin.
        #[arg(long)]
        taken_file: Option<PathBuf>,
        /// Yahoo/player-pool CSV containing player names and eligible positions.
        #[arg(long)]
        eligibility_file: Option<PathBuf>,
        /// Preview the recommendation after hypothetically drafting this player.
        #[arg(long)]
        pick: Option<String>,
        #[arg(long)]
        league: Option<String>,
        /// Completed stats season evaluated under the active league scheme.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 15)]
        top: usize,
        #[arg(long)]
        json: bool,
    },

    /// Show the Monday-Sunday acquisition budget for the selected league.
    WeeklyBudget {
        #[arg(long)]
        league: Option<String>,
        /// RFC3339 evaluation time; defaults to now.
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Rank legal add/drop moves for the remaining Monday-Sunday fantasy week.
    WeeklyPickups {
        /// Evaluation date in YYYY-MM-DD; defaults to today in league timezone.
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 50)]
        candidates: usize,
        #[arg(long, default_value_t = 20)]
        top: usize,
        #[arg(long)]
        json: bool,
    },

    /// Find unrostered skaters with rising league-scored and category rates.
    Sleepers {
        #[arg(long)]
        league: Option<String>,
        /// Evaluation season, normally the latest completed or in-progress sample.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        /// Prior season used as the player-rate baseline.
        #[arg(long, default_value = "20242025")]
        baseline_season: String,
        /// Restrict to one or more positions: C, LW, RW, D.
        #[arg(long, value_delimiter = ',')]
        positions: Vec<String>,
        #[arg(long, default_value_t = 20)]
        top: usize,
        #[arg(long)]
        json: bool,
    },

    /// Record a completed local fantasy add and optional drop.
    AcquisitionRecord {
        #[arg(long)]
        add: String,
        #[arg(long)]
        drop: Option<String>,
        #[arg(long, default_value = "free-agent")]
        kind: String,
        /// RFC3339 effective time; defaults to now.
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        league: Option<String>,
        /// Store the event without consuming the weekly limit.
        #[arg(long)]
        no_count: bool,
        #[arg(long)]
        json: bool,
    },

    /// Record a sourced fantasy availability observation for a player.
    StatusRecord {
        player: String,
        #[arg(long)]
        status: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        source_url: Option<String>,
        #[arg(long)]
        observed_at: Option<String>,
        #[arg(long, default_value = "reported")]
        confidence: String,
        #[arg(long)]
        detail: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Show latest evidence-aware availability statuses.
    StatusShow {
        player: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = 360)]
        max_age_minutes: i64,
        #[arg(long)]
        json: bool,
    },

    /// Record sourced starter evidence for one goalie and NHL game date.
    GoalieStartRecord {
        player: String,
        #[arg(long)]
        date: String,
        #[arg(long)]
        state: String,
        #[arg(long)]
        source: String,
        #[arg(long)]
        source_url: Option<String>,
        #[arg(long)]
        observed_at: Option<String>,
        #[arg(long)]
        detail: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Atomically import same-day goalie starter evidence from CSV or stdin.
    GoalieStartImport {
        /// CSV path, or - for clipboard/stdin input.
        #[arg(long, default_value = "-")]
        file: PathBuf,
        /// Fallback source applied when a CSV row has no source.
        #[arg(long)]
        source: Option<String>,
        /// Fallback RFC3339 observation time applied when a row omits observed_at.
        #[arg(long)]
        observed_at: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Export today's rostered-goalie and stream-candidate evidence checklist CSV.
    GoalieStartTemplate {
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 5)]
        top_streams: usize,
        /// Output CSV path; omit or use - for stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Show latest game-specific goalie starter evidence and freshness.
    GoalieStartShow {
        player: Option<String>,
        #[arg(long)]
        week: Option<String>,
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = 360)]
        max_age_minutes: i64,
        #[arg(long)]
        json: bool,
    },

    /// Build a weekly goalie evidence, slot-capacity, and minimum-appearance plan.
    GoaliePlan {
        #[arg(long)]
        week: Option<String>,
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value = "balanced")]
        strategy: String,
        #[arg(long, default_value_t = 0.0)]
        current_appearances: f64,
        #[arg(long, default_value_t = 360)]
        max_age_minutes: i64,
        #[arg(long)]
        json: bool,
    },

    /// Build an evidence-aware lineup and IR/IR+ placement plan.
    InjuryPlan {
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 360)]
        max_age_minutes: i64,
        #[arg(long)]
        json: bool,
    },

    /// Build the sealed two-page fantasy roster card from the legal daily lineup.
    #[command(name = "roster-card")]
    RosterCard {
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = icelines_core::CURRENT_SEASON)]
        season: u32,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 360)]
        max_age_minutes: i64,
        #[arg(long, default_value_t = 4)]
        off_night_max_games: usize,
        #[arg(long, default_value_t = 8)]
        classes: usize,
        #[arg(long)]
        json: bool,
    },

    /// Build the evidence-aware 07:00 lineup, IR, and pickup briefing.
    Morning {
        #[arg(long)]
        date: Option<String>,
        /// RFC3339 pregame evaluation time; omitted means 07:00 local baseline.
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 360)]
        max_age_minutes: i64,
        /// Goalie appearances already recorded in the current Monday-Sunday matchup.
        #[arg(long, default_value_t = 0.0)]
        current_goalie_appearances: f64,
        /// Suppress text/actions when the decision-bearing fingerprint is unchanged.
        #[arg(long)]
        material_only: bool,
        #[arg(long)]
        json: bool,
    },

    /// Build the sealed two-page Morning Skate card from today's briefing.
    #[command(name = "morning-card")]
    MorningCard {
        #[arg(long)]
        date: Option<String>,
        /// RFC3339 pregame evaluation time; omitted means 07:00 local baseline.
        #[arg(long)]
        at: Option<String>,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long, default_value_t = 360)]
        max_age_minutes: i64,
        /// Goalie appearances already recorded in the current Monday-Sunday matchup.
        #[arg(long, default_value_t = 0.0)]
        current_goalie_appearances: f64,
        #[arg(long)]
        json: bool,
    },

    // Trade
    /// Evaluate or atomically execute a player trade between two teams.
    Trade {
        /// Player or comma-separated package your side sends.
        player1: String,
        #[arg(long)]
        to_team: String,
        #[arg(long = "for-player")]
        /// Player or comma-separated package the other side sends.
        for_player: String,
        #[arg(long)]
        execute: bool,
        /// Save this legal evaluation as a pending offer without changing rosters.
        #[arg(long, conflicts_with = "execute")]
        save_offer: bool,
        #[arg(long)]
        league: Option<String>,
        /// Completed production season used to value the players.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long)]
        json: bool,
    },

    /// Evaluate a trade and seal the result as a two-page UI-neutral card.
    #[command(name = "trade-card")]
    TradeCard {
        /// Player or comma-separated package your side sends.
        player1: String,
        #[arg(long)]
        to_team: String,
        #[arg(long = "for-player")]
        /// Player or comma-separated package the other side sends.
        for_player: String,
        #[arg(long)]
        league: Option<String>,
        /// Completed production season used to value the players.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long)]
        json: bool,
    },

    /// Show locally executed fantasy trades, newest first.
    #[command(name = "trade-history")]
    TradeHistory {
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },

    /// List saved trade offers, newest first.
    #[command(name = "trade-offers")]
    TradeOffers {
        /// Filter by pending, accepted, rejected, cancelled, or expired.
        #[arg(long)]
        status: Option<String>,
        /// Hide offers whose saved players no longer belong to the expected teams.
        #[arg(long)]
        actionable_only: bool,
        #[arg(long)]
        league: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },

    /// Close a pending saved trade offer without changing rosters.
    #[command(name = "trade-offer-close")]
    TradeOfferClose {
        id: String,
        /// accepted, rejected, cancelled, or expired.
        #[arg(long)]
        status: String,
        #[arg(long)]
        league: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Search opponent rosters for fair, legal trade offers.
    #[command(name = "trade-finder")]
    TradeFinder {
        /// Team to improve; defaults to the marked user team.
        #[arg(long)]
        team: Option<String>,
        /// Search only this opponent instead of the entire league.
        #[arg(long)]
        to_team: Option<String>,
        /// Largest package on either side (one or two players).
        #[arg(long, default_value_t = 2)]
        max_package: usize,
        /// Maximum projected value gap for a mutually plausible offer.
        #[arg(long, default_value_t = 10.0)]
        fairness_percent: f64,
        /// Player names that must not appear in outgoing packages.
        #[arg(long, value_delimiter = ',')]
        protect: Vec<String>,
        /// Allow the roster's highest-value player to appear in offers.
        #[arg(long)]
        include_anchors: bool,
        /// Refuse to rank offers unless every searched roster is complete and legal.
        #[arg(long)]
        require_complete: bool,
        #[arg(long, default_value_t = 20)]
        top: usize,
        #[arg(long)]
        league: Option<String>,
        /// Completed production season used to value the players.
        #[arg(long, default_value = "20252026")]
        stats_season: String,
        #[arg(long)]
        json: bool,
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
