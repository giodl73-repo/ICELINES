/// Normalize a player name for fuzzy matching.
/// Strips diacritics (Slafkovský → slafkovsky), lowercases, trims whitespace.
/// Does NOT remove hyphens or apostrophes — they are part of names.
pub fn normalize_name(s: &str) -> String {
    // Unicode NFC then strip non-spacing marks (category Mn)
    let nfc: String = unicode_normalization_nfd(s)
        .filter(|c| !is_combining_mark(*c))
        .collect();
    nfc.to_lowercase().trim().to_owned()
}

/// Manual NFD decomposition — avoids pulling in the unicode-normalization crate
/// for a single use case. Covers all characters that appear in NHL player names.
fn unicode_normalization_nfd(s: &str) -> impl Iterator<Item = char> + '_ {
    s.chars().flat_map(decompose_char)
}

fn decompose_char(c: char) -> Vec<char> {
    // Decompose common accented characters that appear in NHL player names.
    // This is not a complete Unicode NFD implementation — it covers the cases
    // we know exist in NHL rosters. Use the `unicode-normalization` crate for
    // production completeness (add to Cargo.toml when needed).
    match c {
        'Á' | 'á' => vec![base(c, 'A', 'a'), '\u{0301}'],
        'À' | 'à' => vec![base(c, 'A', 'a'), '\u{0300}'],
        'Â' | 'â' => vec![base(c, 'A', 'a'), '\u{0302}'],
        'Ä' | 'ä' => vec![base(c, 'A', 'a'), '\u{0308}'],
        'Å' | 'å' => vec![base(c, 'A', 'a'), '\u{030A}'],
        'É' | 'é' => vec![base(c, 'E', 'e'), '\u{0301}'],
        'È' | 'è' => vec![base(c, 'E', 'e'), '\u{0300}'],
        'Ê' | 'ê' => vec![base(c, 'E', 'e'), '\u{0302}'],
        'Ë' | 'ë' => vec![base(c, 'E', 'e'), '\u{0308}'],
        'Í' | 'í' => vec![base(c, 'I', 'i'), '\u{0301}'],
        'Î' | 'î' => vec![base(c, 'I', 'i'), '\u{0302}'],
        'Ï' | 'ï' => vec![base(c, 'I', 'i'), '\u{0308}'],
        'Ó' | 'ó' => vec![base(c, 'O', 'o'), '\u{0301}'],
        'Ô' | 'ô' => vec![base(c, 'O', 'o'), '\u{0302}'],
        'Ö' | 'ö' => vec![base(c, 'O', 'o'), '\u{0308}'],
        'Ø' | 'ø' => vec![base(c, 'O', 'o'), '\u{0338}'],
        'Ú' | 'ú' => vec![base(c, 'U', 'u'), '\u{0301}'],
        'Û' | 'û' => vec![base(c, 'U', 'u'), '\u{0302}'],
        'Ü' | 'ü' => vec![base(c, 'U', 'u'), '\u{0308}'],
        'Ý' | 'ý' => vec![base(c, 'Y', 'y'), '\u{0301}'],
        'Č' | 'č' => vec![base(c, 'C', 'c'), '\u{030C}'],
        'Š' | 'š' => vec![base(c, 'S', 's'), '\u{030C}'],
        'Ž' | 'ž' => vec![base(c, 'Z', 'z'), '\u{030C}'],
        'Ř' | 'ř' => vec![base(c, 'R', 'r'), '\u{030C}'],
        _ => vec![c],
    }
}

fn base(c: char, upper: char, lower: char) -> char {
    if c.is_uppercase() { upper } else { lower }
}

fn is_combining_mark(c: char) -> bool {
    // Unicode general category Mn (Non-spacing Mark): U+0300–U+036F
    ('\u{0300}'..='\u{036F}').contains(&c)
    || ('\u{1DC0}'..='\u{1DFF}').contains(&c)
    || ('\u{20D0}'..='\u{20FF}').contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_name_slafkovsky_strips_diacritic() {
        assert_eq!(normalize_name("Juraj Slafkovský"), "juraj slafkovsky");
    }

    #[test]
    fn l0_name_mcdavid_unchanged() {
        assert_eq!(normalize_name("Connor McDavid"), "connor mcdavid");
    }

    #[test]
    fn l0_name_empty_string_safe() {
        assert_eq!(normalize_name(""), "");
    }

    #[test]
    fn l0_name_all_whitespace_safe() {
        assert_eq!(normalize_name("   "), "");
    }

    #[test]
    fn l0_name_preserves_hyphen() {
        assert_eq!(normalize_name("Pierre-Luc Dubois"), "pierre-luc dubois");
    }

    #[test]
    fn l0_name_case_insensitive() {
        assert_eq!(normalize_name("MATTY BENIERS"), normalize_name("Matty Beniers"));
    }

    #[test]
    fn l0_name_necas_strips_diacritic() {
        // Martin Nečas
        assert_eq!(normalize_name("Martin Ne\u{010D}as"), "martin necas");
    }
}
