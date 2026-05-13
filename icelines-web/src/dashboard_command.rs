//! Web dashboard command parser.
//!
//! Jack Adams Web keeps the browser command palette deterministic first:
//! commands lower into internal workspace routes or explicit mutation intents.
//! The TUI owns the richer executor today; this module fences the web grammar
//! against the same examples without making `icelines-web` depend on
//! `icelines-cli`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardCommand {
    OpenWorkspace { url: String },
    HidePane(DashboardPane),
    ShowPane(DashboardPane),
    Mutation(DashboardMutationIntent),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardPane {
    Favorites,
    Schedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardMutationIntent {
    FavoriteAdd { player: String, post_url: String },
    FavoriteRemove { player: String, post_url: String },
    WatchPlayer { player: String, post_url: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DashboardCommandError {
    UnknownCommand(String),
    MissingArg {
        command: &'static str,
        arg: &'static str,
    },
    BadPane(String),
    ExternalRoute(String),
}

impl std::fmt::Display for DashboardCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(input) if input.is_empty() => write!(f, "(empty input)"),
            Self::UnknownCommand(input) => write!(f, "unknown command: {input}"),
            Self::MissingArg { command, arg } => write!(f, "{command}: missing <{arg}>"),
            Self::BadPane(pane) => write!(f, "unknown pane: {pane}"),
            Self::ExternalRoute(route) => write!(f, "command resolved outside dashboard: {route}"),
        }
    }
}

impl std::error::Error for DashboardCommandError {}

pub fn parse_dashboard_command(input: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DashboardCommandError::UnknownCommand(String::new()));
    }

    if let Some(rest) = trimmed.strip_prefix('/') {
        return parse_slash(rest);
    }

    parse_verb(trimmed)
}

fn parse_slash(input: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (verb, args) = split_first_word(input);
    match verb.to_ascii_lowercase().as_str() {
        "help" | "h" | "?" => workspace("/docs"),
        "hide" => parse_pane(args).map(DashboardCommand::HidePane),
        "show" => parse_pane(args).map(DashboardCommand::ShowPane),
        "fav" | "favorite" | "favorites" => parse_favorite_mutation(args),
        unknown => Err(DashboardCommandError::UnknownCommand(format!("/{unknown}"))),
    }
}

fn parse_verb(input: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (verb, args) = split_first_word(input);
    match verb.to_ascii_lowercase().as_str() {
        "help" => workspace("/docs"),
        "stats" | "leaders" if args.trim().is_empty() => workspace("/leaders"),
        "stats" | "leaders" | "query" => workspace(&leaders_url(args)),
        "goalies" | "g" => workspace("/goalies"),
        "depth" => workspace("/depth"),
        "scores" | "tonight" => workspace("/scores"),
        "schedule" => workspace("/schedule"),
        "transactions" | "txs" | "tx" => workspace("/transactions"),
        "playoffs" => workspace("/playoffs"),
        "favorites" => workspace("/favorites"),
        "watchlist" => workspace("/watchlist"),
        "poach" => workspace(&poach_url(args)),
        "gaps" | "fantasy-gaps" => workspace(&fantasy_url(args, FantasyMode::Gaps)),
        "simulate" | "sim" | "fantasy-sim" => {
            workspace(&fantasy_url(args, FantasyMode::Simulation))
        }
        "fantasy" => parse_fantasy(args),
        "player" => required_workspace_arg("player", "name", args, |needle| {
            format!("/leaders?filter=name%3D{}", url_component(needle))
        }),
        "team" => parse_team(args),
        "box" => required_workspace_arg("box", "game", args, |game| {
            format!("/game/{}", url_component(game))
        }),
        "compare" => parse_compare(args),
        "fav" | "favorite" => parse_favorite_mutation(args),
        "watch" => parse_watch_mutation(args),
        unknown => Err(DashboardCommandError::UnknownCommand(unknown.to_owned())),
    }
}

fn parse_fantasy(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (sub, rest) = split_first_word(args);
    match sub.to_ascii_lowercase().as_str() {
        "" | "roster" => workspace("/fantasy"),
        "gaps" => workspace(&fantasy_url(rest, FantasyMode::Gaps)),
        "simulate" | "sim" => workspace(&fantasy_url(rest, FantasyMode::Simulation)),
        "poach" => workspace(&poach_url(rest)),
        unknown => Err(DashboardCommandError::UnknownCommand(format!(
            "fantasy {unknown}"
        ))),
    }
}

