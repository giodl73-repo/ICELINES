// Phase Jack Adams.2 — chat-CLI command bar parser + executor.
//
// The MDI dashboard's bottom row is a command bar. Users type
// commands like `query country=CAN`, `box edm@bos`, `player
// Bedard`, `/fav add Bedard`, `/help`. This module defines the
// command grammar, parses input strings into structured
// `Command` values, and executes those commands by mutating
// App state (workspace screen, favorites DB, filter editor
// state).
//
// Per spec forge-5: STRICT verb-or-slash prefix in v1. No
// bare-input disambiguation. `country=CAN` is a parse error;
// users type `query country=CAN`.
//
// Per spec forge-4: structured `ParseError` enum, not a string
// blob. Lets the cmdbar UI render targeted error messages.

#![allow(clippy::module_name_repetitions)]
// Adams.2 ships parser + ParseError + Command + execute fns
// gradually — some variants land in the first commit (parser
// only) before execute_command wires them all. Allow at the
// module level until Adams.2 is fully wired.
#![allow(dead_code)]

use crate::tui::mdi::SidePane;
use icelines_core::{
    model::Position,
    view_model::{PoachAvailabilityFilter, PoachCandidateKind, WatchRuleMutationIntent},
    CANONICAL_TEAMS,
};

// ── Command grammar ──────────────────────────────────────────────────────────

/// Phase Jack Adams.2 — every action the chat-CLI command bar
/// can produce. Returned by `parse_command`; consumed by
/// `execute_command`.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    // ── Meta (slash commands) ────────────────────────────────
    /// `/help` — open the help overlay.
    Help,
    /// `/quit` — exit the TUI. Also bound to bare `q`.
    Quit,
    /// `/hide favorites` or `/hide schedule` — hide a side pane.
    Hide(SidePane),
    /// `/show favorites` or `/show schedule` — show a side pane.
    Show(SidePane),
    /// `admin` — open the operational admin overlay.
    Admin,

    // ── Workspace-swap reads (no args) ────────────────────────
    /// `stats` — workspace becomes Stats / Queries.
    Stats,
    /// `goalies` — workspace becomes Goalies leaderboard.
    Goalies,
    /// `poach` — workspace becomes The Bench's Waiver Wire.
    Poach,
    PoachKv {
        args: PoachCommandArgs,
    },
    /// `fantasy gaps` — workspace becomes active roster-gap board.
    FantasyGaps,
    FantasyGapsKv {
        args: FantasyGapsCommandArgs,
    },
    /// `fantasy simulate` — workspace becomes league simulation board.
    FantasySim,
    FantasySimKv {
        args: FantasySimulationCommandArgs,
    },
    /// `team-card [NYR|SEA|DEX|DRAFT|MORNING|TRADE]` - open a shared UI-neutral card document.
    TeamCard {
        team: String,
    },
    /// `fantasy daily date=YYYY-MM-DD` — hand off to the daily delta read surfaces.
    FantasyDaily {
        date: String,
    },
    /// `fantasy matchup date=YYYY-MM-DD` — hand off to weekly matchup read surfaces.
    FantasyMatchup {
        date: String,
    },
    /// `fantasy import file=... league=... [dry-run]` - hand off to CLI import.
    FantasyImport {
        file: String,
        league: String,
        my_team: Option<String>,
        dry_run: bool,
    },
    /// `fantasy roster-shape ...` - hand off to CLI/API shape validation/setup.
    FantasyRosterShape {
        action: FantasyRosterShapeAction,
    },
    /// `watchlist` - workspace becomes local fantasy Watchlist.
    Watchlist,
    /// `watch <player>` — command-bar bridge to watch-rule/note
    /// creation surfaces.
    WatchPlayer {
        needle: String,
    },
    /// `watch player <name> when=<trigger>` — persist a player watch rule.
    WatchRulePlayer {
        player: String,
        when: String,
    },
    /// `watch enable|disable <rule-id>` — toggle a persisted watch rule.
    WatchRuleSetEnabled {
        id: String,
        enabled: bool,
    },
    GoaliesKv {
        args: crate::tui::filter_state::RosterKvArgs,
    },
    /// `transactions` (alias `txs`) — workspace becomes
    /// Transactions feed.
    Transactions,
    /// `playoffs` — workspace becomes Playoffs bracket.
    Playoffs,
    /// `depth` — workspace becomes Depth chart.
    Depth,
    DepthKv {
        args: crate::tui::filter_state::RosterKvArgs,
    },
    /// `roster` (alias `fantasy roster`) — workspace becomes
    /// the user's active fantasy roster.
    Roster,
    /// `scores` — workspace becomes today's scores.
    Scores,
    /// `schedule` — workspace becomes the weekly schedule view.
    Schedule,
    /// `transactions` is the canonical name; `txs` is an alias
    /// resolved at parse time. (No separate variant.)
    ///
    /// `favorites` — workspace becomes the favorites screen
    /// (NOT the side pane; the full screen view).
    Favorites,
    FavoritesKv {
        args: crate::tui::filter_state::RosterKvArgs,
    },

    // ── Workspace-swap reads (with args) ──────────────────────
    /// `player <name-or-pid>` — workspace becomes that player's
    /// card.
    PlayerCard {
        needle: String,
    },
    /// `team <abbrev>` — workspace becomes the team's depth
    /// chart.
    Team {
        abbrev: String,
    },
    TeamKv {
        abbrev: String,
        args: crate::tui::filter_state::RosterKvArgs,
    },
    /// `team <abbrev> season` — workspace becomes the team's
    /// full-season schedule.
    TeamSeason {
        abbrev: String,
    },
    /// `compare <left> [right]` — workspace becomes the
    /// comparison view. With one arg, `--similar` peers; with
    /// two, head-to-head.
    Compare {
        left: String,
        right: Option<String>,
    },
    /// `box <game-id-or-team@team>` — workspace becomes the
    /// boxscore detail.
    Box {
        game: String,
    },
    /// `class <year>` — workspace becomes the draft class for
    /// that year.
    Class {
        year: u16,
    },
    /// `career league=OHL season=20142015` — command-bar bridge
    /// to the cross-league CareerView surfaces.
    Career {
        args: CareerCommandArgs,
    },
    /// `report poach` / `report weekly` — command-bar bridge
    /// to shared PoachReportView report surfaces.
    Report {
        args: ReportCommandArgs,
    },
    /// `records player <name>` / `records team <ABBR>` — command-bar
    /// bridge to individual records surfaces.
    Records {
        args: RecordsCommandArgs,
    },
    /// `awards player <name>` — open the cached NHL Trophy Case screen.
    Awards {
        player: String,
    },
    /// `streaks player <name>` — open the cached player streaks screen.
    Streaks {
        player: String,
    },
    /// `scouting player <name>` — show canonical scouting report targets.
    Scouting {
        player: String,
    },
    /// `mates player <name>` — show canonical roster-fallback linemate target.
    Mates {
        player: String,
    },
    /// Operational command handoffs. The TUI does not run these
    /// long-running or destructive commands from the cmdbar.
    Data {
        args: String,
    },
    Snapshot {
        args: String,
    },
    Config {
        args: String,
    },

    // ── Write actions (favorites mutation) ────────────────────
    /// `fav add <name-or-pid>` (or `/fav add ...`) — adds the
    /// player to the Favorites group.
    FavAdd {
        needle: String,
    },
    /// `fav remove <name-or-pid>` — removes from Favorites.
    FavRemove {
        needle: String,
    },

    // ── Free-form query — delegates to Phase Art Ross ─────────
    /// `query <filter>` — sets the Stats screen's free-form
    /// filter to the parsed Phase Art Ross expression. Workspace
    /// swaps to Stats. Mutates `app.queries.filter_text` directly
    /// (per spec edge-2: shared state with the Stats filter
    /// editor).
    Query {
        filter: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PoachCommandArgs {
    pub positions: Option<Vec<Position>>,
    pub categories: Option<Vec<String>>,
    pub availability_filter: Option<PoachAvailabilityFilter>,
    pub candidate_kind: Option<PoachCandidateKind>,
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FantasySimulationCommandArgs {
    pub weeks: Option<u8>,
    pub add_player: Option<String>,
    pub drop_player: Option<String>,
    pub clear: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FantasyGapsCommandArgs {
    pub categories: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FantasyRosterShapeAction {
    Show {
        league: Option<String>,
    },
    Set {
        shape: String,
        league: Option<String>,
    },
    Validate {
        league: Option<String>,
        team: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareerCommandArgs {
    pub league: String,
    pub season: Option<u32>,
    pub top: usize,
    pub sort: String,
}

impl Default for CareerCommandArgs {
    fn default() -> Self {
        Self {
            league: "OHL".to_string(),
            season: None,
            top: 20,
            sort: "points".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCommandArgs {
    pub kind: ReportKind,
    pub categories: Vec<String>,
    pub availability: Option<String>,
    pub top: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportKind {
    Poach,
    Weekly,
}

impl ReportKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Poach => "poach",
            Self::Weekly => "weekly",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordsCommandArgs {
    pub target: RecordsTarget,
    pub subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordsTarget {
    Player,
    Team,
}

// ── Parse error shape ───────────────────────────────────────────────────────

/// Phase Adams.2 — structured parse error. The cmdbar UI
/// renders a targeted message based on the variant (vs. a
/// string blob).
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Empty input or unrecognized command verb.
    UnknownCommand(String),
    /// Verb recognized but a required arg is missing.
    /// e.g., `team` without an abbrev → `MissingArg { command: "team", arg: "abbrev" }`.
    MissingArg {
        command: &'static str,
        arg: &'static str,
    },
    /// `query <filter>` parsed but the filter itself failed
    /// Phase Art Ross's parse_query. `details` is the joined
    /// filter parser error.
    BadFilter { details: String },
    /// e.g., `class abc` — non-integer year.
    BadInteger { command: &'static str, raw: String },
    /// e.g., `/hide foo` — `foo` isn't a known SidePane.
    BadSidePane(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnknownCommand(s) if s.is_empty() => {
                write!(f, "(empty input)")
            }
            ParseError::UnknownCommand(s) => {
                write!(f, "unknown command: {s:?} — try /help")
            }
            ParseError::MissingArg { command, arg } => {
                write!(f, "{command}: missing <{arg}>")
            }
            ParseError::BadFilter { details } => {
                write!(f, "filter parse error: {details}")
            }
            ParseError::BadInteger { command, raw } => {
                write!(f, "{command}: {raw:?} is not a valid integer")
            }
            ParseError::BadSidePane(s) => {
                write!(f, "unknown pane {s:?}; expected `favorites` or `schedule`")
            }
        }
    }
}

impl std::error::Error for ParseError {}

// ── Parser ──────────────────────────────────────────────────────────────────

/// Phase Adams.2 — parse a command-bar input string into a
/// structured Command. Per spec forge-5: strict verb-or-slash
/// prefix. Bare input (`country=CAN`) → UnknownCommand.
pub fn parse_command(input: &str) -> Result<Command, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::UnknownCommand(String::new()));
    }

    if let Some(rest) = trimmed.strip_prefix('/') {
        return parse_slash(rest);
    }

    parse_verb(trimmed)
}

/// Slash commands: `/help`, `/quit`, `/hide <pane>`, `/show <pane>`,
/// `/fav add <player>`, `/fav remove <player>`.
fn parse_slash(rest: &str) -> Result<Command, ParseError> {
    let (verb, args) = split_first_word(rest);
    match verb.to_ascii_lowercase().as_str() {
        "help" | "h" | "?" => Ok(Command::Help),
        "quit" | "q" | "exit" => Ok(Command::Quit),
        "hide" => parse_pane(args).map(Command::Hide),
        "show" => parse_pane(args).map(Command::Show),
        "fav" | "favorite" | "favorites" => parse_fav(args),
        unknown => Err(ParseError::UnknownCommand(format!("/{unknown}"))),
    }
}

/// `/hide favorites` → `SidePane::Favorites`. `/hide foo` →
/// BadSidePane.
fn parse_pane(arg: &str) -> Result<SidePane, ParseError> {
    let trimmed = arg.trim();
    if trimmed.is_empty() {
        return Err(ParseError::MissingArg {
            command: "/hide",
            arg: "pane",
        });
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "fav" | "favs" | "favorite" | "favorites" => Ok(SidePane::Favorites),
        "sched" | "schedule" => Ok(SidePane::Schedule),
        other => Err(ParseError::BadSidePane(other.to_owned())),
    }
}

/// `/fav add Bedard` → `Command::FavAdd { needle }`. Same for
/// remove.
fn parse_fav(args: &str) -> Result<Command, ParseError> {
    let (sub, rest) = split_first_word(args);
    let needle = rest.trim();
    match sub.to_ascii_lowercase().as_str() {
        "add" if needle.is_empty() => Err(ParseError::MissingArg {
            command: "/fav add",
            arg: "player",
        }),
        "add" => Ok(Command::FavAdd {
            needle: needle.to_owned(),
        }),
        "remove" | "rm" | "del" if needle.is_empty() => Err(ParseError::MissingArg {
            command: "/fav remove",
            arg: "player",
        }),
        "remove" | "rm" | "del" => Ok(Command::FavRemove {
            needle: needle.to_owned(),
        }),
        "" => Err(ParseError::MissingArg {
            command: "/fav",
            arg: "subcommand (add | remove)",
        }),
        unknown => Err(ParseError::UnknownCommand(format!("/fav {unknown}"))),
    }
}

/// Verb commands (no slash). `query country=CAN`, `player Bedard`,
/// `goalies`, etc.
fn parse_verb(input: &str) -> Result<Command, ParseError> {
    let (verb, args) = split_first_word(input);
    let v = verb.to_ascii_lowercase();
    match v.as_str() {
        "help" => Ok(Command::Help),
        "quit" | "exit" => Ok(Command::Quit),
        "admin" => Ok(Command::Admin),
        // `q` is shorthand for quit — `query` is the long-form verb
        // (consistent with vim). Filter syntax goes through `query`.
        "q" => Ok(Command::Quit),

        // Workspace-swap reads (no args)
        "stats" if args.trim().is_empty() => Ok(Command::Stats),
        "stats" => parse_stats_kv(args),
        "goalies" | "g" if args.trim().is_empty() => Ok(Command::Goalies),
        "goalies" | "g" => Ok(Command::GoaliesKv {
            args: crate::tui::filter_state::parse_roster_kv(args).map_err(|err| {
                ParseError::BadFilter {
                    details: err.to_string(),
                }
            })?,
        }),
        "poach" if args.trim().is_empty() => Ok(Command::Poach),
        "poach" => Ok(Command::PoachKv {
            args: parse_poach_kv(args)?,
        }),
        "gaps" | "fantasy-gaps" if args.trim().is_empty() => Ok(Command::FantasyGaps),
        "gaps" | "fantasy-gaps" => Ok(Command::FantasyGapsKv {
            args: parse_fantasy_gaps_kv(args)?,
        }),
        "simulate" | "sim" | "fantasy-sim" if args.trim().is_empty() => Ok(Command::FantasySim),
        "simulate" | "sim" | "fantasy-sim" => Ok(Command::FantasySimKv {
            args: parse_fantasy_sim_kv(args)?,
        }),
        "draft-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "DRAFT".to_string(),
        }),
        "morning-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "MORNING".to_string(),
        }),
        "trade-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "TRADE".to_string(),
        }),
        "season-card" | "season-sim-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "SIM-NYR".to_string(),
        }),
        "season-card" | "season-sim-card" => {
            let team = args.trim().to_ascii_uppercase();
            if !matches!(team.as_str(), "NYR" | "SEA") {
                return Err(ParseError::BadFilter {
                    details: format!("season-card supports NYR or SEA, got '{team}'"),
                });
            }
            Ok(Command::TeamCard {
                team: format!("SIM-{team}"),
            })
        }
        "replay-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "REPLAY-NYR".to_string(),
        }),
        "replay-card" => {
            let team = args.trim().to_ascii_uppercase();
            if !matches!(team.as_str(), "NYR" | "SEA") {
                return Err(ParseError::BadFilter {
                    details: format!("replay-card supports NYR or SEA, got '{team}'"),
                });
            }
            Ok(Command::TeamCard {
                team: format!("REPLAY-{team}"),
            })
        }
        "movement-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "MOVE-NYR".to_string(),
        }),
        "movement-card" => {
            let team = args.trim().to_ascii_uppercase();
            if !matches!(team.as_str(), "NYR" | "SEA") {
                return Err(ParseError::BadFilter {
                    details: format!("movement-card supports NYR or SEA, got '{team}'"),
                });
            }
            Ok(Command::TeamCard {
                team: format!("MOVE-{team}"),
            })
        }
        "history-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "HISTORY-NYR".to_string(),
        }),
        "history-card" => {
            let team = args.trim().to_ascii_uppercase();
            if !matches!(team.as_str(), "NYR" | "SEA") {
                return Err(ParseError::BadFilter {
                    details: format!("history-card supports NYR or SEA, got '{team}'"),
                });
            }
            Ok(Command::TeamCard {
                team: format!("HISTORY-{team}"),
            })
        }
        "matchup-card" | "line-matchup-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "MATCHUP-NYR".to_string(),
        }),
        "matchup-card" | "line-matchup-card" => {
            let values = args.split_whitespace().collect::<Vec<_>>();
            if values.len() == 3 {
                let season = values[0]
                    .parse::<u32>()
                    .map_err(|_| ParseError::BadFilter {
                        details: "matchup-card season must be an 8-digit value".to_owned(),
                    })?;
                let game_id = values[1]
                    .parse::<u64>()
                    .map_err(|_| ParseError::BadFilter {
                        details: "matchup-card game ID must be numeric".to_owned(),
                    })?;
                let team = values[2].to_ascii_uppercase();
                if season < 20_000_000
                    || game_id == 0
                    || team.len() != 3
                    || !team.bytes().all(|byte| byte.is_ascii_uppercase())
                {
                    return Err(ParseError::BadFilter {
                        details: "matchup-card requires <season> <game-id> <team>".to_owned(),
                    });
                }
                return Ok(Command::TeamCard {
                    team: format!("MATCHUP:{season}:{game_id}:{team}"),
                });
            }
            let team = args.trim().to_ascii_uppercase();
            if values.len() != 1 || team != "NYR" {
                return Err(ParseError::BadFilter {
                    details: format!(
                        "matchup-card requires NYR or <season> <game-id> <team>, got '{args}'"
                    ),
                });
            }
            Ok(Command::TeamCard {
                team: "MATCHUP-NYR".to_string(),
            })
        }
        "arrival-card" | "prospect-arrival-card" if args.trim().is_empty() => {
            Ok(Command::TeamCard {
                team: "ARRIVAL-NYR".to_string(),
            })
        }
        "arrival-board" | "prospect-arrival-board" if args.trim().is_empty() => {
            Ok(Command::TeamCard {
                team: "ARRIVAL-BOARD".to_string(),
            })
        }
        "census-readiness" | "prospect-census-readiness" if args.trim().is_empty() => {
            Ok(Command::TeamCard {
                team: "CENSUS-BOARD".to_string(),
            })
        }
        "authority-progress" | "prospect-authority-progress" if args.trim().is_empty() => {
            Ok(Command::TeamCard {
                team: "AUTHORITY-PROGRESS-BOARD".to_string(),
            })
        }
        "authority-closure" | "prospect-authority-closure" if args.trim().is_empty() => {
            Ok(Command::TeamCard {
                team: "AUTHORITY-CLOSURE-BOARD".to_string(),
            })
        }
        "arrival-card" | "prospect-arrival-card" => {
            let team = args.trim().to_ascii_uppercase();
            if !CANONICAL_TEAMS
                .iter()
                .any(|(abbreviation, _)| *abbreviation == team)
            {
                return Err(ParseError::BadFilter {
                    details: format!("prospect-arrival-card requires an NHL team, got '{team}'"),
                });
            }
            Ok(Command::TeamCard {
                team: format!("ARRIVAL-{team}"),
            })
        }
        "window-card" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "WINDOW-NYR".to_string(),
        }),
        "window" | "window-board" if args.trim().is_empty() => Ok(Command::TeamCard {
            team: "WINDOW-BOARD".to_string(),
        }),
        "window-card" => {
            let team = args.trim().to_ascii_uppercase();
            if !CANONICAL_TEAMS
                .iter()
                .any(|(abbreviation, _)| *abbreviation == team)
            {
                return Err(ParseError::BadFilter {
                    details: format!("window-card requires an NHL team, got '{team}'"),
                });
            }
            Ok(Command::TeamCard {
                team: format!("WINDOW-{team}"),
            })
        }
        "team-card" | "icecast-card" => {
            let team = if args.trim().is_empty() {
                "NYR"
            } else {
                args.trim()
            }
            .to_ascii_uppercase();
            let canonical_arrival = team.strip_prefix("ARRIVAL-").is_some_and(|abbreviation| {
                CANONICAL_TEAMS
                    .iter()
                    .any(|(team, _)| *team == abbreviation)
            });
            if !canonical_arrival
                && !matches!(
                    team.as_str(),
                    "NYR"
                        | "SEA"
                        | "SIM-NYR"
                        | "SIM-SEA"
                        | "REPLAY-NYR"
                        | "REPLAY-SEA"
                        | "MOVE-NYR"
                        | "MOVE-SEA"
                        | "HISTORY-NYR"
                        | "HISTORY-SEA"
                        | "MATCHUP-NYR"
                        | "WINDOW-NYR"
                        | "WINDOW-SEA"
                        | "DEX"
                        | "DRAFT"
                        | "MORNING"
                        | "TRADE"
                )
            {
                return Err(ParseError::BadFilter {
                    details: format!(
                        "team-card supports the sealed card families, including ARRIVAL-<NHL team>, got '{team}'"
                    ),
                });
            }
            Ok(Command::TeamCard { team })
        }
        "daily" | "fantasy-daily" => Ok(Command::FantasyDaily {
            date: parse_daily_date(args)?,
        }),
        "matchup" | "fantasy-matchup" => Ok(Command::FantasyMatchup {
            date: parse_dated_handoff("matchup", args)?,
        }),
        "import-yahoo" | "fantasy-import" => parse_fantasy_import(args),
        "roster-shape" | "fantasy-roster-shape" => parse_fantasy_roster_shape(args),
        "watchlist" if args.trim().is_empty() => Ok(Command::Watchlist),
        "watch" => parse_watch(args),
        "transactions" | "txs" | "tx" => Ok(Command::Transactions),
        "playoffs" => Ok(Command::Playoffs),
        "depth" if args.trim().is_empty() => Ok(Command::Depth),
        "depth" => Ok(Command::DepthKv {
            args: crate::tui::filter_state::parse_roster_kv(args).map_err(|err| {
                ParseError::BadFilter {
                    details: err.to_string(),
                }
            })?,
        }),
        "roster" => Ok(Command::Roster),
        "fantasy" => parse_fantasy(args),
        "scores" => Ok(Command::Scores),
        "schedule" => Ok(Command::Schedule),
        "favorites" | "favs" if args.trim().is_empty() => Ok(Command::Favorites),
        "favorites" | "favs" => Ok(Command::FavoritesKv {
            args: crate::tui::filter_state::parse_roster_kv(args).map_err(|err| {
                ParseError::BadFilter {
                    details: err.to_string(),
                }
            })?,
        }),

        // Workspace-swap reads (with args)
        "player" | "p" => parse_player(args),
        "team" | "t" => parse_team(args),
        "compare" | "cmp" | "vs" => parse_compare(args),
        "box" | "boxscore" => parse_box(args),
        "class" => parse_class(args),
        "career" => parse_career(args),
        "report" | "reports" => parse_report(args),
        "records" | "record" => parse_records(args),
        "awards" | "award" | "trophy" => parse_awards(args),
        "streaks" | "streak" => parse_streaks(args),
        "scouting" | "scout" => parse_scouting(args),
        "mates" | "linemates" => parse_mates(args),
        "data" => Ok(Command::Data {
            args: args.trim().to_string(),
        }),
        "snapshot" | "snapshots" => Ok(Command::Snapshot {
            args: args.trim().to_string(),
        }),
        "config" => Ok(Command::Config {
            args: args.trim().to_string(),
        }),

        // Write actions (also accessible via /fav)
        "fav" | "favorite" => parse_fav(args),

        // Free-form filter
        "query" => parse_query(args),

        unknown => Err(ParseError::UnknownCommand(unknown.to_owned())),
    }
}

