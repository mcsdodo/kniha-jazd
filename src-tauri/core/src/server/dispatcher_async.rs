//! Async RPC dispatcher — handles commands that require `.await`.

use serde_json::Value;

use super::ServerState;

/// Deserialize JSON args into a typed struct, returning a human-readable error.
fn parse_args<T: serde::de::DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|e| format!("Invalid args: {e}"))
}

/// Try to dispatch an async command.
///
/// Returns `None` if the command is not handled here (caller should fall
/// through to the sync dispatcher).  Returns `Some(Ok(Value))` or
/// `Some(Err(message))` for known async commands.
pub async fn dispatch_async(
    command: &str,
    args: Value,
    state: &ServerState,
) -> Option<Result<Value, String>> {
    match command {
        // ====================================================================
        // Receipts — async (3)
        // ====================================================================
        "sync_receipts" => {
            let result = crate::commands_internal::receipts_cmd::sync_receipts_internal(
                &state.db,
                &state.app_state,
                &state.app_dir,
            )
            .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
        "process_pending_receipts" => {
            let result =
                crate::commands_internal::receipts_cmd::process_pending_receipts_internal(
                    &state.db,
                    &state.app_dir,
                )
                .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
        "reprocess_receipt" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                id: String,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result = crate::commands_internal::receipts_cmd::reprocess_receipt_internal(
                &state.db,
                &state.app_state,
                &state.app_dir,
                a.id,
            )
            .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }

        // ====================================================================
        // Integrations — async (6)
        // ====================================================================
        "test_ha_connection" => {
            let result =
                crate::commands_internal::integrations::test_ha_connection_internal(&state.app_dir).await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
        "fetch_ha_odo" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                sensor_id: String,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result =
                crate::commands_internal::integrations::fetch_ha_odo_internal(&state.app_dir, a.sensor_id).await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
        "test_paperless_connection" => {
            let result =
                crate::commands_internal::integrations::test_paperless_connection_internal(&state.app_dir).await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }
        "get_paperless_invoices" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { vehicle_id: String, year: i32 }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result = crate::commands_internal::paperless_cmd::get_paperless_invoices_internal(
                &state.app_dir, &state.db, &a.vehicle_id, a.year,
            ).await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()).map_err(|e| e.to_string()))
        }
        "list_paperless_custom_fields" => {
            let result = crate::commands_internal::paperless_cmd::list_paperless_custom_fields_internal(
                &state.app_dir,
            ).await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()).map_err(|e| e.to_string()))
        }
        "assign_paperless_doc_to_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { doc_id: i64, trip_id: String }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result = crate::commands_internal::paperless_cmd::assign_paperless_doc_to_trip_internal(
                &state.app_state, &state.db, a.doc_id, &a.trip_id,
            );
            Some(result.map(|_| serde_json::to_value(()).unwrap()))
        }
        "unassign_paperless_doc" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args { doc_id: i64 }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result = crate::commands_internal::paperless_cmd::unassign_paperless_doc_internal(
                &state.app_state, &state.db, a.doc_id,
            );
            Some(result.map(|_| serde_json::to_value(()).unwrap()))
        }

        // ====================================================================
        // Export — async (1)
        // ====================================================================
        "export_html" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
                labels: crate::export::ExportLabels,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result = crate::commands_internal::export_html_internal(
                &state.db,
                a.vehicle_id,
                a.year,
                a.labels,
            )
            .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }

        // Not an async command — let the caller fall through to sync dispatch.
        _ => None,
    }
}
