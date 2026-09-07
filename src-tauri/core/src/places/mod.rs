//! The place book: every place a trip names, with a coordinate confirmed by a
//! human. See _tasks/75-place-book/02-design.md.

mod normalise;

pub use normalise::normalise;

#[cfg(test)]
#[path = "normalise_tests.rs"]
mod normalise_tests;
