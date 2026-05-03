//! Shared tabular output for CLI report commands.
//!
//! Most reports are columnar — rank, leaders, history, players-by-team,
//! peers, project, etc. Without a shared emitter every command grew its
//! own custom CSV / JSON branch (or none at all). This module gives them
//! one path so:
//!   - every command can opt in to `--csv` / `--json` consistently
//!   - new commands inherit Excel-friendly output for free
//!   - CSV escaping (commas, quotes, newlines) is fixed in one place
//!
//! Usage:
//! ```ignore
//! let headers = vec!["rank", "name", "team", "ppg"];
//! let rows: Vec<Vec<String>> = players.iter().enumerate().map(|(i, p)| vec![
//!     (i + 1).to_string(),
//!     p.full_name.clone(),
//!     p.team.as_str().to_owned(),
//!     format!("{:.3}", p.ppg),
//! ]).collect();
//! Format::resolve(args.csv, args.json).emit(&headers, &rows);
//! ```

use anyhow::{bail, Context};

/// Output format. `Table` prints the human-readable plain-text table;
/// `Csv` is RFC-4180-ish (always quotes fields with commas/quotes/newlines);
/// `Json` is an array of `{header: value}` objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Table,
    Csv,
    Json,
}

impl Format {
    /// Resolve a `--csv`/`--json` flag pair into a Format. Both true
    /// is a usage error and surfaces as `Err`.
    pub fn resolve(csv: bool, json: bool) -> anyhow::Result<Self> {
        match (csv, json) {
            (true, true) => bail!("--csv and --json are mutually exclusive"),
            (true, false) => Ok(Format::Csv),
            (false, true) => Ok(Format::Json),
            (false, false) => Ok(Format::Table),
        }
    }

    /// Emit a header row + data rows in the chosen format. Width is
    /// auto-fitted for Table; CSV escapes commas/quotes/newlines per
    /// RFC 4180; JSON emits an array of objects keyed by header name.
    pub fn emit(self, headers: &[&str], rows: &[Vec<String>]) {
        let s = self.render(headers, rows);
        println!("{s}");
    }

    /// Emit to a file path (used by `--out`) or stdout when path is None.
    pub fn emit_to(
        self,
        headers: &[&str],
        rows: &[Vec<String>],
        out: Option<&std::path::Path>,
    ) -> anyhow::Result<()> {
        let s = self.render(headers, rows);
        match out {
            Some(p) => std::fs::write(p, format!("{s}\n"))
                .with_context(|| format!("writing report to {}", p.display()))?,
            None => println!("{s}"),
        }
        Ok(())
    }

    /// Format the rows to a single String — useful when the caller
    /// wants to post-process or write themselves.
    pub fn render(self, headers: &[&str], rows: &[Vec<String>]) -> String {
        match self {
            Format::Table => render_table(headers, rows),
            Format::Csv => render_csv(headers, rows),
            Format::Json => render_json(headers, rows),
        }
    }
}

// ── Renderers ─────────────────────────────────────────────────────────────────

fn render_csv(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(
        &headers
            .iter()
            .map(|h| escape_csv(h))
            .collect::<Vec<_>>()
            .join(","),
    );
    for row in rows {
        out.push('\n');
        out.push_str(
            &row.iter()
                .map(|c| escape_csv(c))
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    out
}

fn render_json(headers: &[&str], rows: &[Vec<String>]) -> String {
    let array: Vec<serde_json::Map<String, serde_json::Value>> = rows
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (h, v) in headers.iter().zip(row.iter()) {
                obj.insert((*h).to_owned(), value_for_json(v));
            }
            obj
        })
        .collect();
    serde_json::to_string_pretty(&array).unwrap_or_else(|_| "[]".to_owned())
}

fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return String::new();
    }
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.chars().count());
            }
        }
    }
    let render_row = |cols: &[String]| -> String {
        cols.iter()
            .enumerate()
            .map(|(i, c)| {
                let w = widths.get(i).copied().unwrap_or(0);
                format!("{c:<w$}")
            })
            .collect::<Vec<_>>()
            .join("  ")
    };
    let header_strings: Vec<String> = headers.iter().map(|h| (*h).to_owned()).collect();
    let mut out = String::new();
    out.push_str(&render_row(&header_strings));
    let total_width: usize = widths.iter().sum::<usize>() + 2 * widths.len().saturating_sub(1);
    out.push('\n');
    out.push_str(&"─".repeat(total_width.max(1)));
    for row in rows {
        out.push('\n');
        out.push_str(&render_row(row));
    }
    out
}

// ── CSV / JSON helpers ───────────────────────────────────────────────────────

/// RFC-4180 CSV escape: wrap in double-quotes when the field contains
/// a comma, quote, or newline; double up internal quotes.
pub fn escape_csv(s: &str) -> String {
    let needs_quote = s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !needs_quote {
        return s.to_owned();
    }
    let escaped = s.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

/// Try to coerce a stringified cell back to a JSON-typed value so
/// downstream consumers (Excel Power Query, jq, pandas) get numbers
/// instead of strings. Empty / "—" → null.
fn value_for_json(s: &str) -> serde_json::Value {
    let t = s.trim();
    if t.is_empty() || t == "—" || t == "-" {
        return serde_json::Value::Null;
    }
    if let Ok(n) = t.parse::<i64>() {
        return serde_json::Value::from(n);
    }
    if let Ok(n) = t.parse::<f64>() {
        if n.is_finite() {
            return serde_json::json!(n);
        }
    }
    serde_json::Value::String(s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_format_resolve_default_is_table() {
        assert_eq!(Format::resolve(false, false).unwrap(), Format::Table);
    }

    #[test]
    fn l0_format_resolve_csv_flag() {
        assert_eq!(Format::resolve(true, false).unwrap(), Format::Csv);
    }

    #[test]
    fn l0_format_resolve_json_flag() {
        assert_eq!(Format::resolve(false, true).unwrap(), Format::Json);
    }

    #[test]
    fn l0_format_resolve_both_flags_errors() {
        assert!(
            Format::resolve(true, true).is_err(),
            "passing both --csv and --json must be a usage error"
        );
    }

    #[test]
    fn l0_escape_csv_passes_safe_strings_through() {
        assert_eq!(escape_csv("Connor McDavid"), "Connor McDavid");
        assert_eq!(escape_csv("EDM"), "EDM");
        assert_eq!(escape_csv(""), "");
    }

    #[test]
    fn l0_escape_csv_quotes_comma() {
        // Mid-season trade abbrev is "EDM,PIT" — must be wrapped, not split.
        assert_eq!(escape_csv("EDM,PIT"), "\"EDM,PIT\"");
    }

    #[test]
    fn l0_escape_csv_quotes_internal_quote() {
        // Names with apostrophes are fine, but a literal " is rare yet
        // must be doubled per RFC 4180.
        assert_eq!(escape_csv("Said \"Hello\""), "\"Said \"\"Hello\"\"\"");
    }

    #[test]
    fn l0_escape_csv_quotes_newline() {
        assert_eq!(escape_csv("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn l0_value_for_json_keeps_numbers_typed() {
        assert_eq!(value_for_json("42"), serde_json::json!(42));
        assert_eq!(value_for_json("0.925"), serde_json::json!(0.925));
        assert_eq!(value_for_json("-3"), serde_json::json!(-3));
    }

    #[test]
    fn l0_value_for_json_treats_dash_as_null() {
        assert_eq!(value_for_json("—"), serde_json::Value::Null);
        assert_eq!(value_for_json("-"), serde_json::Value::Null);
        assert_eq!(value_for_json(""), serde_json::Value::Null);
    }

    #[test]
    fn l0_value_for_json_keeps_strings_when_not_numeric() {
        assert_eq!(
            value_for_json("EDM"),
            serde_json::Value::String("EDM".to_owned())
        );
        assert_eq!(
            value_for_json("Connor McDavid"),
            serde_json::Value::String("Connor McDavid".to_owned())
        );
    }
}
