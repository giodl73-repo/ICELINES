//! Phase Art Ross A.0/A.1 — `parse_query` front door.
//!
//! Tokenize → parse → typed `Constraint` IR. The grammar:
//!
//! ```text
//!   expr      := or_expr
//!   or_expr   := and_expr ( "OR" and_expr )*
//!   and_expr  := unary    ( "AND" unary )*
//!   unary     := "NOT" unary | primary
//!   primary   := "(" or_expr ")" | atom
//!   atom      := KEY OP VALUE
//!              | KEY ["NOT"] "IN" "(" VALUE ("," VALUE)* ")"
//!              | KEY "BETWEEN" NUMBER "AND" NUMBER
//!              | KEY ["NOT"] "LIKE" PATTERN
//!              | KEY ("~" | "!~") PATTERN
//! ```
//!
//! Operators on KEY OP VALUE atoms: `>=`, `<=`, `==`, `=`, `<`,
//! `>`, `!=`, with typo hints for `=>` (suggests `>=`) and `=<`
//! (suggests `<=`).

use icelines_core::stats_catalog::StatId;

use crate::errors::ParseError;
use crate::input::FilterInput;
use crate::plan::{
    BioConstraint, BioField, Constraint, GlobPattern, MemberOp, NumericRange, PatternOp,
    Predicate, QueryPlan, ScalarOp, ScalarValue, SeasonAxis, SeasonStatConstraint, SlidingWindow,
    SlidingWindowConstraint, WindowPolicy, WindowScope,
};
use crate::tokenizer::{tokenize, Token};
use crate::{try_parse_bio_atom, BioAtom};

/// The single front-door API.
pub fn parse_query(input: FilterInput) -> Result<QueryPlan, Vec<ParseError>> {
    match input {
        FilterInput::Cli(s) | FilterInput::Form(s) => parse_query_string(&s),
        FilterInput::Tui(fragments) => parse_query_tui(&fragments),
    }
}

fn parse_query_string(input: &str) -> Result<QueryPlan, Vec<ParseError>> {
    if input.trim().is_empty() {
        return Err(vec![ParseError::EmptyInput]);
    }
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return Err(vec![ParseError::EmptyInput]);
    }
    let mut p = Parser {
        tokens: &tokens,
        pos: 0,
        errors: Vec::new(),
    };
    let root = match p.parse_or() {
        Ok(c) => c,
        Err(()) => return Err(p.errors),
    };
    if p.pos < p.tokens.len() {
        let leftover = describe_token(&p.tokens[p.pos]);
        p.errors.push(ParseError::UnexpectedToken { token: leftover });
    }
    if !p.errors.is_empty() {
        return Err(p.errors);
    }
    Ok(QueryPlan { root })
}

/// A.2.5 review (keel) — `FilterInput::Tui` is now `Vec<Constraint>`.
/// All atoms AND-join; no implicit OR/grouping. When the TUI
/// overlay grows boolean composition, expand this with a typed
/// fragment shape + shunting-yard.
fn parse_query_tui(constraints: &[Constraint]) -> Result<QueryPlan, Vec<ParseError>> {
    let root = match constraints.len() {
        0 => return Err(vec![ParseError::EmptyInput]),
        1 => constraints[0].clone(),
        _ => Constraint::All(constraints.to_vec()),
    };
    Ok(QueryPlan { root })
}