fn parse_player(args: &str) -> Result<Command, ParseError> {
    let needle = args.trim();
    if needle.is_empty() {
        return Err(ParseError::MissingArg {
            command: "player",
            arg: "name-or-pid",
        });
    }
    Ok(Command::PlayerCard {
        needle: needle.to_owned(),
    })
}

fn parse_watch(args: &str) -> Result<Command, ParseError> {
    let needle = args.trim();
    if needle.is_empty() {
        return Err(ParseError::MissingArg {
            command: "watch",
            arg: "player",
        });
    }
    let (subcommand, rest) = split_first_word(needle);
    match subcommand.to_ascii_lowercase().as_str() {
        "player" => parse_watch_player_rule(rest),
        "enable" | "on" => parse_watch_rule_set_enabled(rest, true),
        "disable" | "off" => parse_watch_rule_set_enabled(rest, false),
        "team" | "deployment" => Err(ParseError::BadFilter {
            details: "watch team/deployment rule editing is deferred; use `icelines watch deployment ...` for CLI preview or `/watchlist` for player rules".to_string(),
        }),
        _ => Ok(Command::WatchPlayer {
            needle: needle.to_string(),
        }),
    }
}

fn parse_watch_rule_set_enabled(args: &str, enabled: bool) -> Result<Command, ParseError> {
    let id = args.trim();
    if id.is_empty() {
        return Err(ParseError::MissingArg {
            command: "watch",
            arg: "rule-id",
        });
    }
    Ok(Command::WatchRuleSetEnabled {
        id: id.to_string(),
        enabled,
    })
}

fn parse_watch_player_rule(args: &str) -> Result<Command, ParseError> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(ParseError::MissingArg {
            command: "watch player",
            arg: "player",
        });
    }
    let lower = trimmed.to_ascii_lowercase();
    let (player, when) = if let Some(idx) = lower.rfind(" when=") {
        let player = trimmed[..idx].trim();
        let when = trimmed[idx + " when=".len()..].trim();
        (player, when)
    } else {
        (trimmed, "promotion")
    };
    if player.is_empty() {
        return Err(ParseError::MissingArg {
            command: "watch player",
            arg: "player",
        });
    }
    if when.is_empty() || when.split_whitespace().nth(1).is_some() {
        return Err(ParseError::BadFilter {
            details: "watch player expects a single trigger after when=".to_string(),
        });
    }
    Ok(Command::WatchRulePlayer {
        player: player.to_string(),
        when: when.to_string(),
    })
}

fn parse_team(args: &str) -> Result<Command, ParseError> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(ParseError::MissingArg {
            command: "team",
            arg: "abbrev",
        });
    }
    let (abbrev, rest) = split_first_word(trimmed);
    let rest = rest.trim();
    if rest.eq_ignore_ascii_case("season") || rest.eq_ignore_ascii_case("schedule") {
        return Ok(Command::TeamSeason {
            abbrev: abbrev.to_uppercase(),
        });
    }
    if !rest.is_empty() {
        return Ok(Command::TeamKv {
            abbrev: abbrev.to_uppercase(),
            args: crate::tui::filter_state::parse_roster_kv(rest).map_err(|err| {
                ParseError::BadFilter {
                    details: err.to_string(),
                }
            })?,
        });
    }
    Ok(Command::Team {
        abbrev: abbrev.to_uppercase(),
    })
}

fn parse_compare(args: &str) -> Result<Command, ParseError> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(ParseError::MissingArg {
            command: "compare",
            arg: "left [right]",
        });
    }
    // Split on first comma OR on " vs " for a more natural feel.
    // If neither separator present, treat the whole thing as
    // `left` (similarity-search mode).
    if let Some((l, r)) = trimmed.split_once(" vs ") {
        return Ok(Command::Compare {
            left: l.trim().to_owned(),
            right: Some(r.trim().to_owned()),
        });
    }
    if let Some((l, r)) = trimmed.split_once(',') {
        return Ok(Command::Compare {
            left: l.trim().to_owned(),
            right: Some(r.trim().to_owned()),
        });
    }
    Ok(Command::Compare {
        left: trimmed.to_owned(),
        right: None,
    })
}

fn parse_box(args: &str) -> Result<Command, ParseError> {
    let game = args.trim();
    if game.is_empty() {
        return Err(ParseError::MissingArg {
            command: "box",
            arg: "game-id-or-team@team",
        });
    }
    Ok(Command::Box {
        game: game.to_owned(),
    })
}

fn parse_class(args: &str) -> Result<Command, ParseError> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(ParseError::MissingArg {
            command: "class",
            arg: "year",
        });
    }
    let year: u16 = trimmed.parse().map_err(|_| ParseError::BadInteger {
        command: "class",
        raw: trimmed.to_owned(),
    })?;
    Ok(Command::Class { year })
}

fn parse_career(args: &str) -> Result<Command, ParseError> {
    let mut parsed = CareerCommandArgs::default();
    for token in args.split_whitespace() {
        let (key, value) = token.split_once('=').unwrap_or(("league", token));
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        if value.is_empty() {
            return Err(ParseError::BadFilter {
                details: format!("career: empty value in {token:?}"),
            });
        }
        match key.as_str() {
            "league" => parsed.league = value.to_ascii_uppercase(),
            "season" => {
                parsed.season = Some(value.parse::<u32>().map_err(|_| ParseError::BadInteger {
                    command: "career",
                    raw: value.to_string(),
                })?);
            }
            "top" | "limit" => {
                parsed.top = value
                    .parse::<usize>()
                    .map_err(|_| ParseError::BadInteger {
                        command: "career",
                        raw: value.to_string(),
                    })?
                    .clamp(1, 100);
            }
            "sort" => parsed.sort = value.to_ascii_lowercase(),
            other => {
                return Err(ParseError::BadFilter {
                    details: format!("career: unknown filter {other:?}"),
                });
            }
        }
    }
    Ok(Command::Career { args: parsed })
}

fn parse_report(args: &str) -> Result<Command, ParseError> {
    let (kind_raw, rest) = split_first_word(args);
    let kind = match kind_raw.to_ascii_lowercase().as_str() {
        "poach" | "poacher" => ReportKind::Poach,
        "weekly" | "week" => ReportKind::Weekly,
        "" => {
            return Err(ParseError::MissingArg {
                command: "report",
                arg: "poach|weekly",
            });
        }
        other => return Err(ParseError::UnknownCommand(format!("report {other}"))),
    };

    let mut parsed = ReportCommandArgs {
        kind,
        categories: Vec::new(),
        availability: None,
        top: None,
    };
    for token in rest.split_whitespace() {
        let (key, value) = token
            .split_once('=')
            .map_or(("category", token), |(key, value)| {
                (key.trim(), value.trim())
            });
        if value.is_empty() {
            return Err(ParseError::BadFilter {
                details: format!("report: empty value in {token:?}"),
            });
        }
        match key.to_ascii_lowercase().as_str() {
            "cat" | "cats" | "category" | "categories" => {
                parsed.categories = value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect();
            }
            "availability" | "avail" => parsed.availability = Some(value.to_string()),
            "top" | "limit" => {
                parsed.top = Some(value.parse::<u16>().map_err(|_| ParseError::BadInteger {
                    command: "report",
                    raw: value.to_string(),
                })?);
            }
            other => {
                return Err(ParseError::BadFilter {
                    details: format!("report: unknown filter {other:?}"),
                });
            }
        }
    }
    Ok(Command::Report { args: parsed })
}

fn parse_records(args: &str) -> Result<Command, ParseError> {
    let (target_raw, subject_raw) = split_first_word(args);
    let target = match target_raw.to_ascii_lowercase().as_str() {
        "player" | "p" => RecordsTarget::Player,
        "team" | "t" => RecordsTarget::Team,
        "" => {
            return Err(ParseError::MissingArg {
                command: "records",
                arg: "player|team",
            });
        }
        other => return Err(ParseError::UnknownCommand(format!("records {other}"))),
    };
    let subject = subject_raw.trim();
    if subject.is_empty() {
        return Err(ParseError::MissingArg {
            command: "records",
            arg: match target {
                RecordsTarget::Player => "player",
                RecordsTarget::Team => "team",
            },
        });
    }
    Ok(Command::Records {
        args: RecordsCommandArgs {
            target,
            subject: subject.to_string(),
        },
    })
}

fn parse_awards(args: &str) -> Result<Command, ParseError> {
    let trimmed = args.trim();
    let player = trimmed.strip_prefix("player ").unwrap_or(trimmed).trim();
    if player.is_empty() {
        return Err(ParseError::MissingArg {
            command: "awards",
            arg: "player <name>",
        });
    }
    Ok(Command::Awards {
        player: player.to_string(),
    })
}

fn parse_streaks(args: &str) -> Result<Command, ParseError> {
    let trimmed = args.trim();
    let player = trimmed.strip_prefix("player ").unwrap_or(trimmed).trim();
    if player.is_empty() {
        return Err(ParseError::MissingArg {
            command: "streaks",
            arg: "player <name>",
        });
    }
    Ok(Command::Streaks {
        player: player.to_string(),
    })
}

fn parse_scouting(args: &str) -> Result<Command, ParseError> {
    let player = parse_player_subject("scouting", args)?;
    Ok(Command::Scouting { player })
}

fn parse_mates(args: &str) -> Result<Command, ParseError> {
    let player = parse_player_subject("mates", args)?;
    Ok(Command::Mates { player })
}

