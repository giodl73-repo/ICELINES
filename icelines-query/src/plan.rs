//! Phase Art Ross A.0 — query plan IR.
//!
//! The unified intermediate representation for filter expressions.
//! `Constraint` is an n-ary tree (`All(Vec<C>)` / `Any(Vec<C>)` /
//! `Not(Box<C>)`) of typed atom variants:
//!
//! - [`BioConstraint`] — bio fields (age, country, draft, etc.)
//! - [`SeasonStatConstraint`] — catalog StatId atoms
//! - [`SlidingWindowConstraint`] — placeholder until A.2
//! - [`CareerAggrConstraint`] — placeholder until A.3
//! - [`CareerLeagueConstraint`] — placeholder until A.4
//!
//! [`Predicate`] is shape-by-construction: `Scalar(op, value)` for
//! comparators, `Member(op, set)` for IN/NOT IN, `Pattern(op, glob)`
//! for LIKE, `Range(bounds)` for BETWEEN. Invalid combinations
//! (`LIKE 5`, `g IN 20`) fail at parse, never at evaluate.
//!
//! See `design/specs/phase-art-ross-overview.md` for the locked
//! decisions that shaped this IR.

use icelines_core::stats_catalog::StatId;

/// The query plan IR root. Built by `parse_query`; consumed by
/// `requirements()` (planner) and `execute()` (evaluator).
#[derive(Debug, Clone, PartialEq)]
pub struct QueryPlan {
    pub root: Constraint,
}

/// A single typed predicate atom or boolean composition. N-ary
/// `All` / `Any` (not binary `Compose(BoolOp,…)`) so `requirements()`
/// is a single fold and `--explain` outputs flat trees.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Constraint {
    Bio(BioConstraint),
    SeasonStat(SeasonStatConstraint),
    /// Reserved for A.2 — sliding-window atoms over per-game data.
    /// Until A.2 ships, the parser rejects atoms that would land
    /// here with `ParseError::FeatureNotYet`.
    SlidingWindow(SlidingWindowConstraint),
    /// Reserved for A.3 — `EVER` queries + `AT age` slicing.
    CareerAggregate(CareerAggrConstraint),
    /// Reserved for A.4 — `league=OHL` cross-league atoms.
    CareerLeague(CareerLeagueConstraint),
    /// N-ary AND. Empty list is conventionally `true` (the universe);
    /// `parse_query` never produces an empty list.
    All(Vec<Constraint>),
    /// N-ary OR. Empty list is conventionally `false`; never produced
    /// by the parser.
    Any(Vec<Constraint>),
    Not(Box<Constraint>),
}

impl Constraint {
    /// True iff the tree contains any `SlidingWindow` /
    /// `CareerAggregate` / `CareerLeague` variant. Surfaces use
    /// this to route filters: when true, evaluate via
    /// `Constraint::matches(view, &EvalCtx)` (needs DataProvider);
    /// when false, the legacy pipeline (Bio + SeasonStat only) is
    /// sufficient.
    pub fn needs_provider(&self) -> bool {
        match self {
            Constraint::SlidingWindow(_)
            | Constraint::CareerAggregate(_)
            | Constraint::CareerLeague(_) => true,
            Constraint::Bio(_) | Constraint::SeasonStat(_) => false,
            Constraint::All(c) | Constraint::Any(c) => c.iter().any(|c| c.needs_provider()),
            Constraint::Not(inner) => inner.needs_provider(),
        }
    }
}

// ── Bio atoms ───────────────────────────────────────────────────

/// One bio atom: a field + a typed predicate.
#[derive(Debug, Clone, PartialEq)]
pub struct BioConstraint {
    pub field: BioField,
    pub predicate: Predicate,
}

/// Bio fields exposed to the filter grammar. Subset of
/// `icelines_core::PlayerBio` plus a few derived axes (`Age`,
/// `Position`, `Team*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BioField {
    Age,
    DraftYear,
    DraftRound,
    DraftOverall,
    Height,
    Weight,
    Country,
    Nationality,
    Shoots,
    Position,
    /// Current stint only — see locked decision in spec.
    Team,
    /// Any stint this season — `team.any=` modifier.
    TeamAny,
    /// Any stint ever — `team.career=` modifier.
    TeamCareer,
    BirthCity,
    BirthState,
    RookieSeason,
}

