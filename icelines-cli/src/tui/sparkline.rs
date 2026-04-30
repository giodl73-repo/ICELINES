//! Native ASCII sparklines rendered with Unicode block characters.
//!
//! Replaces the proof_lib runtime dependency we briefly carried in
//! Phase 8j. proof's `proof:chart` directive doesn't compose inside
//! `proof:region` bodies (issue filed at design/proof-bug-report.md),
//! and a sparkline is small enough that pulling in a 5MB+ dependency
//! to render it never paid off.
//!
//! This module is ~50 lines, has zero new dependencies, and gives the
//! dashboard panel full control over the rendered output.

/// Unicode block characters from lowest (`▁`) to highest (`█`) — eight
/// levels, one per byte of a sparkline column.
const BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Render a sparkline from `values` clamped/scaled to `width` columns.
/// See `columns` for behaviour notes — this is just a string convenience.
pub fn render(values: &[f64], width: usize) -> String {
    columns(values, width).into_iter().map(|(c, _)| c).collect()
}

/// Render a sparkline as `(block, bucket_value)` pairs — one entry per
/// column. The bucket value is the (averaged) input that maps to that
/// column, so callers can colour each block based on its data point.
///
/// Behaviour:
/// * `values.is_empty()` or `width == 0` → empty Vec.
/// * `width >= values.len()` → one column per value (left-aligned).
/// * `width <  values.len()` → bin into `width` buckets, average each.
/// * All values equal → middle band (`▄`) so the line still draws.
pub fn columns(values: &[f64], width: usize) -> Vec<(char, f64)> {
    if values.is_empty() || width == 0 {
        return Vec::new();
    }
    let cols = if width >= values.len() { values.len() } else { width };
    let bucketed = bucket(values, cols);

    let (min, max) = bucketed.iter()
        .copied()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    if !min.is_finite() || !max.is_finite() {
        return Vec::new();
    }
    let span = max - min;
    if span == 0.0 {
        // All-equal series → middle band so the line still draws.
        let mid = BLOCKS[BLOCKS.len() / 2 - 1];
        return bucketed.into_iter().map(|v| (mid, v)).collect();
    }
    bucketed.into_iter()
        .map(|v| {
            let normalized = (v - min) / span;          // 0.0 ..= 1.0
            let idx = (normalized * (BLOCKS.len() - 1) as f64).round() as usize;
            (BLOCKS[idx.min(BLOCKS.len() - 1)], v)
        })
        .collect()
}

