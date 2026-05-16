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
    FavoriteAdd {
        player: String,
        post_url: String,
    },
    FavoriteRemove {
        player: String,
        post_url: String,
    },
    WatchPlayer {
        player: String,
        trigger: String,
        post_url: String,
    },
    WatchSetEnabled {
        rule_id: String,
        enabled: bool,
        post_url: String,
    },
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
    UnsupportedMutation(String),
}

impl std::fmt::Display for DashboardCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCommand(input) if input.is_empty() => write!(f, "(empty input)"),
            Self::UnknownCommand(input) => write!(f, "unknown command: {input}"),
            Self::MissingArg { command, arg } => write!(f, "{command}: missing <{arg}>"),
            Self::BadPane(pane) => write!(f, "unknown pane: {pane}"),
            Self::ExternalRoute(route) => write!(f, "command resolved outside dashboard: {route}"),
            Self::UnsupportedMutation(message) => write!(f, "{message}"),
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
        "favorites" => parse_favorites_workspace(args),
        "watchlist" => workspace("/watchlist"),
        "roster" => workspace("/fantasy"),
        "career" | "cohort" => workspace(&career_url(args)),
        "class" => workspace(&career_class_url(args)),
        "report" | "reports" => parse_report(args),
        "records" | "record" => parse_records(args),
        "poach" => workspace(&poach_url(args)),
        "gaps" | "fantasy-gaps" => workspace(&fantasy_url(args, FantasyMode::Gaps)),
        "simulate" | "sim" | "fantasy-sim" => {
            workspace(&fantasy_url(args, FantasyMode::Simulation))
        }
        "daily" | "fantasy-daily" => workspace(&fantasy_daily_url(args)),
        "matchup" | "fantasy-matchup" => workspace(&fantasy_matchup_url(args)),
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
        "group" | "groups" => parse_group_workspace(args),
        "watch" => parse_watch_mutation(args),
        unknown => Err(DashboardCommandError::UnknownCommand(unknown.to_owned())),
    }
}

fn parse_favorites_workspace(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let args = args.trim();
    if args.is_empty() {
        return workspace("/favorites");
    }
    if let Some(group) = args.strip_prefix("group=") {
        return required_workspace_arg("favorites", "group", group, |group| {
            format!("/favorites?group={}", url_component(group))
        });
    }
    workspace("/favorites")
}

fn parse_group_workspace(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (subcommand, rest) = split_first_word(args);
    match subcommand.to_ascii_lowercase().as_str() {
        "" | "list" => workspace("/favorites"),
        "show" | "open" => required_workspace_arg("group show", "name", rest, |group| {
            format!("/favorites?group={}", url_component(group))
        }),
        "create" | "delete" | "remove" | "rename" | "add" => {
            Err(DashboardCommandError::UnsupportedMutation(
                "Web dashboard group create/delete/rename/member edits are deferred; use the TUI Groups screen or `icelines group`.".to_owned(),
            ))
        }
        group => workspace(&format!("/favorites?group={}", url_component(group))),
    }
}

fn parse_fantasy(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (sub, rest) = split_first_word(args);
    match sub.to_ascii_lowercase().as_str() {
        "" | "roster" => workspace("/fantasy"),
        "gaps" => workspace(&fantasy_url(rest, FantasyMode::Gaps)),
        "simulate" | "sim" => workspace(&fantasy_url(rest, FantasyMode::Simulation)),
        "daily" => workspace(&fantasy_daily_url(rest)),
        "matchup" => workspace(&fantasy_matchup_url(rest)),
        "poach" => workspace(&poach_url(rest)),
        unknown => Err(DashboardCommandError::UnknownCommand(format!(
            "fantasy {unknown}"
        ))),
    }
}

fn parse_report(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (kind, rest) = split_first_word(args);
    let base = match kind.to_ascii_lowercase().as_str() {
        "poach" | "poacher" => "/reports/poach",
        "weekly" | "week" => "/reports/weekly",
        "" => {
            return Err(DashboardCommandError::MissingArg {
                command: "report",
                arg: "poach|weekly",
            });
        }
        other => {
            return Err(DashboardCommandError::UnknownCommand(format!(
                "report {other}"
            )))
        }
    };
    workspace(&query_url(base, rest))
}

