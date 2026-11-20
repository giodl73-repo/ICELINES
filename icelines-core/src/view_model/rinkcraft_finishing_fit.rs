//! Training-only empirical-Bayes prior, ported from the frozen component pilot.
//! Fixed provider xG is an exposure, not a claim of causal finishing talent.
use super::rinkcraft_finishing::Position;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::function::gamma::ln_gamma;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub player_id: u32,
    pub season: u32,
    pub position: Position,
    pub goals: u32,
    pub xg: f64,
    pub games: u32,
    pub minutes: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub training_seasons: Vec<u32>,
    pub minimum_games: u32,
    pub minimum_minutes: f64,
    pub shape_bounds: [f64; 2],
    pub rows: Vec<Row>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregate {
    pub player_id: u32,
    pub goals: u64,
    pub xg: f64,
    pub games: u64,
    pub minutes: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prior {
    pub schema: String,
    pub status: String,
    pub training_seasons: Vec<u32>,
    pub input_sha256: String,
    pub mean: f64,
    pub shape: f64,
    pub rate: f64,
    pub peers: usize,
    pub negative_log_marginal_likelihood: f64,
    pub at_shape_boundary: bool,
    pub optimizer: String,
    pub training: Vec<Aggregate>,
}
fn season(s: u32) -> bool {
    (1900..=2200).contains(&(s / 10000)) && s % 10000 == s / 10000 + 1
}
fn sum(values: impl Iterator<Item = f64>) -> f64 {
    let (mut total, mut correction) = (0., 0.);
    for value in values {
        let adjusted = value - correction;
        let next = total + adjusted;
        correction = (next - total) - adjusted;
        total = next;
    }
    total
}
fn nll(log_shape: f64, mean: f64, rows: &[Aggregate]) -> f64 {
    let a = log_shape.exp();
    let b = a / mean;
    -sum(rows.iter().map(|r| {
        let g = r.goals as f64;
        ln_gamma(g + a) - ln_gamma(a) - ln_gamma(g + 1.) + a * b.ln() + g * r.xg.ln()
            - (g + a) * (b + r.xg).ln()
    }))
}
fn optimize(mean: f64, rows: &[Aggregate], bounds: [f64; 2]) -> Result<(f64, f64), String> {
    let (lo, hi) = (bounds[0].ln(), bounds[1].ln());
    let mut grid = Vec::new();
    for i in 0..=128 {
        let x = lo + (hi - lo) * f64::from(i) / 128.;
        let y = nll(x, mean, rows);
        if !y.is_finite() {
            return Err("nonfinite marginal likelihood".into());
        }
        grid.push((x, y));
    }
    let best = (0..grid.len())
        .min_by(|&a, &b| grid[a].1.total_cmp(&grid[b].1))
        .unwrap();
    let (mut left, mut right) = (grid[best.saturating_sub(1)].0, grid[(best + 1).min(128)].0);
    let ratio = (5_f64.sqrt() - 1.) / 2.;
    let (mut c, mut d) = (
        right - ratio * (right - left),
        left + ratio * (right - left),
    );
    let (mut fc, mut fd) = (nll(c, mean, rows), nll(d, mean, rows));
    for _ in 0..128 {
        if right - left <= 1e-9 {
            break;
        }
        if fc < fd {
            right = d;
            d = c;
            fd = fc;
            c = right - ratio * (right - left);
            fc = nll(c, mean, rows);
        } else {
            left = c;
            c = d;
            fc = fd;
            d = left + ratio * (right - left);
            fd = nll(d, mean, rows);
        }
    }
    let mut choices = vec![
        grid[best],
        (c, fc),
        (d, fd),
        (lo, nll(lo, mean, rows)),
        (hi, nll(hi, mean, rows)),
    ];
    choices.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (x, y) = choices[0];
    if !x.is_finite() || !y.is_finite() {
        return Err("invalid fitted prior".into());
    }
    Ok((x.exp(), y))
}
pub fn fit(input: &Input) -> Result<Prior, String> {
    let years: BTreeSet<_> = input.training_seasons.iter().copied().collect();
    let [lower, upper] = input.shape_bounds;
    if years.is_empty()
        || years.len() != input.training_seasons.len()
        || years.iter().any(|s| !season(*s))
        || input.minimum_games == 0
        || !input.minimum_minutes.is_finite()
        || input.minimum_minutes <= 0.
        || !lower.is_finite()
        || !upper.is_finite()
        || lower <= 0.
        || upper <= lower
    {
        return Err("invalid training epoch or fit settings".into());
    }
    let mut rows = input.rows.clone();
    rows.sort_by_key(|r| (r.player_id, r.season));
    let mut seen = BTreeSet::new();
    let mut pooled: BTreeMap<u32, Aggregate> = BTreeMap::new();
    for r in &rows {
        if r.player_id == 0
            || !years.contains(&r.season)
            || !seen.insert((r.player_id, r.season))
            || !r.xg.is_finite()
            || r.xg < 0.
            || !r.minutes.is_finite()
            || r.minutes < 0.
            || (r.xg == 0. && r.goals > 0)
        {
            return Err("invalid, duplicate or outside-epoch training row".into());
        }
        let p = pooled.entry(r.player_id).or_insert(Aggregate {
            player_id: r.player_id,
            goals: 0,
            xg: 0.,
            games: 0,
            minutes: 0.,
        });
        p.goals += u64::from(r.goals);
        p.games += u64::from(r.games);
        p.xg += r.xg;
        p.minutes += r.minutes;
    }
    if years.iter().any(|y| !rows.iter().any(|r| r.season == *y)) {
        return Err("missing declared training season".into());
    }
    let training: Vec<_> = pooled
        .into_values()
        .filter(|p| {
            p.games >= u64::from(input.minimum_games)
                && p.minutes >= input.minimum_minutes
                && p.xg > 0.
        })
        .collect();
    if training.len() < 3 {
        return Err("at least three eligible positive-xG players required".into());
    }
    let total_goals = sum(training.iter().map(|r| r.goals as f64));
    let total_xg = sum(training.iter().map(|r| r.xg));
    let mean = total_goals / total_xg;
    if !mean.is_finite() || mean <= 0. {
        return Err("positive training goals and exposure required".into());
    }
    let (shape, objective) = optimize(mean, &training, input.shape_bounds)?;
    let rate = shape / mean;
    let canonical = Input {
        training_seasons: years.iter().copied().collect(),
        minimum_games: input.minimum_games,
        minimum_minutes: input.minimum_minutes,
        shape_bounds: input.shape_bounds,
        rows,
    };
    Ok(Prior {
        schema: "icelines.finishing-prior.v1".into(),
        status: "experimental_conditional_model".into(),
        training_seasons: years.into_iter().collect(),
        input_sha256: format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&canonical).map_err(|e| e.to_string())?)
        ),
        mean,
        shape,
        rate,
        peers: training.len(),
        negative_log_marginal_likelihood: objective,
        at_shape_boundary: shape < lower * 1.01 || shape > upper / 1.01,
        optimizer: "bounded log-shape grid bracket and golden-section; compensated likelihood sum"
            .into(),
        training,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            training_seasons: vec![20232024, 20242025],
            minimum_games: 10,
            minimum_minutes: 300.,
            shape_bounds: [0.05, 100000.],
            rows: (1..=12)
                .flat_map(|id| {
                    [20232024, 20242025].map(move |season| Row {
                        player_id: id,
                        season,
                        position: Position::C,
                        goals: id * 2,
                        xg: f64::from(id),
                        games: 20,
                        minutes: 200.,
                    })
                })
                .collect(),
        }
    }
    #[test]
    fn pooled_eligibility_unique_players_and_order() {
        let i = input();
        let a = fit(&i).unwrap();
        assert_eq!(a.peers, 12);
        assert_eq!(a.mean, 2.);
        assert!(a.at_shape_boundary);
        let mut reversed = i.clone();
        reversed.rows.reverse();
        assert_eq!(a.input_sha256, fit(&reversed).unwrap().input_sha256);
    }
    #[test]
    fn future_duplicates_invalid_exposure_and_zero_training_goals_fail() {
        let mut i = input();
        i.rows[0].season = 20252026;
        assert!(fit(&i).is_err());
        let mut i = input();
        i.rows.push(i.rows[0].clone());
        assert!(fit(&i).is_err());
        let mut i = input();
        i.rows[0].xg = 0.;
        assert!(fit(&i).is_err());
        let mut i = input();
        for r in &mut i.rows {
            r.goals = 0;
        }
        assert!(fit(&i).is_err());
    }
    #[test]
    fn conditional_update_uses_rate_and_retains_zero_evidence() {
        use super::super::rinkcraft_finishing::update_multiplier;
        let p = update_multiplier(4., 2., 6, 3.).unwrap();
        assert_eq!((p.shape, p.rate, p.mean), (10., 5., 2.));
        assert_eq!(update_multiplier(4., 2., 0, 0.).unwrap().mean, 2.);
        assert!(update_multiplier(4., 2., 1, 0.).is_err());
    }
}