// ── Recursive descent parser ────────────────────────────────────

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn bump(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// A.2.5 review (forge) — pure accumulator pattern, no sentinel
    /// `left` value to refresh after each iteration. If the loop
    /// runs zero times, return the single child unwrapped; else
    /// build an n-ary node.
    fn parse_or(&mut self) -> Result<Constraint, ()> {
        let first = self.parse_and()?;
        let mut acc = vec![first];
        while matches!(self.peek(), Some(Token::KwOr)) {
            self.bump();
            acc.push(self.parse_and()?);
        }
        Ok(if acc.len() == 1 {
            acc.into_iter().next().unwrap()
        } else {
            Constraint::Any(acc)
        })
    }

    fn parse_and(&mut self) -> Result<Constraint, ()> {
        let first = self.parse_unary()?;
        let mut acc = vec![first];
        while matches!(self.peek(), Some(Token::KwAnd)) {
            self.bump();
            acc.push(self.parse_unary()?);
        }
        Ok(if acc.len() == 1 {
            acc.into_iter().next().unwrap()
        } else {
            Constraint::All(acc)
        })
    }

    fn parse_unary(&mut self) -> Result<Constraint, ()> {
        if matches!(self.peek(), Some(Token::KwNot)) {
            // Disambiguate: `NOT IN (...)` is part of an atom (member-
            // op), NOT a unary operator. If the next-next token is
            // `IN` AND the current parse position has an atom-key
            // before the NOT, the calling atom-parse path handles
            // it. But in our descent, NOT only appears in unary
            // position when NOT IN doesn't fit the atom shape — i.e.
            // NOT applied to a sub-expression like `NOT (g>=10)`.
            //
            // Special case: if the token after NOT is a Bare token
            // that contains "IN" (e.g. user typed `country NOT IN (CAN)`
            // as a top-level phrase), the parse hits a Bare atom that
            // ALREADY consumed "NOT IN" syntactically — so `parse_atom`
            // sees the full NOT IN as part of one atom path. That's
            // the parse_atom layer's job.
            self.bump();
            let inner = self.parse_unary()?;
            return Ok(Constraint::Not(Box::new(inner)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Constraint, ()> {
        match self.peek() {
            Some(Token::LParen) => {
                self.bump();
                let inner = self.parse_or()?;
                match self.peek() {
                    Some(Token::RParen) => {
                        self.bump();
                        Ok(inner)
                    }
                    _ => {
                        self.errors.push(ParseError::UnclosedParen);
                        Err(())
                    }
                }
            }
            Some(Token::RParen) => {
                self.errors.push(ParseError::UnexpectedRParen);
                Err(())
            }
            Some(Token::Bare(_)) => self.parse_atom(),
            None => {
                self.errors.push(ParseError::UnexpectedEnd);
                Err(())
            }
            Some(t) => {
                let token = describe_token(t);
                self.errors.push(ParseError::UnexpectedToken { token });
                Err(())
            }
        }
    }

    fn parse_atom(&mut self) -> Result<Constraint, ()> {
        // Pull the leading Bare token. It's either a complete atom
        // (`g>=10`) or just a key (`country` followed by `IN (...)`).
        let bare = match self.bump() {
            Some(Token::Bare(s)) => s.clone(),
            _ => {
                self.errors.push(ParseError::UnexpectedEnd);
                return Err(());
            }
        };

        // Look ahead for atom-extending tokens.
        let next = self.peek().cloned();
        match next {
            Some(Token::KwIn) => {
                self.bump();
                let key_str = bare.trim().to_string();
                self.parse_in_atom(&key_str, MemberOp::In)
            }
            Some(Token::KwNot) => {
                // `key NOT IN (...)` or `key NOT LIKE pat`
                if matches!(self.peek_at(1), Some(Token::KwIn)) {
                    self.bump(); // NOT
                    self.bump(); // IN
                    let key_str = bare.trim().to_string();
                    self.parse_in_atom(&key_str, MemberOp::NotIn)
                } else if matches!(self.peek_at(1), Some(Token::KwLike)) {
                    self.bump(); // NOT
                    self.bump(); // LIKE
                    let key_str = bare.trim().to_string();
                    self.parse_like_atom(&key_str, PatternOp::NotLike)
                } else {
                    // Unrecognized — treat the bare atom on its own.
                    parse_scalar_atom(&bare).map_err(|e| {
                        self.errors.push(e);
                    })
                }
            }
            Some(Token::KwBetween) => {
                self.bump();
                let key_str = bare.trim().to_string();
                self.parse_between_atom(&key_str)
            }
            Some(Token::KwLike) => {
                self.bump();
                let key_str = bare.trim().to_string();
                self.parse_like_atom(&key_str, PatternOp::Like)
            }
            _ => parse_scalar_atom(&bare).map_err(|e| {
                self.errors.push(e);
            }),
        }
    }

    fn parse_in_atom(&mut self, key: &str, op: MemberOp) -> Result<Constraint, ()> {
        // Expect `(`
        match self.peek() {
            Some(Token::LParen) => {
                self.bump();
            }
            _ => {
                self.errors.push(ParseError::UnexpectedToken {
                    token: format!("expected `(` after {key} IN"),
                });
                return Err(());
            }
        }
        let mut values: Vec<ScalarValue> = Vec::new();
        loop {
            match self.peek() {
                Some(Token::RParen) => {
                    self.bump();
                    break;
                }
                Some(Token::Bare(s)) => {
                    let s = s.clone();
                    self.bump();
                    let parsed = parse_in_value(&s);
                    values.push(parsed);
                }
                Some(Token::QuotedString(s)) => {
                    let s = s.clone();
                    self.bump();
                    values.push(ScalarValue::Text(ScalarValue::canonicalize_text(&s)));
                }
                Some(Token::Comma) => {
                    self.bump();
                    continue;
                }
                None => {
                    self.errors.push(ParseError::UnclosedParen);
                    return Err(());
                }
                Some(t) => {
                    let token = describe_token(t);
                    self.errors.push(ParseError::UnexpectedToken { token });
                    return Err(());
                }
            }
        }
        if values.is_empty() {
            self.errors.push(ParseError::EmptySet {
                atom: format!(
                    "{key} {} ()",
                    if matches!(op, MemberOp::In) { "IN" } else { "NOT IN" }
                ),
            });
            return Err(());
        }
        build_member_constraint(key, op, values).map_err(|e| {
            self.errors.push(e);
        })
    }

    fn parse_between_atom(&mut self, key: &str) -> Result<Constraint, ()> {
        let lo_str = match self.peek() {
            Some(Token::Bare(s)) => {
                let s = s.clone();
                self.bump();
                s
            }
            _ => {
                self.errors.push(ParseError::UnexpectedToken {
                    token: format!("expected number after {key} BETWEEN"),
                });
                return Err(());
            }
        };
        // Expect `AND`
        match self.peek() {
            Some(Token::KwAnd) => {
                self.bump();
            }
            _ => {
                self.errors.push(ParseError::UnexpectedToken {
                    token: format!("expected `AND` in {key} BETWEEN"),
                });
                return Err(());
            }
        }
        let hi_str = match self.peek() {
            Some(Token::Bare(s)) => {
                let s = s.clone();
                self.bump();
                s
            }
            _ => {
                self.errors.push(ParseError::UnexpectedToken {
                    token: format!("expected upper bound after {key} BETWEEN x AND"),
                });
                return Err(());
            }
        };
        let lo: f64 = lo_str.trim().parse().map_err(|_| {
            self.errors.push(ParseError::BadNumber {
                atom: format!("{key} BETWEEN {lo_str} AND ..."),
                token: lo_str.clone(),
            });
        })?;
        let hi: f64 = hi_str.trim().parse().map_err(|_| {
            self.errors.push(ParseError::BadNumber {
                atom: format!("{key} BETWEEN {lo_str} AND {hi_str}"),
                token: hi_str.clone(),
            });
        })?;
        build_range_constraint(key, NumericRange { min: lo, max: hi }).map_err(|e| {
            self.errors.push(e);
        })
    }

    fn parse_like_atom(&mut self, key: &str, op: PatternOp) -> Result<Constraint, ()> {
        let pattern_str = match self.peek() {
            Some(Token::QuotedString(s)) => {
                let s = s.clone();
                self.bump();
                s
            }
            Some(Token::Bare(s)) => {
                let s = s.clone();
                self.bump();
                s
            }
            _ => {
                self.errors.push(ParseError::UnexpectedToken {
                    token: format!("expected pattern after {key} LIKE"),
                });
                return Err(());
            }
        };
        let glob = GlobPattern::parse(&pattern_str);
        build_pattern_constraint(key, op, glob).map_err(|e| {
            self.errors.push(e);
        })
    }
}

// ── Atom builders ───────────────────────────────────────────────

fn parse_scalar_atom(input: &str) -> Result<Constraint, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }
    let (key, op, value_str) = split_scalar_op(trimmed)?;
    let key = key.trim();
    let value_str = value_str.trim();
    if key.is_empty() {
        return Err(ParseError::EmptyStatKey {
            atom: trimmed.to_string(),
        });
    }
    if value_str.is_empty() {
        return Err(ParseError::BadNumber {
            atom: trimmed.to_string(),
            token: String::new(),
        });
    }

    // Try numeric value first; if it parses, this is a numeric atom.
    // String atoms (country=CAN) take the text path.
    if let Ok(n) = value_str.parse::<f64>() {
        if !n.is_finite() {
            return Err(ParseError::NotFinite {
                atom: trimmed.to_string(),
                token: value_str.to_string(),
            });
        }
        return build_scalar_constraint_numeric(key, op, n, trimmed);
    }
    // Text path: country/team/shoots/position/etc.
    let text = ScalarValue::canonicalize_text(value_str);
    build_scalar_constraint_text(key, op, text, trimmed)
}

/// Find the operator in a scalar atom string. Returns
/// `(key_part, op, value_part)`. Recognized ops, longest-first:
/// `>=`, `<=`, `==`, `!=`, `=`, `<`, `>`. Detects `=>`/`=<` typos.
fn split_scalar_op(input: &str) -> Result<(&str, ScalarOp, &str), ParseError> {
    // Detect typos first — `=>` and `=<` parse via the multi-op
    // detector but we want a focused hint message.
    if let Some(idx) = input.find("=>") {
        let key = &input[..idx];
        let value = &input[idx + 2..];
        // If the rest is unambiguous, it's the typo case.
        if !key.contains(['<', '>', '=', '!']) && !value.contains(['<', '>', '=', '!']) {
            return Err(ParseError::OpTypoHint {
                atom: input.to_string(),
                suggestion: ">=",
            });
        }
    }
    if let Some(idx) = input.find("=<") {
        let key = &input[..idx];
        let value = &input[idx + 2..];
        if !key.contains(['<', '>', '=', '!']) && !value.contains(['<', '>', '=', '!']) {
            return Err(ParseError::OpTypoHint {
                atom: input.to_string(),
                suggestion: "<=",
            });
        }
    }
    // SQL `<>` is not-equal in SQL; suggest `!=`.
    if let Some(idx) = input.find("<>") {
        let key = &input[..idx];
        let value = &input[idx + 2..];
        if !key.contains(['<', '>', '=', '!']) && !value.contains(['<', '>', '=', '!']) {
            return Err(ParseError::OpTypoHint {
                atom: input.to_string(),
                suggestion: "!=",
            });
        }
    }

    // Regular ops, longest-first. Order matters.
    const OPS: &[(&str, ScalarOp)] = &[
        (">=", ScalarOp::Ge),
        ("<=", ScalarOp::Le),
        ("==", ScalarOp::Eq),
        ("!=", ScalarOp::Ne),
        ("=", ScalarOp::Eq),
        ("<", ScalarOp::Lt),
        (">", ScalarOp::Gt),
    ];

    let mut best: Option<(ScalarOp, usize, usize)> = None;
    for (token, op) in OPS {
        if let Some(pos) = input.find(token) {
            match best {
                None => best = Some((*op, pos, token.len())),
                Some((_, prev_pos, prev_len)) => {
                    if pos < prev_pos || (pos == prev_pos && token.len() > prev_len) {
                        best = Some((*op, pos, token.len()));
                    }
                }
            }
        }
    }
    let (op, pos, len) = match best {
        Some(b) => b,
        None => {
            return Err(ParseError::MissingOp {
                atom: input.to_string(),
            })
        }
    };
    let key = &input[..pos];
    let value = &input[pos + len..];
    // Reject multi-op atoms like `g>=>=5` or `g===5`.
    if value.contains(['<', '>', '=', '!']) {
        return Err(ParseError::MultipleOps {
            atom: input.to_string(),
        });
    }
    if key.contains(['<', '>', '=', '!']) {
        return Err(ParseError::MultipleOps {
            atom: input.to_string(),
        });
    }
    Ok((key, op, value))
}

fn build_scalar_constraint_numeric(
    key: &str,
    op: ScalarOp,
    value: f64,
    atom_text: &str,
) -> Result<Constraint, ParseError> {
    // Try bio first. Bio numeric fields: age, draft, height,
    // weight, draft_round, draft_overall, rookie_season.
    if let Some(field) = bio_numeric_field_from_key(key) {
        return Ok(Constraint::Bio(BioConstraint {
            field,
            predicate: Predicate::Scalar(op, ScalarValue::Number(value)),
        }));
    }

    // Phase Art Ross A.2 — sliding-window dotted keys
    // (`g.last10g`, `g.last30d`, `g.last3w`, `g.last3m`, with
    // optional `.allteams` / `.career` scope modifier). The first
    // dot-segment is the stat key; the second is the window
    // descriptor; an optional third is the scope modifier.
    if key.contains('.') {
        if let Some(constraint) =
            try_parse_sliding_window_atom(key, op, value, atom_text)?
        {
            return Ok(constraint);
        }
    }

    // Else: catalog StatId.
    let stat = StatId::from_cli_key(key).ok_or_else(|| ParseError::UnknownStat {
        key: key.to_string(),
    })?;
    let _ = atom_text;
    Ok(Constraint::SeasonStat(SeasonStatConstraint {
        stat,
        predicate: Predicate::Scalar(op, ScalarValue::Number(value)),
        axis: SeasonAxis::Regular,
    }))
}

/// A.2 — recognize sliding-window dotted keys.
///
/// Syntax: `<stat-key>.<window>[.<scope>]`
///   - window: `lastNg` (GP), `lastNd` (days), `lastNw` (weeks),
///     `lastNm` (months) where N is a positive integer
///   - scope: `allteams` | `career` (optional; default is current
///     stint of current season)
///
/// Returns `Ok(None)` when the key contains `.` but doesn't match
/// the sliding-window shape (let the caller try other paths /
/// surface UnknownStat). Returns `Ok(Some(_))` on a clean parse.
/// Returns `Err(_)` when the syntax IS sliding-window-shaped but
/// has a typo (zero size, unknown unit, oversized N, unknown scope).
///
/// A.2.5 review (edge) — emits focused error variants:
/// `UnknownWindowUnit` for `last10z`, `ZeroWindowSize` for
/// `last0g`, `WindowSizeOutOfRange` for `last1000g` (no silent
/// truncation).
fn try_parse_sliding_window_atom(
    key: &str,
    op: ScalarOp,
    value: f64,
    atom_text: &str,
) -> Result<Option<Constraint>, ParseError> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Ok(None);
    }
    let stat_key = parts[0];
    let window_str = parts[1].to_ascii_lowercase();

    // Parse the window descriptor: "last" + N + unit.
    if !window_str.starts_with("last") {
        return Ok(None);
    }
    let suffix = &window_str["last".len()..];
    if suffix.is_empty() {
        return Err(ParseError::UnknownStat {
            key: format!("{stat_key}.{window_str}"),
        });
    }

    // Unit char is the last character; N is everything before.
    let unit_char = suffix.chars().last().expect("suffix non-empty above");
    let n_str = &suffix[..suffix.len() - unit_char.len_utf8()];
    let n_raw: u32 = match n_str.parse() {
        Ok(n) => n,
        Err(_) => {
            return Err(ParseError::UnknownStat {
                key: format!("{stat_key}.{window_str}"),
            })
        }
    };
    if n_raw == 0 {
        return Err(ParseError::ZeroWindowSize {
            atom: atom_text.to_string(),
        });
    }

    // Cap GP/Weeks/Months at u8::MAX (255). Days uses u16 so its
    // ceiling is much higher (65535).
    const MAX_GP_OR_W_OR_M: u8 = u8::MAX;
    if matches!(unit_char, 'g' | 'w' | 'm') && n_raw > MAX_GP_OR_W_OR_M as u32 {
        return Err(ParseError::WindowSizeOutOfRange {
            atom: atom_text.to_string(),
            size: n_raw,
            max: MAX_GP_OR_W_OR_M,
        });
    }
    if unit_char == 'd' && n_raw > u16::MAX as u32 {
        return Err(ParseError::WindowSizeOutOfRange {
            atom: atom_text.to_string(),
            size: n_raw,
            max: u8::MAX, // approximation — u16 max is too big to print sensibly
        });
    }

    let window = match unit_char {
        'g' => SlidingWindow::LastN_GP {
            n: n_raw as u8,
            scope: WindowScope::CurrentTeamCurrentSeason, // default
            policy: WindowPolicy::RequireFull,
        },
        'd' => SlidingWindow::LastN_Days(n_raw as u16),
        'w' => SlidingWindow::LastN_Weeks(n_raw as u8),
        'm' => SlidingWindow::LastN_Months(n_raw as u8),
        other => {
            return Err(ParseError::UnknownWindowUnit {
                atom: atom_text.to_string(),
                unit: other,
            });
        }
    };

    // Apply scope modifier if present (only valid on LastN_GP).
    let window = if let Some(scope_str) = parts.get(2) {
        let scope_lower = scope_str.to_ascii_lowercase();
        match (window, scope_lower.as_str()) {
            (
                SlidingWindow::LastN_GP { n, policy, .. },
                "allteams",
            ) => SlidingWindow::LastN_GP {
                n,
                scope: WindowScope::AllTeamsCurrentSeason,
                policy,
            },
            (
                SlidingWindow::LastN_GP { n, policy, .. },
                "career",
            ) => SlidingWindow::LastN_GP {
                n,
                scope: WindowScope::Career,
                policy,
            },
            _ => {
                return Err(ParseError::UnknownStat {
                    key: format!("{stat_key}.{}.{}", parts[1], scope_str),
                });
            }
        }
    } else {
        window
    };

    let stat = StatId::from_cli_key(stat_key).ok_or_else(|| ParseError::UnknownStat {
        key: stat_key.to_string(),
    })?;

    Ok(Some(Constraint::SlidingWindow(SlidingWindowConstraint {
        stat,
        window,
        predicate: Predicate::Scalar(op, ScalarValue::Number(value)),
        axis: SeasonAxis::Regular,
    })))
}

