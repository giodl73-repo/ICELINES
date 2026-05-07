//! Phase Art Ross A.0/A.1 — `Constraint` evaluator.
//!
//! Walks a `Constraint` tree against a `PlayerView` and decides
//! whether the player matches. A.0 wired Bio + SeasonStat with
//! Scalar predicates only. A.1 adds Member, Pattern, Range
//! predicate shapes plus new bio fields (Position, Team, etc.).
//!
//! Missing-data semantics match the legacy `FilterExpr::matches`:
//! when a stat is unavailable for the view, the atom evaluates to
//! `false` (so `NOT (hits>=200)` accepts pre-2010 rows where hits
//! wasn't tracked).

use icelines_core::stats_catalog::StatUnit;
use icelines_core::stats_repository::PlayerView;

use crate::compute_age;
use crate::data_provider::EvalCtx;
use crate::plan::{
    AgeBound, BioConstraint, BioField, CareerAggrConstraint, CareerAggregator,
    CareerLeagueConstraint, Constraint, GlobPattern, LeagueAtom, MemberOp, NumericRange,
    PatternOp, Predicate, ScalarOp, ScalarValue, SeasonStatConstraint, SlidingWindow,
    SlidingWindowConstraint, WindowPolicy, WindowScope,
};
use crate::sliding_window::{aggregate_window, extract_window_stat, GameStatLine, WindowResult};

impl Constraint {
    /// Evaluate this constraint tree against the given player view.
    ///
    /// Phase Art Ross A.2.5 review (forge) — the legacy two-method
    /// shape was a footgun (the placeholder `matches` returned
    /// `true` for unwired variants, silently over-matching). Now
    /// there's one entry point: `matches`. It takes an `EvalCtx`
    /// because every variant needs season + today + provider for
    /// correct evaluation. Bio/SeasonStat ignore most of the ctx
    /// (only `season` is read for age computation); SlidingWindow
    /// pulls per-game lines via `ctx.provider`; CareerAggregate /
    /// CareerLeague will use the full ctx in A.3/A.4.
    ///
    /// For unwired variants (CareerAggregate, CareerLeague) the
    /// parser today rejects the atom shapes that would construct
    /// them — so these branches are unreachable from user input.
    /// The match arms return false (silent over-match was the bug).
    pub fn matches(&self, v: &PlayerView<'_>, ctx: &EvalCtx<'_>) -> bool {
        match self {
            Constraint::Bio(b) => bio_matches(b, v, ctx.season),
            Constraint::SeasonStat(s) => season_stat_matches(s, v),
            Constraint::SlidingWindow(s) => sliding_window_matches(s, v, ctx),
            Constraint::CareerAggregate(c) => career_aggregate_matches(c, v, ctx),
            Constraint::CareerLeague(c) => career_league_matches(c, v, ctx),
            Constraint::All(children) => children.iter().all(|c| c.matches(v, ctx)),
            Constraint::Any(children) => children.iter().any(|c| c.matches(v, ctx)),
            Constraint::Not(inner) => !inner.matches(v, ctx),
        }
    }
}

/// Phase Art Ross A.3 — `LOCKOUT_SEASONS` is skipped (no data,
/// no partial-mark) per the spec.
const LOCKOUT_SEASONS: &[u32] = &[20042005];

