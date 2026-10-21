//! Compare the additive read surface with observations from unmodified production code.
#[path = "support/season_read_cases.rs"]
mod cases;
use icelines_core::{
    identity::PlayerId,
    model::{Position, Season},
    season_stats::*,
    stats_catalog::{parse_filter_expr, StatId},
    stats_repository::PlayerView,
    SeasonStatsRead,
};
use serde_json::{json, Value};

// Deliberately cannot yield &SeasonStats: payloads live in independent allocations.
// The catalog must consume the borrowed read contract rather than materialize a row.
struct SplitRow {
    player_id: PlayerId,
    season: Season,
    season_type: SeasonType,
    position: Position,
    sweater_number: Option<u32>,
    team_stints: Vec<TeamStint>,
    totals: StatTotals,
    realtime: Option<Box<RealtimeStats>>,
    advanced: Option<Box<AdvancedStats>>,
    goalie: Option<Box<GoalieSeasonStats>>,
    time_on_ice: Option<Box<TimeOnIceStats>>,
    goals_for_against: Option<Box<GoalsForAgainstStats>>,
    goalie_advanced: Option<Box<GoalieAdvancedStats>>,
    goalie_saves_by_strength: Option<Box<GoalieSavesByStrengthStats>>,
    goalie_bios: Option<Box<GoalieBios>>,
}
impl From<SeasonStats> for SplitRow {
    fn from(row: SeasonStats) -> Self {
        let SeasonStats {
            player_id,
            season,
            season_type,
            position,
            sweater_number,
            team_stints,
            totals,
            realtime,
            advanced,
            goalie,
            time_on_ice,
            goals_for_against,
            goalie_advanced,
            goalie_saves_by_strength,
            goalie_bios,
        } = row;
        Self {
            player_id,
            season,
            season_type,
            position,
            sweater_number,
            team_stints,
            totals,
            realtime: realtime.map(Box::new),
            advanced: advanced.map(Box::new),
            goalie: goalie.map(Box::new),
            time_on_ice: time_on_ice.map(Box::new),
            goals_for_against: goals_for_against.map(Box::new),
            goalie_advanced: goalie_advanced.map(Box::new),
            goalie_saves_by_strength: goalie_saves_by_strength.map(Box::new),
            goalie_bios: goalie_bios.map(Box::new),
        }
    }
}
impl SeasonStatsRead for SplitRow {
    fn player_id(&self) -> PlayerId {
        self.player_id
    }
    fn season(&self) -> Season {
        self.season
    }
    fn season_type(&self) -> SeasonType {
        self.season_type
    }
    fn position(&self) -> Position {
        self.position
    }
    fn sweater_number(&self) -> Option<u32> {
        self.sweater_number
    }
    fn totals(&self) -> &StatTotals {
        &self.totals
    }
    fn team_stints(&self) -> &[TeamStint] {
        &self.team_stints
    }
    fn realtime(&self) -> Option<&RealtimeStats> {
        self.realtime.as_deref()
    }
    fn advanced(&self) -> Option<&AdvancedStats> {
        self.advanced.as_deref()
    }
    fn goalie(&self) -> Option<&GoalieSeasonStats> {
        self.goalie.as_deref()
    }
    fn time_on_ice(&self) -> Option<&TimeOnIceStats> {
        self.time_on_ice.as_deref()
    }
    fn goals_for_against(&self) -> Option<&GoalsForAgainstStats> {
        self.goals_for_against.as_deref()
    }
    fn goalie_advanced(&self) -> Option<&GoalieAdvancedStats> {
        self.goalie_advanced.as_deref()
    }
    fn goalie_saves_by_strength(&self) -> Option<&GoalieSavesByStrengthStats> {
        self.goalie_saves_by_strength.as_deref()
    }
    fn goalie_bios(&self) -> Option<&GoalieBios> {
        self.goalie_bios.as_deref()
    }
}
fn observations(rows: &[&dyn SeasonStatsRead]) -> Value {
    let stats:Vec<_>=StatId::all().iter().map(|stat| {
        let values:Vec<_>=rows.iter().map(|r|stat.read_from(*r).map(f64::to_bits)).collect();
        let mut sorted=rows.to_vec();sorted.sort_by(|a,b|stat.sort_read_cmp(*a,*b));
        json!({"stat":stat.cli_key(),"value_bits":values,"sorted_ids":sorted.iter().map(|r|r.player_id().0).collect::<Vec<_>>()})
    }).collect();
    let filters:Vec<_>=cases::filters().iter().map(|filter| {
        let parsed=parse_filter_expr(filter).unwrap();json!({"filter":filter,"matches":rows.iter().map(|r|parsed.matches_read(*r)).collect::<Vec<_>>()})
    }).collect();
    json!({"schema_version":1,"scope":"synthetic-legacy-catalog-boundaries","rows":rows.len(),"stats":stats,"filters":filters})
}
fn assert_observations(label: &str, actual: Value, expected: &Value) {
    assert_eq!(actual["rows"], expected["rows"]);
    assert_eq!(actual["stats"].as_array().unwrap().len(), 108);
    for (a, b) in actual["stats"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["stats"].as_array().unwrap())
    {
        assert_eq!(a, b, "{label} stat {}", a["stat"]);
    }
    assert_eq!(
        actual["filters"], expected["filters"],
        "{label} filter semantics"
    );
}
#[test]
fn all_catalog_values_bits_sort_orders_and_filters_match_pre_refactor_oracle() {
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/season-read-legacy.json")).unwrap();
    let cases = cases::cases();
    assert_eq!(cases.len(), 288);
    assert_observations(
        "legacy PlayerView",
        cases::legacy_observations(&cases),
        &expected,
    );
    let inline: Vec<&dyn SeasonStatsRead> = cases
        .iter()
        .map(|(_, row)| row as &dyn SeasonStatsRead)
        .collect();
    assert_observations("inline read", observations(&inline), &expected);
    let views: Vec<_> = cases
        .iter()
        .map(|(identity, stats)| PlayerView {
            identity,
            stats,
            contract: None,
        })
        .collect();
    let view_reads: Vec<&dyn SeasonStatsRead> =
        views.iter().map(|r| r as &dyn SeasonStatsRead).collect();
    assert_observations("PlayerView read", observations(&view_reads), &expected);
    let split: Vec<_> = cases
        .iter()
        .map(|(_, row)| SplitRow::from(row.clone()))
        .collect();
    let split_reads: Vec<&dyn SeasonStatsRead> =
        split.iter().map(|r| r as &dyn SeasonStatsRead).collect();
    assert_observations(
        "separately owned payloads",
        observations(&split_reads),
        &expected,
    );
}
#[test]
fn typed_accessors_preserve_metadata_and_every_missing_payload() {
    for (_, row) in cases::cases() {
        let split = SplitRow::from(row.clone());
        assert_eq!(split.player_id(), row.player_id);
        assert_eq!(split.season(), row.season);
        assert_eq!(split.season_type(), row.season_type);
        assert_eq!(split.position(), row.position);
        assert_eq!(split.sweater_number(), row.sweater_number);
        assert_eq!(split.totals(), &row.totals);
        assert_eq!(split.team_stints(), row.team_stints);
        assert_eq!(split.realtime(), row.realtime.as_ref());
        assert_eq!(split.advanced(), row.advanced.as_ref());
        assert_eq!(split.goalie(), row.goalie.as_ref());
        assert_eq!(split.time_on_ice(), row.time_on_ice.as_ref());
        assert_eq!(split.goals_for_against(), row.goals_for_against.as_ref());
        assert_eq!(split.goalie_advanced(), row.goalie_advanced.as_ref());
        assert_eq!(
            split.goalie_saves_by_strength(),
            row.goalie_saves_by_strength.as_ref()
        );
        assert_eq!(split.goalie_bios(), row.goalie_bios.as_ref());
    }
}
#[test]
fn player_view_identity_sort_key_semantics_are_preserved() {
    let (mut identity, row) = cases::cases().remove(0);
    identity.id = PlayerId(123456);
    let view = PlayerView {
        identity: &identity,
        stats: &row,
        contract: None,
    };
    assert_eq!(SeasonStatsRead::player_id(&view), view.id());
}

