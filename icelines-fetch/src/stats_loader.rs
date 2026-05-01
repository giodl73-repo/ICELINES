//! Phase Hart.3 — `StatsRepository` loader.
//!
//! Populates a fresh `StatsRepository` from the bundled+snapshot data
//! tiers. Surfaces partial-fetch conditions via `LoadOutcome.missing` so
//! callers can render specific banners ("MoneyPuck unavailable") rather
//! than silently shipping `advanced=None` for every player.
//!
//! Parallel-run with the legacy `PlayerRepository::load_all()` path:
//! both can run from the same snapshot store. Hart.4 sub-phases migrate
//! consumers commit-by-commit; Hart.5 deletes the legacy path.

use std::collections::HashMap;

use icelines_core::contract::PlayerContract;
use icelines_core::identity::{PlayerBio, PlayerId, PlayerIdentity};
use icelines_core::model::{Position, Season, TeamAbbr};
use icelines_core::name::normalize_name;
use icelines_core::scoring::compute_pace_score;
use icelines_core::season_stats::{
    AdvancedStats, GoalieSeasonStats, RealtimeStats, SeasonStatsBuilder, SeasonType, StatTotals,
    TeamStint,
};
use icelines_core::stats_repository::{RepoError, StatsRepository};
use thiserror::Error;

use crate::bundled;
use crate::moneypuck::MoneyPuckStats;
use crate::schema::{
    GoalieStats, PlayerContract as LegacyContract, SkaterBio, SkaterRealtime, SkaterStats,
};
use crate::snapshot::{SnapshotMetaFlags, SnapshotStore, SnapshotTier};

/// Bundled-JSON file format version this binary understands. Bumps on
/// non-`Option` field additions to existing types in the bundles.
pub const MAX_KNOWN_BUNDLE_SCHEMA: u32 = 1;

/// In-memory `StatsRepository` model version. Bumps on every breaking
/// change to the `icelines-core` model. Phase Hart starts at 1.
pub const MAX_KNOWN_REPO_VERSION: u32 = 1;

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum BundleError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Parse(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("season {season} not bundled in this build")]
    SeasonNotBundled { season: String },
    #[error("season {season} has no {season_type:?} bundle")]
    MissingBundle {
        season: String,
        season_type: SeasonType,
    },
    #[error("bundle schema version {found} unknown (this binary supports up to {max_known})")]
    BundleSchemaUnknown { found: u32, max_known: u32 },
    #[error("bundle repository version {found} unknown (this binary supports up to {max_known})")]
    RepoVersionUnknown { found: u32, max_known: u32 },
    #[error("repository error: {0}")]
    Repo(#[from] RepoError),
    /// I/O + parse failures wrapped under one variant per FORGE: keeps
    /// the public error surface small while `?` stays ergonomic.
    #[error("bundle read/parse failure: {source}")]
    Bundle {
        #[from]
        source: BundleError,
    },
}

// ── LoadOutcome / MissingSource ─────────────────────────────────────────────

/// Per-source partial-fetch signal. Each variant maps to a specific
/// user-facing banner in the CLI / TUI.
#[derive(Debug, Clone, PartialEq)]
pub enum MissingSource {
    Realtime {
        season: String,
        season_type: SeasonType,
        reason: String,
    },
    MoneyPuck {
        season: String,
        reason: String,
    },
    Contracts {
        reason: String,
    },
    GoalieStats {
        season: String,
        season_type: SeasonType,
        reason: String,
    },
}

/// Result of populating a `StatsRepository` from one (season, type)
/// load. `missing` is empty for a clean load; non-empty entries identify
/// specific tiers that didn't materialize. `missing_files` is a
/// finer-grained diagnostic — file names attempted but not found.
#[derive(Debug)]
pub struct LoadOutcome {
    pub repo: StatsRepository,
    pub missing: Vec<MissingSource>,
    pub missing_files: Vec<String>,
    pub fetched_at: String,
}

// ── Loader ──────────────────────────────────────────────────────────────────

