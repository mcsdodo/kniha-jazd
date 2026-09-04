//! HTML export command implementations (framework-free).

use std::path::Path;

use crate::commands_internal::{route_maps, statistics};
use crate::db::Database;
use crate::export::{generate_html, ExportData, ExportLabels, ExportTotals};
use crate::route_map::tiles::CachedTileFetcher;

/// Generate the printed logbook HTML.
///
/// `app_dir` locates only the disposable map-tile cache; nothing that must
/// survive is placed from it.
///
/// `sort_direction` ("asc" = oldest first, "desc" = newest first) is used by
/// *both* the row assembly and the `ExportData` below, so the attachment record
/// numbers cannot drift from the order the table is printed in. Never pass a
/// different value to the two.
pub async fn export_html_internal(
    db: &Database,
    app_dir: &Path,
    vehicle_id: String,
    year: i32,
    labels: ExportLabels,
    hidden_columns: Vec<String>,
    sort_direction: String,
) -> Result<String, String> {
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Settings not found - please configure company info first".to_string())?;

    let mut grid_data = statistics::build_trip_grid_data(db, &vehicle_id, year)?;

    if grid_data.trips.is_empty() {
        return Err("No trips found for this year".to_string());
    }

    // Synthetic year-opening row: carries the odometer the year started at, so the
    // printed logbook shows the same baseline the on-screen grid does
    // (TripGrid.svelte FIRST_RECORD_ID). Uuid::nil() is the marker export.rs keys
    // its special-case rendering off (`is_first_record`). It must land before the
    // totals and the row assembly, because the row participates in numbering.
    let first_record_date =
        chrono::NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(|| "Invalid year".to_string())?;
    let first_record = crate::models::Trip {
        id: uuid::Uuid::nil(),
        vehicle_id: vehicle.id,
        start_datetime: first_record_date.and_hms_opt(0, 0, 0).unwrap(),
        end_datetime: None,
        origin: "-".to_string(),
        destination: "-".to_string(),
        distance_km: 0.0,
        odometer: grid_data.year_start_odometer,
        purpose: "Prvý záznam".to_string(),
        fuel_liters: None,
        fuel_cost_eur: None,
        full_tank: true,
        energy_kwh: None,
        energy_cost_eur: None,
        full_charge: false,
        soc_override_percent: None,
        other_costs_eur: None,
        other_costs_note: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    grid_data.trips.push(first_record);
    grid_data
        .fuel_remaining
        .insert(uuid::Uuid::nil().to_string(), grid_data.year_start_fuel);
    grid_data
        .trip_numbers
        .insert(uuid::Uuid::nil().to_string(), 0);
    grid_data
        .odometer_start
        .insert(uuid::Uuid::nil().to_string(), grid_data.year_start_odometer);

    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let baseline_consumption_kwh = vehicle.baseline_consumption_kwh.unwrap_or_default();
    // `ExportTotals::calculate` skips the synthetic rows, so the extra row above
    // does not move the totals (see `test_export_totals_excludes_dummy_rows`).
    let totals =
        ExportTotals::calculate(&grid_data.trips, tp_consumption, baseline_consumption_kwh);

    // Attachment record numbers are read out of the rows the printed table is
    // numbered from — never re-derived — so this export and the desktop one
    // cite the same record for the same trip.
    let rows = route_maps::assemble_export_rows(&grid_data, &sort_direction);
    let tiles = CachedTileFetcher::osm(route_maps::tile_cache_dir(app_dir));
    let map_pages = route_maps::collect_route_map_pages(db, &tiles, &rows).await;

    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
        hidden_columns,
        sort_direction,
        route_maps: map_pages,
    };

    generate_html(export_data)
}
