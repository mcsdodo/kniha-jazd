//! Which candidate set a loop uses, from the place it starts at (task 91).
//!
//! The genetic algorithm does not depend on the area: it needs points with the
//! start at index 0 and a matrix between them. This module only decides which
//! points. A new area is a new candidate file plus one branch here.

use super::Dataset;

/// Stable marker at the start of the "no candidates here" error. The page
/// matches it (`NO_LOOP_CANDIDATES` in src/routes/mapa/+page.svelte).
pub const NO_LOOP_CANDIDATES: &str = "NO_LOOP_CANDIDATES";

/// A loop starting this close to home node 0 uses the bundled 67-node set.
pub const HOME_RADIUS_KM: f64 = 5.0;
/// Bratislava, Hlavné námestie.
pub const BRATISLAVA_CENTRE: (f64, f64) = (48.1486, 17.1077);
/// Covers all 17 districts: Čunovo, the farthest, is 14.9 km from the centre.
pub const BRATISLAVA_RADIUS_KM: f64 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopArea {
    Home,
    Bratislava,
}

/// Great-circle distance in km between two `(lat, lon)` points.
pub fn haversine_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    const R: f64 = 6371.0;
    let (la1, la2) = (a.0.to_radians(), b.0.to_radians());
    let dla = la2 - la1;
    let dlo = (b.1 - a.1).to_radians();
    let h = (dla / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlo / 2.0).sin().powi(2);
    2.0 * R * h.sqrt().asin()
}

/// `None` means the generator has no candidates for this place.
pub fn loop_area(lat: f64, lon: f64) -> Option<LoopArea> {
    let ds = Dataset::bundled();
    let home = (ds.nodes[0].lat, ds.nodes[0].lon);
    if haversine_km((lat, lon), home) <= HOME_RADIUS_KM {
        Some(LoopArea::Home)
    } else if haversine_km((lat, lon), BRATISLAVA_CENTRE) <= BRATISLAVA_RADIUS_KM {
        Some(LoopArea::Bratislava)
    } else {
        None
    }
}