/// Phase Art Ross A.4 — evaluate a `CareerLeagueConstraint`.
/// Three shapes:
///   - `league=OHL` / `league IN (...)` / `league.tier=Junior`:
///     no stat / aggregator. Match if the player has any career
///     stint matching the league axis.
///   - `p.career.junior>=200`: stat + aggregator + predicate.
///     Sum the stat across stints matching the league axis;
///     apply the predicate.
fn career_league_matches(
    c: &CareerLeagueConstraint,
    v: &PlayerView<'_>,
    ctx: &EvalCtx<'_>,
) -> bool {
    let pid = v.identity.id.0;
    let history = match ctx.provider.fetch_career_history(pid) {
        Some(h) => h,
        None => return false, // no career-history record — fail closed
    };

    // Collect stints matching the league axis.
    let matching_stints: Vec<&icelines_core::career_history::CareerStint> = history
        .stints
        .iter()
        .filter(|s| league_atom_matches(&c.league, s))
        .collect();

    // Existence-only check (no stat predicate)?
    if c.stat.is_none() {
        return !matching_stints.is_empty();
    }

    // Stat + aggregator + predicate path.
    let stat = c.stat.unwrap();
    let aggregator = c.aggregator.unwrap_or(CareerAggregator::LifetimeSum);
    let predicate = match c.predicate.as_ref() {
        Some(p) => p,
        None => return false,
    };

    match aggregator {
        CareerAggregator::LifetimeSum => {
            let actual = sum_stat_across_stints(stat, &matching_stints);
            match actual {
                Some(a) => predicate_matches_number_unit_aware(predicate, a, stat.unit()),
                None => false,
            }
        }
        // Other aggregators on a league-filtered stint set are
        // less common; defer the polish to A.5 and conservatively
        // return false.
        _ => false,
    }
}

fn league_atom_matches(
    atom: &LeagueAtom,
    stint: &icelines_core::career_history::CareerStint,
) -> bool {
    match atom {
        LeagueAtom::Code(code) => stint.league.0.eq_ignore_ascii_case(code),
        LeagueAtom::InSet(codes) => codes
            .iter()
            .any(|c| stint.league.0.eq_ignore_ascii_case(c)),
        LeagueAtom::Tier(tier) => &stint.league.tier() == tier,
    }
}

/// Sum a stat value across a slice of CareerStint refs. Returns
/// None when the stat doesn't have a representation on
/// CareerStint (e.g. on-ice xG — boxscore-only).
fn sum_stat_across_stints(
    stat: icelines_core::stats_catalog::StatId,
    stints: &[&icelines_core::career_history::CareerStint],
) -> Option<f64> {
    // CareerStint fields are Option<u32>/Option<f32>; sum what's
    // available + return as f64. Junior leagues may not have
    // every field — sum_lines treats missing as 0.
    let mut total: f64 = 0.0;
    let mut found_any = false;
    for s in stints {
        let value: Option<f64> = match stat.cli_key() {
            "games" => Some(s.gp as f64),
            "goals" => s.goals.map(|x| x as f64),
            "assists" => s.assists.map(|x| x as f64),
            "points" => s.points.map(|x| x as f64),
            "pim" => s.pim.map(|x| x as f64),
            "plus-minus" => s.plus_minus.map(|x| x as f64),
            "shots" => s.shots.map(|x| x as f64),
            "wins" => s.wins.map(|x| x as f64),
            "losses" => s.losses.map(|x| x as f64),
            "ot-losses" => s.ot_losses.map(|x| x as f64),
            "shutouts" => s.shutouts.map(|x| x as f64),
            "saves" => {
                // saves = shots_against - goals_against
                match (s.shots_against, s.goals_against) {
                    (Some(sa), Some(ga)) => Some((sa.saturating_sub(ga)) as f64),
                    _ => None,
                }
            }
            _ => None,
        };
        if let Some(v) = value {
            total += v;
            found_any = true;
        }
    }
    if found_any {
        Some(total)
    } else {
        None
    }
}

