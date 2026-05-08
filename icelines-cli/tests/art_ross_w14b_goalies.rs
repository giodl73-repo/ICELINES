//! Phase Art Ross Wave 14b — goalie semantic correctness
//! against bundled data.
//!
//! Wave 14 used `load_player_career_into_repo` per-pid which is
//! skater-only. Goalies aren't pulled into the repo via that path.
//! Wave 14b uses `load_into_repo(season, type, store)` to get the
//! FULL season roster including goalies.
//!
//! ## Data limitation surfaced during this wave
//!
//! The bundled snapshot does NOT include the `/goalie/bios`
//! Tier-1 endpoint. So at load time, every goalie has:
//!   - `position == Position::Goalie` ✓ (verified)
//!   - `identity.bio.birth_country == None`
//!   - `stats.goalie_bios == None`
//!
//! Country / nationality / draft / height filters on goalies
//! therefore have NO source-of-truth in the bundle. The executor
//! handles this correctly (returns false when the field is None,
//! per the fail-closed default), but it means we can't VERIFY
//! `country=USA AND pos=G → Hellebuyck` against bundled data —
//! Hellebuyck's bio is empty in the bundle.
//!
//! What this wave DOES verify:
//!   - `pos=G` correctly identifies goalies in the loaded set
//!   - `pos!=G` and `pos IN (skater-set)` correctly EXCLUDES them
//!   - The query pipeline composes pos × stat filters cleanly
//!     (no crash, no false-positives on missing data)
//!   - `applies_to` semantics: skater-only stats on goalies and
//!     goalie-only stats on skaters return false (silent-pass
//!     for the catalog non-applicability rule)
//!
//! Once the goalie/bios snapshot is bundled (or the runtime
//! loader pulls it on demand), a Wave 14c can extend with the
//! country-typed assertions.

use std::collections::HashSet;

use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::StatsRepository;
use icelines_fetch::snapshot::SnapshotStore;
use icelines_fetch::stats_loader::load_into_repo;
use icelines_query::data_provider::{
    DataProvider, EvalCtx, FetchError, FetchEvent, PlanRequirement,
};
use icelines_query::{parse_query, FilterInput, StrictMode};

/// Active goalies whose presence in bundled current-roster snapshot
/// we rely on.
const GOALIES: &[(u32, &str)] = &[
    (8476945, "Connor Hellebuyck"),
    (8476883, "Andrei Vasilevskiy"),
    (8478048, "Igor Shesterkin"),
    (8477424, "Juuse Saros"),
    (8475683, "Sergei Bobrovsky"),
    (8474593, "Jacob Markstrom"),
    (8476999, "Linus Ullmark"),
    (8475883, "Frederik Andersen"),
];

const NON_GOALIE_PIDS: &[(u32, &str)] = &[
    (8478402, "Connor McDavid"),
    (8480069, "Cale Makar"),
];

const SAMPLE_SEASONS: &[u32] = &[20252026, 20242025, 20232024];

struct NoOpProvider;
impl DataProvider for NoOpProvider {
    fn ensure(
        &self,
        _req: &PlanRequirement,
        _events: &mut dyn FnMut(FetchEvent),
    ) -> Result<(), FetchError> {
        Ok(())
    }
}

fn build_repo() -> (StatsRepository, u32) {
    let store = SnapshotStore::new(SnapshotStore::default_root());
    for season_id in SAMPLE_SEASONS {
        let outcome = match load_into_repo(Season(*season_id), SeasonType::Regular, &store)
        {
            Ok(o) => o,
            Err(_) => continue,
        };
        let has_any = GOALIES.iter().any(|(pid, _)| {
            outcome
                .repo
                .view(PlayerId(*pid), Season(*season_id), SeasonType::Regular)
                .is_some()
        });
        if has_any {
            return (outcome.repo, *season_id);
        }
    }
    panic!("no candidate season had goalie data");
}

fn fixed_today() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 5, 7).unwrap()
}

