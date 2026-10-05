//! The place book. See _tasks/_done/75-place-book/02-design.md.

use crate::db::Database;
use crate::models::Place;
use crate::places::{Candidate, GeocodeProvider};

/// Every place in the book with its use count. Unplaced first (the work left
/// to do), then by use, then by name, so the order is stable across calls.
pub fn list_places_internal(db: &Database) -> Result<Vec<Place>, String> {
    let uses = db.place_uses().map_err(|e| e.to_string())?;
    let mut out: Vec<Place> = db
        .all_places()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| {
            let n = uses.get(&row.id).copied().unwrap_or(0);
            Place::from_row(row, n)
        })
        .collect();
    out.sort_by(|a, b| {
        a.lat
            .is_some()
            .cmp(&b.lat.is_some())
            .then(b.uses.cmp(&a.uses))
            .then(a.name.cmp(&b.name))
    });
    Ok(out)
}

/// Candidate coordinates for one typed address. Writes nothing — looking and
/// committing are separate calls, so no coordinate is stored without a human
/// confirming it.
///
/// The provider is passed in rather than built here, the same way
/// `generate_route_internal` takes its `RouteProvider`: it is the only seam at
/// which a test can answer for the geocoder without a network stack of any
/// kind underneath, and it keeps the choice of *which* geocoder in the
/// dispatcher, where the rest of the process-level wiring already lives.
pub async fn geocode_place_internal(
    provider: &dyn GeocodeProvider,
    query: String,
) -> Result<Vec<Candidate>, String> {
    provider.search(&query).await
}

#[cfg(test)]
#[path = "places_cmd_tests.rs"]
mod tests;
