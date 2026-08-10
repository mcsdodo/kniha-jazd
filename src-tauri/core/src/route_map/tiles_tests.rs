use super::tiles::*;
use std::path::Path;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Every file anywhere under `dir`, relative paths, sorted. A missing directory
/// counts as empty — never having created it is as good as having created
/// nothing in it.
fn files_under(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, base, out);
            } else {
                out.push(
                    path.strip_prefix(base)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[test]
fn projects_known_coordinates_to_known_tiles() {
    // Null Island at zoom 1 is tile (1, 1).
    assert_eq!(tile_xy(0.0, 0.0, 1), (1, 1));
    // Home base at zoom 12 — precomputed reference:
    //   x = (20.553 + 180) / 360 * 4096          = 2281.847… -> 2281
    //   y = (1 - ln(tan φ + sec φ) / π) / 2 * 4096 = 1407.783… -> 1407
    let (x, y) = tile_xy(48.935, 20.553, 12);
    assert_eq!((x, y), (2281, 1407));
}

#[test]
fn picks_the_largest_zoom_that_fits_the_bounds() {
    let bounds = Bounds {
        min_lat: 48.85,
        max_lat: 49.05,
        min_lon: 20.40,
        max_lon: 20.75,
    };
    let z = pick_zoom(&bounds, 1400, 900);
    assert!((9..=14).contains(&z), "unexpected zoom {z}");
    assert!(fits(&bounds, z, 1400, 900));
    assert!(
        !fits(&bounds, z + 1, 1400, 900),
        "z+1 must overflow the canvas"
    );
}

#[test]
fn bounds_from_points_covers_every_point() {
    let pts = [(48.90, 20.50), (49.02, 20.71), (48.87, 20.62)];
    let b = bounds_from_points(&pts);
    assert!(pts.iter().all(|&(lat, lon)| {
        lat >= b.min_lat && lat <= b.max_lat && lon >= b.min_lon && lon <= b.max_lon
    }));
}

#[test]
fn a_single_point_yields_a_valid_zoom() {
    // Degenerate bounds must not divide by zero or loop forever.
    let b = Bounds {
        min_lat: 48.9,
        max_lat: 48.9,
        min_lon: 20.5,
        max_lon: 20.5,
    };
    assert!(pick_zoom(&b, 1400, 900) <= MAX_ZOOM);
}

#[test]
fn grid_covers_the_bounds_inclusively() {
    let b = Bounds {
        min_lat: 48.85,
        max_lat: 49.05,
        min_lon: 20.40,
        max_lon: 20.75,
    };
    let z = 11;
    let grid = TileGrid::for_bounds(&b, z);
    let (x_nw, y_nw) = tile_xy(b.max_lat, b.min_lon, z); // north-west corner
    let (x_se, y_se) = tile_xy(b.min_lat, b.max_lon, z); // south-east corner
    assert!(grid.min_x <= x_nw && grid.max_x >= x_se);
    assert!(grid.min_y <= y_nw && grid.max_y >= y_se);
}

/// Web Mercator is undefined at the poles and wraps at the antimeridian; every
/// coordinate must still land on a real tile rather than a saturated `u32`.
#[test]
fn extreme_coordinates_stay_inside_the_tile_grid() {
    let z = 3;
    let last = (1u32 << z) - 1;
    for &(lat, lon) in &[
        (90.0, 180.0),
        (-90.0, 180.0),
        (89.9, -180.0),
        (-89.9, 179.999_999),
        (f64::NAN, f64::NAN),
        (f64::INFINITY, f64::NEG_INFINITY),
    ] {
        let (x, y) = tile_xy(lat, lon, z);
        assert!(x <= last && y <= last, "({lat}, {lon}) -> ({x}, {y})");
    }
    // The poles clamp to the top and bottom rows, not past them.
    assert_eq!(tile_xy(90.0, 0.0, z).1, 0);
    assert_eq!(tile_xy(-90.0, 0.0, z).1, last);
    // The antimeridian belongs to the last column, not to a phantom column n.
    assert_eq!(tile_xy(0.0, 180.0, z).0, last);
}

/// Pixels are measured from the grid's north-west corner, so the corners of the
/// bounds must land in the first and last tile of the rendered canvas.
#[test]
fn pixels_are_relative_to_the_grids_north_west_corner() {
    let b = Bounds {
        min_lat: 48.85,
        max_lat: 49.05,
        min_lon: 20.40,
        max_lon: 20.75,
    };
    let grid = TileGrid::for_bounds(&b, 11);
    let tile = TILE_SIZE as f32;
    assert_eq!(grid.width_px(), (grid.max_x - grid.min_x + 1) * TILE_SIZE);
    assert_eq!(grid.height_px(), (grid.max_y - grid.min_y + 1) * TILE_SIZE);

    let (nw_x, nw_y) = project_to_pixel(b.max_lat, b.min_lon, &grid);
    assert!(
        (0.0..tile).contains(&nw_x) && (0.0..tile).contains(&nw_y),
        "north-west corner must fall inside the first tile, got ({nw_x}, {nw_y})"
    );

    let (se_x, se_y) = project_to_pixel(b.min_lat, b.max_lon, &grid);
    assert!(
        se_x > nw_x && se_y > nw_y,
        "x must grow eastwards and y southwards"
    );
    assert!(
        se_x < grid.width_px() as f32 && se_y < grid.height_px() as f32,
        "south-east corner must fall inside the canvas, got ({se_x}, {se_y})"
    );
    assert!(
        se_x >= grid.width_px() as f32 - tile && se_y >= grid.height_px() as f32 - tile,
        "south-east corner must fall inside the last tile"
    );
}

// ---------------------------------------------------------------------------
// Tile fetching and caching
//
// Every test below runs against a `wiremock` server and a `tempdir` cache —
// nothing here touches tile.openstreetmap.org or any real cache directory.
// ---------------------------------------------------------------------------

/// OSM's tile usage policy makes caching mandatory, not an optimisation: a tile
/// is immutable for a given (z, x, y), so a second render must never re-ask.
#[tokio::test]
async fn a_cached_tile_is_not_refetched() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PNGBYTES".to_vec()))
        .expect(1)
        .mount(&server)
        .await;

    let cache = tempfile::tempdir().expect("tempdir");
    let fetcher = CachedTileFetcher::new(cache.path().to_path_buf(), server.uri());

    let first = fetcher.tile(12, 2281, 1407).await.expect("first fetch");
    let second = fetcher.tile(12, 2281, 1407).await.expect("second fetch");

    assert_eq!(first, b"PNGBYTES".to_vec());
    assert_eq!(second, first, "the cached tile must be byte-identical");

    let requests = server
        .received_requests()
        .await
        .expect("wiremock records requests by default");
    assert_eq!(
        requests.len(),
        1,
        "the second call must be served from the cache, not the network"
    );
}

/// OSM blocks clients that do not identify themselves. A missing or generic
/// User-Agent gets the whole application banned, not just one request.
#[tokio::test]
async fn sends_an_identifying_user_agent() {
    let server = MockServer::start().await;
    // Deliberately permissive: assert on the request we RECORDED rather than
    // letting a header matcher turn a wrong User-Agent into an opaque 404.
    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PNGBYTES".to_vec()))
        .mount(&server)
        .await;

    let cache = tempfile::tempdir().expect("tempdir");
    let fetcher = CachedTileFetcher::new(cache.path().to_path_buf(), server.uri());
    fetcher.tile(3, 4, 5).await.expect("fetch should succeed");

    let requests = server.received_requests().await.expect("recorded requests");
    assert_eq!(requests.len(), 1);
    let agent = requests[0]
        .headers
        .get("user-agent")
        .map(|v| v.to_str().unwrap_or_default().to_string())
        .unwrap_or_default();
    assert!(
        agent.contains("kniha-jazd"),
        "User-Agent must name this application, got: {agent:?}"
    );
}

/// A cached error body would poison every later render of that tile until the
/// user found and deleted the cache directory by hand. Nothing but a 2xx body
/// may ever reach the disk.
#[tokio::test]
async fn an_empty_body_is_not_written_to_the_cache() {
    // A 200 carrying zero bytes is not a tile. Caching it poisons the square
    // exactly the way a cached error body would, just through a different door,
    // and renders blank on every future export until someone finds the cache.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(Vec::<u8>::new()))
        .mount(&server)
        .await;

    let cache = tempfile::tempdir().expect("tempdir");
    let fetcher = CachedTileFetcher::new(cache.path().to_path_buf(), server.uri());

    fetcher
        .tile(12, 2281, 1407)
        .await
        .expect_err("an empty body must not be reported as a tile");

    assert_eq!(
        files_under(cache.path()),
        Vec::<String>::new(),
        "an empty body must leave nothing behind, not even a partial file"
    );
}

#[tokio::test]
async fn a_failed_tile_is_not_written_to_the_cache() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream exploded"))
        .mount(&server)
        .await;

    let cache = tempfile::tempdir().expect("tempdir");
    let fetcher = CachedTileFetcher::new(cache.path().to_path_buf(), server.uri());

    let err = fetcher
        .tile(12, 2281, 1407)
        .await
        .expect_err("a 500 must not be reported as a tile");
    assert!(
        err.contains("500"),
        "the error should name the HTTP status, got: {err}"
    );

    assert_eq!(
        files_under(cache.path()),
        Vec::<String>::new(),
        "a failed fetch must leave no file anywhere under the cache directory"
    );
}

/// The cache is on disk, not in the process: a later export — or a later run of
/// the application entirely — must reuse what an earlier one downloaded.
#[tokio::test]
async fn a_cached_tile_survives_a_new_fetcher_instance() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PNGBYTES".to_vec()))
        .mount(&server)
        .await;

    let cache = tempfile::tempdir().expect("tempdir");
    let first = CachedTileFetcher::new(cache.path().to_path_buf(), server.uri());
    let bytes = first.tile(12, 2281, 1407).await.expect("first fetch");
    drop(first);

    // From here on the server can only fail, so anything returned came from disk.
    server.reset().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/\d+/\d+/\d+\.png$"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let second = CachedTileFetcher::new(cache.path().to_path_buf(), server.uri());
    let cached = second
        .tile(12, 2281, 1407)
        .await
        .expect("a tile cached on disk must survive a new fetcher");
    assert_eq!(cached, bytes);

    let requests = server.received_requests().await.expect("recorded requests");
    assert!(
        requests.is_empty(),
        "the new fetcher must not have gone to the network at all"
    );
}
