use crate::config::Config;
use anyhow::Context;
use icelines_core::model::{Season, TeamAbbr};
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    build_cap_projection, CapProjectionContractInput, CapProjectionPlayerInput, CapProjectionView,
    SalaryBasis,
};
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::load_into_repo;
use serde::Serialize;
use std::fmt::Write as _;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
struct ReportCatalogEntry {
    name: &'static str,
    status: &'static str,
    canonical: &'static str,
    formats: &'static str,
    screens: &'static str,
    notes: &'static str,
}

const REPORT_CATALOG: &[ReportCatalogEntry] = &[
    ReportCatalogEntry {
        name: "cap-forecast",
        status: "available",
        canonical: "icelines report cap-forecast [--team NYR] [--json]",
        formats: "text,json",
        screens: "CLI durable report",
        notes: "Five-year current-roster market-cost scenario with confirmed/modelled values.",
    },
    ReportCatalogEntry {
        name: "leaderboards",
        status: "available",
        canonical: "icelines query leaders | icelines x leaders | icelines export md leaders",
        formats: "table,json,csv,markdown",
        screens: "TUI Stats, web /leaders",
        notes: "Use query for filters; x for quick CSV/JSON; export md for durable docs.",
    },
    ReportCatalogEntry {
        name: "goalies",
        status: "available",
        canonical: "icelines query goalies | icelines x goalies",
        formats: "table,json,csv",
        screens: "TUI Goalies",
        notes: "Goalie filters route through query goalies.",
    },
    ReportCatalogEntry {
        name: "player",
        status: "available",
        canonical: "icelines query player <name> | icelines history <name> | icelines x history",
        formats: "table,json,csv",
        screens: "TUI/Web player card",
        notes: "Player card is the screen; history is the exportable season log.",
    },
    ReportCatalogEntry {
        name: "compare",
        status: "available",
        canonical:
            "icelines query compare <a> <b> | icelines x compare | icelines export md compare",
        formats: "table,json,csv,markdown",
        screens: "TUI handoff, web compare",
        notes: "Use query compare for interactive output; export md for report packets.",
    },
    ReportCatalogEntry {
        name: "team",
        status: "available",
        canonical: "icelines team <ABBR> | icelines export md team",
        formats: "table,markdown",
        screens: "TUI Team/Depth, web team",
        notes: "Team depth remains the roster/depth view.",
    },
    ReportCatalogEntry {
        name: "team-season",
        status: "available",
        canonical: "icelines team-season <ABBR> | icelines export md team-season",
        formats: "table,json,markdown",
        screens: "TUI/Web team season",
        notes: "Season record, splits, form, remaining schedule, and opponent context.",
    },
    ReportCatalogEntry {
        name: "fantasy-poach",
        status: "available",
        canonical: "icelines poach | icelines report poach | icelines export md fantasy",
        formats: "table,json,markdown",
        screens: "TUI Poach, web /poach",
        notes: "Report poach emits a durable PoachReportView document.",
    },
    ReportCatalogEntry {
        name: "weekly-fantasy",
        status: "available",
        canonical: "icelines report weekly",
        formats: "markdown,json",
        screens: "web /reports/weekly",
        notes: "Weekly prep report over the same poach ViewModel plus watch context.",
    },
    ReportCatalogEntry {
        name: "draft-class",
        status: "available",
        canonical: "icelines class <year> | icelines x class",
        formats: "table,json,csv",
        screens: "TUI command handoff",
        notes: "Draft-year cohort ranking.",
    },
    ReportCatalogEntry {
        name: "peers",
        status: "available",
        canonical: "icelines peers <name> | icelines x peers",
        formats: "table,json,csv",
        screens: "TUI command handoff",
        notes: "Statistical similarity cohort.",
    },
    ReportCatalogEntry {
        name: "transactions",
        status: "available",
        canonical: "icelines transactions | icelines x transactions",
        formats: "table,json,csv",
        screens: "TUI Transactions, web /transactions",
        notes: "League/team/player transaction feed.",
    },
    ReportCatalogEntry {
        name: "records",
        status: "available",
        canonical: "icelines records player <name> | icelines records team <ABBR>",
        formats: "table,json,csv",
        screens: "future Player Records / Team Records",
        notes:
            "Available: teams/goalies scored against plus fight opponents from cached play-by-play.",
    },
    ReportCatalogEntry {
        name: "stathead-packs",
        status: "available",
        canonical:
            "icelines stathead | icelines stathead --markdown | icelines stathead --commands --read-only | icelines stathead --commands --writes-only",
        formats: "text,json,markdown,commands",
        screens: "CLI docs/report discovery",
        notes: "Curated editorial query recipes; use --commands --read-only or --writes-only to filter by command effect.",
    },
];

