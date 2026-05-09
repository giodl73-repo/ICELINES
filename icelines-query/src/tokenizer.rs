//! Phase Art Ross A.1 — tokenizer for the unified filter grammar.
//!
//! The legacy `parse_filter_expr` in `icelines-core::stats_catalog`
//! handles only `>=`, `<=`, `==`, `=` ops. A.1 expands the grammar
//! to include strict comparators (`<`, `>`), `!=`, set membership
//! (`IN (...)` / `NOT IN`), range (`BETWEEN x AND y`), and pattern
//! matching (`LIKE "pat"` / `~` substring / `!~` not-substring).
//!
//! The tokenizer recognizes:
//!   - Booleans: AND, OR, NOT (case-insensitive, word-boundaried)
//!   - Atom keywords: IN, BETWEEN, LIKE (also case-insensitive)
//!   - Punctuation: `(` `)` `,`
//!   - Quoted strings: `"..."` and `'...'`
//!   - Bare atoms: everything else (operator + value chunks)
//!
//! Atoms are NOT pre-split here — that happens in the parser layer
//! which knows the per-key context (numeric vs string predicate).

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `,` — used inside `IN (a, b, c)` lists
    Comma,
    /// AND keyword (case-insensitive). Also appears INSIDE `BETWEEN
    /// x AND y` — the parser decides which based on context.
    KwAnd,
    /// OR keyword
    KwOr,
    /// NOT keyword
    KwNot,
    /// IN keyword
    KwIn,
    /// BETWEEN keyword
    KwBetween,
    /// LIKE keyword
    KwLike,
    /// A.3 — `EVER` modifier on `g.any10g>=5 EVER` queries.
    /// Walks every bundled season for the constraint.
    KwEver,
    /// A.3 — `AT` modifier on `... AT age<=22` queries. Slices
    /// the season set by player-age-at-time before aggregating.
    KwAt,
    /// A quoted string value (`"Mc*"` or `'Mc*'`). Quotes are
    /// stripped; the raw inner content is preserved.
    QuotedString(String),
    /// A bare token: a key (`g`, `age`), a value (`10`, `1.5`),
    /// or an operator (`>=`, `<=`, `<`, `>`, `==`, `=`, `!=`,
    /// `~`, `!~`). The parser splits an atom string into its
    /// `(key, op, value)` parts.
    Bare(String),
}

