//! Read-only MCP endpoint at `/mcp` (task 89, ADR "Read-only MCP endpoint").
//!
//! Stateless streamable HTTP with `rmcp`: no session IDs, so a client that
//! cached one keeps working after a restart. The tools call `LogbookReader`
//! directly, not `/api/rpc`. This module never holds a `Database`; the test
//! `mcp_read_path_has_no_write_access` checks the sources.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Json;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::{schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler};
use serde::Serialize;

use crate::commands_internal::{LogbookReader, ReadError};
use crate::db::Database;

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
}

#[tool_handler]
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
