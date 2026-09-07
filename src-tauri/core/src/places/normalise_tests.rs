use super::normalise::normalise;

#[test]
fn folds_case() {
    assert_eq!(normalise("KOSICE"), "kosice");
}

#[test]
fn folds_slovak_diacritics() {
    assert_eq!(normalise("Spišská Nová Ves"), "spisska nova ves");
    assert_eq!(normalise("Ľubovňa"), "lubovna");
}

#[test]
fn folds_czech_and_hungarian_diacritics() {
    // The book is not country-restricted (ADR-035), so these must fold too.
    assert_eq!(normalise("Řež"), "rez");
    assert_eq!(normalise("Fót"), "fot");
    assert_eq!(normalise("Győr"), "gyor");
}

#[test]
fn collapses_whitespace() {
    assert_eq!(normalise("  Nova   Ves \t"), "nova ves");
}

#[test]
fn keeps_punctuation_that_distinguishes_addresses() {
    // "Street 1, Town" and "Street 11, Town" must not collide.
    assert_eq!(normalise("Hlavna 1, Mesto"), "hlavna 1, mesto");
}

#[test]
fn empty_input_is_empty_output() {
    assert_eq!(normalise("   "), "");
}