/// Tokenize an input string into a Vec<Token>. The tokenizer is
/// permissive — it only catches structural characters (parens,
/// commas, quotes, keyword boundaries). Atom-level lexing
/// (splitting on `>=` etc.) happens in the parser.
pub fn tokenize(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut i = 0;
    let mut atom_buf = String::new();

    let flush = |buf: &mut String, tokens: &mut Vec<Token>| {
        let trimmed = buf.trim();
        if !trimmed.is_empty() {
            tokens.push(Token::Bare(trimmed.to_owned()));
        }
        buf.clear();
    };

    while i < chars.len() {
        let c = chars[i];

        // Quoted string
        if c == '"' || c == '\'' {
            let quote = c;
            flush(&mut atom_buf, &mut tokens);
            i += 1;
            let start = i;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            let content: String = chars[start..i].iter().collect();
            tokens.push(Token::QuotedString(content));
            // Skip closing quote (or EOF — permissive)
            if i < chars.len() {
                i += 1;
            }
            continue;
        }

        // Punctuation
        if c == '(' {
            flush(&mut atom_buf, &mut tokens);
            tokens.push(Token::LParen);
            i += 1;
            continue;
        }
        if c == ')' {
            flush(&mut atom_buf, &mut tokens);
            tokens.push(Token::RParen);
            i += 1;
            continue;
        }
        if c == ',' {
            flush(&mut atom_buf, &mut tokens);
            tokens.push(Token::Comma);
            i += 1;
            continue;
        }

        // Keyword detection at word boundary. The legacy fence
        // applies: keyword must be at the START of a word (prev
        // char is whitespace, paren, or start-of-input) AND
        // followed by a word boundary (whitespace, paren, EOI).
        let prev_is_boundary =
            i == 0 || chars[i - 1].is_whitespace() || matches!(chars[i - 1], '(' | ')' | ',');
        if prev_is_boundary {
            let mut matched = false;
            // Try keywords longest-first so BETWEEN doesn't shadow
            // shorter prefixes (NOT before NOT IN handling — the
            // parser collapses sequential NOT + IN tokens).
            for (kw_str, tok) in [
                ("BETWEEN", Token::KwBetween),
                ("LIKE", Token::KwLike),
                ("EVER", Token::KwEver),
                ("AND", Token::KwAnd),
                ("NOT", Token::KwNot),
                ("OR", Token::KwOr),
                ("AT", Token::KwAt),
                ("IN", Token::KwIn),
            ] {
                let kw_len = kw_str.len();
                if i + kw_len > chars.len() {
                    continue;
                }
                let candidate: String = chars[i..i + kw_len]
                    .iter()
                    .map(|c| c.to_ascii_uppercase())
                    .collect();
                let next_is_boundary = match chars.get(i + kw_len) {
                    None => true,
                    Some(&c) => c.is_whitespace() || matches!(c, '(' | ')' | ','),
                };
                if candidate == kw_str && next_is_boundary {
                    flush(&mut atom_buf, &mut tokens);
                    tokens.push(tok);
                    i += kw_len;
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }
        }

        // Default: accumulate into atom buffer.
        atom_buf.push(c);
        i += 1;
    }
    flush(&mut atom_buf, &mut tokens);
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_tokenize_simple_atom() {
        let toks = tokenize("g>=10");
        assert_eq!(toks, vec![Token::Bare("g>=10".into())]);
    }

    #[test]
    fn l0_tokenize_atom_with_spaces() {
        let toks = tokenize("g >= 10");
        assert_eq!(toks, vec![Token::Bare("g >= 10".into())]);
    }

    #[test]
    fn l0_tokenize_and_chain() {
        let toks = tokenize("g>=10 AND a>=10");
        assert_eq!(
            toks,
            vec![
                Token::Bare("g>=10".into()),
                Token::KwAnd,
                Token::Bare("a>=10".into()),
            ]
        );
    }

    #[test]
    fn l0_tokenize_or_lowercase() {
        let toks = tokenize("g>=10 or a>=10");
        assert_eq!(
            toks,
            vec![
                Token::Bare("g>=10".into()),
                Token::KwOr,
                Token::Bare("a>=10".into()),
            ]
        );
    }

    #[test]
    fn l0_tokenize_not_unary() {
        let toks = tokenize("NOT g>=10");
        assert_eq!(toks, vec![Token::KwNot, Token::Bare("g>=10".into())]);
    }

    #[test]
    fn l0_tokenize_in_set() {
        let toks = tokenize("country IN (CAN, USA, SWE)");
        assert_eq!(
            toks,
            vec![
                Token::Bare("country".into()),
                Token::KwIn,
                Token::LParen,
                Token::Bare("CAN".into()),
                Token::Comma,
                Token::Bare("USA".into()),
                Token::Comma,
                Token::Bare("SWE".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn l0_tokenize_not_in_set() {
        let toks = tokenize("team NOT IN (BOS, NYR)");
        assert_eq!(
            toks,
            vec![
                Token::Bare("team".into()),
                Token::KwNot,
                Token::KwIn,
                Token::LParen,
                Token::Bare("BOS".into()),
                Token::Comma,
                Token::Bare("NYR".into()),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn l0_tokenize_between_range() {
        let toks = tokenize("g BETWEEN 20 AND 40");
        assert_eq!(
            toks,
            vec![
                Token::Bare("g".into()),
                Token::KwBetween,
                Token::Bare("20".into()),
                Token::KwAnd,
                Token::Bare("40".into()),
            ]
        );
    }

    #[test]
    fn l0_tokenize_like_quoted() {
        let toks = tokenize(r#"name LIKE "Mc*""#);
        assert_eq!(
            toks,
            vec![
                Token::Bare("name".into()),
                Token::KwLike,
                Token::QuotedString("Mc*".into()),
            ]
        );
    }

    #[test]
    fn l0_tokenize_like_single_quoted() {
        let toks = tokenize("name LIKE 'Mc*'");
        assert_eq!(
            toks,
            vec![
                Token::Bare("name".into()),
                Token::KwLike,
                Token::QuotedString("Mc*".into()),
            ]
        );
    }

    #[test]
    fn l0_tokenize_paren_grouping() {
        let toks = tokenize("(g>=10 OR a>=10) AND p>=20");
        assert_eq!(
            toks,
            vec![
                Token::LParen,
                Token::Bare("g>=10".into()),
                Token::KwOr,
                Token::Bare("a>=10".into()),
                Token::RParen,
                Token::KwAnd,
                Token::Bare("p>=20".into()),
            ]
        );
    }

    /// Wave 11 #176 — `andes` (substring contains "AND") must NOT
    /// tokenize as a keyword. Pin the word-boundary fence.
    #[test]
    fn l0_tokenize_keyword_substring_in_atom_is_not_keyword() {
        let toks = tokenize("andes>=5");
        // "andes" doesn't have a word boundary after "and" — the
        // `e` immediately follows. So we should get a single bare
        // token, not Kw:And + bare:"es>=5".
        assert_eq!(toks, vec![Token::Bare("andes>=5".into())]);
    }

    /// Wave 11 #178 — `notable` similarly has NOT as prefix but no
    /// boundary after.
    #[test]
    fn l0_tokenize_keyword_substring_prefix_is_not_keyword() {
        let toks = tokenize("notable>=5");
        assert_eq!(toks, vec![Token::Bare("notable>=5".into())]);
    }

    #[test]
    fn l0_tokenize_substring_contains_op() {
        // ~ and !~ are part of atoms; tokenizer keeps them in the
        // bare buffer and the atom-level parser splits them out.
        let toks = tokenize("name ~ Mc");
        assert_eq!(toks, vec![Token::Bare("name ~ Mc".into())]);
    }

    #[test]
    fn l0_tokenize_empty_input() {
        let toks = tokenize("");
        assert!(toks.is_empty());
    }

    #[test]
    fn l0_tokenize_whitespace_only() {
        let toks = tokenize("   \t  ");
        assert!(toks.is_empty());
    }

    #[test]
    fn l0_tokenize_quoted_string_with_spaces() {
        let toks = tokenize(r#"name LIKE "Mc Donald""#);
        assert_eq!(
            toks,
            vec![
                Token::Bare("name".into()),
                Token::KwLike,
                Token::QuotedString("Mc Donald".into()),
            ]
        );
    }

    /// A.3 — `EVER` is a global modifier suffix.
    #[test]
    fn l0_a3_tokenize_ever_keyword() {
        let toks = tokenize("g.any10g>=5 EVER");
        assert_eq!(toks, vec![Token::Bare("g.any10g>=5".into()), Token::KwEver]);
    }

    /// A.3 — `AT` introduces a sub-clause.
    #[test]
    fn l0_a3_tokenize_at_keyword() {
        let toks = tokenize("g.season>=50 AT age<=22");
        assert_eq!(
            toks,
            vec![
                Token::Bare("g.season>=50".into()),
                Token::KwAt,
                Token::Bare("age<=22".into()),
            ]
        );
    }

    /// A.3 — `EVER` and `AT` together.
    #[test]
    fn l0_a3_tokenize_ever_with_at() {
        let toks = tokenize("g.any10g>=5 EVER AT age<=25");
        assert_eq!(
            toks,
            vec![
                Token::Bare("g.any10g>=5".into()),
                Token::KwEver,
                Token::KwAt,
                Token::Bare("age<=25".into()),
            ]
        );
    }

    /// A.3 — case insensitive.
    #[test]
    fn l0_a3_tokenize_ever_lowercase() {
        let toks = tokenize("g.any10g>=5 ever");
        assert_eq!(toks, vec![Token::Bare("g.any10g>=5".into()), Token::KwEver]);
    }

    /// Word boundary fence: `evergreen` stays bare.
    #[test]
    fn l0_a3_tokenize_ever_substring_not_keyword() {
        let toks = tokenize("evergreen>=5");
        assert_eq!(toks, vec![Token::Bare("evergreen>=5".into())]);
    }
}
