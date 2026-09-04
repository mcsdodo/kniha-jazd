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
        // Trips (10)
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
        // Settings (14)
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
        // Receipts (11)
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
        // Backup (11)
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
        // Integrations — sync only (6)
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
        // Route maps — sync (3)
        // ====================================================================
        //
        // generate_route lives in dispatcher_async — it awaits OSRM.
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
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                trip_id: String,
                waypoints: Vec<crate::models::Waypoint>,
                polyline: String,
                target_km: f64,
                road_km: f64,
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
    /// compiles cleanly in both languages and only shows up at runtime. The
    /// payloads below are exactly what `api.ts` sends.
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
}
