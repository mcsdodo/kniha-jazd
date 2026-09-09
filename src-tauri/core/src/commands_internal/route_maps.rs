//! Generated route map commands (framework-free).
//!
//! Generating and saving are deliberately separate: `generate_route_internal`
//! only proposes a route (it writes nothing), and the caller confirms it with
//! `save_trip_route_internal`. That is what lets the user regenerate until a
//! route looks right without leaving discarded maps behind.
//!
//! Both read paths return `coordinates` — the polyline decoded into `[lat, lon]`
//! pairs — so the frontend can draw a saved map without shipping its own
//! polyline decoder (ADR-008: logic in Rust, display in the frontend).

use std::path::{Path, PathBuf};

use base64::Engine as _;
use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::check_read_only;
use crate::commands_internal::list_places_internal;
use crate::db::Database;
use crate::export::RouteMapPage;
use crate::models::{Place, RouteMap, RouteMode, RouteStart, TripGridData, Waypoint};
use crate::places::normalise;
use crate::route_map::polyline::decode;
use crate::route_map::render::render_route;
use crate::route_map::tiles::TileFetcher;
use crate::route_map::{generate_route_random, Dataset, FetchedRoute, RouteProvider, TOLERANCE};

/// A freshly generated route. Not persisted — see the module docs.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedRoute {
    pub waypoints: Vec<Waypoint>,
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for L.polyline.
    pub coordinates: Vec<[f64; 2]>,
    pub target_km: f64,
    pub road_km: f64,
    /// Estimated driving time in seconds. Shown while choosing; not persisted.
    pub duration_s: f64,
    /// Signed percentage by which the road distance misses the target.
    pub deviation_percent: f64,
    /// Whether that deviation exceeds [`TOLERANCE`].
    pub off_target: bool,
    /// `None` for direct routes -- no dataset node was involved.
    pub dataset_version: Option<String>,
    pub mode: RouteMode,
}

/// How far the finished route's road distance falls from the target, and
/// whether that is far enough to flag.
///
/// Computed here rather than in the frontend so the threshold has one home
/// (ADR-008). The display cannot invent a second, differently-measured notion
/// of "close enough".
fn deviation(target_km: f64, road_km: f64) -> (f64, bool) {
    if target_km <= 0.0 {
        return (0.0, false);
    }
    let fraction = (road_km - target_km) / target_km;
    (fraction * 100.0, fraction.abs() > TOLERANCE)
}

/// A route map loaded back from the database, ready to draw.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRouteMap {
    pub trip_id: String,
    pub waypoints: Vec<Waypoint>,
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for L.polyline.
    pub coordinates: Vec<[f64; 2]>,
    pub target_km: f64,
    pub road_km: f64,
    /// Signed percentage by which the road distance misses the target.
    pub deviation_percent: f64,
    /// Whether that deviation exceeds [`TOLERANCE`].
    pub off_target: bool,
    pub dataset_version: Option<String>,
    pub mode: RouteMode,
    /// Direct mode only: whether this saved route closes back to its own
    /// start. The frontend restores the round-trip checkbox from this on
    /// cold load -- without it, reopening a saved round trip would always
    /// show the box unticked (Task 20).
    pub round_trip: bool,
    pub created_at: String,
}

impl From<RouteMap> for SavedRouteMap {
    fn from(map: RouteMap) -> Self {
        let (deviation_percent, off_target) = deviation(map.target_km, map.road_km);
        Self {
            trip_id: map.trip_id.to_string(),
            waypoints: map.waypoints,
            coordinates: decode_coordinates(&map.polyline),
            polyline: map.polyline,
            target_km: map.target_km,
            road_km: map.road_km,
            deviation_percent,
            off_target,
            dataset_version: map.dataset_version,
            mode: map.mode,
            round_trip: map.round_trip,
            created_at: map.created_at.to_rfc3339(),
        }
    }
}

/// Polyline5 -> `[lat, lon]` pairs. `decode` never panics; malformed input
/// simply yields the prefix that parsed cleanly.
fn decode_coordinates(polyline: &str) -> Vec<[f64; 2]> {
    decode(polyline)
        .into_iter()
        .map(|(lat, lon)| [lat, lon])
        .collect()
}