fn matched_names(
    repo: &StatsRepository,
    season: u32,
    pids: &[(u32, &str)],
    filter: &str,
) -> HashSet<String> {
    let plan = parse_query(FilterInput::Cli(filter.into()))
        .unwrap_or_else(|e| panic!("filter {filter:?} failed: {e:?}"));
    let provider = NoOpProvider;
    let ctx = EvalCtx::new(&provider, StrictMode::Off, false, fixed_today(), season);
    let mut out = HashSet::new();
    for (pid, name) in pids {
        if let Some(view) = repo.view(PlayerId(*pid), Season(season), SeasonType::Regular)
        {
            if plan.root.matches(&view, &ctx) {
                out.insert((*name).to_string());
            }
        }
    }
    out
}

fn all_pids() -> Vec<(u32, &'static str)> {
    let mut v: Vec<(u32, &'static str)> = GOALIES.to_vec();
    v.extend_from_slice(NON_GOALIE_PIDS);
    v
}

fn assert_contains(set: &HashSet<String>, name: &str, filter: &str) {
    assert!(
        set.contains(name),
        "expected {name:?} in result for {filter:?}; got: {set:?}"
    );
}

fn assert_excludes(set: &HashSet<String>, name: &str, filter: &str) {
    assert!(
        !set.contains(name),
        "expected {name:?} EXCLUDED from {filter:?}; got: {set:?}"
    );
}

fn loaded_goalies(repo: &StatsRepository, season: u32) -> Vec<&'static str> {
    GOALIES
        .iter()
        .filter(|(pid, _)| {
            repo.view(PlayerId(*pid), Season(season), SeasonType::Regular)
                .is_some()
        })
        .map(|(_, name)| *name)
        .collect()
}

// ── pos=G filter actually identifies goalies ──────────────────

#[test]
fn w14b_pos_g_includes_every_loaded_goalie() {
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos=G";
    let m = matched_names(&repo, season, &pids, f);
    for name in loaded_goalies(&repo, season) {
        assert_contains(&m, name, f);
    }
    // Skaters NEVER match.
    assert_excludes(&m, "Connor McDavid", f);
    assert_excludes(&m, "Cale Makar", f);
}

#[test]
fn w14b_pos_ne_g_excludes_every_loaded_goalie() {
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos!=G";
    let m = matched_names(&repo, season, &pids, f);
    for name in loaded_goalies(&repo, season) {
        assert_excludes(&m, name, f);
    }
}

#[test]
fn w14b_pos_in_skater_set_excludes_every_loaded_goalie() {
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos IN (C, LW, RW, D)";
    let m = matched_names(&repo, season, &pids, f);
    for name in loaded_goalies(&repo, season) {
        assert_excludes(&m, name, f);
    }
}

#[test]
fn w14b_pos_in_set_with_g_includes_goalies() {
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos IN (G, D)"; // goalies AND defensemen
    let m = matched_names(&repo, season, &pids, f);
    for name in loaded_goalies(&repo, season) {
        assert_contains(&m, name, f);
    }
    // Forwards excluded.
    assert_excludes(&m, "Connor McDavid", f);
}

// ── Stat applies_to non-applicability ─────────────────────────

#[test]
fn w14b_skater_stat_atom_passes_for_goalies_silently() {
    // Per the catalog `applies_to` rule, a skater-only stat
    // (e.g. faceoff-win-pct) on a goalie evaluates as a no-op
    // (returns true to NOT exclude the goalie via missing-data).
    // This is intentional legacy behavior — verify the executor
    // honors it.
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "faceoff-win-pct>=0";
    let m = matched_names(&repo, season, &pids, f);
    // Goalies should pass through (atom is no-op for them).
    for name in loaded_goalies(&repo, season) {
        assert_contains(&m, name, f);
    }
}

#[test]
fn w14b_goalie_stat_atom_no_op_for_skaters() {
    // Same in the other direction: a goalie-only stat on a
    // skater is a no-op.
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "save-pct>=0";
    let m = matched_names(&repo, season, &pids, f);
    // McDavid is a skater — `save-pct` doesn't apply, but
    // applies_to-no-op means the atom doesn't filter him out.
    if repo
        .view(PlayerId(8478402), Season(season), SeasonType::Regular)
        .is_some()
    {
        assert_contains(&m, "Connor McDavid", f);
    }
}

