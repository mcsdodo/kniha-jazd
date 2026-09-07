//! The one notion of "the same place name" in this application.

/// Lowercase, strip diacritics, collapse whitespace, trim.
///
/// Digits and punctuation survive: they are what distinguishes one street
/// number from another, and the book's entries are addresses.
///
/// Not [`crate::db::normalize_location`]: that one only collapses whitespace
/// and keeps case and diacritics, because it rewrites the string a trip stores.
/// This one throws that information away to make a key.
///
/// Folding is a closed table, not Unicode NFD: NFD would pull in a new
/// dependency for a 44-letter problem, and being a one-liner in JS it invites
/// the frontend to grow a second implementation that disagrees (ADR-008). The
/// price is that a letter the table does not know keeps its accent, and so
/// gets a key of its own. Decomposed (NFD) input is the same case: `Kos` +
/// U+030C + `ice` keys apart from the precomposed spelling — a duplicate row
/// in the cache, not a wrong coordinate, and keyboards and Nominatim emit
/// precomposed.
pub fn normalise(query: &str) -> String {
    let folded: String = query.chars().flat_map(fold_char).collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fold_char(c: char) -> Vec<char> {
    // `to_lowercase` can yield several chars — realistically only 'İ', which
    // becomes 'i' plus a combining dot. Taking the first drops the dot, which is
    // the key we want. The iterator is never empty, so the fallback never fires.
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
