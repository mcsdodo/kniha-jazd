use super::render::*;
use super::tiles::{TileFetcher, TILE_SIZE};
use tiny_skia::{Color, Pixmap};

/// The canvas the export uses. Fixed here so every test asserts the same thing.
const WIDTH: u32 = 1400;
const HEIGHT: u32 = 900;

/// The stroke colour `render_route` paints the route with (`#0066cc`).
const ROUTE_RGB: (u8, u8, u8) = (0x00, 0x66, 0xcc);

/// OSM's land colour, painted where no tile arrived (`#f2efe9`).
const LAND_RGB: (u8, u8, u8) = (0xf2, 0xef, 0xe9);

/// A colour no part of the renderer can produce by itself, so finding it in the
/// output proves a tile was composited rather than guessed at.
const TILE_RGB: (u8, u8, u8) = (0xff, 0x00, 0xff);

/// A short route through eastern Slovakia — three distinct points, enough to
/// need several tiles and to produce a stroke with a join in it.
fn route() -> Vec<(f64, f64)> {
    vec![(48.85, 20.40), (48.95, 20.55), (49.05, 20.75)]
}

/// A flat-colour tile PNG, synthesised in memory.
///
/// Deliberately not a fixture file: a checked-in binary would have to be trusted
/// on sight, and the renderer cares about nothing in a real tile but its pixels.
fn flat_tile(rgb: (u8, u8, u8)) -> Vec<u8> {
    let (r, g, b) = rgb;
    let mut tile = Pixmap::new(TILE_SIZE, TILE_SIZE).expect("tile pixmap");
    tile.fill(Color::from_rgba8(r, g, b, 255));
    tile.encode_png().expect("encode tile")
}

/// Serves the same flat tile for every coordinate.
struct FlatTiles(Vec<u8>);

impl FlatTiles {
    fn new(rgb: (u8, u8, u8)) -> Self {
        Self(flat_tile(rgb))
    }
}

#[async_trait::async_trait]
impl TileFetcher for FlatTiles {
    async fn tile(&self, _z: u8, _x: u32, _y: u32) -> Result<Vec<u8>, String> {
        Ok(self.0.clone())
    }
}

/// The offline case: every tile fails.
struct NoTiles;

