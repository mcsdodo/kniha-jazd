//! Generated trip route maps — GA route selection, OSRM geometry, tile rendering.

pub mod dataset;
pub use dataset::Dataset;

#[cfg(test)]
#[path = "dataset_tests.rs"]
mod dataset_tests;
