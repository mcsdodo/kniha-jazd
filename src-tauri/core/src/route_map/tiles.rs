//! Web Mercator (slippy-map) tile geometry for route map rendering.
//!
//! This module is pure geometry — no I/O. It answers three questions for a set
//! of route coordinates: which tiles are needed, at what zoom, and where does
//! each coordinate land in pixels. Tile *fetching* and caching live elsewhere.
//!
//! Units, because mixing them up is the classic bug here:
//!
//! | Name          | Unit                                                      |
//! |---------------|-----------------------------------------------------------|
//! | `lat` / `lon` | WGS84 **degrees** (radians only appear inside `world_px`) |
//! | world pixels  | absolute pixels at a zoom: `TILE_SIZE * 2^zoom` per axis   |
//! | tile index    | world pixels / `TILE_SIZE`, floored; `0..2^zoom`           |
//! | canvas pixels | pixels relative to a [`TileGrid`]'s north-west corner      |

use std::f64::consts::PI;

/// Edge length of an OpenStreetMap raster tile, in pixels.
pub const TILE_SIZE: u32 = 256;

/// Highest zoom [`pick_zoom`] will ever return. OSM tiles exist beyond this,
/// but a logbook route map gains nothing from more detail.
pub const MAX_ZOOM: u8 = 17;

/// Latitude where Web Mercator is cut off (degrees). The projection diverges at
/// the poles — `tan(90°)` is infinite — so every latitude is clamped to this.
pub const MAX_MERCATOR_LAT: f64 = 85.051_128_779_806_59;

/// Guards `2^zoom` against overflowing the `u32` tile-index space. Zooms this
/// deep are nonsense for a map, but a caller must not be able to wrap the math.
const MAX_SAFE_ZOOM: u8 = 30;

/// A geographic bounding box in degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}

/// Smallest box covering every `(lat, lon)` point, in degrees.
///
/// Non-finite coordinates are ignored; an empty (or entirely non-finite) input
/// yields a zeroed box, which still projects and renders as a single point.
pub fn bounds_from_points(points: &[(f64, f64)]) -> Bounds {
    let mut bounds = Bounds {
        min_lat: f64::INFINITY,
        max_lat: f64::NEG_INFINITY,
        min_lon: f64::INFINITY,
        max_lon: f64::NEG_INFINITY,
    };
    let mut seen = false;

    for &(lat, lon) in points {
        if !lat.is_finite() || !lon.is_finite() {
            continue;
        }
        seen = true;
        bounds.min_lat = bounds.min_lat.min(lat);
        bounds.max_lat = bounds.max_lat.max(lat);
        bounds.min_lon = bounds.min_lon.min(lon);
        bounds.max_lon = bounds.max_lon.max(lon);
    }

    if seen {
        bounds
    } else {
        Bounds {
            min_lat: 0.0,
            max_lat: 0.0,
            min_lon: 0.0,
            max_lon: 0.0,
        }
    }
}

/// Slippy-map tile containing this coordinate at the given zoom.
///
/// The result is always a real tile: latitude is clamped to the Web Mercator
/// range, longitude to `±180°`, non-finite input falls back to `(0°, 0°)`, and
/// the index is clamped to `0..=2^zoom - 1` (so `lon = 180°` belongs to the
/// last column rather than to a phantom one past the edge).
pub fn tile_xy(lat: f64, lon: f64, zoom: u8) -> (u32, u32) {
    let (x_px, y_px) = world_px(lat, lon, zoom);
    let tile = f64::from(TILE_SIZE);
    let last = last_tile_index(zoom);
    (
        to_tile_index(x_px / tile, last),
        to_tile_index(y_px / tile, last),
    )
}

/// True when the bounds fit inside a canvas of `width` x `height` px at this zoom.
pub fn fits(bounds: &Bounds, zoom: u8, width: u32, height: u32) -> bool {
    let (span_x, span_y) = pixel_span(bounds, zoom);
    span_x <= f64::from(width) && span_y <= f64::from(height)
}

/// Largest zoom (capped at [`MAX_ZOOM`]) whose rendering fits the canvas.
///
/// Degenerate bounds — a single point — fit at every zoom and therefore return
/// [`MAX_ZOOM`]. Bounds too large for the canvas even at zoom 0 return 0.
pub fn pick_zoom(bounds: &Bounds, width: u32, height: u32) -> u8 {
    (0..=MAX_ZOOM)
        .rev()
        .find(|&zoom| fits(bounds, zoom, width, height))
        .unwrap_or(0)
}