/// Turn the genetic algorithm's node indices into waypoints carrying the
/// dataset's name and index.
fn waypoints_for(sequence: &[usize], ds: &Dataset) -> Result<Vec<Waypoint>, String> {
    sequence
        .iter()
        .map(|&idx| {
            let node = ds
                .nodes
                .get(idx)
                .ok_or_else(|| format!("Route referenced unknown dataset node {idx}"))?;
            let node_idx = i32::try_from(node.idx)
                .map_err(|_| format!("Dataset node index {} is out of range", node.idx))?;
            Ok(Waypoint {
                lat: node.lat,
                lon: node.lon,
                name: Some(node.name.clone()),
                node_idx: Some(node_idx),
            })
        })
        .collect()
}

/// Propose a round trip of roughly `target_km`, with road-following geometry
/// from `provider`. Persists NOTHING — the caller confirms with
/// `save_trip_route_internal`.
pub async fn generate_route_internal(
    provider: &dyn RouteProvider,
    target_km: f64,
) -> Result<GeneratedRoute, String> {
    let ds = Dataset::bundled();
    let result = generate_route_random(target_km, &ds);
    let waypoints = waypoints_for(&result.sequence, &ds)?;

    let coords: Vec<(f64, f64)> = waypoints.iter().map(|w| (w.lat, w.lon)).collect();
    let fetched = provider.fetch(&coords).await?;

    let (deviation_percent, off_target) = deviation(target_km, fetched.road_km);

    Ok(GeneratedRoute {
        coordinates: decode_coordinates(&fetched.polyline),
        polyline: fetched.polyline,
        waypoints,
        target_km,
        road_km: fetched.road_km,
        duration_s: fetched.duration_s,
        deviation_percent,
        off_target,
        dataset_version: Some(ds.version),
        mode: RouteMode::Loop,
    })
}

/// How many routes to offer. Three is what a navigation app shows and what
/// fits a panel; more is noise nobody reads.
const MAX_ALTERNATIVES: usize = 3;

/// A point the user dragged off `polyline`, to be placed into the waypoint
/// list before routing.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertPoint {
    pub lat: f64,
    pub lon: f64,
    /// The geometry it was dragged from -- the ordering that decides which leg
    /// it belongs to.
    pub polyline: String,
}

