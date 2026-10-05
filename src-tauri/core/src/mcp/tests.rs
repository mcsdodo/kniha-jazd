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

/// Rule for the database handle. Every identifier `db` or `*_db` in the
/// guarded files must be one of these, whitespace ignored, else the guard fails:
///   - `use crate::db::Database;` (the module name)
///   - `db: Arc<Database>` (field and parameter)
///   - `Self { db }` (constructor)
///   - `X::new(db)` (hand-off to the reader)
///   - `allowed_fn(&self.db` (passed straight into an ALLOWED_READS function)
///   - `db.allowed_method` where the method is in ALLOWED_READS
/// An alias (`let d = &self.db;`), a `*_db` name or a chain on a new line
/// matches none of these, so it fails closed, even if it only reads.
fn db_violations(code: &str) -> Vec<String> {
    let mut bad = Vec::new();
    let mut i = 0;
    let bytes = code.as_bytes();
    while i < bytes.len() {
        if !(bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
            i += 1;
        }
        let ident = &code[start..i];
        if !ident.ends_with("db") {
            continue;
        }
        let before: String = code[..start].chars().filter(|c| !c.is_whitespace()).collect();
        let after: String = code[i..].chars().filter(|c| !c.is_whitespace()).collect();
        let method: String = after
            .strip_prefix('.')
            .unwrap_or("")
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let passed_to_read = before.strip_suffix("(&self.").map_or(false, |head| {
            let name: String = head
                .chars()
                .rev()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            ALLOWED_READS.contains(&name.as_str())
        });
        let ok = ident == "db"
            && (after.starts_with(":Arc<Database>")
                || (before.ends_with("crate::") && after.starts_with("::Database;"))
                || (before.ends_with("Self{") && after.starts_with('}'))
                || (before.ends_with("::new(") && after.starts_with(')'))
                || passed_to_read
                || (!method.is_empty() && ALLOWED_READS.contains(&method.as_str())));
        if !ok {
            let tail: String = after.chars().take(30).collect();
            bad.push(format!("`{ident}` used as `...{tail}`"));
        }
    }
    bad
}

/// Every `.rs` file the read path owns, except this test file.
fn guarded_sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().map_or(false, |e| e == "rs")
                && path.file_name().map_or(false, |n| n != "tests.rs")
            {
                out.push(path);
            }
        }
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut paths = Vec::new();
    walk(&src.join("mcp"), &mut paths);
    walk(&src.join("journeys"), &mut paths);
    paths.push(src.join("commands_internal/journeys_cmd.rs"));
    paths
        .into_iter()
        .map(|p| (p.display().to_string(), std::fs::read_to_string(&p).unwrap()))
        .collect()
}

#[test]
fn mcp_read_path_has_no_write_access() {
    // Read-only by construction (ADR "Read-only MCP endpoint"). The MCP module
    // holds only a LogbookReader. Checks, on every guarded file (see
    // `guarded_sources`, so a new file in mcp/ cannot dodge them):
    //  1. allowlist: every `crate::` item and every `db.` method is in ALLOWED_READS
    //  2. every `db` / `*_db` identifier follows the rule in `db_violations`
    //  3. no `Database::` / `Vehicle::` associated calls, no `use ... as ...`
    //  4. denylist of words that must not appear at all
    let sources = guarded_sources();
    assert!(sources.len() >= 3, "guarded files not found");
    for (file, source) in &sources {
        // Comment lines are prose, not calls.
        let code: String = source
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        let source = code.as_str();
        assert!(!source.contains("::*"), "{file}: glob imports hide what is called");
        assert!(!source.contains("super::"), "{file}: use full `crate::` paths");
        for line in source.lines().filter(|l| l.trim_start().starts_with("use ")) {
            assert!(!line.contains(" as "), "{file}: `use ... as` hides a name: {line}");
        }
        for assoc in ["Database::", "Vehicle::"] {
            for (pos, _) in source.match_indices(assoc) {
                let before = source[..pos].chars().next_back();
                let part_of_longer = before.map_or(false, |c| c.is_alphanumeric() || c == '_');
                assert!(part_of_longer, "{file}: associated call `{assoc}...` is not allowed");
            }
        }
        let mut used = names_after(source, "crate::");
        used.extend(names_after(source, "commands_internal::"));
        for name in used {
            assert!(
                ALLOWED_READS.contains(&name.as_str()),
                "{file} uses `{name}`, which is not in ALLOWED_READS"
            );
        }
        let bad = db_violations(source);
        assert!(bad.is_empty(), "{file}: db handle used outside the allowed forms: {bad:?}");
        let banned = [
            "check_read_only", "connection", "restore", "sql_query", "execute", "transaction",
            "create_", "update_", "delete_", "save_", "set_", "upsert", "insert",
        ];
        for word in banned {
            assert!(!source.contains(word), "{file} must not contain `{word}`");
        }
    }
}