fn parse_records(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let (target, subject) = split_first_word(args);
    let subject = subject.trim();
    match target.to_ascii_lowercase().as_str() {
        "player" | "p" if subject.is_empty() => Err(DashboardCommandError::MissingArg {
            command: "records player",
            arg: "nhl-id",
        }),
        "player" | "p" => workspace(&format!("/records/player/{}", url_component(subject))),
        "team" | "t" if subject.is_empty() => Err(DashboardCommandError::MissingArg {
            command: "records team",
            arg: "abbrev",
        }),
        "team" | "t" => workspace(&format!(
            "/records/team/{}",
            url_component(&subject.to_ascii_uppercase())
        )),
        "" => Err(DashboardCommandError::MissingArg {
            command: "records",
            arg: "player|team",
        }),
        other => Err(DashboardCommandError::UnknownCommand(format!(
            "records {other}"
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
        "season" => workspace(&format!("/team/{team}/season")),
        "schedule" => workspace(&format!("/schedule?team={team}")),
        _ => workspace(&format!("/team/{team}")),
    }
}

fn parse_compare(args: &str) -> Result<DashboardCommand, DashboardCommandError> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(DashboardCommandError::MissingArg {
            command: "compare",
            arg: "left player",
        });
    };

    if let Some((left, right)) = split_compare_pair(trimmed) {
        return workspace(&compare_url(left, Some(right)));
    }

    let parts = shell_words(trimmed);
    let Some(left) = parts.first() else {
        return Err(DashboardCommandError::MissingArg {
            command: "compare",
            arg: "left player",
        });
    };
    workspace(&compare_url(left, parts.get(1).map(String::as_str)))
}

fn split_compare_pair(input: &str) -> Option<(&str, &str)> {
    let lower = input.to_ascii_lowercase();
    if let Some(idx) = lower.find(" vs ") {
        let (left, rest) = input.split_at(idx);
        let right = &rest[4..];
        return Some((left.trim(), right.trim()));
    }
    input
        .split_once(',')
        .map(|(left, right)| (left.trim(), right.trim()))
}

fn compare_url(left: &str, right: Option<&str>) -> String {
    let mut url = format!("/compare?left={}", url_component(left));
    if let Some(right) = right.filter(|value| !value.is_empty()) {
        url.push_str("&right=");
        url.push_str(&url_component(right));
    }
    url
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
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(DashboardCommandError::MissingArg {
            command: "watch",
            arg: "player",
        });
    }
    let (subcommand, rest) = split_first_word(trimmed);
    let (player, trigger) = match subcommand.to_ascii_lowercase().as_str() {
        "enable" | "on" => return parse_watch_rule_set_enabled(rest, true),
        "disable" | "off" => return parse_watch_rule_set_enabled(rest, false),
        "team" | "deployment" => {
            return Err(DashboardCommandError::UnsupportedMutation(
                "watch team/deployment rule editing is deferred; use `icelines watch deployment ...` for CLI preview or `/watchlist` for player rules".to_owned(),
            ))
        }
        "player" => parse_watch_player_args(rest)?,
        _ => (trimmed.to_owned(), "available".to_owned()),
    };
    Ok(DashboardCommand::Mutation(
        DashboardMutationIntent::WatchPlayer {
            player,
            trigger,
            post_url: "/watch-rules/create".to_owned(),
        },
    ))
}

fn parse_watch_rule_set_enabled(
    args: &str,
    enabled: bool,
) -> Result<DashboardCommand, DashboardCommandError> {
    let rule_id = args.trim();
    if rule_id.is_empty() {
        return Err(DashboardCommandError::MissingArg {
            command: "watch",
            arg: "rule-id",
        });
    }
    Ok(DashboardCommand::Mutation(
        DashboardMutationIntent::WatchSetEnabled {
            rule_id: rule_id.to_owned(),
            enabled,
            post_url: "/watch-rules/set-enabled".to_owned(),
        },
    ))
}