/// Route an ordered waypoint list, offering alternatives where the routing
/// service can produce them.
///
/// Persists NOTHING -- like `generate_route_internal`, the caller confirms with
/// `save_trip_route_internal`.
///
/// The returned `waypoints` are AUTHORITATIVE: when `insert` is present they
/// already include the new point in its computed slot, so the frontend adopts
/// the list rather than maintaining its own ordering. `round_trip` is applied
/// the same way, and symmetrically (Task 20, fix round 2): the list is
/// normalised to match `round_trip` regardless of whether the caller handed
/// it open or already closed, appending a clone of the (post-insert) first
/// waypoint when `round_trip` is true and the list is open, and stripping a
/// stale closing point back off when `round_trip` is false and the list is
/// already closed. The caller's own waypoint list is never trusted to
/// already be in the right shape -- see the comment at the guard itself.
pub async fn route_direct_internal(
    provider: &dyn RouteProvider,
    waypoints: Vec<Waypoint>,
    target_km: f64,
    insert: Option<InsertPoint>,
    round_trip: bool,
) -> Result<Vec<GeneratedRoute>, String> {
    // Guard BEFORE inserting: a one-point list plus a dragged-in point would
    // otherwise become a routable two-point route, silently inventing a
    // journey out of half a one.
    if waypoints.len() < 2 {
        return Err(format!(
            "A route needs a start and an end, got {} point(s).",
            waypoints.len()
        ));
    }

    let mut waypoints = match insert {
        Some(p) => insert_waypoint(&waypoints, &p.polyline, p.lat, p.lon),
        None => waypoints,
    };

    // Close (or open) the loop AFTER insert, never before (design decision
    // 2): the dragged-in via is placed by nearest-vertex geometry, and that
    // geometry is ambiguous on a route that already doubles back on itself.
    // Insert against the open one-way line, then normalise it.
    //
    // Symmetric, not just idempotent (Task 20, fix round 2 -- this is the
    // THIRD frontend entry point that leaked a stale closed list into this
    // function with `round_trip: false`; closing each one individually
    // invites a fourth). This function is the single place ADR-008 says must
    // be authoritative, so it normalises the list to match `round_trip`
    // regardless of what shape the caller handed it:
    //
    // - `round_trip == true`, list open -> append a clone of the first point.
    // - `round_trip == false`, list closed -> strip the trailing point back off.
    // - Otherwise the list already matches `round_trip` -- leave it alone.
    //
    // "Closed" cannot be decided from coordinates alone. `mode_for` (below)
    // guards Direct-vs-Loop by comparing NAMES after `places::normalise`,
    // never by coordinate, and `save_place_internal` (`places_cmd.rs`) keys
    // the book on `normalised_name` alone -- it enforces no coordinate
    // uniqueness. So two DIFFERENT book entries for one real address under
    // different spellings (e.g. "Mlynske Nivy 14" and "Mlynske Nivy 14,
    // Bratislava") can hold bit-identical coordinates and still reach this
    // function in Direct mode: `mode_for` never sees them as the same place,
    // because their names differ.
    //
    // A genuine closing point, by contrast, is always a CLONE of the first
    // waypoint -- see the `push` below -- so it carries the identical name
    // too (`None` clones to `None`, `Some(x)` clones to `Some(x)`). Comparing
    // the name as well as the coordinate is what tells "this function closed
    // it before" apart from "two distinct, merely co-located places": the
    // former matches on both, the latter only on coordinate.
    //
    // `None == None` counts as a name match, so two UNNAMED points at one
    // coordinate still read as closed. That is the right call, not a gap:
    // `first`/`last` are the row's origin/destination, always sourced from a
    // named place (`name` is `None` only for a via a human dragged onto the
    // map, never for an endpoint), so two unnamed endpoints sharing a
    // coordinate is not a real scenario this guard needs to separate.
    let already_closed = waypoints.len() > 1
        && waypoints.first().zip(waypoints.last()).is_some_and(|(first, last)| {
            first.lat == last.lat && first.lon == last.lon && first.name == last.name
        });
    if round_trip {
        if !already_closed {
            let first = waypoints[0].clone();
            waypoints.push(first);
        }
    } else if already_closed && waypoints.len() > 2 {
        // The `> 2` guard IS load-bearing, not defensive: a 2-point list
        // that is "closed" (same coordinate AND same name on both ends) can
        // still reach here, because `route_direct_internal` is reachable
        // directly over `POST /api/rpc`, not only through the UI's
        // `mode_for` decision that would normally have routed such a row as
        // Loop instead. Without this guard, that call would pop down to a
        // single, unroutable point. With it, the list is left alone -- the
        // "otherwise leave it alone" case in the summary above.
        waypoints.pop();
    }

    let coords: Vec<(f64, f64)> = waypoints.iter().map(|w| (w.lat, w.lon)).collect();
    let fetched = provider
        .fetch_alternatives(&coords, MAX_ALTERNATIVES)
        .await?;

    Ok(fetched
        .into_iter()
        .map(|route| {
            // The same deviation helper loop mode uses. A second, separately
            // measured notion of "close enough" is exactly what ADR-008 rules
            // out.
            let (deviation_percent, off_target) = deviation(target_km, route.road_km);
            GeneratedRoute {
                coordinates: decode_coordinates(&route.polyline),
                polyline: route.polyline,
                waypoints: waypoints.clone(),
                target_km,
                road_km: route.road_km,
                duration_s: route.duration_s,
                deviation_percent,
                off_target,
                dataset_version: None,
                mode: RouteMode::Direct,
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Round trip as two legs (Task 78)
// ---------------------------------------------------------------------------

/// Which leg of a round trip a dragged-in point belongs to.
///
/// Reported by the caller, never re-derived here from geometry. The browser
/// knows it for certain -- the ghost handle is attached to one leg's own
/// polyline -- and the derivation is exactly what Task 72 got wrong: a via
/// dropped on the way home was searched for in the open outbound list against
/// the closed polyline, so it landed on the way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Leg {
    Outbound,
    Inbound,
}

/// A point the user dragged off ONE leg's polyline.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegInsertPoint {
    pub lat: f64,
    pub lon: f64,
    /// That leg's own geometry -- the ordering that decides where in that
    /// leg's waypoint list the point belongs.
    pub polyline: String,
    pub leg: Leg,
}

/// One alternative for one leg. Carries no deviation of its own: a leg is
/// half a journey, and the target distance describes the whole one.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegRoute {
    pub polyline: String,
    /// Decoded `[lat, lon]` pairs, ready for L.polyline.
    pub coordinates: Vec<[f64; 2]>,
    pub road_km: f64,
    pub duration_s: f64,
}

impl LegRoute {
    fn from_fetched(route: &FetchedRoute) -> Self {
        Self {
            coordinates: decode_coordinates(&route.polyline),
            polyline: route.polyline.clone(),
            road_km: route.road_km,
            duration_s: route.duration_s,
        }
    }
}

/// What one pair of legs adds up to. Precomputed for every pair the two
/// requests could produce (at most 3 x 3), so selecting an alternative is an
/// index change in the browser and not a calculation (ADR-008).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombinedLeg {
    pub road_km: f64,
    pub duration_s: f64,
    /// Signed percentage by which the PAIR misses the target -- the whole
    /// journey is what the trip records, so the whole journey is what the
    /// deviation measures.
    pub deviation_percent: f64,
    pub off_target: bool,
}

/// Both legs of a round trip, and the table of what each pair adds up to.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundTripRoutes {
    /// The normalised OPEN list for each leg. AUTHORITATIVE: the caller adopts
    /// these rather than keeping its own, exactly as `route_direct_internal`'s
    /// `waypoints` are adopted.
    pub outbound_waypoints: Vec<Waypoint>,
    pub inbound_waypoints: Vec<Waypoint>,
    pub outbound: Vec<LegRoute>,
    pub inbound: Vec<LegRoute>,
    /// `combined[i][j]` for outbound alternative `i` and return alternative `j`.
    pub combined: Vec<Vec<CombinedLeg>>,
    pub target_km: f64,
}