/// Average input values into exactly `cols` buckets. When `values.len()
/// <= cols`, each value becomes its own bucket (no averaging). Used by
/// `render` so callers don't need to pre-bin their data.
fn bucket(values: &[f64], cols: usize) -> Vec<f64> {
    if values.len() <= cols {
        return values.to_vec();
    }
    let mut out = Vec::with_capacity(cols);
    let step = values.len() as f64 / cols as f64;
    for i in 0..cols {
        let start = (i as f64 * step) as usize;
        let end = (((i + 1) as f64 * step) as usize).min(values.len());
        let slice = &values[start..end.max(start + 1)];
        let avg = slice.iter().sum::<f64>() / slice.len() as f64;
        out.push(avg);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_render_empty_input_returns_empty_string() {
        assert_eq!(render(&[], 10), "");
        assert_eq!(render(&[1.0, 2.0], 0), "");
    }

    #[test]
    fn l0_render_single_value_returns_one_block() {
        let s = render(&[42.0], 10);
        assert_eq!(s.chars().count(), 1, "one input → one column");
        // All-equal → middle-band block (▄).
        assert!(s.contains('▄'), "single value should be middle-band, got {s:?}");
    }

    #[test]
    fn l0_render_all_equal_uses_middle_band() {
        let s = render(&[5.0, 5.0, 5.0, 5.0], 4);
        assert!(s.chars().all(|c| c == '▄'),
            "constant series must render flat middle band, got {s:?}");
    }

    #[test]
    fn l0_render_increasing_walks_low_to_high() {
        // 5 increasing values, 5 columns → first should be lowest block,
        // last should be highest. Middle values progress upward.
        let s: Vec<char> = render(&[1.0, 2.0, 3.0, 4.0, 5.0], 5)
            .chars().collect();
        assert_eq!(s.len(), 5);
        assert_eq!(s[0], '▁', "min input → lowest block, got {s:?}");
        assert_eq!(s[4], '█', "max input → highest block, got {s:?}");
        // Strictly non-decreasing block indices on a strictly-increasing
        // input — the bin indices map to BLOCKS in the same order.
        let idxs: Vec<usize> = s.iter()
            .map(|c| BLOCKS.iter().position(|b| b == c).unwrap())
            .collect();
        assert!(idxs.windows(2).all(|w| w[0] <= w[1]),
            "increasing input → non-decreasing blocks, got {idxs:?}");
    }

    #[test]
    fn l0_render_buckets_when_input_longer_than_width() {
        // 10 values into 5 cols → averaging halves of each pair.
        let s = render(&[1.0, 1.0, 5.0, 5.0, 9.0, 9.0, 5.0, 5.0, 1.0, 1.0], 5);
        assert_eq!(s.chars().count(), 5,
            "must hit requested width when over-binning, got {s:?}");
        // Peak in the middle, low on the edges → first/last lower than middle.
        let chars: Vec<char> = s.chars().collect();
        let pos = |c: char| BLOCKS.iter().position(|b| *b == c).unwrap();
        assert!(pos(chars[0]) < pos(chars[2]),
            "valley on left should be lower than peak in middle, got {s:?}");
        assert!(pos(chars[4]) < pos(chars[2]),
            "valley on right should be lower than peak in middle, got {s:?}");
    }

    #[test]
    fn l0_render_handles_negative_values() {
        // +/- swing → range shifts but blocks still scale 0..7.
        let s = render(&[-3.0, 0.0, 3.0], 3);
        let chars: Vec<char> = s.chars().collect();
        assert_eq!(chars[0], '▁', "min should be lowest, got {s:?}");
        assert_eq!(chars[2], '█', "max should be highest, got {s:?}");
    }

    #[test]
    fn l0_render_known_mcdavid_goals_trend_shape() {
        // Real series: 44, 64, 32, 26, 48 → up, peak, down, valley, recover.
        // Block indices should peak at idx 1 and bottom at idx 3.
        let s = render(&[44.0, 64.0, 32.0, 26.0, 48.0], 5);
        let chars: Vec<char> = s.chars().collect();
        let pos = |c: char| BLOCKS.iter().position(|b| *b == c).unwrap();
        assert_eq!(pos(chars[1]), BLOCKS.len() - 1,
            "64 (max) should be ▇█, got {s:?}");
        assert_eq!(pos(chars[3]), 0,
            "26 (min) should be ▁, got {s:?}");
        // Recover: idx 4 (48) should be higher than idx 3 (26).
        assert!(pos(chars[4]) > pos(chars[3]),
            "recovery should be visible, got {s:?}");
    }

    #[test]
    fn l0_render_width_clamps_to_input_when_smaller_window() {
        // Width 100, only 5 inputs → output is 5 chars (one per value).
        let s = render(&[1.0, 2.0, 3.0, 4.0, 5.0], 100);
        assert_eq!(s.chars().count(), 5);
    }

    #[test]
    fn l0_columns_returns_per_column_value_pairs() {
        // The bucket value for each column should round-trip the input
        // when input.len() <= width (no averaging).
        let cols = columns(&[1.0, 2.0, 3.0], 5);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].1, 1.0);
        assert_eq!(cols[1].1, 2.0);
        assert_eq!(cols[2].1, 3.0);
        // Block chars match the render() output for the same input.
        let blocks: String = cols.iter().map(|(c, _)| *c).collect();
        assert_eq!(blocks, render(&[1.0, 2.0, 3.0], 5));
    }

    #[test]
    fn l0_columns_returns_bucketed_averages() {
        // 6 inputs into 3 cols → each column averages 2 inputs.
        let cols = columns(&[10.0, 20.0, 100.0, 100.0, 0.0, 0.0], 3);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].1, 15.0);
        assert_eq!(cols[1].1, 100.0);
        assert_eq!(cols[2].1, 0.0);
    }
}