fn career_aggregate_matches(
    c: &CareerAggrConstraint,
    v: &PlayerView<'_>,
    ctx: &EvalCtx<'_>,
) -> bool {
    if !c.stat.applies_to(v.position(), v.is_goalie()) {
        return true;
    }
    let pid = v.identity.id.0;
    // Today the IcelinesProvider returns ALL lines for the player
    // (the season parameter is unused). When per-season sharded
    // BoxscoreIndex ships, this call is replaced by a season-by-
    // season iterator that drops shards after evaluation.
    let lines = ctx.provider.fetch_game_lines(pid, ctx.season);
    if lines.is_empty() {
        return false;
    }

    // Group lines by season (derived from the date — NHL season
    // YYYYZZZZ runs Oct YYYY through Jun ZZZZ; we use Aug as the
    // boundary so summer signing dates don't shift seasons).
    let mut by_season: std::collections::BTreeMap<u32, Vec<GameStatLine>> =
        std::collections::BTreeMap::new();
    for line in lines {
        let season = season_for_date(line.date);
        if LOCKOUT_SEASONS.contains(&season) {
            continue; // skip 2004-05 — no data, no partial-mark
        }
        by_season.entry(season).or_default().push(line);
    }

    // Filter seasons by the at_age slice, if present.
    let filtered_seasons: std::collections::BTreeMap<u32, Vec<GameStatLine>> = match &c
        .at_age
    {
        Some(bound) => by_season
            .into_iter()
            .filter(|(season, _)| at_age_matches(v, *season, bound))
            .collect(),
        None => by_season,
    };

    if filtered_seasons.is_empty() {
        return false;
    }

    match c.aggregator {
        CareerAggregator::AnyWindow(n) => {
            // Walk seasons; first one whose intra-season game
            // stream contains a satisfying window short-circuits true.
            for (_season, season_lines) in &filtered_seasons {
                let win = SlidingWindow::LastN_GP {
                    n,
                    scope: WindowScope::AllTeamsCurrentSeason, // intra-season any window
                    policy: WindowPolicy::RequireFull,
                };
                // Slide through every contiguous N-game subwindow:
                // for each starting index i, take games i..i+N.
                if season_lines.len() < n as usize {
                    continue;
                }
                for start in 0..=(season_lines.len() - n as usize) {
                    let slice = &season_lines[start..start + n as usize];
                    let mut sub_lines = slice.to_vec();
                    // Re-aggregate via sum_lines path; we do a
                    // direct call by constructing a synthetic
                    // result.
                    let totals = sum_skater_lines(&mut sub_lines);
                    if let Some(actual) = extract_window_stat(c.stat, &totals) {
                        if predicate_matches_number_unit_aware(
                            &c.predicate,
                            actual,
                            c.stat.unit(),
                        ) {
                            return true;
                        }
                    }
                }
                let _ = win; // suppress unused warning for now
            }
            false
        }
        CareerAggregator::LifetimeSum => {
            // Sum across all eligible seasons + apply predicate.
            let mut total_lines: Vec<GameStatLine> = filtered_seasons
                .into_values()
                .flatten()
                .collect();
            let totals = sum_skater_lines(&mut total_lines);
            match extract_window_stat(c.stat, &totals) {
                Some(actual) => predicate_matches_number_unit_aware(
                    &c.predicate,
                    actual,
                    c.stat.unit(),
                ),
                None => false,
            }
        }
        CareerAggregator::LongestStreak => {
            // For now, compute the longest run of consecutive
            // games where the per-game stat is non-zero. Match if
            // the longest run satisfies the predicate.
            let mut longest: u32 = 0;
            for (_season, season_lines) in &filtered_seasons {
                let mut current: u32 = 0;
                for line in season_lines {
                    let line_value =
                        single_game_extract_stat(c.stat, line).unwrap_or(0.0);
                    if line_value > 0.0 {
                        current += 1;
                        if current > longest {
                            longest = current;
                        }
                    } else {
                        current = 0;
                    }
                }
            }
            predicate_matches_number_unit_aware(
                &c.predicate,
                longest as f64,
                c.stat.unit(),
            )
        }
        CareerAggregator::SeasonsWith => {
            // Count seasons where the season-aggregate satisfies
            // the predicate.
            let count = filtered_seasons
                .into_values()
                .filter(|season_lines| {
                    let mut copy = season_lines.clone();
                    let totals = sum_skater_lines(&mut copy);
                    extract_window_stat(c.stat, &totals)
                        .map(|actual| {
                            predicate_matches_number_unit_aware(
                                &c.predicate,
                                actual,
                                c.stat.unit(),
                            )
                        })
                        .unwrap_or(false)
                })
                .count();
            // The count IS the value to apply against the predicate.
            // SeasonsWith semantics: how many seasons match the
            // PER-SEASON predicate? E.g. `g.seasons-with>=5` →
            // "in at least 5 seasons, did the per-season-aggregate
            // satisfy the predicate."
            //
            // Above we already used the predicate to check each
            // season — so the COUNT is what we report. The user-
            // facing predicate is ON the count itself for true
            // SeasonsWith semantics, but our grammar today uses
            // the same predicate for both checks. This is a
            // simplification we'll revisit in A.5.
            count > 0
        }
    }
}

