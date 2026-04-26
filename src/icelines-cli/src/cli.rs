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

    // ── Phase 3 stubs ────────────────────────────────────────────────────────
    /// Serve the static site locally.
    Serve,
    /// Deploy the site.
    Deploy,
    /// Show tonight's schedule and projected scoring.
    Tonight,
    /// Show the full season schedule.
    Schedule,
    /// Evaluate a trade offer.
    Trade,
    /// Project player performance for the rest of the season.
    Project,
    /// Launch the interactive TUI.
    Tui,
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
        #[arg(long)]
        json: bool,
    },
    /// Find line-mates for a player.
    Mates,
    /// Manage player watchlists and custom groups.
    #[command(subcommand)]
    Group(GroupSubcommand),
    /// Run scouting report for a player or team.
    Scouting,
    /// Manage fantasy scoring schemes.
    #[command(subcommand)]
    Scheme(SchemeSubcommand),
    /// Open the metrics dashboard.
    Dashboard,
}

// ── Fetch sub-commands ────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub enum FetchSubcommand {
    /// Fetch all 32 team rosters (headshots, positions, bios).
    Rosters {
        #[arg(long, default_value = "20252026")]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch skater season stats (G, A, GP, TOI).
    Stats {
        #[arg(long, default_value = "20252026")]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Fetch rosters then stats in one pass.
    All {
        #[arg(long, default_value = "20252026")]
        season: String,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Refresh position eligibility from boxscore data (Phase 2).
    Positions {
        #[arg(long, default_value = "20252026")]
        season: String,
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
}

#[derive(Debug, Subcommand)]
pub enum SchemeSubcommand {
    /// List all available schemes (built-in + user-defined).
    List,
    /// Show scoring weights for a named scheme.
    Show { name: String },
    /// Detect scoreable stats from a Yahoo CSV and create a scheme template.
    FromCsv {
        path: String,
        #[arg(long)]
        name: Option<String>,
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
}
