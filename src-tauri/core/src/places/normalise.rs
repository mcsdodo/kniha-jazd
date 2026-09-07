//! The one notion of "the same place name" in this application.

/// Lowercase, strip diacritics, collapse whitespace, trim.
///
/// Digits and punctuation survive: they are what distinguishes one street
/// number from another, and the book's entries are addresses.
///
/// Folding is a closed table rather than Unicode NFD because the frontend must
/// never grow a second implementation to disagree with (ADR-008) — a table is
/// explicit about which letters it knows, and an unknown letter is left alone
/// rather than silently stripped of its accent.
pub fn normalise(query: &str) -> String {
    let folded: String = query.chars().flat_map(fold_char).collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fold_char(c: char) -> Vec<char> {
    let lower = c.to_lowercase().next().unwrap_or(c);
    let replacement = match lower {
        'á' | 'ä' | 'à' | 'â' | 'ą' | 'ă' => "a",
        'č' | 'ć' | 'ç' => "c",
        'ď' => "d",
        'é' | 'ě' | 'è' | 'ê' | 'ë' | 'ę' => "e",
        'í' | 'ì' | 'î' | 'ï' => "i",
        'ĺ' | 'ľ' | 'ł' => "l",
        'ň' | 'ń' => "n",
        'ó' | 'ô' | 'ö' | 'ő' | 'ò' | 'õ' => "o",
        'ŕ' | 'ř' => "r",
        'š' | 'ś' | 'ş' => "s",
        'ť' | 'ţ' => "t",
        'ú' | 'ů' | 'ü' | 'ű' | 'ù' | 'û' => "u",
        'ý' | 'ÿ' => "y",
        'ž' | 'ź' | 'ż' => "z",
        'ß' => "ss",
        _ => return vec![lower],
    };
    replacement.chars().collect()
}
