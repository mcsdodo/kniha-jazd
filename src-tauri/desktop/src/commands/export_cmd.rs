//! HTML Export Tauri command wrappers.
//!
//! `export_to_browser` keeps its body in desktop because it writes a temp
//! file and opens the system browser via the `open` crate — Tauri-shell
//! flavored side effects.

use std::fs;
use std::sync::Arc;

use chrono::Utc;
use tauri::State;
use uuid::Uuid;

use kniha_jazd_core::commands_internal::{export_cmd as inner, statistics};
use kniha_jazd_core::db::Database;
use kniha_jazd_core::export::{generate_html, ExportData, ExportLabels, ExportTotals};
use kniha_jazd_core::models::Trip;

/// Export trips to browser - generates HTML and opens in default browser.
#[tauri::command]
pub async fn export_to_browser(
    _app: tauri::AppHandle,
    db: State<'_, Arc<Database>>,
    vehicle_id: String,
    year: i32,
    license_plate: String,
    _sort_column: String,
    _sort_direction: String,
    labels: ExportLabels,
    hidden_columns: Vec<String>,
) -> Result<(), String> {
    let vehicle = db
        .get_vehicle(&vehicle_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vehicle not found".to_string())?;

    let settings = db
        .get_settings()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Settings not found - please configure company info first".to_string())?;

    let mut grid_data = statistics::build_trip_grid_data(&db, &vehicle_id, year)?;

    let first_record_date =
        chrono::NaiveDate::from_ymd_opt(year, 1, 1).ok_or_else(|| "Invalid year".to_string())?;
    let first_record = Trip {
        id: Uuid::nil(),
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
        sort_order: 999999,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    grid_data.trips.push(first_record);
    grid_data
        .fuel_remaining
        .insert(Uuid::nil().to_string(), grid_data.year_start_fuel);
    grid_data.trip_numbers.insert(Uuid::nil().to_string(), 0);
    grid_data
        .odometer_start
        .insert(Uuid::nil().to_string(), grid_data.year_start_odometer);

    let tp_consumption = vehicle.tp_consumption.unwrap_or_default();
    let baseline_consumption_kwh = vehicle.baseline_consumption_kwh.unwrap_or_default();
    let totals =
        ExportTotals::calculate(&grid_data.trips, tp_consumption, baseline_consumption_kwh);

    let export_data = ExportData {
        vehicle,
        settings,
        grid_data,
        year,
        totals,
        labels,
        hidden_columns,
    };

    let html = generate_html(export_data)?;

    let temp_dir = std::env::temp_dir();
    let filename = format!("kniha-jazd-{}-{}.html", license_plate, year);
    let temp_path = temp_dir.join(&filename);

    fs::write(&temp_path, html).map_err(|e| format!("Failed to write temp file: {}", e))?;

    open::that(&temp_path).map_err(|e| format!("Failed to open browser: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn export_html(
    db: State<'_, Arc<Database>>,
    vehicle_id: String,
    year: i32,
    labels: ExportLabels,
) -> Result<String, String> {
    inner::export_html_internal(&db, vehicle_id, year, labels).await
}
