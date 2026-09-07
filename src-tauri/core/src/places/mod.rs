//! The place book: every place a trip names, with a coordinate confirmed by a
//! human. See _tasks/_done/75-place-book/02-design.md.

mod geocode;
mod normalise;

pub use geocode::{Candidate, GeocodeProvider, HttpGeocodeProvider};
pub use normalise::normalise;

#[cfg(test)]
#[path = "geocode_tests.rs"]
mod geocode_tests;

#[cfg(test)]
#[path = "normalise_tests.rs"]
mod normalise_tests;