/// Route a round trip as TWO requests, one per leg.
///
/// This is the whole point of the task. OSRM offers alternatives only for a
/// two-point request, so the single `[A, B, A]` call this replaces could never
/// have offered a choice, and the way home was whatever the through-route
/// produced. Two requests give each leg its own alternatives and let the
/// return take a different road.
///
/// Persists NOTHING. The caller confirms with
/// `save_trip_round_trip_route_internal`.
///
/// The returned waypoint lists are authoritative in the same sense as
/// `route_direct_internal`'s (ADR-041): the caller's own shapes are never
/// trusted. The return leg is derived when it is absent, and its two ends are
/// overwritten from the outbound leg when it is present, so the pair always
/// joins -- dragging the outbound leg's destination handle moves the return
/// leg's start with it, and no caller can hand in a broken pair.
pub async fn route_round_trip_internal(
    provider: &dyn RouteProvider,
    outbound: Vec<Waypoint>,
    inbound: Vec<Waypoint>,
    target_km: f64,
    insert: Option<LegInsertPoint>,
) -> Result<RoundTripRoutes, String> {
    if outbound.len() < 2 {
        return Err(format!(
            "A route needs a start and an end, got {} point(s).",
            outbound.len()
        ));
    }

    // The same first/last comparison ADR-041 uses -- coordinate AND name. A
    // row naming one place twice is a Loop (`mode_for`), and a loop is already
    // closed: it has no second leg to route.
    let first = &outbound[0];
    let last = &outbound[outbound.len() - 1];
    if first.lat == last.lat && first.lon == last.lon && first.name == last.name {
        return Err("A round trip needs two different endpoints.".to_string());
    }

    let mut outbound = outbound;
    let mut inbound = inbound;

    // Insert into the named leg only, against that leg's own polyline, and
    // BEFORE the ends are re-joined below. `insert_waypoint` never returns a
    // list with a new first or last element, so a drag can never move where a
    // leg began or ended.
    if let Some(point) = insert {
        match point.leg {
            Leg::Outbound => {
                outbound = insert_waypoint(&outbound, &point.polyline, point.lat, point.lon);
            }
            Leg::Inbound => {
                if inbound.len() >= 2 {
                    inbound = insert_waypoint(&inbound, &point.polyline, point.lat, point.lon);
                }
                // A drag on a return leg the caller did not send is not a real
                // scenario -- the leg has to be on screen to be dragged -- and
                // inserting into a list that is about to be replaced wholesale
                // would only invent a via nobody placed.
            }
        }
    }

    if inbound.len() < 2 {
        inbound = vec![outbound[outbound.len() - 1].clone(), outbound[0].clone()];
    } else {
        let last_index = inbound.len() - 1;
        inbound[0] = outbound[outbound.len() - 1].clone();
        inbound[last_index] = outbound[0].clone();
    }

    let out_coords: Vec<(f64, f64)> = outbound.iter().map(|w| (w.lat, w.lon)).collect();
    let in_coords: Vec<(f64, f64)> = inbound.iter().map(|w| (w.lat, w.lon)).collect();

    let out_routes = provider
        .fetch_alternatives(&out_coords, MAX_ALTERNATIVES)
        .await?;
    let in_routes = provider
        .fetch_alternatives(&in_coords, MAX_ALTERNATIVES)
        .await?;

    let combined = out_routes
        .iter()
        .map(|out| {
            in_routes
                .iter()
                .map(|back| {
                    let road_km = out.road_km + back.road_km;
                    // The same `deviation` helper loop mode and one-way mode
                    // use. A second, separately measured notion of "close
                    // enough" is exactly what ADR-008 rules out.
                    let (deviation_percent, off_target) = deviation(target_km, road_km);
                    CombinedLeg {
                        road_km,
                        duration_s: out.duration_s + back.duration_s,
                        deviation_percent,
                        off_target,
                    }
                })
                .collect()
        })
        .collect();

    Ok(RoundTripRoutes {
        outbound_waypoints: outbound,
        inbound_waypoints: inbound,
        outbound: out_routes.iter().map(LegRoute::from_fetched).collect(),
        inbound: in_routes.iter().map(LegRoute::from_fetched).collect(),
        combined,
        target_km,
    })
}