/// Populate a fresh `StatsRepository` for the given (season, type).
///
/// Source order:
/// - bios + stats: snapshot first, bundled fallback (the existing
///   `bundled::load_*_with_fallback` chain).
/// - realtime / moneypuck / contracts: snapshot-only — flagged as
///   `MissingSource` if not present.
/// - goalie-stats: bundled-only for v0.13 — flagged if missing.
///
/// Hart.6 captures playoff bundled data; until then this returns
/// `LoadError::MissingBundle` for `season_type = SeasonType::Playoff`.
pub fn load_into_repo(
    season: Season,
    season_type: SeasonType,
    store: &SnapshotStore,
) -> Result<LoadOutcome, LoadError> {
    let season_str = season.as_str();

    // Hart.6 will populate bundled playoff data; until then refuse cleanly.
    if season_type == SeasonType::Playoff {
        return Err(LoadError::MissingBundle {
            season: season_str.clone(),
            season_type,
        });
    }

    // Schema-version gate. Missing _meta.json (cold-start) is fine —
    // SnapshotMetaFlags::default() yields version 0, which we treat as
    // "pre-Hart, no version stamp" and accept. Only positive values that
    // *exceed* what this binary knows are an error.
    let meta = SnapshotMetaFlags::load(store.root(), &season_str);
    if meta.bundle_schema_version > MAX_KNOWN_BUNDLE_SCHEMA {
        return Err(LoadError::BundleSchemaUnknown {
            found: meta.bundle_schema_version,
            max_known: MAX_KNOWN_BUNDLE_SCHEMA,
        });
    }
    if meta.repository_version > MAX_KNOWN_REPO_VERSION {
        return Err(LoadError::RepoVersionUnknown {
            found: meta.repository_version,
            max_known: MAX_KNOWN_REPO_VERSION,
        });
    }

    // ── Tier reads ──────────────────────────────────────────────────────────

    // Bios — fallback chain. Hard-fail if neither snapshot nor bundle has them
    // (loader contract: identities are required, stats and below can be empty).
    let bios = bundled::load_bios_with_fallback(&season_str, store).map_err(|_| {
        LoadError::SeasonNotBundled {
            season: season_str.clone(),
        }
    })?;
    if bios.is_empty() {
        return Err(LoadError::SeasonNotBundled {
            season: season_str.clone(),
        });
    }
    let stats = bundled::load_stats_with_fallback(&season_str, store).unwrap_or_default();
    let goalie_stats = bundled::get_goalie_stats(&season_str).unwrap_or_default();

    let mut missing: Vec<MissingSource> = Vec::new();
    let mut missing_files: Vec<String> = Vec::new();

    let realtime: Vec<SkaterRealtime> =
        match store.read_tier::<Vec<SkaterRealtime>>(&SnapshotTier::Realtime, "realtime.json") {
            Ok(rt) => rt,
            Err(_) => {
                missing.push(MissingSource::Realtime {
                    season: season_str.clone(),
                    season_type,
                    reason: "realtime.json not present in snapshot store".into(),
                });
                missing_files.push("snapshot:realtime.json".into());
                Vec::new()
            }
        };
    let moneypuck: Vec<MoneyPuckStats> =
        match store.read_tier::<Vec<MoneyPuckStats>>(&SnapshotTier::MoneyPuck, "moneypuck.json") {
            Ok(m) => m,
            Err(_) => {
                missing.push(MissingSource::MoneyPuck {
                    season: season_str.clone(),
                    reason: "moneypuck.json not present in snapshot store".into(),
                });
                missing_files.push("snapshot:moneypuck.json".into());
                Vec::new()
            }
        };
    let contracts: Vec<LegacyContract> =
        match store.read_tier::<Vec<LegacyContract>>(&SnapshotTier::Contracts, "contracts.json") {
            Ok(c) => c,
            Err(_) => {
                missing.push(MissingSource::Contracts {
                    reason: "contracts.json not present in snapshot store".into(),
                });
                missing_files.push("snapshot:contracts.json".into());
                Vec::new()
            }
        };
    if goalie_stats.is_empty() {
        missing.push(MissingSource::GoalieStats {
            season: season_str.clone(),
            season_type,
            reason: "goalie-stats.json not bundled for this season".into(),
        });
        missing_files.push("bundled:goalie-stats.json".into());
    }

    // ── Indexes ─────────────────────────────────────────────────────────────

    let stats_idx: HashMap<u32, &SkaterStats> = stats.iter().map(|s| (s.player_id, s)).collect();
    let realtime_idx: HashMap<u32, &SkaterRealtime> =
        realtime.iter().map(|r| (r.player_id, r)).collect();
    let moneypuck_idx: HashMap<u32, &MoneyPuckStats> =
        moneypuck.iter().map(|m| (m.player_id, m)).collect();
    let contracts_idx: HashMap<u32, &LegacyContract> =
        contracts.iter().map(|c| (c.player_id, c)).collect();

    // Dedup bios by player_id, last-occurrence-wins. Matches the OLD
    // `build_players_from_bios` invariant: traded players emit one bio
    // row per stint; we keep the most-recent (current-team) row.
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let bios_dedup: Vec<&SkaterBio> = bios
        .iter()
        .rev()
        .filter(|b| seen.insert(b.player_id))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // ── Populate repository ─────────────────────────────────────────────────

    let mut repo = StatsRepository::new();

    // 1. Skater identities + stats.
    for bio in &bios_dedup {
        let Some(position) = Position::from_api_code(&bio.position_code) else {
            continue;
        };
        // Skip pure goalie rows that crept into bios (none expected, but defensive).
        if matches!(position, Position::Goalie) {
            continue;
        }
        let pid = PlayerId(bio.player_id);
        let identity = build_identity(pid, bio);
        repo.upsert_identity(identity)?;

        let stats_row = stats_idx.get(&bio.player_id).copied();
        let realtime_row = realtime_idx.get(&bio.player_id).copied();
        let mp_row = moneypuck_idx.get(&bio.player_id).copied();
        let stats = build_skater_stats(
            pid,
            season,
            season_type,
            position,
            bio,
            stats_row,
            realtime_row,
            mp_row,
        );
        repo.upsert_stats(stats)?;

        if let Some(c) = contracts_idx.get(&bio.player_id) {
            repo.upsert_contract(pid, build_contract(c));
        }
    }

    // 2. Goalies (different source — goalie-stats.json carries name/team).
    for g in &goalie_stats {
        let pid = PlayerId(g.player_id);
        // A goalie's identity may already exist from contracts; insert
        // bare-bones identity if not.
        if repo.identity(pid).is_none() {
            repo.upsert_identity(build_goalie_identity(g))?;
        }
        let stats = build_goalie_season_stats(pid, season, season_type, g);
        repo.upsert_stats(stats)?;
        if let Some(c) = contracts_idx.get(&g.player_id) {
            repo.upsert_contract(pid, build_contract(c));
        }
    }

    Ok(LoadOutcome {
        repo,
        missing,
        missing_files,
        fetched_at: now_iso8601(),
    })
}

