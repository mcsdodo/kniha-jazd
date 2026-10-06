use super::*;

#[test]
fn haversine_matches_the_fuelio_segment_column() {
    // Rows 1 and 2 of route-1790866465210.csv: Fuelio says 13.228 m.
    let d = haversine_m((48.915_648_2, 20.580_570_1), (48.915_549_8, 20.580_671_5));
    assert!((d - 13.228).abs() < 0.1, "got {d}");
}

#[test]
fn point_on_the_line_has_zero_distance() {
    let line = [(48.0, 20.0), (48.0, 20.1)];
    assert!(point_to_polyline_m((48.0, 20.05), &line) < 1.0);
}

#[test]
fn point_beside_a_segment_measures_the_perpendicular() {
    // 0.01 deg lat north of an east-west line: about 1112 m.
    let line = [(48.0, 20.0), (48.0, 20.1)];
    let d = point_to_polyline_m((48.01, 20.05), &line);
    assert!((d - 1112.0).abs() < 5.0, "got {d}");
}

#[test]
fn point_past_the_end_measures_to_the_end_point() {
    let line = [(48.0, 20.0), (48.0, 20.1)];
    let d = point_to_polyline_m((48.0, 20.11), &line);
    assert!((d - haversine_m((48.0, 20.1), (48.0, 20.11))).abs() < 2.0);
}

#[test]
fn empty_or_single_point_line() {
    assert_eq!(point_to_polyline_m((48.0, 20.0), &[]), f64::INFINITY);
    let d = point_to_polyline_m((48.0, 20.01), &[(48.0, 20.0)]);
    assert!((d - haversine_m((48.0, 20.0), (48.0, 20.01))).abs() < 1.0);
}

#[test]
fn off_route_share_counts_points_beyond_the_threshold() {
    let line = [(48.0, 20.0), (48.0, 20.1)];
    let track = [
        (48.0, 20.0),
        (48.0, 20.02),
        (48.02, 20.05), // about 2.2 km off
        (48.0, 20.1),
    ];
    assert!((off_route_share(&track, &line, 500.0) - 0.25).abs() < 1e-9);
    assert_eq!(off_route_share(&[], &line, 500.0), 0.0);
}
