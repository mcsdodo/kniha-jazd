//! Framework-free command implementations.
//!
//! Each `*_internal` function takes plain types (`&Database`, `&AppState`,
//! plain args) instead of web-framework extractors, so it can be unit tested
//! without standing a server up. The JSON-RPC dispatchers in
//! `kniha_jazd_core::server::{dispatcher, dispatcher_async}` are the only
//! callers: they deserialize the request args and forward them here.

pub mod helpers;
pub use helpers::*;

pub mod backup;
pub use backup::*;

pub mod trips;
pub use trips::*;

pub mod vehicles;
pub use vehicles::*;

pub mod statistics;
pub use statistics::*;

pub mod export_cmd;
pub use export_cmd::*;

pub mod receipts_cmd;
pub use receipts_cmd::*;

pub mod settings_cmd;
pub use settings_cmd::*;

pub mod integrations;
pub use integrations::*;

pub mod reveal;
pub use reveal::*;

pub mod invoices;
pub use invoices::*;

pub mod paperless_cmd;
pub use paperless_cmd::*;

pub mod route_maps;
pub use route_maps::*;

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
