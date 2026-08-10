use super::tiles::*;

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