fn parse_player_subject(command: &'static str, args: &str) -> Result<String, ParseError> {
    let trimmed = args.trim();
    let player = trimmed.strip_prefix("player ").unwrap_or(trimmed).trim();
    if player.is_empty() {
        return Err(ParseError::MissingArg {
            command,
            arg: "player <name>",
        });
    }
    Ok(player.to_string())
}

/// `fantasy roster` → Roster; `fantasy gaps` → roster-gap board.
fn parse_fantasy(args: &str) -> Result<Command, ParseError> {
    let (sub, rest) = split_first_word(args);
    match sub.to_ascii_lowercase().as_str() {
        "" | "roster" => Ok(Command::Roster),
        "gaps" | "gap" if !rest.trim().is_empty() => Ok(Command::FantasyGapsKv {
            args: parse_fantasy_gaps_kv(rest)?,
        }),
        "gaps" | "gap" => Ok(Command::FantasyGaps),
        "simulate" | "sim" if !rest.trim().is_empty() => Ok(Command::FantasySimKv {
            args: parse_fantasy_sim_kv(rest)?,
        }),
        "simulate" | "sim" => Ok(Command::FantasySim),
        "daily" => Ok(Command::FantasyDaily {
            date: parse_daily_date(rest)?,
        }),
        "matchup" => Ok(Command::FantasyMatchup {
            date: parse_dated_handoff("matchup", rest)?,
        }),
        "import" | "import-yahoo" => parse_fantasy_import(rest),
        "roster-shape" | "shape" => parse_fantasy_roster_shape(rest),
        "poach" if rest.trim().is_empty() => Ok(Command::Poach),
        "poach" => Ok(Command::PoachKv {
            args: parse_poach_kv(rest)?,
        }),
        unknown => Err(ParseError::UnknownCommand(format!("fantasy {unknown}"))),
    }
}

fn parse_daily_date(args: &str) -> Result<String, ParseError> {
    parse_dated_handoff("daily", args)
}

fn parse_dated_handoff(command: &'static str, args: &str) -> Result<String, ParseError> {
    let value = args
        .trim()
        .strip_prefix("date=")
        .unwrap_or(args.trim())
        .trim();
    if value.is_empty() {
        return Err(ParseError::MissingArg {
            command,
            arg: "date=YYYY-MM-DD",
        });
    }
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| ParseError::BadFilter {
        details: format!("{command}: date {value:?} must be YYYY-MM-DD"),
    })?;
    Ok(value.to_string())
}

fn parse_fantasy_import(args: &str) -> Result<Command, ParseError> {
    let segments = parse_command_segments(args, &["file", "league", "my-team", "dry-run"])?;
    let mut file = None;
    let mut league = None;
    let mut my_team = None;
    let mut dry_run = false;

    for (key, value) in segments {
        match key.as_str() {
            "file" => file = Some(required_segment_value("import-yahoo", "file", &value)?),
            "league" => league = Some(required_segment_value("import-yahoo", "league", &value)?),
            "my-team" => {
                my_team = Some(required_segment_value("import-yahoo", "my-team", &value)?);
            }
            "dry-run" => {
                if !value.trim().is_empty() {
                    return Err(ParseError::BadFilter {
                        details: "import-yahoo: dry-run does not take a value".to_string(),
                    });
                }
                dry_run = true;
            }
            other => {
                return Err(ParseError::BadFilter {
                    details: format!("import-yahoo: unknown filter {other:?}"),
                });
            }
        }
    }

    Ok(Command::FantasyImport {
        file: file.ok_or(ParseError::MissingArg {
            command: "import-yahoo",
            arg: "file",
        })?,
        league: league.ok_or(ParseError::MissingArg {
            command: "import-yahoo",
            arg: "league",
        })?,
        my_team,
        dry_run,
    })
}

fn parse_fantasy_roster_shape(args: &str) -> Result<Command, ParseError> {
    let (sub, rest) = split_first_word(args);
    let action = match sub.to_ascii_lowercase().as_str() {
        "" | "show" => {
            let segments = parse_command_segments(rest, &["league"])?;
            let mut league = None;
            for (key, value) in segments {
                match key.as_str() {
                    "league" => {
                        league = Some(required_segment_value("roster-shape", "league", &value)?);
                    }
                    other => {
                        return Err(ParseError::BadFilter {
                            details: format!("roster-shape: unknown filter {other:?}"),
                        })
                    }
                }
            }
            FantasyRosterShapeAction::Show { league }
        }
        "set" => {
            let (bare_shape, remaining) = leading_bare_value(rest, &["shape", "league"]);
            let segments = parse_command_segments(remaining, &["shape", "league"])?;
            let mut shape = bare_shape;
            let mut league = None;
            for (key, value) in segments {
                match key.as_str() {
                    "shape" => {
                        shape = Some(required_segment_value("roster-shape", "shape", &value)?);
                    }
                    "league" => {
                        league = Some(required_segment_value("roster-shape", "league", &value)?);
                    }
                    other => {
                        return Err(ParseError::BadFilter {
                            details: format!("roster-shape: unknown filter {other:?}"),
                        })
                    }
                }
            }
            FantasyRosterShapeAction::Set {
                shape: shape.ok_or(ParseError::MissingArg {
                    command: "roster-shape set",
                    arg: "shape",
                })?,
                league,
            }
        }
        "validate" | "check" => {
            let segments = parse_command_segments(rest, &["league", "team"])?;
            let mut league = None;
            let mut team = None;
            for (key, value) in segments {
                match key.as_str() {
                    "league" => {
                        league = Some(required_segment_value("roster-shape", "league", &value)?);
                    }
                    "team" => {
                        team = Some(required_segment_value("roster-shape", "team", &value)?);
                    }
                    other => {
                        return Err(ParseError::BadFilter {
                            details: format!("roster-shape: unknown filter {other:?}"),
                        })
                    }
                }
            }
            FantasyRosterShapeAction::Validate { league, team }
        }
        other => return Err(ParseError::UnknownCommand(format!("roster-shape {other}"))),
    };
    Ok(Command::FantasyRosterShape { action })
}

fn leading_bare_value<'a>(input: &'a str, keys: &[&'static str]) -> (Option<String>, &'a str) {
    let (first, rest) = split_first_word(input);
    let key = first.to_ascii_lowercase();
    if first.is_empty() || first.contains('=') || keys.contains(&key.as_str()) {
        (None, input)
    } else {
        (Some(first.replace('_', " ")), rest)
    }
}

fn parse_fantasy_sim_kv(args: &str) -> Result<FantasySimulationCommandArgs, ParseError> {
    let segments = parse_command_segments(args, &["add", "drop", "weeks", "clear"])?;
    let mut parsed = FantasySimulationCommandArgs::default();
    for (key, value) in segments {
        match key.as_str() {
            "add" => parsed.add_player = Some(required_segment_value("simulate", "add", &value)?),
            "drop" => {
                parsed.drop_player = Some(required_segment_value("simulate", "drop", &value)?);
            }
            "weeks" => {
                let raw = required_segment_value("simulate", "weeks", &value)?;
                let weeks = raw.parse::<u8>().map_err(|_| ParseError::BadInteger {
                    command: "simulate",
                    raw,
                })?;
                parsed.weeks = Some(weeks.clamp(1, 26));
            }
            "clear" => {
                if !value.trim().is_empty() {
                    return Err(ParseError::BadFilter {
                        details: "simulate: clear does not take a value".to_string(),
                    });
                }
                parsed.clear = true;
            }
            other => {
                return Err(ParseError::BadFilter {
                    details: format!("simulate: unknown filter {other:?}"),
                });
            }
        }
    }
    if !parsed.clear
        && parsed.weeks.is_none()
        && parsed.add_player.is_none()
        && parsed.drop_player.is_none()
    {
        return Err(ParseError::MissingArg {
            command: "simulate",
            arg: "add/drop/weeks/clear",
        });
    }
    Ok(parsed)
}

fn parse_fantasy_gaps_kv(args: &str) -> Result<FantasyGapsCommandArgs, ParseError> {
    let mut parsed = FantasyGapsCommandArgs::default();
    for token in args.split_whitespace() {
        let (key, value) = token
            .split_once('=')
            .map_or(("", token), |(key, value)| (key.trim(), value.trim()));
        let key = key.to_ascii_lowercase();
        let value = value.trim();
        if value.is_empty() {
            return Err(ParseError::BadFilter {
                details: format!("gaps: empty value in {token:?}"),
            });
        }
        match key.as_str() {
            "" | "cat" | "cats" | "category" | "categories" => {
                parsed.categories = Some(parse_category_list(value));
            }
            "top" | "limit" => {
                let limit = value.parse::<usize>().map_err(|_| ParseError::BadInteger {
                    command: "gaps",
                    raw: value.to_string(),
                })?;
                parsed.limit = Some(limit.clamp(1, 64));
            }
            other => {
                return Err(ParseError::BadFilter {
                    details: format!("gaps: unknown filter {other:?}"),
                });
            }
        }
    }
    Ok(parsed)
}

fn required_segment_value(
    command: &'static str,
    key: &'static str,
    value: &str,
) -> Result<String, ParseError> {
    let value = value.trim();
    if value.is_empty() {
        Err(ParseError::MissingArg { command, arg: key })
    } else {
        Ok(value.to_string())
    }
}

fn parse_command_segments(
    input: &str,
    keys: &[&'static str],
) -> Result<Vec<(String, String)>, ParseError> {
    let mut segments: Vec<(String, String)> = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_value: Vec<String> = Vec::new();

    for token in input.split_whitespace() {
        if let Some((raw_key, raw_value)) = token.split_once('=') {
            let key = raw_key.to_ascii_lowercase();
            if !keys.contains(&key.as_str()) {
                return Err(ParseError::BadFilter {
                    details: format!("unknown argument {raw_key:?}"),
                });
            }
            if let Some(key) = current_key.take() {
                segments.push((key, current_value.join(" ")));
                current_value.clear();
            }
            current_key = Some(key);
            if !raw_value.is_empty() {
                current_value.push(raw_value.replace('_', " "));
            }
        } else {
            let key = token.to_ascii_lowercase();
            if keys.contains(&key.as_str()) {
                if let Some(key) = current_key.take() {
                    segments.push((key, current_value.join(" ")));
                    current_value.clear();
                }
                current_key = Some(key);
            } else if current_key.is_some() {
                current_value.push(token.replace('_', " "));
            } else {
                return Err(ParseError::BadFilter {
                    details: format!("unknown argument {token:?}"),
                });
            }
        }
    }

    if let Some(key) = current_key {
        segments.push((key, current_value.join(" ")));
    }
    Ok(segments)
}

fn parse_poach_kv(args: &str) -> Result<PoachCommandArgs, ParseError> {
    let mut parsed = PoachCommandArgs::default();
    for token in args.split_whitespace() {
        let (key, value) = token
            .split_once('=')
            .map_or(("", token), |(key, value)| (key.trim(), value.trim()));
        let key = key.to_ascii_lowercase();
        let value = value.trim();
        if value.is_empty() {
            return Err(ParseError::BadFilter {
                details: format!("poach: empty value in {token:?}"),
            });
        }
        match key.as_str() {
            "" => apply_poach_bare_token(&mut parsed, value)?,
            "pos" | "position" | "positions" => {
                parsed.positions = Some(parse_poach_positions(value)?);
            }
            "cat" | "cats" | "category" | "categories" => {
                parsed.categories = Some(parse_poach_categories(value));
            }
            "avail" | "availability" => {
                parsed.availability_filter = Some(parse_poach_availability(value)?);
            }
            "kind" | "candidate" | "type" => {
                parsed.candidate_kind = Some(parse_poach_candidate_kind(value)?);
            }
            "top" | "limit" => {
                let limit = value.parse::<u16>().map_err(|_| ParseError::BadInteger {
                    command: "poach",
                    raw: value.to_string(),
                })?;
                parsed.limit = Some(limit.clamp(1, 100));
            }
            other => {
                return Err(ParseError::BadFilter {
                    details: format!("poach: unknown filter {other:?}"),
                });
            }
        }
    }
    Ok(parsed)
}

fn apply_poach_bare_token(args: &mut PoachCommandArgs, value: &str) -> Result<(), ParseError> {
    if let Ok(positions) = parse_poach_positions(value) {
        args.positions = Some(positions);
        return Ok(());
    }
    if let Ok(availability) = parse_poach_availability(value) {
        args.availability_filter = Some(availability);
        return Ok(());
    }
    if let Ok(kind) = parse_poach_candidate_kind(value) {
        args.candidate_kind = Some(kind);
        return Ok(());
    }
    Err(ParseError::BadFilter {
        details: format!("poach: unknown filter token {value:?}"),
    })
}

fn parse_poach_positions(value: &str) -> Result<Vec<Position>, ParseError> {
    let mut out = Vec::new();
    for raw in value.split(',') {
        let raw = raw.trim().to_ascii_lowercase();
        match raw.as_str() {
            "all" | "any" => return Ok(Vec::new()),
            "c" | "center" => push_unique_position(&mut out, Position::Center),
            "l" | "lw" | "left" | "left-wing" | "left_wing" => {
                push_unique_position(&mut out, Position::LeftWing);
            }
            "r" | "rw" | "right" | "right-wing" | "right_wing" => {
                push_unique_position(&mut out, Position::RightWing);
            }
            "d" | "def" | "defense" | "defence" => {
                push_unique_position(&mut out, Position::Defense);
            }
            "g" | "goalie" | "goalies" => push_unique_position(&mut out, Position::Goalie),
            "f" | "forward" | "forwards" => {
                push_unique_position(&mut out, Position::Center);
                push_unique_position(&mut out, Position::LeftWing);
                push_unique_position(&mut out, Position::RightWing);
            }
            "" => {}
            _ => {
                return Err(ParseError::BadFilter {
                    details: format!("poach: unknown position {raw:?}"),
                });
            }
        }
    }
    Ok(out)
}

fn push_unique_position(out: &mut Vec<Position>, position: Position) {
    if !out.contains(&position) {
        out.push(position);
    }
}

fn parse_poach_categories(value: &str) -> Vec<String> {
    parse_category_list(value)
}

fn parse_category_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .filter_map(|raw| {
            let category = raw.trim().to_ascii_lowercase().replace('_', "-");
            if category.is_empty() || matches!(category.as_str(), "all" | "default" | "scheme") {
                None
            } else {
                Some(category)
            }
        })
        .collect()
}

fn parse_poach_availability(value: &str) -> Result<PoachAvailabilityFilter, ParseError> {
    match value.to_ascii_lowercase().as_str() {
        "any" | "all" => Ok(PoachAvailabilityFilter::Any),
        "available" | "avail" => Ok(PoachAvailabilityFilter::Available),
        "free" | "imported-available" | "imported_available" => {
            Ok(PoachAvailabilityFilter::ImportedAvailable)
        }
        "not-mine" | "not_my_roster" | "not-my-roster" | "notonuser" | "not-on-user" => {
            Ok(PoachAvailabilityFilter::NotOnUserRoster)
        }
        "watched" | "watch" | "watchlist" => Ok(PoachAvailabilityFilter::Watched),
        "unknown" => Ok(PoachAvailabilityFilter::Unknown),
        other => Err(ParseError::BadFilter {
            details: format!("poach: unknown availability {other:?}"),
        }),
    }
}

fn parse_poach_candidate_kind(value: &str) -> Result<PoachCandidateKind, ParseError> {
    match value.to_ascii_lowercase().as_str() {
        "all" | "any" => Ok(PoachCandidateKind::All),
        "streamer" | "streamers" => Ok(PoachCandidateKind::Streamer),
        "stash" | "stashes" => Ok(PoachCandidateKind::Stash),
        "category" | "category-specialist" | "category_specialist" | "cats" => {
            Ok(PoachCandidateKind::CategorySpecialist)
        }
        "riser" | "risers" | "deployment" | "deployment-riser" | "deployment_riser" => {
            Ok(PoachCandidateKind::DeploymentRiser)
        }
        "goalie-streamer" | "goalie_streamer" | "goalie" => Ok(PoachCandidateKind::GoalieStreamer),
        "watch-alert" | "watch_alert" | "alert" => Ok(PoachCandidateKind::WatchAlert),
        other => Err(ParseError::BadFilter {
            details: format!("poach: unknown candidate kind {other:?}"),
        }),
    }
}

fn parse_stats_kv(args: &str) -> Result<Command, ParseError> {
    let args =
        crate::tui::filter_state::parse_roster_kv(args).map_err(|err| ParseError::BadFilter {
            details: err.to_string(),
        })?;
    let mut atoms = Vec::new();
    if let Some(country) = args.country {
        atoms.push(format!("nationality={}", country.as_str()));
    }
    if let Some(pos) = args.pos {
        if pos != crate::tui::filter_state::PosFilter::All {
            atoms.push(format!("pos={}", pos.label()));
        }
    }
    if let Some(min_gp) = args.min_gp {
        atoms.push(format!("gp>={min_gp}"));
    }
    if atoms.is_empty() {
        Ok(Command::Stats)
    } else {
        let filter = atoms.join(" AND ");
        match icelines_query::parse_query(icelines_query::FilterInput::Cli(filter.clone())) {
            Ok(_) => Ok(Command::Query { filter }),
            Err(errs) => Err(ParseError::BadFilter {
                details: errs
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            }),
        }
    }
}

