// `mod polyline_tests` is declared in `route_map/mod.rs`, so `super` is `route_map`.
use super::polyline::*;

/// Canonical example from Google's polyline algorithm documentation:
/// (38.5, -120.2), (40.7, -120.95), (43.252, -126.453).
const REFERENCE: &str = "_p~iF~ps|U_ulLnnqC_mqNvxq`@";

#[test]
fn decodes_the_reference_polyline() {
    let pts = decode(REFERENCE);
    assert_eq!(pts.len(), 3);
    assert!((pts[0].0 - 38.5).abs() < 1e-5);
    assert!((pts[0].1 - -120.2).abs() < 1e-5);
    assert!((pts[2].0 - 43.252).abs() < 1e-5);
}

#[test]
fn encodes_the_reference_polyline_byte_for_byte() {
    // Round-tripping only proves encode and decode agree with each other, so a
    // self-consistently non-standard codec would pass it while producing
    // geometry OSRM and Leaflet cannot read. This pins the wire format itself.
    let pts = [(38.5, -120.2), (40.7, -120.95), (43.252, -126.453)];
    assert_eq!(encode(&pts), REFERENCE);
}

#[test]
fn round_trips_within_precision() {
    let pts = vec![(48.935, 20.553), (48.9973, 20.5911), (48.935, 20.553)];
    let decoded = decode(&encode(&pts));
    for (a, b) in pts.iter().zip(decoded.iter()) {
        assert!((a.0 - b.0).abs() < 1e-5 && (a.1 - b.1).abs() < 1e-5);
    }
}

#[test]
fn decoding_garbage_yields_no_points() {
    assert!(decode("!!!not-a-polyline").is_empty());
}

#[test]
fn empty_input_is_a_no_op_in_both_directions() {
    assert!(decode("").is_empty());
    assert_eq!(encode(&[]), "");
}

/// A response truncated by a dropped connection ends mid-chunk: the third
/// point's latitude is complete but its longitude is not. The decoder must
/// keep the clean prefix and not panic reaching past the end.
#[test]
fn truncated_chunk_keeps_clean_prefix() {
    // 18 bytes = two complete points, + "_mqN" (lat #3), + "v" (lon #3, cut off).
    let truncated = &REFERENCE[..23];
    let pts = decode(truncated);
    assert_eq!(pts.len(), 2);
    assert!((pts[1].0 - 40.7).abs() < 1e-5);
}