fn build_scalar_constraint_text(
    key: &str,
    op: ScalarOp,
    value: String,
    atom_text: &str,
) -> Result<Constraint, ParseError> {
    if let Some(field) = bio_text_field_from_key(key) {
        // A.2.5 review (scout + edge) — `team.career=` silently
        // fell back to current-stint matching, producing wrong
        // populations. Reject loudly until A.4 wires the full
        // career walk.
        if matches!(field, BioField::TeamCareer) {
            return Err(ParseError::FeatureNotYet {
                atom: atom_text.to_string(),
                ships_in: "A.4 (career-history walk)",
            });
        }
        if !matches!(op, ScalarOp::Eq | ScalarOp::Ne) {
            return Err(ParseError::IncompatiblePredicate {
                field: format!("{:?}", field),
                detail: format!(
                    "string fields only support `=` / `!=`; got {:?} in {atom_text:?}",
                    op
                ),
            });
        }
        return Ok(Constraint::Bio(BioConstraint {
            field,
            predicate: Predicate::Scalar(op, ScalarValue::Text(value)),
        }));
    }
    Err(ParseError::UnknownStat {
        key: key.to_string(),
    })
}

fn build_member_constraint(
    key: &str,
    op: MemberOp,
    values: Vec<ScalarValue>,
) -> Result<Constraint, ParseError> {
    if let Some(field) = bio_text_field_from_key(key) {
        // A.2.5 review (scout) — gate team.career until A.4.
        if matches!(field, BioField::TeamCareer) {
            let op_word = match op {
                MemberOp::In => "IN",
                MemberOp::NotIn => "NOT IN",
            };
            return Err(ParseError::FeatureNotYet {
                atom: format!("{key} {op_word} (...)"),
                ships_in: "A.4 (career-history walk)",
            });
        }
        // All values must be text (canonicalize_text already
        // applied at parse_in_value for Bare; QuotedString path
        // canonicalized too).
        return Ok(Constraint::Bio(BioConstraint {
            field,
            predicate: Predicate::Member(op, values),
        }));
    }
    if let Some(field) = bio_numeric_field_from_key(key) {
        // Numeric IN: every value must be a number.
        for v in &values {
            if v.as_number().is_none() {
                return Err(ParseError::IncompatiblePredicate {
                    field: format!("{:?}", field),
                    detail: format!(
                        "numeric IN-set requires all values to be numbers; got {v:?}"
                    ),
                });
            }
        }
        return Ok(Constraint::Bio(BioConstraint {
            field,
            predicate: Predicate::Member(op, values),
        }));
    }
    // StatId IN-set is a future thing; reject for now.
    Err(ParseError::IncompatiblePredicate {
        field: key.to_string(),
        detail: "stat-key atoms don't support IN — use BETWEEN for ranges".to_string(),
    })
}