fn parse_team(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (abbrev, rest) = split_first_word(args);
    if abbrev.is_empty() {
        return Err(DashboardCommandError::MissingArg {
            command: "team",
            arg: "abbrev",
        });
    }
    let team = abbrev.to_ascii_uppercase();
    match rest.trim().to_ascii_lowercase().as_str() {
        "" => workspace(&format!("/team/{team}")),
        "season" | "schedule" => workspace(&format!("/schedule?team={team}")),
        _ => workspace(&format!("/team/{team}")),
    }
}

fn parse_compare(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let parts = shell_words(args);
    let Some(left) = parts.first() else {
        return Err(DashboardCommandError::MissingArg {
            command: "compare",
            arg: "left player",
        });
    };
    let mut url = format!("/compare?left={}", url_component(left));
    if let Some(right) = parts.get(1) {
        url.push_str("&right=");
        url.push_str(&url_component(right));
    }
    workspace(&url)
}

fn parse_favorite_mutation(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (sub, rest) = split_first_word(args);
    let player = rest.trim();
    match sub.to_ascii_lowercase().as_str() {
        "add" if player.is_empty() => Err(DashboardCommandError::MissingArg {
            command: "fav add",
            arg: "player",
        }),
        "add" => Ok(DashboardCommand::Mutation(
            DashboardMutationIntent::FavoriteAdd {
                player: player.to_owned(),
                post_url: "/favorites/add".to_owned(),
            },
        )),
        "remove" | "rm" | "del" if player.is_empty() => Err(DashboardCommandError::MissingArg {
            command: "fav remove",
            arg: "player",
        }),
        "remove" | "rm" | "del" => Ok(DashboardCommand::Mutation(
            DashboardMutationIntent::FavoriteRemove {
                player: player.to_owned(),
                post_url: "/favorites/remove".to_owned(),
            },
        )),
        "" => Err(DashboardCommandError::MissingArg {
            command: "fav",
            arg: "subcommand",
        }),
        unknown => Err(DashboardCommandError::UnknownCommand(format!(
            "fav {unknown}"
        ))),
    }
}

fn parse_watch_mutation(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let player = args.trim();
    if player.is_empty() {
        return Err(DashboardCommandError::MissingArg {
            command: "watch",
            arg: "player",
        });
    }
    Ok(DashboardCommand::Mutation(
        DashboardMutationIntent::WatchPlayer {
            player: player.to_owned(),
            post_url: "/watch-rules/create".to_owned(),
        },
    ))
}

fn parse_pane(args: &str) -> Result<DashboardPane, DashboardCommandError> {
    match args.trim().to_ascii_lowercase().as_str() {
        "fav" | "favs" | "favorite" | "favorites" | "watchlist" => Ok(DashboardPane::Favorites),
        "sched" | "schedule" => Ok(DashboardPane::Schedule),
        "" => Err(DashboardCommandError::MissingArg {
            command: "pane",
            arg: "pane",
        }),
        other => Err(DashboardCommandError::BadPane(other.to_owned())),
    }
}

fn required_workspace_arg(
    command: &'static str,
    arg: &'static str,
    args: &str,
    build_url: impl FnOnce(&str) -> String,
) -> Result<DashboardCommand, DashboardCommandError> {
    let value = args.trim();
    if value.is_empty() {
        Err(DashboardCommandError::MissingArg { command, arg })
    } else {
        workspace(&build_url(value))
    }
}

fn workspace(url: &str) -> Result<DashboardCommand, DashboardCommandError> {
    if url.starts_with('/') && !url.starts_with("//") && !url.contains("://") {
        Ok(DashboardCommand::OpenWorkspace {
            url: url.to_owned(),
        })
    } else {
        Err(DashboardCommandError::ExternalRoute(url.to_owned()))
    }
}

fn leaders_url(args: &str) -> String {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        "/leaders".to_owned()
    } else {
        format!("/leaders?filter={}", url_component(trimmed))
    }
}