// ── Mappers ─────────────────────────────────────────────────────────────────

fn build_identity(pid: PlayerId, bio: &SkaterBio) -> PlayerIdentity {
    PlayerIdentity {
        id: pid,
        full_name: bio.skater_full_name.clone(),
        name_normalized: normalize_name(&bio.skater_full_name),
        headshot_canonical_url: Some(format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            pid.0
        )),
        bio: PlayerBio {
            birth_date: bio.birth_date.clone(),
            birth_country: bio.birth_country.clone(),
            nationality_code: bio.nationality_code.clone(),
            height_in_inches: bio.height,
            weight_lbs: bio.weight,
            draft_year: bio.draft_year.map(|v| v as u16),
            draft_round: bio.draft_round.map(|v| v as u8),
            draft_overall: bio.draft_overall.map(|v| v as u16),
            shoots_catches: bio.shoots_catches.clone(),
            rookie_season: bio.first_season_for_game_type.map(|s| s.to_string()),
        },
    }
}

fn build_goalie_identity(g: &GoalieStats) -> PlayerIdentity {
    PlayerIdentity {
        id: PlayerId(g.player_id),
        full_name: g.goalie_full_name.clone(),
        name_normalized: normalize_name(&g.goalie_full_name),
        headshot_canonical_url: Some(format!(
            "https://assets.nhle.com/mugs/nhl/default/{}.png",
            g.player_id
        )),
        bio: PlayerBio {
            shoots_catches: g.shoots_catches.clone(),
            ..Default::default()
        },
    }
}