pub fn get_trip_route_internal(
    db: &Database,
    trip_id: String,
) -> Result<Option<SavedRouteMap>, String> {
    let map = db.get_route_map(&trip_id).map_err(|e| e.to_string())?;
    Ok(map.map(SavedRouteMap::from))
}

/// Save (or replace) the map for a trip.
///
/// `dataset_version` and `created_at` are stamped here rather than accepted
/// from the caller: they describe what the backend actually used and when it
/// stored it, so a client cannot misreport either.
///
/// `round_trip` gets the same treatment as `dataset_version`: a loop is
/// already closed, so `round_trip` is forced to `false` for `RouteMode::Loop`
/// regardless of what the caller sends, rather than trusting the caller to
/// only ever send `false` for a loop (design decision 3, Task 20).
pub fn save_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
    waypoints: Vec<Waypoint>,
    polyline: String,
    target_km: f64,
    road_km: f64,
    mode: RouteMode,
    round_trip: bool,
) -> Result<(), String> {
    check_read_only!(app_state);
    let trip_uuid = Uuid::parse_str(&trip_id).map_err(|e| format!("Invalid trip id: {e}"))?;

    let map = RouteMap {
        trip_id: trip_uuid,
        waypoints,
        polyline,
        target_km,
        road_km,
        mode,
        dataset_version: match mode {
            // Only a loop actually used the bundled node set.
            RouteMode::Loop => Some(Dataset::bundled().version),
            RouteMode::Direct => None,
        },
        created_at: Utc::now(),
        round_trip: match mode {
            RouteMode::Loop => false,
            RouteMode::Direct => round_trip,
        },
        turnaround_index: None,
    };

    db.save_route_map(&map).map_err(|e| e.to_string())
}

