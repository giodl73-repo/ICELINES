use clap::{Parser, Subcommand};

// ── Top-level CLI ─────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "icelines",
    about = "NHL fantasy depth-chart and ranking tool",
    version,
    propagate_version = true
)]
pub struct Cli {
    /// Disable all live NHL API fetches (Phase 8f.1). Suppresses the
    /// Scores / Schedule / Playoffs / boxscore endpoints and the Scores
    /// auto-refresh timer. Useful for airplane mode, demos, rate-limit
    /// avoidance, and deterministic CI runs. Also settable via
    /// `ICELINES_NO_LIVE=1` or `live = false` in `~/.icelines/config.toml`.
    /// Precedence: CLI flag > env var > config file > default (live ON).
    #[arg(long, global = true)]
    pub no_live: bool,

    /// Disable the dashboard side panel on the TUI player card. The panel
    /// is on by default; pass this flag to suppress it. Also settable via
    /// `ICELINES_DASHBOARDS=0` or `dashboards = false` in
    /// `~/.icelines/config.toml`. Precedence: CLI flag > env > config >
    /// default (on).
    #[arg(long, global = true)]
    pub no_dashboards: bool,

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
    },

    /// Manage named data snapshots.
    #[command(subcommand)]
    Snapshot(SnapshotSubcommand),

    // ── Phase 2 implemented ───────────────────────────────────────────────────
    /// Generate site markdown from cached snapshot data.
    Build {
        /// Generate markdown only, do not run mkdocs.
        #[arg(long)]
        no_site: bool,
    },

    /// Serve the static site locally (builds first, then launches mkdocs serve).
    Serve {
        #[arg(long, default_value_t = 8000)] port: u16,
    },
    /// Deploy the site to GitHub Pages.
    Deploy {
        #[arg(long, default_value = "origin")] remote: String,
    },
    /// Show tonight's NHL games.
    Tonight {
        /// Filter to games involving this team.
        #[arg(long)] team: Option<String>,
    },
    /// Show upcoming schedule.
    Schedule {
        #[arg(long)] team: Option<String>,
        #[arg(long, default_value_t = 7)] days: u32,
    },
    /// Evaluate a trade — depth chart before/after.
    Trade {
        /// Player leaving (partial name OK).
        player_out: String,
        /// Literal "for".
        #[arg(value_name = "for")] _for: String,
        /// Player arriving.
        player_in: String,
        /// Team perspective [default: player_out's team].
        #[arg(long)] team: Option<String>,
    },
    /// Project rest-of-season performance.
    Project {
        /// Player name (partial match OK). Omit to use --team.
        player: Option<String>,
        /// Project all skaters on a team.
        #[arg(long)] team: Option<String>,
        /// Projection mode: pace | regressed | composite [default: regressed]
        #[arg(long, default_value = "regressed")] mode: String,
        /// Override remaining games (default: auto from schedule).
        #[arg(long)] games: Option<u32>,
    },
    /// Launch the interactive TUI.
    Tui,
    /// Export analytical output as markdown tables (Phase 8d).
    /// Bridges to proof's DASHBOARD-SPEC compiler — see
    /// `design/specs/export-markdown.md`.
    #[command(subcommand)]
    Export(ExportSubcommand),
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
    },
    /// Find statistical peers for a player (same draft era and position).
    Peers {
        player: String,
        #[arg(long, default_value_t = 10)]
        size: usize,
        #[arg(long)]
        json: bool,
    },
    /// Head-to-head player comparison.
    Compare {
        player1: String,
        player2: String,
        #[arg(long)]
        json: bool,
    },
    /// Show a player's historical season stats.
    History {
        player: String,
        /// Number of seasons to show (default: 5).
        #[arg(long, default_value_t = 5)]
        seasons: usize,
        #[arg(long)]
        json: bool,
    },
    /// Find line-mates for a player.
    Mates {
        /// Player name (fuzzy matched).
        player: String,
        /// Number of top linemates to display.
        #[arg(long, default_value_t = 5)]
        top: usize,
    },
    /// Manage player watchlists and custom groups.
    #[command(subcommand)]
    Group(GroupSubcommand),
    /// Track NHL games you attended in person.
    #[command(subcommand)]
    Games(GamesSubcommand),
    /// Full 8-section scouting report for a player.
    Scouting {
        player: String,
        #[arg(long, default_value = "terminal")] format: String,
    },
    /// Manage fantasy scoring schemes.
    #[command(subcommand)]
    Scheme(SchemeSubcommand),
    /// Open the league dashboard (alias for `icelines tui`).
    Dashboard,  // → launches TUI

    /// Download and manage additional season data bundles.
    #[command(subcommand)]
    Data(DataSubcommand),

    /// Advanced query engine — leaderboards, player profiles, similarity search.
    #[command(subcommand)]
    Query(QuerySubcommand),

    /// Fantasy league management — teams, scoring, trades, server.
    #[command(subcommand)]
    Fantasy(FantasySubcommand),
}

// ── Fetch sub-commands ────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum FetchSubcommand {
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
        /// + stats.json files. Daily-cadence storage shrinks ~10–15× via
        /// content-addressed dedup; readers fall back transparently.
        #[arg(long)]
        chunked: bool,
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
    /// Fetch goalie season stats (W/L/SV%/GAA/SO) — Phase G.2.
    /// Writes goalie-stats.json into the active snapshot store.
    Goalies {
        #[arg(long, default_value = icelines_core::CURRENT_SEASON_STR)]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
    },
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
    /// Export the attended-games list to JSON (default stdout).
    Export {
        /// Output path. Use `-` to write to stdout.
        #[arg(long, default_value = "-")]
        out: String,
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
    /// Install season data bundles from GitHub Releases.
    Install {
        /// Install last N seasons [default: 1 = current season refresh].
        #[arg(long, default_value_t = 1)]
        seasons: u8,
        /// Install a specific season (e.g. 20212022).
        #[arg(long)]
        season: Option<String>,
        /// Re-download even if the season is already installed.
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
        /// Query a specific historical season (e.g. 20242025). Conflicts with
        /// --seasons N. Must match a bundled season — see icelines-fetch::BUNDLED_SEASONS.
        #[arg(long)]
        season: Option<String>,
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
    },

    /// Deep dive on a single player — career arc, league rank, percentiles.
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
    },

    /// Side-by-side comparison or similarity search.
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
        /// Team abbrev for `team` (e.g. `SEA`).
        #[arg(long)]
        team: Option<String>,
        /// Top-N for `leaders`.
        #[arg(long, default_value_t = 25)]
        top: usize,
        /// Sort metric for `leaders`. Defaults to `pts-pace`.
        #[arg(long, default_value = "pts-pace")]
        sort: String,
        /// Min-GP filter for `leaders` (default = current MIN_GP).
        #[arg(long)]
        gp_min: Option<u32>,
        /// First player for `compare`.
        #[arg(long)]
        p1: Option<String>,
        /// Second player for `compare`.
        #[arg(long)]
        p2: Option<String>,
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
    /// Cross-team line-value rankings.
    Depth,
    /// Active fantasy league standings.
    Fantasy,
    /// Two-player head-to-head.
    Compare,
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
            MdShape::Team    => "team",
            MdShape::Depth   => "depth",
            MdShape::Fantasy => "fantasy",
            MdShape::Compare => "compare",
            MdShape::Series  => "series",
            MdShape::Roster  => "roster",
        }
    }
}