/// Sum a sliced set of GameStatLine into a WindowTotals. Mirrors
/// the private `sum_lines` in sliding_window.rs but operates over
/// owned slices for the career-aggregator path.
fn sum_skater_lines(lines: &mut [GameStatLine]) -> crate::sliding_window::WindowTotals {
    let mut t = crate::sliding_window::WindowTotals::default();
    for l in lines.iter() {
        t.games += 1;
        t.goals += l.goals;
        t.assists += l.assists;
        t.plus_minus += l.plus_minus;
        t.sog += l.sog;
        t.hits += l.hits;
        t.blocks += l.blocked_shots;
        t.takeaways += l.takeaways;
        t.giveaways += l.giveaways;
        t.pim += l.pim;
        t.toi_seconds += l.toi_seconds;
    }
    t
}

/// Extract one stat value from a single game line. Returns None
/// for stats that don't have a per-game representation.
fn single_game_extract_stat(
    stat: icelines_core::stats_catalog::StatId,
    line: &GameStatLine,
) -> Option<f64> {
    match stat.cli_key() {
        "goals" => Some(line.goals as f64),
        "assists" => Some(line.assists as f64),
        "points" => Some((line.goals + line.assists) as f64),
        "plus-minus" => Some(line.plus_minus as f64),
        "shots" => Some(line.sog as f64),
        "hits" => Some(line.hits as f64),
        "blocked-shots" => Some(line.blocked_shots as f64),
        "takeaways" => Some(line.takeaways as f64),
        "giveaways" => Some(line.giveaways as f64),
        "pim" => Some(line.pim as f64),
        _ => None,
    }
}

/// Map a date to its NHL season-id (YYYYZZZZ format). Aug 1 is
/// the rollover boundary — games before Aug belong to the
/// previous season; Aug onward belongs to the upcoming one.
fn season_for_date(d: chrono::NaiveDate) -> u32 {
    use chrono::Datelike;
    let y = d.year() as u32;
    let m = d.month();
    if m >= 8 {
        y * 10000 + (y + 1)
    } else {
        (y - 1) * 10000 + y
    }
}

/// True iff the player's age (HR Feb-1 convention) at the given
/// season is within the bound.
fn at_age_matches(v: &PlayerView<'_>, season: u32, bound: &AgeBound) -> bool {
    let bd = match v.identity.bio.birth_date.as_deref() {
        Some(s) => s,
        None => return false,
    };
    let age = match compute_age(bd, season) {
        Some(a) => a,
        None => return false,
    };
    if let Some(min) = bound.min {
        if age < min {
            return false;
        }
    }
    if let Some(max) = bound.max {
        if age > max {
            return false;
        }
    }
    true
}

/// Apply a numeric predicate using the stat's unit-aware Eq
/// tolerance. Mirrors `apply_scalar_op_unit_aware` for callers
/// that already have the actual+target as f64 + the stat's unit.
fn predicate_matches_number_unit_aware(
    p: &Predicate,
    actual: f64,
    unit: icelines_core::stats_catalog::StatUnit,
) -> bool {
    match p {
        Predicate::Scalar(op, ScalarValue::Number(target)) => {
            apply_scalar_op_unit_aware(*op, actual, *target, unit)
        }
        Predicate::Range(NumericRange { min, max }) => actual >= *min && actual <= *max,
        _ => false,
    }
}

