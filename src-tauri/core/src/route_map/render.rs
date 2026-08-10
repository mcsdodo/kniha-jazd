//! Rasterises a saved route over OSM tiles into a PNG.
//!
//! This runs at export time only: the PNG is base64-embedded into the printed
//! logbook. The interactive map in the application is Leaflet in the browser and
//! never comes through here.
//!
//! Two things are deliberately absent:
//!
//! * **Text.** `tiny-skia` renders none, and none is needed — OSM's required
//!   attribution is a caption in the export HTML, next to the image, not baked
//!   into its pixels. Do not add a text rasteriser to put it back.
//! * **Failure.** Only an empty point list is an error. A tile that cannot be
//!   fetched or decoded is skipped, because a whole logbook export failing over
//!   an unreachable tile server would be a far worse outcome than a route drawn
//!   on a blank background.

use super::tiles::{
    bounds_from_points, pick_zoom, project_to_pixel, Bounds, TileFetcher, TileGrid, TILE_SIZE,
};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform,
};

/// OpenStreetMap's land colour. Painted before anything else so a tile that
/// never arrives reads as an unobtrusive blank rather than a black hole.
const LAND: (u8, u8, u8) = (0xf2, 0xef, 0xe9);

/// The route stroke, `#0066cc` — dark enough to stay legible when the logbook is
/// printed in greyscale.
const ROUTE: (u8, u8, u8) = (0x00, 0x66, 0xcc);

/// Opaque `tiny_skia` colour from one of the constants above.
fn opaque(rgb: (u8, u8, u8)) -> Color {
    Color::from_rgba8(rgb.0, rgb.1, rgb.2, 255)
}

/// Stroke thickness in pixels.
const STROKE_WIDTH: f32 = 5.0;

/// How many tiles are fetched at once. OSM's tile usage policy asks for no more
/// than two concurrent connections, so this is a limit rather than a tuning
/// knob — raising it risks getting the application blocked outright.
const PARALLEL_FETCHES: usize = 2;

// There is deliberately no edge margin.
//
// A route that exactly fills the page can put an end point on pixel zero and
// have its round cap ([`STROKE_WIDTH`] / 2) sliced. Insetting the fit box to
// avoid that looks like the obvious fix and is a trap: the inset reaches only
// `pick_zoom`, and zoom levels are powers of two, so the effect is discrete —
// it either changes nothing, or drops a whole level and renders the map at
// *half* scale. It never yields a small inset. So for exactly the routes it
// would "protect", it halves the map; a margin only narrows the window in
// which that happens. Losing 2.5 px of a round cap at the sheet edge is
// imperceptible in print; wasting half the page is not.

/// Render `points` as a route over OSM tiles, returning PNG bytes.
///
/// The result is always exactly `width` x `height`, with the route centred and
/// the basemap carried out to all four edges. Note that the tile grid is chosen
/// to cover the *canvas*, not the route's bounding box: a route far wider than
/// it is tall fits at a zoom whose bounds fill barely half the image, and tiling
/// only those bounds would leave the rest of the page blank.
///
/// Fails only when `points` is empty, or when the requested size is one no
/// pixmap can hold.
pub async fn render_route(
    tiles: &dyn TileFetcher,
    points: &[(f64, f64)],
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    if points.is_empty() {
        return Err("Cannot render a route map: the route has no coordinates.".to_string());
    }

    let mut canvas = Pixmap::new(width, height).ok_or_else(|| {
        format!("Cannot render a route map at {width}x{height} pixels: unusable image size.")
    })?;
    canvas.fill(opaque(LAND));

    let bounds = bounds_from_points(points);
    let zoom = pick_zoom(&bounds, width, height);
    let (grid, offset_x, offset_y) = viewport(&bounds, zoom, width, height);

    draw_tiles(&mut canvas, tiles, &grid, offset_x, offset_y).await;
    draw_route(&mut canvas, points, &grid, offset_x, offset_y);

    canvas
        .encode_png()
        .map_err(|e| format!("Could not encode the route map as a PNG: {e}"))
}

/// The tile grid covering the whole canvas, and where its north-west corner
/// lands on that canvas.
///
/// Built by placing the canvas over the world at `zoom`, centred on the route,
/// and asking which tiles it overlaps — rather than by tiling the route's bounds
/// and hoping they fill the page. The offset is normally negative, since the
/// grid overhangs the canvas by up to a tile on each side.
fn viewport(bounds: &Bounds, zoom: u8, width: u32, height: u32) -> (TileGrid, f32, f32) {
    // `project_to_pixel` is relative to a grid's corner, so go through a grid
    // built from the bounds and add its origin back to get absolute world pixels.
    let base = TileGrid::for_bounds(bounds, zoom);
    let tile = f64::from(TILE_SIZE);
    let (x_nw, y_nw) = project_to_pixel(bounds.max_lat, bounds.min_lon, &base);
    let (x_se, y_se) = project_to_pixel(bounds.min_lat, bounds.max_lon, &base);
    let centre_x = f64::from(base.min_x) * tile + f64::from(x_nw + x_se) / 2.0;
    let centre_y = f64::from(base.min_y) * tile + f64::from(y_nw + y_se) / 2.0;

    // The canvas as a rectangle in absolute world pixels, centred on the route.
    // Rounded to whole pixels so tiles composite without resampling.
    let left = round_finite(centre_x - f64::from(width) / 2.0);
    let top = round_finite(centre_y - f64::from(height) / 2.0);

    let grid = TileGrid {
        zoom,
        min_x: tile_index(left, zoom),
        max_x: tile_index(left + f64::from(width) - 1.0, zoom),
        min_y: tile_index(top, zoom),
        max_y: tile_index(top + f64::from(height) - 1.0, zoom),
    };

    (
        grid,
        (f64::from(grid.min_x) * tile - left) as f32,
        (f64::from(grid.min_y) * tile - top) as f32,
    )
}

