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

    // ── Workspace-swap reads (no args) ────────────────────────
    /// `stats` — workspace becomes Stats / Queries.
    Stats,
    /// `goalies` — workspace becomes Goalies leaderboard.
    Goalies,
    /// `transactions` (alias `txs`) — workspace becomes
    /// Transactions feed.
    Transactions,
    /// `playoffs` — workspace becomes Playoffs bracket.
    Playoffs,
    /// `depth` — workspace becomes Depth chart.
    Depth,
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

    // ── Workspace-swap reads (with args) ──────────────────────
    /// `player <name-or-pid>` — workspace becomes that player's
    /// card.
    PlayerCard { needle: String },
    /// `team <abbrev>` — workspace becomes the team's depth
    /// chart.
    Team { abbrev: String },
    /// `team <abbrev> season` — workspace becomes the team's
    /// full-season schedule.
    TeamSeason { abbrev: String },
    /// `compare <left> [right]` — workspace becomes the
    /// comparison view. With one arg, `--similar` peers; with
    /// two, head-to-head.
    Compare { left: String, right: Option<String> },
    /// `box <game-id-or-team@team>` — workspace becomes the
    /// boxscore detail.
    Box { game: String },
    /// `class <year>` — workspace becomes the draft class for
    /// that year.
    Class { year: u16 },

    // ── Write actions (favorites mutation) ────────────────────
    /// `fav add <name-or-pid>` (or `/fav add ...`) — adds the
    /// player to the Favorites group.
    FavAdd { needle: String },
    /// `fav remove <name-or-pid>` — removes from Favorites.
    FavRemove { needle: String },

    // ── Free-form query — delegates to Phase Art Ross ─────────
    /// `query <filter>` — sets the Stats screen's free-form
    /// filter to the parsed Phase Art Ross expression. Workspace
    /// swaps to Stats. Mutates `app.queries.filter_text` directly
    /// (per spec edge-2: shared state with the Stats filter
    /// editor).
    Query { filter: String },
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
    BadInteger {
        command: &'static str,
        raw: String,
    },
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
                write!(
                    f,
                    "unknown pane {s:?}; expected `favorites` or `schedule`"
                )
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
        // `q` is shorthand for quit — `query` is the long-form verb
        // (consistent with vim). Filter syntax goes through `query`.
        "q" => Ok(Command::Quit),

        // Workspace-swap reads (no args)
        "stats" => Ok(Command::Stats),
        "goalies" | "g" => Ok(Command::Goalies),
        "transactions" | "txs" | "tx" => Ok(Command::Transactions),
        "playoffs" => Ok(Command::Playoffs),
        "depth" => Ok(Command::Depth),
        "roster" => Ok(Command::Roster),
        "fantasy" => parse_fantasy(args),
        "scores" => Ok(Command::Scores),
        "schedule" => Ok(Command::Schedule),
        "favorites" | "favs" => Ok(Command::Favorites),

        // Workspace-swap reads (with args)
        "player" | "p" => parse_player(args),
        "team" | "t" => parse_team(args),
        "compare" | "cmp" | "vs" => parse_compare(args),
        "box" | "boxscore" => parse_box(args),
        "class" => parse_class(args),

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

fn parse_team(args: &str) -> Result<Command, ParseError> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(ParseError::MissingArg {
            command: "team",
            arg: "abbrev",
        });
    }
    let (abbrev, rest) = split_first_word(trimmed);
    if rest.trim().eq_ignore_ascii_case("season") {
        return Ok(Command::TeamSeason {
            abbrev: abbrev.to_uppercase(),
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
    let year: u16 = trimmed
        .parse()
        .map_err(|_| ParseError::BadInteger {
            command: "class",
            raw: trimmed.to_owned(),
        })?;
    Ok(Command::Class { year })
}

/// `fantasy roster` → Roster. Other `fantasy <sub>` is currently
/// unsupported (could expand later).
fn parse_fantasy(args: &str) -> Result<Command, ParseError> {
    let sub = args.trim().to_ascii_lowercase();
    match sub.as_str() {
        "" | "roster" => Ok(Command::Roster),
        unknown => Err(ParseError::UnknownCommand(format!("fantasy {unknown}"))),
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
            ("transactions", Command::Transactions),
            ("txs", Command::Transactions),
            ("playoffs", Command::Playoffs),
            ("depth", Command::Depth),
            ("roster", Command::Roster),
            ("scores", Command::Scores),
            ("schedule", Command::Schedule),
            ("favorites", Command::Favorites),
        ] {
            assert_eq!(
                parse_command(input).unwrap(),
                expected,
                "verb {input:?}"
            );
        }
    }

    #[test]
    fn l0_adams_parse_fantasy_roster() {
        assert_eq!(parse_command("fantasy").unwrap(), Command::Roster);
        assert_eq!(parse_command("fantasy roster").unwrap(), Command::Roster);
    }

    // ── Workspace reads (with args) ────────────────────────────────────────

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
        matches!(e, ParseError::MissingArg { command: "player", .. });
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
    }

    #[test]
    fn l0_adams_parse_team_missing_arg() {
        let e = parse_command("team").unwrap_err();
        matches!(e, ParseError::MissingArg { command: "team", .. });
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
        matches!(e, ParseError::MissingArg { command: "/fav add", .. });
    }

    #[test]
    fn l0_adams_parse_fav_missing_subcommand() {
        let e = parse_command("fav").unwrap_err();
        matches!(e, ParseError::MissingArg { command: "/fav", .. });
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
        matches!(e, ParseError::MissingArg { command: "query", .. });
    }

    // ── Display impl ──────────────────────────────────────────────────────

    #[test]
    fn l0_adams_parse_error_display_is_user_friendly() {
        let cases = vec![
            (
                ParseError::UnknownCommand("xyz".into()),
                "unknown command",
            ),
            (
                ParseError::UnknownCommand(String::new()),
                "(empty input)",
            ),
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
            (
                ParseError::BadSidePane("foo".into()),
                "unknown pane",
            ),
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

    // ── split_first_word helper ───────────────────────────────────────────

    #[test]
    fn l0_adams_split_first_word_basic() {
        assert_eq!(split_first_word("foo bar baz"), ("foo", "bar baz"));
        assert_eq!(split_first_word("solo"), ("solo", ""));
        assert_eq!(split_first_word("  leading  trailing  "), ("leading", "trailing  "));
        assert_eq!(split_first_word(""), ("", ""));
    }
}