fn parse_watch_player_args(args: &str) -> Result<(String, String), DashboardCommandError> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Err(DashboardCommandError::MissingArg {
            command: "watch player",
            arg: "player",
        });
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some(idx) = lower.rfind(" when=") {
        let player = trimmed[..idx].trim();
        let trigger = trimmed[idx + " when=".len()..].trim();
        if player.is_empty() {
            return Err(DashboardCommandError::MissingArg {
                command: "watch player",
                arg: "player",
            });
        }
        if trigger.is_empty() || trigger.split_whitespace().nth(1).is_some() {
            return Err(DashboardCommandError::UnsupportedMutation(
                "watch player expects a single trigger after when=".to_owned(),
            ));
        }
        return Ok((player.to_owned(), trigger.to_owned()));
    }
    Ok((trimmed.to_owned(), "available".to_owned()))
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

fn career_url(args: &str) -> String {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return "/career?league=OHL&sort=points".to_owned();
    }
    if trimmed.contains('=') {
        return query_url("/career", trimmed);
    }
    format!("/career?league={}&sort=points", url_component(trimmed))
}

fn career_class_url(args: &str) -> String {
    let year = args.trim();
    if year.is_empty() {
        "/career?league=OHL&sort=points".to_owned()
    } else if year.contains('=') {
        query_url("/career", year)
    } else {
        format!("/career?season={}&sort=points", url_component(year))
    }
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

fn fantasy_daily_url(args: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return "/api/v1/fantasy/daily".to_string();
    }
    if let Some(date) = args.strip_prefix("date=") {
        return format!("/api/v1/fantasy/daily?date={}", url_component(date.trim()));
    }
    format!("/api/v1/fantasy/daily?date={}", url_component(args))
}

fn fantasy_matchup_url(args: &str) -> String {
    let args = args.trim();
    if args.is_empty() {
        return "/api/v1/fantasy/matchup".to_string();
    }
    if let Some(date) = args.strip_prefix("date=") {
        return format!(
            "/api/v1/fantasy/matchup?date={}",
            url_component(date.trim())
        );
    }
    format!("/api/v1/fantasy/matchup?date={}", url_component(args))
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
    let parts = shell_words(args);
    let mut query = Vec::new();
    let mut idx = 0;

    while idx < parts.len() {
        let part = &parts[idx];

        if let Some((key, value)) = part.split_once('=') {
            push_query_pair(&mut query, key, value);
            idx += 1;
            continue;
        }

        match part.to_ascii_lowercase().as_str() {
            "free" | "available" => query.push("availability=available".to_owned()),
            "watched" => query.push("availability=watched".to_owned()),
            "rostered" => query.push("availability=imported-rostered".to_owned()),
            "rw" | "lw" | "c" | "d" | "g" => {
                query.push(format!("pos={}", url_component(&part.to_ascii_uppercase())));
            }
            "add" | "drop" if parts.get(idx + 1).is_some() => {
                let key = if part.eq_ignore_ascii_case("add") {
                    "add_player"
                } else {
                    "drop_player"
                };
                query.push(format!("{}={}", key, url_component(&parts[idx + 1])));
                idx += 2;
                continue;
            }
            _ => {}
        }
        idx += 1;
    }

    query.join("&")
}

