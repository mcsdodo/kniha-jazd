//! Which "avoid" values the route map accepts and offers.
//!
//! Only `<iso3>:tolls` is supported: "paid roads" means the vignette sections,
//! and free highway sections stay allowed (01-task.md, decision 1). The value
//! is sent to the routing service in a URL, so anything else is refused before
//! a request is built.

const SUFFIX: &str = ":tolls";

fn is_toll_value(v: &str) -> bool {
    match v.strip_suffix(SUFFIX) {
        Some(iso) => iso.len() == 3 && iso.bytes().all(|b| b.is_ascii_lowercase()),
        None => false,
    }
}

/// Validate and canonicalise an avoid list from a caller.
pub fn normalise_avoid(values: Vec<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::with_capacity(values.len());
    for raw in values {
        let v = raw.trim().to_ascii_lowercase();
        if !is_toll_value(&v) {
            return Err(format!(
                "Unsupported avoid value {raw:?}. Expected <country>:tolls, for example cze:tolls."
            ));
        }
        out.push(v);
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// The checkboxes a route offers: its own `*:tolls` values plus the values the
/// request already avoids. A checked country must stay visible even when the
/// new route no longer passes a toll road there.
pub fn toll_options<'a>(possible: impl IntoIterator<Item = &'a str>, requested: &[String]) -> Vec<String> {
    let mut out: Vec<String> = possible
        .into_iter()
        .filter(|v| is_toll_value(v))
        .map(str::to_string)
        .chain(requested.iter().cloned())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Sorted union of several option lists (a round trip has one per leg alternative).
pub fn merge_options<'a>(lists: impl IntoIterator<Item = &'a [String]>) -> Vec<String> {
    let mut out: Vec<String> = lists.into_iter().flatten().cloned().collect();
    out.sort();
    out.dedup();
    out
}
