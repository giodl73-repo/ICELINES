use icelines_core::view_model::rinkcraft_finishing::{credible_interval, GammaPosterior};
use serde::Deserialize;
#[derive(Deserialize)]
struct Oracle {
    absolute_tolerance: f64,
    relative_tolerance: f64,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    shape: f64,
    rate: f64,
    probability: f64,
    lower: f64,
    upper: f64,
}
#[test]
fn conditional_gamma_intervals_agree_with_independent_scipy_grid() {
    let oracle: Oracle =
        serde_json::from_str(include_str!("fixtures/rinkcraft_gamma_intervals.json")).unwrap();
    assert_eq!(oracle.cases.len(), 126);
    for c in oracle.cases {
        let p = GammaPosterior {
            shape: c.shape,
            rate: c.rate,
            mean: c.shape / c.rate,
            variance: c.shape / c.rate / c.rate,
        };
        let interval = credible_interval(&p, c.probability).unwrap_or_else(|e| {
            panic!(
                "shape={} rate={} probability={}: {}",
                c.shape, c.rate, c.probability, e
            )
        });
        for (actual, expected) in [(interval.lower, c.lower), (interval.upper, c.upper)] {
            assert!(
                (actual - expected).abs()
                    <= oracle.absolute_tolerance + oracle.relative_tolerance * expected.abs(),
                "shape={} rate={} mass={} actual={} expected={}",
                c.shape,
                c.rate,
                c.probability,
                actual,
                expected
            );
        }
    }
}
