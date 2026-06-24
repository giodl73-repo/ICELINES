use serde::Serialize;

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
