//! class, peers, compare, history, group commands.

use crate::cli::GroupSubcommand;
use crate::commands::players::load_all_players;
use crate::db::GroupDb;
use anyhow::{bail, Context};
use icelines_core::{
    filter::PlayerFilter, model::Player, name::normalize_name, position::PositionResolver,
};

// ── icelines class ────────────────────────────────────────────────────────────

pub async fn run_class(
    year: u16,
    pos: Option<String>,
    top: Option<usize>,
    _json: bool,
) -> anyhow::Result<()> {
    let players = load_all_players()?;

    let mut filter = PlayerFilter::new();
    filter.draft_years = Some(vec![year]);
    if let Some(p) = pos {
        if let Ok((primary, _)) = PositionResolver::parse(&p) {
            filter.positions = Some(vec![primary]);
        }
    }

    let matched = filter.apply(&players);
    let limit = top.unwrap_or(matched.len());

    println!("DRAFT CLASS {year} — {} players", matched.len());
    println!("{}", "─".repeat(72usize));
    println!(
        "{:<5} {:<24} {:<5} {:<4} {:<5} {:<7} {:<8}",
        "Pick", "Player", "Team", "Pos", "Age", "PPG", "Proj/82"
    );
    println!("{}", "─".repeat(72usize));

    for p in matched.iter().take(limit) {
        let pick = p
            .draft_overall
            .map(|n| format!("#{n}"))
            .unwrap_or_else(|| "UD".to_owned());
        let age = age_from_birth_date(p);
        let (ppg, proj) = pace_strings(p);
        println!(
            "{:<5} {:<24} {:<5} {:<4} {:<5} {:<7} {:<8}",
            pick,
            p.full_name,
            p.team.as_str(),
            p.position.abbreviation(),
            age,
            ppg,
            proj
        );
    }
    Ok(())
}

// ── icelines peers ─────────────────────────────────────────────────────────────

