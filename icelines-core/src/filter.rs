use crate::model::{Player, Position, Region};

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

    /// Apply the filter to a slice of players, returning references to matching players.
    ///
    /// All active filter fields are combined with AND logic.
    pub fn apply<'a>(&self, players: &'a [Player]) -> Vec<&'a Player> {
        players.iter().filter(|p| self.matches(p)).collect()
    }

    fn matches(&self, p: &Player) -> bool {
        // Team filter
        if let Some(ref teams) = self.teams {
            if !teams
                .iter()
                .any(|t| t.eq_ignore_ascii_case(p.team.as_str()))
            {
                return false;
            }
        }

        // Position filter (check primary position)
        if let Some(ref positions) = self.positions {
            if !positions.contains(&p.position) {
                return false;
            }
        }

        // Age filters — parse year from "YYYY-MM-DD", compare to 2026
        if self.age_min.is_some() || self.age_max.is_some() {
            let age_opt = p
                .birth_date
                .as_deref()
                .and_then(|bd| bd.split('-').next())
                .and_then(|yr| yr.parse::<u32>().ok())
                .map(|birth_year| 2026u32.saturating_sub(birth_year) as u8);

            match age_opt {
                None => return false, // no birth_date → cannot satisfy age filter
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

        // Nationality filter
        if let Some(ref nats) = self.nationalities {
            let matches_nat = p
                .nationality_code
                .as_deref()
                .map(|nc| nats.iter().any(|n| n.eq_ignore_ascii_case(nc)))
                .unwrap_or(false);
            if !matches_nat {
                return false;
            }
        }

        // Region filter (based on birth_country)
        if let Some(ref regions) = self.regions {
            let matches_region = p
                .birth_country
                .as_deref()
                .map(|bc| regions.contains(&Region::from_country(bc)))
                .unwrap_or(false);
            if !matches_region {
                return false;
            }
        }

        // Undrafted filter
        if let Some(want_undrafted) = self.undrafted {
            let is_undrafted = p.draft_year.is_none();
            if want_undrafted != is_undrafted {
                return false;
            }
        }

        // Draft year filter
        if let Some(ref years) = self.draft_years {
            let matches_year = p.draft_year.map(|dy| years.contains(&dy)).unwrap_or(false);
            if !matches_year {
                return false;
            }
        }

        // Draft round filter
        if let Some(ref rounds) = self.draft_rounds {
            let matches_round = p
                .draft_round
                .map(|dr| rounds.contains(&dr))
                .unwrap_or(false);
            if !matches_round {
                return false;
            }
        }

        // Draft pick max filter
        if let Some(max_pick) = self.draft_pick_max {
            let matches_pick = p
                .draft_overall
                .map(|pick| pick <= max_pick)
                .unwrap_or(false);
            if !matches_pick {
                return false;
            }
        }

        // Rookie filter
        if let Some(true) = self.rookie_only {
            if p.rookie_season != Some(crate::CURRENT_SEASON) {
                return false;
            }
        }

        // PPG min filter — compares against per-game rate (pace_82 / 82).
        // Flag is --ppg-min 0.80, meaning "at least 0.80 points per game".
        if let Some(ppg_min) = self.ppg_min {
            let ppg = p.pace_score.map(|ps| ps.pace_82 / 82.0).unwrap_or(0.0);
            if ppg < ppg_min {
                return false;
            }
        }

        // PPG max filter — same per-game scale
        if let Some(ppg_max) = self.ppg_max {
            let ppg = p.pace_score.map(|ps| ps.pace_82 / 82.0).unwrap_or(0.0);
            if ppg > ppg_max {
                return false;
            }
        }

        // GP min filter
        if let Some(gp_min) = self.gp_min {
            let gp = p.gp_status.gp().unwrap_or(0);
            if gp < gp_min {
                return false;
            }
        }

        // Handedness filter
        if let Some(ref hand) = self.handedness {
            let matches_hand = p
                .shoots_catches
                .as_deref()
                .map(|sc| sc.eq_ignore_ascii_case(hand))
                .unwrap_or(false);
            if !matches_hand {
                return false;
            }
        }

        // TOI minimum (seconds per game)
        if let Some(toi_min) = self.toi_min_sec {
            let toi = p.toi_per_game_sec.unwrap_or(0.0);
            if toi < toi_min {
                return false;
            }
        }

        // Plus/minus minimum
        if let Some(pm_min) = self.plus_minus_min {
            if p.plus_minus < pm_min {
                return false;
            }
        }

        // Shots per game minimum
        if let Some(spg_min) = self.shots_pg_min {
            let gp = p.gp().unwrap_or(0);
            let spg = if gp > 0 { p.shots as f32 / gp as f32 } else { 0.0 };
            if spg < spg_min {
                return false;
            }
        }

        // Birth province/state filter
        if let Some(ref provinces) = self.birth_provinces {
            let matches = p
                .birth_state_province
                .as_deref()
                .map(|prov| provinces.iter().any(|pr| pr.eq_ignore_ascii_case(prov)))
                .unwrap_or(false);
            if !matches {
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
    use crate::model::{GpStatus, PaceScore, Player, TeamAbbr};
    use crate::name::normalize_name;

    fn make_player(
        name: &str,
        team: &str,
        position: Position,
        gp: u32,
        goals: u32,
        assists: u32,
    ) -> Player {
        let pace_score = if gp >= 10 {
            let pace_82 = (goals + assists) as f64 / gp as f64 * 82.0;
            let goals_per_82 = goals as f64 / gp as f64 * 82.0;
            Some(PaceScore {
                pace_82,
                goals_per_82,
                raw_points: goals + assists,
                gp,
            })
        } else {
            None
        };
        Player {
            nhl_id: None,
            full_name: name.to_owned(),
            name_normalized: normalize_name(name),
            team: TeamAbbr(team.to_owned()),
            position,
            eligible_pos: vec![position],
            gp_status: GpStatus::from_gp(gp),
            season_goals: goals,
            season_assists: assists,
            season_points: goals + assists,
            pace_score,
            pp_goals: 0, pp_points: 0,
            sh_goals: 0, sh_points: 0,
            gwg: 0, ot_goals: 0,
            shots: 0, shooting_pct: None,
            plus_minus: 0,
            toi_per_game_sec: None,
            faceoff_win_pct: None,
            hits: 0, blocked_shots: 0, missed_shots: 0,
            giveaways: 0, takeaways: 0, pim: 0,
            xg: None, xg_per_60: None, cf_pct_5v5: None, ff_pct_5v5: None, xgf_pct_5v5: None,
            headshot_url: None,
            sweater_number: None,
            birth_date: None,
            birth_country: None,
            nationality_code: None,
            birth_city: None,
            birth_state_province: None,
            shoots_catches: None,
            height_in_inches: None,
            weight_lbs: None,
            draft_year: None,
            draft_round: None,
            draft_overall: None,
            rookie_season: None,
            contract_expiry_year: None,
            expiry_type: None,
            salary: None,
        }
    }

    #[test]
    fn filter_by_position() {
        let players = vec![
            make_player("Center One", "SEA", Position::Center, 60, 20, 40),
            make_player("Left Wing", "SEA", Position::LeftWing, 60, 15, 30),
            make_player("Center Two", "NYR", Position::Center, 55, 18, 35),
            make_player("Defenseman", "SEA", Position::Defense, 65, 5, 25),
        ];
        let filter = PlayerFilter {
            positions: Some(vec![Position::Center]),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 2, "only Centers should be returned");
        assert!(result.iter().all(|p| p.position == Position::Center));
    }

    #[test]
    fn filter_by_team() {
        let players = vec![
            make_player("Sea Player One", "SEA", Position::Center, 60, 20, 40),
            make_player("Sea Player Two", "SEA", Position::LeftWing, 55, 15, 25),
            make_player("Rangers Player", "NYR", Position::Center, 70, 25, 50),
            make_player("Oilers Player", "EDM", Position::RightWing, 65, 22, 35),
        ];
        let filter = PlayerFilter {
            teams: Some(vec!["SEA".to_owned()]),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 2, "only SEA players should be returned");
        assert!(result.iter().all(|p| p.team.as_str() == "SEA"));
    }

    #[test]
    fn filter_by_ppg_min() {
        // PPG (per game) for (20+40)/60 = 1.000
        // PPG (per game) for (5+10)/50  = 0.300
        // PPG (per game) for (30+60)/70 = 1.286
        let players = vec![
            make_player("High Scorer",  "SEA", Position::Center,    60, 20, 40), // 1.000 ppg
            make_player("Low Scorer",   "NYR", Position::LeftWing,  50,  5, 10), // 0.300 ppg
            make_player("Elite Scorer", "EDM", Position::Center,    70, 30, 60), // 1.286 ppg
        ];
        let filter = PlayerFilter {
            ppg_min: Some(0.90), // ≥ 0.90 per game → High and Elite qualify
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 2, "only players with PPG >= 0.90 should match");
        assert!(result.iter().all(|p| {
            p.pace_score.map(|ps| ps.pace_82 / 82.0 >= 0.90).unwrap_or(false)
        }));
    }

    #[test]
    fn filter_combined_pos_and_team() {
        let players = vec![
            make_player("SEA Center", "SEA", Position::Center, 60, 20, 40),
            make_player("SEA LW", "SEA", Position::LeftWing, 55, 15, 25),
            make_player("NYR Center", "NYR", Position::Center, 70, 25, 50),
            make_player("SEA Defense", "SEA", Position::Defense, 65, 5, 25),
        ];
        let filter = PlayerFilter {
            teams: Some(vec!["SEA".to_owned()]),
            positions: Some(vec![Position::Center]),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1, "only SEA Centers should match");
        assert_eq!(result[0].full_name, "SEA Center");
    }

    #[test]
    fn filter_empty_result_is_ok() {
        let players = vec![
            make_player("Player One", "SEA", Position::Center, 60, 20, 40),
            make_player("Player Two", "NYR", Position::LeftWing, 55, 15, 25),
        ];
        // Filter for a team with no players in the list
        let filter = PlayerFilter {
            teams: Some(vec!["VGK".to_owned()]),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(
            result.len(),
            0,
            "no matches should return empty vec, not error"
        );
    }

    // ── New filter field tests ────────────────────────────────────────────────

    fn make_player_with_toi(name: &str, toi_sec: f32, gp: u32) -> Player {
        let mut p = make_player(name, "SEA", Position::Center, gp, 10, 20);
        p.toi_per_game_sec = Some(toi_sec);
        p
    }

    #[test]
    fn l0_filter_toi_min_sec_includes_above_threshold() {
        let players = vec![
            make_player_with_toi("High TOI", 1200.0, 60),  // 20:00
            make_player_with_toi("Low TOI", 900.0, 60),    // 15:00
        ];
        let filter = PlayerFilter {
            toi_min_sec: Some(1100.0),  // 18:20 threshold
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "High TOI");
    }

    #[test]
    fn l0_filter_toi_min_sec_excludes_below_threshold() {
        let players = vec![
            make_player_with_toi("Elite TOI", 1500.0, 60),
            make_player_with_toi("Average TOI", 900.0, 60),
        ];
        let filter = PlayerFilter {
            toi_min_sec: Some(1200.0),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "Elite TOI");
    }

    #[test]
    fn l0_filter_toi_min_sec_excludes_player_with_no_toi() {
        // Player with no TOI data should be excluded when toi_min is set
        let mut p = make_player("No TOI", "SEA", Position::Center, 60, 10, 20);
        p.toi_per_game_sec = None;
        let players = vec![p];
        let filter = PlayerFilter {
            toi_min_sec: Some(900.0),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert!(result.is_empty(), "player with None TOI should be excluded");
    }

    #[test]
    fn l0_filter_plus_minus_min_includes_above_threshold() {
        let mut positive = make_player("Plus Player", "EDM", Position::Center, 60, 20, 30);
        positive.plus_minus = 15;
        let mut negative = make_player("Minus Player", "COL", Position::Defense, 60, 5, 15);
        negative.plus_minus = -10;
        let players = vec![positive, negative];

        let filter = PlayerFilter {
            plus_minus_min: Some(0),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "Plus Player");
    }

    #[test]
    fn l0_filter_plus_minus_min_excludes_negatives() {
        let mut p1 = make_player("Even", "SEA", Position::Center, 60, 10, 20);
        p1.plus_minus = 0;
        let mut p2 = make_player("Negative", "VAN", Position::LeftWing, 60, 8, 15);
        p2.plus_minus = -5;
        let players = vec![p1, p2];

        let filter = PlayerFilter {
            plus_minus_min: Some(0),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "Even");
    }

    #[test]
    fn l0_filter_shots_pg_min_includes_shooters() {
        // shots=120, gp=60 → 2.0 shots/game
        let mut shooter = make_player("Shooter", "SEA", Position::RightWing, 60, 20, 30);
        shooter.shots = 120;
        // shots=30, gp=60 → 0.5 shots/game
        let mut passive = make_player("Passive", "EDM", Position::LeftWing, 60, 5, 10);
        passive.shots = 30;

        let players = vec![shooter, passive];
        let filter = PlayerFilter {
            shots_pg_min: Some(1.5),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "Shooter");
    }

    #[test]
    fn l0_filter_shots_pg_min_excludes_no_gp() {
        // Player with GP=0 gets shots_pg=0.0 — should be excluded
        let mut p = make_player("No GP", "SEA", Position::Center, 0, 0, 0);
        p.shots = 10; // shots but no GP context
        let players = vec![p];

        let filter = PlayerFilter {
            shots_pg_min: Some(0.5),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert!(result.is_empty(), "zero GP player should be excluded from shots_pg filter");
    }

    #[test]
    fn l0_filter_birth_province_on_matches_ontario() {
        let mut on_player = make_player("Ontario Guy", "TOR", Position::Center, 60, 20, 30);
        on_player.birth_state_province = Some("ON".to_owned());
        let mut ab_player = make_player("Alberta Guy", "EDM", Position::Center, 60, 15, 25);
        ab_player.birth_state_province = Some("AB".to_owned());
        let players = vec![on_player, ab_player];

        let filter = PlayerFilter {
            birth_provinces: Some(vec!["ON".to_owned()]),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "Ontario Guy");
    }

    #[test]
    fn l0_filter_birth_province_case_insensitive() {
        let mut p = make_player("BC Guy", "VAN", Position::LeftWing, 60, 10, 20);
        p.birth_state_province = Some("BC".to_owned());
        let players = vec![p];

        let filter = PlayerFilter {
            birth_provinces: Some(vec!["bc".to_owned()]),  // lowercase
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1, "birth_province filter should be case-insensitive");
    }

    #[test]
    fn l0_filter_birth_province_multiple_provinces() {
        let mut on_player = make_player("Ontario", "TOR", Position::Center, 60, 20, 30);
        on_player.birth_state_province = Some("ON".to_owned());
        let mut qc_player = make_player("Quebec", "MTL", Position::Center, 60, 15, 25);
        qc_player.birth_state_province = Some("QC".to_owned());
        let mut ab_player = make_player("Alberta", "EDM", Position::RightWing, 60, 10, 20);
        ab_player.birth_state_province = Some("AB".to_owned());
        let players = vec![on_player, qc_player, ab_player];

        let filter = PlayerFilter {
            birth_provinces: Some(vec!["ON".to_owned(), "QC".to_owned()]),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 2);
        let names: Vec<&str> = result.iter().map(|p| p.full_name.as_str()).collect();
        assert!(names.contains(&"Ontario"));
        assert!(names.contains(&"Quebec"));
    }

    #[test]
    fn l0_filter_birth_province_excludes_player_with_no_province() {
        let mut p = make_player("Intl Player", "NYR", Position::Defense, 60, 5, 15);
        p.birth_state_province = None;
        let players = vec![p];

        let filter = PlayerFilter {
            birth_provinces: Some(vec!["ON".to_owned()]),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert!(result.is_empty(), "player with None birth_state_province should be excluded");
    }

    #[test]
    fn l0_filter_combined_toi_and_plus_minus() {
        let mut elite = make_player("Elite", "EDM", Position::Center, 60, 30, 50);
        elite.toi_per_game_sec = Some(1400.0);
        elite.plus_minus = 20;

        let mut grinder = make_player("Grinder", "SEA", Position::Center, 60, 5, 10);
        grinder.toi_per_game_sec = Some(700.0);
        grinder.plus_minus = -5;

        let mut steady = make_player("Steady", "COL", Position::Center, 60, 15, 25);
        steady.toi_per_game_sec = Some(1200.0);
        steady.plus_minus = -2;

        let players = vec![elite, grinder, steady];
        let filter = PlayerFilter {
            toi_min_sec: Some(1100.0),
            plus_minus_min: Some(0),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "Elite");
    }
}
