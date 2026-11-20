//! Descriptive adjacent supplied-snapshot differences, never ability trends.
use super::rinkcraft_history::{HistoryRow, HistorySeason};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub from_season: u32,
    pub to_season: u32,
    pub from_games: Option<u32>,
    pub to_games: Option<u32>,
    pub from_minutes: Option<f64>,
    pub to_minutes: Option<f64>,
    pub rows: Vec<Row>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub state: String,
    pub from_value: Option<f64>,
    pub to_value: Option<f64>,
    pub difference: Option<f64>,
    pub from_percentile: Option<f64>,
    pub to_percentile: Option<f64>,
    pub percentile_point_difference: Option<f64>,
    pub note: String,
}

pub fn build(seasons: &[HistorySeason], rows: &[HistoryRow]) -> Vec<Change> {
    let mut ordered = seasons.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|s| s.season);
    ordered.windows(2).map(|pair| {
        let (a,b)=(&pair[0],&pair[1]);
        let changes=rows.iter().map(|row| {
            let left=row.points.iter().find(|p|p.season==a.season).and_then(|p|p.measurement.as_ref());
            let right=row.points.iter().find(|p|p.season==b.season).and_then(|p|p.measurement.as_ref());
            let mut out=Row { key:row.key.clone(),label:row.label.clone(),
                unit:right.or(left).map(|m|m.unit.clone()).unwrap_or_default(),
                state:"unavailable".into(),from_value:left.and_then(|m|m.value),to_value:right.and_then(|m|m.value),
                difference:None,from_percentile:left.and_then(|m|m.percentile),to_percentile:right.and_then(|m|m.percentile),
                percentile_point_difference:None,note:String::new() };
            if a.state!="available" || b.state!="available" {
                out.note="Player row absent in at least one supplied snapshot; no difference or fallback.".into();return out;
            }
            let Some((l,r))=left.zip(right) else {
                out.note="Metric absent in at least one snapshot; no difference.".into();return out;
            };
            if a.group.is_none() || a.situation.is_none() || a.group!=b.group || a.situation!=b.situation
                || l.unit!=r.unit || l.formula!=r.formula || l.scale!=r.scale {
                out.state="incompatible".into();out.from_value=None;out.to_value=None;
                out.note="Position group, situation, unit, formula or scale changed; differences withheld.".into();return out;
            }
            let Some((lv,rv))=l.value.zip(r.value).filter(|(x,y)|x.is_finite()&&y.is_finite()&&(y-x).is_finite()) else {
                out.note="Finite values absent in at least one snapshot; no difference.".into();return out;
            };
            out.state="descriptive".into();out.difference=Some(rv-lv);
            if l.state=="ranked" && r.state=="ranked" {
                out.percentile_point_difference=l.percentile.zip(r.percentile)
                    .filter(|(x,y)|(0.0..=100.0).contains(x)&&(0.0..=100.0).contains(y)).map(|(x,y)|y-x);
            }
            out.note="Later minus earlier recorded value. Annual peer ranks use different populations; neither difference establishes ability change, causes or statistical significance. Provider/model vintage and deployment comparability are not established.".into();
            if out.percentile_point_difference.is_none() {out.note.push_str(" Peer-rank difference unavailable; one or both snapshots are unranked.");}
            out
        }).collect();
        Change {from_season:a.season,to_season:b.season,from_games:a.games,to_games:b.games,
            from_minutes:a.minutes,to_minutes:b.minutes,rows:changes}
    }).collect()
}