#[derive(Debug, Clone)]
pub struct CapForecastArgs {
    pub season: String,
    pub years: u8,
    pub growth_pct: f64,
    pub team: Option<String>,
    pub json: bool,
    pub out: Option<PathBuf>,
}

pub fn run_cap_forecast(args: CapForecastArgs) -> anyhow::Result<()> {
    let base_season: Season = args
        .season
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid forecast season '{}': {error}", args.season))?;
    let selected_team = args
        .team
        .as_deref()
        .map(TeamAbbr::parse)
        .transpose()
        .map_err(|error| anyhow::anyhow!(error))?;

    let cfg = Config::load()?;
    let stats_season: Season = cfg
        .season_str()
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{}' is not a YYYYZZZZ id", cfg.season_str()))?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome = load_into_repo(stats_season, SeasonType::Regular, &store)
        .map_err(|error| anyhow::anyhow!("{error}\n  Try: icelines fetch all"))?;

    let mut players = Vec::new();
    let teams: Vec<_> = match selected_team.as_ref() {
        Some(team) => vec![team.clone()],
        None => TeamAbbr::all().collect(),
    };
    for team in teams {
        for view in outcome
            .repo
            .team_roster(&team, stats_season, SeasonType::Regular)
        {
            let age = age_at_season_start(view.identity.bio.birth_date.as_deref(), base_season);
            players.push(CapProjectionPlayerInput {
                player_id: view.id().0,
                player: view.full_name().to_owned(),
                team: team.as_str().to_owned(),
                position: view.position(),
                age,
                games_played: view.gp(),
                points_per_82: view.pace_score().map(|score| score.pace_82),
                contract: view.contract.map(|contract| CapProjectionContractInput {
                    valuation_season: contract
                        .valuation_season
                        .as_deref()
                        .and_then(|value| value.parse().ok()),
                    expiry_year: contract.expiry_year,
                    cap_hit: contract.cap_hit,
                    aav: contract.aav,
                    source: contract.source.clone(),
                    source_url: contract.source_url.clone(),
                }),
            });
        }
    }
    if players.is_empty() {
        let suffix = selected_team
            .as_ref()
            .map(|team| format!(" for {}", team.as_str()))
            .unwrap_or_default();
        anyhow::bail!("no current-roster players found{suffix} — run `icelines fetch all`");
    }

    let view = build_cap_projection(players, base_season, args.years, args.growth_pct)?;
    let output = if args.json {
        format!("{}\n", serde_json::to_string_pretty(&view)?)
    } else {
        render_cap_forecast(&view, selected_team.as_ref().map(TeamAbbr::as_str))
    };
    emit_report(&output, args.out.as_ref())
}

fn age_at_season_start(birth_date: Option<&str>, season: Season) -> u8 {
    birth_date
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse::<u16>().ok())
        .map(|year| season.start_year().saturating_sub(year).min(99) as u8)
        .unwrap_or(27)
}

fn emit_report(output: &str, path: Option<&PathBuf>) -> anyhow::Result<()> {
    match path.and_then(|path| path.to_str()) {
        None | Some("-") => print!("{output}"),
        Some(_) => {
            let path = path.expect("path checked above");
            std::fs::write(path, output)
                .with_context(|| format!("writing cap forecast to {}", path.display()))?;
            println!("Wrote cap forecast to {}", path.display());
        }
    }
    Ok(())
}