fn poach_url(args: &str) -> String {
    query_url("/poach", args)
}

#[derive(Debug, Clone, Copy)]
enum FantasyMode {
    Gaps,
    Simulation,
}

fn fantasy_url(args: &str, mode: FantasyMode) -> String {
    let base = match mode {
        FantasyMode::Gaps => "/fantasy",
        FantasyMode::Simulation => "/fantasy",
    };
    query_url(base, args)
}

fn query_url(base: &str, args: &str) -> String {
    let query = command_args_to_query(args);
    if query.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{query}")
    }
}

fn command_args_to_query(args: &str) -> String {
    shell_words(args)
        .into_iter()
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            let key = match key {
                "cats" | "categories" => "category",
                "free" | "available" => "availability",
                "add" => "add_player",
                "drop" => "drop_player",
                other => other,
            };
            Some(format!("{}={}", url_component(key), url_component(value)))
        })
        .collect::<Vec<_>>()
        .join("&")
}

fn split_first_word(input: &str) -> (&str, &str) {
    let trimmed = input.trim();
    match trimmed.find(char::is_whitespace) {
        Some(idx) => (&trimmed[..idx], &trimmed[idx..]),
        None => (trimmed, ""),
    }
}

fn shell_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in input.chars() {
        match ch {
            '"' => in_quote = !in_quote,
            c if c.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }

    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn url_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            b' ' => vec!['+'],
            _ => {
                let hex = format!("%{byte:02X}");
                hex.chars().collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(input: &str) -> String {
        match parse_dashboard_command(input).expect("command parses") {
            DashboardCommand::OpenWorkspace { url } => url,
            other => panic!("expected workspace command, got {other:?}"),
        }
    }

    #[test]
    fn l0_dashboard_command_tui_read_examples_resolve_to_internal_routes() {
        assert_eq!(route("stats"), "/leaders");
        assert_eq!(route("goalies"), "/goalies");
        assert_eq!(route("poach"), "/poach");
        assert_eq!(
            route("gaps cats=hits,blocks top=8"),
            "/fantasy?category=hits%2Cblocks&top=8"
        );
        assert_eq!(
            route("fantasy poach top=8 availability=available"),
            "/poach?top=8&availability=available"
        );
        assert_eq!(
            route("simulate add=Connor_McDavid drop=Bench_Forward weeks=3"),
            "/fantasy?add_player=Connor_McDavid&drop_player=Bench_Forward&weeks=3"
        );
    }

    #[test]
    fn l0_dashboard_command_navigation_examples_resolve_to_internal_routes() {
        assert_eq!(route("team edm"), "/team/EDM");
        assert_eq!(route("team EDM season"), "/schedule?team=EDM");
        assert_eq!(
            route("player Connor McDavid"),
            "/leaders?filter=name%3DConnor+McDavid"
        );
        assert_eq!(route("box EDM@BOS"), "/game/EDM%40BOS");
        assert_eq!(
            route("compare \"Connor McDavid\" \"Nathan MacKinnon\""),
            "/compare?left=Connor+McDavid&right=Nathan+MacKinnon"
        );
    }

    #[test]
    fn l0_dashboard_command_mutations_are_post_intents_not_get_routes() {
        assert_eq!(
            parse_dashboard_command("/fav add Connor McDavid").expect("fav add parses"),
            DashboardCommand::Mutation(DashboardMutationIntent::FavoriteAdd {
                player: "Connor McDavid".to_owned(),
                post_url: "/favorites/add".to_owned(),
            })
        );
        assert_eq!(
            parse_dashboard_command("watch Connor McDavid").expect("watch parses"),
            DashboardCommand::Mutation(DashboardMutationIntent::WatchPlayer {
                player: "Connor McDavid".to_owned(),
                post_url: "/watch-rules/create".to_owned(),
            })
        );
    }

    #[test]
    fn l0_dashboard_command_rejects_unknown_and_external_routes() {
        assert!(matches!(
            parse_dashboard_command("https://evil.example"),
            Err(DashboardCommandError::UnknownCommand(_))
        ));
        assert!(matches!(
            workspace("https://evil.example"),
            Err(DashboardCommandError::ExternalRoute(_))
        ));
    }
}
