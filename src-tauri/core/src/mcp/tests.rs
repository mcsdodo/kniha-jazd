//! `/mcp` over real HTTP, through `HttpServer::start` (the same router the
//! container serves), plus the read-only source guard.

use std::sync::Arc;

use serde_json::{json, Value};

use crate::app_state::AppState;
use crate::db::Database;
use crate::server::HttpServer;

async fn start(db: Arc<Database>) -> (String, tokio::sync::oneshot::Sender<()>) {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let tmp = std::env::temp_dir();
    let addr = HttpServer::start(db, Arc::new(AppState::new()), tmp.clone(), tmp, 0, false, rx)
        .await
        .unwrap();
    (format!("http://{addr}/mcp"), tx)
}

async fn post(url: &str, body: Value, extra: &[(&str, &str)]) -> Value {
    let mut req = reqwest::Client::new()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream");
    for (k, v) in extra {
        req = req.header(*k, *v);
    }
    let resp = req.body(body.to_string()).send().await.unwrap();
    assert_eq!(resp.status(), 200, "HTTP status");
    resp.json().await.unwrap()
}

fn call(id: u32, name: &str, args: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
           "params": {"name": name, "arguments": args}})
}

fn tool_names(body: &Value) -> Vec<String> {
    let mut names: Vec<String> = body["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn tools_list_returns_the_read_only_tools() {
    let (url, stop) = start(Arc::new(Database::in_memory().unwrap())).await;

    let body = post(&url, json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}), &[]).await;

    // Phase A: one tool. Task B3 changes this to the three tools.
    assert_eq!(tool_names(&body), vec!["list_vehicles"]);
    for tool in body["result"]["tools"].as_array().unwrap() {
        let description = tool["description"].as_str().unwrap();
        assert!(description.contains("Read-only"), "{description}");
    }
    let _ = stop.send(());
}

#[tokio::test]
async fn stale_session_id_and_public_host_are_accepted() {
    let (url, stop) = start(Arc::new(Database::in_memory().unwrap())).await;

    let body = post(&url, call(4, "list_vehicles", json!({})),
        &[("Mcp-Session-Id", "from-before-a-restart"), ("Host", "logbook.example.org")]).await;

    assert_eq!(body["result"]["structuredContent"]["vehicles"], json!([]));
    let _ = stop.send(());
}

#[tokio::test]
async fn list_vehicles_returns_the_vehicle_fields() {
    let db = Arc::new(Database::in_memory().unwrap());
    let v = crate::db_tests::create_test_vehicle("Car");
    db.create_vehicle(&v).unwrap();
    let (url, stop) = start(db).await;

    let body = post(&url, call(6, "list_vehicles", json!({})), &[]).await;

    let vehicles = &body["result"]["structuredContent"]["vehicles"];
    assert_eq!(vehicles[0]["id"], v.id.to_string());
    assert_eq!(vehicles[0]["name"], "Car");
    assert_eq!(vehicles[0]["license_plate"], v.license_plate);
    assert_eq!(vehicles[0]["is_active"], v.is_active);
    let _ = stop.send(());
}

/// Items the MCP read path may name from `crate::` paths, plus the methods it
/// may call on `db` / `self.db`. Phase B adds its read functions here by name.
/// Anything else (for example `clear_place_internal`) fails the guard.
const ALLOWED_READS: &[&str] = &[
    // read functions
    "get_vehicles_internal",
    // types and the reader itself
    "LogbookReader", "ReadError", "Database", "Vehicle",
];

/// Names after `marker` up to the end of the path or `use` list. A brace list
/// gives one name per entry; a plain path gives its last segment.
fn names_after(source: &str, marker: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (pos, _) in source.match_indices(marker) {
        let rest = &source[pos + marker.len()..];
        let end = rest.find(|c| c == ';' || c == '(' || c == ')').unwrap_or(rest.len());
        let path: String = rest[..end].chars().filter(|c| !c.is_whitespace()).collect();
        assert!(path.matches('{').count() <= 1, "nested use lists are not supported: {path}");
        let items: Vec<&str> = match path.split_once('{') {
            Some((_, list)) => list.trim_end_matches('}').split(',').collect(),
            None => vec![path.as_str()],
        };
        for item in items.into_iter().filter(|i| !i.is_empty()) {
            names.push(item.rsplit("::").next().unwrap().to_string());
        }
    }
    names
}

/// Method names called on `db.` / `self.db.`.
fn db_methods(source: &str) -> Vec<String> {
    let mut methods = Vec::new();
    for (pos, _) in source.match_indices("db.") {
        let before = source[..pos].chars().next_back();
        if before.map_or(false, |c| c.is_alphanumeric() || c == '_') {
            continue; // part of a longer name such as `my_db.`
        }
        let ident: String = source[pos + 3..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        methods.push(ident);
    }
    methods
}

#[test]
fn mcp_read_path_has_no_write_access() {
    // Read-only by construction (ADR "Read-only MCP endpoint"). The MCP module
    // holds only a LogbookReader. Check 1 is an allowlist: every `crate::` item
    // and every `db.` method in these files must be in ALLOWED_READS. Check 2 is
    // a denylist of words that must not appear at all.
    let sources = [
        ("mcp/mod.rs", include_str!("mod.rs")),
        ("commands_internal/journeys_cmd.rs", include_str!("../commands_internal/journeys_cmd.rs")),
        ("journeys/mod.rs", include_str!("../journeys/mod.rs")),
    ];
    for (file, source) in sources {
        // Comment lines are prose, not calls.
        let code: String = source
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = code.as_str();
        assert!(!source.contains("::*"), "{file}: glob imports hide what is called");
        assert!(!source.contains("super::"), "{file}: use full `crate::` paths");
        let mut used = names_after(source, "crate::");
        used.extend(names_after(source, "commands_internal::"));
        used.extend(db_methods(source));
        for name in used {
            assert!(
                ALLOWED_READS.contains(&name.as_str()),
                "{file} uses `{name}`, which is not in ALLOWED_READS"
            );
        }
    }
    let banned = [
        "check_read_only", "connection", "restore", "sql_query", "execute", "transaction",
        "create_", "update_", "delete_", "save_", "set_", "upsert", "insert",
    ];
    for (file, source) in sources {
        for word in banned {
            assert!(!source.contains(word), "{file} must not contain `{word}`");
        }
    }
}