fn sliding_window_matches(
    s: &SlidingWindowConstraint,
    v: &PlayerView<'_>,
    ctx: &EvalCtx<'_>,
) -> bool {
    if !s.stat.applies_to(v.position(), v.is_goalie()) {
        return true;
    }
    let pid = v.identity.id.0;
    let lines = ctx.provider.fetch_game_lines(pid, ctx.season);
    let current_team = v.team().map(|t| t.0.as_str());
    let result = aggregate_window(&lines, &s.window, ctx.today, current_team);
    let totals = match result {
        WindowResult::Empty => return false,
        WindowResult::Full(t) => t,
        WindowResult::ShortWindow { totals, .. } => totals,
    };
    let actual = match extract_window_stat(s.stat, &totals) {
        Some(x) => x,
        None => return false,
    };
    match &s.predicate {
        Predicate::Scalar(op, ScalarValue::Number(target)) => {
            apply_scalar_op_unit_aware(*op, actual, *target, s.stat.unit())
        }
        Predicate::Range(NumericRange { min, max }) => actual >= *min && actual <= *max,
        _ => false, // Member / Pattern on numeric stat: parser-rejected
    }
}

fn bio_matches(b: &BioConstraint, v: &PlayerView<'_>, season: u32) -> bool {
    match b.field {
        BioField::Age => match age_for(v, season) {
            Some(a) => predicate_matches_number(&b.predicate, a as f64),
            None => false,
        },
        BioField::DraftYear => match v.identity.bio.draft_year {
            Some(y) => predicate_matches_number(&b.predicate, y as f64),
            None => false,
        },
        BioField::DraftRound => match v.identity.bio.draft_round {
            Some(r) => predicate_matches_number(&b.predicate, r as f64),
            None => false,
        },
        BioField::DraftOverall => match v.identity.bio.draft_overall {
            Some(o) => predicate_matches_number(&b.predicate, o as f64),
            None => false,
        },
        BioField::Height => match v.identity.bio.height_in_inches {
            Some(h) => predicate_matches_number(&b.predicate, h as f64),
            None => false,
        },
        BioField::Weight => match v.identity.bio.weight_lbs {
            Some(w) => predicate_matches_number(&b.predicate, w as f64),
            None => false,
        },
        BioField::Country => {
            // country can match either birth_country or
            // nationality_code (legacy semantics from BioAtom).
            let bc = v
                .identity
                .bio
                .birth_country
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            let nc = v
                .identity
                .bio
                .nationality_code
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            // Two candidate strings — pass to the text predicate
            // applied via OR.
            text_predicate_matches_any(&b.predicate, &[bc.as_deref(), nc.as_deref()])
        }
        BioField::Nationality => {
            let nc = v
                .identity
                .bio
                .nationality_code
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, nc.as_deref())
        }
        BioField::Shoots => {
            let s = v
                .identity
                .bio
                .shoots_catches
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, s.as_deref())
        }
        BioField::Position => {
            let p = format!("{:?}", v.position()).to_ascii_lowercase();
            // Position::Center → "center"; map to canonical short
            // tokens "c" / "lw" / "rw" / "d" / "g" so the user's
            // `pos=C` query matches.
            let canonical = position_short_code(&p);
            text_predicate_matches(&b.predicate, Some(canonical.as_str()))
        }
        BioField::Team => {
            // Current stint only.
            let t = v
                .team()
                .map(|abbr| ScalarValue::canonicalize_text(&abbr.0));
            text_predicate_matches(&b.predicate, t.as_deref())
        }
        BioField::TeamAny => {
            // Any stint this season.
            let abbrevs: Vec<String> = v
                .stats
                .team_stints
                .iter()
                .map(|s| ScalarValue::canonicalize_text(&s.team.0))
                .collect();
            let refs: Vec<Option<&str>> = abbrevs.iter().map(|s| Some(s.as_str())).collect();
            text_predicate_matches_any(&b.predicate, &refs)
        }
        BioField::TeamCareer => {
            // A.2.5 review (scout + edge) — parser now rejects
            // `team.career=` atoms with FeatureNotYet, so this
            // branch is unreachable from user input. If a future
            // caller constructs one programmatically, fail closed
            // (return false) — silent over-match was the bug.
            false
        }
        BioField::BirthCity => {
            let s = v
                .identity
                .bio
                .birth_city
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, s.as_deref())
        }
        BioField::BirthState => {
            let s = v
                .identity
                .bio
                .birth_state_province
                .as_deref()
                .map(ScalarValue::canonicalize_text);
            text_predicate_matches(&b.predicate, s.as_deref())
        }
        BioField::RookieSeason => match v.identity.bio.rookie_season.as_deref() {
            Some(s) => match s.parse::<u32>() {
                Ok(n) => predicate_matches_number(&b.predicate, n as f64),
                Err(_) => false,
            },
            None => false,
        },
    }
}

