//! class, peers, compare, history, group commands.

use crate::cli::GroupSubcommand;
use crate::commands::players::load_all_players;
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
    let players = load_all_players()?;
    let p = find_player(&players, &player_name)?;

    // Current season from snapshot — multi-season requires player landing API (Phase 3)
    println!(
        "CAREER HISTORY — {} ({} · {:?})",
        p.full_name,
        p.team.as_str(),
        p.position
    );
    println!("{}", "─".repeat(60usize));
    println!(
        "{:<10} {:<6} {:<4} {:<4} {:<4} {:<7}",
        "Season", "Team", "GP", "G", "A", "Proj/82"
    );
    println!("{}", "─".repeat(60usize));

    if let Some(s) = p.pace_score {
        let (_, proj) = pace_strings(p);
        println!(
            "{:<10} {:<6} {:<4} {:<4} {:<4} {:<7}",
            "2025-26",
            p.team.as_str(),
            s.gp,
            p.season_goals,
            p.season_assists,
            proj
        );
    } else {
        println!(
            "No pace-eligible season data available (GP < {}).",
            icelines_core::model::MIN_GP
        );
    }
    println!("\nNote: multi-season history requires `icelines fetch history` (Phase 3).");
    Ok(())
}

// ── icelines group ─────────────────────────────────────────────────────────────

/// In-memory group store for Phase 2 — persisted to ~/.icelines/groups.json.
/// SQLite persistence is added in task #18.
pub async fn run_group(cmd: GroupSubcommand) -> anyhow::Result<()> {
    let groups_path = {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_owned());
        std::path::PathBuf::from(home)
            .join(".icelines")
            .join("groups.json")
    };

    let mut groups: serde_json::Value = if groups_path.exists() {
        let raw = std::fs::read_to_string(&groups_path)?;
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let save = |groups: &serde_json::Value| -> anyhow::Result<()> {
        if let Some(parent) = groups_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&groups_path, serde_json::to_string_pretty(groups)?)?;
        Ok(())
    };

    match cmd {
        GroupSubcommand::Create { name, desc } => {
            groups[&name] = serde_json::json!({
                "description": desc.unwrap_or_default(),
                "members": []
            });
            save(&groups)?;
            println!("Group '{name}' created.");
        }
        GroupSubcommand::Add { group, player } => {
            let members = groups[&group]["members"]
                .as_array_mut()
                .with_context(|| format!("group '{group}' not found"))?;
            let norm = normalize_name(&player);
            if !members.iter().any(|m| m.as_str() == Some(&norm)) {
                members.push(serde_json::Value::String(norm));
                save(&groups)?;
                println!("Added '{player}' to '{group}'.");
            } else {
                println!("'{player}' is already in '{group}'.");
            }
        }
        GroupSubcommand::Remove { group, player } => {
            let members = groups[&group]["members"]
                .as_array_mut()
                .with_context(|| format!("group '{group}' not found"))?;
            let norm = normalize_name(&player);
            members.retain(|m| m.as_str() != Some(&norm));
            save(&groups)?;
            println!("Removed '{player}' from '{group}'.");
        }
        GroupSubcommand::List => {
            let map = groups.as_object().context("corrupt groups file")?;
            if map.is_empty() {
                println!("No groups. Create one with `icelines group create`.");
                return Ok(());
            }
            println!("{:<24} {:<8} {:<40}", "Group", "Members", "Description");
            println!("{}", "─".repeat(56usize));
            for (name, g) in map {
                let count = g["members"].as_array().map(|a| a.len()).unwrap_or(0);
                let desc = g["description"].as_str().unwrap_or("");
                println!("{:<24} {:<8} {}", name, count, desc);
            }
        }
        GroupSubcommand::Show { name } => {
            let g = &groups[&name];
            if g.is_null() {
                bail!("group '{name}' not found");
            }
            let members = g["members"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            println!("GROUP: {name}  ({} members)", members.len());
            if let Some(d) = g["description"].as_str() {
                if !d.is_empty() {
                    println!("  {d}");
                }
            }
            println!("{}", "─".repeat(56usize));

            if members.is_empty() {
                println!("  (empty)");
                return Ok(());
            }

            let players = load_all_players()?;
            for m in members {
                let norm = m.as_str().unwrap_or("");
                if let Some(p) = players.iter().find(|p| p.name_normalized == norm) {
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
            if groups
                .as_object_mut()
                .and_then(|m| m.remove(&name))
                .is_some()
            {
                save(&groups)?;
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