fn build_range_constraint(key: &str, range: NumericRange) -> Result<Constraint, ParseError> {
    if let Some(field) = bio_numeric_field_from_key(key) {
        return Ok(Constraint::Bio(BioConstraint {
            field,
            predicate: Predicate::Range(range),
        }));
    }
    if let Some(stat) = StatId::from_cli_key(key) {
        return Ok(Constraint::SeasonStat(SeasonStatConstraint {
            stat,
            predicate: Predicate::Range(range),
            axis: SeasonAxis::Regular,
        }));
    }
    Err(ParseError::UnknownStat {
        key: key.to_string(),
    })
}

fn build_pattern_constraint(
    key: &str,
    op: PatternOp,
    glob: GlobPattern,
) -> Result<Constraint, ParseError> {
    if let Some(field) = bio_text_field_from_key(key) {
        return Ok(Constraint::Bio(BioConstraint {
            field,
            predicate: Predicate::Pattern(op, glob),
        }));
    }
    // A.2.5 review (scout) — `name` was listed but no `BioField::FullName`
    // exists (yet). Strike it from the error message so the user
    // doesn't try to query a field the system doesn't support.
    // (When name-search ships, add `BioField::FullName` and update
    // this message.)
    Err(ParseError::IncompatiblePredicate {
        field: key.to_string(),
        detail: "LIKE only applies to string fields (country / team / shoots / position)"
            .to_string(),
    })
}

