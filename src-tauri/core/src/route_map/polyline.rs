//! Polyline5 codec — Google's encoded polyline algorithm at 1e-5 precision.
//!
//! OSRM returns route geometry as an encoded polyline5 string, which we store
//! verbatim in the database and decode when rendering. Both directions live
//! here; the codec is implemented by hand to avoid a dependency.
//!
//! Encoded input arrives from a third-party HTTP API and from the database, so
//! [`decode`] treats it as untrusted: it operates on raw bytes, guards every
//! arithmetic step, and truncates rather than panicking on malformed data.

/// Number of encoded value bits carried by a single character.
const CHUNK_BITS: u32 = 5;
/// Mask for the payload bits of a chunk.
const CHUNK_MASK: u8 = 0x1f;
/// Flag marking "another chunk follows".
const CONTINUATION: u8 = 0x20;
/// ASCII offset applied so chunks land in the printable range.
const ASCII_OFFSET: u8 = 63;
/// Fixed-point scale: five decimal places.
const SCALE: f64 = 1e5;

/// Decode an encoded polyline5 string into `(lat, lon)` pairs.
/// Never panics: malformed input yields whatever prefix parsed cleanly.
pub fn decode(encoded: &str) -> Vec<(f64, f64)> {
    let bytes = encoded.as_bytes();
    // A real point costs roughly ten bytes (two values of ~5 chunks); denser
    // input just regrows the vector.
    let mut points = Vec::with_capacity(bytes.len() / 10);
    let mut cursor = 0usize;
    let mut lat = 0i64;
    let mut lon = 0i64;

    while cursor < bytes.len() {
        // A point needs both halves; a dangling latitude is discarded.
        let Some(delta_lat) = next_delta(bytes, &mut cursor) else {
            break;
        };
        let Some(delta_lon) = next_delta(bytes, &mut cursor) else {
            break;
        };
        let Some(next_lat) = lat.checked_add(delta_lat) else {
            break;
        };
        let Some(next_lon) = lon.checked_add(delta_lon) else {
            break;
        };
        lat = next_lat;
        lon = next_lon;
        points.push((lat as f64 / SCALE, lon as f64 / SCALE));
    }

    points
}

/// Encode `(lat, lon)` pairs as a polyline5 string.
pub fn encode(points: &[(f64, f64)]) -> String {
    // Slovak-scale deltas run 1-6 chars per value; 12 bytes/point rarely regrows.
    let mut encoded = String::with_capacity(points.len() * 12);
    let mut prev_lat = 0i64;
    let mut prev_lon = 0i64;

    for &(lat, lon) in points {
        let fixed_lat = to_fixed(lat);
        let fixed_lon = to_fixed(lon);
        push_delta(&mut encoded, fixed_lat.wrapping_sub(prev_lat));
        push_delta(&mut encoded, fixed_lon.wrapping_sub(prev_lon));
        prev_lat = fixed_lat;
        prev_lon = fixed_lon;
    }

    encoded
}

/// Read one zig-zag encoded delta starting at `cursor`, advancing it past the
/// chunks consumed. Returns `None` — leaving `cursor` past the bad input — when
/// the value runs off the end of the buffer, contains a byte below the ASCII
/// offset, or claims more bits than an `i64` can hold.
fn next_delta(bytes: &[u8], cursor: &mut usize) -> Option<i64> {
    let mut zigzag = 0u64;
    let mut shift = 0u32;

    loop {
        let byte = *bytes.get(*cursor)?;
        *cursor += 1;
        // Anything below the offset is not polyline data at all.
        let chunk = byte.checked_sub(ASCII_OFFSET)?;
        if shift >= u64::BITS {
            return None;
        }
        zigzag |= u64::from(chunk & CHUNK_MASK) << shift;
        shift += CHUNK_BITS;
        if chunk & CONTINUATION == 0 {
            break;
        }
    }

    let magnitude = (zigzag >> 1) as i64;
    Some(if zigzag & 1 == 0 {
        magnitude
    } else {
        !magnitude
    })
}

/// Convert degrees to the fixed-point representation. `as` casts saturate and
/// map NaN to zero, so no input value can panic here.
fn to_fixed(degrees: f64) -> i64 {
    (degrees * SCALE).round() as i64
}

/// Append one zig-zag encoded delta.
fn push_delta(encoded: &mut String, delta: i64) {
    // `wrapping_mul` rather than `<< 1` so a saturated delta cannot trip
    // debug overflow checks.
    let mut zigzag = (delta.wrapping_mul(2) ^ (delta >> (i64::BITS - 1))) as u64;

    while zigzag >= u64::from(CONTINUATION) {
        let chunk = (zigzag as u8 & CHUNK_MASK) | CONTINUATION;
        encoded.push((chunk + ASCII_OFFSET) as char);
        zigzag >>= CHUNK_BITS;
    }
    encoded.push((zigzag as u8 + ASCII_OFFSET) as char);
}
