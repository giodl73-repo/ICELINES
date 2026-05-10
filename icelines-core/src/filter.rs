use crate::model::{Position, Region};

/// Composable filter for Player slices.
///
/// All active fields are combined with AND logic.
/// Fields set to None are ignored (match everything).
#[derive(Debug, Clone, Default)]
pub struct PlayerFilter {
    /// Filter to players on these team abbreviations (e.g. ["SEA", "NYR"])
    pub teams: Option<Vec<String>>,
    /// Filter to players with any of these positions
    pub positions: Option<Vec<Position>>,
    /// Maximum age (inclusive), calculated from birth_date year vs the active
    /// season's end year.
    pub age_max: Option<u8>,
    /// Minimum age (inclusive), calculated from birth_date year vs the active
    /// season's end year.
    pub age_min: Option<u8>,
    /// Filter to players with these ISO alpha-3 nationality codes
    pub nationalities: Option<Vec<String>>,
    /// Filter to players whose birth_country falls in any of these regions
    pub regions: Option<Vec<Region>>,
    /// Filter to players drafted in any of these years
    pub draft_years: Option<Vec<u16>>,
    /// Filter to players drafted in any of these rounds
    pub draft_rounds: Option<Vec<u8>>,
    /// Maximum overall draft pick number (inclusive)
    pub draft_pick_max: Option<u16>,
    /// If Some(true), include only undrafted players; if Some(false), only drafted
    pub undrafted: Option<bool>,
    /// If Some(true), include only rookies (rookie_season is set)
    pub rookie_only: Option<bool>,
    /// Minimum pace_82 (points per 82 games), inclusive
    pub ppg_min: Option<f64>,
    /// Maximum pace_82, inclusive
    pub ppg_max: Option<f64>,
    /// Minimum games played this season, inclusive
    pub gp_min: Option<u32>,
    /// Handedness filter: "L" or "R"
    pub handedness: Option<String>,
    /// Minimum average TOI per game in seconds
    pub toi_min_sec: Option<f32>,
    /// Minimum +/- (inclusive)
    pub plus_minus_min: Option<i32>,
    /// Minimum shots per game (pace-normalized, inclusive)
    pub shots_pg_min: Option<f32>,
    /// Birth province/state code filter (e.g. "ON", "AB", "QC")
    pub birth_provinces: Option<Vec<String>>,
    /// Phase Lindsay L.2.4 — generic stat filters. Each entry is a
    /// `(StatId, FilterOp, value)` triple constructed via `parse_filter`
    /// or `StatFilter::new` (the finite-value gate). Multiple filters
    /// on the same StatId compose per `normalize_stat_filters`:
    ///   - Min+Min → tightest (max) lower bound
    ///   - Max+Max → tightest (min) upper bound
    ///   - Min+Max → closed range
    ///   - Equals+Equals on same stat → rejected at parse time as `MultipleOps`
    pub stat_filters: Vec<crate::stats_catalog::StatFilter>,
    /// Filter.OR — boolean filter expressions (AND / OR / NOT / parens).
    /// Multiple entries are ANDed at the top level. The CLI populates
    /// this when `parse_filter_expr` returns a compound; bare atoms
    /// still flow through `stat_filters` to keep normalization
    /// (Min+Min → tightest, etc.) working for the simple case.
    pub expr_filters: Vec<crate::stats_catalog::FilterExpr>,
}