fn build_skater_stats(
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
    position: Position,
    bio: &SkaterBio,
    stats: Option<&SkaterStats>,
    realtime: Option<&SkaterRealtime>,
    mp: Option<&MoneyPuckStats>,
) -> icelines_core::season_stats::SeasonStats {
    // Field-for-field parity with the OLD `make_player` path so the
    // parallel-run field-parity test holds.
    let goals = stats.map(|s| s.goals).unwrap_or(bio.goals);
    let assists = stats.map(|s| s.assists).unwrap_or(bio.assists);
    let gp = stats.map(|s| s.games_played).unwrap_or(bio.games_played);
    let pp_goals = stats.map(|s| s.pp_goals).unwrap_or(0);
    let pp_points = stats.map(|s| s.pp_points).unwrap_or(0);
    let gwg = stats.map(|s| s.game_winning_goals).unwrap_or(0);
    let plus_minus = stats.map(|s| s.plus_minus).unwrap_or(0);
    let shots = stats.map(|s| s.shots).unwrap_or(0);
    let shooting_pct = stats.and_then(|s| s.shooting_pctg);
    let toi_per_game_sec = stats.and_then(|s| s.time_on_ice_per_game).map(|v| v as u32);
    let faceoff_win_pct = stats.and_then(|s| s.faceoff_win_pct);

    let totals = StatTotals {
        gp,
        goals,
        assists,
        points: goals + assists,
        plus_minus,
        pim: realtime.map(|r| r.pim).unwrap_or(0),
        shots,
        shooting_pct,
        toi_per_game_sec,
        pp_goals,
        pp_points,
        gwg,
        faceoff_win_pct,
        pace_score: compute_pace_score(goals, assists, gp),
    };

    // Single TeamStint synthesized from current_team_abbrev — bundled
    // bios don't carry stint history. Retired/unsigned players land
    // under "RET" matching the OLD path.
    let team_str = bio.current_team_abbrev.as_deref().unwrap_or("RET");
    let stint = TeamStint {
        team: TeamAbbr(team_str.to_owned()),
        started: None,
        ended: None,
        gp,
        goals,
        assists,
        points: goals + assists,
        goalie: None,
    };

    let mut builder = SeasonStatsBuilder::new(pid, season, season_type, position)
        .with_totals(totals)
        .add_team_stint(stint);

    if let Some(rt) = realtime {
        builder = builder.with_realtime(RealtimeStats {
            hits: rt.hits,
            blocked_shots: rt.blocked_shots,
            takeaways: rt.takeaways,
            giveaways: rt.giveaways,
        });
    }
    if let Some(m) = mp {
        builder = builder.with_advanced(AdvancedStats {
            xg: Some(m.xg_all as f64),
            xg_per_60: Some(m.xg_per_60 as f64),
            cf_pct: Some(m.cf_pct_5v5 as f64),
            ff_pct: Some(m.ff_pct_5v5 as f64),
            xgf_pct: Some(m.xgf_pct_5v5 as f64),
        });
    }

    builder.build()
}