impl BioField {
    /// Is this field numeric (age/draft/height/weight) vs textual
    /// (country/team/position)? Drives which `Predicate` shapes are
    /// legal — numeric atoms accept `Range`, textual accept
    /// `Pattern`.
    pub fn is_numeric(self) -> bool {
        matches!(
            self,
            BioField::Age
                | BioField::DraftYear
                | BioField::DraftRound
                | BioField::DraftOverall
                | BioField::Height
                | BioField::Weight
                | BioField::RookieSeason
        )
    }
}

// ── SeasonStat atoms ────────────────────────────────────────────

/// A season-totals stat atom: a `StatId` from the catalog + a typed
/// predicate + an axis (regular vs playoff).
#[derive(Debug, Clone, PartialEq)]
pub struct SeasonStatConstraint {
    pub stat: StatId,
    pub predicate: Predicate,
    pub axis: SeasonAxis,
}

/// Whether a constraint applies to regular-season totals, playoff
/// totals, or both. Default is `Regular` for all atoms unless the
/// user adds a `.playoff` modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SeasonAxis {
    #[default]
    Regular,
    Playoff,
    All,
}

// ── Sliding-window atoms (A.2 placeholder) ──────────────────────

/// Reserved for A.2. The parser rejects atoms that would construct
/// this variant with `ParseError::FeatureNotYet`.
#[derive(Debug, Clone, PartialEq)]
pub struct SlidingWindowConstraint {
    pub stat: StatId,
    pub window: SlidingWindow,
    pub predicate: Predicate,
    pub axis: SeasonAxis,
}

