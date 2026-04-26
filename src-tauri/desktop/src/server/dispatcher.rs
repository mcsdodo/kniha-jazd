//! Sync RPC dispatcher — routes command name strings to `_internal` functions.

use serde_json::Value;

use super::ServerState;

/// Deserialize JSON args into a typed struct, returning a human-readable error.
fn parse_args<T: serde::de::DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|e| format!("Invalid args: {e}"))
}

/// Dispatch a synchronous command by name.
///
/// Returns `Ok(Value)` on success or `Err(message)` on failure.
/// Unknown commands produce an `Err` with "Unknown command: …".
#[allow(clippy::too_many_lines)]
pub fn dispatch_sync(command: &str, args: Value, state: &ServerState) -> Result<Value, String> {
    match command {
        // ====================================================================
        // Vehicles (6)
        // ====================================================================
        "get_vehicles" => {
            let v = kniha_jazd_core::commands_internal::get_vehicles_internal(&state.db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_active_vehicle" => {
            let v = kniha_jazd_core::commands_internal::get_active_vehicle_internal(&state.db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "create_vehicle" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                name: String,
                license_plate: String,
                initial_odometer: f64,
                vehicle_type: Option<String>,
                tank_size_liters: Option<f64>,
                tp_consumption: Option<f64>,
                battery_capacity_kwh: Option<f64>,
                baseline_consumption_kwh: Option<f64>,
                initial_battery_percent: Option<f64>,
                vin: Option<String>,
                driver_name: Option<String>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::create_vehicle_internal(
                &state.db,
                &state.app_state,
                a.name,
                a.license_plate,
                a.initial_odometer,
                a.vehicle_type,
                a.tank_size_liters,
                a.tp_consumption,
                a.battery_capacity_kwh,
                a.baseline_consumption_kwh,
                a.initial_battery_percent,
                a.vin,
                a.driver_name,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "update_vehicle" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle: crate::models::Vehicle,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::update_vehicle_internal(&state.db, &state.app_state, a.vehicle)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "delete_vehicle" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::delete_vehicle_internal(&state.db, &state.app_state, a.id)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "set_active_vehicle" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::set_active_vehicle_internal(&state.db, &state.app_state, a.id)?;
            Ok(serde_json::to_value(()).unwrap())
        }

        // ====================================================================
        // Trips (10)
        // ====================================================================
        "get_trips" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::get_trips_internal(&state.db, a.vehicle_id)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_trips_for_year" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v =
                kniha_jazd_core::commands_internal::get_trips_for_year_internal(&state.db, a.vehicle_id, a.year)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_years_with_trips" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::get_years_with_trips_internal(&state.db, a.vehicle_id)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "create_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                start_datetime: String,
                end_datetime: String,
                origin: String,
                destination: String,
                distance_km: f64,
                odometer: f64,
                purpose: String,
                fuel_liters: Option<f64>,
                fuel_cost: Option<f64>,
                full_tank: Option<bool>,
                energy_kwh: Option<f64>,
                energy_cost_eur: Option<f64>,
                full_charge: Option<bool>,
                soc_override_percent: Option<f64>,
                other_costs: Option<f64>,
                other_costs_note: Option<String>,
                insert_at_position: Option<i32>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::create_trip_internal(
                &state.db,
                &state.app_state,
                a.vehicle_id,
                a.start_datetime,
                a.end_datetime,
                a.origin,
                a.destination,
                a.distance_km,
                a.odometer,
                a.purpose,
                a.fuel_liters,
                a.fuel_cost,
                a.full_tank,
                a.energy_kwh,
                a.energy_cost_eur,
                a.full_charge,
                a.soc_override_percent,
                a.other_costs,
                a.other_costs_note,
                a.insert_at_position,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "update_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
                start_datetime: String,
                end_datetime: String,
                origin: String,
                destination: String,
                distance_km: f64,
                odometer: f64,
                purpose: String,
                fuel_liters: Option<f64>,
                fuel_cost_eur: Option<f64>,
                full_tank: Option<bool>,
                energy_kwh: Option<f64>,
                energy_cost_eur: Option<f64>,
                full_charge: Option<bool>,
                soc_override_percent: Option<f64>,
                other_costs_eur: Option<f64>,
                other_costs_note: Option<String>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::update_trip_internal(
                &state.db,
                &state.app_state,
                a.id,
                a.start_datetime,
                a.end_datetime,
                a.origin,
                a.destination,
                a.distance_km,
                a.odometer,
                a.purpose,
                a.fuel_liters,
                a.fuel_cost_eur,
                a.full_tank,
                a.energy_kwh,
                a.energy_cost_eur,
                a.full_charge,
                a.soc_override_percent,
                a.other_costs_eur,
                a.other_costs_note,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "delete_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::delete_trip_internal(&state.db, &state.app_state, a.id)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "reorder_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
                new_sort_order: i32,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::reorder_trip_internal(
                &state.db,
                &state.app_state,
                a.trip_id,
                a.new_sort_order,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_routes" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::get_routes_internal(&state.db, a.vehicle_id)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_purposes" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::get_purposes_internal(&state.db, a.vehicle_id)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_inferred_trip_time_for_route" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                origin: String,
                destination: String,
                row_date: String,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::get_inferred_trip_time_for_route_internal(
                &state.db,
                a.vehicle_id,
                a.origin,
                a.destination,
                a.row_date,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Statistics (4)
        // ====================================================================
        "calculate_trip_stats" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::calculate_trip_stats_internal(
                &state.db,
                a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_trip_grid_data" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::build_trip_grid_data(&state.db, &a.vehicle_id, a.year)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "calculate_magic_fill_liters" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
                current_trip_km: f64,
                editing_trip_id: Option<String>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::calculate_magic_fill_liters_internal(
                &state.db,
                a.vehicle_id,
                a.year,
                a.current_trip_km,
                a.editing_trip_id,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "preview_trip_calculation" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
                distance_km: i32,
                fuel_liters: Option<f64>,
                full_tank: bool,
                insert_at_sort_order: Option<i32>,
                editing_trip_id: Option<String>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::preview_trip_calculation_internal(
                &state.db,
                a.vehicle_id,
                a.year,
                a.distance_km,
                a.fuel_liters,
                a.full_tank,
                a.insert_at_sort_order,
                a.editing_trip_id,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Settings (14)
        // ====================================================================
        "get_settings" => {
            let v = crate::commands::get_settings_internal(&state.db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "save_settings" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                company_name: String,
                company_ico: String,
                buffer_trip_purpose: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands::save_settings_internal(
                &state.db,
                &state.app_state,
                a.company_name,
                a.company_ico,
                a.buffer_trip_purpose,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_optimal_window_size" => {
            let v = crate::commands::get_optimal_window_size_internal();
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_theme_preference" => {
            let v = crate::commands::get_theme_preference_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_theme_preference" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                theme: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands::set_theme_preference_internal(&state.app_dir, a.theme)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_auto_check_updates" => {
            let v = crate::commands::get_auto_check_updates_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_auto_check_updates" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                enabled: bool,
            }
            let a: Args = parse_args(args)?;
            crate::commands::set_auto_check_updates_internal(&state.app_dir, a.enabled)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_date_prefill_mode" => {
            let v = crate::commands::get_date_prefill_mode_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_date_prefill_mode" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                mode: crate::settings::DatePrefillMode,
            }
            let a: Args = parse_args(args)?;
            crate::commands::set_date_prefill_mode_internal(&state.app_dir, a.mode)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_hidden_columns" => {
            let v = crate::commands::get_hidden_columns_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_hidden_columns" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                columns: Vec<String>,
            }
            let a: Args = parse_args(args)?;
            crate::commands::set_hidden_columns_internal(&state.app_dir, a.columns)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_db_location" => {
            let v = crate::commands::get_db_location_internal(&state.app_state)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_app_mode" => {
            let v = crate::commands::get_app_mode_internal(&state.app_state)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "check_target_has_db" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                target_path: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands::check_target_has_db_internal(a.target_path)?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Receipts (11)
        // ====================================================================
        "get_receipts" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                year: Option<i32>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::receipts_cmd::get_receipts_internal(
                &state.db, a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_receipts_for_vehicle" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: Option<i32>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::receipts_cmd::get_receipts_for_vehicle_internal(
                &state.db,
                a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_unassigned_receipts" => {
            let v = kniha_jazd_core::commands_internal::receipts_cmd::get_unassigned_receipts_internal(&state.db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "update_receipt" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                receipt: crate::models::Receipt,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::receipts_cmd::update_receipt_internal(
                &state.db,
                &state.app_state,
                a.receipt,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "delete_receipt" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::receipts_cmd::delete_receipt_internal(
                &state.db,
                &state.app_state,
                a.id,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "unassign_receipt" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::receipts_cmd::unassign_receipt_internal(
                &state.db,
                &state.app_state,
                a.id,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "revert_receipt_override" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::receipts_cmd::revert_receipt_override_internal(
                &state.db,
                &state.app_state,
                a.id,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "assign_receipt_to_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                receipt_id: String,
                trip_id: String,
                vehicle_id: String,
                assignment_type: String,
                mismatch_override: bool,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::receipts_cmd::assign_receipt_to_trip_internal(
                &state.db,
                &a.receipt_id,
                &a.trip_id,
                &a.vehicle_id,
                &a.assignment_type,
                a.mismatch_override,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_trips_for_receipt_assignment" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                receipt_id: String,
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::receipts_cmd::get_trips_for_receipt_assignment_internal(
                &state.db,
                &a.receipt_id,
                &a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "verify_receipts" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::receipts_cmd::verify_receipts_internal(
                &state.db,
                &a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_receipt_settings" => {
            let v = kniha_jazd_core::commands_internal::receipts_cmd::get_receipt_settings_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_gemini_api_key" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                api_key: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::receipts_cmd::set_gemini_api_key_internal(
                &state.app_dir,
                &state.app_state,
                a.api_key,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "set_receipts_folder_path" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                path: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::receipts_cmd::set_receipts_folder_path_internal(
                &state.app_dir,
                &state.app_state,
                a.path,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "scan_receipts" => {
            let v = kniha_jazd_core::commands_internal::receipts_cmd::scan_receipts_internal(
                &state.db,
                &state.app_state,
                &state.app_dir,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Backup (10)
        // ====================================================================
        "create_backup" => {
            let v = kniha_jazd_core::commands_internal::create_backup_internal(
                &state.app_dir,
                &state.db,
                &state.app_state,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "create_backup_with_type" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                backup_type: String,
                update_version: Option<String>,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::create_backup_with_type_internal(
                &state.app_dir,
                &state.db,
                &state.app_state,
                a.backup_type,
                a.update_version,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_cleanup_preview" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                keep_count: u32,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::get_cleanup_preview_internal(
                &state.app_dir,
                a.keep_count,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "cleanup_pre_update_backups" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                keep_count: u32,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::cleanup_pre_update_backups_internal(
                &state.app_dir,
                a.keep_count,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_backup_retention" => {
            let v = kniha_jazd_core::commands_internal::get_backup_retention_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_backup_retention" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                retention: crate::settings::BackupRetention,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::set_backup_retention_internal(
                &state.app_dir,
                &state.app_state,
                a.retention,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "list_backups" => {
            let v = kniha_jazd_core::commands_internal::list_backups_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_backup_info" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                filename: String,
            }
            let a: Args = parse_args(args)?;
            let v =
                kniha_jazd_core::commands_internal::get_backup_info_internal(&state.app_dir, a.filename)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "delete_backup" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                filename: String,
            }
            let a: Args = parse_args(args)?;
            kniha_jazd_core::commands_internal::delete_backup_internal(
                &state.app_dir,
                &state.app_state,
                a.filename,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_backup_path" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                filename: String,
            }
            let a: Args = parse_args(args)?;
            let v = kniha_jazd_core::commands_internal::get_backup_path_internal(&state.app_dir, a.filename)?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Integrations — sync only (3)
        // ====================================================================
        "get_ha_settings" => {
            let v = crate::commands::get_ha_settings_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_local_settings_for_ha" => {
            let v = crate::commands::get_local_settings_for_ha_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "save_ha_settings" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                url: Option<String>,
                token: Option<String>,
            }
            let a: Args = parse_args(args)?;
            crate::commands::save_ha_settings_internal(
                &state.app_dir,
                &state.app_state,
                a.url,
                a.token,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }

        // ====================================================================
        // Unknown
        // ====================================================================
        _ => Err(format!("Unknown command: {command}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_state() -> ServerState {
        ServerState {
            db: std::sync::Arc::new(crate::db::Database::in_memory().unwrap()),
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: std::env::temp_dir(),
            static_dir: std::env::temp_dir(),
        }
    }

    #[test]
    fn unknown_command_returns_error() {
        let state = test_state();
        let result = dispatch_sync("nonexistent", json!({}), &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown"));
    }

    #[test]
    fn get_vehicles_returns_empty_list() {
        let state = test_state();
        let result = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(result, json!([]));
    }

    #[test]
    fn create_vehicle_then_get() {
        let state = test_state();
        let args = json!({
            "name": "Test Car",
            "licensePlate": "BA-123AB",
            "initialOdometer": 50000.0,
            "vehicleType": "Ice",
            "tankSizeLiters": 50.0,
            "tpConsumption": 6.5
        });
        let created = dispatch_sync("create_vehicle", args, &state).unwrap();
        assert_eq!(created["name"], "Test Car");

        let vehicles = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(vehicles.as_array().unwrap().len(), 1);
    }

    #[test]
    fn write_command_fails_in_read_only_mode() {
        let state = test_state();
        state.app_state.enable_read_only("Test read-only");

        let result = dispatch_sync(
            "create_vehicle",
            json!({
                "name": "Test",
                "licensePlate": "XX",
                "initialOdometer": 0.0,
                "vehicleType": "Ice",
                "tankSizeLiters": 50.0,
                "tpConsumption": 6.5
            }),
            &state,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("režime len na čítanie"));
    }
}