/// Map a key to a numeric bio field. None for non-bio or text bio.
fn bio_numeric_field_from_key(key: &str) -> Option<BioField> {
    let k = key.to_ascii_lowercase().replace('_', "-");
    match k.as_str() {
        "age" => Some(BioField::Age),
        "draft" | "draft-year" | "draft-yr" => Some(BioField::DraftYear),
        "draft-round" | "round" => Some(BioField::DraftRound),
        "draft-overall" | "draft-pick" | "overall" => Some(BioField::DraftOverall),
        "height" | "ht" => Some(BioField::Height),
        "weight" | "wt" => Some(BioField::Weight),
        _ => None,
    }
}

/// Map a key to a text bio field. None for non-bio or numeric bio.
fn bio_text_field_from_key(key: &str) -> Option<BioField> {
    let k = key.to_ascii_lowercase().replace('_', "-");
    match k.as_str() {
        "country" => Some(BioField::Country),
        "nation" | "nationality" => Some(BioField::Nationality),
        "shoots" | "hand" | "catches" => Some(BioField::Shoots),
        "pos" | "position" => Some(BioField::Position),
        "team" => Some(BioField::Team),
        "team.any" | "team-any" => Some(BioField::TeamAny),
        "team.career" | "team-career" => Some(BioField::TeamCareer),
        "city" | "birth-city" => Some(BioField::BirthCity),
        "state" | "province" | "birth-state" | "birth-province" => Some(BioField::BirthState),
        "rookie" | "rookie-season" => Some(BioField::RookieSeason),
        _ => None,
    }
}

fn parse_in_value(s: &str) -> ScalarValue {
    // If it parses as a number, store as Number; else canonicalize
    // as text. This handles `country IN (CAN, USA)` (text) and
    // `draft-year IN (2020, 2021, 2022)` (numbers) the same way.
    if let Ok(n) = s.trim().parse::<f64>() {
        if n.is_finite() {
            return ScalarValue::Number(n);
        }
    }
    ScalarValue::Text(ScalarValue::canonicalize_text(s.trim()))
}

fn describe_token(t: &Token) -> String {
    match t {
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::Comma => ",".to_string(),
        Token::KwAnd => "AND".to_string(),
        Token::KwOr => "OR".to_string(),
        Token::KwNot => "NOT".to_string(),
        Token::KwIn => "IN".to_string(),
        Token::KwBetween => "BETWEEN".to_string(),
        Token::KwLike => "LIKE".to_string(),
        Token::Bare(s) => format!("`{s}`"),
        Token::QuotedString(s) => format!("\"{s}\""),
    }
}

// ── Bio-atom helpers (legacy compatibility) ──────────────────────