pub async fn run_peers(player_name: String, size: usize, _json: bool) -> anyhow::Result<()> {
    let players = load_all_players()?;
    let target = find_player(&players, &player_name)?;

    // Peer group: same draft year ± 1 and same position
    let draft_year = target.draft_year.unwrap_or(0);
    let mut filter = PlayerFilter::new();
    filter.positions = Some(vec![target.position]);
    filter.draft_years = Some(
        vec![draft_year.saturating_sub(1), draft_year, draft_year + 1]
            .into_iter()
            .filter(|&y| y > 0)
            .collect(),
    );

    let mut peers: Vec<&Player> = filter.apply(&players);
    peers.sort_by(|a, b| {
        let sa = a.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
        let sb = b.pace_score.map(|s| s.pace_82).unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let target_rank = peers
        .iter()
        .position(|p| p.name_normalized == target.name_normalized)
        .map(|i| i + 1)
        .unwrap_or(0);

    println!(
        "PEERS OF {} ({} · {:?} · Draft {})",
        target.full_name,
        target.team.as_str(),
        target.position,
        draft_year
    );
    println!(
        "Peer group: Draft class {}-{} at {:?}",
        draft_year.saturating_sub(1),
        draft_year + 1,
        target.position
    );
    println!("{}", "─".repeat(64usize));
    println!(
        "{:<5} {:<24} {:<5} {:<7} {:<8}",
        "Rank", "Player", "Team", "PPG", "Proj/82"
    );
    println!("{}", "─".repeat(64usize));

    for (i, p) in peers.iter().take(size).enumerate() {
        let marker = if p.name_normalized == target.name_normalized {
            " ←"
        } else {
            ""
        };
        let (ppg, proj) = pace_strings(p);
        println!(
            "{:<5} {:<24} {:<5} {:<7} {:<8}{}",
            i + 1,
            p.full_name,
            p.team.as_str(),
            ppg,
            proj,
            marker
        );
    }
    println!(
        "\n{} in peer group. {} ranks #{} of {}.",
        target.full_name,
        target.full_name,
        target_rank,
        peers.len()
    );
    Ok(())
}

// ── icelines compare ───────────────────────────────────────────────────────────

pub async fn run_compare(name1: String, name2: String, _json: bool) -> anyhow::Result<()> {
    let players = load_all_players()?;
    let p1 = find_player(&players, &name1)?;
    let p2 = find_player(&players, &name2)?;

    let (ppg1, proj1) = pace_strings(p1);
    let (ppg2, proj2) = pace_strings(p2);

    println!(
        "{:<28} {:<28}",
        p1.full_name.as_str(),
        p2.full_name.as_str()
    );
    println!("{:<28} {:<28}", p1.team.as_str(), p2.team.as_str());
    println!("{}", "─".repeat(60usize));

    let row = |label: &str, v1: &str, v2: &str| {
        println!("  {:<18} {:<18} {:<18}", label, v1, v2);
    };

    row(
        "Position",
        p1.position.abbreviation(),
        p2.position.abbreviation(),
    );
    row("Age", &age_from_birth_date(p1), &age_from_birth_date(p2));
    row("Draft", &draft_str(p1), &draft_str(p2));
    row("PPG", &ppg1, &ppg2);
    row("Proj/82g", &proj1, &proj2);
    row(
        "Goals/82g",
        &p1.pace_score
            .map(|s| format!("{:.1}", s.goals_per_82))
            .unwrap_or_else(|| "—".to_owned()),
        &p2.pace_score
            .map(|s| format!("{:.1}", s.goals_per_82))
            .unwrap_or_else(|| "—".to_owned()),
    );

    Ok(())
}

// ── icelines history ───────────────────────────────────────────────────────────

pub async fn run_history(player_name: String, _json: bool) -> anyhow::Result<()> {
    use icelines_fetch::career::load_career;
    use crate::config::Config;
    use icelines_fetch::snapshot::SnapshotStore;

    let cfg   = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());

    let summary = load_career(&player_name, 5, &store)
        .with_context(|| format!("'{player_name}' not found in bundled season data"))?;

    println!("CAREER HISTORY — {} (last {} seasons)",
        summary.full_name, summary.seasons.len());
    println!("{}", "─".repeat(64usize));
    println!("{:<10} {:<6} {:<4} {:<4} {:<4} {:<8} {:<8}",
        "Season", "Team", "GP", "G", "A", "PPG", "Pts/82");
    println!("{}", "─".repeat(64usize));

    for line in &summary.seasons {
        let label = if line.season.len() == 8 {
            format!("{}-{}", &line.season[2..4], &line.season[6..8])
        } else { line.season.clone() };
        println!("{:<10} {:<6} {:<4} {:<4} {:<4} {:<8.3} {:<8.1}",
            label, line.team, line.gp, line.goals, line.assists,
            line.ppg, line.pts_per_82());
    }
    println!("{}", "─".repeat(64usize));
    let peak_label = {
        let s = &summary.peak_season;
        if s.len() == 8 { format!("{}-{}", &s[2..4], &s[6..8]) } else { s.clone() }
    };
    println!("Career:  {:.3} pts/gp  |  Peak: {} ({:.3} pts/gp)",
        summary.career_ppg, peak_label, summary.peak_ppg);
    Ok(())
}

// ── icelines group ─────────────────────────────────────────────────────────────

/// Group management backed by SQLite (`~/.icelines/icelines.db`).
pub async fn run_group(cmd: GroupSubcommand) -> anyhow::Result<()> {
    let db = GroupDb::open()?;

    match cmd {
        GroupSubcommand::Create { name, desc } => {
            db.create_group(&name, &desc.unwrap_or_default())?;
            println!("Group '{name}' created.");
        }
        GroupSubcommand::Add { group, player } => {
            let norm = normalize_name(&player);
            let added = db.add_member(&group, &norm)?;
            if added {
                println!("Added '{player}' to '{group}'.");
            } else {
                println!("'{player}' is already in '{group}' — no change.");
            }
        }
        GroupSubcommand::Remove { group, player } => {
            let norm = normalize_name(&player);
            db.remove_member(&group, &norm)?;
            println!("Removed '{player}' from '{group}'.");
        }
        GroupSubcommand::List => {
            let groups = db.list_groups()?;
            if groups.is_empty() {
                println!("No groups. Create one with `icelines group create`.");
                return Ok(());
            }
            println!("{:<24} {:<8} {:<40}", "Group", "Members", "Description");
            println!("{}", "─".repeat(56usize));
            for g in &groups {
                println!("{:<24} {:<8} {}", g.name, g.member_count, g.description);
            }
        }
        GroupSubcommand::Show { name } => {
            let members = db.list_members(&name)?;
            println!("GROUP: {name}  ({} members)", members.len());

            // Show description from list_groups.
            let all = db.list_groups()?;
            if let Some(g) = all.iter().find(|g| g.name == name) {
                if !g.description.is_empty() {
                    println!("  {}", g.description);
                }
            }
            println!("{}", "─".repeat(56usize));

            if members.is_empty() {
                println!("  (empty)");
                return Ok(());
            }

            let players = load_all_players()?;
            for norm in &members {
                if let Some(p) = players.iter().find(|p| &p.name_normalized == norm) {
                    let (ppg, proj) = pace_strings(p);
                    println!(
                        "  {:<24} {:<5} {:<4} {} / {}",
                        p.full_name,
                        p.team.as_str(),
                        p.position.abbreviation(),
                        ppg,
                        proj
                    );
                } else {
                    println!("  {norm}  (not in current snapshot)");
                }
            }
        }
        GroupSubcommand::Delete { name } => {
            let found = db.delete_group(&name)?;
            if found {
                println!("Group '{name}' deleted.");
            } else {
                bail!("group '{name}' not found");
            }
        }
    }
    Ok(())
}

// ── Shared helpers ─────────────────────────────────────────────────────────────

fn find_player<'a>(players: &'a [Player], name: &str) -> anyhow::Result<&'a Player> {
    let norm = normalize_name(name);
    players
        .iter()
        .find(|p| p.name_normalized.contains(&norm))
        .with_context(|| format!("player '{name}' not found in snapshot — try a partial name"))
}

fn pace_strings(p: &Player) -> (String, String) {
    match p.pace_score {
        Some(s) => (
            format!("{:.2}", s.pace_82 / 82.0),
            format!("{:.0}", s.pace_82),
        ),
        None => ("—".to_owned(), "—".to_owned()),
    }
}

fn age_from_birth_date(p: &Player) -> String {
    p.birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string())
        .unwrap_or_else(|| "—".to_owned())
}

fn draft_str(p: &Player) -> String {
    match (p.draft_year, p.draft_round, p.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r} #{o}"),
        (Some(y), _, _) => format!("{y}"),
        _ => "Undrafted".to_owned(),
    }
}