#[async_trait::async_trait]
impl TileFetcher for NoTiles {
    async fn tile(&self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, String> {
        Err(format!("no route to the tile service for {z}/{x}/{y}"))
    }
}

/// Half the tiles arrive, half fail — the usual shape of a flaky connection.
struct FlakyTiles(Vec<u8>);

#[async_trait::async_trait]
impl TileFetcher for FlakyTiles {
    async fn tile(&self, z: u8, x: u32, y: u32) -> Result<Vec<u8>, String> {
        if (x + y) % 2 == 0 {
            Ok(self.0.clone())
        } else {
            Err(format!("tile {z}/{x}/{y} failed"))
        }
    }
}

/// Bytes that are not a PNG at all — a truncated download, a captive portal's
/// HTML, a corrupted cache entry.
struct CorruptTiles;

#[async_trait::async_trait]
impl TileFetcher for CorruptTiles {
    async fn tile(&self, _z: u8, _x: u32, _y: u32) -> Result<Vec<u8>, String> {
        Ok(b"<html>this is not a tile</html>".to_vec())
    }
}

fn decode(png: &[u8]) -> Pixmap {
    Pixmap::decode_png(png).expect("render_route must return a decodable PNG")
}

/// How many pixels sit within `tolerance` of `rgb` on every channel.
///
/// A tolerance rather than an exact match because the stroke is anti-aliased and
/// its edge pixels are blends; the core of the line is still the exact colour.
fn count_near(pixmap: &Pixmap, rgb: (u8, u8, u8), tolerance: u8) -> usize {
    let (r, g, b) = rgb;
    let close = |a: u8, b: u8| a.abs_diff(b) <= tolerance;
    pixmap
        .pixels()
        .iter()
        .map(|px| px.demultiply())
        .filter(|c| close(c.red(), r) && close(c.green(), g) && close(c.blue(), b))
        .count()
}

#[tokio::test]
async fn renders_a_png_of_the_requested_size() {
    let png = render_route(&FlatTiles::new(TILE_RGB), &route(), WIDTH, HEIGHT)
        .await
        .expect("render should succeed");

    let pixmap = decode(&png);
    // The tile grid is a multiple of 256 and almost never 1400x900; the renderer
    // must crop and centre rather than hand the export a ragged tile-multiple.
    assert_eq!((pixmap.width(), pixmap.height()), (WIDTH, HEIGHT));

    // A second, deliberately awkward size: small, portrait, and not a multiple
    // of the tile size in either axis. Asserting only ever at 1400x900 would be
    // satisfied by a renderer that hardcoded one pixmap size.
    let odd = render_route(&FlatTiles::new(TILE_RGB), &route(), 317, 511)
        .await
        .expect("render should succeed at an awkward size");
    let odd = decode(&odd);
    assert_eq!((odd.width(), odd.height()), (317, 511));
}

/// Bounding box `(x0, y0, x1, y1)` of everything painted in the route's colour,
/// or `None` if nothing was.
///
/// Extent, not just a pixel count: "some route-coloured pixels exist" is far too
/// weak a claim. The single-point fallback paints a 5px dot in exactly that
/// colour, so a renderer that drew only the dot and dropped the whole line would
/// satisfy a `> 0` count while producing a useless map.
fn route_bbox(pixmap: &Pixmap) -> Option<(u32, u32, u32, u32)> {
    let mut bbox: Option<(u32, u32, u32, u32)> = None;
    for y in 0..pixmap.height() {
        for x in 0..pixmap.width() {
            let c = pixmap.pixel(x, y).expect("in-bounds pixel").demultiply();
            let close = |a: u8, b: u8| a.abs_diff(b) <= 8;
            if close(c.red(), ROUTE_RGB.0)
                && close(c.green(), ROUTE_RGB.1)
                && close(c.blue(), ROUTE_RGB.2)
            {
                bbox = Some(match bbox {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
    }
    bbox
}

#[tokio::test]
async fn the_route_stroke_is_actually_drawn() {
    // The tiles are a colour the renderer cannot produce on its own, so every
    // pixel near #0066cc had to come from the stroke. Without this a silently
    // skipped stroke would still produce a perfectly valid PNG.
    let png = render_route(&FlatTiles::new(TILE_RGB), &route(), WIDTH, HEIGHT)
        .await
        .expect("render should succeed");

    let pixmap = decode(&png);
    let (x0, y0, x1, y1) = route_bbox(&pixmap).expect("no route stroke found in the rendered map");

    // The zoom is chosen to fit the route to the canvas, so a route that was
    // really stroked spans most of it. Half is a wide margin around that.
    assert!(
        x1 - x0 > WIDTH / 2 && y1 - y0 > HEIGHT / 2,
        "the route covers only {}x{} of a {WIDTH}x{HEIGHT} canvas — it was not stroked end to end",
        x1 - x0,
        y1 - y0
    );

    // And the tiles really were composited, not left as bare background.
    assert!(
        count_near(&pixmap, TILE_RGB, 0) > 0,
        "the fetched tiles were not drawn"
    );
}

#[tokio::test]
async fn renders_on_a_plain_background_when_every_tile_fails() {
    // The export-time offline fallback. A whole logbook export must not fail
    // because the tile server is unreachable — the route itself is the point,
    // and the basemap is decoration.
    let png = render_route(&NoTiles, &route(), WIDTH, HEIGHT)
        .await
        .expect("an unreachable tile service must not fail the render");

    let pixmap = decode(&png);
    assert_eq!((pixmap.width(), pixmap.height()), (WIDTH, HEIGHT));
    let (x0, y0, x1, y1) = route_bbox(&pixmap).expect("the route must still be drawn");
    assert!(
        x1 - x0 > WIDTH / 2 && y1 - y0 > HEIGHT / 2,
        "the whole route must be drawn without any tiles, not just part of it"
    );
    assert!(
        count_near(&pixmap, LAND_RGB, 0) > 0,
        "a missing tile should read as OSM's land colour, not as a black hole"
    );
}

#[tokio::test]
async fn a_partial_tile_failure_still_produces_a_png() {
    let png = render_route(&FlakyTiles(flat_tile(TILE_RGB)), &route(), WIDTH, HEIGHT)
        .await
        .expect("a partial tile failure must not fail the render");

    let pixmap = decode(&png);
    assert_eq!((pixmap.width(), pixmap.height()), (WIDTH, HEIGHT));
    assert!(
        count_near(&pixmap, TILE_RGB, 0) > 0,
        "the tiles that did arrive must be drawn"
    );
    let (x0, y0, x1, y1) = route_bbox(&pixmap).expect("the route must be drawn across the gaps");
    assert!(
        x1 - x0 > WIDTH / 2 && y1 - y0 > HEIGHT / 2,
        "the route must span the gaps, not stop at the first missing tile"
    );
}

/// A corrupt tile reaches the decoder as `Ok(bytes)`, so it cannot be caught by
/// the fetch error path. Unwrapping the decode would panic mid-export.
#[tokio::test]
async fn an_undecodable_tile_is_skipped_rather_than_panicking() {
    let png = render_route(&CorruptTiles, &route(), WIDTH, HEIGHT)
        .await
        .expect("a corrupt tile must be skipped, not fatal");

    let pixmap = decode(&png);
    assert_eq!((pixmap.width(), pixmap.height()), (WIDTH, HEIGHT));
    assert!(
        count_near(&pixmap, LAND_RGB, 0) > 0,
        "an undecodable tile should leave the land colour behind"
    );
}

#[tokio::test]
async fn an_empty_point_list_is_an_error_not_a_panic() {
    let result = render_route(&FlatTiles::new(TILE_RGB), &[], WIDTH, HEIGHT).await;
    assert!(
        result.is_err(),
        "a route with no coordinates has nothing to render"
    );
}

/// Degenerate bounds make `pick_zoom` return `MAX_ZOOM` and give the route a
/// zero-length path. Neither may divide by zero, produce a zero-sized canvas,
/// nor silently drop the only thing the map is meant to show.
#[tokio::test]
async fn a_single_point_route_is_marked_rather_than_dropped() {
    let png = render_route(
        &FlatTiles::new(TILE_RGB),
        &[(48.935, 20.553)],
        WIDTH,
        HEIGHT,
    )
    .await
    .expect("a one-point route must render");

    let pixmap = decode(&png);
    assert_eq!((pixmap.width(), pixmap.height()), (WIDTH, HEIGHT));
    assert!(
        route_bbox(&pixmap).is_some(),
        "a one-point route must still leave a visible mark"
    );
}

/// The tile grid covers the route's bounds, but the canvas is chosen for the
/// page, not for the route: a Bratislava–Košice route fits at a zoom whose
/// bounds occupy barely half the image. The basemap has to be extended to the
/// edges of the canvas, or the export is a small map adrift in blank paper.
#[tokio::test]
async fn the_basemap_covers_the_whole_canvas() {
    // Deliberately a route much wider than it is tall, so the vertical slack
    // between its bounds and the canvas is large.
    let wide = vec![(48.15, 17.11), (48.72, 19.13), (48.94, 21.24)];
    let png = render_route(&FlatTiles::new(TILE_RGB), &wide, WIDTH, HEIGHT)
        .await
        .expect("render should succeed");

    let pixmap = decode(&png);
    assert_eq!(
        count_near(&pixmap, LAND_RGB, 0),
        0,
        "every pixel should be covered by a tile when every tile is available"
    );
}

/// The route is not clipped at the image border.
///
/// Note what this does NOT pin: the renderer applies no margin, so the
/// clearance here is whatever slack the power-of-two zoom fit happens to leave
/// (about 6 px for this route). It would still pass if the route sat one pixel
/// from the edge. Insetting the fit box to guarantee clearance was tried and
/// removed — see the comment in render.rs: the inset only reaches `pick_zoom`,
/// so it cannot produce a small gap, only drop a whole zoom level and halve the
/// map. This asserts the outcome we care about (nothing sliced off), not a
/// mechanism.
#[tokio::test]
async fn the_route_is_not_clipped_at_the_border() {
    let png = render_route(&FlatTiles::new(TILE_RGB), &route(), WIDTH, HEIGHT)
        .await
        .expect("render should succeed");

    let pixmap = decode(&png);
    let (x0, y0, x1, y1) = route_bbox(&pixmap).expect("route should be drawn");
    const FRAME: u32 = 2;
    let in_frame = x0 < FRAME || y0 < FRAME || x1 >= WIDTH - FRAME || y1 >= HEIGHT - FRAME;
    assert!(
        !in_frame,
        "the route reaches the image border and is being clipped"
    );
}