impl PlayerFilter {
    /// Create a new filter with all fields set to None (matches everything).
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply the filter to a `PlayerView` iterator, returning matching views
    /// in iteration order. AND logic across all active filter fields.
    pub fn apply_views<'a, I>(&'a self, views: I) -> Vec<crate::stats_repository::PlayerView<'a>>
    where
        I: IntoIterator<Item = crate::stats_repository::PlayerView<'a>>,
    {
        views.into_iter().filter(|v| self.matches_view(v)).collect()
    }

    /// Field-by-field match against a single `PlayerView`. Pure boolean
    /// over the active filter axes (team / position / age / nationality
    /// / region / undrafted / draft year+round / ppg+goals+gp mins /
    /// birth_province / TOI / plus_minus / shots-pg).
    pub fn matches_view(&self, v: &crate::stats_repository::PlayerView<'_>) -> bool {
        // Team filter
        if let Some(ref teams) = self.teams {
            if !teams
                .iter()
                .any(|t| t.eq_ignore_ascii_case(v.team_display()))
            {
                return false;
            }
        }

        // Position filter (check primary position)
        if let Some(ref positions) = self.positions {
            if !positions.contains(&v.position()) {
                return false;
            }
        }

        // Age filter — birth_date lives on identity.bio.
        if self.age_min.is_some() || self.age_max.is_some() {
            let age_opt = v
                .identity
                .bio
                .birth_date
                .as_deref()
                .and_then(|bd| bd.split('-').next())
                .and_then(|yr| yr.parse::<u32>().ok())
                .map(|birth_year| {
                    v.stats.season.end_year().saturating_sub(birth_year as u16) as u8
                });
            match age_opt {
                None => return false,
                Some(age) => {
                    if let Some(min) = self.age_min {
                        if age < min {
                            return false;
                        }
                    }
                    if let Some(max) = self.age_max {
                        if age > max {
                            return false;
                        }
                    }
                }
            }
        }

        // Nationality
        if let Some(ref nats) = self.nationalities {
            let m = v
                .identity
                .bio
                .nationality_code
                .as_deref()
                .map(|nc| nats.iter().any(|n| n.eq_ignore_ascii_case(nc)))
                .unwrap_or(false);
            if !m {
                return false;
            }
        }

        // Region
        if let Some(ref regions) = self.regions {
            let m = v
                .identity
                .bio
                .birth_country
                .as_deref()
                .map(|bc| regions.contains(&Region::from_country(bc)))
                .unwrap_or(false);
            if !m {
                return false;
            }
        }

        // Undrafted
        if let Some(want_undrafted) = self.undrafted {
            let is_undrafted = v.identity.bio.draft_year.is_none();
            if want_undrafted != is_undrafted {
                return false;
            }
        }

        // Draft year
        if let Some(ref years) = self.draft_years {
            let m = v
                .identity
                .bio
                .draft_year
                .map(|dy| years.contains(&dy))
                .unwrap_or(false);
            if !m {
                return false;
            }
        }

        // Draft round
        if let Some(ref rounds) = self.draft_rounds {
            let m = v
                .identity
                .bio
                .draft_round
                .map(|dr| rounds.contains(&dr))
                .unwrap_or(false);
            if !m {
                return false;
            }
        }

        // Draft pick max
        if let Some(max_pick) = self.draft_pick_max {
            let m = v
                .identity
                .bio
                .draft_overall
                .map(|pick| pick <= max_pick)
                .unwrap_or(false);
            if !m {
                return false;
            }
        }

        // Rookie — legacy uses CURRENT_SEASON as a u32; new bio
        // rookie_season is String "YYYYZZZZ".
        if let Some(true) = self.rookie_only {
            let want = crate::CURRENT_SEASON_STR;
            if v.identity.bio.rookie_season.as_deref() != Some(want) {
                return false;
            }
        }

        // PPG min/max — pace_82 / 82
        if let Some(ppg_min) = self.ppg_min {
            let ppg = v.pace_82().map(|p| p / 82.0).unwrap_or(0.0);
            if ppg < ppg_min {
                return false;
            }
        }
        if let Some(ppg_max) = self.ppg_max {
            let ppg = v.pace_82().map(|p| p / 82.0).unwrap_or(0.0);
            if ppg > ppg_max {
                return false;
            }
        }

        // GP min — view.gp() is u32 (always populated for resident rows).
        if let Some(gp_min) = self.gp_min {
            if v.gp() < gp_min {
                return false;
            }
        }

        // Handedness
        if let Some(ref hand) = self.handedness {
            let m = v
                .identity
                .bio
                .shoots_catches
                .as_deref()
                .map(|sc| sc.eq_ignore_ascii_case(hand))
                .unwrap_or(false);
            if !m {
                return false;
            }
        }

        // TOI minimum (seconds per game). Legacy field is f32; new is u32.
        if let Some(toi_min) = self.toi_min_sec {
            let toi = v
                .stats
                .totals
                .toi_per_game_sec
                .map(|t| t as f32)
                .unwrap_or(0.0);
            if toi < toi_min {
                return false;
            }
        }

        // Plus/minus
        if let Some(pm_min) = self.plus_minus_min {
            if v.plus_minus() < pm_min {
                return false;
            }
        }

        // Shots per game
        if let Some(spg_min) = self.shots_pg_min {
            let gp = v.gp();
            let spg = if gp > 0 {
                v.shots() as f32 / gp as f32
            } else {
                0.0
            };
            if spg < spg_min {
                return false;
            }
        }

        // Birth province/state
        if let Some(ref provinces) = self.birth_provinces {
            let m = v
                .identity
                .bio
                .birth_state_province
                .as_deref()
                .map(|prov| provinces.iter().any(|pr| pr.eq_ignore_ascii_case(prov)))
                .unwrap_or(false);
            if !m {
                return false;
            }
        }

        // ── Phase Lindsay L.2.4 — generic stat filters ──────────────────
        if !self.matches_stat_filters(v) {
            return false;
        }

        // ── Filter.OR — boolean filter expressions ─────────────────────
        // Each top-level expression is ANDed against the others.
        for expr in &self.expr_filters {
            if !expr.matches(v) {
                return false;
            }
        }

        true
    }

    /// Match every `stat_filters` entry against the view (DI-08).
    /// Filters whose `StatId::applies_to(position, is_goalie)` is false
    /// are silently dropped at row-level — same CLI grammar can target
    /// `--filter "save-pct>=.91"` against a mixed pool without errors
    /// for skater rows. Missing data (read returns None) → row fails
    /// the filter (intentional per spec — `hits-min 100` shouldn't
    /// surface pre-2005 rows where hits is unknown).
    pub fn matches_stat_filters(&self, v: &crate::stats_repository::PlayerView<'_>) -> bool {
        use crate::stats_catalog::{FilterOp, StatUnit};
        for f in &self.stat_filters {
            // DI-08 — skip non-applicable filters silently.
            if !f.stat.applies_to(v.position(), v.is_goalie()) {
                continue;
            }
            let actual = match f.stat.read(v) {
                Some(x) => x,
                None => return false, // missing data ≠ matches
            };
            // Type-aware tolerance for Equals (L2-B1).
            let ok = match f.op {
                FilterOp::Min => actual >= f.value,
                FilterOp::Max => actual <= f.value,
                FilterOp::Equals => match f.stat.unit() {
                    StatUnit::Count | StatUnit::Seconds => (actual - f.value).abs() < 0.5,
                    StatUnit::Per60 => (actual - f.value).abs() < 1e-3,
                    StatUnit::Pct | StatUnit::Rate | StatUnit::Inverted => {
                        (actual - f.value).abs() < 1e-6
                    }
                },
            };
            if !ok {
                return false;
            }
        }
        true
    }

    /// Same-StatId multi-filter normalization (EDGE-R2 / II-06):
    ///
    ///   - `Min+Min` on the same StatId → keep the tightest (max value).
    ///   - `Max+Max` on the same StatId → keep the tightest (min value).
    ///   - `Min+Max` mix → keep both (composes to a closed range).
    ///   - `Equals+Equals` on the same StatId → reject at parse time
    ///     (returns `Err(FilterParseError::MultipleOps)`), so this
    ///     method never sees that case. If two such filters sneak in
    ///     via direct mutation, the SECOND wins (last-write).
    ///
    /// Idempotent: calling twice has no effect after the first call.
    /// Stable order: surviving filters appear in the order their
    /// (stat, op-kind) pair was first seen.
    pub fn normalize_stat_filters(&mut self) {
        use crate::stats_catalog::FilterOp;

        // Index by (StatId, op-kind) to find dupes, preserving first-seen
        // order via `seen_order`.
        let mut seen_order: Vec<(crate::stats_catalog::StatId, FilterOp)> = Vec::new();
        let mut tightest: std::collections::HashMap<(crate::stats_catalog::StatId, FilterOp), f64> =
            std::collections::HashMap::new();

        for f in &self.stat_filters {
            let key = (f.stat, f.op);
            match tightest.get(&key) {
                None => {
                    tightest.insert(key, f.value);
                    seen_order.push(key);
                }
                Some(&existing) => {
                    let new = match f.op {
                        // Min: tightest = max of lower bounds.
                        FilterOp::Min => existing.max(f.value),
                        // Max: tightest = min of upper bounds.
                        FilterOp::Max => existing.min(f.value),
                        // Equals: last-write semantic (parser usually rejects).
                        FilterOp::Equals => f.value,
                    };
                    tightest.insert(key, new);
                }
            }
        }

        // Rebuild stat_filters in seen-order with tightest values.
        // `StatFilter::new` re-validates the (already-finite) value;
        // unwrap is safe because the original `f.value` was finite by
        // construction (StatFilter::new gate).
        self.stat_filters = seen_order
            .into_iter()
            .map(|(stat, op)| {
                let value = tightest[&(stat, op)];
                crate::stats_catalog::StatFilter::new(stat, op, value)
                    .expect("normalized values inherited finite-gate guarantee")
            })
            .collect();
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats_repository::StatsRepository;
    use std::collections::BTreeSet;

    /// apply_views collects matching views in iteration order and
    /// excludes non-matches.
    #[test]
    fn l0_apply_views_filters_correctly() {
        let mut repo = StatsRepository::new();
        for (pid, team) in [(1u32, "EDM"), (2, "TOR"), (3, "EDM")] {
            repo.upsert_identity(crate::fixtures::identity(pid).build())
                .unwrap();
            repo.upsert_stats(crate::fixtures::stats(pid, 20242025, team).build())
                .unwrap();
        }

        let f = PlayerFilter {
            teams: Some(vec!["EDM".to_string()]),
            ..PlayerFilter::new()
        };
        let result = f.apply_views(repo.skaters(
            crate::model::Season(20242025),
            crate::season_stats::SeasonType::Regular,
        ));
        assert_eq!(result.len(), 2, "two EDM players match");
        assert!(result.iter().all(|v| v.team_display() == "EDM"));
    }

    /// apply_views must produce a deterministic subset of the
    /// identity-filter (no axes) result for any filter shape.
    #[test]
    fn l0_apply_views_named_filters_subset_of_identity() {
        let mut repo = StatsRepository::new();
        for (pid, team) in [
            (8478402u32, "EDM"),
            (8479318, "TOR"),
            (8480039, "EDM"),
            (8478427, "FLA"),
        ] {
            repo.upsert_identity(crate::fixtures::identity(pid).build())
                .unwrap();
            repo.upsert_stats(crate::fixtures::stats(pid, 20242025, team).build())
                .unwrap();
        }

        let s = crate::model::Season(20242025);
        let t = crate::season_stats::SeasonType::Regular;

        let cases: Vec<PlayerFilter> = vec![
            PlayerFilter::new(),
            PlayerFilter {
                teams: Some(vec!["EDM".into()]),
                ..PlayerFilter::new()
            },
            PlayerFilter {
                positions: Some(vec![Position::Center]),
                ..PlayerFilter::new()
            },
            PlayerFilter {
                gp_min: Some(50),
                ..PlayerFilter::new()
            },
            PlayerFilter {
                ppg_min: Some(1.0),
                ..PlayerFilter::new()
            },
            PlayerFilter {
                draft_years: Some(vec![2015]),
                ..PlayerFilter::new()
            },
            PlayerFilter {
                birth_provinces: Some(vec!["ON".into()]),
                ..PlayerFilter::new()
            },
        ];

        let identity_filter_views: BTreeSet<u32> = PlayerFilter::new()
            .apply_views(repo.skaters(s, t))
            .iter()
            .map(|v| v.identity.id.0)
            .collect();
        assert_eq!(identity_filter_views.len(), 4);

        for filter in cases {
            let view_ids: BTreeSet<u32> = filter
                .apply_views(repo.skaters(s, t))
                .iter()
                .map(|v| v.identity.id.0)
                .collect();
            assert!(
                view_ids.is_subset(&identity_filter_views),
                "filter {filter:?} produced an id outside the source set"
            );
        }
    }

    // ─── L.2.4 — normalize_stat_filters + matches_stat_filters ────────────

    use crate::fixtures::stat_catalog_variants;
    use crate::stats_catalog::{FilterOp, StatFilter, StatId};
    use crate::stats_repository::PlayerView;

    /// Min+Min on same StatId → tightest (max value) lower bound.
    #[test]
    fn l0_lindsay_normalize_min_min_keeps_tightest() {
        let mut pf = PlayerFilter::new();
        pf.stat_filters = vec![
            StatFilter::new(StatId::Hits, FilterOp::Min, 50.0).unwrap(),
            StatFilter::new(StatId::Hits, FilterOp::Min, 100.0).unwrap(),
        ];
        pf.normalize_stat_filters();
        assert_eq!(pf.stat_filters.len(), 1, "Min+Min should collapse to one");
        assert_eq!(pf.stat_filters[0].value, 100.0, "tightest = max");
    }

    /// Max+Max on same StatId → tightest (min value) upper bound.
    #[test]
    fn l0_lindsay_normalize_max_max_keeps_tightest() {
        let mut pf = PlayerFilter::new();
        pf.stat_filters = vec![
            StatFilter::new(StatId::Hits, FilterOp::Max, 200.0).unwrap(),
            StatFilter::new(StatId::Hits, FilterOp::Max, 150.0).unwrap(),
        ];
        pf.normalize_stat_filters();
        assert_eq!(pf.stat_filters.len(), 1);
        assert_eq!(pf.stat_filters[0].value, 150.0, "tightest = min");
    }

    /// Min+Max on same StatId → keep both (closed range).
    #[test]
    fn l0_lindsay_normalize_min_max_composes_to_range() {
        let mut pf = PlayerFilter::new();
        pf.stat_filters = vec![
            StatFilter::new(StatId::Hits, FilterOp::Min, 50.0).unwrap(),
            StatFilter::new(StatId::Hits, FilterOp::Max, 200.0).unwrap(),
        ];
        pf.normalize_stat_filters();
        assert_eq!(pf.stat_filters.len(), 2, "Min+Max → both kept");
        assert!(pf
            .stat_filters
            .iter()
            .any(|f| f.op == FilterOp::Min && f.value == 50.0));
        assert!(pf
            .stat_filters
            .iter()
            .any(|f| f.op == FilterOp::Max && f.value == 200.0));
    }

    /// Cross-stat filters preserved (Hits-min + Goals-max coexist).
    #[test]
    fn l0_lindsay_normalize_cross_stat_independent() {
        let mut pf = PlayerFilter::new();
        pf.stat_filters = vec![
            StatFilter::new(StatId::Hits, FilterOp::Min, 50.0).unwrap(),
            StatFilter::new(StatId::Goals, FilterOp::Max, 30.0).unwrap(),
        ];
        pf.normalize_stat_filters();
        assert_eq!(pf.stat_filters.len(), 2);
    }

    /// Idempotent: calling twice has no effect after the first call.
    #[test]
    fn l0_lindsay_normalize_idempotent() {
        let mut pf = PlayerFilter::new();
        pf.stat_filters = vec![
            StatFilter::new(StatId::Hits, FilterOp::Min, 50.0).unwrap(),
            StatFilter::new(StatId::Hits, FilterOp::Min, 100.0).unwrap(),
            StatFilter::new(StatId::Hits, FilterOp::Max, 200.0).unwrap(),
        ];
        pf.normalize_stat_filters();
        let first = pf.stat_filters.clone();
        pf.normalize_stat_filters();
        assert_eq!(pf.stat_filters, first, "idempotent");
    }

    /// `matches_stat_filters` — DI-08 silently drops non-applicable
    /// filters. SavePct on a skater view is non-applicable; the row
    /// passes (the filter is skipped, not failed).
    #[test]
    fn l0_lindsay_matches_skips_non_applicable_filters() {
        let (id, stats) = stat_catalog_variants::skater_modern();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };
        let mut pf = PlayerFilter::new();
        // SavePct on a skater is non-applicable → silently dropped (DI-08).
        pf.stat_filters = vec![StatFilter::new(StatId::SavePct, FilterOp::Min, 0.91).unwrap()];
        assert!(
            pf.matches_stat_filters(&view),
            "non-applicable filter dropped silently"
        );
    }

    /// `matches_stat_filters` — missing data fails the filter (per spec
    /// "missing-data treats as `false`" semantic).
    #[test]
    fn l0_lindsay_matches_missing_data_fails_filter() {
        let (id, stats) = stat_catalog_variants::skater_pre_2005();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };
        let mut pf = PlayerFilter::new();
        // Hits is era-gated for pre-2005 → read returns None → fail.
        pf.stat_filters = vec![StatFilter::new(StatId::Hits, FilterOp::Min, 50.0).unwrap()];
        assert!(
            !pf.matches_stat_filters(&view),
            "pre-2005 hits=None should fail the filter"
        );
    }

    /// `matches_stat_filters` — happy path: skater_modern has 30 hits
    /// → passes "hits>=20", fails "hits>=50".
    #[test]
    fn l0_lindsay_matches_happy_path() {
        let (id, stats) = stat_catalog_variants::skater_modern();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };

        let mut pf = PlayerFilter::new();
        pf.stat_filters = vec![StatFilter::new(StatId::Hits, FilterOp::Min, 20.0).unwrap()];
        assert!(pf.matches_stat_filters(&view), "30 hits >= 20 must pass");

        pf.stat_filters = vec![StatFilter::new(StatId::Hits, FilterOp::Min, 50.0).unwrap()];
        assert!(!pf.matches_stat_filters(&view), "30 hits < 50 must fail");
    }

    /// Equals tolerance is unit-aware (L2-B1). Count uses < 0.5 (integer
    /// compare); Pct uses 1e-6.
    #[test]
    fn l0_lindsay_matches_equals_unit_aware_tolerance() {
        let (id, stats) = stat_catalog_variants::skater_modern();
        let view = PlayerView {
            identity: &id,
            stats: &stats,
            contract: None,
        };

        // Count (Goals=50): 50.4 should match (< 0.5 tolerance).
        let mut pf = PlayerFilter::new();
        pf.stat_filters = vec![StatFilter::new(StatId::Goals, FilterOp::Equals, 50.4).unwrap()];
        assert!(
            pf.matches_stat_filters(&view),
            "Goals=50 within 0.5 of 50.4"
        );

        // Goals=50, asking == 51.0 — outside Count tolerance.
        pf.stat_filters = vec![StatFilter::new(StatId::Goals, FilterOp::Equals, 51.0).unwrap()];
        assert!(!pf.matches_stat_filters(&view), "Goals=50 != 51");
    }
}