/// The inclusive rectangle of tiles covering some bounds at a fixed zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileGrid {
    pub zoom: u8,
    pub min_x: u32,
    pub max_x: u32,
    pub min_y: u32,
    pub max_y: u32,
}

impl TileGrid {
    /// Every tile touched by `bounds` at `zoom`, corners included.
    pub fn for_bounds(bounds: &Bounds, zoom: u8) -> Self {
        // North-west is (max_lat, min_lon): Mercator y grows southwards.
        let (x_nw, y_nw) = tile_xy(bounds.max_lat, bounds.min_lon, zoom);
        let (x_se, y_se) = tile_xy(bounds.min_lat, bounds.max_lon, zoom);
        Self {
            zoom,
            min_x: x_nw.min(x_se),
            max_x: x_nw.max(x_se),
            min_y: y_nw.min(y_se),
            max_y: y_nw.max(y_se),
        }
    }

    /// Canvas width in pixels: whole tiles, so a multiple of [`TILE_SIZE`].
    pub fn width_px(&self) -> u32 {
        (self.max_x - self.min_x + 1).saturating_mul(TILE_SIZE)
    }

    /// Canvas height in pixels: whole tiles, so a multiple of [`TILE_SIZE`].
    pub fn height_px(&self) -> u32 {
        (self.max_y - self.min_y + 1).saturating_mul(TILE_SIZE)
    }
}

/// Pixel position of a coordinate relative to the grid's north-west corner.
///
/// Coordinates outside the grid project to negative or overflowing values on
/// purpose — clipping is the renderer's decision, not the projection's.
pub fn project_to_pixel(lat: f64, lon: f64, grid: &TileGrid) -> (f32, f32) {
    let (x_px, y_px) = world_px(lat, lon, grid.zoom);
    let tile = f64::from(TILE_SIZE);
    let origin_x = f64::from(grid.min_x) * tile;
    let origin_y = f64::from(grid.min_y) * tile;
    ((x_px - origin_x) as f32, (y_px - origin_y) as f32)
}

/// Absolute world-pixel position of a coordinate at `zoom`.
///
/// Always finite and within `0..=world_size_px`, so callers may cast it.
fn world_px(lat: f64, lon: f64, zoom: u8) -> (f64, f64) {
    let world = world_size_px(zoom);
    let lat_rad = sanitize_lat(lat).to_radians();
    let lon = sanitize_lon(lon);

    let x = (lon + 180.0) / 360.0 * world;
    let y = (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / PI) / 2.0 * world;

    (x.clamp(0.0, world), y.clamp(0.0, world))
}

/// Width and height of the whole world in pixels at `zoom`.
fn world_size_px(zoom: u8) -> f64 {
    tiles_per_axis(zoom) * f64::from(TILE_SIZE)
}

/// `2^zoom`, saturated at [`MAX_SAFE_ZOOM`].
fn tiles_per_axis(zoom: u8) -> f64 {
    f64::from(1u32 << zoom.min(MAX_SAFE_ZOOM))
}

/// Highest valid tile index on either axis at `zoom`.
fn last_tile_index(zoom: u8) -> u32 {
    (1u32 << zoom.min(MAX_SAFE_ZOOM)) - 1
}

/// Width and height of `bounds` in pixels at `zoom`.
fn pixel_span(bounds: &Bounds, zoom: u8) -> (f64, f64) {
    let (x_nw, y_nw) = world_px(bounds.max_lat, bounds.min_lon, zoom);
    let (x_se, y_se) = world_px(bounds.min_lat, bounds.max_lon, zoom);
    ((x_se - x_nw).abs(), (y_se - y_nw).abs())
}

/// Degrees of latitude, clamped to the range Web Mercator can represent.
fn sanitize_lat(lat: f64) -> f64 {
    if lat.is_finite() {
        lat.clamp(-MAX_MERCATOR_LAT, MAX_MERCATOR_LAT)
    } else {
        0.0
    }
}

/// Degrees of longitude, clamped to the world's edges.
fn sanitize_lon(lon: f64) -> f64 {
    if lon.is_finite() {
        lon.clamp(-180.0, 180.0)
    } else {
        0.0
    }
}

/// Floors a tile coordinate into `0..=last`, refusing to saturate silently.
fn to_tile_index(value: f64, last: u32) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    let floored = value.floor();
    if floored <= 0.0 {
        0
    } else if floored >= f64::from(last) {
        last
    } else {
        floored as u32
    }
}