fn render_cap_forecast(view: &CapProjectionView, selected_team: Option<&str>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "FIVE-YEAR ROSTER MARKET-COST FORECAST");
    let _ = writeln!(out, "Schema: {}", view.schema);
    let _ = writeln!(out, "Method: {}", view.method);
    let _ = writeln!(
        out,
        "Scenario growth after announced limits: {:.1}%",
        view.assumptions.modeled_growth_pct
    );
    let _ = writeln!(out, "Market anchor: {}", view.assumptions.market_anchor);
    let _ = writeln!(
        out,
        "Cap-limit source: {}",
        view.assumptions.cap_limit_source_url
    );
    let _ = writeln!(
        out,
        "Market-anchor source: {}",
        view.assumptions.market_anchor_url
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:<9} {:<4} {:>7} {:>6} {:>3} {:>4} {:>4} {:>11} {:>7}  Pressure",
        "Season", "Team", "Cap $M", "Active", "Out", "Conf", "Model", "Spend $M", "Share"
    );
    let _ = writeln!(out, "{}", "-".repeat(88));

    let mut summaries: Vec<_> = view
        .teams
        .iter()
        .flat_map(|team| team.seasons.iter().map(move |season| (&team.team, season)))
        .collect();
    summaries.sort_by(|(team_a, a), (team_b, b)| {
        a.season
            .cmp(&b.season)
            .then_with(|| {
                b.cap_share_pct
                    .partial_cmp(&a.cap_share_pct)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| team_a.cmp(team_b))
    });
    for (team, row) in summaries {
        let _ = writeln!(
            out,
            "{:<9} {:<4} {:>7.1} {:>6} {:>3} {:>4} {:>4} {:>11.1} {:>6.1}%  {}",
            season_label(row.season),
            team,
            millions(row.upper_limit),
            row.roster_players,
            row.excluded_depth_players,
            row.confirmed_players,
            row.modeled_players,
            millions(row.projected_cap_hit),
            row.cap_share_pct,
            row.pressure.label()
        );
    }

    if let Some(team) = selected_team {
        if let Some(team_view) = view.teams.iter().find(|row| row.team == team) {
            if let Some(first) = team_view.seasons.first() {
                let _ = writeln!(out);
                let _ = writeln!(
                    out,
                    "{} PLAYER MARKET — {}",
                    team,
                    season_label(first.season)
                );
                let _ = writeln!(
                    out,
                    "{:<24} {:<4} {:<16} {:<9} {:>8} {:>17}",
                    "Player", "Pos", "Role", "Basis", "Mid $M", "Low-high $M"
                );
                let _ = writeln!(out, "{}", "-".repeat(84));
                for player in &first.players {
                    let basis = match player.salary_basis {
                        SalaryBasis::Confirmed => "confirmed",
                        SalaryBasis::Modeled => "modeled",
                    };
                    let _ = writeln!(
                        out,
                        "{:<24} {:<4} {:<16} {:<9} {:>8.2} {:>8.2}-{:>8.2}",
                        truncate(&player.player, 24),
                        player.position.abbreviation(),
                        player.role.label(),
                        basis,
                        millions(player.projected_cap_hit),
                        millions(player.projected_cap_hit_low),
                        millions(player.projected_cap_hit_high)
                    );
                }
            }
        }
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "DISCLOSURES");
    for disclosure in &view.disclosures {
        let _ = writeln!(out, "- {disclosure}");
    }
    let _ = writeln!(out, "NON-CLAIMS");
    for non_claim in &view.non_claims {
        let _ = writeln!(out, "- {non_claim}");
    }
    out
}

fn millions(value: u64) -> f64 {
    value as f64 / 1_000_000.0
}

fn season_label(season: u32) -> String {
    let text = season.to_string();
    if text.len() == 8 {
        format!("{}-{}", &text[..4], &text[6..])
    } else {
        text
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_owned()
    } else {
        let mut output: String = value.chars().take(width.saturating_sub(1)).collect();
        output.push('…');
        output
    }
}

pub fn run_list(json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(REPORT_CATALOG)?);
        return Ok(());
    }

    println!("IceLines report surface");
    println!();
    println!("Use `query` when you are asking a question, `x` when you want CSV/JSON,");
    println!("`export md` when you want a markdown packet, `report` for durable decision");
    println!("reports, and `stathead` for curated editorial query recipes.");
    println!();
    println!(
        "{:<16} {:<10} {:<26} Canonical command",
        "Report", "Status", "Formats"
    );
    println!("{:-<16} {:-<10} {:-<26} {:-<1}", "", "", "", "");
    for entry in REPORT_CATALOG {
        println!(
            "{:<16} {:<10} {:<26} {}",
            entry.name, entry.status, entry.formats, entry.canonical
        );
    }
    println!();
    println!("Available records examples:");
    println!("  icelines records player \"Andre Burakovsky\" --metric teams-scored-against");
    println!("  icelines records player \"Andre Burakovsky\" --metric goalies-scored-against");
    println!("  icelines records player \"Andre Burakovsky\" --metric fight-opponents");
    println!("  icelines records team SEA --metric players-scored-against-team");
    println!("  icelines records team SEA --metric goalies-beaten-by-team");
    println!("  icelines records team SEA --metric fight-opponents-by-team");
    println!();
    println!("Stathead starter examples:");
    println!("  icelines stathead");
    println!("  icelines stathead --markdown --out stathead-packs.md");
    Ok(())
}
