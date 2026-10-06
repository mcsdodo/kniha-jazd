//! Fuelio cross-check (Task 90, POC): compare the drives that the Fuelio
//! Android app recorded with the logbook trips.
//!
//! Fuelio backs up each drive to Google Drive as `route-<epoch ms>.data`, a zip
//! with one CSV of GPS fixes. The files reach `<DATA_DIR>/fuelio` from
//! [`dropbox`] (the sync button) or from a copy outside the app (rclone).
//! Nothing here writes to the database; the overwrite of a trip lives in
//! `commands_internal::fuelio_cmd`.
//!
//! - [`dropbox`]: the year sync from Dropbox into the folder
//! - [`parse`]: the files -> [`Drive`]s
//! - [`geo`]: distances on the earth's surface
//! - [`matching`]: drives against trips -> report rows

pub mod dropbox;
pub mod geo;
pub mod matching;
pub mod parse;

pub use matching::{crosscheck, CrosscheckRow, Flag, RowStatus, TripRef};
pub use parse::{scan_dir, Drive};

/// The folder below the data directory that holds the Fuelio files.
pub const FOLDER_NAME: &str = "fuelio";