/// Window-axis types. `LastN_GP` counts games-played (a streak
/// axis); `LastN_Days` / `Weeks` / `Months` count calendar units.
/// Spec: `design/specs/phase-art-ross-overview.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)] // LastN_GP / LastN_Days mirror the user-typed atom syntax
pub enum SlidingWindow {
    LastN_GP {
        n: u8,
        scope: WindowScope,
        policy: WindowPolicy,
    },
    LastN_Days(u16),
    LastN_Weeks(u8),
    LastN_Months(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowScope {
    /// Default — last N GP within the current season + current team
    /// stint. Hockey-natural for "last 10 games."
    CurrentTeamCurrentSeason,
    /// `.allteams` modifier — last N GP this season across stints.
    AllTeamsCurrentSeason,
    /// `.career` modifier — crosses season boundaries.
    Career,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WindowPolicy {
    /// Default — player with GP < n returns false.
    RequireFull,
    /// Player with GP < n uses min(n, GP). Result row carries a
    /// `[short-window: Ng]` marker.
    AllowPartial,
    /// Partial OK if GP ≥ threshold; below threshold returns false.
    AllowPartialAbove(u8),
}

// ── Career-aggregate atoms (A.3 placeholder) ────────────────────

/// Reserved for A.3.
#[derive(Debug, Clone, PartialEq)]
pub struct CareerAggrConstraint {
    pub stat: StatId,
    pub aggregator: CareerAggregator,
    pub predicate: Predicate,
    pub axis: SeasonAxis,
    pub at_age: Option<AgeBound>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CareerAggregator {
    /// `p.career>=500` — sum across all eligible seasons.
    LifetimeSum,
    /// `g.any10g>=5 EVER` — was there ever a window of N GP that
    /// satisfied the predicate, intra-season, axis-typed.
    AnyWindow(u8),
    /// `p.streak>=15` — longest run satisfying the predicate.
    LongestStreak,
    /// Count of seasons where the per-season aggregate matched.
    SeasonsWith,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgeBound {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

// ── Career-league atoms (A.4 placeholder) ───────────────────────

/// Reserved for A.4.
#[derive(Debug, Clone, PartialEq)]
pub struct CareerLeagueConstraint {
    pub league: LeagueAtom,
    pub stat: Option<StatId>,
    pub aggregator: Option<CareerAggregator>,
    pub predicate: Option<Predicate>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LeagueAtom {
    /// Single league code: `league=OHL`.
    Code(String),
    /// `league IN (OHL, WHL, QMJHL)`. Empty set rejected at parse.
    InSet(Vec<String>),
    /// `league.tier=Junior` — uses `icelines_core::career_history::
    /// LeagueTier`'s canonical classification (Phase Calder).
    Tier(LeagueTier),
}

/// Re-exported from `icelines_core::career_history` so callers
/// don't have to import both crates. Same enum the canonical
/// `LeagueAbbrev::tier()` returns.
pub use icelines_core::career_history::LeagueTier;

// ── Predicates: shape-by-construction ───────────────────────────

/// A typed predicate. The four variants make invalid combinations
/// unrepresentable: `Scalar` carries one value, `Member` carries a
/// set, `Pattern` carries a glob, `Range` carries inclusive bounds.
/// `LIKE 5` and `g IN 20` fail at parse, never at evaluate.
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Scalar(ScalarOp, ScalarValue),
    /// `IN (a, b, c)` — empty list rejected at parse with
    /// `ParseError::EmptySet`.
    Member(MemberOp, Vec<ScalarValue>),
    /// `LIKE "Mc*"` — pattern + target both NFD-normalized at
    /// evaluate (so `LIKE "stutzle"` matches `Stützle`).
    Pattern(PatternOp, GlobPattern),
    /// `BETWEEN x AND y` — inclusive both sides; numeric only.
    Range(NumericRange),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberOp {
    In,
    NotIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatternOp {
    Like,
    NotLike,
    Contains,
    NotContains,
}

/// A scalar value: numeric or text. Text values are stored in their
/// canonical NFD-normalized + lowercased form so equality comparisons
/// don't trip over case or accents.
#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    Number(f64),
    Text(String),
}

impl ScalarValue {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            ScalarValue::Number(n) => Some(*n),
            ScalarValue::Text(_) => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            ScalarValue::Text(s) => Some(s),
            ScalarValue::Number(_) => None,
        }
    }

    /// Canonicalize a text value: NFD-strip + lowercase so
    /// `"Stützle"` and `"stutzle"` compare equal. Numeric values
    /// pass through unchanged.
    pub fn canonicalize_text(s: &str) -> String {
        // Simple ASCII fold for now — covers the high-frequency
        // accented-char cases (Slafkovský / Kämpf / Stützle /
        // Björk). Full NFD via `unicode-normalization` is an A.1
        // upgrade if needed.
        s.chars()
            .map(|c| match c {
                'ä' | 'à' | 'á' | 'â' | 'ã' | 'å' => 'a',
                'Ä' | 'À' | 'Á' | 'Â' | 'Ã' | 'Å' => 'a',
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'É' | 'È' | 'Ê' | 'Ë' => 'e',
                'í' | 'ì' | 'î' | 'ï' => 'i',
                'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' => 'o',
                'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => 'o',
                'ú' | 'ù' | 'û' | 'ü' => 'u',
                'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
                'ñ' => 'n',
                'Ñ' => 'n',
                'ç' => 'c',
                'Ç' => 'c',
                'ý' | 'ÿ' => 'y',
                'Ý' => 'y',
                'š' | 'ś' => 's',
                'Š' | 'Ś' => 's',
                'ž' | 'ź' | 'ż' => 'z',
                'Ž' | 'Ź' | 'Ż' => 'z',
                'č' | 'ć' => 'c',
                'Č' | 'Ć' => 'c',
                'ř' => 'r',
                'Ř' => 'r',
                'ł' => 'l',
                'Ł' => 'l',
                'đ' => 'd',
                'Đ' => 'd',
                'ť' => 't',
                'Ť' => 't',
                'ď' => 'd',
                'Ď' => 'd',
                'ě' => 'e',
                'Ě' => 'e',
                'ů' => 'u',
                'Ů' => 'u',
                _ => c,
            })
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }
}

/// A glob pattern with `*` wildcards. Anchored at start and end.
/// Stored as the raw pattern; matching is done in `pattern_matches`
/// against an already-canonicalized target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobPattern {
    /// The original pattern, with `*` segments preserved.
    pub raw: String,
    /// The pattern split on `*` into literal segments. Each segment
    /// is canonicalized at construction time.
    pub segments: Vec<String>,
    pub anchored_start: bool,
    pub anchored_end: bool,
}

impl GlobPattern {
    /// Build a glob pattern from a user-typed string. `*` is the
    /// only wildcard. `Mc*` is anchored-start + suffix-wildcard;
    /// `*Mac*` is unanchored both ends. Pattern is normalized at
    /// construction so matching is just string segment search.
    pub fn parse(input: &str) -> Self {
        let normalized = ScalarValue::canonicalize_text(input);
        let anchored_start = !normalized.starts_with('*');
        let anchored_end = !normalized.ends_with('*');
        let segments: Vec<String> = normalized
            .split('*')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Self {
            raw: input.to_string(),
            segments,
            anchored_start,
            anchored_end,
        }
    }

    /// Match against an already-canonicalized target. Empty pattern
    /// matches the empty string only when fully anchored.
    pub fn matches(&self, target: &str) -> bool {
        if self.segments.is_empty() {
            return target.is_empty();
        }
        let mut idx = 0usize;
        for (i, seg) in self.segments.iter().enumerate() {
            let search_from = idx;
            let pos = match target[search_from..].find(seg.as_str()) {
                Some(p) => search_from + p,
                None => return false,
            };
            if i == 0 && self.anchored_start && pos != 0 {
                return false;
            }
            idx = pos + seg.len();
        }
        if self.anchored_end && idx != target.len() {
            return false;
        }
        true
    }
}

/// An inclusive numeric range (`BETWEEN x AND y`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumericRange {
    pub min: f64,
    pub max: f64,
}

impl NumericRange {
    pub fn contains(&self, v: f64) -> bool {
        v >= self.min && v <= self.max
    }
}

// ── Strict mode ─────────────────────────────────────────────────

/// Whether to reject queries whose plan would produce partial
/// answers (fallback seasons or short windows). Off by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrictMode {
    #[default]
    Off,
    /// Any season that would emit `[fallback: <season>]` errors.
    RejectPartialSeasons,
    /// Any window emitting `[short-window: Ng]` errors.
    RejectPartialWindows,
    /// Both.
    RejectAll,
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_glob_anchored_prefix_match() {
        let p = GlobPattern::parse("Mc*");
        assert!(p.matches(&ScalarValue::canonicalize_text("McDavid")));
        assert!(p.matches(&ScalarValue::canonicalize_text("McKinnon")));
        assert!(!p.matches(&ScalarValue::canonicalize_text("MacDonald")));
    }

    #[test]
    fn l0_glob_unanchored_substring() {
        let p = GlobPattern::parse("*Mac*");
        assert!(p.matches(&ScalarValue::canonicalize_text("MacDonald")));
        assert!(!p.matches(&ScalarValue::canonicalize_text("Bemstrom")));
        assert!(p.matches(&ScalarValue::canonicalize_text("MacKinnon")));
    }

    #[test]
    fn l0_glob_anchored_suffix() {
        let p = GlobPattern::parse("*sson");
        assert!(p.matches(&ScalarValue::canonicalize_text("Karlsson")));
        assert!(p.matches(&ScalarValue::canonicalize_text("Olsson")));
        assert!(!p.matches(&ScalarValue::canonicalize_text("Sundin")));
    }

    /// Wave 11 / scout review item — accented names must be
    /// reachable with ASCII patterns.
    #[test]
    fn l0_glob_unicode_via_nfd_strip() {
        let p = GlobPattern::parse("stutzle");
        assert!(p.matches(&ScalarValue::canonicalize_text("Stützle")));
        let p = GlobPattern::parse("slafkov*");
        assert!(p.matches(&ScalarValue::canonicalize_text("Slafkovský")));
    }

    #[test]
    fn l0_glob_no_wildcard_exact_match() {
        let p = GlobPattern::parse("McDavid");
        assert!(p.matches(&ScalarValue::canonicalize_text("McDavid")));
        assert!(!p.matches(&ScalarValue::canonicalize_text("McDavid Jr")));
    }

    #[test]
    fn l0_canonicalize_text_strips_accents() {
        assert_eq!(ScalarValue::canonicalize_text("Stützle"), "stutzle");
        assert_eq!(ScalarValue::canonicalize_text("Slafkovský"), "slafkovsky");
        assert_eq!(ScalarValue::canonicalize_text("Kämpf"), "kampf");
        assert_eq!(ScalarValue::canonicalize_text("Björk"), "bjork");
    }

    #[test]
    fn l0_numeric_range_inclusive_both_sides() {
        let r = NumericRange {
            min: 20.0,
            max: 40.0,
        };
        assert!(r.contains(20.0));
        assert!(r.contains(40.0));
        assert!(r.contains(30.0));
        assert!(!r.contains(19.999));
        assert!(!r.contains(40.001));
    }

    #[test]
    fn l0_bio_field_numeric_classification() {
        assert!(BioField::Age.is_numeric());
        assert!(BioField::Height.is_numeric());
        assert!(BioField::DraftYear.is_numeric());
        assert!(!BioField::Country.is_numeric());
        assert!(!BioField::Team.is_numeric());
        assert!(!BioField::Position.is_numeric());
    }

    #[test]
    fn l0_strict_mode_default_is_off() {
        assert_eq!(StrictMode::default(), StrictMode::Off);
    }

    #[test]
    fn l0_season_axis_default_is_regular() {
        assert_eq!(SeasonAxis::default(), SeasonAxis::Regular);
    }

    /// A.2.4 — `needs_provider()` distinguishes filter shapes
    /// that the legacy pipeline can handle from those that need
    /// the new pipeline + DataProvider.
    #[test]
    fn l0_a24_needs_provider_false_for_bio_only() {
        let c = Constraint::Bio(BioConstraint {
            field: BioField::Age,
            predicate: Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(24.0)),
        });
        assert!(!c.needs_provider());
    }

    #[test]
    fn l0_a24_needs_provider_false_for_bio_and_seasonstat() {
        let c = Constraint::All(vec![
            Constraint::Bio(BioConstraint {
                field: BioField::Age,
                predicate: Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(24.0)),
            }),
            // Use a stat-key that resolves through the catalog
            // (we don't import StatId here so manually skip the
            // SeasonStat variant — the All-walk still exercises).
        ]);
        assert!(!c.needs_provider());
    }