#[test]
fn borrowed_catalog_keeps_toi_precedence_floors_trade_and_position_guards() {
    let cases = cases::cases();
    let rich = |toi, traded| {
        cases
            .iter()
            .find(|(_, r)| {
                r.totals.gp == 10
                    && r.goalie.is_none()
                    && r.realtime.as_ref().is_some_and(|p| p.hits == 7)
                    && r.time_on_ice
                        .as_ref()
                        .is_some_and(|p| p.time_on_ice_sec == toi)
                    && (r.team_stints.len() > 1) == traded
            })
            .unwrap()
            .1
            .clone()
    };
    let below = SplitRow::from(rich(299, false));
    let floor = SplitRow::from(rich(300, false));
    assert_eq!(StatId::HitsPer60.read_from(&below), None);
    assert_eq!(StatId::HitsPer60.read_from(&floor), Some(84.0));
    assert_eq!(StatId::TotalToiPerGame.read_from(&floor), Some(999.0));
    let traded = SplitRow::from(rich(300, true));
    assert_eq!(StatId::EvGoalsFor.read_from(&traded), None);
    assert_eq!(
        StatId::EvenStrengthTimeOnIcePerGame.read_from(&traded),
        Some(7.0)
    );
    let mut row = rich(300, false);
    row.totals.faceoff_win_pct = Some(0.0);
    row.position = Position::Defense;
    let filter = parse_filter_expr(&format!("{}>=0.5", StatId::FaceoffWinPct.cli_key())).unwrap();
    assert!(filter.matches_read(&SplitRow::from(row.clone())));
    row.position = Position::Center;
    assert!(!filter.matches_read(&SplitRow::from(row)));
}
