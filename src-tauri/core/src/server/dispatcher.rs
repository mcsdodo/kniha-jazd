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
        // Vehicles
        // ====================================================================
        "get_vehicles" => {
            let v = crate::commands_internal::get_vehicles_internal(&state.db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_active_vehicle" => {
            let v = crate::commands_internal::get_active_vehicle_internal(&state.db)?;
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
            let v = crate::commands_internal::create_vehicle_internal(
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
            crate::commands_internal::update_vehicle_internal(&state.db, &state.app_state, a.vehicle)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "delete_vehicle" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::delete_vehicle_internal(&state.db, &state.app_state, a.id)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "set_active_vehicle" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::set_active_vehicle_internal(&state.db, &state.app_state, a.id)?;
            Ok(serde_json::to_value(()).unwrap())
        }

        // ====================================================================
        // Trips
        // ====================================================================
        "get_trips" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::get_trips_internal(&state.db, a.vehicle_id)?;
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
                crate::commands_internal::get_trips_for_year_internal(&state.db, a.vehicle_id, a.year)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_years_with_trips" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::get_years_with_trips_internal(&state.db, a.vehicle_id)?;
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
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::create_trip_internal(
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
            let v = crate::commands_internal::update_trip_internal(
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
        "recalculate_odometers" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
                dry_run: bool,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::recalculate_odometers_internal(
                &state.db,
                &state.app_state,
                a.vehicle_id,
                a.year,
                a.dry_run,
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
            crate::commands_internal::delete_trip_internal(&state.db, &state.app_state, a.id)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_routes" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::get_routes_internal(&state.db, a.vehicle_id)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_purposes" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::get_purposes_internal(&state.db, a.vehicle_id)?;
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
            let v = crate::commands_internal::get_inferred_trip_time_for_route_internal(
                &state.db,
                &state.app_dir,
                a.vehicle_id,
                a.origin,
                a.destination,
                a.row_date,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_copied_trip_defaults" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::get_copied_trip_defaults_internal(
                &state.db,
                a.trip_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Statistics
        // ====================================================================
        "calculate_trip_stats" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::calculate_trip_stats_internal(
                &state.db,
                a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "reveal_secret" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                field: crate::commands_internal::reveal::SecretField,
                /// Absent is treated as an empty PIN, which is rejected like
                /// any other wrong one.
                #[serde(default)]
                pin: String,
            }
            let a: Args = parse_args(args)?;
            let value = crate::commands_internal::reveal::reveal_secret_internal(
                &state.app_dir,
                &state.app_state,
                a.field,
                &a.pin,
            )?;
            Ok(serde_json::to_value(value).unwrap())
        }

        // get_trip_grid_data lives in dispatcher_async — it also performs the
        // fire-and-forget HA suggested-fillup push, which needs a runtime.
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
            let v = crate::commands_internal::calculate_magic_fill_liters_internal(
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
                insert_at_trip_id: Option<String>,
                editing_trip_id: Option<String>,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::preview_trip_calculation_internal(
                &state.db,
                a.vehicle_id,
                a.year,
                a.distance_km,
                a.fuel_liters,
                a.full_tank,
                a.insert_at_trip_id,
                a.editing_trip_id,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Settings
        // ====================================================================
        "get_settings" => {
            let v = crate::commands_internal::settings_cmd::get_settings_internal(&state.db)?;
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
            let v = crate::commands_internal::settings_cmd::save_settings_internal(
                &state.db,
                &state.app_state,
                a.company_name,
                a.company_ico,
                a.buffer_trip_purpose,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_theme_preference" => {
            let v = crate::commands_internal::settings_cmd::get_theme_preference_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_theme_preference" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                theme: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::settings_cmd::set_theme_preference_internal(&state.app_dir, a.theme)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_date_prefill_mode" => {
            let v = crate::commands_internal::settings_cmd::get_date_prefill_mode_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_date_prefill_mode" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                mode: crate::settings::DatePrefillMode,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::settings_cmd::set_date_prefill_mode_internal(&state.app_dir, a.mode)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_hidden_columns" => {
            let v = crate::commands_internal::settings_cmd::get_hidden_columns_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_hidden_columns" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                columns: Vec<String>,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::settings_cmd::set_hidden_columns_internal(&state.app_dir, a.columns)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_infer_trip_times" => {
            let v = crate::commands_internal::settings_cmd::get_infer_trip_times_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_infer_trip_times" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                enabled: bool,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::settings_cmd::set_infer_trip_times_internal(&state.app_dir, a.enabled)?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_db_location" => {
            let v = crate::commands_internal::settings_cmd::get_db_location_internal(&state.app_state)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_app_mode" => {
            let v = crate::commands_internal::settings_cmd::get_app_mode_internal(&state.app_state)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        // The workspace version in src-tauri/Cargo.toml, which `/release` bumps in
        // lockstep with package.json — so this is the ghcr tag the container came from.
        "get_app_version" => Ok(serde_json::to_value(env!("CARGO_PKG_VERSION")).unwrap()),

        // ====================================================================
        // Receipts
        // ====================================================================
        "get_receipts" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                year: Option<i32>,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::receipts_cmd::get_receipts_internal(
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
            let v = crate::commands_internal::receipts_cmd::get_receipts_for_vehicle_internal(
                &state.db,
                a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_unassigned_receipts" => {
            let v = crate::commands_internal::receipts_cmd::get_unassigned_receipts_internal(&state.db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "update_receipt" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                receipt: crate::models::Receipt,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::receipts_cmd::update_receipt_internal(
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
            crate::commands_internal::receipts_cmd::delete_receipt_internal(
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
            crate::commands_internal::receipts_cmd::revert_receipt_override_internal(
                &state.db,
                &state.app_state,
                a.id,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_trips_for_invoice_assignment" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                invoice_ref: crate::invoice::InvoiceRef,
                invoice_data: Option<crate::invoice::InvoiceData>,
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::invoices::get_trips_for_invoice_assignment_internal(
                &state.db,
                &a.invoice_ref,
                a.invoice_data.as_ref(),
                &a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "unassign_invoice" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                invoice_ref: crate::invoice::InvoiceRef,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::invoices::unassign_invoice_internal(
                &state.db,
                &state.app_state,
                &a.invoice_ref,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "verify_receipts" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::receipts_cmd::verify_receipts_internal(
                &state.db,
                &a.vehicle_id,
                a.year,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_receipt_settings" => {
            let v = crate::commands_internal::receipts_cmd::get_receipt_settings_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_gemini_api_key" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                api_key: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::receipts_cmd::set_gemini_api_key_internal(
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
            crate::commands_internal::receipts_cmd::set_receipts_folder_path_internal(
                &state.app_dir,
                &state.app_state,
                a.path,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "scan_receipts" => {
            let v = crate::commands_internal::receipts_cmd::scan_receipts_internal(
                &state.db,
                &state.app_state,
                &state.app_dir,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Backup
        // ====================================================================
        "create_backup" => {
            let v = crate::commands_internal::create_backup_internal(
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
            let v = crate::commands_internal::create_backup_with_type_internal(
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
            let v = crate::commands_internal::get_cleanup_preview_internal(
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
            let v = crate::commands_internal::cleanup_pre_update_backups_internal(
                &state.app_dir,
                a.keep_count,
            )?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_backup_retention" => {
            let v = crate::commands_internal::get_backup_retention_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "set_backup_retention" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                retention: crate::settings::BackupRetention,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::set_backup_retention_internal(
                &state.app_dir,
                &state.app_state,
                a.retention,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "list_backups" => {
            let v = crate::commands_internal::list_backups_internal(&state.app_dir)?;
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
                crate::commands_internal::get_backup_info_internal(&state.app_dir, a.filename)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "delete_backup" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                filename: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::delete_backup_internal(
                &state.app_dir,
                &state.app_state,
                a.filename,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "restore_backup" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                filename: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::restore_backup_internal(
                &state.app_dir,
                &state.app_state,
                a.filename,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }

        // ====================================================================
        // Integrations — sync only
        // ====================================================================
        "get_ha_settings" => {
            let v = crate::commands_internal::integrations::get_ha_settings_internal(&state.app_dir)?;
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
            crate::commands_internal::integrations::save_ha_settings_internal(
                &state.app_dir,
                &state.app_state,
                a.url,
                a.token,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_paperless_settings" => {
            let v = crate::commands_internal::integrations::get_paperless_settings_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "save_paperless_settings" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                url: Option<String>,
                token: Option<String>,
                enabled: Option<bool>,
                field_name_datetime: Option<String>,
                field_name_liters: Option<String>,
                field_name_total: Option<String>,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::integrations::save_paperless_settings_internal(
                &state.app_dir,
                &state.app_state,
                a.url,
                a.token,
                a.enabled,
                a.field_name_datetime,
                a.field_name_liters,
                a.field_name_total,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "get_invoice_source_mode" => {
            let v = crate::commands_internal::integrations::get_invoice_source_mode_internal(&state.app_dir)?;
            Ok(serde_json::to_value(v).unwrap())
        }

        // ====================================================================
        // Route maps — sync
        // ====================================================================
        //
        // generate_route and route_direct live in dispatcher_async -- they
        // await OSRM. start_route_for_trip stays here: the book's endpoints
        // come from a database lookup, not a geocode, so it awaits nothing.
        "start_route_for_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::start_route_for_trip_internal(&state.db, a.trip_id)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "get_trip_route" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
            }
            let a: Args = parse_args(args)?;
            let v = crate::commands_internal::get_trip_route_internal(&state.db, a.trip_id)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "save_trip_route" => {
            // datasetVersion and createdAt are deliberately absent: the backend
            // stamps both, so a client cannot misreport what it used.
            //
            // roundTrip is `#[serde(default)]`, unlike `mode`: a payload that
            // does not mention a round trip is not describing one, so `false`
            // is the truthful default (design decision 1, Task 20) -- not a
            // guess the way defaulting `mode` would be.
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
                waypoints: Vec<crate::models::Waypoint>,
                polyline: String,
                target_km: f64,
                road_km: f64,
                mode: crate::models::RouteMode,
                #[serde(default)]
                round_trip: bool,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::save_trip_route_internal(
                &state.db,
                &state.app_state,
                a.trip_id,
                a.waypoints,
                a.polyline,
                a.target_km,
                a.road_km,
                a.mode,
                a.round_trip,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "delete_trip_route" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::delete_trip_route_internal(
                &state.db,
                &state.app_state,
                a.trip_id,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }

        // ====================================================================
        // Place book — sync
        // ====================================================================
        //
        // geocode_place lives in dispatcher_async — it awaits Nominatim.
        "list_places" => {
            let v = crate::commands_internal::list_places_internal(&state.db)?;
            Ok(serde_json::to_value(v).unwrap())
        }
        "save_place" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                display_name: String,
                lat: f64,
                lon: f64,
                source: crate::models::PlaceSource,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::save_place_internal(
                &state.db,
                &state.app_state,
                a.display_name,
                a.lat,
                a.lon,
                a.source,
            )?;
            Ok(serde_json::to_value(()).unwrap())
        }
        "clear_place" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                display_name: String,
            }
            let a: Args = parse_args(args)?;
            crate::commands_internal::clear_place_internal(
                &state.db,
                &state.app_state,
                a.display_name,
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
    fn reveal_secret_over_rpc_is_disabled_without_a_configured_pin() {
        let _env = crate::settings::test_env::lock();
        let state = test_state();
        let err = dispatch_sync(
            "reveal_secret",
            json!({ "field": "haApiToken", "pin": "4269" }),
            &state,
        )
        .unwrap_err();
        assert!(err.contains("KNIHA_JAZD_REVEAL_PIN"), "got: {err}");
    }

    #[test]
    fn reveal_secret_over_rpc_requires_the_correct_pin() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = crate::settings::LocalSettings::default();
        s.ha_api_token = Some("file-ha".into());
        s.save(dir.path()).unwrap();

        crate::settings::test_env::with_env_vars(&[("KNIHA_JAZD_REVEAL_PIN", "4269")], || {
            let state = ServerState {
                db: std::sync::Arc::new(crate::db::Database::in_memory().unwrap()),
                app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
                app_dir: dir.path().to_path_buf(),
                static_dir: std::env::temp_dir(),
            };

            let err = dispatch_sync(
                "reveal_secret",
                json!({ "field": "haApiToken", "pin": "0000" }),
                &state,
            )
            .unwrap_err();
            assert!(err.to_lowercase().contains("pin"), "got: {err}");

            let ok = dispatch_sync(
                "reveal_secret",
                json!({ "field": "haApiToken", "pin": "4269" }),
                &state,
            )
            .unwrap();
            assert_eq!(ok, json!("file-ha"));
        });
    }

    /// A missing "pin" argument must not be mistaken for a local caller.
    #[test]
    fn reveal_secret_over_rpc_without_a_pin_argument_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = crate::settings::LocalSettings::default();
        s.ha_api_token = Some("file-ha".into());
        s.save(dir.path()).unwrap();

        crate::settings::test_env::with_env_vars(&[("KNIHA_JAZD_REVEAL_PIN", "4269")], || {
            let state = ServerState {
                db: std::sync::Arc::new(crate::db::Database::in_memory().unwrap()),
                app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
                app_dir: dir.path().to_path_buf(),
                static_dir: std::env::temp_dir(),
            };
            let err = dispatch_sync("reveal_secret", json!({ "field": "haApiToken" }), &state)
                .unwrap_err();
            assert!(!err.contains("file-ha"), "omitting the pin revealed the secret: {err}");
        });
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

    #[test]
    fn restore_backup_roundtrip() {
        // File-backed DB matching get_db_paths_for_dir layout:
        // <app_dir>/kniha-jazd.db, backups in <app_dir>/backups.
        let dir = tempfile::tempdir().unwrap();
        let db =
            crate::db::Database::new(dir.path().join(crate::constants::paths::DB_FILENAME))
                .unwrap();
        let state = ServerState {
            db: std::sync::Arc::new(db),
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: dir.path().to_path_buf(),
            static_dir: std::env::temp_dir(),
        };

        let vehicle_args = |name: &str, plate: &str| {
            json!({
                "name": name,
                "licensePlate": plate,
                "initialOdometer": 0.0,
                "vehicleType": "Ice",
                "tankSizeLiters": 50.0,
                "tpConsumption": 6.5
            })
        };

        // One vehicle → snapshot → second vehicle → restore → one vehicle again.
        dispatch_sync("create_vehicle", vehicle_args("Original", "BA-111AA"), &state).unwrap();
        let backup = dispatch_sync("create_backup", json!({}), &state).unwrap();
        let filename = backup["filename"].as_str().unwrap().to_string();

        dispatch_sync("create_vehicle", vehicle_args("Second", "BA-222BB"), &state).unwrap();
        let vehicles = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(vehicles.as_array().unwrap().len(), 2);

        dispatch_sync("restore_backup", json!({ "filename": filename }), &state).unwrap();

        let vehicles = dispatch_sync("get_vehicles", json!({}), &state).unwrap();
        assert_eq!(vehicles.as_array().unwrap().len(), 1);
        assert_eq!(vehicles[0]["name"], "Original");
    }

    /// The argument names are a contract with `src/lib/api.ts`: a mismatch
    /// compiles cleanly in both languages and only shows up at runtime.
    ///
    /// This payload deliberately OMITS `roundTrip` -- unlike the live
    /// `api.ts::saveTripRoute`, which always sends it (Task 20). That
    /// omission is the point: it pins the backward-compatibility guarantee
    /// that `#[serde(default)] round_trip: bool` exists to give, the same
    /// way an older client (or a caller that predates this field) still
    /// parses and stores `false`. See `save_trip_route_without_round_trip_field_stores_false`
    /// below for the same guarantee pinned explicitly against `get_trip_route`.
    #[test]
    fn route_map_commands_round_trip_with_frontend_argument_names() {
        let state = test_state();
        let vehicle =
            crate::models::Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
        state.db.create_vehicle(&vehicle).unwrap();
        let mut trip = crate::models::Trip::test_ice_trip(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            120.0,
            None,
            true,
        );
        trip.vehicle_id = vehicle.id;
        state.db.create_trip(&trip).unwrap();
        let trip_id = trip.id.to_string();

        dispatch_sync(
            "save_trip_route",
            json!({
                "tripId": trip_id,
                "waypoints": [{ "lat": 48.935, "lon": 20.553, "name": "Domov", "nodeIdx": 0 }],
                "polyline": "_p~iF~ps|U",
                "targetKm": 120.0,
                "roadKm": 118.4,
                "mode": "loop",
            }),
            &state,
        )
        .unwrap();

        let loaded = dispatch_sync("get_trip_route", json!({ "tripId": trip_id }), &state).unwrap();
        assert_eq!(loaded["tripId"], trip_id);
        assert_eq!(loaded["roadKm"], 118.4);
        assert!(
            !loaded["coordinates"].as_array().unwrap().is_empty(),
            "the map must arrive decoded and ready to draw: {loaded}"
        );

        dispatch_sync("delete_trip_route", json!({ "tripId": trip_id }), &state).unwrap();
        assert!(dispatch_sync("get_trip_route", json!({ "tripId": trip_id }), &state)
            .unwrap()
            .is_null());
    }

    /// A trip and vehicle to save a route against, shared by the round-trip
    /// pair below.
    fn seed_trip_for_route(state: &ServerState) -> String {
        let vehicle = crate::models::Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
        state.db.create_vehicle(&vehicle).unwrap();
        let mut trip = crate::models::Trip::test_ice_trip(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            120.0,
            None,
            true,
        );
        trip.vehicle_id = vehicle.id;
        state.db.create_trip(&trip).unwrap();
        trip.id.to_string()
    }

    /// A payload omitting `roundTrip` (every caller before Task 20) must still
    /// parse and store `false` -- `#[serde(default)] round_trip: bool` is the
    /// backward-compatibility guarantee. Paired with the test below that sends
    /// `roundTrip: true`: alone, this test would pass even if the dispatcher
    /// hardcoded `false` regardless of input.
    #[test]
    fn save_trip_route_without_round_trip_field_stores_false() {
        let state = test_state();
        let trip_id = seed_trip_for_route(&state);

        dispatch_sync(
            "save_trip_route",
            json!({
                "tripId": trip_id,
                "waypoints": [
                    { "lat": 48.1486, "lon": 17.1077, "name": "Bratislava" },
                    { "lat": 48.9444, "lon": 20.5675, "name": "Spišská" },
                ],
                "polyline": "_p~iF~ps|U",
                "targetKm": 420.0,
                "roadKm": 400.0,
                "mode": "direct",
            }),
            &state,
        )
        .unwrap();

        let loaded = dispatch_sync("get_trip_route", json!({ "tripId": trip_id }), &state).unwrap();
        assert_eq!(
            loaded["roundTrip"], false,
            "a payload omitting roundTrip must store false, got: {loaded}"
        );
    }

    /// The other half of the pair above: an explicit `roundTrip: true` on a
    /// direct route must be threaded through and stored, not just defaulted.
    #[test]
    fn save_trip_route_with_round_trip_true_stores_true() {
        let state = test_state();
        let trip_id = seed_trip_for_route(&state);

        dispatch_sync(
            "save_trip_route",
            json!({
                "tripId": trip_id,
                "waypoints": [
                    { "lat": 48.1486, "lon": 17.1077, "name": "Bratislava" },
                    { "lat": 48.9444, "lon": 20.5675, "name": "Spišská" },
                    { "lat": 48.1486, "lon": 17.1077, "name": "Bratislava" },
                ],
                "polyline": "_p~iF~ps|U",
                "targetKm": 420.0,
                "roadKm": 400.0,
                "mode": "direct",
                "roundTrip": true,
            }),
            &state,
        )
        .unwrap();

        let loaded = dispatch_sync("get_trip_route", json!({ "tripId": trip_id }), &state).unwrap();
        assert_eq!(
            loaded["roundTrip"], true,
            "an explicit roundTrip: true must be stored, got: {loaded}"
        );
    }

    /// start_route_for_trip must be routed here (the book's endpoints come
    /// from a database lookup, not a geocode, so it awaits nothing) and must
    /// take `tripId`, the name `src/lib/api.ts` sends. A missing id fails
    /// during parsing, so this pins the name without touching the database.
    #[test]
    fn start_route_for_trip_over_rpc_takes_trip_id() {
        let state = test_state();
        let err = dispatch_sync("start_route_for_trip", json!({}), &state).unwrap_err();
        assert!(err.contains("tripId"), "got: {err}");
    }

    /// A same-place row is planned entirely offline: no endpoint is geocoded,
    /// so this exercises the real command through the dispatcher without a
    /// stub or a network call.
    #[test]
    fn start_route_for_trip_plans_a_loop_without_geocoding() {
        let state = test_state();
        let vehicle = crate::models::Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
        state.db.create_vehicle(&vehicle).unwrap();
        let trip = crate::db_tests::seed_trip_between(&state.db, &vehicle.id, "Domov", "domov ");

        let plan = dispatch_sync(
            "start_route_for_trip",
            json!({ "tripId": trip.id.to_string() }),
            &state,
        )
        .unwrap();

        assert_eq!(plan["mode"], "loop");
        assert!(plan["origin"].is_null());
    }

    #[test]
    fn restore_backup_fails_in_read_only_mode() {
        let state = test_state();
        state.app_state.enable_read_only("Test read-only");
        let result = dispatch_sync("restore_backup", json!({ "filename": "x.db" }), &state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("režime len na čítanie"));
    }

    /// With versioned ghcr tags as the deployment unit, "which image am I running"
    /// is the question the settings page most needs to answer.
    #[test]
    fn get_app_version_reports_the_crate_version() {
        let state = test_state();
        let v = dispatch_sync("get_app_version", json!({}), &state).unwrap();
        assert_eq!(v.as_str().unwrap(), env!("CARGO_PKG_VERSION"));
        assert!(!v.as_str().unwrap().is_empty());
    }

    /// Seed one trip naming two places, so the derived list has something in it.
    fn state_with_a_trip_between(origin: &str, destination: &str) -> ServerState {
        let state = test_state();
        let vehicle = crate::models::Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
        state.db.create_vehicle(&vehicle).unwrap();
        crate::db_tests::seed_trip_between(&state.db, &vehicle.id, origin, destination);
        state
    }

    fn place_named<'a>(places: &'a Value, name: &str) -> &'a Value {
        places
            .as_array()
            .expect("list_places must return an array")
            .iter()
            .find(|p| p["displayName"] == name)
            .unwrap_or_else(|| panic!("{name} missing from {places}"))
    }

    /// The argument names are a contract with `src/lib/api.ts`: a mismatch
    /// compiles cleanly in both languages and only shows up at runtime. The
    /// payloads below are exactly what `api.ts` sends.
    #[test]
    fn place_book_commands_round_trip_with_frontend_argument_names() {
        let state = state_with_a_trip_between("Office, City A", "Depot, City B");

        // The list is derived from the trips, so both endpoints appear before
        // anyone has placed anything.
        let places = dispatch_sync("list_places", json!({}), &state).unwrap();
        assert_eq!(places.as_array().unwrap().len(), 2, "got: {places}");
        assert!(
            place_named(&places, "Office, City A")["lat"].is_null(),
            "a place nobody has placed has no coordinate: {places}"
        );

        dispatch_sync(
            "save_place",
            json!({
                "displayName": "Office, City A",
                "lat": 48.1486,
                "lon": 17.1077,
                "source": "geocoder",
            }),
            &state,
        )
        .unwrap();

        let places = dispatch_sync("list_places", json!({}), &state).unwrap();
        let office = place_named(&places, "Office, City A");
        assert_eq!(office["lat"], 48.1486);
        assert_eq!(office["lon"], 17.1077);
        assert_eq!(office["source"], "geocoder");
        assert_eq!(office["uses"], 1);
        assert!(
            place_named(&places, "Depot, City B")["lat"].is_null(),
            "placing one place must not place the other: {places}"
        );

        dispatch_sync(
            "clear_place",
            json!({ "displayName": "Office, City A" }),
            &state,
        )
        .unwrap();

        let places = dispatch_sync("list_places", json!({}), &state).unwrap();
        let office = place_named(&places, "Office, City A");
        assert!(
            office["lat"].is_null(),
            "clear_place left a coordinate: {office}"
        );
        assert_eq!(
            office["uses"], 1,
            "the place itself stays — the trip still names it: {office}"
        );
    }

    /// `source` is a typed `PlaceSource`, not a String, so an unrecognised
    /// value is rejected while the arguments are parsed and never reaches the
    /// database. That holds only as long as the enum has no catch-all: a
    /// `#[serde(other)]` fallback, or loosening the field back to a String,
    /// would silently start storing whatever a client sent.
    #[test]
    fn save_place_rejects_an_unknown_source_at_argument_parsing() {
        let state = state_with_a_trip_between("Office, City A", "Depot, City B");

        let err = dispatch_sync(
            "save_place",
            json!({
                "displayName": "Office, City A",
                "lat": 48.1486,
                "lon": 17.1077,
                "source": "satellite",
            }),
            &state,
        )
        .unwrap_err();
        // The "Invalid args:" prefix is `parse_args`' own and nothing else's,
        // so it separates a rejection during parsing from a command that ran
        // and failed later for some unrelated reason — which a bare `is_err()`
        // would not.
        assert!(
            err.starts_with("Invalid args:"),
            "an unknown source must be refused while parsing, got: {err}"
        );
        assert!(
            err.contains("satellite") && err.contains("geocoder"),
            "the error should name the value rejected and the ones accepted, got: {err}"
        );

        let places = dispatch_sync("list_places", json!({}), &state).unwrap();
        let office = place_named(&places, "Office, City A");
        assert!(
            office["lat"].is_null() && office["source"].is_null(),
            "the rejected save reached the database anyway: {office}"
        );
    }

    /// A refusal has to be a refusal: the guard must stop the write, not report
    /// an error after making it.
    #[test]
    fn save_place_and_clear_place_are_refused_in_read_only_mode() {
        let state = state_with_a_trip_between("Office, City A", "Depot, City B");
        dispatch_sync(
            "save_place",
            json!({
                "displayName": "Office, City A",
                "lat": 48.1486,
                "lon": 17.1077,
                "source": "geocoder",
            }),
            &state,
        )
        .unwrap();

        state.app_state.enable_read_only("Test read-only");

        // Every field differs from what is stored, `source` included, so each
        // assertion below fails on its own if the write got through.
        let err = dispatch_sync(
            "save_place",
            json!({
                "displayName": "Office, City A",
                "lat": 0.0,
                "lon": 0.0,
                "source": "manual",
            }),
            &state,
        )
        .unwrap_err();
        assert!(err.contains("režime len na čítanie"), "got: {err}");

        let err = dispatch_sync(
            "clear_place",
            json!({ "displayName": "Office, City A" }),
            &state,
        )
        .unwrap_err();
        assert!(err.contains("režime len na čítanie"), "got: {err}");

        let places = dispatch_sync("list_places", json!({}), &state).unwrap();
        let office = place_named(&places, "Office, City A");
        assert_eq!(
            office["lat"], 48.1486,
            "the refused save moved the pin anyway: {office}"
        );
        assert_eq!(office["lon"], 17.1077, "got: {office}");
        assert_eq!(
            office["source"], "geocoder",
            "the refused save overwrote the stored source: {office}"
        );
    }

    // ------------------------------------------------------------------
    // recalculate_odometers (Task 3): the controller's override removed the
    // `api.ts` wrapper, so this dispatcher arm is the command's only
    // reachable path in this repo. Nothing else exercises it.
    // ------------------------------------------------------------------

    /// Same shape as `seed_trip_for_route`: one vehicle (initial odometer
    /// 0.0), one trip whose stored odometer (10000.0, from
    /// `Trip::test_ice_trip`) does not match the running total (0.0 + 120.0
    /// km), so a dry run always has something to report.
    fn seed_vehicle_id_with_one_trip(state: &ServerState) -> String {
        let vehicle = crate::models::Vehicle::new_ice("V".into(), "BA-1".into(), 50.0, 6.5, 0.0);
        state.db.create_vehicle(&vehicle).unwrap();
        let mut trip = crate::models::Trip::test_ice_trip(
            chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            120.0,
            None,
            true,
        );
        trip.vehicle_id = vehicle.id;
        state.db.create_trip(&trip).unwrap();
        vehicle.id.to_string()
    }

    /// Pins the wire contract: a JSON array whose objects carry the
    /// camelCase keys `tripId`, `tripNumber`, `oldOdometer`, `newOdometer` --
    /// the fields `api.ts` would need if it ever called this. A typo in the
    /// command string, a wrong field in `Args`, or a serde rename mismatch
    /// on `Vec<OdometerChange>` would be caught by nothing else in this repo.
    #[test]
    fn recalculate_odometers_over_rpc_returns_camelcase_changes() {
        let state = test_state();
        let vehicle_id = seed_vehicle_id_with_one_trip(&state);

        let result = dispatch_sync(
            "recalculate_odometers",
            json!({ "vehicleId": vehicle_id, "year": 2026, "dryRun": true }),
            &state,
        )
        .unwrap();

        let changes = result.as_array().expect("must return a JSON array");
        assert!(
            !changes.is_empty(),
            "the seeded trip's stored odometer does not match the running \
             total, so a dry run must propose at least one change: {result}"
        );
        let change = &changes[0];
        assert!(change.get("tripId").is_some(), "got: {change}");
        assert!(change.get("tripNumber").is_some(), "got: {change}");
        assert!(change.get("oldOdometer").is_some(), "got: {change}");
        assert!(change.get("newOdometer").is_some(), "got: {change}");
    }

    /// `Args::dry_run` has no `#[serde(default)]`, so an omitted `dryRun`
    /// must fail closed rather than silently defaulting to a write -- the
    /// right behaviour on a write command. Pins it so nobody adds a default
    /// later without noticing.
    #[test]
    fn recalculate_odometers_over_rpc_requires_dry_run_field() {
        let state = test_state();
        let vehicle_id = seed_vehicle_id_with_one_trip(&state);

        let result = dispatch_sync(
            "recalculate_odometers",
            json!({ "vehicleId": vehicle_id, "year": 2026 }),
            &state,
        );
        assert!(
            result.is_err(),
            "an omitted dryRun must be refused, not defaulted, on a write command"
        );
    }
}
