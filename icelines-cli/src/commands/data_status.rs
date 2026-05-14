//! `icelines data status` command.
//!
//! Reads the on-disk data manifest and projects it through `DataStatusView`
//! before printing a compact terminal table.

use anyhow::{Context, Result};
use icelines_core::{
    freshness::SystemClock, DataStatusEntryInput, DataStatusView, ViewContext, ViewWindow,
    CURRENT_SEASON,
};
use icelines_fetch::datastore::DataStore;
use icelines_fetch::manifest::{DataKey, DataKind};

pub async fn run(shard: Option<String>, stale_only: bool) -> Result<()> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("cannot determine home directory"))?;
    let data_root = home.join(".icelines").join("data");
    let store = DataStore::open(&data_root).context("open DataStore")?;

    let kind_filter = shard.as_deref().map(parse_kind).transpose()?;
    let rows = collect_rows(&store, kind_filter, stale_only);
    let view = DataStatusView::from_entries(
        ViewContext::new(ViewWindow::new(
            icelines_core::model::Season(CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )),
        data_root.display().to_string(),
        shard,
        stale_only,
        rows,
    );

    if view.rows.is_empty() {
        if let Some(k) = kind_filter {
            println!(
                "No manifest entries for {k:?}{}.",
                if stale_only { " (stale-only)" } else { "" }
            );
        } else {
            println!("Manifest is empty.");
            println!("Run `icelines setup --accept-defaults` then `icelines fetch sync`");
            println!("to populate.");
        }
        return Ok(());
    }

    print_table(&view);
    Ok(())
}

fn collect_rows(
    store: &DataStore,
    kind_filter: Option<DataKind>,
    stale_only: bool,
) -> Vec<DataStatusEntryInput> {
    let clock = SystemClock;
    let kinds: Vec<DataKind> = match kind_filter {
        Some(k) => vec![k],
        None => DataKind::all().to_vec(),
    };
    let mut rows: Vec<DataStatusEntryInput> = Vec::new();
    for k in kinds {
        for entry in store.manifest().list(k) {
            if stale_only && !entry.freshness.is_stale(&clock) {
                continue;
            }
            rows.push(DataStatusEntryInput {
                source: entry.freshness.source,
                kind: format!("{k:?}"),
                key: short_key(&entry.key),
                freshness: entry.freshness,
            });
        }
    }
    rows
}

fn parse_kind(s: &str) -> Result<DataKind> {
    Ok(match s.to_ascii_lowercase().as_str() {
        "bios" => DataKind::Bios,
        "stats" => DataKind::Stats,
        "goalie_stats" | "goalies" => DataKind::GoalieStats,
        "transactions" => DataKind::Transactions,
        "boxscore" | "boxscores" => DataKind::Boxscore,
        "play_by_play" | "play-by-play" | "pbp" => DataKind::PlayByPlay,
        "career_history" | "career" => DataKind::CareerHistory,
        "schedule" => DataKind::Schedule,
        "score" | "scores" => DataKind::Score,
        "playoff_bracket" | "playoffs" => DataKind::PlayoffBracket,
        other => anyhow::bail!(
            "unknown shard '{other}' - try one of: bios, stats, goalie_stats, transactions, boxscore, play_by_play, career_history, schedule, score, playoff_bracket"
        ),
    })
}

fn print_table(view: &DataStatusView) {
    println!("DATA STATUS - {}", view.root);
    println!("{}", "-".repeat(76));
    println!("{:<14} {:<16} {:<24} Freshness", "Source", "Kind", "Key");
    println!("{}", "-".repeat(76));
    for row in &view.rows {
        println!(
            "{:<14} {:<16} {:<24} {}",
            row.source, row.kind, row.key, row.freshness,
        );
    }
    println!("{}", "-".repeat(76));
    println!("{} entry(ies).", view.total);
}

fn short_key(key: &DataKey) -> String {
    match key {
        DataKey::Season(s) => s.as_str(),
        DataKey::SeasonType(s, t) => format!("{}/{}", s.as_str(), t.label()),
        DataKey::Game(g) => format!("game:{}", g.0),
        DataKey::Date(d) => d.clone(),
        DataKey::Player(p) => format!("player:{}", p.0),
        DataKey::Global => "<global>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_data_status_parse_kind_known_values() {
        assert!(matches!(parse_kind("bios").unwrap(), DataKind::Bios));
        assert!(matches!(parse_kind("BIOS").unwrap(), DataKind::Bios));
        assert!(matches!(
            parse_kind("goalies").unwrap(),
            DataKind::GoalieStats
        ));
        assert!(matches!(
            parse_kind("boxscore").unwrap(),
            DataKind::Boxscore
        ));
        assert!(matches!(parse_kind("scores").unwrap(), DataKind::Score));
    }

    #[test]
    fn l0_data_status_parse_kind_unknown_lists_options() {
        let err = parse_kind("garbage").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown shard"));
        assert!(msg.contains("bios"));
        assert!(msg.contains("stats"));
    }

    #[test]
    fn l0_data_status_freshness_label_buckets_by_unit() {
        use icelines_core::freshness::{FetchSource, Freshness, Ttl};
        use icelines_core::view_model::data_status::freshness_label;
        use std::time::Duration;

        let mk = |ttl| Freshness {
            fetched_at: chrono::Utc::now(),
            source: FetchSource::Live,
            ttl,
        };

        assert_eq!(freshness_label(&mk(Ttl::Static)), "static");
        assert_eq!(
            freshness_label(&mk(Ttl::After(Duration::from_secs(60)))),
            "ttl 1m"
        );
        assert_eq!(
            freshness_label(&mk(Ttl::After(Duration::from_secs(3600)))),
            "ttl 1h"
        );
        assert_eq!(
            freshness_label(&mk(Ttl::After(Duration::from_secs(86400)))),
            "ttl 1d"
        );
        assert_eq!(
            freshness_label(&mk(Ttl::After(Duration::from_secs(7 * 86400)))),
            "ttl 7d"
        );
    }

    #[test]
    fn l0_data_status_short_key_per_variant() {
        use icelines_core::identity::{GameId, PlayerId};
        use icelines_core::model::Season;
        use icelines_core::season_stats::SeasonType;

        assert_eq!(short_key(&DataKey::Season(Season(20252026))), "20252026");
        assert_eq!(
            short_key(&DataKey::SeasonType(Season(20252026), SeasonType::Playoff)),
            "20252026/playoff"
        );
        assert_eq!(
            short_key(&DataKey::Game(GameId(2025020001))),
            "game:2025020001"
        );
        assert_eq!(
            short_key(&DataKey::Player(PlayerId(8478402))),
            "player:8478402"
        );
        assert_eq!(short_key(&DataKey::Date("2026-01-15".into())), "2026-01-15");
        assert_eq!(short_key(&DataKey::Global), "<global>");
    }
}