fn parse_query(args: &str) -> Result<Command, ParseError> {
    let filter = args.trim();
    if filter.is_empty() {
        return Err(ParseError::MissingArg {
            command: "query",
            arg: "filter",
        });
    }
    // Validate via Phase Art Ross's parser. We don't STORE the
    // plan here (execute_command does that against app); just
    // confirm the filter parses. If it doesn't, surface as
    // BadFilter.
    match icelines_query::parse_query(icelines_query::FilterInput::Cli(filter.to_owned())) {
        Ok(_) => Ok(Command::Query {
            filter: filter.to_owned(),
        }),
        Err(errs) => Err(ParseError::BadFilter {
            details: errs
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        }),
    }
}

// ── Executor ────────────────────────────────────────────────────────────────

/// Phase Adams.2 — outcome of running an executed Command.
/// Matches the spec's "what does the orchestrator do next" model:
/// Continue is the no-op default; Quit propagates; Flash sets a
/// transient success or error message in the cmdbar/status row.
#[derive(Debug, Clone)]
pub enum ExecResult {
    Continue,
    Quit,
    Flash(String),
}

fn set_workspace_screen(app: &mut crate::tui::app::App, screen: crate::tui::app::Screen) {
    app.screen = screen;
    if let Some(mdi) = app.mdi.as_mut() {
        if let Some(id) = crate::tui::workbench::workbench_for_screen(&app.screen) {
            mdi.select_workbench_id(id);
            if let Some(experience) = crate::tui::workbench::tui_experience_for_workbench(id) {
                mdi.apply_experience(experience);
                return;
            }
        }
        mdi.clear_active_experience();
    }
}

/// Phase Adams.2 — run a parsed Command against `App`. Mutates
/// `app.screen` (workspace swap), `app.mdi.show_*` (pane
/// toggles), `app.queries.filter_text` + filter_plan (Query),
/// favorites DB (FavAdd/FavRemove), or returns Quit/Flash for
/// the orchestrator to handle.
///
/// Per spec edge-2: command-bar Query mutates Stats screen
/// state directly — single source of truth for filter text.
pub fn execute_command(cmd: Command, app: &mut crate::tui::app::App) -> ExecResult {
    use crate::tui::app::Screen;
    match cmd {
        // ── Meta ─────────────────────────────────────────────
        Command::Help => {
            app.show_help = true;
            ExecResult::Continue
        }
        Command::Quit => ExecResult::Quit,
        Command::Hide(pane) => {
            if let Some(mdi) = &mut app.mdi {
                mdi.set_side_pane_visible(pane, false);
            }
            ExecResult::Continue
        }
        Command::Show(pane) => {
            if let Some(mdi) = &mut app.mdi {
                mdi.set_side_pane_visible(pane, true);
            }
            ExecResult::Continue
        }
        Command::Admin => {
            app.show_admin = true;
            ExecResult::Flash("admin overlay opened".to_string())
        }

        // ── Workspace swap (no args) ─────────────────────────
        Command::Stats => {
            set_workspace_screen(app, Screen::Queries);
            ExecResult::Continue
        }
        Command::Goalies => {
            set_workspace_screen(app, Screen::Goalies);
            ExecResult::Continue
        }
        Command::GoaliesKv { args } => exec_goalies_kv(app, args),
        Command::Poach => {
            set_workspace_screen(app, Screen::Poach);
            ExecResult::Continue
        }
        Command::PoachKv { args } => exec_poach_kv(app, args),
        Command::FantasyGaps => {
            set_workspace_screen(app, Screen::FantasyGaps);
            ExecResult::Continue
        }
        Command::FantasyGapsKv { args } => exec_fantasy_gaps_kv(app, args),
        Command::FantasySim => {
            set_workspace_screen(app, Screen::FantasySim);
            ExecResult::Continue
        }
        Command::FantasySimKv { args } => exec_fantasy_sim_kv(app, args),
        Command::TeamCard { team } => {
            app.selected = 0;
            set_workspace_screen(
                app,
                Screen::TeamCard {
                    team,
                    compare: false,
                },
            );
            ExecResult::Continue
        }
        Command::FantasyDaily { date } => ExecResult::Flash(format!(
            "daily fantasy delta: run `icelines fantasy daily --date {date}` or open `/api/v1/fantasy/daily?date={date}`"
        )),
        Command::FantasyMatchup { date } => ExecResult::Flash(format!(
            "fantasy matchup week: run `icelines fantasy matchup --date {date}` or open `/api/v1/fantasy/matchup?date={date}`"
        )),
        Command::FantasyImport {
            file,
            league,
            my_team,
            dry_run,
        } => ExecResult::Flash(format!(
            "fantasy roster import: run `icelines fantasy import-yahoo --file {file} --league \"{league}\"{}{}`; web dashboard import is POST-deferred",
            my_team
                .as_ref()
                .map(|team| format!(" --my-team \"{team}\""))
                .unwrap_or_default(),
            if dry_run { " --dry-run" } else { "" }
        )),
        Command::FantasyRosterShape { action } => ExecResult::Flash(roster_shape_handoff(action)),
        Command::Watchlist => {
            set_workspace_screen(app, Screen::GroupDetail("Watchlist".to_string()));
            ExecResult::Continue
        }
        Command::WatchPlayer { needle } => ExecResult::Flash(format!(
            "watch: run `icelines watch note \"{needle}\" \"reason\"`, preview `icelines watch player \"{needle}\" --when pp1 --save`, or open `/watchlist`"
        )),
        Command::WatchRulePlayer { player, when } => exec_watch_rule_player(&player, &when),
        Command::WatchRuleSetEnabled { id, enabled } => exec_watch_rule_set_enabled(&id, enabled),
        Command::Transactions => {
            set_workspace_screen(app, Screen::Transactions);
            ExecResult::Continue
        }
        Command::Playoffs => {
            set_workspace_screen(app, Screen::Playoffs);
            ExecResult::Continue
        }
        Command::Depth => {
            set_workspace_screen(app, Screen::Depth);
            ExecResult::Continue
        }
        Command::DepthKv { args } => exec_depth_kv(app, args),
        Command::Scores => {
            set_workspace_screen(app, Screen::Tonight);
            ExecResult::Continue
        }
        Command::Schedule => {
            set_workspace_screen(app, Screen::Schedule);
            ExecResult::Continue
        }
        Command::Favorites => {
            set_workspace_screen(app, Screen::Favorites);
            ExecResult::Continue
        }
        Command::FavoritesKv { args } => exec_favorites_kv(app, args),

        // ── Workspace swap (with args) ───────────────────────
        Command::PlayerCard { needle } => {
            // Resolve via the cross-bundled name lookup the CLI
            // uses for `query player <name>`. On miss, surface a
            // flash with candidates (or just a "not found" hint).
            match icelines_fetch::stats_loader::resolve_player_id_by_name(&needle) {
                Some(pid) => {
                    set_workspace_screen(
                        app,
                        Screen::PlayerById(icelines_core::identity::PlayerId(pid)),
                    );
                    ExecResult::Continue
                }
                None => ExecResult::Flash(format!("player not found: {needle:?}")),
            }
        }
        Command::Team { abbrev } => {
            set_workspace_screen(app, Screen::Team(abbrev));
            ExecResult::Continue
        }
        Command::TeamKv { abbrev, args } => exec_team_kv(app, abbrev, args),
        Command::TeamSeason { abbrev } => {
            set_workspace_screen(app, Screen::ScheduleTeam(abbrev));
            ExecResult::Continue
        }
        Command::Compare { left, right } => {
            if let Some(right) = right {
                return ExecResult::Flash(format!(
                    "compare: run `icelines query compare \"{left}\" \"{right}\"` or open `/compare?left={}&right={}`",
                    url_component(&left),
                    url_component(&right)
                ));
            }
            match icelines_fetch::stats_loader::resolve_player_id_by_name(&left) {
                Some(pid) => {
                    set_workspace_screen(
                        app,
                        Screen::CompsById(icelines_core::identity::PlayerId(pid)),
                    );
                    ExecResult::Continue
                }
                None => ExecResult::Flash(format!("player not found: {left:?}")),
            }
        }
        Command::Box { game } => {
            match game.parse::<u64>() {
                Ok(game_id) => {
                    set_workspace_screen(app, Screen::GameDetail(game_id));
                    ExecResult::Continue
                }
                Err(_) => match resolve_matchup_game_id(app, &game) {
                    Ok(Some(game_id)) => {
                        set_workspace_screen(app, Screen::GameDetail(game_id));
                        crate::tui::tonight::maybe_fetch_boxscore(
                            app.tonight.boxscore_cache.clone(),
                            game_id,
                        );
                        ExecResult::Continue
                    }
                    Ok(None) => ExecResult::Flash(format!(
                        "box: no loaded game matched {game:?}; open Scores/Schedule first or use numeric game-id"
                    )),
                    Err(message) => ExecResult::Flash(message),
                },
            }
        }
        Command::Class { year } => {
            let filter = format!("draft-year={year}");
            match icelines_query::parse_query(icelines_query::FilterInput::Cli(filter.clone())) {
                Ok(plan) => {
                    app.queries.filter_text = filter.clone();
                    app.queries.filter_plan = Some(plan);
                    app.queries.filter_error = None;
                    set_workspace_screen(app, Screen::Queries);
                    ExecResult::Flash(format!("draft class applied: {filter}"))
                }
                Err(errs) => ExecResult::Flash(format!(
                    "draft-class filter parse error: {}",
                    errs.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                )),
            }
        }
        Command::Career { args } => {
            let season_arg = args
                .season
                .map(|season| format!(" --season {season}"))
                .unwrap_or_default();
            let season_query = args
                .season
                .map(|season| format!("&season={season}"))
                .unwrap_or_default();
            ExecResult::Flash(format!(
                "career cohorts: run `icelines query career --league {}{} --sort {} --top {}` or open `/career?league={}&sort={}{}&top={}`",
                args.league,
                season_arg,
                args.sort,
                args.top,
                args.league,
                args.sort,
                season_query,
                args.top
            ))
        }
        Command::Report { args } => {
            let mut cli_args = Vec::new();
            let mut query_args = Vec::new();
            if !args.categories.is_empty() {
                let categories = args.categories.join(",");
                cli_args.push(format!("--category {categories}"));
                query_args.push(format!("category={}", url_component(&categories)));
            }
            if let Some(availability) = &args.availability {
                cli_args.push(format!("--availability {availability}"));
                query_args.push(format!("availability={}", url_component(availability)));
            }
            if let Some(top) = args.top {
                cli_args.push(format!("--top {top}"));
                query_args.push(format!("top={top}"));
            }
            let cli_suffix = if cli_args.is_empty() {
                String::new()
            } else {
                format!(" {}", cli_args.join(" "))
            };
            let query_suffix = if query_args.is_empty() {
                String::new()
            } else {
                format!("?{}", query_args.join("&"))
            };
            ExecResult::Flash(format!(
                "report: run `icelines report {}{}` or open `/reports/{}{}`",
                args.kind.as_str(),
                cli_suffix,
                args.kind.as_str(),
                query_suffix
            ))
        }
        Command::Records { args } => match args.target {
            RecordsTarget::Player => {
                match icelines_fetch::stats_loader::resolve_player_id_by_name(&args.subject) {
                    Some(pid) => {
                        let player_id = icelines_core::identity::PlayerId(pid);
                        if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(
                            &mut app.repo,
                            player_id,
                        ) {
                            return ExecResult::Flash(format!(
                                "records: could not load player {pid}: {e}"
                            ));
                        }
                        app.prev_screen = Some(app.screen.clone());
                        set_workspace_screen(app, Screen::PlayerRecordsById(player_id));
                        ExecResult::Flash(format!(
                            "records: {}  -  CLI: icelines records player \"{}\" --metric ...",
                            args.subject, args.subject
                        ))
                    }
                    None => {
                        ExecResult::Flash(format!("records: player not found: {:?}", args.subject))
                    }
                }
            }
            RecordsTarget::Team => {
                let team = args.subject.to_ascii_uppercase();
                ExecResult::Flash(format!(
                    "records: run `icelines records team {team} --metric players-scored-against-team|goalies-beaten-by-team|fight-opponents-by-team` or open `/records/team/{team}?metric=...`"
                ))
            }
        },
        Command::Awards { player } => {
            match icelines_fetch::stats_loader::resolve_player_id_by_name(&player) {
                Some(pid) => {
                    let player_id = icelines_core::identity::PlayerId(pid);
                    if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(
                        &mut app.repo,
                        player_id,
                    ) {
                        return ExecResult::Flash(format!(
                            "awards: could not load player {pid}: {e}"
                        ));
                    }
                    app.prev_screen = Some(app.screen.clone());
                    set_workspace_screen(app, Screen::PlayerAwardsById(player_id));
                    ExecResult::Flash(format!(
                        "awards: {player}  -  fetch/update with `icelines awards \"{player}\"`"
                    ))
                }
                None => ExecResult::Flash(format!("awards: player not found: {player:?}")),
            }
        }
        Command::Streaks { player } => {
            match icelines_fetch::stats_loader::resolve_player_id_by_name(&player) {
                Some(pid) => {
                    let player_id = icelines_core::identity::PlayerId(pid);
                    if let Err(e) = icelines_fetch::stats_loader::load_player_career_into_repo(
                        &mut app.repo,
                        player_id,
                    ) {
                        return ExecResult::Flash(format!(
                            "streaks: could not load player {pid}: {e}"
                        ));
                    }
                    app.prev_screen = Some(app.screen.clone());
                    set_workspace_screen(app, Screen::PlayerStreaksById(player_id));
                    ExecResult::Flash(format!(
                        "streaks: {player}  -  CLI: icelines streaks \"{player}\""
                    ))
                }
                None => ExecResult::Flash(format!("streaks: player not found: {player:?}")),
            }
        }
        Command::Scouting { player } => {
            match icelines_fetch::stats_loader::resolve_player_id_by_name(&player) {
                Some(pid) => ExecResult::Flash(format!(
                    "scouting: run `icelines scouting \"{player}\"` or open `/scouting/{pid}`"
                )),
                None => ExecResult::Flash(format!(
                    "scouting: run `icelines scouting \"{player}\"`"
                )),
            }
        }
        Command::Mates { player } => ExecResult::Flash(format!(
            "mates: run `icelines mates \"{player}\"` for roster fallback; shifts locked off"
        )),
        Command::Data { args } => ExecResult::Flash(cli_handoff("data", &args, "/admin")),
        Command::Snapshot { args } => ExecResult::Flash(cli_handoff("snapshot", &args, "/admin")),
        Command::Config { args } => ExecResult::Flash(cli_handoff("config", &args, "/admin")),

        // ── Roster / fantasy ─────────────────────────────────
        Command::Roster => {
            set_workspace_screen(app, Screen::FantasyGaps);
            ExecResult::Flash("active fantasy roster gaps".to_string())
        }

        // ── Write actions: favorites mutation ────────────────
        Command::FavAdd { needle } => exec_fav_add(app, &needle),
        Command::FavRemove { needle } => exec_fav_remove(app, &needle),

        // ── Free-form filter ─────────────────────────────────
        Command::Query { filter } => {
            // Per spec edge-2: shared state with Stats filter
            // editor. Mutate app.queries.filter_text directly;
            // re-parse to populate filter_plan; swap workspace
            // to Stats.
            match icelines_query::parse_query(icelines_query::FilterInput::Cli(filter.clone())) {
                Ok(plan) => {
                    app.queries.filter_text = filter.clone();
                    app.queries.filter_plan = Some(plan);
                    app.queries.filter_error = None;
                    set_workspace_screen(app, Screen::Queries);
                    ExecResult::Flash(format!("filter applied: {filter}"))
                }
                Err(errs) => ExecResult::Flash(format!(
                    "filter parse error: {}",
                    errs.iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("; ")
                )),
            }
        }
    }
}

