//! Experimental descriptive bands. No skill, class or family score is implied.
use crate::{
    identity::PlayerId,
    model::{Position, Season},
    season_stats::SeasonType,
    stats_catalog::StatId,
    stats_repository::ReadPlayerView,
    SeasonStatsRead,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const ATTRIBUTE_PROTOCOL: &str = "rinkcraft.descriptive-attributes.0.1.0-experimental";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeSeason {
    pub protocol: String,
    pub season: Season,
    pub season_type: SeasonType,
    pub group: String,
    pub games: u32,
    pub minimum_games: u32,
    pub minimum_peers: usize,
    pub eligible_peers: usize,
    pub strength_scope: String,
    pub attributes: Vec<Attribute>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub key: String,
    pub label: String,
    pub layer: String,
    pub unit: String,
    pub value: Option<f64>,
    pub percentile: Option<f64>,
    pub band: Option<u8>,
    pub measured_peers: usize,
    pub reference_sha256: String,
    pub state: String,
    pub formula: String,
    pub interpretation: String,
}
struct Definition {
    key: &'static str,
    label: &'static str,
    layer: &'static str,
    stat: StatId,
    per_game: bool,
    unit: &'static str,
    meaning: &'static str,
}
fn definitions(goalie: bool) -> Vec<Definition> {
    use StatId::*;
    if goalie {
        return vec![
        Definition{key:"save-outcome",label:"Observed save outcome",layer:"Observed outcome",stat:SavePct,per_game:false,unit:"Pct",meaning:"Saves relative to shots faced; not adjusted for shot quality or a goalie skill estimate."},
        Definition{key:"shots-faced",label:"Shots faced",layer:"Workload",stat:ShotsAgainst,per_game:true,unit:"PerGame",meaning:"Shots faced per appearance, including relief appearances; workload, not save effectiveness."},
        Definition{key:"starting-workload",label:"Starting workload",layer:"Deployment",stat:GoalieStarts,per_game:false,unit:"Count",meaning:"Starts in this loaded season; affected by season progress and team deployment, not goalie talent."},
    ];
    }
    vec![
        Definition{key:"shooting-involvement",label:"Shooting involvement",layer:"Involvement",stat:Shots,per_game:true,unit:"PerGame",meaning:"Shots on goal per appearance, excluding misses and blocked attempts; not finishing ability."},
        Definition{key:"assist-output",label:"Assist output",layer:"Observed output",stat:Assists,per_game:true,unit:"PerGame",meaning:"Primary and secondary assists per appearance; production, not passing creativity or chance creation."},
        Definition{key:"goal-output",label:"Goal output",layer:"Observed output",stat:Goals,per_game:true,unit:"PerGame",meaning:"Goals per appearance; scoring output, not a persistent finishing estimate."},
        Definition{key:"recorded-contact",label:"Recorded contact",layer:"Involvement",stat:Hits,per_game:true,unit:"PerGame",meaning:"Recorded hits per appearance; not pressure, successful recovery or a hunter classification."},
        Definition{key:"shot-blocking",label:"Shot-blocking involvement",layer:"Involvement",stat:BlockedShots,per_game:true,unit:"PerGame",meaning:"Recorded blocks per appearance; defending exposure can affect this, not complete defensive effectiveness."},
        Definition{key:"ice-time",label:"Ice-time deployment",layer:"Deployment",stat:TotalToiPerGame,per_game:false,unit:"Seconds",meaning:"Average time on ice per appearance; usage, not ability. All manpower situations combined."},
        Definition{key:"conversion-outcome",label:"Observed shot conversion",layer:"Observed outcome",stat:ShootingPct,per_game:false,unit:"Pct",meaning:"Goals divided by shots on goal; shot mix and sampling affect it, not a calibrated finisher grade."},
    ]
}
pub fn is_attribute_key(key: &str) -> bool {
    definitions(false)
        .iter()
        .chain(definitions(true).iter())
        .any(|d| d.key == key)
}
fn group(p: Position) -> &'static str {
    if p.is_forward() {
        "Forwards"
    } else if p == Position::Goalie {
        "Goalies"
    } else {
        "Defense"
    }
}
fn measured<R: SeasonStatsRead>(d: &Definition, v: &ReadPlayerView<'_, R>) -> Option<f64> {
    if !d.stat.applies_to(v.position(), v.is_goalie()) || !d.stat.applies_to_era(v.season()) {
        return None;
    }
    let raw = d.stat.read_from(&v.stats)?;
    let value = if d.per_game {
        if v.gp() == 0 {
            return None;
        }
        raw / f64::from(v.gp())
    } else {
        raw
    };
    value.is_finite().then_some(value)
}
/// Caller supplies one prepared league window. Filtering, dedup and ranking live here.
pub fn build_attribute_season<R: SeasonStatsRead>(
    pid: PlayerId,
    season: Season,
    ty: SeasonType,
    views: Vec<ReadPlayerView<'_, R>>,
) -> Option<AttributeSeason> {
    let mut rows = BTreeMap::new();
    for v in views
        .into_iter()
        .filter(|v| v.season() == season && v.season_type() == ty)
    {
        rows.entry(v.id()).or_insert(v);
    }
    let target = rows.get(&pid)?;
    let goalie = target.position() == Position::Goalie;
    let minimum_games = if goalie { 5 } else { 10 };
    let minimum_peers = if goalie { 10 } else { 20 };
    let eligible: Vec<_> = rows
        .values()
        .filter(|v| group(v.position()) == group(target.position()) && v.gp() >= minimum_games)
        .collect();
    let attributes = definitions(goalie)
        .iter()
        .map(|d| {
            let value = measured(d, target);
            let peers: Vec<_> = eligible
                .iter()
                .filter_map(|v| measured(d, v).map(|x| (v.id(), x)))
                .collect();
            let mut digest = Sha256::new();
            digest.update(ATTRIBUTE_PROTOCOL.as_bytes());
            digest.update(season.0.to_le_bytes());
            digest.update(ty.to_string().as_bytes());
            digest.update(group(target.position()).as_bytes());
            digest.update(d.key.as_bytes());
            for (id, x) in &peers {
                digest.update(id.0.to_le_bytes());
                digest.update(x.to_bits().to_le_bytes());
            }
            let state = if value.is_none() {
                "unavailable"
            } else if target.gp() < minimum_games {
                "insufficient_exposure"
            } else if peers.len() < minimum_peers {
                "insufficient_reference"
            } else {
                "ranked"
            };
            let percentile = value.filter(|_| state == "ranked").map(|x| {
                let below = peers.iter().filter(|(_, v)| *v < x).count();
                let equal = peers.iter().filter(|(_, v)| *v == x).count();
                100.0 * (below as f64 + equal as f64 * 0.5) / peers.len() as f64
            });
            Attribute {
                key: d.key.into(),
                label: d.label.into(),
                layer: d.layer.into(),
                unit: d.unit.into(),
                value,
                percentile,
                band: percentile.map(|p| ((p / 20.0).floor() as u8 + 1).min(5)),
                measured_peers: peers.len(),
                reference_sha256: format!("{:x}", digest.finalize()),
                state: state.into(),
                formula: if d.per_game {
                    format!("{} / games played", d.stat.cli_key())
                } else {
                    format!("catalog: {}", d.stat.cli_key())
                },
                interpretation: d.meaning.into(),
            }
        })
        .collect();
    Some(AttributeSeason {
        protocol: ATTRIBUTE_PROTOCOL.into(),
        season,
        season_type: ty,
        group: group(target.position()).into(),
        games: target.gp(),
        minimum_games,
        minimum_peers,
        eligible_peers: eligible.len(),
        strength_scope: "All strengths · unadjusted".into(),
        attributes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures, stats_repository::StatsRepository};
    fn repo() -> StatsRepository {
        let mut r = StatsRepository::with_lru_cap(4);
        for id in 1..=25 {
            r.upsert_identity(fixtures::identity(id).build()).unwrap();
            let mut row = fixtures::stats(id, 20252026, "AAA").build();
            row.position = Position::Defense;
            row.totals.gp = 10;
            row.totals.shots = id * 10;
            row.totals.goals = 0;
            row.realtime = None;
            r.upsert_stats(row).unwrap();
        }
        r
    }
    #[test]
    fn descriptive_bands_use_rates_ties_zeros_and_missingness() {
        let r = repo();
        let s = build_attribute_season(
            PlayerId(25),
            Season(20252026),
            SeasonType::Regular,
            r.read_league(Season(20252026), SeasonType::Regular)
                .collect(),
        )
        .unwrap();
        let a = &s.attributes[0];
        assert_eq!(a.value, Some(25.0));
        assert_eq!(a.percentile, Some(98.0));
        assert_eq!(a.band, Some(5));
        assert_eq!(s.attributes[2].value, Some(0.0));
        assert_eq!(s.attributes[2].percentile, Some(50.0));
        assert_eq!(s.attributes[2].band, Some(3));
        assert_eq!(s.attributes[3].value, None);
        assert_eq!(s.attributes[3].band, None);
        assert_eq!(a.reference_sha256.len(), 64);
    }
    #[test]
    fn insufficient_exposure_small_reference_and_duplicate_ids_are_explicit() {
        let mut r = repo();
        let mut row = r
            .season(PlayerId(25), Season(20252026), SeasonType::Regular)
            .unwrap()
            .clone();
        row.totals.gp = 1;
        r.upsert_stats(row).unwrap();
        let s = build_attribute_season(
            PlayerId(25),
            Season(20252026),
            SeasonType::Regular,
            r.read_league(Season(20252026), SeasonType::Regular)
                .collect(),
        )
        .unwrap();
        assert_eq!(s.attributes[0].state, "insufficient_exposure");
        assert_eq!(s.attributes[0].band, None);
        let views = r
            .read_league(Season(20252026), SeasonType::Regular)
            .take(2)
            .collect::<Vec<_>>();
        let pid = views[0].id();
        let mut duplicate = views.clone();
        duplicate.extend(views);
        let small =
            build_attribute_season(pid, Season(20252026), SeasonType::Regular, duplicate).unwrap();
        assert_eq!(small.eligible_peers, 2);
        assert_eq!(small.attributes[0].state, "insufficient_reference");
        assert!(build_attribute_season(
            pid,
            Season(20252026),
            SeasonType::Playoff,
            r.read_league(Season(20252026), SeasonType::Regular)
                .collect()
        )
        .is_none());
    }

    #[test]
    fn rates_use_actual_games_and_forward_cohorts_do_not_change_defense_ranks() {
        let mut r = repo();
        let mut row = r
            .season(PlayerId(2), Season(20252026), SeasonType::Regular)
            .unwrap()
            .clone();
        row.totals.gp = 20;
        row.totals.shots = 20;
        r.upsert_stats(row).unwrap();
        for id in 101..=130 {
            r.upsert_identity(fixtures::identity(id).build()).unwrap();
            let mut row = fixtures::stats(id, 20252026, "AAA").build();
            row.position = Position::Center;
            row.totals.gp = 10;
            row.totals.shots = 1000;
            r.upsert_stats(row).unwrap();
        }
        let s = build_attribute_season(
            PlayerId(1),
            Season(20252026),
            SeasonType::Regular,
            r.read_league(Season(20252026), SeasonType::Regular)
                .collect(),
        )
        .unwrap();
        assert_eq!(s.eligible_peers, 25);
        assert_eq!(s.attributes[0].value, Some(1.0));
        assert_eq!(s.attributes[0].percentile, Some(4.0));
    }

    #[test]
    fn goalies_have_separate_workload_and_outcome_attributes() {
        let mut r = repo();
        for id in 101..=110 {
            r.upsert_identity(fixtures::identity(id).build()).unwrap();
            let mut row = fixtures::goalie_mid_playoff_trade_default(id, 20252026).build();
            row.season_type = SeasonType::Regular;
            row.totals.gp = 10;
            r.upsert_stats(row).unwrap();
        }
        let s = build_attribute_season(
            PlayerId(101),
            Season(20252026),
            SeasonType::Regular,
            r.read_league(Season(20252026), SeasonType::Regular)
                .collect(),
        )
        .unwrap();
        assert_eq!(s.group, "Goalies");
        assert_eq!(s.eligible_peers, 10);
        assert_eq!(s.attributes.len(), 3);
        assert_eq!(s.attributes[0].key, "save-outcome");
        assert_eq!(s.attributes[0].band, Some(3));
        assert_eq!(s.attributes[1].value, Some(28.0));
    }
}