/// `value` rounded to a whole pixel; a non-finite projection becomes 0 rather
/// than a NaN offset that would silently drop every tile.
fn round_finite(value: f64) -> f64 {
    if value.is_finite() {
        value.round()
    } else {
        0.0
    }
}

/// Tile index containing an absolute world pixel, clamped to the tiles that
/// actually exist at `zoom`. A canvas hanging off the edge of the world gets
/// the background colour there, not a phantom tile.
fn tile_index(world_px: f64, zoom: u8) -> u32 {
    let last = (1u32 << zoom.min(30)) - 1;
    let index = (world_px / f64::from(TILE_SIZE)).floor();
    if !index.is_finite() || index <= 0.0 {
        0
    } else if index >= f64::from(last) {
        last
    } else {
        index as u32
    }
}

/// One tile's coordinates paired with whatever the fetcher had to say about it.
type FetchedTile = ((u32, u32), Result<Vec<u8>, String>);

/// Composite every tile of the grid onto the canvas, skipping the ones that fail.
async fn draw_tiles(
    canvas: &mut Pixmap,
    tiles: &dyn TileFetcher,
    grid: &TileGrid,
    offset_x: f32,
    offset_y: f32,
) {
    let coords: Vec<(u32, u32)> = (grid.min_y..=grid.max_y)
        .flat_map(|y| (grid.min_x..=grid.max_x).map(move |x| (x, y)))
        .collect();

    for chunk in coords.chunks(PARALLEL_FETCHES) {
        let mut fetched: Vec<FetchedTile> = Vec::with_capacity(chunk.len());
        match chunk {
            [a, b] => {
                let (first, second) = tokio::join!(
                    tiles.tile(grid.zoom, a.0, a.1),
                    tiles.tile(grid.zoom, b.0, b.1)
                );
                fetched.push((*a, first));
                fetched.push((*b, second));
            }
            // A trailing chunk of one — and, vacuously, an empty one.
            rest => {
                for &(x, y) in rest {
                    fetched.push(((x, y), tiles.tile(grid.zoom, x, y).await));
                }
            }
        }

        for ((x, y), result) in fetched {
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(e) => {
                    log::warn!("Skipping map tile {}/{x}/{y}: {e}", grid.zoom);
                    continue;
                }
            };
            // Never unwrap here: a truncated download or a corrupted cache entry
            // arrives as perfectly valid `Ok(bytes)` and must not panic an export.
            let tile = match Pixmap::decode_png(&bytes) {
                Ok(tile) => tile,
                Err(e) => {
                    log::warn!("Skipping unreadable map tile {}/{x}/{y}: {e}", grid.zoom);
                    continue;
                }
            };

            // i64 throughout: the multiplication would be the one place a wide
            // grid could wrap, and `draw_pixmap` clips whatever falls outside.
            let px = offset_x as i64 + i64::from(x - grid.min_x) * i64::from(TILE_SIZE);
            let py = offset_y as i64 + i64::from(y - grid.min_y) * i64::from(TILE_SIZE);

            canvas.draw_pixmap(
                clamp_to_i32(px),
                clamp_to_i32(py),
                tile.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }
}

fn clamp_to_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// Stroke the route on top of the tiles.
fn draw_route(
    canvas: &mut Pixmap,
    points: &[(f64, f64)],
    grid: &TileGrid,
    offset_x: f32,
    offset_y: f32,
) {
    let projected: Vec<(f32, f32)> = points
        .iter()
        .map(|&(lat, lon)| {
            let (x, y) = project_to_pixel(lat, lon, grid);
            (x + offset_x, y + offset_y)
        })
        .collect();
    let Some(&(first_x, first_y)) = projected.first() else {
        return;
    };

    let mut paint = Paint::default();
    paint.set_color(opaque(ROUTE));
    paint.anti_alias = true;

    // A route whose points all project to the same pixel — a single point, or a
    // stop-and-return at one place — has no length to stroke, and a zero-length
    // segment rasterises to nothing. Mark it with a dot instead of dropping the
    // only thing the map exists to show.
    let has_length = projected
        .iter()
        .any(|&(x, y)| (x - first_x).abs() >= 0.5 || (y - first_y).abs() >= 0.5);

    let path = if has_length {
        let mut builder = PathBuilder::new();
        builder.move_to(first_x, first_y);
        for &(x, y) in &projected[1..] {
            builder.line_to(x, y);
        }
        builder.finish()
    } else {
        None
    };

    match path {
        Some(path) => canvas.stroke_path(
            &path,
            &paint,
            &Stroke {
                width: STROKE_WIDTH,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        ),
        None => {
            if let Some(dot) = PathBuilder::from_circle(first_x, first_y, STROKE_WIDTH / 2.0) {
                canvas.fill_path(&dot, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
    }
}