fn exec_watch_rule_player(player: &str, when: &str) -> ExecResult {
    let rule = crate::commands::poach::player_watch_rule(player, when);
    let Ok(intent) = WatchRuleMutationIntent::create(&rule.id) else {
        return ExecResult::Flash("watch rule id is required".to_string());
    };
    let trigger_json = match serde_json::to_string(&rule.trigger) {
        Ok(json) => json,
        Err(err) => return ExecResult::Flash(format!("Could not serialize watch rule: {err}")),
    };
    let unsupported_sources_json = match serde_json::to_string(&rule.unsupported_sources) {
        Ok(json) => json,
        Err(err) => {
            return ExecResult::Flash(format!("Could not serialize watch rule sources: {err}"))
        }
    };
    match crate::db::GroupDb::open().and_then(|db| {
        db.upsert_watch_rule(
            &intent.rule_id,
            &rule.label,
            true,
            &trigger_json,
            &unsupported_sources_json,
        )
    }) {
        Ok(()) => ExecResult::Flash(format!("Saved watch rule '{}'", intent.rule_id)),
        Err(err) => ExecResult::Flash(format!(
            "Could not save watch rule '{}': {err}",
            intent.rule_id
        )),
    }
}

fn exec_watch_rule_set_enabled(id: &str, enabled: bool) -> ExecResult {
    let intent = match WatchRuleMutationIntent::resolve(id, enabled) {
        Ok(intent) => intent,
        Err(message) => return ExecResult::Flash(message),
    };
    match crate::db::GroupDb::open()
        .and_then(|db| db.set_watch_rule_enabled(&intent.rule_id, intent.enabled))
    {
        Ok(true) => ExecResult::Flash(format!(
            "{} watch rule '{}'",
            if intent.enabled {
                "Enabled"
            } else {
                "Disabled"
            },
            intent.rule_id
        )),
        Ok(false) => {
            ExecResult::Flash(format!("unknown persisted watch rule '{}'", intent.rule_id))
        }
        Err(err) => ExecResult::Flash(format!(
            "Could not update watch rule '{}': {err}",
            intent.rule_id
        )),
    }
}

fn exec_goalies_kv(
    app: &mut crate::tui::app::App,
    args: crate::tui::filter_state::RosterKvArgs,
) -> ExecResult {
    use crate::tui::app::Screen;
    use crate::tui::filter_state::ForcedColumns;

    if let Some(sort) = args.sort.as_deref() {
        let Some(sort_idx) = goalie_sort_index(sort) else {
            return ExecResult::Flash(format!("unknown goalie sort: {sort}"));
        };
        app.goalies.sort = sort_idx;
    }

    if let Some(min_gp) = args.min_gp {
        app.goalies.min_gp = min_gp;
    }
    if let Some(country) = args.country {
        app.goalies.filters.country_filter = Some(country);
    }
    if let Some(pos) = args.pos {
        app.goalies.filters.pos_filter = pos;
    }
    if args.forced_column_keys.contains(ForcedColumns::SAVES) {
        if args.forced_columns.contains(ForcedColumns::SAVES) {
            if !app
                .goalies
                .filters
                .forced_columns
                .contains(ForcedColumns::SAVES)
            {
                app.goalies
                    .filters
                    .forced_columns
                    .toggle(ForcedColumns::SAVES);
            }
        } else if app
            .goalies
            .filters
            .forced_columns
            .contains(ForcedColumns::SAVES)
        {
            app.goalies
                .filters
                .forced_columns
                .toggle(ForcedColumns::SAVES);
        }
    }

    app.goalies.filters.invalidate();
    app.goalies.selected = 0;
    set_workspace_screen(app, Screen::Goalies);
    ExecResult::Flash("goalies filters applied".to_string())
}

fn exec_poach_kv(app: &mut crate::tui::app::App, args: PoachCommandArgs) -> ExecResult {
    use crate::tui::app::Screen;

    if let Some(positions) = args.positions {
        app.poach.positions = positions;
    }
    if let Some(categories) = args.categories {
        app.poach.categories = categories;
    }
    if let Some(availability_filter) = args.availability_filter {
        app.poach.availability_filter = availability_filter;
    }
    if let Some(candidate_kind) = args.candidate_kind {
        app.poach.candidate_kind = candidate_kind;
    }
    if let Some(limit) = args.limit {
        app.poach.limit = limit;
    }
    app.selected = 0;
    set_workspace_screen(app, Screen::Poach);
    ExecResult::Flash(format!(
        "poach filters applied: {}",
        app.poach.context_label()
    ))
}

fn exec_fantasy_sim_kv(
    app: &mut crate::tui::app::App,
    args: FantasySimulationCommandArgs,
) -> ExecResult {
    use crate::tui::app::Screen;

    if args.clear {
        app.fantasy_sim.add_player = None;
        app.fantasy_sim.drop_player = None;
    }
    if let Some(weeks) = args.weeks {
        app.fantasy_sim.weeks = weeks;
    }
    if let Some(add_player) = args.add_player {
        app.fantasy_sim.add_player = Some(add_player);
    }
    if let Some(drop_player) = args.drop_player {
        app.fantasy_sim.drop_player = Some(drop_player);
    }
    set_workspace_screen(app, Screen::FantasySim);
    ExecResult::Flash(format!(
        "fantasy simulation: {} over {} weeks",
        app.fantasy_sim.scenario_label(),
        app.fantasy_sim.weeks
    ))
}

fn exec_fantasy_gaps_kv(
    app: &mut crate::tui::app::App,
    args: FantasyGapsCommandArgs,
) -> ExecResult {
    use crate::tui::app::Screen;

    if let Some(categories) = args.categories {
        app.fantasy_gaps.categories = categories;
    }
    if let Some(limit) = args.limit {
        app.fantasy_gaps.limit = limit;
    }
    app.selected = 0;
    set_workspace_screen(app, Screen::FantasyGaps);
    ExecResult::Flash(format!(
        "fantasy gaps filters applied: {}",
        app.fantasy_gaps.context_label()
    ))
}

fn exec_depth_kv(
    app: &mut crate::tui::app::App,
    args: crate::tui::filter_state::RosterKvArgs,
) -> ExecResult {
    use crate::tui::app::Screen;

    if let Some(sort) = args.sort.as_deref() {
        return ExecResult::Flash(format!("depth: sort is not a supported kv yet: {sort}"));
    }
    if let Some(country) = args.country {
        app.depth_filters.country_filter = Some(country);
    }
    if let Some(pos) = args.pos {
        app.depth_filters.pos_filter = pos;
    }
    if let Some(min_gp) = args.min_gp {
        app.depth_filters.min_gp = min_gp;
    }
    app.depth_filters.invalidate();
    app.selected = 0;
    set_workspace_screen(app, Screen::Depth);
    ExecResult::Flash("depth filters applied".to_string())
}

fn exec_favorites_kv(
    app: &mut crate::tui::app::App,
    args: crate::tui::filter_state::RosterKvArgs,
) -> ExecResult {
    use crate::tui::app::Screen;

    if let Some(sort) = args.sort.as_deref() {
        let Some(sort) = favorites_sort(sort) else {
            return ExecResult::Flash(format!("unknown favorites sort: {sort}"));
        };
        app.favorites.sort = sort;
    }
    if let Some(country) = args.country {
        app.favorites.filters.country_filter = Some(country);
    }
    if let Some(pos) = args.pos {
        app.favorites.filters.pos_filter = pos;
    }
    if let Some(min_gp) = args.min_gp {
        app.favorites.filters.min_gp = min_gp;
    }
    app.favorites.filters.invalidate();
    app.selected = 0;
    set_workspace_screen(app, Screen::Favorites);
    ExecResult::Flash("favorites filters applied".to_string())
}

fn roster_shape_handoff(action: FantasyRosterShapeAction) -> String {
    match action {
        FantasyRosterShapeAction::Show { league } => format!(
            "fantasy roster shape: run `icelines fantasy roster-shape{}` or open `/api/v1/fantasy/roster-shape{}`",
            league
                .as_ref()
                .map(|league| format!(" --league \"{league}\""))
                .unwrap_or_default(),
            league
                .as_ref()
                .map(|league| format!("?league={}", url_component(league)))
                .unwrap_or_default()
        ),
        FantasyRosterShapeAction::Set { shape, league } => format!(
            "fantasy roster shape setup: run `icelines fantasy roster-shape-set {shape}{}`; web dashboard shape mutation is POST-deferred",
            league
                .map(|league| format!(" --league \"{league}\""))
                .unwrap_or_default()
        ),
        FantasyRosterShapeAction::Validate { league, team } => {
            let mut cli = "icelines fantasy roster-shape-validate".to_string();
            if let Some(league) = &league {
                cli.push_str(&format!(" --league \"{league}\""));
            }
            if let Some(team) = &team {
                cli.push_str(&format!(" --team \"{team}\""));
            }
            let mut route = "/api/v1/fantasy/roster-shape".to_string();
            let mut params = Vec::new();
            if let Some(league) = &league {
                params.push(format!("league={}", url_component(league)));
            }
            if let Some(team) = &team {
                params.push(format!("team={}", url_component(team)));
            }
            if !params.is_empty() {
                route.push('?');
                route.push_str(&params.join("&"));
            }
            format!("fantasy roster shape validation: run `{cli}` or open `{route}`")
        }
    }
}

fn resolve_matchup_game_id(
    app: &crate::tui::app::App,
    matchup: &str,
) -> Result<Option<u64>, String> {
    let Some((away, home)) = parse_matchup_arg(matchup) else {
        return Err(format!(
            "box: expected numeric game-id or TEAM@TEAM matchup; got {matchup:?}"
        ));
    };

    for game in loaded_score_games(app) {
        if scheduled_game_matches(&game, away, home) {
            return Ok(Some(game.game_id));
        }
    }

    for game in loaded_schedule_games(app) {
        if scheduled_game_matches(&game, away, home) {
            return Ok(Some(game.game_id));
        }
    }

    Ok(None)
}

fn parse_matchup_arg(input: &str) -> Option<(&str, &str)> {
    let (away, home) = input.split_once('@')?;
    let away = away.trim();
    let home = home.trim();
    if away.is_empty() || home.is_empty() {
        return None;
    }
    Some((away, home))
}

fn loaded_score_games(app: &crate::tui::app::App) -> Vec<icelines_fetch::nhl_api::ScheduledGame> {
    match crate::tui::tonight::lookup(&app.tonight.cache, &app.tonight.date) {
        crate::tui::tonight::TonightState::Loaded(games) => games,
        _ => Vec::new(),
    }
}

fn loaded_schedule_games(
    app: &crate::tui::app::App,
) -> Vec<icelines_fetch::nhl_api::ScheduledGame> {
    let mut games = Vec::new();
    {
        let map = app.schedule.week_cache.lock().unwrap();
        if let Some(crate::tui::schedule::ScheduleState::Loaded(week_games)) =
            map.get(&app.schedule.week)
        {
            games.extend(week_games.iter().cloned());
        }
    }
    {
        let map = app.schedule.team_cache.lock().unwrap();
        for state in map.values() {
            if let crate::tui::schedule::ScheduleState::Loaded(team_games) = state {
                games.extend(team_games.iter().cloned());
            }
        }
    }
    games
}

fn scheduled_game_matches(
    game: &icelines_fetch::nhl_api::ScheduledGame,
    away: &str,
    home: &str,
) -> bool {
    game.away_abbrev.eq_ignore_ascii_case(away) && game.home_abbrev.eq_ignore_ascii_case(home)
}

fn exec_team_kv(
    app: &mut crate::tui::app::App,
    abbrev: String,
    args: crate::tui::filter_state::RosterKvArgs,
) -> ExecResult {
    use crate::tui::app::Screen;
    use crate::tui::filter_state::ForcedColumns;

    if let Some(sort) = args.sort.as_deref() {
        let Some(sort) = team_sort(sort) else {
            return ExecResult::Flash(format!("unknown team sort: {sort}"));
        };
        app.team.sort = sort;
    }
    if let Some(country) = args.country {
        app.team.filters.country_filter = Some(country);
    }
    if let Some(pos) = args.pos {
        app.team.filters.pos_filter = pos;
    }
    if let Some(min_gp) = args.min_gp {
        app.team.filters.min_gp = min_gp;
    }
    apply_forced_column(
        &mut app.team.filters.forced_columns,
        args.forced_columns,
        args.forced_column_keys,
        ForcedColumns::HITS,
    );
    app.team.filters.invalidate();
    app.selected = 0;
    set_workspace_screen(app, Screen::Team(abbrev));
    ExecResult::Flash("team filters applied".to_string())
}

fn goalie_sort_index(raw: &str) -> Option<u8> {
    match raw.to_ascii_lowercase().as_str() {
        "save-pct" | "save_pct" | "sv%" | "svpct" | "sv-pct" => Some(0),
        "gaa" => Some(1),
        "wins" | "w" => Some(2),
        "gp" => Some(3),
        "saves" | "sv" => Some(4),
        "so" | "shutouts" => Some(5),
        _ => None,
    }
}

fn favorites_sort(raw: &str) -> Option<crate::tui::screens::favorites::FavoritesSort> {
    use crate::tui::screens::favorites::FavoritesSort;
    match raw.to_ascii_lowercase().as_str() {
        "recent" | "recently-added" | "recently_added" | "added" => {
            Some(FavoritesSort::RecentlyAdded)
        }
        "name" | "player" => Some(FavoritesSort::Name),
        "kind" | "type" => Some(FavoritesSort::Kind),
        _ => None,
    }
}

fn team_sort(raw: &str) -> Option<crate::tui::screens::team::TeamSort> {
    use crate::tui::screens::team::TeamSort;
    match raw.to_ascii_lowercase().as_str() {
        "pace" | "pts82" | "pts-82" | "pts/82" | "points" | "p" => Some(TeamSort::Pace),
        "name" => Some(TeamSort::Name),
        "pos" | "position" => Some(TeamSort::Position),
        "goals" | "g" => Some(TeamSort::Goals),
        "hits" | "h" => Some(TeamSort::Hits),
        _ => None,
    }
}

fn apply_forced_column(
    target: &mut crate::tui::filter_state::ForcedColumns,
    values: crate::tui::filter_state::ForcedColumns,
    keys: crate::tui::filter_state::ForcedColumns,
    flag: crate::tui::filter_state::ForcedColumns,
) {
    if !keys.contains(flag) {
        return;
    }
    if values.contains(flag) {
        if !target.contains(flag) {
            target.toggle(flag);
        }
    } else if target.contains(flag) {
        target.toggle(flag);
    }
}

/// Resolve a team abbrev or player needle, then upsert into the Favorites group.
fn exec_fav_add(app: &mut crate::tui::app::App, needle: &str) -> ExecResult {
    let _ = app; // App not needed for the DB call (open-by-path)
    if let Ok(abbr) = icelines_core::TeamAbbr::parse(needle) {
        return match crate::db::GroupDb::open() {
            Ok(db) => match db.add_member_kind("Favorites", &abbr.0, crate::db::MemberKind::Team) {
                Ok(true) => ExecResult::Flash(format!("★ added {} to Favorites", abbr.0)),
                Ok(false) => ExecResult::Flash(format!("★ {} is already in Favorites", abbr.0)),
                Err(e) => ExecResult::Flash(format!("DB error: {e}")),
            },
            Err(e) => ExecResult::Flash(format!("couldn't open Favorites DB: {e}")),
        };
    }

    let pid = match icelines_fetch::stats_loader::resolve_player_id_by_name(needle) {
        Some(pid) => pid,
        None => return ExecResult::Flash(format!("player not found: {needle:?}")),
    };
    // Resolve the canonical name + normalized form.
    let (full_name, normalized) = match resolve_pid_to_names(pid) {
        Some(p) => p,
        None => {
            return ExecResult::Flash(format!(
                "couldn't resolve canonical name for pid {pid} — refusing to add stub"
            ))
        }
    };
    let _ = full_name;
    match crate::db::GroupDb::open() {
        Ok(db) => match db.add_member("Favorites", &normalized) {
            Ok(true) => ExecResult::Flash(format!("★ added {needle} to Favorites")),
            Ok(false) => ExecResult::Flash(format!("★ {needle} is already in Favorites")),
            Err(e) => ExecResult::Flash(format!("DB error: {e}")),
        },
        Err(e) => ExecResult::Flash(format!("couldn't open Favorites DB: {e}")),
    }
}

fn exec_fav_remove(app: &mut crate::tui::app::App, needle: &str) -> ExecResult {
    let _ = app;
    if let Ok(abbr) = icelines_core::TeamAbbr::parse(needle) {
        return match crate::db::GroupDb::open() {
            Ok(db) => {
                match db.remove_member_kind("Favorites", &abbr.0, crate::db::MemberKind::Team) {
                    Ok(()) => ExecResult::Flash(format!("removed {} from Favorites", abbr.0)),
                    Err(e) => ExecResult::Flash(format!("DB error: {e}")),
                }
            }
            Err(e) => ExecResult::Flash(format!("couldn't open Favorites DB: {e}")),
        };
    }

    let pid = match icelines_fetch::stats_loader::resolve_player_id_by_name(needle) {
        Some(pid) => pid,
        None => return ExecResult::Flash(format!("player not found: {needle:?}")),
    };
    let (_full_name, normalized) = match resolve_pid_to_names(pid) {
        Some(p) => p,
        None => return ExecResult::Flash(format!("couldn't resolve canonical name for pid {pid}")),
    };
    match crate::db::GroupDb::open() {
        Ok(db) => match db.remove_member("Favorites", &normalized) {
            Ok(()) => ExecResult::Flash(format!("removed {needle} from Favorites")),
            Err(e) => ExecResult::Flash(format!("DB error: {e}")),
        },
        Err(e) => ExecResult::Flash(format!("couldn't open Favorites DB: {e}")),
    }
}

