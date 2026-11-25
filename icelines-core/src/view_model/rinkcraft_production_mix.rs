//! Goal/assist composition of recorded points, separate from player class or skill.
use super::rinkcraft_stat_scores::Profile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub label: String,
    pub rate: String,
    pub share: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    pub headline: String,
    pub rows: Vec<Row>,
    pub assist_detail: Vec<Row>,
    pub note: String,
}
pub fn build(p: &Profile) -> Option<Record> {
    if !["Forwards", "Defense"].contains(&p.group.as_str())
        || p.games == 0
        || !p.minutes.is_finite()
        || p.minutes <= 0.
        || !p.scope.to_ascii_lowercase().starts_with("all strengths")
    {
        return None;
    }
    let c = p.skater_context.as_ref()?;
    let value = |key: &str| {
        c.values
            .get(key)
            .copied()
            .filter(|v| v.is_finite() && *v >= 0.)
    };
    let (goals, assists) = (value("goals")?, value("assists")?);
    let points = goals + assists;
    if !points.is_finite() {
        return None;
    }
    let row = |label: &str, rate: f64| Row {
        label: label.into(),
        rate: format!("{rate:.2} /60"),
        share: if points > 0. {
            format!("{:.1}% of points", 100. * rate / points)
        } else {
            "No recorded points".into()
        },
    };
    let mut assist_detail = vec![];
    if let Some((a, b)) = value("primary").zip(value("secondary")) {
        let (primary, secondary) = (a * 60. / p.minutes, b * 60. / p.minutes);
        // Only use whole recorded credits whose sum matches this NHL snapshot.
        if a.fract() == 0.
            && b.fract() == 0.
            && primary.is_finite()
            && secondary.is_finite()
            && ((primary + secondary) - assists).abs() <= 1e-8
        {
            assist_detail = vec![
                row("Primary assists", primary),
                row("Secondary assists", secondary),
            ];
        }
    }
    Some(Record {schema:"rinkcraft.production-mix.v1".into(),player_id:p.player_id,season:p.season,
        group:p.group.clone(),games:p.games,minutes:p.minutes,
        headline:if points>0. {format!("{:.1}% goal share",100.*goals/points)} else {"No recorded points".into()},
        rows:vec![row("Goals",goals),row("Assists",assists)],assist_detail,
        note:"All-strength NHL point composition: goals / (goals + assists), and assists / (goals + assists), using matched per-60 rates. Primary and secondary credits expand only when their sum matches these assists; their shares also divide by all points, not just assists. Zero points have no composition percentage. This describes credited production, not all passes, playmaking skill, finishing skill or an assigned role. Position, deployment, teammates and opportunities affect the mix. No ten-GP gate, new class, feat, capability grade or simulation bonus.".into()})
}
pub fn validate(p: &Profile, r: &Record) -> Result<(), String> {
    if build(p).as_ref() != Some(r) {
        Err("Production mix does not match admitted inputs/window".into())
    } else {
        Ok(())
    }
}