fn season_stat_matches(s: &SeasonStatConstraint, v: &PlayerView<'_>) -> bool {
    if !s.stat.applies_to(v.position(), v.is_goalie()) {
        return true;
    }
    let actual = match s.stat.read(v) {
        Some(x) => x,
        None => return false,
    };
    match &s.predicate {
        Predicate::Scalar(op, ScalarValue::Number(target)) => {
            apply_scalar_op_unit_aware(*op, actual, *target, s.stat.unit())
        }
        Predicate::Range(NumericRange { min, max }) => actual >= *min && actual <= *max,
        // Member / Pattern on numeric stat atoms is rejected at
        // parse, so this branch is unreachable from user input.
        _ => false,
    }
}

fn age_for(v: &PlayerView<'_>, season: u32) -> Option<u32> {
    v.identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| compute_age(d, season))
}

/// Apply a numeric predicate. Used by both bio numeric fields and
/// (via the SeasonStat path) catalog stats. Range and Member with
/// numeric values are honored.
fn predicate_matches_number(p: &Predicate, actual: f64) -> bool {
    match p {
        Predicate::Scalar(op, ScalarValue::Number(target)) => {
            apply_scalar_op_num(*op, actual, *target)
        }
        Predicate::Range(NumericRange { min, max }) => actual >= *min && actual <= *max,
        Predicate::Member(op, vals) => {
            let any = vals.iter().any(|v| match v {
                ScalarValue::Number(n) => (actual - *n).abs() < 1e-9,
                _ => false,
            });
            match op {
                MemberOp::In => any,
                MemberOp::NotIn => !any,
            }
        }
        // Pattern on numeric is parser-rejected; defensively false.
        _ => false,
    }
}

/// Apply a string predicate. `actual` is the canonicalized field
/// value (already NFD-stripped + lowercased). None means the field
/// is missing on the player; predicate returns false.
fn text_predicate_matches(p: &Predicate, actual: Option<&str>) -> bool {
    match p {
        Predicate::Scalar(op, ScalarValue::Text(target)) => match actual {
            Some(s) => match op {
                ScalarOp::Eq => s == target,
                ScalarOp::Ne => s != target,
                _ => false, // <, >, <=, >= on strings: parser rejects
            },
            None => false,
        },
        Predicate::Member(op, vals) => match actual {
            Some(s) => {
                let any = vals.iter().any(|v| match v {
                    ScalarValue::Text(t) => s == t.as_str(),
                    _ => false,
                });
                match op {
                    MemberOp::In => any,
                    MemberOp::NotIn => !any,
                }
            }
            None => false,
        },
        Predicate::Pattern(op, glob) => match actual {
            Some(s) => match op {
                PatternOp::Like => glob.matches(s),
                PatternOp::NotLike => !glob.matches(s),
                PatternOp::Contains => contains_match(glob, s),
                PatternOp::NotContains => !contains_match(glob, s),
            },
            None => false,
        },
        // Scalar Number on string field: parser-rejected.
        _ => false,
    }
}

