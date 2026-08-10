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
        // Statistics — async because of the fire-and-forget HA push
        // ====================================================================
        //
        // Handled here rather than in dispatch_sync so the server performs the
        // same suggested-fillup push as the Tauri wrapper. Keeping the push in
        // only one of the two frontends is exactly how it silently stopped
        // working when the server became the canonical deployment (ADR-024).
        "get_trip_grid_data" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                vehicle_id: String,
                year: i32,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let grid_data =
                match crate::commands_internal::build_trip_grid_data(&state.db, &a.vehicle_id, a.year)
                {
                    Ok(g) => g,
                    Err(e) => return Some(Err(e)),
                };

            if let Ok(Some(vehicle)) = state.db.get_vehicle(&a.vehicle_id) {
                if let Some((entity_id, value)) =
                    crate::commands_internal::integrations::ha_fillup_push_payload(
                        &vehicle, &grid_data,
                    )
                {
                    let app_dir = state.app_dir.clone();
                    tokio::spawn(crate::commands_internal::integrations::push_ha_input_text(
                        app_dir, entity_id, value,
                    ));
                }
            }

            Some(Ok(serde_json::to_value(grid_data).unwrap()))
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

        // ====================================================================
        // Invoices — async (1, Paperless fetch required for writes)
        // ====================================================================
        "assign_invoice_to_trip" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                invoice_ref: crate::invoice::InvoiceRef,
                trip_id: String,
                vehicle_id: String,
                assignment_type: crate::models::AssignmentType,
                mismatch_override: bool,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let paperless_doc = match &a.invoice_ref {
                crate::invoice::InvoiceRef::Paperless(id) => {
                    match crate::commands_internal::paperless_cmd::fetch_paperless_doc_by_id(
                        &state.app_dir, *id,
                    ).await {
                        Ok(doc) => Some(doc),
                        Err(e) => return Some(Err(e.to_string())),
                    }
                }
                crate::invoice::InvoiceRef::Receipt(_) => None,
            };
            let result = crate::commands_internal::invoices::assign_invoice_to_trip_internal(
                &state.db,
                &state.app_state,
                &a.invoice_ref,
                paperless_doc.as_ref(),
                &a.trip_id,
                &a.vehicle_id,
                a.assignment_type,
                a.mismatch_override,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path as wm_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Regression guard for the gap this module's `get_trip_grid_data` arm closes:
    /// the push used to exist only in the Tauri wrapper, so the server — the
    /// canonical deployment — never pushed anything to Home Assistant.
    #[tokio::test]
    async fn get_trip_grid_data_pushes_suggested_fillup_to_ha() {
        let _env = crate::settings::test_env::lock();
        let ha = MockServer::start().await;
        Mock::given(method("POST"))
            .and(wm_path("/api/services/input_text/set_value"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&ha)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let mut settings = crate::settings::LocalSettings::default();
        settings.ha_url = Some(ha.uri());
        settings.ha_api_token = Some("ha-token".into());
        settings.save(dir.path()).unwrap();

        let db = std::sync::Arc::new(crate::db::Database::in_memory().unwrap());
        let mut vehicle =
            crate::models::Vehicle::new_ice("Test".into(), "BA-123AB".into(), 50.0, 6.5, 0.0);
        vehicle.ha_fillup_sensor = Some("input_text.kniha_jazd_fillup".into());
        db.create_vehicle(&vehicle).unwrap();

        let state = ServerState {
            db,
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: dir.path().to_path_buf(),
            static_dir: std::env::temp_dir(),
        };

        let result = dispatch_async(
            "get_trip_grid_data",
            json!({ "vehicleId": vehicle.id.to_string(), "year": 2026 }),
            &state,
        )
        .await;
        assert!(result.is_some(), "get_trip_grid_data must be handled here, not by dispatch_sync");
        assert!(result.unwrap().is_ok());

        // The push is spawned, so poll rather than assume it already landed.
        let mut requests = Vec::new();
        for _ in 0..50 {
            requests = ha.received_requests().await.unwrap_or_default();
            if !requests.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert_eq!(requests.len(), 1, "server path must push the suggested fillup to HA");

        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["entity_id"], "input_text.kniha_jazd_fillup");
        // No trips seeded → no open period → "full tank"
        assert_eq!(body["value"], "Plná nádrž");
    }

    /// A vehicle without the helper configured must not generate HA traffic.
    #[tokio::test]
    async fn get_trip_grid_data_does_not_push_without_sensor() {
        let _env = crate::settings::test_env::lock();
        let ha = MockServer::start().await;
        Mock::given(method("POST"))
            .and(wm_path("/api/services/input_text/set_value"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&ha)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let mut settings = crate::settings::LocalSettings::default();
        settings.ha_url = Some(ha.uri());
        settings.ha_api_token = Some("ha-token".into());
        settings.save(dir.path()).unwrap();

        let db = std::sync::Arc::new(crate::db::Database::in_memory().unwrap());
        let vehicle =
            crate::models::Vehicle::new_ice("Test".into(), "BA-123AB".into(), 50.0, 6.5, 0.0);
        db.create_vehicle(&vehicle).unwrap();

        let state = ServerState {
            db,
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: dir.path().to_path_buf(),
            static_dir: std::env::temp_dir(),
        };

        dispatch_async(
            "get_trip_grid_data",
            json!({ "vehicleId": vehicle.id.to_string(), "year": 2026 }),
            &state,
        )
        .await
        .unwrap()
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        assert!(ha.received_requests().await.unwrap_or_default().is_empty());
    }
}
