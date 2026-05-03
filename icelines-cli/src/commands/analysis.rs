//! class, peers, compare, history, group commands.

use crate::cli::GroupSubcommand;
use crate::config::Config;
use crate::db::GroupDb;
use anyhow::{bail, Context};
use icelines_core::filter::PlayerFilter;
use icelines_core::model::Season;
use icelines_core::name::normalize_name;
use icelines_core::position::PositionResolver;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::PlayerView;
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::load_into_repo;

/// Hart.5b2: load all skaters as PlayerView for the configured season.
/// Caller holds the LoadOutcome alive so the views' borrows remain valid.
fn load_views() -> anyhow::Result<icelines_fetch::stats_loader::LoadOutcome> {
    let cfg = Config::load()?;
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .map_err(|_| anyhow::anyhow!("season '{}' is not a YYYYZZZZ id", cfg.season_str()))?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    load_into_repo(Season(season_u32), SeasonType::Regular, &store)
        .map_err(|e| anyhow::anyhow!("{e}\n  Try: icelines fetch all"))
}

// ── icelines class ────────────────────────────────────────────────────────────

pub async fn run_class(
    year: u16,
    pos: Option<String>,
    top: Option<usize>,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    let outcome = load_views()?;
    let cfg = Config::load()?;
    let season_u32: u32 = cfg.season_str().parse().unwrap();

    let mut filter = PlayerFilter::new();
    filter.draft_years = Some(vec![year]);
    if let Some(p) = pos {
        if let Ok((primary, _)) = PositionResolver::parse(&p) {
            filter.positions = Some(vec![primary]);
        }
    }

    let matched = filter.apply_views(
        outcome
            .repo
            .skaters(Season(season_u32), SeasonType::Regular),
    );
    let limit = top.unwrap_or(matched.len());

    if matched.is_empty() {
        let current_year = 2026u16;
        if year < current_year - 4 {
            eprintln!("DRAFT CLASS {year} — 0 active players found in installed seasons.");
            eprintln!("  Players from the {year} draft class may have retired before the bundled data window.");
            eprintln!("  Install older seasons to see historical draft class data:");
            eprintln!("    icelines data install <YYYYZZZZ>");
        } else {
            eprintln!("DRAFT CLASS {year} — 0 players matched. Check the draft year.");
        }
        return Ok(());
    }

    let headers = &["pick", "player", "team", "pos", "age", "ppg", "proj_82"];
    let rows: Vec<Vec<String>> = matched
        .iter()
        .take(limit)
        .map(|v| {
            let pick = v
                .identity
                .bio
                .draft_overall
                .map(|n| format!("#{n}"))
                .unwrap_or_else(|| "UD".to_owned());
            let (ppg, proj) = pace_strings_view(v);
            vec![
                pick,
                v.full_name().to_owned(),
                v.team_display().to_owned(),
                v.position().abbreviation().to_owned(),
                age_from_view(v),
                ppg,
                proj,
            ]
        })
        .collect();

    let format = Format::resolve(csv, json)?;
    if format == Format::Table && out.is_none() {
        println!("DRAFT CLASS {year} — {} players", matched.len());
    }
    format.emit_to(headers, &rows, out.as_deref())?;
    Ok(())
}

// ── icelines peers ─────────────────────────────────────────────────────────────