/// Walk bundled bios for a pid; return (full_name, normalized)
/// for the GroupDb upsert. Mirrors the resolution path used by
/// the existing player-card group-add flow.
fn resolve_pid_to_names(pid: u32) -> Option<(String, String)> {
    use icelines_fetch::bundled;
    for season_id in bundled::BUNDLED_SEASONS {
        if let Some(bios) = bundled::get_bios(season_id) {
            for b in bios {
                if b.player_id == pid {
                    let full = b.skater_full_name.clone();
                    let normalized = full.to_ascii_lowercase().replace(' ', ".");
                    return Some((full, normalized));
                }
            }
        }
        if let Some(goalies) = bundled::get_goalie_stats(season_id) {
            for g in goalies {
                if g.player_id == pid {
                    let full = g.goalie_full_name.clone();
                    let normalized = full.to_ascii_lowercase().replace(' ', ".");
                    return Some((full, normalized));
                }
            }
        }
    }
    None
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Split off the first whitespace-delimited word; return
/// (first, rest). Both are trimmed of surrounding whitespace.
fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    match s.find(char::is_whitespace) {
        None => (s, ""),
        Some(i) => (&s[..i], s[i..].trim_start()),
    }
}

fn url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn cli_handoff(command: &str, args: &str, route: &str) -> String {
    let suffix = if args.trim().is_empty() {
        String::new()
    } else {
        format!(" {}", args.trim())
    };
    format!("admin: run `icelines {command}{suffix}` or open `{route}`")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Empty / unknown ────────────────────────────────────────────────────

    #[test]
    fn l0_adams_parse_empty_is_unknown() {
        let e = parse_command("").unwrap_err();
        matches!(e, ParseError::UnknownCommand(_));
    }

    #[test]
    fn l0_adams_parse_whitespace_is_unknown() {
        let e = parse_command("   \t   ").unwrap_err();
        matches!(e, ParseError::UnknownCommand(_));
    }

    #[test]
    fn l0_adams_parse_bare_filter_rejected() {
        // forge-5: strict verb-or-slash prefix. `country=CAN`
        // alone is not a valid command — must be `query country=CAN`.
        let e = parse_command("country=CAN").unwrap_err();
        match e {
            ParseError::UnknownCommand(s) => assert_eq!(s, "country=can"),
            other => panic!("expected UnknownCommand, got {other:?}"),
        }
    }

    // ── Slash commands ─────────────────────────────────────────────────────

    #[test]
    fn l0_adams_parse_help_slash() {
        assert_eq!(parse_command("/help").unwrap(), Command::Help);
        assert_eq!(parse_command("/h").unwrap(), Command::Help);
        assert_eq!(parse_command("/?").unwrap(), Command::Help);
    }

    #[test]
    fn l0_adams_parse_quit_slash() {
        assert_eq!(parse_command("/quit").unwrap(), Command::Quit);
        assert_eq!(parse_command("/q").unwrap(), Command::Quit);
        assert_eq!(parse_command("/exit").unwrap(), Command::Quit);
    }

    #[test]
    fn l0_adams_parse_quit_bare() {
        // `q` and `quit` work without slash too (vim convention).
        assert_eq!(parse_command("quit").unwrap(), Command::Quit);
        assert_eq!(parse_command("q").unwrap(), Command::Quit);
    }

    #[test]
    fn l0_adams_parse_hide_show_panes() {
        assert_eq!(
            parse_command("/hide favorites").unwrap(),
            Command::Hide(SidePane::Favorites)
        );
        assert_eq!(
            parse_command("/hide schedule").unwrap(),
            Command::Hide(SidePane::Schedule)
        );
        assert_eq!(
            parse_command("/show favorites").unwrap(),
            Command::Show(SidePane::Favorites)
        );
        // Aliases.
        assert_eq!(
            parse_command("/hide fav").unwrap(),
            Command::Hide(SidePane::Favorites)
        );
        assert_eq!(
            parse_command("/hide sched").unwrap(),
            Command::Hide(SidePane::Schedule)
        );
    }

    #[test]
    fn l0_adams_parse_hide_unknown_pane_errors() {
        let e = parse_command("/hide foo").unwrap_err();
        matches!(e, ParseError::BadSidePane(_));
    }

    #[test]
    fn l0_adams_parse_hide_missing_pane_errors() {
        let e = parse_command("/hide").unwrap_err();
        matches!(e, ParseError::MissingArg { .. });
    }

    // ── Workspace reads (no args) ──────────────────────────────────────────

    #[test]
    fn l0_adams_parse_workspace_no_arg_verbs() {
        for (input, expected) in [
            ("stats", Command::Stats),
            ("goalies", Command::Goalies),
            ("poach", Command::Poach),
            ("admin", Command::Admin),
            ("watchlist", Command::Watchlist),
            ("transactions", Command::Transactions),
            ("txs", Command::Transactions),
            ("playoffs", Command::Playoffs),
            ("depth", Command::Depth),
            ("roster", Command::Roster),
            ("scores", Command::Scores),
            ("schedule", Command::Schedule),
            ("favorites", Command::Favorites),
        ] {
            assert_eq!(parse_command(input).unwrap(), expected, "verb {input:?}");
        }
    }

    #[test]
    fn l0_adams_parse_fantasy_roster() {
        assert_eq!(parse_command("fantasy").unwrap(), Command::Roster);
        assert_eq!(parse_command("fantasy roster").unwrap(), Command::Roster);
    }

    #[test]
    fn l0_adams_parse_watch_cmdbar_handoff() {
        assert_eq!(
            parse_command("watch Connor McDavid").unwrap(),
            Command::WatchPlayer {
                needle: "Connor McDavid".to_string(),
            }
        );
    }

    #[test]
    fn l0_tui_watch_parse_rule_editor_commands() {
        assert_eq!(
            parse_command("watch player Connor McDavid when=available").unwrap(),
            Command::WatchRulePlayer {
                player: "Connor McDavid".to_string(),
                when: "available".to_string(),
            }
        );
        assert_eq!(
            parse_command("watch enable player-connor-mcdavid").unwrap(),
            Command::WatchRuleSetEnabled {
                id: "player-connor-mcdavid".to_string(),
                enabled: true,
            }
        );
        assert_eq!(
            parse_command("watch disable player-connor-mcdavid").unwrap(),
            Command::WatchRuleSetEnabled {
                id: "player-connor-mcdavid".to_string(),
                enabled: false,
            }
        );
        let err = parse_command("watch deployment TOR").expect_err("deployment editor deferred");
        assert!(
            err.to_string()
                .contains("watch team/deployment rule editing is deferred"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn l0_adams_parse_admin_cmdbar_handoffs() {
        assert_eq!(parse_command("admin").unwrap(), Command::Admin);
        assert_eq!(
            parse_command("data status").unwrap(),
            Command::Data {
                args: "status".to_string(),
            }
        );
        assert_eq!(
            parse_command("snapshot list").unwrap(),
            Command::Snapshot {
                args: "list".to_string(),
            }
        );
        assert_eq!(
            parse_command("config list").unwrap(),
            Command::Config {
                args: "list".to_string(),
            }
        );
    }

    // ── Workspace reads (with args) ────────────────────────────────────────

    #[test]
    fn l0_parse_fantasy_gap_and_sim_workspaces() {
        assert_eq!(parse_command("gaps").unwrap(), Command::FantasyGaps);
        assert_eq!(parse_command("fantasy gaps").unwrap(), Command::FantasyGaps);
        assert_eq!(parse_command("simulate").unwrap(), Command::FantasySim);
        assert_eq!(
            parse_command("fantasy simulate").unwrap(),
            Command::FantasySim
        );
        assert_eq!(
            parse_command("fantasy daily date=2026-01-15").unwrap(),
            Command::FantasyDaily {
                date: "2026-01-15".to_string()
            }
        );
        assert_eq!(
            parse_command("fantasy matchup date=2026-01-15").unwrap(),
            Command::FantasyMatchup {
                date: "2026-01-15".to_string()
            }
        );
        assert_eq!(
            parse_command(
                "fantasy import file=C:\\exports\\rosters.csv league Office_Pool my-team My_Team dry-run"
            )
            .unwrap(),
            Command::FantasyImport {
                file: "C:\\exports\\rosters.csv".to_string(),
                league: "Office Pool".to_string(),
                my_team: Some("My Team".to_string()),
                dry_run: true,
            }
        );
        assert_eq!(
            parse_command("fantasy roster-shape validate team My_Team").unwrap(),
            Command::FantasyRosterShape {
                action: FantasyRosterShapeAction::Validate {
                    league: None,
                    team: Some("My Team".to_string()),
                },
            }
        );
        assert_eq!(
            parse_command("fantasy roster-shape set shape yahoo-standard league Office_Pool")
                .unwrap(),
            Command::FantasyRosterShape {
                action: FantasyRosterShapeAction::Set {
                    shape: "yahoo-standard".to_string(),
                    league: Some("Office Pool".to_string()),
                },
            }
        );
        assert_eq!(
            parse_command("fantasy roster-shape set yahoo-standard league Office_Pool").unwrap(),
            Command::FantasyRosterShape {
                action: FantasyRosterShapeAction::Set {
                    shape: "yahoo-standard".to_string(),
                    league: Some("Office Pool".to_string()),
                },
            }
        );
        assert_eq!(parse_command("fantasy poach").unwrap(), Command::Poach);
    }

    #[test]
    fn l0_adams_parse_fantasy_gaps_filters() {
        assert_eq!(
            parse_command("gaps cats=hits,blocks top=8").unwrap(),
            Command::FantasyGapsKv {
                args: FantasyGapsCommandArgs {
                    categories: Some(vec!["hits".to_string(), "blocks".to_string()]),
                    limit: Some(8),
                },
            }
        );
        assert_eq!(
            parse_command("fantasy gaps shots limit=6").unwrap(),
            Command::FantasyGapsKv {
                args: FantasyGapsCommandArgs {
                    categories: Some(vec!["shots".to_string()]),
                    limit: Some(6),
                },
            }
        );
    }

    #[test]
    fn l0_adams_parse_career_cmdbar_handoff() {
        assert_eq!(
            parse_command("career league=OHL season=20142015 top=8 sort=goals").unwrap(),
            Command::Career {
                args: CareerCommandArgs {
                    league: "OHL".to_string(),
                    season: Some(20142015),
                    top: 8,
                    sort: "goals".to_string(),
                },
            }
        );
        assert_eq!(
            parse_command("career whl").unwrap(),
            Command::Career {
                args: CareerCommandArgs {
                    league: "WHL".to_string(),
                    ..CareerCommandArgs::default()
                },
            }
        );
    }

    #[test]
    fn l0_adams_parse_report_cmdbar_handoff() {
        assert_eq!(
            parse_command("report weekly cats=shots,hits availability=imported-available top=12")
                .unwrap(),
            Command::Report {
                args: ReportCommandArgs {
                    kind: ReportKind::Weekly,
                    categories: vec!["shots".to_string(), "hits".to_string()],
                    availability: Some("imported-available".to_string()),
                    top: Some(12),
                },
            }
        );
        assert_eq!(
            parse_command("reports poach blocks").unwrap(),
            Command::Report {
                args: ReportCommandArgs {
                    kind: ReportKind::Poach,
                    categories: vec!["blocks".to_string()],
                    availability: None,
                    top: None,
                },
            }
        );
    }

    #[test]
    fn l0_adams_parse_records_cmdbar_handoff() {
        assert_eq!(
            parse_command("records player Andre Burakovsky").unwrap(),
            Command::Records {
                args: RecordsCommandArgs {
                    target: RecordsTarget::Player,
                    subject: "Andre Burakovsky".to_string(),
                },
            }
        );
        assert_eq!(
            parse_command("records team edm").unwrap(),
            Command::Records {
                args: RecordsCommandArgs {
                    target: RecordsTarget::Team,
                    subject: "edm".to_string(),
                },
            }
        );
    }

    #[test]
    fn l0_profile_parse_awards_cmdbar() {
        assert_eq!(
            parse_command("awards player Connor McDavid").unwrap(),
            Command::Awards {
                player: "Connor McDavid".to_string()
            }
        );
        assert_eq!(
            parse_command("trophy Connor McDavid").unwrap(),
            Command::Awards {
                player: "Connor McDavid".to_string()
            }
        );
    }

    #[test]
    fn l0_profile_parse_streaks_cmdbar() {
        assert_eq!(
            parse_command("streaks player Connor McDavid").unwrap(),
            Command::Streaks {
                player: "Connor McDavid".to_string()
            }
        );
        assert_eq!(
            parse_command("streak Connor McDavid").unwrap(),
            Command::Streaks {
                player: "Connor McDavid".to_string()
            }
        );
    }

    #[test]
    fn l0_profile_parse_player_hub_handoffs() {
        assert_eq!(
            parse_command("scouting player Connor McDavid").unwrap(),
            Command::Scouting {
                player: "Connor McDavid".to_string()
            }
        );
        assert_eq!(
            parse_command("mates Connor McDavid").unwrap(),
            Command::Mates {
                player: "Connor McDavid".to_string()
            }
        );
        assert_eq!(
            parse_command("linemates Connor McDavid").unwrap(),
            Command::Mates {
                player: "Connor McDavid".to_string()
            }
        );
        let err = parse_command("deployment Connor McDavid").expect_err("deployment is not mates");
        assert!(
            err.to_string().contains("unknown command"),
            "deployment alias should not silently map to mates: {err}"
        );
    }

    #[test]
    fn l0_adams_fantasy_cmdbar_examples_are_documented() {
        const COMMANDS_MD: &str = include_str!("../../../COMMANDS.md");
        for example in [
            "gaps cats=hits,blocks,shots top=8",
            "poach rw cats=hits,blocks free top=12",
            "simulate add=Connor_McDavid drop=Bench_Forward weeks=3",
            "Fantasy screen shortcuts",
            "Apply draft-year query, swap to Queries",
            "roster",
            "class 2024",
            "career league=OHL season=20142015 top=8",
            "report weekly cats=shots,hits top=12",
            "records player Andre Burakovsky",
            "records team SEA",
            "watch Connor McDavid",
            "data status",
            "snapshot list",
            "config list",
            "compare McDavid vs Crosby",
            "box EDM@BOS",
        ] {
            assert!(
                COMMANDS_MD.contains(example),
                "COMMANDS.md must document fantasy cmdbar example {example:?}"
            );
        }
    }

    #[test]
    fn l0_adams_parse_fantasy_simulation_scenario() {
        assert_eq!(
            parse_command("simulate add=Connor McDavid drop=Bench Forward weeks=3").unwrap(),
            Command::FantasySimKv {
                args: FantasySimulationCommandArgs {
                    weeks: Some(3),
                    add_player: Some("Connor McDavid".to_string()),
                    drop_player: Some("Bench Forward".to_string()),
                    clear: false,
                },
            }
        );
        assert_eq!(
            parse_command("fantasy simulate add Connor_McDavid drop Bench Forward").unwrap(),
            Command::FantasySimKv {
                args: FantasySimulationCommandArgs {
                    weeks: None,
                    add_player: Some("Connor McDavid".to_string()),
                    drop_player: Some("Bench Forward".to_string()),
                    clear: false,
                },
            }
        );
        assert_eq!(
            parse_command("simulate clear").unwrap(),
            Command::FantasySimKv {
                args: FantasySimulationCommandArgs {
                    clear: true,
                    ..Default::default()
                },
            }
        );
    }

    #[test]
    fn l0_adams_parse_poach_filters() {
        assert_eq!(
            parse_command("poach rw cats=hits,blocks free top=12").unwrap(),
            Command::PoachKv {
                args: PoachCommandArgs {
                    positions: Some(vec![Position::RightWing]),
                    categories: Some(vec!["hits".to_string(), "blocks".to_string()]),
                    availability_filter: Some(PoachAvailabilityFilter::ImportedAvailable),
                    candidate_kind: None,
                    limit: Some(12),
                },
            }
        );
        assert_eq!(
            parse_command("fantasy poach top=8 available").unwrap(),
            Command::PoachKv {
                args: PoachCommandArgs {
                    positions: None,
                    categories: None,
                    availability_filter: Some(PoachAvailabilityFilter::Available),
                    candidate_kind: None,
                    limit: Some(8),
                },
            }
        );
        assert_eq!(
            parse_command("poach pos=f kind=streamer").unwrap(),
            Command::PoachKv {
                args: PoachCommandArgs {
                    positions: Some(vec![
                        Position::Center,
                        Position::LeftWing,
                        Position::RightWing,
                    ]),
                    categories: None,
                    availability_filter: None,
                    candidate_kind: Some(PoachCandidateKind::Streamer),
                    limit: None,
                },
            }
        );
    }

    #[test]
    fn l0_adams_parse_poach_unknown_filter_errors() {
        let e = parse_command("poach contract-year").unwrap_err();
        match e {
            ParseError::BadFilter { details } => {
                assert!(details.contains("unknown filter token"));
            }
            other => panic!("expected BadFilter, got {other:?}"),
        }
    }

    #[test]
    fn l0_adams_parse_player() {
        assert_eq!(
            parse_command("player Bedard").unwrap(),
            Command::PlayerCard {
                needle: "Bedard".into(),
            }
        );
        assert_eq!(
            parse_command("p Connor McDavid").unwrap(),
            Command::PlayerCard {
                needle: "Connor McDavid".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_player_missing_arg() {
        let e = parse_command("player").unwrap_err();
        matches!(
            e,
            ParseError::MissingArg {
                command: "player",
                ..
            }
        );
    }

    #[test]
    fn l0_adams_parse_team_uppercases() {
        assert_eq!(
            parse_command("team edm").unwrap(),
            Command::Team {
                abbrev: "EDM".into(),
            }
        );
        assert_eq!(
            parse_command("t bos").unwrap(),
            Command::Team {
                abbrev: "BOS".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_team_season_distinct_variant() {
        assert_eq!(
            parse_command("team edm season").unwrap(),
            Command::TeamSeason {
                abbrev: "EDM".into(),
            }
        );
        assert_eq!(
            parse_command("team edm schedule").unwrap(),
            Command::TeamSeason {
                abbrev: "EDM".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_team_missing_arg() {
        let e = parse_command("team").unwrap_err();
        matches!(
            e,
            ParseError::MissingArg {
                command: "team",
                ..
            }
        );
    }

    #[test]
    fn l0_adams_parse_compare_two_args() {
        // " vs " separator.
        assert_eq!(
            parse_command("compare McDavid vs Crosby").unwrap(),
            Command::Compare {
                left: "McDavid".into(),
                right: Some("Crosby".into()),
            }
        );
        // Comma separator.
        assert_eq!(
            parse_command("compare McDavid, Crosby").unwrap(),
            Command::Compare {
                left: "McDavid".into(),
                right: Some("Crosby".into()),
            }
        );
    }

    #[test]
    fn l0_adams_parse_compare_one_arg_similarity() {
        assert_eq!(
            parse_command("compare McDavid").unwrap(),
            Command::Compare {
                left: "McDavid".into(),
                right: None,
            }
        );
    }

    #[test]
    fn l0_adams_parse_compare_alias_vs() {
        // `vs` as a verb is also accepted.
        assert_eq!(
            parse_command("vs McDavid Crosby").unwrap(),
            Command::Compare {
                left: "McDavid Crosby".into(),
                right: None,
            }
        );
    }

    #[test]
    fn l0_adams_parse_box() {
        assert_eq!(
            parse_command("box edm@bos").unwrap(),
            Command::Box {
                game: "edm@bos".into(),
            }
        );
        assert_eq!(
            parse_command("boxscore 2025020100").unwrap(),
            Command::Box {
                game: "2025020100".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_box_missing_arg() {
        let e = parse_command("box").unwrap_err();
        matches!(e, ParseError::MissingArg { command: "box", .. });
    }

    #[test]
    fn l0_adams_parse_class() {
        assert_eq!(
            parse_command("class 2015").unwrap(),
            Command::Class { year: 2015 }
        );
    }

    #[test]
    fn l0_adams_parse_class_bad_integer() {
        let e = parse_command("class abc").unwrap_err();
        match e {
            ParseError::BadInteger { command, .. } => assert_eq!(command, "class"),
            other => panic!("expected BadInteger; got {other:?}"),
        }
    }

    // ── Write actions ─────────────────────────────────────────────────────

    #[test]
    fn l0_adams_parse_fav_add() {
        // Verb form.
        assert_eq!(
            parse_command("fav add Bedard").unwrap(),
            Command::FavAdd {
                needle: "Bedard".into(),
            }
        );
        // Slash form.
        assert_eq!(
            parse_command("/fav add Bedard").unwrap(),
            Command::FavAdd {
                needle: "Bedard".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_fav_remove_aliases() {
        assert_eq!(
            parse_command("fav remove Bedard").unwrap(),
            Command::FavRemove {
                needle: "Bedard".into(),
            }
        );
        assert_eq!(
            parse_command("fav rm Bedard").unwrap(),
            Command::FavRemove {
                needle: "Bedard".into(),
            }
        );
        assert_eq!(
            parse_command("/fav del Bedard").unwrap(),
            Command::FavRemove {
                needle: "Bedard".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_fav_add_missing_player() {
        let e = parse_command("fav add").unwrap_err();
        matches!(
            e,
            ParseError::MissingArg {
                command: "/fav add",
                ..
            }
        );
    }

    #[test]
    fn l0_adams_parse_fav_missing_subcommand() {
        let e = parse_command("fav").unwrap_err();
        matches!(
            e,
            ParseError::MissingArg {
                command: "/fav",
                ..
            }
        );
    }

    // ── Query (Phase Art Ross delegation) ─────────────────────────────────

    #[test]
    fn l0_adams_parse_query_valid_filter() {
        assert_eq!(
            parse_command("query country=CAN").unwrap(),
            Command::Query {
                filter: "country=CAN".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_query_compound_filter() {
        let cmd = parse_command("query country=CAN AND age<25 AND p>=20").unwrap();
        assert_eq!(
            cmd,
            Command::Query {
                filter: "country=CAN AND age<25 AND p>=20".into(),
            }
        );
    }

    #[test]
    fn l0_adams_parse_query_bad_filter() {
        let e = parse_command("query (((").unwrap_err();
        matches!(e, ParseError::BadFilter { .. });
    }

    #[test]
    fn l0_adams_parse_query_missing_arg() {
        let e = parse_command("query").unwrap_err();
        matches!(
            e,
            ParseError::MissingArg {
                command: "query",
                ..
            }
        );
    }

    // ── Display impl ──────────────────────────────────────────────────────

    #[test]
    fn l0_adams_parse_error_display_is_user_friendly() {
        let cases = vec![
            (ParseError::UnknownCommand("xyz".into()), "unknown command"),
            (ParseError::UnknownCommand(String::new()), "(empty input)"),
            (
                ParseError::MissingArg {
                    command: "team",
                    arg: "abbrev",
                },
                "missing",
            ),
            (
                ParseError::BadFilter {
                    details: "syntax".into(),
                },
                "filter parse error",
            ),
            (
                ParseError::BadInteger {
                    command: "class",
                    raw: "abc".into(),
                },
                "not a valid integer",
            ),
            (ParseError::BadSidePane("foo".into()), "unknown pane"),
        ];
        for (err, expected_substr) in cases {
            let msg = err.to_string();
            assert!(
                msg.contains(expected_substr),
                "ParseError {err:?} display {msg:?} should contain {expected_substr:?}"
            );
        }
    }

    // ── Case insensitivity ────────────────────────────────────────────────

    #[test]
    fn l0_adams_parse_verb_case_insensitive() {
        assert_eq!(parse_command("GOALIES").unwrap(), Command::Goalies);
        assert_eq!(parse_command("Stats").unwrap(), Command::Stats);
        assert_eq!(parse_command("Help").unwrap(), Command::Help);
    }

    #[test]
    fn l0_messier_6_parse_stats_kv_lowers_to_query_filter() {
        assert_eq!(
            parse_command("stats nationality=CAN pos=LW min-gp=20").unwrap(),
            Command::Query {
                filter: "nationality=CAN AND pos=LW AND gp>=20".to_string()
            }
        );
    }

    #[test]
    fn l0_messier_6_parse_goalies_kv_is_typed() {
        let cmd = parse_command("goalies sort=gaa min-gp=20 nationality=CAN saves=on").unwrap();
        let Command::GoaliesKv { args } = cmd else {
            panic!("expected GoaliesKv");
        };
        assert_eq!(args.sort.as_deref(), Some("gaa"));
        assert_eq!(args.min_gp, Some(20));
        assert_eq!(
            args.country,
            Some(crate::tui::filter_state::CountryCode::CAN)
        );
        assert!(args
            .forced_columns
            .contains(crate::tui::filter_state::ForcedColumns::SAVES));
        assert!(args
            .forced_column_keys
            .contains(crate::tui::filter_state::ForcedColumns::SAVES));
    }

    #[test]
    fn l0_messier_6_parse_depth_favorites_and_team_kv_are_typed() {
        assert!(matches!(
            parse_command("depth pos=LW nationality=CAN min-gp=10").unwrap(),
            Command::DepthKv { .. }
        ));
        assert!(matches!(
            parse_command("favorites sort=name nationality=CAN").unwrap(),
            Command::FavoritesKv { .. }
        ));
        match parse_command("team EDM pos=LW nationality=CAN hits=on").unwrap() {
            Command::TeamKv { abbrev, args } => {
                assert_eq!(abbrev, "EDM");
                assert_eq!(args.pos, Some(crate::tui::filter_state::PosFilter::LW));
                assert_eq!(
                    args.country,
                    Some(crate::tui::filter_state::CountryCode::CAN)
                );
                assert!(args
                    .forced_columns
                    .contains(crate::tui::filter_state::ForcedColumns::HITS));
            }
            other => panic!("expected TeamKv, got {other:?}"),
        }
    }

    // ── split_first_word helper ───────────────────────────────────────────

    #[test]
    fn l0_adams_split_first_word_basic() {
        assert_eq!(split_first_word("foo bar baz"), ("foo", "bar baz"));
        assert_eq!(split_first_word("solo"), ("solo", ""));
        assert_eq!(
            split_first_word("  leading  trailing  "),
            ("leading", "trailing  ")
        );
        assert_eq!(split_first_word(""), ("", ""));
    }

    // ── Executor tests (Adams.2) ──────────────────────────────────────────
    //
    // The executor mutates an `App` directly. These tests exercise the
    // pure decisions: which `Screen` variant gets selected, which
    // `mdi.show_*` flag toggles, which `ExecResult` flavor returns.
    //
    // Most executor tests avoid the GroupDb. Watch-rule editor tests use a
    // temp HOME because the feature is explicitly a persistence adapter.

    use crate::tui::app::{App, Screen};
    use crate::tui::mdi::MdiLayout;

    fn fresh_app_with_mdi() -> App {
        let mut app = App::new(true);
        app.mdi = Some(MdiLayout::default());
        app
    }

    fn scheduled_game(id: u64, away: &str, home: &str) -> icelines_fetch::nhl_api::ScheduledGame {
        icelines_fetch::nhl_api::ScheduledGame {
            game_id: id,
            date: "2026-05-12".to_string(),
            game_type: 2,
            away_abbrev: away.to_string(),
            away_name: away.to_string(),
            home_abbrev: home.to_string(),
            home_name: home.to_string(),
            start_time_utc: "2026-05-12T23:00:00Z".to_string(),
            away_score: None,
            home_score: None,
            game_state: Some("FUT".to_string()),
            last_period: None,
            series_game: None,
            away_wins: None,
            home_wins: None,
        }
    }

    #[test]
    fn l0_adams_exec_help_sets_show_help() {
        let mut app = fresh_app_with_mdi();
        assert!(!app.show_help);
        let r = execute_command(Command::Help, &mut app);
        assert!(matches!(r, ExecResult::Continue));
        assert!(app.show_help);
    }

    #[test]
    fn l0_adams_exec_quit_returns_quit() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(Command::Quit, &mut app);
        assert!(matches!(r, ExecResult::Quit));
    }

    #[test]
    fn l0_adams_exec_hide_favorites_clears_flag() {
        let mut app = fresh_app_with_mdi();
        assert!(app.mdi.as_ref().unwrap().show_favorites);
        let r = execute_command(Command::Hide(SidePane::Favorites), &mut app);
        assert!(matches!(r, ExecResult::Continue));
        assert!(!app.mdi.as_ref().unwrap().show_favorites);
    }

    #[test]
    fn l0_adams_exec_hide_schedule_clears_flag() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(Command::Hide(SidePane::Schedule), &mut app);
        assert!(matches!(r, ExecResult::Continue));
        assert!(!app.mdi.as_ref().unwrap().show_schedule);
    }

    #[test]
    fn l0_adams_exec_show_favorites_after_hide_restores() {
        let mut app = fresh_app_with_mdi();
        app.mdi.as_mut().unwrap().show_favorites = false;
        let r = execute_command(Command::Show(SidePane::Favorites), &mut app);
        assert!(matches!(r, ExecResult::Continue));
        assert!(app.mdi.as_ref().unwrap().show_favorites);
    }

    #[test]
    fn l0_adams_exec_hide_in_sdi_is_noop_continue() {
        // No mdi attached — Hide should still parse & "continue"
        // without panicking. The mutation just vanishes.
        let mut app = App::new(true);
        let r = execute_command(Command::Hide(SidePane::Favorites), &mut app);
        assert!(matches!(r, ExecResult::Continue));
        assert!(app.mdi.is_none());
    }

    #[test]
    fn l0_adams_exec_screen_swaps() {
        let cases: Vec<(Command, Screen)> = vec![
            (Command::Stats, Screen::Queries),
            (Command::Goalies, Screen::Goalies),
            (Command::Poach, Screen::Poach),
            (
                Command::Watchlist,
                Screen::GroupDetail("Watchlist".to_string()),
            ),
            (Command::Transactions, Screen::Transactions),
            (Command::Playoffs, Screen::Playoffs),
            (Command::Depth, Screen::Depth),
            (Command::Scores, Screen::Tonight),
            (Command::Schedule, Screen::Schedule),
            (Command::Favorites, Screen::Favorites),
        ];
        for (cmd, expected) in cases {
            let mut app = fresh_app_with_mdi();
            let r = execute_command(cmd.clone(), &mut app);
            assert!(
                matches!(r, ExecResult::Continue),
                "expected Continue for {cmd:?}"
            );
            assert_eq!(
                std::mem::discriminant(&app.screen),
                std::mem::discriminant(&expected),
                "{cmd:?} should land on {expected:?}, got {:?}",
                app.screen
            );
        }
    }

    #[test]
    fn l0_mdi_exec_workspace_swap_applies_bound_experience() {
        let mut app = fresh_app_with_mdi();

        let r = execute_command(Command::Stats, &mut app);

        assert!(matches!(r, ExecResult::Continue));
        let mdi = app.mdi.as_ref().expect("mdi attached");
        assert_eq!(
            mdi.active_experience,
            Some(icelines_core::WorkbenchExperienceId::ScoringRoom)
        );
        assert_eq!(
            mdi.left_pane_binding,
            icelines_core::WorkbenchPaneBindingId::SavedQueriesLeft
        );
        assert_eq!(
            mdi.right_pane_binding,
            icelines_core::WorkbenchPaneBindingId::StatFilterRight
        );
        assert_eq!(
            mdi.selected_workbench_id(),
            Some(icelines_core::WorkbenchId::Stats)
        );

        let r = execute_command(Command::Goalies, &mut app);

        assert!(matches!(r, ExecResult::Continue));
        let mdi = app.mdi.as_ref().unwrap();
        assert!(
            mdi.active_experience.is_none(),
            "workspace swaps without a bound room must clear stale room context"
        );
        assert_eq!(
            mdi.selected_workbench_id(),
            Some(icelines_core::WorkbenchId::Goalies)
        );
    }

    #[test]
    fn l0_mdi_hide_command_moves_focus_off_hidden_pane() {
        let mut app = fresh_app_with_mdi();
        app.mdi.as_mut().unwrap().focus = crate::tui::mdi::MdiFocus::RightPane;

        let r = execute_command(Command::Hide(SidePane::Schedule), &mut app);

        assert!(matches!(r, ExecResult::Continue));
        let mdi = app.mdi.as_ref().unwrap();
        assert!(!mdi.show_schedule);
        assert_eq!(mdi.focus, crate::tui::mdi::MdiFocus::Workspace);
    }

    #[test]
    fn l0_messier_6_exec_goalies_kv_applies_filters() {
        let mut app = fresh_app_with_mdi();
        let cmd = parse_command("goalies sort=gaa min-gp=20 nationality=CAN saves=on").unwrap();
        let r = execute_command(cmd, &mut app);

        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::Goalies));
        assert_eq!(app.goalies.sort, 1);
        assert_eq!(app.goalies.min_gp, 20);
        assert_eq!(
            app.goalies.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::CAN)
        );
        assert!(app
            .goalies
            .filters
            .forced_columns
            .contains(crate::tui::filter_state::ForcedColumns::SAVES));
    }

    #[test]
    fn l0_messier_6_exec_goalies_kv_unknown_sort_flashes_without_mutation() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            Command::GoaliesKv {
                args: crate::tui::filter_state::RosterKvArgs {
                    sort: Some("weird".to_string()),
                    min_gp: Some(20),
                    ..Default::default()
                },
            },
            &mut app,
        );

        assert!(matches!(r, ExecResult::Flash(_)));
        assert_ne!(app.goalies.min_gp, 20);
    }

    #[test]
    fn l0_adams_exec_career_cmdbar_handoff_flashes_targets() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("career league=OHL season=20142015 top=8").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("career handoff should flash canonical targets");
        };
        assert!(message.contains("icelines query career --league OHL --season 20142015"));
        assert!(message.contains("/career?league=OHL"));
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn l0_adams_exec_report_cmdbar_handoff_flashes_targets() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("report weekly cats=shots,hits top=12").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("report handoff should flash canonical targets");
        };
        assert!(message.contains("icelines report weekly --category shots,hits --top 12"));
        assert!(message.contains("/reports/weekly?category=shots%2Chits&top=12"));
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn l0_adams_exec_watch_cmdbar_handoff_flashes_targets() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(parse_command("watch Connor McDavid").unwrap(), &mut app);

        let ExecResult::Flash(message) = r else {
            panic!("watch handoff should flash canonical targets");
        };
        assert!(message.contains("icelines watch note \"Connor McDavid\""));
        assert!(message.contains("icelines watch player \"Connor McDavid\" --when pp1 --save"));
        assert!(message.contains("/watchlist"));
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn l1_tui_watch_cmdbar_rule_editor_persists_and_toggles_without_erasing_history() {
        let dir = tempfile::TempDir::new().unwrap();
        crate::db::with_test_home(dir.path(), || {
            let mut app = fresh_app_with_mdi();

            let r = execute_command(
                parse_command("watch player Matthew Knies when=available").unwrap(),
                &mut app,
            );
            let ExecResult::Flash(message) = r else {
                panic!("watch player should flash saved rule");
            };
            assert!(message.contains("Saved watch rule 'player-matthew-knies'"));

            let db = crate::db::GroupDb::open().expect("open group db");
            let rules = db.list_watch_rules().expect("list watch rules");
            assert_eq!(rules.len(), 1);
            assert_eq!(rules[0].id, "player-matthew-knies");
            assert!(rules[0].enabled);

            let r = execute_command(
                parse_command("watch disable player-matthew-knies").unwrap(),
                &mut app,
            );
            let ExecResult::Flash(message) = r else {
                panic!("watch disable should flash mutation result");
            };
            assert!(message.contains("Disabled watch rule 'player-matthew-knies'"));
            assert!(!db.list_watch_rules().expect("list rules after disable")[0].enabled);

            db.record_watch_rule_event(
                "player-matthew-knies",
                Some("player:matthew-knies"),
                "Knies became available",
            )
            .expect("record watch event");

            let r = execute_command(
                parse_command("watch enable player-matthew-knies").unwrap(),
                &mut app,
            );
            let ExecResult::Flash(message) = r else {
                panic!("watch enable should flash mutation result");
            };
            assert!(message.contains("Enabled watch rule 'player-matthew-knies'"));
            assert!(db.list_watch_rules().expect("list rules after enable")[0].enabled);
            assert_eq!(
                db.list_watch_rule_events(10)
                    .expect("list history after toggle")
                    .len(),
                1,
                "TUI rule toggles must not erase fired-alert history"
            );
        });
    }

    #[test]
    fn l0_adams_exec_admin_and_operational_cmdbar_handoffs() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(Command::Admin, &mut app);
        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(app.show_admin);

        for (input, expected) in [
            ("data status", "icelines data status"),
            ("snapshot list", "icelines snapshot list"),
            ("config list", "icelines config list"),
        ] {
            let r = execute_command(parse_command(input).unwrap(), &mut app);
            let ExecResult::Flash(message) = r else {
                panic!("{input} should flash canonical target");
            };
            assert!(message.contains(expected), "{message}");
            assert!(message.contains("/admin"), "{message}");
        }
    }

    #[test]
    fn l0_mates_cmdbar_handoff_reports_shift_lock() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(parse_command("mates Connor McDavid").unwrap(), &mut app);
        let ExecResult::Flash(message) = r else {
            panic!("mates should flash canonical handoff");
        };

        assert!(
            message.contains("icelines mates \"Connor McDavid\""),
            "{message}"
        );
        assert!(message.contains("roster fallback"), "{message}");
        assert!(message.contains("shifts locked off"), "{message}");
        assert!(!message.contains("linemates/deployment"), "{message}");
    }

    #[test]
    fn l0_adams_exec_poach_kv_applies_filters() {
        let mut app = fresh_app_with_mdi();
        app.selected = 5;
        let r = execute_command(
            parse_command("poach pos=rw cat=hits,blocks free kind=streamer top=11").unwrap(),
            &mut app,
        );

        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::Poach));
        assert_eq!(app.selected, 0);
        assert_eq!(app.poach.positions, vec![Position::RightWing]);
        assert_eq!(
            app.poach.categories,
            vec!["hits".to_string(), "blocks".to_string()]
        );
        assert_eq!(
            app.poach.availability_filter,
            PoachAvailabilityFilter::ImportedAvailable
        );
        assert_eq!(app.poach.candidate_kind, PoachCandidateKind::Streamer);
        assert_eq!(app.poach.limit, 11);
    }

    #[test]
    fn l0_adams_exec_fantasy_sim_kv_applies_scenario() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("simulate add=Connor McDavid drop=Bench Forward weeks=2").unwrap(),
            &mut app,
        );

        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::FantasySim));
        assert_eq!(app.fantasy_sim.weeks, 2);
        assert_eq!(
            app.fantasy_sim.add_player.as_deref(),
            Some("Connor McDavid")
        );
        assert_eq!(
            app.fantasy_sim.drop_player.as_deref(),
            Some("Bench Forward")
        );

        let r = execute_command(parse_command("simulate clear").unwrap(), &mut app);
        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(app.fantasy_sim.add_player.is_none());
        assert!(app.fantasy_sim.drop_player.is_none());
        assert_eq!(app.fantasy_sim.weeks, 2);
    }

    #[test]
    fn l0_adams_exec_fantasy_gaps_kv_applies_filters() {
        let mut app = fresh_app_with_mdi();
        app.selected = 3;
        let r = execute_command(
            parse_command("gaps cats=hits,blocks top=7").unwrap(),
            &mut app,
        );

        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::FantasyGaps));
        assert_eq!(app.selected, 0);
        assert_eq!(
            app.fantasy_gaps.categories,
            vec!["hits".to_string(), "blocks".to_string()]
        );
        assert_eq!(app.fantasy_gaps.limit, 7);
    }

    #[test]
    fn l0_adams_exec_fantasy_daily_hands_off_read_surface() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("fantasy daily date=2026-01-15").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("daily command should return a handoff flash");
        };
        assert!(message.contains("icelines fantasy daily --date 2026-01-15"));
        assert!(message.contains("/api/v1/fantasy/daily?date=2026-01-15"));
    }

    #[test]
    fn l0_adams_exec_fantasy_matchup_hands_off_read_surface() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("fantasy matchup date=2026-01-15").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("matchup command should return a handoff flash");
        };
        assert!(message.contains("icelines fantasy matchup --date 2026-01-15"));
        assert!(message.contains("/api/v1/fantasy/matchup?date=2026-01-15"));
    }

    #[test]
    fn l0_adams_exec_fantasy_import_hands_off_cli_surface() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("fantasy import file=rosters.csv league Office_Pool dry-run").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("import command should return a handoff flash");
        };
        assert!(message.contains("icelines fantasy import-yahoo --file rosters.csv"));
        assert!(message.contains("--league \"Office Pool\""));
        assert!(message.contains("--dry-run"));
        assert!(message.contains("POST-deferred"));
    }

    #[test]
    fn l0_adams_exec_fantasy_roster_shape_hands_off_cli_and_api_surfaces() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("fantasy roster-shape validate team My_Team").unwrap(),
            &mut app,
        );
        let ExecResult::Flash(message) = r else {
            panic!("roster-shape validate command should return a handoff flash");
        };
        assert!(message.contains("icelines fantasy roster-shape-validate"));
        assert!(message.contains("--team \"My Team\""));
        assert!(message.contains("/api/v1/fantasy/roster-shape?team=My+Team"));

        let r = execute_command(
            parse_command("fantasy roster-shape set shape yahoo-standard").unwrap(),
            &mut app,
        );
        let ExecResult::Flash(message) = r else {
            panic!("roster-shape set command should return a handoff flash");
        };
        assert!(message.contains("icelines fantasy roster-shape-set yahoo-standard"));
        assert!(message.contains("web dashboard shape mutation"));
    }

    #[test]
    fn l0_messier_6_exec_depth_kv_applies_filters() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("depth pos=LW nationality=CAN min-gp=10").unwrap(),
            &mut app,
        );

        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::Depth));
        assert_eq!(
            app.depth_filters.pos_filter,
            crate::tui::filter_state::PosFilter::LW
        );
        assert_eq!(
            app.depth_filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::CAN)
        );
        assert_eq!(app.depth_filters.min_gp, 10);
    }

    #[test]
    fn l0_messier_6_exec_favorites_kv_applies_filters() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("favorites sort=name pos=F nationality=CAN").unwrap(),
            &mut app,
        );

        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::Favorites));
        assert_eq!(
            app.favorites.sort,
            crate::tui::screens::favorites::FavoritesSort::Name
        );
        assert_eq!(
            app.favorites.filters.pos_filter,
            crate::tui::filter_state::PosFilter::Forwards
        );
        assert_eq!(
            app.favorites.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::CAN)
        );
    }

    #[test]
    fn l0_messier_6_exec_team_kv_applies_filters() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("team EDM sort=hits pos=D nationality=CAN hits=on").unwrap(),
            &mut app,
        );

        assert!(matches!(r, ExecResult::Flash(_)));
        match &app.screen {
            Screen::Team(abbrev) => assert_eq!(abbrev, "EDM"),
            other => panic!("expected Team screen, got {other:?}"),
        }
        assert_eq!(app.team.sort, crate::tui::screens::team::TeamSort::Hits);
        assert_eq!(
            app.team.filters.pos_filter,
            crate::tui::filter_state::PosFilter::Defense
        );
        assert_eq!(
            app.team.filters.country_filter,
            Some(crate::tui::filter_state::CountryCode::CAN)
        );
        assert!(app
            .team
            .filters
            .forced_columns
            .contains(crate::tui::filter_state::ForcedColumns::HITS));
    }

    #[test]
    fn l0_adams_exec_team_lands_on_team_screen() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            Command::Team {
                abbrev: "EDM".to_string(),
            },
            &mut app,
        );
        assert!(matches!(r, ExecResult::Continue));
        match &app.screen {
            Screen::Team(abbr) => assert_eq!(abbr, "EDM"),
            other => panic!("expected Team screen, got {other:?}"),
        }
    }

    #[test]
    fn l0_adams_exec_team_season_lands_on_schedule_team() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            Command::TeamSeason {
                abbrev: "BOS".to_string(),
            },
            &mut app,
        );
        assert!(matches!(r, ExecResult::Continue));
        match &app.screen {
            Screen::ScheduleTeam(abbr) => assert_eq!(abbr, "BOS"),
            other => panic!("expected ScheduleTeam screen, got {other:?}"),
        }
    }

    #[test]
    fn l0_adams_exec_compare_head_to_head_handoff_flashes_targets() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("compare Connor McDavid vs Sidney Crosby").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("head-to-head compare should flash canonical targets");
        };
        assert!(message.contains("icelines query compare \"Connor McDavid\" \"Sidney Crosby\""));
        assert!(message.contains("/compare?left=Connor+McDavid&right=Sidney+Crosby"));
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn l0_adams_exec_records_team_handoff_flashes_cli_and_web() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(parse_command("records team edm").unwrap(), &mut app);

        let ExecResult::Flash(message) = r else {
            panic!("records team should flash canonical targets");
        };
        assert!(message.contains("icelines records team EDM"));
        assert!(message.contains("/records/team/EDM"));
        assert!(matches!(app.screen, Screen::Home));
    }

    #[test]
    fn l0_profile_exec_records_player_opens_tui_records_screen() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("records player Connor McDavid").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("records player should flash canonical CLI target");
        };
        assert!(message.contains("icelines records player \"Connor McDavid\""));
        assert!(matches!(app.screen, Screen::PlayerRecordsById(_)));
    }

    #[test]
    fn l0_profile_exec_awards_player_opens_tui_awards_screen() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            parse_command("awards player Connor McDavid").unwrap(),
            &mut app,
        );

        let ExecResult::Flash(message) = r else {
            panic!("awards player should flash canonical CLI target");
        };
        assert!(message.contains("icelines awards \"Connor McDavid\""));
        assert!(matches!(app.screen, Screen::PlayerAwardsById(_)));
    }

    #[test]
    fn l0_adams_exec_box_numeric_lands_on_game_detail() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            Command::Box {
                game: "2025020001".to_string(),
            },
            &mut app,
        );
        assert!(matches!(r, ExecResult::Continue));
        match &app.screen {
            Screen::GameDetail(id) => assert_eq!(*id, 2_025_020_001),
            other => panic!("expected GameDetail screen, got {other:?}"),
        }
    }

    #[test]
    fn l0_adams_exec_box_non_numeric_flashes() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            Command::Box {
                game: "edm@bos".to_string(),
            },
            &mut app,
        );
        assert!(matches!(r, ExecResult::Flash(_)));
        // Screen should not have moved.
        assert!(!matches!(app.screen, Screen::GameDetail(_)));
    }

    #[test]
    fn l0_adams_exec_box_matchup_uses_loaded_scores_cache() {
        let mut app = fresh_app_with_mdi();
        app.tonight.cache.lock().unwrap().insert(
            app.tonight.date.clone(),
            crate::tui::tonight::TonightState::Loaded(vec![scheduled_game(
                2_025_020_321,
                "EDM",
                "BOS",
            )]),
        );

        let r = execute_command(
            Command::Box {
                game: "edm@bos".to_string(),
            },
            &mut app,
        );

        assert!(matches!(r, ExecResult::Continue));
        assert!(matches!(app.screen, Screen::GameDetail(2_025_020_321)));
    }

    #[test]
    fn l0_adams_exec_box_matchup_uses_loaded_schedule_cache() {
        let mut app = fresh_app_with_mdi();
        app.schedule.week_cache.lock().unwrap().insert(
            app.schedule.week.clone(),
            crate::tui::schedule::ScheduleState::Loaded(vec![scheduled_game(
                2_025_020_654,
                "NYR",
                "WSH",
            )]),
        );

        let r = execute_command(
            Command::Box {
                game: "nyr@wsh".to_string(),
            },
            &mut app,
        );

        assert!(matches!(r, ExecResult::Continue));
        assert!(matches!(app.screen, Screen::GameDetail(2_025_020_654)));
    }

    #[test]
    fn l0_adams_exec_class_lowers_to_draft_year_query() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(Command::Class { year: 2024 }, &mut app);
        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::Queries));
        assert_eq!(app.queries.filter_text, "draft-year=2024");
        assert!(app.queries.filter_plan.is_some());
    }

    #[test]
    fn l0_adams_exec_roster_opens_fantasy_gaps() {
        let mut app = fresh_app_with_mdi();
        let r = execute_command(Command::Roster, &mut app);
        assert!(matches!(r, ExecResult::Flash(_)));
        assert!(matches!(app.screen, Screen::FantasyGaps));
    }

    #[test]
    fn l0_adams_exec_query_valid_filter_applies_to_queries() {
        // edge-2: query verb mutates app.queries.filter_text and
        // swaps to Stats. Flash carries the applied filter.
        let mut app = fresh_app_with_mdi();
        let r = execute_command(
            Command::Query {
                filter: "g >= 30".to_string(),
            },
            &mut app,
        );
        match r {
            ExecResult::Flash(s) => assert!(
                s.contains("filter applied"),
                "expected 'filter applied' flash, got {s:?}"
            ),
            other => panic!("expected Flash, got {other:?}"),
        }
        assert!(matches!(app.screen, Screen::Queries));
        assert_eq!(app.queries.filter_text, "g >= 30");
        assert!(app.queries.filter_plan.is_some());
        assert!(app.queries.filter_error.is_none());
    }

    #[test]
    fn l0_adams_exec_query_invalid_filter_flashes_error() {
        // Invalid filter — flash carries parse error; no screen swap.
        let mut app = fresh_app_with_mdi();
        let original_screen = std::mem::discriminant(&app.screen);
        let r = execute_command(
            Command::Query {
                filter: "((".to_string(),
            },
            &mut app,
        );
        match r {
            ExecResult::Flash(s) => assert!(
                s.contains("parse error") || s.contains("error"),
                "expected error-shaped flash, got {s:?}"
            ),
            other => panic!("expected Flash, got {other:?}"),
        }
        // Screen unchanged.
        assert_eq!(std::mem::discriminant(&app.screen), original_screen);
    }
}
