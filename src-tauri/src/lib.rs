mod calculations;
mod calculations_energy;
mod calculations_phev;
mod commands;
mod db;
mod error;
mod export;
mod gemini;
mod models;
mod receipts;
mod settings;
mod suggestions;

use std::path::PathBuf;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // Initialize database
      // Allow tests to override data directory via environment variable
      let app_dir = match std::env::var("KNIHA_JAZD_DATA_DIR") {
        Ok(path) => PathBuf::from(path),
        Err(_) => app.path().app_data_dir().expect("Failed to get app data dir"),
      };
      std::fs::create_dir_all(&app_dir).expect("Failed to create app data directory");
      let db_path = app_dir.join("kniha-jazd.db");
      let db = db::Database::new(db_path).expect("Failed to initialize database");

      // Manage database state
      app.manage(db);

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::get_vehicles,
      commands::get_active_vehicle,
      commands::create_vehicle,
      commands::update_vehicle,
      commands::delete_vehicle,
      commands::set_active_vehicle,
      commands::get_trips,
      commands::get_trips_for_year,
      commands::get_years_with_trips,
      commands::create_trip,
      commands::update_trip,
      commands::delete_trip,
      commands::reorder_trip,
      commands::get_routes,
      commands::get_compensation_suggestion,
      commands::get_settings,
      commands::save_settings,
      commands::calculate_trip_stats,
      commands::create_backup,
      commands::list_backups,
      commands::get_backup_info,
      commands::restore_backup,
      commands::delete_backup,
      commands::get_trip_grid_data,
      commands::export_html,
      commands::export_to_browser,
      commands::get_receipt_settings,
      commands::get_receipts,
      commands::get_unassigned_receipts,
      commands::scan_receipts,
      commands::sync_receipts,
      commands::process_pending_receipts,
      commands::update_receipt,
      commands::delete_receipt,
      commands::reprocess_receipt,
      commands::assign_receipt_to_trip,
      commands::verify_receipts,
      commands::get_optimal_window_size,
      commands::preview_trip_calculation,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
