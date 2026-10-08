//! Distances between GPS points, in metres. Points are `(lat, lon)` degrees.

const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Great-circle distance.
pub fn haversine_m(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (lat1, lat2) = (a.0.to_radians(), b.0.to_radians());
    let dlat = lat2 - lat1;
    let dlon = (b.1 - a.1).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * h.sqrt().asin()
}

/// Shortest distance from `p` to the line through `line`'s points.
/// An empty line is infinitely far.
///
/// Projects onto a flat plane around `p` (equirectangular). The error is
/// small at the scale of a route check (a few km around `p`).
pub fn point_to_polyline_m(p: (f64, f64), line: &[(f64, f64)]) -> f64 {
    match line {
        [] => f64::INFINITY,
        [only] => haversine_m(p, *only),
        _ => {
            let k_lat = EARTH_RADIUS_M.to_radians();
            let k_lon = k_lat * p.0.to_radians().cos();
            let xy = |q: (f64, f64)| ((q.1 - p.1) * k_lon, (q.0 - p.0) * k_lat);
            line.windows(2)
                .map(|w| segment_to_origin(xy(w[0]), xy(w[1])))
                .fold(f64::INFINITY, f64::min)
        }
    }
}

/// Distance from the origin to the segment a-b, in the plane.
fn segment_to_origin(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (-(a.0 * dx + a.1 * dy) / len2).clamp(0.0, 1.0)
    };
    let (x, y) = (a.0 + t * dx, a.1 + t * dy);
    (x * x + y * y).sqrt()
}

/// The share (0 to 1) of `track` points further than `threshold_m` from `line`.
pub fn off_route_share(track: &[(f64, f64)], line: &[(f64, f64)], threshold_m: f64) -> f64 {
    if track.is_empty() {
        return 0.0;
    }
    let off = track
        .iter()
        .filter(|p| point_to_polyline_m(**p, line) > threshold_m)
        .count();
    off as f64 / track.len() as f64
}

#[cfg(test)]
#[path = "geo_tests.rs"]
mod tests;
