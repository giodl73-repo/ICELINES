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

    // ── Phase 2 / 3 stubs ────────────────────────────────────────────────────
    /// Build static site output.
    Build,
    /// Serve the static site locally.
    Serve,
    /// Deploy the site.
    Deploy,
    /// Compare two players head-to-head.
    Compare,
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
    /// List all available players.
    Players,
    /// Classify players into tiers.
    Class,
    /// Find statistical peers for a player.
    Peers,
    /// Show a player's historical stats.
    History,
    /// Find line-mates for a player.
    Mates,
    /// Group players into a custom set for comparison.
    Group,
    /// Run scouting report for a player or team.
    Scouting,
    /// Manage color / display schemes.
    Scheme,
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
    Positions,
}