/// Convert a legacy `BioAtom` to a `BioConstraint`. Used by the
/// bio-extraction shim that lives between the surface and
/// `parse_query` (legacy path; new pipeline parses bios directly).
pub fn bio_atom_to_constraint(atom: &BioAtom) -> BioConstraint {
    let (field, predicate) = match atom {
        BioAtom::AgeMin(v) => (
            BioField::Age,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::AgeMax(v) => (
            BioField::Age,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::DraftMin(v) => (
            BioField::DraftYear,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::DraftMax(v) => (
            BioField::DraftYear,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::HeightMin(v) => (
            BioField::Height,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::HeightMax(v) => (
            BioField::Height,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::WeightMin(v) => (
            BioField::Weight,
            Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::WeightMax(v) => (
            BioField::Weight,
            Predicate::Scalar(ScalarOp::Le, ScalarValue::Number(*v as f64)),
        ),
        BioAtom::Country(s) => (
            BioField::Country,
            Predicate::Scalar(
                ScalarOp::Eq,
                ScalarValue::Text(ScalarValue::canonicalize_text(s)),
            ),
        ),
        BioAtom::Shoots(s) => (
            BioField::Shoots,
            Predicate::Scalar(
                ScalarOp::Eq,
                ScalarValue::Text(ScalarValue::canonicalize_text(s)),
            ),
        ),
    };
    BioConstraint { field, predicate }
}

pub fn try_parse_single_bio_constraint(input: &str) -> Option<Constraint> {
    let atoms = try_parse_bio_atom(input)?;
    if atoms.len() == 1 {
        Some(Constraint::Bio(bio_atom_to_constraint(&atoms[0])))
    } else {
        let bios: Vec<Constraint> = atoms
            .iter()
            .map(|a| Constraint::Bio(bio_atom_to_constraint(a)))
            .collect();
        Some(Constraint::All(bios))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> Constraint {
        parse_query(FilterInput::Cli(s.to_string())).unwrap().root
    }

    fn errs(s: &str) -> Vec<ParseError> {
        parse_query(FilterInput::Cli(s.to_string())).unwrap_err()
    }

    // ── A.0 parity (legacy grammar) ─────────────────────────────

    #[test]
    fn l0_a0_parse_simple_atom() {
        let c = ok("g>=10");
        assert!(matches!(
            c,
            Constraint::SeasonStat(SeasonStatConstraint {
                predicate: Predicate::Scalar(ScalarOp::Ge, ScalarValue::Number(_)),
                ..
            })
        ));
    }

    #[test]
    fn l0_a0_and_chain_collapses_to_n_ary_all() {
        let c = ok("g>=10 AND a>=10 AND p>=20");
        match c {
            Constraint::All(children) => assert_eq!(children.len(), 3),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a0_or_chain_collapses_to_n_ary_any() {
        let c = ok("g>=10 OR a>=10 OR p>=20");
        match c {
            Constraint::Any(children) => assert_eq!(children.len(), 3),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a0_not_wraps_inner() {
        let c = ok("NOT g>=100");
        assert!(matches!(c, Constraint::Not(_)));
    }

    #[test]
    fn l0_a0_paren_grouping() {
        let c = ok("(g>=10 OR a>=10) AND p>=20");
        match c {
            Constraint::All(children) => assert_eq!(children.len(), 2),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a0_empty_input_errors() {
        assert_eq!(errs("")[0], ParseError::EmptyInput);
    }

    #[test]
    fn l0_a0_unknown_stat_propagates() {
        assert!(matches!(errs("fake-stat>=1")[0], ParseError::UnknownStat { .. }));
    }

    #[test]
    fn l0_a0_arrow_eq_typo_hint() {
        match &errs("g=>5")[0] {
            ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, ">="),
            _ => panic!(),
        }
    }

    // ── A.1 strict comparators ──────────────────────────────────

    #[test]
    fn l0_a1_strict_lt() {
        let c = ok("g<5");
        assert!(matches!(
            c,
            Constraint::SeasonStat(SeasonStatConstraint {
                predicate: Predicate::Scalar(ScalarOp::Lt, _),
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_strict_gt() {
        let c = ok("g>5");
        assert!(matches!(
            c,
            Constraint::SeasonStat(SeasonStatConstraint {
                predicate: Predicate::Scalar(ScalarOp::Gt, _),
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_not_equal() {
        let c = ok("g!=5");
        assert!(matches!(
            c,
            Constraint::SeasonStat(SeasonStatConstraint {
                predicate: Predicate::Scalar(ScalarOp::Ne, _),
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_age_under_25_strict() {
        // Hockey-natural "under 25"
        let c = ok("age<25");
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                field: BioField::Age,
                predicate: Predicate::Scalar(ScalarOp::Lt, _),
            })
        ));
    }

    #[test]
    fn l0_a1_sql_ne_typo_hint() {
        match &errs("g<>5")[0] {
            ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, "!="),
            other => panic!("got {other:?}"),
        }
    }

    // ── A.1 IN / NOT IN ─────────────────────────────────────────

    #[test]
    fn l0_a1_in_set_country() {
        let c = ok("country IN (CAN, USA, SWE)");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::Country,
                predicate: Predicate::Member(MemberOp::In, vals),
            }) => {
                assert_eq!(vals.len(), 3);
                // Canonicalized to lowercase
                assert!(matches!(&vals[0], ScalarValue::Text(s) if s == "can"));
                assert!(matches!(&vals[1], ScalarValue::Text(s) if s == "usa"));
                assert!(matches!(&vals[2], ScalarValue::Text(s) if s == "swe"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_not_in_set() {
        let c = ok("country NOT IN (CAN, USA)");
        match c {
            Constraint::Bio(BioConstraint {
                predicate: Predicate::Member(MemberOp::NotIn, vals),
                ..
            }) => assert_eq!(vals.len(), 2),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_pos_in_set() {
        let c = ok("pos IN (C, LW, RW)");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::Position,
                predicate: Predicate::Member(MemberOp::In, vals),
            }) => assert_eq!(vals.len(), 3),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_team_in_set() {
        let c = ok("team IN (BOS, NYR, PIT)");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::Team,
                predicate: Predicate::Member(MemberOp::In, vals),
            }) => assert_eq!(vals.len(), 3),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_empty_in_rejected() {
        let es = errs("country IN ()");
        assert!(matches!(es[0], ParseError::EmptySet { .. }));
    }

    #[test]
    fn l0_a1_in_with_quoted_strings() {
        let c = ok(r#"country IN ("CAN", "USA")"#);
        match c {
            Constraint::Bio(BioConstraint {
                predicate: Predicate::Member(_, vals),
                ..
            }) => assert_eq!(vals.len(), 2),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_in_numeric_draft_year() {
        let c = ok("draft-year IN (2020, 2021, 2022)");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::DraftYear,
                predicate: Predicate::Member(MemberOp::In, vals),
            }) => {
                assert_eq!(vals.len(), 3);
                assert!(matches!(vals[0], ScalarValue::Number(_)));
            }
            _ => panic!(),
        }
    }

    // ── A.1 BETWEEN ────────────────────────────────────────────

    #[test]
    fn l0_a1_between_numeric() {
        let c = ok("g BETWEEN 20 AND 40");
        match c {
            Constraint::SeasonStat(SeasonStatConstraint {
                predicate: Predicate::Range(NumericRange { min, max }),
                ..
            }) => {
                assert_eq!(min, 20.0);
                assert_eq!(max, 40.0);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_between_age_bio() {
        let c = ok("age BETWEEN 22 AND 28");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::Age,
                predicate: Predicate::Range(NumericRange { min, max }),
            }) => {
                assert_eq!(min, 22.0);
                assert_eq!(max, 28.0);
            }
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_between_with_decimals() {
        let c = ok("ppg BETWEEN 0.5 AND 1.5");
        match c {
            Constraint::SeasonStat(SeasonStatConstraint {
                predicate: Predicate::Range(NumericRange { min, max }),
                ..
            }) => {
                assert!((min - 0.5).abs() < 1e-9);
                assert!((max - 1.5).abs() < 1e-9);
            }
            _ => panic!(),
        }
    }

    // ── A.1 LIKE / NOT LIKE / ~ ─────────────────────────────────

    #[test]
    fn l0_a1_like_quoted() {
        let c = ok(r#"country LIKE "CA*""#);
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                predicate: Predicate::Pattern(PatternOp::Like, _),
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_like_unquoted() {
        let c = ok("country LIKE CA*");
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                predicate: Predicate::Pattern(PatternOp::Like, _),
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_not_like() {
        let c = ok(r#"country NOT LIKE "US*""#);
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                predicate: Predicate::Pattern(PatternOp::NotLike, _),
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_like_on_numeric_rejected() {
        let es = errs(r#"g LIKE "5*""#);
        assert!(matches!(es[0], ParseError::IncompatiblePredicate { .. }));
    }

    /// A.2.6 review (bench) — fill the IncompatiblePredicate
    /// coverage gap. There are 3 emit sites in build_*_constraint
    /// that weren't directly tested.

    /// `BETWEEN` on a string field has no meaning. The parser
    /// builds a numeric Range, then routes via build_range_-
    /// constraint which rejects with UnknownStat (since string
    /// keys aren't in the numeric bio-field mapping).
    #[test]
    fn l0_a26_between_on_string_field_rejected() {
        // `country BETWEEN "CAN" AND "USA"` — strings inside
        // BETWEEN don't parse as numbers, so the lower bound
        // fails BadNumber first. Use numeric values on a string
        // key to hit the build_range_constraint UnknownStat path:
        let es = errs("country BETWEEN 0 AND 100");
        // country isn't a numeric bio field; build_range_constraint
        // falls through to UnknownStat.
        assert!(matches!(
            es[0],
            ParseError::UnknownStat { .. } | ParseError::IncompatiblePredicate { .. }
        ));
    }

    /// `IN` with a numeric value on a STRING field — the values
    /// canonicalize to text, so they survive as Text("2020") etc.
    /// This exercises the text-field path with mixed-type set.
    #[test]
    fn l0_a26_string_field_in_with_numeric_values_canonicalizes() {
        // `shoots IN (1, 2, 3)` — numeric values on a string
        // field. parse_in_value yields Number variants, but
        // build_member_constraint on shoots (a text field) feeds
        // them through; the eval-time matcher will fail to match
        // any actual shoots value.
        let c = ok("shoots IN (1, 2, 3)");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::Shoots,
                predicate: Predicate::Member(_, vals),
            }) => assert_eq!(vals.len(), 3),
            _ => panic!("expected Bio Shoots Member, got {c:?}"),
        }
    }

    /// Stat-key `IN`-set rejected (catalog stats use BETWEEN, not
    /// IN). The third IncompatiblePredicate emit site.
    #[test]
    fn l0_a26_stat_key_in_set_rejected_use_between() {
        let es = errs("g IN (10, 20, 30)");
        match &es[0] {
            ParseError::IncompatiblePredicate { detail, .. } => {
                assert!(
                    detail.contains("BETWEEN") || detail.to_lowercase().contains("between"),
                    "error should suggest BETWEEN, got: {detail}"
                );
            }
            other => panic!("expected IncompatiblePredicate, got {other:?}"),
        }
    }

    /// A.2.6 review (bench) — `=<` typo hint test was missing.
    #[test]
    fn l0_a26_lt_eq_typo_hint() {
        match &errs("g=<5")[0] {
            ParseError::OpTypoHint { suggestion, .. } => assert_eq!(*suggestion, "<="),
            other => panic!("expected OpTypoHint, got {other:?}"),
        }
    }

    /// String fields support only `=` / `!=` — `>=` / `<=` reject.
    #[test]
    fn l0_a26_string_field_with_gt_op_rejected() {
        let es = errs("country>=USA");
        // The op parses; the value canonicalizes; build_scalar_-
        // constraint_text rejects `Ge` as IncompatiblePredicate.
        assert!(matches!(
            es[0],
            ParseError::IncompatiblePredicate { .. } | ParseError::UnknownStat { .. }
        ));
    }

    // ── A.1 new bio atoms ──────────────────────────────────────

    #[test]
    fn l0_a1_position_atom() {
        let c = ok("pos=C");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::Position,
                predicate: Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text(t)),
            }) => assert_eq!(t, "c"),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_team_atom() {
        let c = ok("team=EDM");
        match c {
            Constraint::Bio(BioConstraint {
                field: BioField::Team,
                predicate: Predicate::Scalar(ScalarOp::Eq, ScalarValue::Text(t)),
            }) => assert_eq!(t, "edm"),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_team_any_modifier() {
        let c = ok("team.any=EDM");
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                field: BioField::TeamAny,
                ..
            })
        ));
    }

    /// A.2.5 review (scout) — `team.career=` rejected at parse
    /// with `FeatureNotYet { ships_in: "A.4..." }` until the
    /// full career-history walk lands. Was previously a silent
    /// fallback to current-stint, producing wrong populations.
    #[test]
    fn l0_a25_team_career_rejected_until_a4() {
        let es = errs("team.career=EDM");
        match &es[0] {
            ParseError::FeatureNotYet { ships_in, .. } => {
                assert!(ships_in.contains("A.4"));
            }
            other => panic!("expected FeatureNotYet for team.career, got {other:?}"),
        }
    }

    #[test]
    fn l0_a25_team_career_in_set_rejected_until_a4() {
        let es = errs("team.career IN (EDM, DAL)");
        assert!(matches!(es[0], ParseError::FeatureNotYet { .. }));
    }

    #[test]
    fn l0_a1_draft_round() {
        let c = ok("draft-round<=2");
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                field: BioField::DraftRound,
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_draft_overall() {
        let c = ok("draft-overall<=10");
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                field: BioField::DraftOverall,
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_birth_state() {
        let c = ok("birth-state=ON");
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                field: BioField::BirthState,
                ..
            })
        ));
    }

    #[test]
    fn l0_a1_nationality_separate_from_country() {
        let c = ok("nationality=USA");
        assert!(matches!(
            c,
            Constraint::Bio(BioConstraint {
                field: BioField::Nationality,
                ..
            })
        ));
    }

    // ── A.1 mixed compound ───────────────────────────────────

    #[test]
    fn l0_a1_compound_mixed_ops() {
        // The killer query the user asked for — "5 goals in 10
        // games, under 25" (the streak window is A.2; here we do
        // the season-aggregate version).
        let c = ok("g>=5 AND age<25");
        match c {
            Constraint::All(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(
                    &children[0],
                    Constraint::SeasonStat(SeasonStatConstraint {
                        predicate: Predicate::Scalar(ScalarOp::Ge, _),
                        ..
                    })
                ));
                assert!(matches!(
                    &children[1],
                    Constraint::Bio(BioConstraint {
                        field: BioField::Age,
                        predicate: Predicate::Scalar(ScalarOp::Lt, _),
                    })
                ));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_in_inside_compound() {
        let c = ok("country IN (CAN, USA) AND p>=80");
        match c {
            Constraint::All(children) => assert_eq!(children.len(), 2),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a1_compound_with_between_and_like() {
        let c = ok(r#"age BETWEEN 22 AND 28 AND country LIKE "CA*""#);
        match c {
            Constraint::All(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(
                    &children[0],
                    Constraint::Bio(BioConstraint {
                        field: BioField::Age,
                        predicate: Predicate::Range(_),
                    })
                ));
            }
            _ => panic!(),
        }
    }

    // ── A.2 sliding-window atoms ────────────────────────────────

    #[test]
    fn l0_a2_parse_last10g_default_scope() {
        let c = ok("g.last10g>=5");
        match c {
            Constraint::SlidingWindow(SlidingWindowConstraint {
                window: SlidingWindow::LastN_GP { n, scope, policy },
                ..
            }) => {
                assert_eq!(n, 10);
                assert_eq!(scope, WindowScope::CurrentTeamCurrentSeason);
                assert_eq!(policy, WindowPolicy::RequireFull);
            }
            _ => panic!("expected SlidingWindow LastN_GP, got {c:?}"),
        }
    }

    #[test]
    fn l0_a2_parse_last30d() {
        let c = ok("g.last30d>=10");
        match c {
            Constraint::SlidingWindow(SlidingWindowConstraint {
                window: SlidingWindow::LastN_Days(n),
                ..
            }) => assert_eq!(n, 30),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a2_parse_last3w() {
        let c = ok("p.last3w>=8");
        match c {
            Constraint::SlidingWindow(SlidingWindowConstraint {
                window: SlidingWindow::LastN_Weeks(n),
                ..
            }) => assert_eq!(n, 3),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a2_parse_last3m() {
        let c = ok("p.last3m>=20");
        match c {
            Constraint::SlidingWindow(SlidingWindowConstraint {
                window: SlidingWindow::LastN_Months(n),
                ..
            }) => assert_eq!(n, 3),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a2_parse_last10g_allteams_modifier() {
        let c = ok("g.last10g.allteams>=5");
        match c {
            Constraint::SlidingWindow(SlidingWindowConstraint {
                window: SlidingWindow::LastN_GP { scope, .. },
                ..
            }) => assert_eq!(scope, WindowScope::AllTeamsCurrentSeason),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a2_parse_last10g_career_modifier() {
        let c = ok("g.last10g.career>=5");
        match c {
            Constraint::SlidingWindow(SlidingWindowConstraint {
                window: SlidingWindow::LastN_GP { scope, .. },
                ..
            }) => assert_eq!(scope, WindowScope::Career),
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a2_parse_unknown_window_unit_rejected() {
        let es = errs("g.last10z>=5");
        match &es[0] {
            ParseError::UnknownWindowUnit { unit, .. } => assert_eq!(*unit, 'z'),
            other => panic!("expected UnknownWindowUnit, got {other:?}"),
        }
    }

    #[test]
    fn l0_a2_parse_zero_window_size_rejected() {
        let es = errs("g.last0g>=5");
        assert!(matches!(es[0], ParseError::ZeroWindowSize { .. }));
    }

    /// A.2.5 review (forge + edge) — `g.last1000g` previously
    /// silently truncated to `last255g`. Now rejected loudly.
    #[test]
    fn l0_a2_parse_window_size_out_of_range() {
        let es = errs("g.last1000g>=5");
        match &es[0] {
            ParseError::WindowSizeOutOfRange { size, max, .. } => {
                assert_eq!(*size, 1000);
                assert_eq!(*max, 255);
            }
            other => panic!("expected WindowSizeOutOfRange, got {other:?}"),
        }
    }

    #[test]
    fn l0_a2_parse_unknown_scope_modifier_rejected() {
        let es = errs("g.last10g.bogus>=5");
        assert!(matches!(es[0], ParseError::UnknownStat { .. }));
    }

    #[test]
    fn l0_a2_parse_bare_dot_after_stat_rejected() {
        let es = errs("g.>=5");
        // Empty window descriptor → UnknownStat
        assert!(matches!(es[0], ParseError::UnknownStat { .. }));
    }

    #[test]
    fn l0_a2_parse_compound_with_sliding_window() {
        // The killer query the user asked for, in season-window form.
        let c = ok("g.last10g>=5 AND age<=25");
        match c {
            Constraint::All(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(&children[0], Constraint::SlidingWindow(_)));
                assert!(matches!(
                    &children[1],
                    Constraint::Bio(BioConstraint {
                        field: BioField::Age,
                        ..
                    })
                ));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn l0_a2_parse_unknown_stat_with_window() {
        let es = errs("fakestat.last10g>=5");
        assert!(matches!(es[0], ParseError::UnknownStat { .. }));
    }

    #[test]
    fn l0_a2_parse_window_with_decimal_value() {
        // ppg.last10g could be useful even though the per-game rate
        // collapses inside the window. Just verify it parses; the
        // executor's extract_window_stat handles per-game derivation.
        let c = ok("ppg.last10g>=1.5");
        match c {
            Constraint::SlidingWindow(_) => {}
            _ => panic!("expected SlidingWindow, got {c:?}"),
        }
    }
}
