//! Async RPC dispatcher — handles commands that require `.await`.

use serde_json::Value;

use super::ServerState;

/// Deserialize JSON args into a typed struct, returning a human-readable error.
fn parse_args<T: serde::de::DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|e| format!("Invalid args: {e}"))
}

/// Older callers omit `sortDirection`; keep their behaviour (oldest first).
fn default_sort_direction() -> String {
    "asc".to_string()
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
        // Handled here rather than in dispatch_sync because the push needs an
        // async runtime. The push once lived outside the dispatcher entirely,
        // which is exactly how it silently stopped working when the server
        // became the canonical deployment (ADR-024).
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
                #[serde(default)]
                hidden_columns: Vec<String>,
                #[serde(default = "default_sort_direction")]
                sort_direction: String,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let result = crate::commands_internal::export_html_internal(
                &state.db,
                &state.app_dir,
                a.vehicle_id,
                a.year,
                a.labels,
                a.hidden_columns,
                a.sort_direction,
            )
            .await;
            Some(result.map(|v| serde_json::to_value(v).unwrap()))
        }

        // ====================================================================
        // Route maps — async (1, OSRM geometry fetch)
        // ====================================================================
        //
        // The other three route map commands are sync and live in dispatcher.rs.
        "generate_route" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Args {
                target_km: f64,
            }
            let a: Args = match parse_args(args) {
                Ok(a) => a,
                Err(e) => return Some(Err(e)),
            };
            let provider = crate::route_map::HttpRouteProvider::public();
            let result =
                crate::commands_internal::generate_route_internal(&provider, a.target_km).await;
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
    /// the push used to live outside the dispatcher, so the server — the
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

    /// generate_route must be routed here (it awaits OSRM) and must take
    /// `targetKm`, the name `src/lib/api.ts` sends. Bad args fail during
    /// parsing, so this pins both without touching the network.
    #[tokio::test]
    async fn generate_route_is_an_async_command_taking_target_km() {
        let state = ServerState {
            db: std::sync::Arc::new(crate::db::Database::in_memory().unwrap()),
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: std::env::temp_dir(),
            static_dir: std::env::temp_dir(),
        };

        let result = dispatch_async("generate_route", json!({}), &state).await;
        let err = result
            .expect("generate_route must be handled here, not by dispatch_sync")
            .unwrap_err();
        assert!(err.contains("targetKm"), "got: {err}");
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

    /// Web-mode export must honour the user's column visibility and sort choice,
    /// and must prepend the synthetic year-opening row. Before Task 73 all three
    /// were hardcoded away, so the browser export silently disagreed with the
    /// grid the user was looking at.
    #[tokio::test]
    async fn export_html_honours_hidden_columns_and_sort_direction() {
        use crate::models::{Settings, Trip};
        use chrono::{NaiveDate, Utc};

        let dir = tempfile::tempdir().unwrap();
        let db = std::sync::Arc::new(crate::db::Database::in_memory().unwrap());

        let vehicle =
            crate::models::Vehicle::new_ice("Test".into(), "BA-123AB".into(), 50.0, 6.5, 10_000.0);
        db.create_vehicle(&vehicle).unwrap();
        db.save_settings(&Settings::default()).unwrap();

        // Two trips on different days, so sort direction is observable.
        let make_trip = |day: u32, destination: &str, odo: f64| Trip {
            id: uuid::Uuid::new_v4(),
            vehicle_id: vehicle.id,
            start_datetime: NaiveDate::from_ymd_opt(2026, 3, day)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap(),
            end_datetime: None,
            origin: "Bratislava".into(),
            destination: destination.into(),
            distance_km: 60.0,
            odometer: odo,
            purpose: "test".into(),
            fuel_liters: None,
            fuel_cost_eur: None,
            full_tank: false,
            energy_kwh: None,
            energy_cost_eur: None,
            full_charge: false,
            soc_override_percent: None,
            other_costs_eur: None,
            other_costs_note: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        db.create_trip(&make_trip(1, "TRNAVA", 10_060.0)).unwrap();
        db.create_trip(&make_trip(2, "KOSICE", 10_120.0)).unwrap();

        let state = ServerState {
            db,
            app_state: std::sync::Arc::new(crate::app_state::AppState::new()),
            app_dir: dir.path().to_path_buf(),
            static_dir: std::env::temp_dir(),
        };

        // "time" is one of the five hideable columns (grep `is_visible("` in
        // export.rs for the full set: time, fuelConsumed, fuelRemaining,
        // otherCosts, otherCostsNote). It renders the `col_end_datetime`
        // header, so give THAT label a distinctive marker - presence/absence in
        // the HTML is then unambiguous.
        let mut labels = serde_json::to_value(crate::export::sample_export_labels()).unwrap();
        labels["col_end_datetime"] = serde_json::json!("CAS-MARKER");

        let visible = dispatch_async(
            "export_html",
            serde_json::json!({
                "vehicleId": vehicle.id.to_string(),
                "year": 2026,
                "labels": labels.clone(),
                "hiddenColumns": [],
                "sortDirection": "desc"
            }),
            &state,
        )
        .await
        .unwrap()
        .unwrap();
        let visible = visible.as_str().unwrap();
        assert!(
            visible.contains("CAS-MARKER"),
            "time column should render when not hidden"
        );

        let hidden = dispatch_async(
            "export_html",
            serde_json::json!({
                "vehicleId": vehicle.id.to_string(),
                "year": 2026,
                "labels": labels.clone(),
                "hiddenColumns": ["time"],
                "sortDirection": "desc"
            }),
            &state,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            !hidden.as_str().unwrap().contains("CAS-MARKER"),
            "hiddenColumns was ignored - the time column still rendered"
        );

        // sortDirection: "desc" puts the newest trip first. The `visible` render
        // above already used "desc", so compare against an "asc" render.
        let ascending = dispatch_async(
            "export_html",
            serde_json::json!({
                "vehicleId": vehicle.id.to_string(),
                "year": 2026,
                "labels": labels,
                "hiddenColumns": [],
                "sortDirection": "asc"
            }),
            &state,
        )
        .await
        .unwrap()
        .unwrap();
        let ascending = ascending.as_str().unwrap();

        let desc_kosice = visible
            .find("KOSICE")
            .expect("KOSICE missing from desc render");
        let desc_trnava = visible
            .find("TRNAVA")
            .expect("TRNAVA missing from desc render");
        assert!(
            desc_kosice < desc_trnava,
            "sortDirection=desc should put the newer trip (KOSICE) first"
        );

        let asc_kosice = ascending
            .find("KOSICE")
            .expect("KOSICE missing from asc render");
        let asc_trnava = ascending
            .find("TRNAVA")
            .expect("TRNAVA missing from asc render");
        assert!(
            asc_trnava < asc_kosice,
            "sortDirection=asc should put the older trip (TRNAVA) first - \
             the argument was ignored"
        );

        // The synthetic year-opening row. Desktop prepends it; web mode never did,
        // so after the migration the printed logbook would silently lose the
        // baseline odometer the on-screen grid still shows.
        assert!(
            ascending.contains("Prvý záznam"),
            "the synthetic first-record row is missing from the web export"
        );
        assert!(
            ascending.contains("10000"),
            "the first-record row should carry year_start_odometer (10000)"
        );
    }
}