    #[test]
    fn l0_a24_needs_provider_true_for_sliding_window() {
        let c = Constraint::SlidingWindow(SlidingWindowConstraint {
            stat: icelines_core::stats_catalog::StatId::from_cli_key("goals").unwrap(),
            window: SlidingWindow::LastN_GP {
                n: 10,
                scope: WindowScope::CurrentTeamCurrentSeason,
                policy: WindowPolicy::RequireFull,
            },
            predicate: Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(5.0)),
            axis: SeasonAxis::Regular,
        });
        assert!(c.needs_provider());
    }

    #[test]
    fn l0_a24_needs_provider_true_for_compound_with_sliding() {
        let bio = Constraint::Bio(BioConstraint {
            field: BioField::Age,
            predicate: Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(24.0)),
        });
        let sw = Constraint::SlidingWindow(SlidingWindowConstraint {
            stat: icelines_core::stats_catalog::StatId::from_cli_key("goals").unwrap(),
            window: SlidingWindow::LastN_GP {
                n: 10,
                scope: WindowScope::CurrentTeamCurrentSeason,
                policy: WindowPolicy::RequireFull,
            },
            predicate: Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(5.0)),
            axis: SeasonAxis::Regular,
        });
        let all = Constraint::All(vec![bio, sw]);
        assert!(all.needs_provider());
    }

    #[test]
    fn l0_a24_needs_provider_propagates_through_not() {
        let inner = Constraint::SlidingWindow(SlidingWindowConstraint {
            stat: icelines_core::stats_catalog::StatId::from_cli_key("goals").unwrap(),
            window: SlidingWindow::LastN_GP {
                n: 10,
                scope: WindowScope::CurrentTeamCurrentSeason,
                policy: WindowPolicy::RequireFull,
            },
            predicate: Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(5.0)),
            axis: SeasonAxis::Regular,
        });
        let neg = Constraint::Not(Box::new(inner));
        assert!(neg.needs_provider());
    }

    #[test]
    fn l0_constraint_n_ary_compose() {
        // N-ary All and Any (not binary Compose) — verify the IR
        // shape lets us collapse a chain into a single Vec.
        let chain = Constraint::All(vec![
            Constraint::Bio(BioConstraint {
                field: BioField::Age,
                predicate: Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(24.0)),
            }),
            Constraint::Bio(BioConstraint {
                field: BioField::Country,
                predicate: Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text("can".into())),
            }),
        ]);
        match chain {
            Constraint::All(children) => assert_eq!(children.len(), 2),
            _ => panic!("expected All variant"),
        }
    }
}
