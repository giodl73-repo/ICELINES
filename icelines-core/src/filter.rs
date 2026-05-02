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
    /// Maximum age (inclusive), calculated from birth_date year vs 2026
    pub age_max: Option<u8>,
    /// Minimum age (inclusive), calculated from birth_date year vs 2026
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
        views
            .into_iter()
            .filter(|v| self.matches_view(v))
            .collect()
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
                .map(|birth_year| 2026u32.saturating_sub(birth_year) as u8);
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
            let toi = v.stats.totals.toi_per_game_sec.map(|t| t as f32).unwrap_or(0.0);
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

        true
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
            repo.upsert_identity(crate::fixtures::identity(pid).build()).unwrap();
            repo.upsert_stats(crate::fixtures::stats(pid, 20242025, team).build())
                .unwrap();
        }

        let f = PlayerFilter {
            teams: Some(vec!["EDM".to_string()]),
            ..PlayerFilter::new()
        };
        let result = f.apply_views(
            repo.skaters(crate::model::Season(20242025), crate::season_stats::SeasonType::Regular),
        );
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
            repo.upsert_identity(crate::fixtures::identity(pid).build()).unwrap();
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
}
