use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde::Serialize;

const READ_ONLY_EFFECT: &str = "read-only stdout";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandEffectFilter {
    All,
    ReadOnly,
    WritesOnly,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatheadQuery {
    pub label: &'static str,
    pub command: &'static str,
    pub why: &'static str,
    pub requires: &'static str,
    pub effect: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatheadPack {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub queries: &'static [StatheadQuery],
}

const ERA_LEADERS: &[StatheadQuery] = &[
    StatheadQuery {
        label: "Bundled all-era points",
        command: "icelines query leaders --seasons 38 --sort points --top 25",
        why: "Start with the full bundled-history scoring leaderboard.",
        requires: "Bundled season totals; no live fetch required.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Recent multi-season goal pace",
        command: "icelines query leaders --seasons 5 --sort goals-pace --top 25",
        why: "Compare modern goal scorers across the deepest bundled seasons.",
        requires: "Modern bundled season totals; fresher local installs can override bundles.",
        effect: READ_ONLY_EFFECT,
    },
];

const YOUNG_STARS: &[StatheadQuery] = &[
    StatheadQuery {
        label: "Under-24 production",
        command: "icelines query leaders --age-max 23 --sort ppg --top 25",
        why: "Find current young players driving scoring rate.",
        requires: "Current configured season from bundled or installed stats.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Young physical scorers",
        command:
            "icelines query leaders --age-max 24 --filter \"hits>=100\" --filter \"p>=40\" --top 25",
        why: "Blend age, production, and physical category filters.",
        requires: "Current configured season with hit totals available.",
        effect: READ_ONLY_EFFECT,
    },
];

const PLAYOFF_RUNS: &[StatheadQuery] = &[
    StatheadQuery {
        label: "Playoff scoring leaders",
        command: "icelines query leaders --playoff --sort points --top 25",
        why: "Use cached playoff boxscore aggregates for cup-run context.",
        requires: "Cached playoff boxscore rows; run playoff boxscore fetches first if empty.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Series momentum",
        command: "icelines playoffs --series A",
        why: "Pair player leaderboards with the active playoff-series summary.",
        requires: "Playoff bracket data from bundled, installed, or fetched playoff sources.",
        effect: READ_ONLY_EFFECT,
    },
];

const GOALIE_NOTEBOOK: &[StatheadQuery] = &[
    StatheadQuery {
        label: "Save percentage leaders",
        command: "icelines query goalies --filter \"save-pct>=0.91\" --top 25",
        why: "Start a goalie notebook with a clear sample-quality filter.",
        requires: "Goalie season totals from bundled or installed goalie stats.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Goalie workload",
        command: "icelines query goalies --sort goalie-games --top 25",
        why: "Check who is carrying the largest regular-season workload.",
        requires: "Goalie season totals from bundled or installed goalie stats.",
        effect: READ_ONLY_EFFECT,
    },
];

const RECORDS_NOTEBOOK: &[StatheadQuery] = &[
    StatheadQuery {
        label: "Player teams scored against",
        command: "icelines records player \"Andre Burakovsky\" --metric teams-scored-against",
        why: "Start a player records notebook with opponent-team breadth.",
        requires:
            "Cached boxscore goal rows; run boxscore fetches for the target window first if empty.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Player goalies scored against",
        command: "icelines records player \"Andre Burakovsky\" --metric goalies-scored-against",
        why: "Switch from team breadth to goalie-specific scoring history.",
        requires: "Cached play-by-play participant rows joined to boxscore goals.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Team players scored against",
        command: "icelines records team SEA --metric players-scored-against-team",
        why: "Find the player list that has scored against a selected team.",
        requires: "Cached boxscore goal rows for games involving the selected team.",
        effect: READ_ONLY_EFFECT,
    },
];

const FANTASY_PREP: &[StatheadQuery] = &[
    StatheadQuery {
        label: "Category poach board",
        command: "icelines poach --category hits,blocks,shots --top 20",
        why: "Start a fantasy add/drop session from category needs.",
        requires: "Current configured season stats plus any local FantasyDb roster/import context.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Available-shot streamers",
        command: "icelines poach --category shots --availability available --top 15",
        why: "Narrow the poach board to available players for a shooting-volume stream.",
        requires: "Local FantasyDb availability context; unknown rosters may reduce availability precision.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Weekly prep packet",
        command: "icelines report weekly --category hits,blocks --top 10 --out weekly.md",
        why: "Generate a durable weekly fantasy prep document from the poach report path.",
        requires: "Current stats, local FantasyDb league/team setup, and write access to the output path.",
        effect: "writes weekly.md",
    },
];

const DRAFT_SCOUTING: &[StatheadQuery] = &[
    StatheadQuery {
        label: "Draft class forwards",
        command: "icelines class 2023 --pos F --top 25",
        why: "Start a draft-class review with the top forward cohort.",
        requires: "Bundled or installed season totals with draft-year metadata.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Player peer cohort",
        command: "icelines peers \"Connor Bedard\" --size 12",
        why: "Build a statistical-peer shortlist around a focal player.",
        requires: "Current player stats plus same-era/position cohort data.",
        effect: READ_ONLY_EFFECT,
    },
    StatheadQuery {
        label: "Full scouting report",
        command: "icelines scouting \"Connor Bedard\" --format markdown",
        why: "Generate a durable player scouting packet for editorial review.",
        requires: "Bundled or installed player stats and scouting report inputs.",
        effect: READ_ONLY_EFFECT,
    },
];

const PACKS: &[StatheadPack] = &[
    StatheadPack {
        slug: "era-leaders",
        title: "Era leaders",
        description: "Historical and multi-season leaderboards from bundled data.",
        queries: ERA_LEADERS,
    },
    StatheadPack {
        slug: "young-stars",
        title: "Young stars",
        description: "Age-gated scorer and category-filter starter queries.",
        queries: YOUNG_STARS,
    },
    StatheadPack {
        slug: "playoff-runs",
        title: "Playoff runs",
        description: "Playoff scoring and series-context entry points.",
        queries: PLAYOFF_RUNS,
    },
    StatheadPack {
        slug: "goalie-notebook",
        title: "Goalie notebook",
        description: "Goalie leaderboard starters with workload and rate checks.",
        queries: GOALIE_NOTEBOOK,
    },
    StatheadPack {
        slug: "records-notebook",
        title: "Records notebook",
        description: "Cached event-data record starters for player and team notebooks.",
        queries: RECORDS_NOTEBOOK,
    },
    StatheadPack {
        slug: "fantasy-prep",
        title: "Fantasy prep",
        description: "Fantasy poach and weekly-prep starter recipes.",
        queries: FANTASY_PREP,
    },
    StatheadPack {
        slug: "draft-scouting",
        title: "Draft scouting",
        description: "Draft class, peer cohort, and scouting-report starters.",
        queries: DRAFT_SCOUTING,
    },
];

pub fn run(
    pack: Option<String>,
    json: bool,
    markdown: bool,
    commands: bool,
    read_only: bool,
    writes_only: bool,
    out: Option<PathBuf>,
) -> Result<()> {
    if [json, markdown, commands]
        .into_iter()
        .filter(|enabled| *enabled)
        .count()
        > 1
    {
        bail!("--json, --markdown, and --commands are mutually exclusive");
    }
    if out.is_some() && !markdown && !commands {
        bail!("--out requires --markdown or --commands");
    }
    if read_only && !commands {
        bail!("--read-only requires --commands");
    }
    if writes_only && !commands {
        bail!("--writes-only requires --commands");
    }
    if read_only && writes_only {
        bail!("--read-only and --writes-only are mutually exclusive");
    }

    let effect_filter = match (read_only, writes_only) {
        (true, false) => CommandEffectFilter::ReadOnly,
        (false, true) => CommandEffectFilter::WritesOnly,
        _ => CommandEffectFilter::All,
    };

    match pack {
        Some(slug) => {
            let selected = find_pack(&slug)?;
            if json {
                println!("{}", serde_json::to_string_pretty(selected)?);
            } else if markdown {
                emit_markdown(selected, out)?;
            } else if commands {
                emit_commands(&[selected], effect_filter, out)?;
            } else {
                print_pack(selected);
            }
        }
        None => {
            if markdown {
                emit_markdown_index(out)?;
            } else if commands {
                let packs = PACKS.iter().collect::<Vec<_>>();
                emit_commands(&packs, effect_filter, out)?;
            } else if json {
                println!("{}", serde_json::to_string_pretty(PACKS)?);
            } else {
                print_pack_list();
            }
        }
    }
    Ok(())
}

fn find_pack(slug: &str) -> Result<&'static StatheadPack> {
    PACKS
        .iter()
        .find(|pack| pack.slug.eq_ignore_ascii_case(slug))
        .ok_or_else(|| {
            let known = PACKS
                .iter()
                .map(|pack| pack.slug)
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::anyhow!("unknown stathead pack '{slug}'. Known packs: {known}")
        })
}

fn print_pack_list() {
    println!("Stathead query packs");
    println!();
    for pack in PACKS {
        println!("  {:<16} {}", pack.slug, pack.title);
        println!("  {:<16} {}", "", pack.description);
    }
    println!();
    println!("Show a pack: icelines stathead <pack>");
    println!("JSON:        icelines stathead --json");
    println!("Markdown:    icelines stathead --markdown --out stathead-packs.md");
    println!("Commands:    icelines stathead --commands --read-only");
    println!("File writes: icelines stathead --commands --writes-only");
}

fn print_pack(pack: &StatheadPack) {
    println!("{} ({})", pack.title, pack.slug);
    println!("{}", pack.description);
    println!();
    for query in pack.queries {
        println!("{}:", query.label);
        println!("  {}", query.command);
        println!("  {}", query.why);
        println!("  Requires: {}", query.requires);
        println!("  Effect: {}", query.effect);
        println!();
    }
}

fn render_pack_markdown(pack: &StatheadPack) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", pack.title));
    out.push_str(&format!("_Pack: `{}`_\n\n", pack.slug));
    out.push_str(pack.description);
    out.push_str("\n\n");
    out.push_str("| Query | Command | Why | Requires | Effect |\n");
    out.push_str("|---|---|---|---|---|\n");
    for query in pack.queries {
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            escape_markdown_table(query.label),
            escape_markdown_table(query.command),
            escape_markdown_table(query.why),
            escape_markdown_table(query.requires),
            escape_markdown_table(query.effect)
        ));
    }
    out
}

