//! Generated trip route maps — GA route selection, OSRM geometry, tile rendering.

pub mod dataset;
pub mod ga;
pub mod osrm;
pub mod polyline;
pub mod render;
pub mod tiles;

pub use dataset::Dataset;
pub use ga::{
    generate_route, generate_route_random, RouteResult, RouteRng, ThreadRouteRng, TOLERANCE,
};
pub use osrm::{FetchedRoute, HttpRouteProvider, RouteProvider};

#[cfg(test)]
#[path = "dataset_tests.rs"]
mod dataset_tests;

#[cfg(test)]
#[path = "ga_tests.rs"]
mod ga_tests;

#[cfg(test)]
#[path = "osrm_tests.rs"]
mod osrm_tests;

#[cfg(test)]
#[path = "polyline_tests.rs"]
mod polyline_tests;

#[cfg(test)]
#[path = "tiles_tests.rs"]
mod tiles_tests;

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