fn push_query_pair(query: &mut Vec<String>, key: &str, value: &str) {
    let key = match key {
        "cats" | "categories" => "category",
        "free" | "available" => "availability",
        "add" => "add_player",
        "drop" => "drop_player",
        "pos" | "position" => "pos",
        other => other,
    };
    query.push(format!("{}={}", url_component(key), url_component(value)));
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
        assert_eq!(route("roster"), "/fantasy");
        assert_eq!(
            route("poach rw cats=hits,blocks free top=12"),
            "/poach?pos=RW&category=hits%2Cblocks&availability=available&top=12"
        );
        assert_eq!(
            route("gaps cats=hits,blocks top=8"),
            "/fantasy?category=hits%2Cblocks&top=8"
        );
        assert_eq!(
            route("fantasy poach top=8 availability=available"),
            "/poach?top=8&availability=available"
        );
        assert_eq!(
            route("fantasy poach top=8 available"),
            "/poach?top=8&availability=available"
        );
        assert_eq!(
            route("simulate add=Connor_McDavid drop=Bench_Forward weeks=3"),
            "/fantasy?add_player=Connor_McDavid&drop_player=Bench_Forward&weeks=3"
        );
        assert_eq!(
            route("fantasy simulate add Connor_McDavid drop Bench_Forward"),
            "/fantasy?add_player=Connor_McDavid&drop_player=Bench_Forward"
        );
        assert_eq!(
            route("fantasy daily date=2026-01-15"),
            "/api/v1/fantasy/daily?date=2026-01-15"
        );
        assert_eq!(
            route("fantasy matchup date=2026-01-15"),
            "/api/v1/fantasy/matchup?date=2026-01-15"
        );
        assert_eq!(
            route("report weekly cats=shots,hits top=12"),
            "/reports/weekly?category=shots%2Chits&top=12"
        );
        assert_eq!(
            route("report poach availability=imported-available"),
            "/reports/poach?availability=imported-available"
        );
    }

    #[test]
    fn l0_dashboard_command_navigation_examples_resolve_to_internal_routes() {
        assert_eq!(route("help"), "/docs");
        assert_eq!(route("/help"), "/docs");
        assert_eq!(route("team edm"), "/team/EDM");
        assert_eq!(route("team EDM season"), "/team/EDM/season");
        assert_eq!(route("team EDM schedule"), "/schedule?team=EDM");
        assert_eq!(route("records team edm"), "/records/team/EDM");
        assert_eq!(route("records player 8478402"), "/records/player/8478402");
        assert_eq!(route("career"), "/career?league=OHL&sort=points");
        assert_eq!(route("class 2015"), "/career?season=2015&sort=points");
        assert_eq!(
            route("career league=OHL season=20142015 top=8"),
            "/career?league=OHL&season=20142015&top=8"
        );
        assert_eq!(
            route("player Connor McDavid"),
            "/leaders?filter=name%3DConnor+McDavid"
        );
        assert_eq!(route("box EDM@BOS"), "/game/EDM%40BOS");
        assert_eq!(
            route("compare \"Connor McDavid\" \"Nathan MacKinnon\""),
            "/compare?left=Connor+McDavid&right=Nathan+MacKinnon"
        );
        assert_eq!(
            route("compare Connor McDavid vs Sidney Crosby"),
            "/compare?left=Connor+McDavid&right=Sidney+Crosby"
        );
        assert_eq!(
            route("compare Connor McDavid, Nathan MacKinnon"),
            "/compare?left=Connor+McDavid&right=Nathan+MacKinnon"
        );
        assert_eq!(
            route("favorites group=Prospects"),
            "/favorites?group=Prospects"
        );
        assert_eq!(route("group show Prospects"), "/favorites?group=Prospects");
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
                trigger: "available".to_owned(),
                post_url: "/watch-rules/create".to_owned(),
            })
        );
        assert_eq!(
            parse_dashboard_command("watch disable player-connor-mcdavid")
                .expect("watch disable parses"),
            DashboardCommand::Mutation(DashboardMutationIntent::WatchSetEnabled {
                rule_id: "player-connor-mcdavid".to_owned(),
                enabled: false,
                post_url: "/watch-rules/set-enabled".to_owned(),
            })
        );
    }

    #[test]
    fn l0_dashboard_command_watch_player_supports_trigger_and_fences_deployment() {
        match parse_dashboard_command("watch player Connor McDavid when=pp1")
            .expect("watch player parses")
        {
            DashboardCommand::Mutation(DashboardMutationIntent::WatchPlayer {
                player,
                trigger,
                post_url,
            }) => {
                assert_eq!(player, "Connor McDavid");
                assert_eq!(trigger, "pp1");
                assert_eq!(post_url, "/watch-rules/create");
            }
            other => panic!("expected watch mutation, got {other:?}"),
        }

        let err = parse_dashboard_command("watch deployment TOR")
            .expect_err("deployment editor is deferred");
        assert!(
            err.to_string()
                .contains("watch team/deployment rule editing is deferred"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn l0_dashboard_command_group_mutations_are_deferred() {
        let err = parse_dashboard_command("group create Prospects")
            .expect_err("group create is deferred");
        assert!(
            err.to_string()
                .contains("group create/delete/rename/member edits are deferred"),
            "unexpected error: {err}"
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
