//! Read-only MCP endpoint at `/mcp` (task 89, ADR "Read-only MCP endpoint").
//!
//! Stateless streamable HTTP with `rmcp`: no session IDs, so a client that
//! cached one keeps working after a restart. The tools call `LogbookReader`
//! directly, not `/api/rpc`. This module never holds a `Database`; the test
//! `mcp_read_path_has_no_write_access` checks the sources.

use std::sync::Arc;

use chrono::{NaiveDate, NaiveDateTime};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::{Deserialize, Serialize};

use crate::commands_internal::{LogbookReader, ReadError};
use crate::db::Database;

// ---------------------------------------------------------------- inputs

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RangeArgs {
    /// First day, YYYY-MM-DD, inclusive.
    pub date_from: String,
    /// Last day, YYYY-MM-DD, inclusive.
    pub date_to: String,
    /// Vehicle ID. Leave out for all vehicles.
    #[serde(default)]
    pub vehicle_id: Option<String>,
}

// ---------------------------------------------------------------- outputs

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VehicleOut {
    pub id: String,
    pub name: String,
    pub license_plate: String,
    /// True for the one vehicle that is selected in the app UI.
    pub is_active: bool,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct VehicleList {
    pub vehicles: Vec<VehicleOut>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TripOut {
    pub id: String,
    pub vehicle_id: String,
    /// YYYY-MM-DDTHH:MM
    pub start: String,
    /// YYYY-MM-DDTHH:MM, or null if the trip has no end time.
    pub end: Option<String>,
    pub origin: String,
    pub destination: String,
    pub distance_km: f64,
    pub purpose: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct TripList {
    pub trips: Vec<TripOut>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct JourneyOut {
    pub vehicle_id: String,
    /// Start of the first leg, YYYY-MM-DDTHH:MM.
    pub start: String,
    /// Start of the return leg, or null if the journey is incomplete.
    pub end: Option<String>,
    /// Calendar days between the start date and the end date, or null.
    pub nights: Option<i64>,
    pub total_km: f64,
    /// Distinct destinations in order, without home.
    pub places: Vec<String>,
    pub purposes: Vec<String>,
    pub complete: bool,
    pub leg_ids: Vec<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct JourneyListOut {
    pub home_place: String,
    pub journeys: Vec<JourneyOut>,
}

fn minute(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M").to_string()
}

/// Strict YYYY-MM-DD: "2026-1-1" is refused, not read as January 1.
fn parse_day(field: &str, value: &str) -> Result<NaiveDate, ErrorData> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .filter(|d| d.format("%Y-%m-%d").to_string() == value)
        .ok_or_else(|| {
            ErrorData::invalid_params(format!("{field} must be YYYY-MM-DD, got '{value}'"), None)
        })
}

fn to_error(e: ReadError) -> ErrorData {
    match e {
        ReadError::Invalid(m) => ErrorData::invalid_params(m, None),
        ReadError::Internal(m) => ErrorData::internal_error(m, None),
    }
}

/// Run a read on the blocking pool, the same as `rpc_handler` does for DB work.
async fn blocking<T, F>(read: F) -> Result<T, ErrorData>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ReadError> + Send + 'static,
{
    tokio::task::spawn_blocking(read)
        .await
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?
        .map_err(to_error)
}

// ---------------------------------------------------------------- server

#[derive(Clone)]
pub struct KnihaJazdMcp {
    reader: LogbookReader,
    tool_router: ToolRouter<Self>,
}

impl KnihaJazdMcp {
    pub fn new(reader: LogbookReader) -> Self {
        Self { reader, tool_router: Self::tool_router() }
    }
}

#[tool_router]
impl KnihaJazdMcp {
    #[tool(description = "Read-only. Lists all vehicles of the logbook: id, name, license_plate, is_active (true for the vehicle selected in the app UI).")]
    async fn list_vehicles(&self) -> Result<Json<VehicleList>, ErrorData> {
        let reader = self.reader.clone();
        let vehicles = blocking(move || reader.vehicles()).await?;
        Ok(Json(VehicleList {
            vehicles: vehicles
                .into_iter()
                .map(|v| VehicleOut {
                    id: v.id.to_string(),
                    name: v.name,
                    license_plate: v.license_plate,
                    is_active: v.is_active,
                })
                .collect(),
        }))
    }

    #[tool(description = "Read-only. Lists the trip rows (legs) whose start is between date_from and date_to (YYYY-MM-DD, both inclusive), sorted by start time. Without vehicle_id: all vehicles.")]
    async fn list_trips(&self, Parameters(args): Parameters<RangeArgs>) -> Result<Json<TripList>, ErrorData> {
        let from = parse_day("date_from", &args.date_from)?;
        let to = parse_day("date_to", &args.date_to)?;
        let reader = self.reader.clone();
        let trips = blocking(move || reader.trips_in_range(from, to, args.vehicle_id.as_deref())).await?;
        Ok(Json(TripList {
            trips: trips
                .into_iter()
                .map(|t| TripOut {
                    id: t.id.to_string(),
                    vehicle_id: t.vehicle_id.to_string(),
                    start: minute(t.start_datetime),
                    end: t.end_datetime.map(minute),
                    origin: t.origin,
                    destination: t.destination,
                    distance_km: t.distance_km,
                    purpose: t.purpose,
                })
                .collect(),
        }))
    }

    #[tool(description = "Read-only. Groups trip legs into journeys away from the home place and returns each journey that overlaps date_from..date_to (YYYY-MM-DD, both inclusive). Without vehicle_id: all vehicles. A journey starts at a leg from home and ends at the next leg back home. Single-leg day trips and home-to-home loops are not journeys. complete=false means no return leg exists: the car has not come back yet, or it left home again before a return leg was recorded. Error if no home place is set. The app applies no accounting rule: filter on nights and total_km yourself.")]
    async fn list_journeys(&self, Parameters(args): Parameters<RangeArgs>) -> Result<Json<JourneyListOut>, ErrorData> {
        let from = parse_day("date_from", &args.date_from)?;
        let to = parse_day("date_to", &args.date_to)?;
        let reader = self.reader.clone();
        let list = blocking(move || reader.journeys(from, to, args.vehicle_id.as_deref())).await?;
        Ok(Json(JourneyListOut {
            home_place: list.home_place,
            journeys: list
                .journeys
                .into_iter()
                .map(|j| JourneyOut {
                    vehicle_id: j.vehicle_id.to_string(),
                    start: minute(j.start),
                    end: j.end.map(minute),
                    nights: j.nights,
                    total_km: j.total_km,
                    places: j.places,
                    purposes: j.purposes,
                    complete: j.complete,
                    leg_ids: j.leg_ids.iter().map(|id| id.to_string()).collect(),
                })
                .collect(),
        }))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for KnihaJazdMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("kniha-jazd", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Read-only access to a vehicle logbook (Kniha jázd). No tool can change data.",
            )
    }
}

/// The `/mcp` service. Stateless: no session store, no SSE keep-alive, JSON
/// answers. The host check is off: behind a reverse proxy the Host header is
/// the public name, and `/api/rpc` (which can write) has no host check either.
pub fn mcp_service(db: Arc<Database>) -> StreamableHttpService<KnihaJazdMcp, NeverSessionManager> {
    let reader = LogbookReader::new(db);
    let config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .disable_allowed_hosts();
    StreamableHttpService::new(
        move || Ok(KnihaJazdMcp::new(reader.clone())),
        Default::default(),
        config,
    )
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
