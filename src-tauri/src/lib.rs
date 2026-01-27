mod calculations;
mod app_state;
mod calculations_energy;
mod calculations_phev;
mod commands;
mod db;
mod db_location;
mod export;
mod gemini;
mod models;
mod receipts;
mod schema;
mod settings;
mod suggestions;

use std::path::PathBuf;
use crate::app_state::AppState;
use crate::db_location::{resolve_db_paths, check_lock, acquire_lock, LockStatus};
use crate::settings::LocalSettings;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      // Initialize app state
      let app_state = AppState::new();

      // Get app data directory (or from env for tests)
      let app_dir = match std::env::var("KNIHA_JAZD_DATA_DIR") {
        Ok(path) => PathBuf::from(path),
        Err(_) => app.path().app_data_dir().expect("Failed to get app data dir"),
      };
      std::fs::create_dir_all(&app_dir).expect("Failed to create app data directory");

      // Load local settings to check for custom database path
      let local_settings = LocalSettings::load(&app_dir);

      // Resolve database paths (custom or default)
      let (db_paths, is_custom) = resolve_db_paths(
        &app_dir,
        local_settings.custom_db_path.as_deref(),
      );

      // Ensure target directory exists (especially for custom paths)
      if let Some(parent) = db_paths.db_file.parent() {
        std::fs::create_dir_all(parent).ok();
      }

      // Check lock file status
      match check_lock(&db_paths.lock_file) {
        LockStatus::Locked { pc_name, since } => {
          log::warn!(
            "Database appears to be locked by {} since {}",
            pc_name,
            since.format("%Y-%m-%d %H:%M:%S UTC")
          );
          // TODO: Future enhancement - emit event to show warning dialog
          // For now, we continue but user should be aware
        }
        LockStatus::Stale { pc_name } => {
          log::info!("Taking over stale lock from {}", pc_name);
        }
        LockStatus::Free => {
          log::debug!("Lock file is free");
        }
      }

      // Initialize database
      let db = db::Database::new(db_paths.db_file.clone())
        .expect("Failed to initialize database");

      // Check migration compatibility
      match db.check_migration_compatibility() {
        Ok(()) => {
          log::info!("Database migration compatibility: OK");
        }
        Err(unknown_migrations) => {
          log::warn!(
            "Database has unknown migrations: {:?}. Entering read-only mode.",
            unknown_migrations
          );
          app_state.enable_read_only(
            "Databáza bola aktualizovaná novšou verziou aplikácie."
          );
        }
      }

      // Acquire lock (only if not in read-only mode)
      let lock_file_path = db_paths.lock_file.clone();
      if !app_state.is_read_only() {
        let version = env!("CARGO_PKG_VERSION");
        if let Err(e) = acquire_lock(&lock_file_path, version) {
          log::warn!("Failed to acquire lock: {}", e);
        } else {
          // Start background heartbeat task to keep lock fresh
          let heartbeat_lock_path = lock_file_path.clone();
          std::thread::spawn(move || {
            loop {
              std::thread::sleep(std::time::Duration::from_secs(30));
              if let Err(e) = db_location::refresh_lock(&heartbeat_lock_path) {
                log::warn!("Failed to refresh lock: {}", e);
                // Lock file may have been deleted - stop heartbeat
                break;
              }
            }
          });
        }
      }

      // Store paths in app state
      app_state.set_db_path(db_paths.db_file, is_custom);

      // Check read-only before moving app_state
      let is_read_only = app_state.is_read_only();

      // Manage database and app state
      app.manage(db);
      app.manage(app_state);

      // Run post-update cleanup in background if retention is enabled
      if !is_read_only {
        let cleanup_app_handle = app.handle().clone();
        let cleanup_app_dir = app_dir.clone();
        std::thread::spawn(move || {
          // Load retention settings
          let settings = LocalSettings::load(&cleanup_app_dir);
          if let Some(retention) = settings.backup_retention {
            if retention.enabled && retention.keep_count > 0 {
              // Run cleanup silently
              if let Err(e) = commands::cleanup_pre_update_backups_internal(
                &cleanup_app_handle,
                retention.keep_count,
              ) {
                log::warn!("Failed to run post-update cleanup: {}", e);
              } else {
                log::info!("Post-update backup cleanup completed");
              }
            }
          }
        });
      }

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
      commands::get_purposes,
      commands::get_settings,
      commands::save_settings,
      commands::calculate_trip_stats,
      commands::create_backup,
      commands::create_backup_with_type,
      commands::get_cleanup_preview,
      commands::cleanup_pre_update_backups,
      commands::get_backup_retention,
      commands::set_backup_retention,
      commands::list_backups,
      commands::get_backup_info,
      commands::restore_backup,
      commands::delete_backup,
      commands::get_backup_path,
      commands::get_trip_grid_data,
      commands::calculate_magic_fill_liters,
      commands::export_html,
      commands::export_to_browser,
      commands::get_receipt_settings,
      commands::get_receipts,
      commands::get_receipts_for_vehicle,
      commands::get_unassigned_receipts,
      commands::scan_receipts,
      commands::sync_receipts,
      commands::process_pending_receipts,
      commands::update_receipt,
      commands::delete_receipt,
      commands::reprocess_receipt,
      commands::assign_receipt_to_trip,
      commands::get_trips_for_receipt_assignment,
      commands::verify_receipts,
      commands::get_optimal_window_size,
      commands::preview_trip_calculation,
      commands::get_theme_preference,
      commands::set_theme_preference,
      commands::get_auto_check_updates,
      commands::set_auto_check_updates,
      commands::get_date_prefill_mode,
      commands::set_date_prefill_mode,
      commands::get_hidden_columns,
      commands::set_hidden_columns,
      commands::get_db_location,
      commands::get_app_mode,
      commands::check_target_has_db,
      commands::move_database,
      commands::reset_database_location,
      commands::set_gemini_api_key,
      commands::set_receipts_folder_path,
      commands::get_ha_settings,
      commands::save_ha_settings,
      commands::get_local_settings_for_ha,
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|app, event| {
      // Release lock file on clean exit
      if let tauri::RunEvent::Exit = event {
        if let Some(app_state) = app.try_state::<AppState>() {
          if let Some(db_path) = app_state.get_db_path() {
            if let Some(parent) = db_path.parent() {
              let lock_path = parent.join("kniha-jazd.lock");
              if let Err(e) = db_location::release_lock(&lock_path) {
                log::warn!("Failed to release lock on exit: {}", e);
              } else {
                log::debug!("Lock released on exit");
              }
            }
          }
        }
      }
    });
}