fn render_all_packs_markdown() -> String {
    let mut out = String::new();
    out.push_str("# IceLines stathead query packs\n\n");
    out.push_str("Curated editorial/stathead query recipes over existing IceLines commands.\n\n");
    for (index, pack) in PACKS.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&render_pack_markdown(pack).replacen("# ", "## ", 1));
    }
    out
}

fn emit_markdown(pack: &StatheadPack, out: Option<PathBuf>) -> Result<()> {
    let markdown = render_pack_markdown(pack);
    emit_markdown_text(markdown, out)
}

fn emit_markdown_index(out: Option<PathBuf>) -> Result<()> {
    let markdown = render_all_packs_markdown();
    emit_markdown_text(markdown, out)
}

fn emit_markdown_text(markdown: String, out: Option<PathBuf>) -> Result<()> {
    match out {
        Some(path) => std::fs::write(&path, markdown)
            .with_context(|| format!("writing stathead markdown to {}", path.display()))?,
        None => print!("{markdown}"),
    }
    Ok(())
}

fn render_commands(packs: &[&StatheadPack], effect_filter: CommandEffectFilter) -> String {
    packs
        .iter()
        .flat_map(|pack| {
            pack.queries
                .iter()
                .filter(move |query| match effect_filter {
                    CommandEffectFilter::All => true,
                    CommandEffectFilter::ReadOnly => is_read_only(query),
                    CommandEffectFilter::WritesOnly => !is_read_only(query),
                })
                .map(|query| query.command)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn emit_commands(
    packs: &[&StatheadPack],
    effect_filter: CommandEffectFilter,
    out: Option<PathBuf>,
) -> Result<()> {
    let mut commands = render_commands(packs, effect_filter);
    commands.push('\n');
    match out {
        Some(path) => std::fs::write(&path, commands)
            .with_context(|| format!("writing stathead commands to {}", path.display()))?,
        None => print!("{commands}"),
    }
    Ok(())
}

fn is_read_only(query: &StatheadQuery) -> bool {
    query.effect == READ_ONLY_EFFECT
}

fn escape_markdown_table(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_stathead_packs_have_unique_slugs() {
        let mut slugs = PACKS.iter().map(|pack| pack.slug).collect::<Vec<_>>();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), PACKS.len());
    }

    #[test]
    fn l0_stathead_packs_are_non_empty() {
        for pack in PACKS {
            assert!(!pack.title.is_empty());
            assert!(!pack.description.is_empty());
            assert!(
                !pack.queries.is_empty(),
                "pack {} should include at least one query",
                pack.slug
            );
            for query in pack.queries {
                assert!(
                    !query.requires.is_empty(),
                    "query {} should name data requirements",
                    query.label
                );
                assert!(
                    !query.effect.is_empty(),
                    "query {} should name command effects",
                    query.label
                );
            }
        }
    }

    #[test]
    fn l0_find_pack_is_case_insensitive() {
        let pack = find_pack("YOUNG-STARS").expect("pack should resolve");
        assert_eq!(pack.slug, "young-stars");
    }

    #[test]
    fn l0_unknown_pack_names_known_packs() {
        let err = find_pack("missing").expect_err("unknown pack should fail");
        let msg = err.to_string();
        assert!(msg.contains("unknown stathead pack 'missing'"));
        assert!(msg.contains("young-stars"));
    }

    #[test]
    fn l0_render_pack_markdown_contains_commands() {
        let pack = find_pack("young-stars").expect("pack should resolve");
        let markdown = render_pack_markdown(pack);
        assert!(markdown.contains("# Young stars"));
        assert!(markdown.contains("| Query | Command | Why | Requires | Effect |"));
        assert!(markdown.contains("icelines query leaders --age-max 23"));
        assert!(markdown.contains("Current configured season"));
        assert!(markdown.contains("read-only stdout"));
    }

    #[test]
    fn l0_escape_markdown_table_escapes_pipes_and_newlines() {
        assert_eq!(escape_markdown_table("a|b\nc"), "a\\|b c");
    }

    #[test]
    fn l0_render_all_packs_markdown_contains_every_pack() {
        let markdown = render_all_packs_markdown();
        assert!(markdown.contains("# IceLines stathead query packs"));
        for pack in PACKS {
            assert!(
                markdown.contains(&format!("## {}", pack.title)),
                "missing pack {}",
                pack.slug
            );
        }
    }

    #[test]
    fn l0_render_commands_returns_one_command_per_line() {
        let pack = find_pack("goalie-notebook").expect("pack should resolve");
        let commands = render_commands(&[pack], CommandEffectFilter::All);
        let lines = commands.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), pack.queries.len());
        assert_eq!(
            lines[0],
            "icelines query goalies --filter \"save-pct>=0.91\" --top 25"
        );
    }

    #[test]
    fn l0_stathead_commands_have_supported_shape() {
        for pack in PACKS {
            for query in pack.queries {
                let args = split_command(query.command);
                assert_eq!(
                    args.first().map(String::as_str),
                    Some("icelines"),
                    "stathead command should start with binary name: {}",
                    query.command
                );
                assert!(
                    matches!(
                        args.get(1).map(String::as_str),
                        Some(
                            "class"
                                | "peers"
                                | "playoffs"
                                | "poach"
                                | "query"
                                | "records"
                                | "report"
                                | "scouting"
                        )
                    ),
                    "stathead command uses unsupported top-level command for pack {} / query {}: {}",
                    pack.slug,
                    query.label,
                    query.command
                );
            }
        }
    }

    #[test]
    fn l0_render_commands_can_filter_to_read_only() {
        let pack = find_pack("fantasy-prep").expect("pack should resolve");
        let commands = render_commands(&[pack], CommandEffectFilter::ReadOnly);
        assert!(commands.contains("icelines poach --category hits,blocks,shots --top 20"));
        assert!(!commands.contains("weekly.md"));
    }

    #[test]
    fn l0_render_commands_can_filter_to_writes_only() {
        let pack = find_pack("fantasy-prep").expect("pack should resolve");
        let commands = render_commands(&[pack], CommandEffectFilter::WritesOnly);
        assert_eq!(
            commands,
            "icelines report weekly --category hits,blocks --top 10 --out weekly.md"
        );
    }

    #[test]
    fn l0_stathead_effects_use_known_shapes() {
        for pack in PACKS {
            for query in pack.queries {
                assert!(
                    is_read_only(query) || query.effect.starts_with("writes "),
                    "query {} has unsupported effect shape: {}",
                    query.label,
                    query.effect
                );
            }
        }
    }

    fn split_command(command: &str) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        for ch in command.chars() {
            match ch {
                '"' => in_quotes = !in_quotes,
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        args.push(std::mem::take(&mut current));
                    }
                }
                _ => current.push(ch),
            }
        }
        if !current.is_empty() {
            args.push(current);
        }
        assert!(
            !in_quotes,
            "stathead command has unmatched quotes: {command}"
        );
        args
    }
}