fn build_goalie_season_stats(
    pid: PlayerId,
    season: Season,
    season_type: SeasonType,
    g: &GoalieStats,
) -> icelines_core::season_stats::SeasonStats {
    // The legacy goalie row carries `team_abbrevs` as a comma-separated
    // string for traded goalies (e.g. "BOS,OTT"). For Hart.3 we synthesize
    // one TeamStint per token; per-stint goalie counts (W/L/GS split by
    // team) are NOT in the bundled data — only the season-aggregate
    // GoalieSeasonStats has them. Hart.6 captures real per-stint history.
    let teams: Vec<&str> = g
        .team_abbrevs
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    let n = teams.len().max(1) as u32;

    let stints: Vec<TeamStint> = if teams.is_empty() {
        vec![TeamStint {
            team: TeamAbbr("RET".into()),
            started: None,
            ended: None,
            gp: g.games_played,
            goals: g.goals,
            assists: g.assists,
            points: g.points,
            goalie: None,
        }]
    } else {
        // Roughly equal split — sum-equals invariant on (gp, goals,
        // assists, points). The remainder lands on the LAST stint so
        // current-home semantics stay correct.
        teams
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let is_last = i == teams.len() - 1;
                let take_n = |total: u32| -> u32 {
                    if is_last {
                        total - (total / n) * (n - 1)
                    } else {
                        total / n
                    }
                };
                TeamStint {
                    team: TeamAbbr((*t).to_owned()),
                    started: None,
                    ended: None,
                    gp: take_n(g.games_played),
                    goals: take_n(g.goals),
                    assists: take_n(g.assists),
                    points: take_n(g.points),
                    goalie: None,
                }
            })
            .collect()
    };

    let totals = StatTotals {
        gp: g.games_played,
        goals: g.goals,
        assists: g.assists,
        points: g.points,
        plus_minus: 0,
        pim: g.penalty_minutes,
        shots: 0,
        shooting_pct: None,
        toi_per_game_sec: if g.games_played > 0 {
            Some(g.time_on_ice / g.games_played)
        } else {
            None
        },
        pp_goals: 0,
        pp_points: 0,
        gwg: 0,
        faceoff_win_pct: None,
        pace_score: None,
    };

    let goalie = GoalieSeasonStats {
        games_started: g.games_started,
        wins: g.wins,
        losses: g.losses,
        ot_losses: g.ot_losses,
        ties: g.ties,
        shots_against: g.shots_against,
        goals_against: g.goals_against,
        saves: g.saves,
        save_pct: g.save_pct,
        goals_against_average: g.goals_against_average,
        shutouts: g.shutouts,
        time_on_ice_sec: g.time_on_ice,
    };

    SeasonStatsBuilder::new(pid, season, season_type, Position::Goalie)
        .with_totals(totals)
        .replace_team_stints(stints)
        .with_goalie(goalie)
        .build()
}

fn build_contract(c: &LegacyContract) -> PlayerContract {
    PlayerContract {
        expiry_year: c.expiry_year,
        expiry_type: c.expiry_type.clone(),
        salary: c.salary,
    }
}

fn now_iso8601() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_bundle_error_wraps_io_and_parse() {
        let io: BundleError = std::io::Error::new(std::io::ErrorKind::NotFound, "missing").into();
        match io {
            BundleError::Io(e) => assert_eq!(e.kind(), std::io::ErrorKind::NotFound),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn l0_load_error_repo_wraps_repo_error() {
        let inner = RepoError::StatsWithoutIdentity {
            id: PlayerId(1),
            season: Season(20232024),
            season_type: SeasonType::Regular,
        };
        let outer: LoadError = inner.into();
        match outer {
            LoadError::Repo(_) => {}
            other => panic!("expected Repo, got {other:?}"),
        }
    }

    #[test]
    fn l0_missing_source_partial_eq() {
        let a = MissingSource::Realtime {
            season: "20242025".into(),
            season_type: SeasonType::Regular,
            reason: "x".into(),
        };
        let b = MissingSource::Realtime {
            season: "20242025".into(),
            season_type: SeasonType::Regular,
            reason: "x".into(),
        };
        assert_eq!(a, b);
    }

    #[test]
    fn l0_playoff_returns_missing_bundle_for_now() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = SnapshotStore::new(dir.path());
        let err = load_into_repo(Season(20242025), SeasonType::Playoff, &store).unwrap_err();
        match err {
            LoadError::MissingBundle { season_type, .. } => {
                assert_eq!(season_type, SeasonType::Playoff);
            }
            other => panic!("expected MissingBundle, got {other:?}"),
        }
    }
}
