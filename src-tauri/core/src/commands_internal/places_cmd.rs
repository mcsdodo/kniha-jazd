//! The place book. See _tasks/75-place-book/02-design.md.

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use crate::db::Database;
use crate::models::{Place, PlaceSource};
use crate::places::normalise;

/// One place part-way through the fold: the spelling to show, how often that
/// exact spelling is used, and the total across every spelling of the place.
///
/// `display_uses` is tracked separately from `total` because the two answer
/// different questions — which spelling leads, and how many endpoints the place
/// has — and after two spellings have been folded they no longer agree.
struct Folded {
    display: String,
    display_uses: i64,
    total: i64,
}

/// Every place any trip names, joined onto its stored coordinate.
///
/// The join is done here rather than in SQL because the key is `normalise()`,
/// which SQLite cannot call. At tens of places the cost is nil, and the list is
/// derived rather than stored precisely so it cannot drift (ADR-033).
pub fn list_places_internal(db: &Database) -> Result<Vec<Place>, String> {
    let raw = db.distinct_trip_places().map_err(|e| e.to_string())?;
    let stored: HashMap<String, _> = db
        .all_places()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|r| (r.normalised_name.clone(), r))
        .collect();

    // Fold spellings onto one entry, keeping the most-used as what to display.
    let mut by_key: HashMap<String, Folded> = HashMap::new();
    for (spelling, uses) in raw {
        let key = normalise(&spelling);
        if key.is_empty() {
            continue;
        }
        match by_key.entry(key) {
            Entry::Vacant(slot) => {
                slot.insert(Folded {
                    display: spelling,
                    display_uses: uses,
                    total: uses,
                });
            }
            Entry::Occupied(mut slot) => {
                let folded = slot.get_mut();
                // Against the leading spelling's OWN count, never the running
                // total: the total has already absorbed other spellings, so it
                // outgrows any single one and would freeze the display on
                // whichever spelling SQLite happened to return first. Equal
                // counts are settled on the spelling itself — the byte-wise
                // smaller wins — so that order does not decide those either.
                let leads = uses > folded.display_uses
                    || (uses == folded.display_uses && spelling < folded.display);
                if leads {
                    folded.display = spelling;
                    folded.display_uses = uses;
                }
                folded.total += uses;
            }
        }
    }

    let mut out: Vec<Place> = by_key
        .into_iter()
        .map(|(key, folded)| {
            let row = stored.get(&key);
            Place {
                display_name: folded.display,
                normalised_name: key,
                uses: folded.total,
                lat: row.and_then(|r| r.lat),
                lon: row.and_then(|r| r.lon),
                source: row.and_then(|r| PlaceSource::parse(&r.source)),
            }
        })
        .collect();

    // Unplaced first — they are the work left to do — then by use, then by name
    // so the order is stable across calls.
    out.sort_by(|a, b| {
        a.lat
            .is_some()
            .cmp(&b.lat.is_some())
            .then(b.uses.cmp(&a.uses))
            .then(a.display_name.cmp(&b.display_name))
    });
    Ok(out)
}

#[cfg(test)]
#[path = "places_cmd_tests.rs"]
mod tests;