// ── Compound: pos=G with stat predicate ───────────────────────

#[test]
fn w14b_compound_pos_g_with_save_pct_applies() {
    // `pos=G AND save-pct>=0` — pos filter restricts to goalies,
    // save-pct atom is no-op on its loaded value (whatever it
    // is, ≥ 0). Should match every loaded goalie.
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos=G AND save-pct>=0";
    let m = matched_names(&repo, season, &pids, f);
    for name in loaded_goalies(&repo, season) {
        assert_contains(&m, name, f);
    }
    assert_excludes(&m, "Connor McDavid", f);
}

#[test]
fn w14b_compound_pos_g_with_huge_save_pct_threshold() {
    // `pos=G AND save-pct>=2.0` — impossible; even if save-pct
    // is loaded, no goalie has SV%≥200%. So the result is empty.
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos=G AND save-pct>=2.0";
    let m = matched_names(&repo, season, &pids, f);
    assert!(
        m.is_empty(),
        "no goalie should have SV% ≥ 2.0; got: {m:?}"
    );
}

// ── pos negation across boolean composition ──────────────────

#[test]
fn w14b_demorgan_holds_for_pos_g() {
    // NOT (pos=G) === pos!=G — verify over real data.
    let (repo, season) = build_repo();
    let pids = all_pids();
    let lhs = matched_names(&repo, season, &pids, "NOT pos=G");
    let rhs = matched_names(&repo, season, &pids, "pos!=G");
    assert_eq!(lhs, rhs, "NOT pos=G must equal pos!=G over real data");
}

#[test]
fn w14b_or_pos_g_or_d() {
    // pos=G OR pos=D — every loaded goalie + Makar.
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos=G OR pos=D";
    let m = matched_names(&repo, season, &pids, f);
    for name in loaded_goalies(&repo, season) {
        assert_contains(&m, name, f);
    }
    if repo
        .view(PlayerId(8480069), Season(season), SeasonType::Regular)
        .is_some()
    {
        assert_contains(&m, "Cale Makar", f);
    }
    assert_excludes(&m, "Connor McDavid", f); // C, not G or D
}

// ── Bundle-data-limitation tests ──────────────────────────────

#[test]
fn w14b_country_filter_on_goalies_returns_empty_due_to_missing_bios() {
    // Per the comment at the top of this file: bundled goalie
    // bios are missing, so country atoms on goalies always
    // evaluate to false. This test PINS that behavior so a
    // future change that populates goalie bios will fail this
    // test — and prompt updating.
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "pos=G AND country=USA";
    let m = matched_names(&repo, season, &pids, f);
    // Today: empty (no goalies have country populated). When
    // the goalie/bios snapshot ships, this should include
    // Hellebuyck.
    assert!(
        m.is_empty(),
        "country on goalies is currently unpopulated; got: {m:?}\n\
         If this test fails, goalie bios are now bundled — \
         update Wave 14b to assert real country matches."
    );
}

#[test]
fn w14b_country_filter_on_skaters_works_as_expected() {
    // Sanity: country filter still works on skaters in the
    // load_into_repo path (they DO have skater/bios data).
    let (repo, season) = build_repo();
    let pids = all_pids();
    let f = "country=CAN";
    let m = matched_names(&repo, season, &pids, f);
    if repo
        .view(PlayerId(8478402), Season(season), SeasonType::Regular)
        .is_some()
    {
        assert_contains(&m, "Connor McDavid", f);
    }
    if repo
        .view(PlayerId(8480069), Season(season), SeasonType::Regular)
        .is_some()
    {
        assert_contains(&m, "Cale Makar", f);
    }
}

// ── Loader sanity ────────────────────────────────────────────

#[test]
fn w14b_at_least_three_goalies_load_from_bundle() {
    let (repo, season) = build_repo();
    let count = loaded_goalies(&repo, season).len();
    assert!(
        count >= 3,
        "expected ≥3 goalies to load; got {count} from season {season}"
    );
}

#[test]
fn w14b_chosen_season_is_recent() {
    let (_repo, season) = build_repo();
    assert!(season >= 20232024, "should pick recent season; got {season}");
}
