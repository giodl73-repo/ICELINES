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
            if p.rookie_season.is_none() {
                return false;
            }
        }

        // PPG (pace_82) min filter
        if let Some(ppg_min) = self.ppg_min {
            let pace = p.pace_score.map(|ps| ps.pace_82).unwrap_or(0.0);
            if pace < ppg_min {
                return false;
            }
        }

        // PPG (pace_82) max filter
        if let Some(ppg_max) = self.ppg_max {
            let pace = p.pace_score.map(|ps| ps.pace_82).unwrap_or(0.0);
            if pace > ppg_max {
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
            headshot_url: None,
            birth_date: None,
            birth_country: None,
            nationality_code: None,
            shoots_catches: None,
            draft_year: None,
            draft_round: None,
            draft_overall: None,
            rookie_season: None,
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
        // pace_82 for (20+40)/60 * 82 = 82.0
        // pace_82 for (5+10)/50 * 82 = 24.6
        // pace_82 for (30+60)/70 * 82 = 105.4...
        let players = vec![
            make_player("High Scorer", "SEA", Position::Center, 60, 20, 40), // pace=82.0
            make_player("Low Scorer", "NYR", Position::LeftWing, 50, 5, 10), // pace=24.6
            make_player("Elite Scorer", "EDM", Position::Center, 70, 30, 60), // pace=105.4
        ];
        let filter = PlayerFilter {
            ppg_min: Some(80.0),
            ..PlayerFilter::new()
        };
        let result = filter.apply(&players);
        assert_eq!(
            result.len(),
            2,
            "only players with pace_82 >= 80.0 should match"
        );
        assert!(result
            .iter()
            .all(|p| { p.pace_score.map(|ps| ps.pace_82 >= 80.0).unwrap_or(false) }));
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
}