pub async fn run_peers(
    player_name: String,
    size: usize,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    let outcome = load_views()?;
    let cfg = Config::load()?;
    let season_u32: u32 = cfg.season_str().parse().unwrap();

    // Find target by partial name match across all skaters.
    let target_norm = normalize_name(&player_name);
    let target = outcome
        .repo
        .skaters(Season(season_u32), SeasonType::Regular)
        .find(|v| v.name_normalized().contains(&target_norm))
        .with_context(|| {
            format!("player '{player_name}' not found in snapshot — try a partial name")
        })?;

    // Peer group: same draft year ± 1 and same position
    let draft_year = target.identity.bio.draft_year.unwrap_or(0);
    let target_position = target.position();
    let target_full_name = target.full_name().to_owned();
    let target_team = target.team_display().to_owned();
    let target_norm_owned = target.name_normalized().to_owned();

    let mut filter = PlayerFilter::new();
    filter.positions = Some(vec![target_position]);
    filter.draft_years = Some(
        vec![draft_year.saturating_sub(1), draft_year, draft_year + 1]
            .into_iter()
            .filter(|&y| y > 0)
            .collect(),
    );

    let mut peers = filter.apply_views(
        outcome
            .repo
            .skaters(Season(season_u32), SeasonType::Regular),
    );
    peers.sort_by(|a, b| {
        let sa = a.pace_82().unwrap_or(0.0);
        let sb = b.pace_82().unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let target_rank = peers
        .iter()
        .position(|v| v.name_normalized() == target_norm_owned)
        .map(|i| i + 1)
        .unwrap_or(0);

    let headers = &["rank", "player", "team", "ppg", "proj_82", "is_target"];
    let rows: Vec<Vec<String>> = peers
        .iter()
        .take(size)
        .enumerate()
        .map(|(i, v)| {
            let (ppg, proj) = pace_strings_view(v);
            let is_target = if v.name_normalized() == target_norm_owned {
                "true"
            } else {
                "false"
            };
            vec![
                (i + 1).to_string(),
                v.full_name().to_owned(),
                v.team_display().to_owned(),
                ppg,
                proj,
                is_target.to_owned(),
            ]
        })
        .collect();

    let format = Format::resolve(csv, json)?;
    if format == Format::Table && out.is_none() {
        println!(
            "PEERS OF {} ({} · {:?} · Draft {})",
            target_full_name, target_team, target_position, draft_year,
        );
    }
    format.emit_to(headers, &rows, out.as_deref())?;
    if format == Format::Table && out.is_none() {
        println!(
            "\n{} ranks #{} of {} in the peer group.",
            target_full_name,
            target_rank,
            peers.len()
        );
    }
    Ok(())
}

// ── icelines compare ───────────────────────────────────────────────────────────

pub async fn run_compare(
    name1: String,
    name2: String,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;

    let outcome = load_views()?;
    let cfg = Config::load()?;
    let season_u32: u32 = cfg.season_str().parse().unwrap();
    let views: Vec<PlayerView<'_>> = outcome
        .repo
        .skaters(Season(season_u32), SeasonType::Regular)
        .collect();

    let v1 = find_view(&views, &name1)?;
    let v2 = find_view(&views, &name2)?;

    let (ppg1, proj1) = pace_strings_view(v1);
    let (ppg2, proj2) = pace_strings_view(v2);
    let goals1 = v1
        .pace_score()
        .map(|s| format!("{:.1}", s.goals_per_82))
        .unwrap_or_else(|| "—".to_owned());
    let goals2 = v2
        .pace_score()
        .map(|s| format!("{:.1}", s.goals_per_82))
        .unwrap_or_else(|| "—".to_owned());

    let p1_label = v1.full_name().to_owned();
    let p2_label = v2.full_name().to_owned();
    let headers: Vec<String> = vec!["stat".to_owned(), p1_label.clone(), p2_label.clone()];
    let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();
    let rows: Vec<Vec<String>> = vec![
        vec![
            "team".to_owned(),
            v1.team_display().to_owned(),
            v2.team_display().to_owned(),
        ],
        vec![
            "position".to_owned(),
            v1.position().abbreviation().to_owned(),
            v2.position().abbreviation().to_owned(),
        ],
        vec!["age".to_owned(), age_from_view(v1), age_from_view(v2)],
        vec!["draft".to_owned(), draft_str_view(v1), draft_str_view(v2)],
        vec!["ppg".to_owned(), ppg1, ppg2],
        vec!["proj_82".to_owned(), proj1, proj2],
        vec!["goals_82".to_owned(), goals1, goals2],
    ];

    let format = Format::resolve(csv, json)?;
    format.emit_to(&header_refs, &rows, out.as_deref())?;
    Ok(())
}

// ── icelines history ───────────────────────────────────────────────────────────

pub async fn run_history(
    player_name: String,
    seasons: usize,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    use crate::commands::output::Format;
    use crate::config::Config;
    use icelines_fetch::career::load_career;
    use icelines_fetch::snapshot::SnapshotStore;

    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());

    let summary = load_career(&player_name, seasons, &store)
        .with_context(|| format!(
            "'{player_name}' not found in installed seasons (checked last {seasons}).\n\
             Tip: install older seasons with `icelines data install <YYYYZZZZ>` to view historical players."
        ))?;

    let headers = &[
        "season",
        "team",
        "gp",
        "goals",
        "assists",
        "ppg",
        "pts_per_82",
    ];
    let rows: Vec<Vec<String>> = summary
        .seasons
        .iter()
        .map(|line| {
            vec![
                line.season.clone(),
                line.team.clone(),
                line.gp.to_string(),
                line.goals.to_string(),
                line.assists.to_string(),
                format!("{:.3}", line.ppg),
                format!("{:.1}", line.pts_per_82()),
            ]
        })
        .collect();

    let format = Format::resolve(csv, json)?;
    if format == Format::Table && out.is_none() {
        println!(
            "CAREER HISTORY — {} (last {} seasons)",
            summary.full_name,
            summary.seasons.len()
        );
    }
    format.emit_to(headers, &rows, out.as_deref())?;
    if format == Format::Table && out.is_none() {
        let peak_label = {
            let s = &summary.peak_season;
            if s.len() == 8 {
                format!("{}-{}", &s[2..4], &s[6..8])
            } else {
                s.clone()
            }
        };
        println!(
            "\nCareer: {:.3} pts/gp  |  Peak: {} ({:.3} pts/gp)",
            summary.career_ppg, peak_label, summary.peak_ppg
        );
    }
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

            let outcome = load_views()?;
            let cfg = Config::load()?;
            let season_u32: u32 = cfg.season_str().parse().unwrap();
            let views: Vec<PlayerView<'_>> = outcome
                .repo
                .skaters(Season(season_u32), SeasonType::Regular)
                .collect();
            for norm in &members {
                if let Some(v) = views
                    .iter()
                    .find(|v| v.name_normalized().contains(norm.as_str()))
                {
                    let (ppg, proj) = pace_strings_view(v);
                    println!(
                        "  {:<24} {:<5} {:<4} {} / {}",
                        v.full_name(),
                        v.team_display(),
                        v.position().abbreviation(),
                        ppg,
                        proj
                    );
                } else {
                    println!("  {norm}  (player not found — may have been traded or retired)");
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

        // ── Phase 8f.6: portable group I/O ──────────────────────────────────
        GroupSubcommand::Export { name, out } => {
            let payload = build_export_payload(&db, &name)?;
            let json =
                serde_json::to_string_pretty(&payload).context("serializing group export")?;
            if out == "-" {
                println!("{json}");
            } else {
                std::fs::write(&out, &json).with_context(|| format!("writing {out}"))?;
                println!(
                    "✓ Exported group '{}' ({} members) to {}",
                    payload.name,
                    payload.members.len(),
                    out
                );
            }
        }
        GroupSubcommand::Import { path, as_name } => {
            let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
            let mut payload: GroupExport = serde_json::from_str(&text)
                .with_context(|| format!("parsing {path} as a group export"))?;
            if let Some(rename) = as_name {
                payload.name = rename;
            }
            db.create_group(&payload.name, &payload.description)
                .with_context(|| format!("creating group '{}'", payload.name))?;
            let inserted = db.add_members_bulk(&payload.name, &payload.members)?;
            println!(
                "✓ Imported group '{}' — created with {} member(s).",
                payload.name, inserted
            );
        }
        GroupSubcommand::Rename { old, new } => {
            db.rename_group(&old, &new)?;
            println!("✓ Renamed group '{old}' → '{new}'.");
        }
    }
    Ok(())
}

// ── Group export payload (Phase 8f.6) ───────────────────────────────────────

/// Wire format for `group export` / `group import`. Stable by design — the
/// JSON is committable and shareable, so additive changes only.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct GroupExport {
    name: String,
    #[serde(default)]
    description: String,
    members: Vec<String>, // normalized names
    /// Schema version. Bump when the wire format changes.
    #[serde(default = "default_export_version")]
    version: u32,
}

fn default_export_version() -> u32 {
    1
}

fn build_export_payload(db: &GroupDb, name: &str) -> anyhow::Result<GroupExport> {
    let members = db.list_members(name)?;
    let description = db.group_description(name).unwrap_or_default();
    Ok(GroupExport {
        name: name.to_owned(),
        description,
        members,
        version: default_export_version(),
    })
}

// ── Attended games (Phase 8 follow-up) ────────────────────────────────────────

pub async fn run_games(cmd: crate::cli::GamesSubcommand) -> anyhow::Result<()> {
    use crate::cli::GamesSubcommand;
    use crate::db::AttendedGameInput;
    let db = GroupDb::open()?;
    match cmd {
        GamesSubcommand::Add { game_id, note } => {
            // Try to fetch boxscore metadata so the row is self-describing
            // even after the API rotates the box out. Failures are fine —
            // the row still records `game_id` and the user's note.
            let meta = lookup_game_meta(game_id).await;
            db.add_attended_game(&AttendedGameInput {
                game_id,
                game_date: meta.as_ref().and_then(|m| m.date.clone()),
                away_abbrev: meta.as_ref().map(|m| m.away_abbrev.clone()),
                home_abbrev: meta.as_ref().map(|m| m.home_abbrev.clone()),
                away_score: meta.as_ref().and_then(|m| m.away_score),
                home_score: meta.as_ref().and_then(|m| m.home_score),
                note,
            })?;
            match meta {
                Some(m) => println!(
                    "✓ Recorded: {} {} @ {} ({})",
                    m.date.as_deref().unwrap_or("?"),
                    m.away_abbrev,
                    m.home_abbrev,
                    score_pair(m.away_score, m.home_score)
                ),
                None => println!("✓ Recorded game {game_id} (boxscore unavailable; row saved)"),
            }
        }
        GamesSubcommand::Remove { game_id } => {
            if db.remove_attended_game(game_id)? {
                println!("✓ Removed game {game_id} from attended list.");
            } else {
                bail!("game {game_id} was not on your attended list");
            }
        }
        GamesSubcommand::List => {
            let rows = db.list_attended_games()?;
            if rows.is_empty() {
                println!("No attended games yet. Add one with `icelines games add <game_id>`.");
                return Ok(());
            }
            println!(
                "{:<10} {:<12} {:<24} {:<8} {}",
                "Game ID", "Date", "Matchup", "Score", "Note"
            );
            println!("{}", "─".repeat(78));
            for r in &rows {
                let date = r.game_date.as_deref().unwrap_or("—");
                let matchup = format!("{} @ {}", r.away_abbrev, r.home_abbrev);
                let score = score_pair(r.away_score, r.home_score);
                println!(
                    "{:<10} {:<12} {:<24} {:<8} {}",
                    r.game_id, date, matchup, score, r.note
                );
            }
            println!("\n{} game(s) attended.", rows.len());
        }
        GamesSubcommand::Export { out, json } => {
            let rows = db.list_attended_games()?;
            let body = if json {
                serde_json::to_string_pretty(&AttendedGamesExport {
                    version: 1,
                    games: rows
                        .iter()
                        .map(|r| ExportRow {
                            game_id: r.game_id,
                            date: r.game_date.clone(),
                            away: r.away_abbrev.clone(),
                            home: r.home_abbrev.clone(),
                            away_score: r.away_score,
                            home_score: r.home_score,
                            note: r.note.clone(),
                        })
                        .collect(),
                })
                .context("serializing attended games")?
            } else {
                // Default: CSV — opens directly in Excel.
                let headers = &[
                    "game_id",
                    "date",
                    "away",
                    "home",
                    "away_score",
                    "home_score",
                    "note",
                ];
                let csv_rows: Vec<Vec<String>> = rows
                    .iter()
                    .map(|r| {
                        vec![
                            r.game_id.to_string(),
                            r.game_date.clone().unwrap_or_default(),
                            r.away_abbrev.clone(),
                            r.home_abbrev.clone(),
                            r.away_score.map(|n| n.to_string()).unwrap_or_default(),
                            r.home_score.map(|n| n.to_string()).unwrap_or_default(),
                            r.note.clone(),
                        ]
                    })
                    .collect();
                crate::commands::output::Format::Csv.render(headers, &csv_rows)
            };
            if out == "-" {
                println!("{body}");
            } else {
                std::fs::write(&out, &body).with_context(|| format!("writing {out}"))?;
                println!("✓ Exported {} game(s) to {}", rows.len(), out);
            }
        }
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct AttendedGamesExport {
    version: u32,
    games: Vec<ExportRow>,
}

#[derive(Debug, serde::Serialize)]
struct ExportRow {
    game_id: u64,
    date: Option<String>,
    away: String,
    home: String,
    away_score: Option<u8>,
    home_score: Option<u8>,
    note: String,
}

#[derive(Debug, Clone)]
struct GameMeta {
    date: Option<String>,
    away_abbrev: String,
    home_abbrev: String,
    away_score: Option<u8>,
    home_score: Option<u8>,
}

/// Best-effort fetch of metadata for a game_id. Returns `None` on any
/// failure (offline, 404, malformed) — the caller still records the
/// game with the user-supplied note + ID.
async fn lookup_game_meta(game_id: u64) -> Option<GameMeta> {
    use icelines_fetch::nhl_api::NhlApiClient;
    let client = NhlApiClient::production();
    let bs = client.fetch_boxscore(game_id).await.ok()?;
    Some(GameMeta {
        // The boxscore endpoint exposes `gameDate` at the top — extract
        // via a separate call if the parser doesn't already publish it.
        // For now leave date as None and let users see the matchup.
        date: None,
        away_abbrev: bs.away_abbrev,
        home_abbrev: bs.home_abbrev,
        away_score: Some(bs.away_score),
        home_score: Some(bs.home_score),
    })
}

fn score_pair(away: Option<u8>, home: Option<u8>) -> String {
    match (away, home) {
        (Some(a), Some(h)) => format!("{a}-{h}"),
        _ => "—".to_owned(),
    }
}

// ── Shared helpers (Hart.5b2: PlayerView-based) ────────────────────────────────

fn find_view<'a, 'v>(
    views: &'a [PlayerView<'v>],
    name: &str,
) -> anyhow::Result<&'a PlayerView<'v>> {
    let norm = normalize_name(name);
    views
        .iter()
        .find(|v| v.name_normalized().contains(&norm))
        .with_context(|| format!("player '{name}' not found in snapshot — try a partial name"))
}

fn pace_strings_view(v: &PlayerView<'_>) -> (String, String) {
    match v.pace_score() {
        Some(s) => (
            format!("{:.2}", s.pace_82 / 82.0),
            format!("{:.0}", s.pace_82),
        ),
        None => ("—".to_owned(), "—".to_owned()),
    }
}

fn age_from_view(v: &PlayerView<'_>) -> String {
    v.identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string())
        .unwrap_or_else(|| "—".to_owned())
}

fn draft_str_view(v: &PlayerView<'_>) -> String {
    let bio = &v.identity.bio;
    match (bio.draft_year, bio.draft_round, bio.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} R{r} #{o}"),
        (Some(y), _, _) => format!("{y}"),
        _ => "Undrafted".to_owned(),
    }
}
