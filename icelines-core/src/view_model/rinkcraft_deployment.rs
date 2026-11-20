//! Descriptive workload, never a unit/station assignment or capability rating.
use crate::{identity::PlayerId, model::Season, season_stats::SeasonType, SeasonStatsRead};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageRow {
    pub key: String,
    pub label: String,
    pub seconds: u32,
    pub seconds_per_game: f64,
    pub share: Option<f64>,
    pub per_game_display: String,
    pub minutes_display: String,
    pub share_display: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentEvidence {
    pub player_id: PlayerId,
    pub season: Season,
    pub season_type: SeasonType,
    pub games: u32,
    pub goalie: bool,
    pub starts: Option<u32>,
    pub rows: Vec<UsageRow>,
    pub source_label: String,
    pub source_url: Option<String>,
    pub source_sha256: Option<String>,
    pub notes: Vec<String>,
}
pub struct UsageInput {
    pub player_id: PlayerId,
    pub season: Season,
    pub season_type: SeasonType,
    pub games: u32,
    pub total: u32,
    pub skater_strengths: Option<[u32; 3]>,
    pub goalie_starts: Option<u32>,
}
pub fn build(input: UsageInput) -> Result<DeploymentEvidence, String> {
    let UsageInput {
        player_id,
        season,
        season_type,
        games,
        total,
        skater_strengths,
        goalie_starts,
    } = input;
    if games == 0 || skater_strengths.is_some() == goalie_starts.is_some() {
        return Err("usage requires positive GP and exactly one skater/goalie payload".into());
    }
    if let Some(strengths) = skater_strengths {
        if strengths.iter().map(|&s| u64::from(s)).sum::<u64>() != u64::from(total) {
            return Err("EV + PP + SH must equal total TOI; OT is not added again".into());
        }
    }
    if goalie_starts.is_some_and(|s| s > games) {
        return Err("starts exceed appearances".into());
    }
    let row = |key: &str, label: &str, seconds: u32| {
        let per_game = f64::from(seconds) / f64::from(games);
        let rounded = per_game.round() as u64;
        let share = (total > 0).then(|| f64::from(seconds) / f64::from(total));
        UsageRow {
            key: key.into(),
            label: label.into(),
            seconds,
            seconds_per_game: per_game,
            share,
            per_game_display: format!("{}:{:02}", rounded / 60, rounded % 60),
            minutes_display: format!("{:.1}", f64::from(seconds) / 60.0),
            share_display: share
                .map(|s| format!("{:.1}%", 100.0 * s))
                .unwrap_or_else(|| "—".into()),
        }
    };
    let mut rows = vec![row("total", "All strengths", total)];
    if let Some([ev, pp, sh]) = skater_strengths {
        rows.extend([
            row("ev", "Even strength", ev),
            row("pp", "Power play", pp),
            row("sh", "Shorthanded", sh),
        ]);
    }
    Ok(DeploymentEvidence { player_id,season,season_type,games,goalie:goalie_starts.is_some(),starts:goalie_starts,rows,
        source_label:if goalie_starts.is_some(){"NHL goalie/summary"}else{"NHL skater/timeonice"}.into(),source_url:None,source_sha256:None,
        notes:vec!["Season usage, not an assigned line, pair, PP unit, station or capability grade.".into(),
            "Per-game TOI = total seconds / appearances; rounded only for display. Shares use this player's total ice time, not team opportunities.".into(),
            if goalie_starts.is_some(){"Goalie workload is not skater PP/PK deployment or save quality."}else{"Even strength is not exclusively 5v5. Overtime can overlap strength categories; do not add OT to their sum."}.into()] })
}
pub fn from_read<R: SeasonStatsRead>(r: &R) -> Option<DeploymentEvidence> {
    let (total, strengths, starts) = if let Some(g) = r.goalie() {
        (g.time_on_ice_sec, None, Some(g.games_started))
    } else {
        let t = r.time_on_ice()?;
        (
            t.time_on_ice_sec,
            Some([
                t.ev_time_on_ice_sec,
                t.pp_time_on_ice_sec,
                t.sh_time_on_ice_sec,
            ]),
            None,
        )
    };
    build(UsageInput {
        player_id: r.player_id(),
        season: r.season(),
        season_type: r.season_type(),
        games: r.totals().gp,
        total,
        skater_strengths: strengths,
        goalie_starts: starts,
    })
    .ok()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> UsageInput {
        UsageInput {
            player_id: PlayerId(1),
            season: Season(20242025),
            season_type: SeasonType::Regular,
            games: 2,
            total: 3600,
            skater_strengths: Some([3000, 600, 0]),
            goalie_starts: None,
        }
    }
    #[test]
    fn usage_keeps_zero_and_uses_appearances_and_player_seconds() {
        let c = build(input()).unwrap();
        assert_eq!(c.rows[0].per_game_display, "30:00");
        assert_eq!(c.rows[2].seconds_per_game, 300.0);
        assert_eq!(c.rows[3].per_game_display, "0:00");
        assert_eq!(c.rows[3].share, Some(0.0));
        assert!(c.notes[2].contains("not exclusively 5v5"));
    }
    #[test]
    fn incoherent_inputs_reject_instead_of_guessing() {
        let mut i = input();
        i.games = 0;
        assert!(build(i).is_err());
        let mut i = input();
        i.total += 1;
        assert!(build(i).is_err());
        let mut i = input();
        i.goalie_starts = Some(1);
        assert!(build(i).is_err());
    }
    #[test]
    fn goalie_starts_and_zero_total_are_separate() {
        let mut i = input();
        i.skater_strengths = None;
        i.goalie_starts = Some(1);
        i.total = 0;
        let c = build(i).unwrap();
        assert_eq!(c.rows.len(), 1);
        assert_eq!(c.starts, Some(1));
        assert_eq!(c.rows[0].share, None);
        let mut i = input();
        i.skater_strengths = None;
        i.goalie_starts = Some(3);
        assert!(build(i).is_err());
    }
}
