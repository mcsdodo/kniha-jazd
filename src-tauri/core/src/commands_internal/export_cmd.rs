//! HTML export command implementations (framework-free).

use std::path::Path;

use crate::commands_internal::{route_maps, statistics};
use crate::db::Database;
use crate::export::{generate_html, ExportData, ExportLabels, ExportTotals};
use crate::route_map::tiles::CachedTileFetcher;

/// Server-mode export keeps the legacy default order (oldest first). Shared by
/// the row assembly and the `ExportData` below so the attachment record numbers
/// cannot drift from the order the table is printed in.
const SORT_DIRECTION: &str = "asc";

/// Generate the printed logbook HTML.
///
/// `app_dir` locates only the disposable map-tile cache; nothing that must
/// survive is placed from it.
pub async fn export_html_internal(
    db: &Database,
    app_dir: &Path,
    vehicle_id: String,
    year: i32,
    labels: ExportLabels,
) -> Result<String, String> {
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Settings not found - please configure company info first".to_string())?;

    let grid_data = statistics::build_trip_grid_data(db, &vehicle_id, year)?;

    if grid_data.trips.is_empty() {
        return Err("No trips found for this year".to_string());
    }

    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let baseline_consumption_kwh = vehicle.baseline_consumption_kwh.unwrap_or_default();
    let totals =
        ExportTotals::calculate(&grid_data.trips, tp_consumption, baseline_consumption_kwh);

    // Attachment record numbers are read out of the rows the printed table is
    // numbered from — never re-derived — so this export and the desktop one
    // cite the same record for the same trip.
    let rows = route_maps::assemble_export_rows(&grid_data, SORT_DIRECTION);
    let tiles = CachedTileFetcher::osm(route_maps::tile_cache_dir(app_dir));
    let map_pages = route_maps::collect_route_map_pages(db, &tiles, &rows).await;

    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
        hidden_columns: Vec::new(),
        sort_direction: SORT_DIRECTION.to_string(),
        route_maps: map_pages,
    };

    generate_html(export_data)
}