/// Same as `text_predicate_matches` but tries multiple candidate
/// values (useful for `country` which matches birth_country OR
/// nationality_code, or `team.any` which checks every stint).
fn text_predicate_matches_any(p: &Predicate, candidates: &[Option<&str>]) -> bool {
    candidates
        .iter()
        .any(|c| text_predicate_matches(p, *c))
}

/// `~ pattern` is "contains" (substring match, no anchoring).
/// We treat `glob` as a literal substring (segments joined).
fn contains_match(glob: &GlobPattern, target: &str) -> bool {
    if glob.segments.is_empty() {
        return target.is_empty();
    }
    glob.segments.iter().all(|seg| target.contains(seg.as_str()))
}

fn apply_scalar_op_num(op: ScalarOp, actual: f64, target: f64) -> bool {
    match op {
        ScalarOp::Ge => actual >= target,
        ScalarOp::Le => actual <= target,
        ScalarOp::Gt => actual > target,
        ScalarOp::Lt => actual < target,
        ScalarOp::Eq => (actual - target).abs() < 1e-9,
        ScalarOp::Ne => (actual - target).abs() >= 1e-9,
    }
}

fn apply_scalar_op_unit_aware(op: ScalarOp, actual: f64, target: f64, unit: StatUnit) -> bool {
    match op {
        ScalarOp::Ge => actual >= target,
        ScalarOp::Le => actual <= target,
        ScalarOp::Gt => actual > target,
        ScalarOp::Lt => actual < target,
        ScalarOp::Eq => match unit {
            StatUnit::Count | StatUnit::Seconds => (actual - target).abs() < 0.5,
            StatUnit::Per60 => (actual - target).abs() < 1e-3,
            StatUnit::Pct | StatUnit::Rate | StatUnit::Inverted => (actual - target).abs() < 1e-6,
        },
        ScalarOp::Ne => !match unit {
            StatUnit::Count | StatUnit::Seconds => (actual - target).abs() < 0.5,
            StatUnit::Per60 => (actual - target).abs() < 1e-3,
            StatUnit::Pct | StatUnit::Rate | StatUnit::Inverted => (actual - target).abs() < 1e-6,
        },
    }
}