/// Deleting a map a trip never had is a no-op, not an error.
pub fn delete_trip_route_internal(
    db: &Database,
    app_state: &AppState,
    trip_id: String,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_route_map(&trip_id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Export attachments
// ---------------------------------------------------------------------------

/// Attachment canvas, in pixels. Sized for the A4-landscape attachment page the
/// export lays out (`max-height: 170mm`) at roughly 150 dpi — enough that a
/// printed map stays readable, small enough that a year of them still base64s
/// into one HTML file.
const MAP_WIDTH: u32 = 1400;
const MAP_HEIGHT: u32 = 900;

/// The printed table's rows, in printed order, as `(record number, trip id)`.
///
/// This mirrors [`crate::export::generate_html`]'s own row assembly, and is the
/// single place either export path may get record numbers from:
///
/// * The number is `trip_numbers[trip_id]` — literally the value printed in the
///   table's first column — not a position in any list. Positions differ
///   between the two export modes (desktop prepends a synthetic first record,
///   and month-end rows are interleaved); the printed number does not.
/// * The order follows `sort_direction`, so attachments are numbered in the
///   order a reader meets their rows.
///
/// Month-end rows are absent because they are not trips and can hold no map.
/// The synthetic "Prvý záznam" row (`Uuid::nil()`) is skipped because it prints
/// an empty record number — an attachment citing "záznam č. 0" would point at
/// a row that carries no number at all.
pub fn assemble_export_rows(grid_data: &TripGridData, sort_direction: &str) -> Vec<(usize, String)> {
    let mut rows: Vec<(usize, String)> = grid_data
        .trips
        .iter()
        .filter(|trip| trip.id != Uuid::nil())
        .map(|trip| {
            let trip_id = trip.id.to_string();
            let number = grid_data
                .trip_numbers
                .get(&trip_id)
                .copied()
                .unwrap_or_default();
            (usize::try_from(number).unwrap_or_default(), trip_id)
        })
        .collect();

    // Same rule as the export: "desc" is newest first, anything else ascending.
    // `sort_by` is stable, so rows sharing a number keep the grid's order in
    // both directions, exactly as the table does.
    let descending = sort_direction.eq_ignore_ascii_case("desc");
    rows.sort_by(|a, b| {
        let ordering = a.0.cmp(&b.0);
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });

    rows
}

/// Where rendered exports cache OSM tiles, given the application's data dir.
///
/// A subdirectory rather than the data dir itself: the cache is disposable and
/// is deliberately neither backed up nor moved with the database, so it must be
/// separable from everything that is.
pub fn tile_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cache")
}

/// Build attachment pages from ALREADY-ASSEMBLED rows.
///
/// `rows` must be the same ordered `(record number, trip id)` list the printed
/// table is numbered from — see [`assemble_export_rows`], and never recompute
/// it. Desktop injects a synthetic row that server mode does not, so an
/// independently derived record number makes the two modes cite different rows
/// for the same map, and the printed evidence then points at the wrong journey.
///
/// Attachment numbers run 1, 2, 3 … over the pages actually produced, so a row
/// whose map cannot be drawn closes the gap rather than leaving a hole.
///
/// Nothing here can fail the export: a database error yields no attachments,
/// and a route that cannot be decoded or rendered costs only its own page.
pub async fn collect_route_map_pages(
    db: &Database,
    tiles: &dyn TileFetcher,
    rows: &[(usize, String)],
) -> Vec<RouteMapPage> {
    let trip_ids: Vec<String> = rows.iter().map(|(_, trip_id)| trip_id.clone()).collect();

    // One batched query for the whole export — a lookup per row would be a
    // year's worth of queries for a document that is generated in one go.
    let maps = match db.get_route_maps_for_trips(&trip_ids) {
        Ok(maps) => maps,
        Err(e) => {
            log::warn!("Could not load route maps for the export, attaching none: {e}");
            return Vec::new();
        }
    };

    let mut pages: Vec<RouteMapPage> = Vec::new();
    for (row_number, trip_id) in rows {
        let Some(map) = maps.get(trip_id) else {
            continue;
        };

        // `decode` never panics; malformed geometry simply yields no points,
        // which `render_route` reports as an error rather than drawing blank.
        let points = decode(&map.polyline);
        match render_route(tiles, &points, MAP_WIDTH, MAP_HEIGHT).await {
            Ok(png) => pages.push(RouteMapPage {
                attachment_no: pages.len() + 1,
                row_number: *row_number,
                png_base64: base64::engine::general_purpose::STANDARD.encode(png),
            }),
            Err(e) => log::warn!(
                "Skipping the route map attachment for record {row_number} (trip {trip_id}): {e}"
            ),
        }
    }

    pages
}

// ---------------------------------------------------------------------------
// Route mode decision (Task 72, Phase 2)
// ---------------------------------------------------------------------------

/// Loop when the row names the same place twice, direct otherwise.
///
/// Compared after `places::normalise`, the same function the book keys on, so a
/// row cannot be direct-mode here and collide onto one book entry there.
fn mode_for(origin: &str, destination: &str) -> Result<RouteMode, String> {
    let origin_key = normalise(origin);
    let destination_key = normalise(destination);
    if origin_key.is_empty() || destination_key.is_empty() {
        return Err("A trip needs both an origin and a destination".to_string());
    }

    if origin_key == destination_key {
        Ok(RouteMode::Loop)
    } else {
        Ok(RouteMode::Direct)
    }
}

/// The book's entry for `name`, or `None` when a human has not yet confirmed a
/// coordinate for it (or no trip has ever named it at all).
fn placed_endpoint(places: &[Place], name: &str) -> Option<Place> {
    let key = normalise(name);
    places
        .iter()
        .find(|p| p.normalised_name == key && p.lat.is_some() && p.lon.is_some())
        .cloned()
}

/// The map view's entry point, and `mode_for`'s only caller.
///
/// Synchronous and network-free: the endpoints are a database lookup against the
/// place book, not a geocode. That is what the book bought -- see ADR-032.
pub fn start_route_for_trip_internal(
    db: &Database,
    trip_id: String,
) -> Result<RouteStart, String> {
    let trip = db
        .get_trip(&trip_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Trip not found: {trip_id}"))?;

    let mode = mode_for(&trip.origin, &trip.destination)?;

    // The book's own read path, so there is one place that knows how a trip's
    // spelling becomes a book entry (list_places_internal folds spellings and
    // joins the stored coordinate).
    let places = list_places_internal(db)?;

    Ok(RouteStart {
        mode,
        origin: placed_endpoint(&places, &trip.origin),
        destination: placed_endpoint(&places, &trip.destination),
    })
}

// ---------------------------------------------------------------------------
// Waypoint insertion placement (Task 7, route maps V2)
// ---------------------------------------------------------------------------

/// Place a dragged-in point into an ordered waypoint list.
///
/// The polyline is the geometry the point was dragged off, so its vertices
/// give the ordering that matters: every existing waypoint lies on the line,
/// so mapping each to its nearest vertex yields the leg boundaries, and the
/// new point's nearest vertex says which leg it came from.
///
/// Comparing squared degrees rather than true distances is deliberate -- over a
/// single route's extent the distortion cannot reorder two candidates, and
/// nothing here needs a distance, only an argmin.
///
/// Never returns a list with a new first or last element: a drag must not
/// silently move where the journey began or ended.
pub fn insert_waypoint(
    waypoints: &[Waypoint],
    polyline: &str,
    lat: f64,
    lon: f64,
) -> Vec<Waypoint> {
    let new_point = Waypoint { lat, lon, name: None, node_idx: None };
    let mut out = waypoints.to_vec();

    // Fewer than two waypoints is not a route; appending is the only sane act.
    if out.len() < 2 {
        out.push(new_point);
        return out;
    }

    let points = decode(polyline);
    // No usable geometry: put it immediately before the destination, the one
    // slot that is always valid.
    if points.len() < 2 {
        out.insert(out.len() - 1, new_point);
        return out;
    }

    let nearest = |lat: f64, lon: f64| -> usize {
        points
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (a.0 - lat).powi(2) + (a.1 - lon).powi(2);
                let db = (b.0 - lat).powi(2) + (b.1 - lon).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    };

    let new_vertex = nearest(lat, lon);
    // The first slot a new point may take is 1, the last is len()-1.
    let mut slot = out.len() - 1;
    for (i, wp) in out.iter().enumerate().skip(1) {
        if new_vertex <= nearest(wp.lat, wp.lon) {
            slot = i;
            break;
        }
    }
    let slot = slot.clamp(1, out.len() - 1);

    out.insert(slot, new_point);
    out
}

#[cfg(test)]
#[path = "route_maps_tests.rs"]
mod tests;
