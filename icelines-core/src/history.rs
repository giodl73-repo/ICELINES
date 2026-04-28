//! Multi-season career history types.

use serde::{Deserialize, Serialize};

/// One season's worth of stats for a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonLine {
    pub season:     String,   // "20252026"
    pub team:       String,
    pub gp:         u32,
    pub goals:      u32,
    pub assists:    u32,
    /// Points per game this season
    pub ppg:        f32,
    /// Goals per 82 games (pace-projected)
    pub goals_per_82: f32,
}

impl SeasonLine {
    pub fn new(season: &str, team: &str, gp: u32, goals: u32, assists: u32) -> Self {
        let ppg = if gp > 0 { (goals + assists) as f32 / gp as f32 } else { 0.0 };
        let goals_per_82 = if gp > 0 { goals as f32 / gp as f32 * 82.0 } else { 0.0 };
        Self {
            season:       season.to_owned(),
            team:         team.to_owned(),
            gp, goals, assists, ppg, goals_per_82,
        }
    }

    pub fn points(&self) -> u32 { self.goals + self.assists }
    pub fn pts_per_82(&self) -> f32 { if self.gp > 0 { self.ppg * 82.0 } else { 0.0 } }
}

/// Multi-season career summary for a player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerSummary {
    pub player_id:    Option<u32>,
    pub full_name:    String,
    pub seasons:      Vec<SeasonLine>,  // sorted newest first
    pub career_ppg:   f32,
    pub peak_ppg:     f32,
    pub peak_season:  String,
}

impl CareerSummary {
    pub fn from_seasons(
        player_id: Option<u32>,
        full_name: &str,
        mut lines: Vec<SeasonLine>,
    ) -> Self {
        // Sort newest first
        lines.sort_by(|a, b| b.season.cmp(&a.season));

        let total_pts: u32 = lines.iter().map(|l| l.goals + l.assists).sum();
        let total_gp:  u32 = lines.iter().map(|l| l.gp).sum();
        let career_ppg = if total_gp > 0 {
            total_pts as f32 / total_gp as f32
        } else { 0.0 };

        // Peak = season with highest PPG (min 10 GP)
        let (peak_ppg, peak_season) = lines.iter()
            .filter(|l| l.gp >= 10)
            .max_by(|a, b| a.ppg.partial_cmp(&b.ppg).unwrap_or(std::cmp::Ordering::Equal))
            .map(|l| (l.ppg, l.season.clone()))
            .unwrap_or((0.0, String::new()));

        Self {
            player_id,
            full_name:   full_name.to_owned(),
            seasons:     lines,
            career_ppg,
            peak_ppg,
            peak_season,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(season: &str, gp: u32, g: u32, a: u32) -> SeasonLine {
        SeasonLine::new(season, "EDM", gp, g, a)
    }

    #[test]
    fn l0_career_ppg_weighted_by_gp() {
        // Season A: 50G+90A in 82 GP = 1.707 ppg
        // Season B: 10G+20A in 40 GP = 0.750 ppg
        // Career:   60G+110A in 122 GP = 170/122 = 1.393 ppg
        let lines = vec![line("20252026", 82, 50, 90), line("20242025", 40, 10, 20)];
        let summary = CareerSummary::from_seasons(None, "Test", lines);
        assert!((summary.career_ppg - 170.0/122.0).abs() < 0.001,
            "expected {:.3}, got {:.3}", 170.0/122.0, summary.career_ppg);
    }

    #[test]
    fn l0_peak_season_is_highest_ppg() {
        let lines = vec![
            line("20252026", 82, 50, 90),  // 1.707 ppg ← peak
            line("20242025", 75, 30, 50),  // 1.067 ppg
        ];
        let summary = CareerSummary::from_seasons(None, "Test", lines);
        assert_eq!(summary.peak_season, "20252026");
        assert!((summary.peak_ppg - 140.0/82.0).abs() < 0.001);
    }

    #[test]
    fn l0_seasons_sorted_newest_first() {
        let lines = vec![line("20222023", 70, 20, 30), line("20252026", 82, 50, 90)];
        let summary = CareerSummary::from_seasons(None, "Test", lines);
        assert_eq!(summary.seasons[0].season, "20252026");
        assert_eq!(summary.seasons[1].season, "20222023");
    }

    #[test]
    fn l0_below_min_gp_excluded_from_peak() {
        // 5 GP season should not be considered for peak
        let lines = vec![
            line("20252026", 5, 10, 0),  // 2.0 ppg but only 5 GP — exclude
            line("20242025", 82, 30, 50), // 0.976 ppg — this is peak
        ];
        let summary = CareerSummary::from_seasons(None, "Test", lines);
        assert_eq!(summary.peak_season, "20242025");
    }
}