/// Map a Position debug-printed string to its canonical short code
/// for `pos=C` queries.
fn position_short_code(debug_form: &str) -> String {
    match debug_form {
        "center" => "c".to_string(),
        "leftwing" | "left_wing" => "lw".to_string(),
        "rightwing" | "right_wing" => "rw".to_string(),
        "defenseman" | "defense" => "d".to_string(),
        "goalie" => "g".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_a0_apply_scalar_op_ge() {
        assert!(apply_scalar_op_num(ScalarOp::Ge, 10.0, 5.0));
        assert!(apply_scalar_op_num(ScalarOp::Ge, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Ge, 4.0, 5.0));
    }

    #[test]
    fn l0_a0_apply_scalar_op_le() {
        assert!(apply_scalar_op_num(ScalarOp::Le, 4.0, 5.0));
        assert!(apply_scalar_op_num(ScalarOp::Le, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Le, 6.0, 5.0));
    }

    #[test]
    fn l0_a1_apply_scalar_op_lt_strict() {
        assert!(apply_scalar_op_num(ScalarOp::Lt, 4.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Lt, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Lt, 6.0, 5.0));
    }

    #[test]
    fn l0_a1_apply_scalar_op_gt_strict() {
        assert!(apply_scalar_op_num(ScalarOp::Gt, 6.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Gt, 5.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Gt, 4.0, 5.0));
    }

    #[test]
    fn l0_a1_apply_scalar_op_ne() {
        assert!(apply_scalar_op_num(ScalarOp::Ne, 4.0, 5.0));
        assert!(!apply_scalar_op_num(ScalarOp::Ne, 5.0, 5.0));
    }

    #[test]
    fn l0_a1_predicate_range_inclusive() {
        let p = Predicate::Range(NumericRange {
            min: 20.0,
            max: 40.0,
        });
        assert!(predicate_matches_number(&p, 20.0));
        assert!(predicate_matches_number(&p, 40.0));
        assert!(predicate_matches_number(&p, 30.0));
        assert!(!predicate_matches_number(&p, 19.0));
        assert!(!predicate_matches_number(&p, 41.0));
    }

    #[test]
    fn l0_a1_predicate_in_numeric() {
        let p = Predicate::Member(
            MemberOp::In,
            vec![
                ScalarValue::Number(2020.0),
                ScalarValue::Number(2021.0),
                ScalarValue::Number(2022.0),
            ],
        );
        assert!(predicate_matches_number(&p, 2020.0));
        assert!(predicate_matches_number(&p, 2021.0));
        assert!(!predicate_matches_number(&p, 2019.0));
    }

    #[test]
    fn l0_a1_predicate_not_in_numeric() {
        let p = Predicate::Member(
            MemberOp::NotIn,
            vec![ScalarValue::Number(2020.0), ScalarValue::Number(2021.0)],
        );
        assert!(predicate_matches_number(&p, 2019.0));
        assert!(predicate_matches_number(&p, 2022.0));
        assert!(!predicate_matches_number(&p, 2020.0));
    }

    #[test]
    fn l0_a1_predicate_in_text() {
        let p = Predicate::Member(
            MemberOp::In,
            vec![
                ScalarValue::Text("can".into()),
                ScalarValue::Text("usa".into()),
                ScalarValue::Text("swe".into()),
            ],
        );
        assert!(text_predicate_matches(&p, Some("can")));
        assert!(text_predicate_matches(&p, Some("usa")));
        assert!(!text_predicate_matches(&p, Some("rus")));
        assert!(!text_predicate_matches(&p, None));
    }

    #[test]
    fn l0_a1_predicate_not_in_text() {
        let p = Predicate::Member(
            MemberOp::NotIn,
            vec![ScalarValue::Text("can".into())],
        );
        assert!(text_predicate_matches(&p, Some("usa")));
        assert!(!text_predicate_matches(&p, Some("can")));
    }

    #[test]
    fn l0_a1_predicate_like_pattern() {
        let glob = GlobPattern::parse("Mc*");
        let p = Predicate::Pattern(PatternOp::Like, glob);
        assert!(text_predicate_matches(&p, Some("mcdavid")));
        assert!(!text_predicate_matches(&p, Some("crosby")));
    }

    #[test]
    fn l0_a1_predicate_not_like() {
        let glob = GlobPattern::parse("Mc*");
        let p = Predicate::Pattern(PatternOp::NotLike, glob);
        assert!(!text_predicate_matches(&p, Some("mcdavid")));
        assert!(text_predicate_matches(&p, Some("crosby")));
    }

    #[test]
    fn l0_a1_predicate_contains() {
        let glob = GlobPattern::parse("Da");
        let p = Predicate::Pattern(PatternOp::Contains, glob);
        assert!(text_predicate_matches(&p, Some("mcdavid")));
        assert!(!text_predicate_matches(&p, Some("crosby")));
    }

    #[test]
    fn l0_a1_position_short_codes() {
        assert_eq!(position_short_code("center"), "c");
        assert_eq!(position_short_code("leftwing"), "lw");
        assert_eq!(position_short_code("rightwing"), "rw");
        assert_eq!(position_short_code("defenseman"), "d");
        assert_eq!(position_short_code("goalie"), "g");
    }
}
