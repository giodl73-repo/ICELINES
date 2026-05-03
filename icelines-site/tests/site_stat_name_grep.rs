//! Phase Lindsay L.5b — SI-03 grep fence on site source files.
//!
//! Scans every `.rs` source file under `icelines-site/src/` for
//! canonical stat name string literals (`Goals`, `Assists`, `Points`,
//! `Hits`, `Blocks`, `Saves`, `GAA`). Any hit outside the allowlist
//! at `icelines-site/.stat-name-allowlist` fails the test.
//!
//! The intent: site templates must route stat names through
//! `StatId::label()` or `StatId::short_label()` (catalog dispatch).
//! Hardcoded canonical names are the smell — when the catalog renames
//! a stat, the site shouldn't drift.
//!
//! Allowlist format: one regex pattern per line; comments start with
//! `#`. A hit matches the allowlist if any pattern's regex matches the
//! raw line (whole-line match). Add new entries with a justifying
//! comment.

use regex::Regex;
use std::path::PathBuf;

fn site_src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn allowlist_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".stat-name-allowlist")
}

fn load_allowlist() -> Vec<Regex> {
    let raw = std::fs::read_to_string(allowlist_path())
        .expect("missing allowlist file at icelines-site/.stat-name-allowlist");
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|pat| {
            Regex::new(pat).unwrap_or_else(|e| {
                panic!("invalid allowlist regex `{pat}`: {e}")
            })
        })
        .collect()
}

fn enumerate_rs_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).expect("read site src dir");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(enumerate_rs_files(&path));
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}

/// SI-03: every site `.rs` source file is free of hardcoded canonical
/// stat name strings outside the allowlist. New hits force migration to
/// `StatId::label()` / `short_label()` or an explicit allowlist entry.
#[test]
fn l1_lindsay_l5b_site_source_no_hardcoded_stat_names() {
    // Word-boundary match so `Goals` matches but `goalsForPct` doesn't.
    let pattern = Regex::new(r"\b(Goals|Assists|Points|Hits|Blocks|Saves|GAA)\b")
        .expect("regex compiles");
    let allowlist = load_allowlist();
    let files = enumerate_rs_files(&site_src_dir());
    assert!(
        !files.is_empty(),
        "no .rs files found under icelines-site/src — test setup broken"
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        for (lineno, line) in content.lines().enumerate() {
            let trimmed = line.trim_start();
            // Skip comment lines — comments aren't rendered. Doc comments
            // (`///`) and inner doc comments (`//!`) are also skipped.
            if trimmed.starts_with("//") {
                continue;
            }
            if !pattern.is_match(line) {
                continue;
            }
            // Allowlist match: any pattern whose regex matches the line.
            if allowlist.iter().any(|re| re.is_match(line)) {
                continue;
            }
            violations.push(format!(
                "{}:{}: {}",
                file.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(file)
                    .display(),
                lineno + 1,
                line.trim()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "SI-03 violation: {} hardcoded stat name(s) found in site source. \
         Either route through `StatId::label()` / `short_label()` (catalog \
         dispatch) or add an entry to icelines-site/.stat-name-allowlist with \
         a justifying comment.\n\nViolations:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

/// SI-03 (gap-fill) — second pass scanning *rendered* markdown output
/// at `docs/teams/*.md` + `docs/index.md`. The source-file scan above
/// catches hardcoded string literals; this pass catches stat names that
/// reach the page via Tera/format-string interpolation but aren't a
/// literal in the source.
///
/// Same allowlist file. The set of files to scan comes from the workspace
/// `docs/` dir at test time. If the directory doesn't exist (clean
/// checkout that hasn't run `icelines build` yet), the test passes
/// vacuously — no false positives.
#[test]
fn l1_lindsay_l5b_rendered_markdown_no_hardcoded_stat_names() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let docs_dir = workspace.join("docs");
    if !docs_dir.exists() {
        // Clean checkout — nothing rendered yet. Vacuous pass; the
        // source-scan test still gates source-side violations.
        return;
    }

    let pattern = Regex::new(r"\b(Goals|Assists|Points|Hits|Blocks|Saves|GAA)\b")
        .expect("regex compiles");
    let allowlist = load_allowlist();

    // Collect target files: docs/index.md + docs/teams/*.md.
    let mut files: Vec<PathBuf> = Vec::new();
    let index = docs_dir.join("index.md");
    if index.exists() {
        files.push(index);
    }
    let teams_dir = docs_dir.join("teams");
    if teams_dir.is_dir() {
        for entry in std::fs::read_dir(&teams_dir).expect("read teams dir").flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(p);
            }
        }
    }

    if files.is_empty() {
        return;  // No rendered output to scan.
    }

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        for (lineno, line) in content.lines().enumerate() {
            // Skip HTML comments — they don't render visibly.
            let trimmed = line.trim_start();
            if trimmed.starts_with("<!--") {
                continue;
            }
            if !pattern.is_match(line) {
                continue;
            }
            if allowlist.iter().any(|re| re.is_match(line)) {
                continue;
            }
            violations.push(format!(
                "{}:{}: {}",
                file.strip_prefix(workspace).unwrap_or(file).display(),
                lineno + 1,
                line.trim()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "SI-03 (rendered-output): {} hardcoded stat name(s) found in \
         generated docs/. Stat names that reach the page must come from \
         `StatId::label()` / `short_label()`. If a hit is intentional \
         (e.g. team name like `Bruins`), add an allowlist entry.\n\n\
         Violations:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}
