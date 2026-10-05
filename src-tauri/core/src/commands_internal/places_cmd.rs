//! The place book. See _tasks/_done/75-place-book/02-design.md.

use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::{Database, DeletePlaceOutcome};
use crate::models::{NewPlaceRow, Place, PlaceSource};
use crate::places::{normalise, Candidate, GeocodeProvider};

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

/// Mark a place as home, or clear the mark with `None` (task 89). The journey
/// grouping and the MCP tool `list_journeys` read this mark.
pub fn set_home_place_internal(
    db: &Database,
    app_state: &AppState,
    id: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);
    match db.set_home_place(id.as_deref()) {
        Ok(()) => Ok(()),
        Err(diesel::result::Error::NotFound) => Err("Place not found".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

fn place_by_id(db: &Database, id: &str) -> Result<Place, String> {
    let row = db
        .get_place(id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Miesto neexistuje: {id}"))?;
    let uses = db.place_uses().map_err(|e| e.to_string())?.get(id).copied().unwrap_or(0);
    Ok(Place::from_row(row, uses))
}

/// The trimmed name and its key, or the error for a blank name.
fn name_and_key(name: &str) -> Result<(String, String), String> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let key = normalise(&name);
    if key.is_empty() {
        return Err("Miesto musí mať názov".to_string());
    }
    Ok((name, key))
}

/// Refuse a key that another place already holds. `except` is the place
/// being renamed: its own key is not a collision.
fn ensure_key_free(db: &Database, key: &str, except: Option<&str>) -> Result<(), String> {
    match db.get_place_by_key(key).map_err(|e| e.to_string())? {
        Some(other) if Some(other.id.as_str()) != except => {
            Err(format!("Miesto s týmto názvom už existuje: {}", other.name))
        }
        _ => Ok(()),
    }
}

pub fn create_place_internal(
    db: &Database,
    app_state: &AppState,
    name: String,
    lat: f64,
    lon: f64,
    source: PlaceSource,
) -> Result<Place, String> {
    check_read_only!(app_state);
    let (name, key) = name_and_key(&name)?;
    ensure_key_free(db, &key, None)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    db.insert_place(&NewPlaceRow {
        id: &id,
        name: &name,
        normalised_name: &key,
        lat: Some(lat),
        lon: Some(lon),
        source: Some(source.as_str()),
        created_at: &now,
    })
    .map_err(|e| e.to_string())?;
    place_by_id(db, &id)
}

pub fn rename_place_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
    name: String,
) -> Result<Place, String> {
    check_read_only!(app_state);
    let (name, key) = name_and_key(&name)?;
    ensure_key_free(db, &key, Some(&id))?;
    if db.rename_place(&id, &name, &key).map_err(|e| e.to_string())? != 1 {
        return Err(format!("Miesto neexistuje: {id}"));
    }
    place_by_id(db, &id)
}

pub fn set_place_position_internal(
    db: &Database,
    app_state: &AppState,
    id: String,
    lat: f64,
    lon: f64,
    source: PlaceSource,
) -> Result<Place, String> {
    check_read_only!(app_state);
    if db.set_place_position(&id, lat, lon, source.as_str()).map_err(|e| e.to_string())? != 1 {
        return Err(format!("Miesto neexistuje: {id}"));
    }
    place_by_id(db, &id)
}

pub fn delete_place_internal(db: &Database, app_state: &AppState, id: String) -> Result<(), String> {
    check_read_only!(app_state);
    match db.delete_place_if_unused(&id).map_err(|e| e.to_string())? {
        DeletePlaceOutcome::Deleted => Ok(()),
        DeletePlaceOutcome::InUse(n) => Err(format!("Miesto používa {n} jázd, nedá sa zmazať")),
        DeletePlaceOutcome::NotFound => Err(format!("Miesto neexistuje: {id}")),
    }
}

/// The place whose key equals `normalise(name)`. The trip form calls this
/// for typed text, so the fold stays in Rust (ADR-008).
pub fn find_place_internal(db: &Database, name: String) -> Result<Option<Place>, String> {
    let key = normalise(&name);
    if key.is_empty() {
        return Ok(None);
    }
    match db.get_place_by_key(&key).map_err(|e| e.to_string())? {
        Some(row) => place_by_id(db, &row.id).map(Some),
        None => Ok(None),
    }
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
