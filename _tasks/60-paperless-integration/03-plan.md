**Date:** 2026-05-03
**Subject:** Paperless-ngx integration — implementation plan
**Status:** Ready

# Paperless-ngx Integration — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement
> this plan task-by-task. Fresh subagent per task; do not bundle.

**Companion docs:** [01-task.md](./01-task.md), [02-design.md](./02-design.md), [description.md](./description.md)

**Goal:** Make the [doklady page](../../src/routes/doklady/+page.svelte) sourceable
from [Paperless-ngx](https://docs.paperless-ngx.com/) (toggled by populating
[Settings](../../src/routes/settings/+page.svelte) → Paperless), so invoices
already stored in Paperless are reused without re-OCR. Local-mode behavior is
preserved.

**Architecture:** A backend mode-switch command drives a single SvelteKit page
renderer. A new [paperless_trip_links](../../src-tauri/core/migrations/) table holds
1:1 trip↔doc links. The existing
[receipts](../../src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql)
table is untouched. All conditional logic lives in Rust
([ADR-008 in DECISIONS.md](../../DECISIONS.md)).

**Tech Stack:** Rust + Diesel + SQLite (backend), `reqwest` (HTTP, already in
dependency tree per HA — see [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml)),
SvelteKit + TypeScript (frontend), WebdriverIO (Tier 2,
see [tests/integration/](../../tests/integration/)).

---

## Design Adjustments from Live-API Probing (2026-05-03)

I probed [https://documents.lacny.me](https://documents.lacny.me) against the
user-supplied PAT before drafting, and reviewed three reference documents (435 =
fuel, 423 = parking/other, 391 = car wash/other) the user pointed to. Three
clarifications/corrections vs. [02-design.md](./02-design.md):

### 1. Add `litres` to the custom-field map (not in original design)

Doc 435 (fuel) carries a custom field `litres` (id 5, type `float`) with value
`63.34`. The existing local-receipts grid shows liters per fuel row; without the
`litres` lookup, Paperless mode would regress that column for fuel docs. Doc 423
(parking) and 391 (car wash) do **not** carry `litres` — that's expected: car-tagged
docs don't track liters.

**Effect:** add a third name → ID resolution and surface it as `Option<f64>` on
`PaperlessInvoiceRow`. Resolution failure (field missing in Paperless) is a hard
sync error, same pattern as `total_amount` and `receipt_datetime`.

### 2. Field name is `litres` (British spelling), not `liters`

Easy bug source. Hardcode the British spelling in Rust; assert-by-name in tests.

### 3. `receipt_datetime` is full ISO-8601 datetime *without* timezone or fractional seconds

Confirmed format (real values): `"2026-04-27T13:24:14"`, `"2026-04-14T15:31:00"`,
`"2026-03-27T14:41:00"`. Parse as `chrono::NaiveDateTime` with format
`"%Y-%m-%dT%H:%M:%S"`. Per-doc `null` value remains possible → fallback to `created`
(which is `YYYY-MM-DD`) for year filtering and `?` for display.

### 4. Verified facts (locked in for the plan)

| Item | Verified value |
|---|---|
| Auth header | `Authorization: Token <PAT>` — `Bearer` returns 401 |
| Test endpoint | `GET /api/ui_settings/` → 200 with `~4.7 KB` JSON for valid PAT |
| Tag `fuel` ID | 51 (12 docs) |
| Tag `car` ID | 59 (8 docs) |
| Tag query | `?tags__id__in=51,59` (server-side OR; URL-encoded comma `%2C`) |
| Custom field `total_amount` | id 1, `float`, on every invoice |
| Custom field `litres` | id 5, `float`, on fuel invoices only |
| Custom field `receipt_datetime` | id 6, `string`, on every invoice (ISO-8601 no TZ) |
| Doc HTML URL | `{base}/documents/{id}/` (302 → login page when unauth; the user's browser is logged in) |
| Pagination | `next` URL until null; default page_size 100 fits 20-doc current dataset trivially |

### 5. Reference fixtures (use verbatim in unit tests)

```json
// Fuel — doc 435, tag 51, custom_fields complete
{
  "id": 435, "title": "OMV Slovensko, s.r.o. - Scanned_20260427-1325",
  "tags": [54, 55, 51, 48], "created": "2026-04-27",
  "custom_fields": [
    {"value": 113.95, "field": 1},
    {"value": 63.34,  "field": 5},
    {"value": "2026-04-27T13:24:14", "field": 6}
  ]
}

// Other / parking — doc 423, tag 59, no litres
{
  "id": 423, "title": "Hlavné mesto SR Bratislava - 1776180674432",
  "tags": [54, 55, 59, 48], "created": "2026-04-14",
  "custom_fields": [
    {"value": 1.95, "field": 1},
    {"value": "1776180674432", "field": 4},
    {"value": "2026-04-14T15:31:00", "field": 6}
  ]
}

// Other / car wash — doc 391, tag 59
{
  "id": 391, "title": "Mataso s.r.o. - 0003",
  "tags": [44, 55, 59, 48], "created": "2026-03-27",
  "custom_fields": [
    {"value": 110.0, "field": 1},
    {"value": "0003", "field": 4},
    {"value": "2026-03-27T14:41:00", "field": 6}
  ]
}
```

Tags `48`, `54`, `55`, `44`, `34`, `32` are the user's organizational tags
(business/year/owner) — irrelevant to this feature; we only care about 51 + 59.

---

## Updated `PaperlessInvoiceRow` shape

```rust
// src-tauri/core/src/models.rs (new struct)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperlessInvoiceRow {
    pub paperless_document_id: i64,
    pub title: String,
    pub paperless_url: String,                      // {base}/documents/{id}/
    pub total_price_eur: Option<f64>,               // from total_amount
    pub liters: Option<f64>,                        // from litres (None for car-only)
    pub receipt_datetime: Option<NaiveDateTime>,    // from receipt_datetime field
    pub created_date: NaiveDate,                    // doc.created — fallback for year
    pub assignment_type: AssignmentType,            // Fuel | Other (priority: Fuel)
    pub trip_id: Option<String>,                    // from paperless_trip_links join
}
```

Lives in [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs).

---

## Verification environment

- **Working dir:** `C:\_dev\kniha-jazd` (project root,
  see [CLAUDE.md](../../CLAUDE.md))
- **Rust workspace:** `cd src-tauri` before any `cargo` command
  (Cargo manifest at [src-tauri/Cargo.toml](../../src-tauri/Cargo.toml))
- **PAT for live testing** (already in [.env](../../.env), which is
  [.gitignore](../../.gitignore)d):
  `PAPERLESS_API_TOKEN=REDACTED`.
  Use only for manual verification; **all automated tests must use a mock HTTP
  server** so CI doesn't depend on the live Paperless instance. Do NOT commit
  [.env](../../.env).

## Pre-flight — dependency & convention notes

Verified against [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml) on
2026-05-03:

**Already present in `[dependencies]`:** `reqwest` (with `json`, `rustls-tls`),
`tokio` (with `macros`, `rt-multi-thread`, `sync`), `url`, `thiserror`,
`chrono` (with `serde`), `diesel` (sqlite), `diesel_migrations`, `serde`,
`serde_json`, `log`. No need to add any of these.

**Must be added before tasks that use them:**

| Crate | Where used | Section | Add as |
|---|---|---|---|
| `wiremock = "0.6"` | Tasks 3, 7, 8 (HTTP mocking) | `[dev-dependencies]` | new line under existing `tempfile = "3"` |
| `urlencoding = "2"` | Task 7 (`resolve_tag_id`) | `[dependencies]` | new line, alphabetical with `url = "2"` |

`tokio` is already a regular dependency with `macros` enabled, so
`#[tokio::test]` macro works in tests without re-declaring it under
`[dev-dependencies]`. If `cargo test` ever errors with "cannot find attribute
`tokio::test` in this scope", re-add `tokio = { version = "1", features =
["macros", "rt-multi-thread"] }` to `[dev-dependencies]` as a fallback.

**Database test convention (do not reinvent):** The project's idiomatic test setup
is [Database::in_memory()](../../src-tauri/core/src/db.rs):68 (which runs embedded
migrations) followed by typed methods on `Database` (e.g.
[db.create_trip(&trip)](../../src-tauri/core/src/db.rs):268). There is **no**
`db::tests::test_connection`, **no** `seed_trip`, **no** `CountRow` helper —
those are inventions. Mirror the existing pattern in
[db_tests.rs](../../src-tauri/core/src/db_tests.rs).

**Read-only mode API:** Use
[app_state.enable_read_only("reason")](../../src-tauri/core/src/app_state.rs):98,
**not** `set_read_only(true, ...)` (which does not exist). See
[app_state.rs:97-101](../../src-tauri/core/src/app_state.rs).

**Diesel schema regeneration:** [diesel.toml](../../src-tauri/core/diesel.toml)
sets `print_schema.file = "src/schema.rs"`. The project uses
[embed_migrations!()](../../src-tauri/core/src/db.rs):23 — tests pick up new
migrations automatically. To update [schema.rs](../../src-tauri/core/src/schema.rs)
itself, either edit it manually (mirroring an existing small block like
[vehicles](../../src-tauri/core/src/schema.rs)), or run
`diesel migration run --database-url <temp.db> && diesel print-schema > src/schema.rs`.
Manual editing is the faster path for a one-table addition; see
[Task 5](#task-5-migration-add_paperless_trip_links--diesel-schema).

---

## Task list (TDD, bite-sized, frequent commits)

> Each task ends with a commit. Do not bundle multiple tasks into one commit.

- [Task 0: Branch + commit planning docs](#task-0-branch--commit-planning-docs)
- [Task 1: Add Paperless fields to `LocalSettings`](#task-1-add-paperless-fields-to-localsettings)
- [Task 2: `save_paperless_settings` + `get_paperless_settings` commands](#task-2-save_paperless_settings--get_paperless_settings-commands)
- [Task 3: `test_paperless_connection` command](#task-3-test_paperless_connection-command)
- [Task 4: `get_invoice_source_mode` command](#task-4-get_invoice_source_mode-command)
- [Task 5: Migration `add_paperless_trip_links` + Diesel schema](#task-5-migration-add_paperless_trip_links--diesel-schema)
- [Task 6: `PaperlessTripLink` model + db CRUD](#task-6-paperlesstriplink-model--db-crud)
- [Task 7: `paperless` module — tag + custom-field ID resolution](#task-7-paperless-module--tag--custom-field-id-resolution)
- [Task 8: `paperless` module — fetch + parse documents](#task-8-paperless-module--fetch--parse-documents)
- [Task 9: `get_paperless_invoices` command](#task-9-get_paperless_invoices-command)
- [Task 10: `assign_paperless_doc_to_trip` + `unassign_paperless_doc` commands](#task-10-assign_paperless_doc_to_trip--unassign_paperless_doc-commands)
- [Task 11: Register all new Tauri commands in lib.rs](#task-11-register-all-new-tauri-commands-in-librs)
- [Task 12: i18n keys (Slovak + English)](#task-12-i18n-keys-slovak--english)
- [Task 13: Settings page — Paperless section](#task-13-settings-page--paperless-section)
- [Task 14: Doklady page — mode switch + Paperless renderer](#task-14-doklady-page--mode-switch--paperless-renderer)
- [Task 15: Tier-2 integration test with mock Paperless server](#task-15-tier-2-integration-test-with-mock-paperless-server)
- [Task 16: CHANGELOG, DECISIONS.md, feature doc](#task-16-changelog-decisionsmd-feature-doc)
- [Task 17: /verify and ship](#task-17-verify-and-ship)

---

### Task 0: Branch + commit planning docs

**Why first:** [_tasks/CLAUDE.md](../CLAUDE.md) mandates committing planning docs
**before** implementation, so design rationale lands in version control before
code.

**Files:**
- Modify: [_tasks/index.md](../index.md) (add row for task 60 if not already present)

**Steps:**

1. Create a feature branch:
   ```powershell
   git checkout -b feat/paperless-integration
   ```

2. Verify [_tasks/index.md](../index.md) lists task 60 in **Active Tasks**
   with status 📋 / 🟡. If not, add a row:
   ```markdown
   | 60 | [Paperless-ngx integration](./60-paperless-integration/) | 🟡 In Progress |
   ```

3. Stage planning artifacts only:
   ```powershell
   git add _tasks/60-paperless-integration/03-plan.md _tasks/index.md
   git status   # confirm nothing else is staged
   git commit -m "docs(tasks): add implementation plan for paperless integration"
   ```

---

### Task 1: Add Paperless fields to `LocalSettings`

**Files:**
- Modify: [src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs):27-43
  (struct fields)
- Modify: [src-tauri/core/src/settings_tests.rs](../../src-tauri/core/src/settings_tests.rs)
  (round-trip test)

**Step 1: Write the failing test**

Append to [src-tauri/core/src/settings_tests.rs](../../src-tauri/core/src/settings_tests.rs):

```rust
#[test]
fn local_settings_round_trips_paperless_fields() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = LocalSettings::default();
    s.paperless_url = Some("https://documents.lacny.me".to_string());
    s.paperless_api_token = Some("test-token-abc".to_string());
    s.save(dir.path()).unwrap();

    let loaded = LocalSettings::load(dir.path());
    assert_eq!(loaded.paperless_url.as_deref(), Some("https://documents.lacny.me"));
    assert_eq!(loaded.paperless_api_token.as_deref(), Some("test-token-abc"));
}
```

**Step 2: Run, verify it fails**

```powershell
cd src-tauri
cargo test -p kniha_jazd_core local_settings_round_trips_paperless_fields
```
Expected: compile error — `paperless_url` / `paperless_api_token` are not fields of
`LocalSettings`.

**Step 3: Implement**

Add two fields to the `LocalSettings` struct in
[src-tauri/core/src/settings.rs](../../src-tauri/core/src/settings.rs), grouped after
the existing HA block:

```rust
// Paperless-ngx integration
pub paperless_url: Option<String>,        // e.g. "https://documents.lacny.me"
pub paperless_api_token: Option<String>,  // PAT, plaintext (matches HA convention)
```

**Step 4: Run, verify it passes**

```powershell
cargo test -p kniha_jazd_core local_settings_round_trips_paperless_fields
```
Expected: 1 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/settings.rs src-tauri/core/src/settings_tests.rs
git commit -m "feat(settings): add paperless_url and paperless_api_token to LocalSettings"
```

---

### Task 2: `save_paperless_settings` + `get_paperless_settings` commands

Mirror the HA pair (`save_ha_settings_internal` / `get_ha_settings_internal`)
at [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs):46-172.

**Files:**
- Modify: [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs)
- Create: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)
  *(if absent — otherwise extend existing one)*
- Modify: [src-tauri/core/src/commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs)
  *(if you needed to add the test file)*

**Step 1: Write failing tests**

```rust
// integrations_tests.rs — add to existing file or create with the standard
// #[path = "..."] mod tests pattern in integrations.rs.
#[test]
fn save_paperless_settings_persists_url_and_token() {
    let dir = tempfile::tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    super::save_paperless_settings_internal(
        dir.path(), &app_state,
        Some("https://documents.lacny.me".into()),
        Some("tok-1".into()),
    ).unwrap();

    let loaded = crate::settings::LocalSettings::load(dir.path());
    assert_eq!(loaded.paperless_url.as_deref(), Some("https://documents.lacny.me"));
    assert_eq!(loaded.paperless_api_token.as_deref(), Some("tok-1"));
}

#[test]
fn save_paperless_settings_rejects_invalid_url() {
    let dir = tempfile::tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    let err = super::save_paperless_settings_internal(
        dir.path(), &app_state,
        Some("not-a-url".into()),
        Some("tok".into()),
    ).unwrap_err();
    assert!(err.contains("URL must start with http"));
}

#[test]
fn save_paperless_settings_blocked_by_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let app_state = crate::app_state::AppState::new();
    app_state.enable_read_only("test");   // see app_state.rs:98
    let err = super::save_paperless_settings_internal(
        dir.path(), &app_state,
        Some("https://x.example".into()), Some("t".into()),
    ).unwrap_err();
    assert!(err.to_lowercase().contains("read"));
}

#[test]
fn get_paperless_settings_hides_token() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x.example".into());
    s.paperless_api_token = Some("super-secret".into());
    s.save(dir.path()).unwrap();

    let r = super::get_paperless_settings_internal(dir.path()).unwrap();
    assert_eq!(r.url.as_deref(), Some("https://x.example"));
    assert!(r.has_token);
    // token itself is NOT in the response struct — compile-time guarantee
}
```

**Step 2: Run, verify failures**

```powershell
cargo test -p kniha_jazd_core integrations_tests
```
Expected: compile errors for the missing `save_paperless_settings_internal`,
`get_paperless_settings_internal`, and the response struct.

**Step 3: Implement**

In [integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs),
copy the HA pattern and adapt:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperlessSettingsResponse {
    pub url: Option<String>,
    pub has_token: bool,
}

pub fn get_paperless_settings_internal(app_dir: &Path) -> Result<PaperlessSettingsResponse, String> {
    let settings = LocalSettings::load(app_dir);
    Ok(PaperlessSettingsResponse {
        url: settings.paperless_url,
        has_token: settings.paperless_api_token.is_some(),
    })
}

pub fn save_paperless_settings_internal(
    app_dir: &Path, app_state: &AppState,
    url: Option<String>, token: Option<String>,
) -> Result<(), String> {
    check_read_only!(app_state);
    if let Some(ref url_str) = url {
        if !url_str.is_empty() {
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
            if url::Url::parse(url_str).is_err() {
                return Err("Invalid URL format".to_string());
            }
        }
    }
    let mut settings = LocalSettings::load(app_dir);
    if let Some(u) = url { settings.paperless_url = if u.is_empty() { None } else { Some(u) }; }
    if let Some(t) = token { settings.paperless_api_token = if t.is_empty() { None } else { Some(t) }; }
    settings.save(app_dir).map_err(|e| e.to_string())
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core integrations_tests::save_paperless_settings_persists_url_and_token integrations_tests::save_paperless_settings_rejects_invalid_url integrations_tests::save_paperless_settings_blocked_by_read_only integrations_tests::get_paperless_settings_hides_token
```
Expected: 4 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/commands_internal/integrations.rs src-tauri/core/src/commands_internal/integrations_tests.rs
git commit -m "feat(integrations): save/get paperless settings (mirrors HA pattern)"
```

---

### Task 3: `test_paperless_connection` command

The one place we **diverge** from the HA implementation: header is
`Authorization: Token <PAT>`, **not** `Bearer`. Lock this with an explicit unit
test.

**Files:**
- Modify: [src-tauri/core/src/Cargo.toml](../../src-tauri/core/Cargo.toml)
  (add `wiremock` to `[dev-dependencies]`)
- Modify: [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs)
- Modify: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)

**Step 0: Add `wiremock` to dev-dependencies** *(first task that needs it; no
existing tests use it — verified by repo-wide grep)*

In [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml), under the
existing `[dev-dependencies]` block (currently just `tempfile = "3"`):

```toml
[dev-dependencies]
tempfile = "3"
wiremock = "0.6"
```

Then verify the dep resolves:

```powershell
cd src-tauri
cargo build -p kniha_jazd_core --tests
```
Expected: clean build (downloads `wiremock` and its tree on first run).

**Step 1: Write failing tests**

```rust
// integrations_tests.rs — add at bottom
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_paperless_connection_uses_token_auth_header_not_bearer() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/ui_settings/"))
        .and(header("authorization", "Token my-pat-123")) // EXACT format
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&mock).await;

    let dir = tempfile::tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(mock.uri());
    s.paperless_api_token = Some("my-pat-123".into());
    s.save(dir.path()).unwrap();

    let ok = super::test_paperless_connection_internal(dir.path()).await.unwrap();
    assert!(ok, "Token-format auth header should yield connected=true");
}

#[tokio::test]
async fn test_paperless_connection_rejects_bearer_header() {
    // If a future maintainer accidentally uses Bearer, the mock won't match
    // and the response is 404 (default). Connection should report false.
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/ui_settings/"))
        .and(header("authorization", "Bearer my-pat-123"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock).await;

    let dir = tempfile::tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(mock.uri());
    s.paperless_api_token = Some("my-pat-123".into());
    s.save(dir.path()).unwrap();

    let ok = super::test_paperless_connection_internal(dir.path()).await.unwrap();
    assert!(!ok, "Bearer header must NOT match — Paperless DRF requires Token");
}

#[tokio::test]
async fn test_paperless_connection_unconfigured_returns_false_silently() {
    let dir = tempfile::tempdir().unwrap();
    let ok = super::test_paperless_connection_internal(dir.path()).await.unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn test_paperless_connection_401_returns_false() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/ui_settings/"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock).await;

    let dir = tempfile::tempdir().unwrap();
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(mock.uri());
    s.paperless_api_token = Some("bad".into());
    s.save(dir.path()).unwrap();

    assert!(!super::test_paperless_connection_internal(dir.path()).await.unwrap());
}
```

**Step 2: Run, verify failures**

```powershell
cargo test -p kniha_jazd_core test_paperless_connection
```
Expected: compile error — `test_paperless_connection_internal` undefined.

**Step 3: Implement**

```rust
// integrations.rs — add below the HA block
pub async fn test_paperless_connection_internal(app_dir: &Path) -> Result<bool, String> {
    let settings = LocalSettings::load(app_dir);
    let (url, token) = match (settings.paperless_url, settings.paperless_api_token) {
        (Some(u), Some(t)) if !u.is_empty() && !t.is_empty() => (u, t),
        _ => return Ok(false),
    };
    let api_url = format!("{}/api/ui_settings/", url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build().map_err(|e| e.to_string())?;

    // Paperless DRF token auth — NOT Bearer. See task 60 / 03-plan.md.
    let response = client.get(&api_url)
        .header("Authorization", format!("Token {}", token))
        .header("Accept", "application/json")
        .send().await
        .map_err(|e| e.to_string())?;

    Ok(response.status().is_success())
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core test_paperless_connection
```
Expected: 4 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/commands_internal/integrations.rs src-tauri/core/src/commands_internal/integrations_tests.rs src-tauri/core/Cargo.toml
git commit -m "feat(integrations): test_paperless_connection with DRF Token auth"
```

---

### Task 4: `get_invoice_source_mode` command

Single source of truth for "are we in Paperless mode?". Frontend calls this; never
inspects raw settings (per [ADR-008 in DECISIONS.md](../../DECISIONS.md)).

**Files:**
- Modify: [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs)
- Modify: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)

**Step 1: Write failing tests**

```rust
#[test]
fn invoice_source_mode_is_paperless_when_both_fields_populated() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    s.paperless_api_token = Some("t".into());
    assert_eq!(super::get_invoice_source_mode_from_settings(&s),
               super::InvoiceSourceMode::Paperless);
}

#[test]
fn invoice_source_mode_is_local_when_url_missing() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_api_token = Some("t".into());
    assert_eq!(super::get_invoice_source_mode_from_settings(&s),
               super::InvoiceSourceMode::Local);
}

#[test]
fn invoice_source_mode_is_local_when_token_missing() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some("https://x".into());
    assert_eq!(super::get_invoice_source_mode_from_settings(&s),
               super::InvoiceSourceMode::Local);
}

#[test]
fn invoice_source_mode_is_local_when_url_is_empty_string() {
    let mut s = crate::settings::LocalSettings::default();
    s.paperless_url = Some(String::new());
    s.paperless_api_token = Some("t".into());
    assert_eq!(super::get_invoice_source_mode_from_settings(&s),
               super::InvoiceSourceMode::Local);
}
```

**Step 2: Run, verify failure**

```powershell
cargo test -p kniha_jazd_core invoice_source_mode
```
Expected: compile error — `InvoiceSourceMode` undefined.

**Step 3: Implement**

```rust
// integrations.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InvoiceSourceMode { Local, Paperless }

pub fn get_invoice_source_mode_from_settings(s: &LocalSettings) -> InvoiceSourceMode {
    match (&s.paperless_url, &s.paperless_api_token) {
        (Some(u), Some(t)) if !u.is_empty() && !t.is_empty() => InvoiceSourceMode::Paperless,
        _ => InvoiceSourceMode::Local,
    }
}

pub fn get_invoice_source_mode_internal(app_dir: &Path) -> Result<InvoiceSourceMode, String> {
    Ok(get_invoice_source_mode_from_settings(&LocalSettings::load(app_dir)))
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core invoice_source_mode
```
Expected: 4 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/commands_internal/integrations.rs src-tauri/core/src/commands_internal/integrations_tests.rs
git commit -m "feat(integrations): get_invoice_source_mode (single source of truth)"
```

---

### Task 5: Migration `add_paperless_trip_links` + Diesel schema

**Files:**
- Create: [src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql)
- Create: [src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/down.sql](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/down.sql)
- Modify: [src-tauri/core/src/schema.rs](../../src-tauri/core/src/schema.rs)
  *(re-generated by `diesel migration run`)*

Reference patterns: see existing migrations in
[src-tauri/core/migrations/](../../src-tauri/core/migrations/),
e.g. [2026-01-08-095218-0000_baseline](../../src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql)
for the receipts FK pattern, or
[2026-02-12-100000_add_vehicle_ha_fuel_level_sensor](../../src-tauri/core/migrations/2026-02-12-100000_add_vehicle_ha_fuel_level_sensor/up.sql)
for a small schema change.

**Step 1: Write the SQL**

[up.sql](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql):
```sql
CREATE TABLE paperless_trip_links (
    trip_id TEXT PRIMARY KEY,
    paperless_document_id INTEGER NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (trip_id) REFERENCES trips(id)
);
CREATE INDEX idx_paperless_links_doc ON paperless_trip_links(paperless_document_id);
```

[down.sql](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/down.sql):
```sql
DROP INDEX IF EXISTS idx_paperless_links_doc;
DROP TABLE IF EXISTS paperless_trip_links;
```

**Step 2: Manually update [schema.rs](../../src-tauri/core/src/schema.rs)**

The project uses
[embed_migrations!()](../../src-tauri/core/src/db.rs):23 — the migration runs
automatically inside
[Database::in_memory()](../../src-tauri/core/src/db.rs):68 and
[Database::new()](../../src-tauri/core/src/db.rs):53. But Diesel's compile-time
type checking still needs the table declared in
[schema.rs](../../src-tauri/core/src/schema.rs). Append this block (mirroring the
shape of an existing small block like the `vehicles` or `receipts` table):

```rust
diesel::table! {
    paperless_trip_links (trip_id) {
        trip_id -> Text,
        paperless_document_id -> BigInt,
        created_at -> Text,
        updated_at -> Text,
    }
}
```

If the file has a `joinable!` or `allow_tables_to_appear_in_same_query!` block at
the bottom, also add `paperless_trip_links` there next to `trips`:

```rust
diesel::joinable!(paperless_trip_links -> trips (trip_id));

diesel::allow_tables_to_appear_in_same_query!(
    // ... existing tables ...
    paperless_trip_links,
);
```

**Optional alternative (Diesel CLI):** if you'd rather have Diesel write the
schema for you, point it at a throwaway DB:

```powershell
cd src-tauri/core
diesel migration run --database-url $env:TEMP\paperless-schema-gen.db
diesel print-schema --database-url $env:TEMP\paperless-schema-gen.db > src/schema.rs
Remove-Item $env:TEMP\paperless-schema-gen.db
```
[diesel.toml](../../src-tauri/core/diesel.toml) has
`print_schema.file = "src/schema.rs"` configured, so the second command knows
where to write.

**Step 3: Verify [schema.rs](../../src-tauri/core/src/schema.rs) change**

```powershell
git diff src-tauri/core/src/schema.rs
```
Expected: a new `paperless_trip_links` block with the four columns listed.

**Step 4: Build (also exercises the embedded migration via `Database::in_memory`)**

```powershell
cd src-tauri
cargo build -p kniha_jazd_core
cargo test -p kniha_jazd_core test_database_creation
```
Expected: clean compile + 1 passed (the existing test verifies migrations run
from `:memory:`; if the new migration's SQL is invalid, this test fails first).

**Step 5: Commit**

```powershell
git add src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/ src-tauri/core/src/schema.rs
git commit -m "feat(db): add paperless_trip_links table (1:1 trip↔doc link)"
```

---

### Task 6: `PaperlessTripLink` model + db CRUD

**Files:**
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)
  (new struct + Insertable variant)
- Modify: [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) (CRUD functions)
- Modify: [src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs)

**Step 1: Write failing tests**

Append to [db_tests.rs](../../src-tauri/core/src/db_tests.rs) using the project's
existing pattern (see
[test_database_creation](../../src-tauri/core/src/db_tests.rs):8 and
[test_vehicle_crud_lifecycle](../../src-tauri/core/src/db_tests.rs):20 — methods
on `Database`, no free functions, no `tests` submodule):

```rust
use crate::models::Trip;
use chrono::NaiveDateTime;

// Tiny seed helper local to this test module — mirrors create_test_vehicle pattern
// already in db_tests.rs:15. Trip has a wide field set; only the fields needed
// for FK satisfaction are tweaked.
fn seed_test_trip(db: &Database, vehicle_id: &str) -> String {
    use uuid::Uuid;
    let trip = Trip {
        id: Uuid::new_v4(),
        vehicle_id: Uuid::parse_str(vehicle_id).unwrap(),
        origin: "BA".into(), destination: "TT".into(),
        distance_km: 50.0, odometer: 12345.0,
        purpose: "test".into(),
        fuel_liters: None, fuel_cost_eur: None,
        other_costs_eur: None, other_costs_note: None,
        full_tank: false, sort_order: 0,
        energy_kwh: None, energy_cost_eur: None,
        full_charge: false, soc_override_percent: None,
        start_datetime: NaiveDateTime::parse_from_str(
            "2026-01-01T08:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
        end_datetime: None,
        created_at: chrono::Utc::now(), updated_at: chrono::Utc::now(),
    };
    let id = trip.id.to_string();
    db.create_trip(&trip).expect("seed trip");
    id
}

#[test]
fn paperless_link_upsert_creates_then_replaces() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let v_id = v.id.to_string();
    let trip_a = seed_test_trip(&db, &v_id);
    let trip_b = seed_test_trip(&db, &v_id);

    db.upsert_paperless_link(&trip_a, 435).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), Some(trip_a.clone()));

    // Reassigning the same doc to a different trip moves the link.
    db.upsert_paperless_link(&trip_b, 435).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), Some(trip_b.clone()));
    assert_eq!(db.get_paperless_link_for_trip(&trip_a).unwrap(), None);
}

#[test]
fn paperless_link_delete_is_idempotent() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = seed_test_trip(&db, &v.id.to_string());

    db.upsert_paperless_link(&trip, 435).unwrap();
    db.delete_paperless_link_for_doc(435).unwrap();
    db.delete_paperless_link_for_doc(435).unwrap(); // no-op, no error
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), None);
}

#[test]
fn paperless_link_unique_doc_invariant() {
    let db = Database::in_memory().expect("db");
    let v = create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip_a = seed_test_trip(&db, &v.id.to_string());
    let trip_b = seed_test_trip(&db, &v.id.to_string());

    db.upsert_paperless_link(&trip_a, 435).unwrap();
    // Same doc on a second trip: UPSERT replaces, doesn't duplicate.
    db.upsert_paperless_link(&trip_b, 435).unwrap();
    assert_eq!(db.count_paperless_links().unwrap(), 1);
}
```

**Step 2: Run, verify failure**

```powershell
cd src-tauri
cargo test -p kniha_jazd_core paperless_link
```
Expected: compile errors — model + `Database` methods missing.

**Step 3: Implement**

In [models.rs](../../src-tauri/core/src/models.rs):

```rust
use diesel::prelude::*;
use crate::schema::paperless_trip_links;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Insertable, AsChangeset)]
#[diesel(table_name = paperless_trip_links)]
#[diesel(primary_key(trip_id))]
pub struct PaperlessTripLink {
    pub trip_id: String,
    pub paperless_document_id: i64,
    pub created_at: String,
    pub updated_at: String,
}
```

In [db.rs](../../src-tauri/core/src/db.rs) — add methods on `Database`
(matching the convention of
[db.create_trip(&trip)](../../src-tauri/core/src/db.rs):268,
[db.get_trip(&id)](../../src-tauri/core/src/db.rs):311). UPSERT semantics:
PRIMARY KEY conflict on `trip_id` REPLACES; UNIQUE conflict on
`paperless_document_id` requires explicit delete-then-insert because SQLite UPSERT
only handles one conflict target.

```rust
// In impl Database { ... }, near the existing receipt CRUD methods:

pub fn upsert_paperless_link(&self, trip_id: &str, doc_id: i64) -> QueryResult<()> {
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    let now = chrono::Utc::now().to_rfc3339();

    conn.transaction::<_, diesel::result::Error, _>(|tx| {
        // Clear any prior link to THIS doc (might be on a different trip).
        diesel::delete(p::paperless_trip_links.filter(p::paperless_document_id.eq(doc_id)))
            .execute(tx)?;
        // Clear any prior link FROM this trip (trip might have linked a different doc).
        diesel::delete(p::paperless_trip_links.filter(p::trip_id.eq(trip_id)))
            .execute(tx)?;
        diesel::insert_into(p::paperless_trip_links)
            .values((
                p::trip_id.eq(trip_id),
                p::paperless_document_id.eq(doc_id),
                p::created_at.eq(&now),
                p::updated_at.eq(&now),
            ))
            .execute(tx)?;
        Ok(())
    })
}

pub fn delete_paperless_link_for_doc(&self, doc_id: i64) -> QueryResult<()> {
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    diesel::delete(p::paperless_trip_links.filter(p::paperless_document_id.eq(doc_id)))
        .execute(conn).map(|_| ())
}

pub fn get_paperless_link_for_doc(&self, doc_id: i64) -> QueryResult<Option<String>> {
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    p::paperless_trip_links
        .filter(p::paperless_document_id.eq(doc_id))
        .select(p::trip_id)
        .first::<String>(conn).optional()
}

pub fn get_paperless_link_for_trip(&self, trip_id: &str) -> QueryResult<Option<i64>> {
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    p::paperless_trip_links
        .filter(p::trip_id.eq(trip_id))
        .select(p::paperless_document_id)
        .first::<i64>(conn).optional()
}

pub fn list_paperless_links_for_docs(&self, doc_ids: &[i64])
    -> QueryResult<Vec<(i64, String)>>
{
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    p::paperless_trip_links
        .filter(p::paperless_document_id.eq_any(doc_ids))
        .select((p::paperless_document_id, p::trip_id))
        .load::<(i64, String)>(conn)
}

#[cfg(test)]
pub fn count_paperless_links(&self) -> QueryResult<i64> {
    use crate::schema::paperless_trip_links::dsl as p;
    let conn = &mut *self.conn.lock().unwrap();
    p::paperless_trip_links.count().get_result(conn)
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core paperless_link
```
Expected: 3 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/models.rs src-tauri/core/src/db.rs src-tauri/core/src/db_tests.rs
git commit -m "feat(db): PaperlessTripLink model + UPSERT CRUD"
```

---

### Task 7: `paperless` module — tag + custom-field ID resolution

Two name → ID lookups, both cached for the session in `AppState`. If a name is
missing in the user's Paperless, return a structured error the UI can show.

**Files:**
- Modify: [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml)
  (add `urlencoding = "2"` to `[dependencies]`)
- Create: [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)
- Create: [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs)
- Modify: [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs)
  (add `pub mod paperless;`)

**Step 0a: Add `urlencoding` to `[dependencies]`**

In [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml), under
`[dependencies]`, alongside `url = "2"`:

```toml
urlencoding = "2"
```

**Step 0b: Create empty module + register in [lib.rs](../../src-tauri/core/src/lib.rs)**

This must happen BEFORE Step 1 so `cargo test` can resolve `super::*` in the
test file. Create
[src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs) with
just a doc comment:

```rust
//! Paperless-ngx HTTP client + parsing.
```

Then add to [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs)
(grouped with other `pub mod` declarations near the top):

```rust
pub mod paperless;
```

Verify the module is wired up:

```powershell
cd src-tauri
cargo build -p kniha_jazd_core
```
Expected: clean build (the module is empty so nothing to compile-check yet).

**Step 1: Write failing tests**

```rust
// paperless_tests.rs
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn resolve_tag_id_returns_existing_tag() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/tags/")).and(query_param("name__iexact", "fuel"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1, "results": [{"id": 51, "name": "fuel"}]
        })))
        .mount(&mock).await;

    let client = super::PaperlessClient::new(mock.uri(), "tok".into());
    let id = client.resolve_tag_id("fuel").await.unwrap();
    assert_eq!(id, 51);
}

#[tokio::test]
async fn resolve_tag_id_errors_when_tag_missing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/tags/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 0, "results": []
        })))
        .mount(&mock).await;

    let client = super::PaperlessClient::new(mock.uri(), "tok".into());
    let err = client.resolve_tag_id("nonexistent").await.unwrap_err();
    assert!(matches!(err, super::PaperlessError::TagNotFound(ref n) if n == "nonexistent"));
}

#[tokio::test]
async fn resolve_field_map_finds_all_three_required_fields() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {"id": 1, "name": "total_amount", "data_type": "float"},
                {"id": 5, "name": "litres", "data_type": "float"},
                {"id": 6, "name": "receipt_datetime", "data_type": "string"},
                {"id": 4, "name": "order_id", "data_type": "string"},
            ]
        })))
        .mount(&mock).await;

    let client = super::PaperlessClient::new(mock.uri(), "tok".into());
    let map = client.resolve_field_map().await.unwrap();
    assert_eq!(map.total_amount_id, 1);
    assert_eq!(map.litres_id, 5);
    assert_eq!(map.receipt_datetime_id, 6);
}

#[tokio::test]
async fn resolve_field_map_errors_when_required_field_missing() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/custom_fields/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"id": 1, "name": "total_amount", "data_type": "float"}]
        })))
        .mount(&mock).await;

    let client = super::PaperlessClient::new(mock.uri(), "tok".into());
    let err = client.resolve_field_map().await.unwrap_err();
    match err {
        super::PaperlessError::CustomFieldNotFound(ref n) => {
            assert!(n == "litres" || n == "receipt_datetime");
        }
        _ => panic!("expected CustomFieldNotFound, got {:?}", err),
    }
}
```

**Step 2: Run, verify failures**

```powershell
cargo test -p kniha_jazd_core paperless_tests
```
Expected: compile errors — module / types absent.

**Step 3: Implement**

```rust
// paperless.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum PaperlessError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Paperless returned status {0}")]
    Http(u16),
    #[error("Tag '{0}' not found in Paperless")]
    TagNotFound(String),
    #[error("Custom field '{0}' not found in Paperless")]
    CustomFieldNotFound(String),
    #[error("Paperless URL not configured")]
    NotConfigured,
    #[error("Failed to parse Paperless response: {0}")]
    Parse(String),
}

impl From<reqwest::Error> for PaperlessError {
    fn from(e: reqwest::Error) -> Self { PaperlessError::Network(e.to_string()) }
}

#[derive(Debug, Clone, Copy)]
pub struct PaperlessFieldMap {
    pub total_amount_id: i64,
    pub litres_id: i64,
    pub receipt_datetime_id: i64,
}

pub struct PaperlessClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl PaperlessClient {
    pub fn new(base_url: String, token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build().expect("reqwest client");
        Self { base_url: base_url.trim_end_matches('/').to_string(), token, http }
    }

    fn auth(&self) -> String { format!("Token {}", self.token) }

    pub async fn resolve_tag_id(&self, name: &str) -> Result<i64, PaperlessError> {
        #[derive(Deserialize)] struct Tag { id: i64 }
        #[derive(Deserialize)] struct Resp { results: Vec<Tag> }

        let url = format!("{}/api/tags/?name__iexact={}", self.base_url, urlencoding::encode(name));
        let resp = self.http.get(&url).header("Authorization", self.auth()).send().await?;
        if !resp.status().is_success() { return Err(PaperlessError::Http(resp.status().as_u16())); }
        let body: Resp = resp.json().await.map_err(|e| PaperlessError::Parse(e.to_string()))?;
        body.results.first().map(|t| t.id).ok_or_else(|| PaperlessError::TagNotFound(name.to_string()))
    }

    pub async fn resolve_field_map(&self) -> Result<PaperlessFieldMap, PaperlessError> {
        #[derive(Deserialize)] struct Field { id: i64, name: String }
        #[derive(Deserialize)] struct Resp { results: Vec<Field> }

        let url = format!("{}/api/custom_fields/", self.base_url);
        let resp = self.http.get(&url).header("Authorization", self.auth()).send().await?;
        if !resp.status().is_success() { return Err(PaperlessError::Http(resp.status().as_u16())); }
        let body: Resp = resp.json().await.map_err(|e| PaperlessError::Parse(e.to_string()))?;

        let find = |n: &str| body.results.iter().find(|f| f.name == n).map(|f| f.id)
            .ok_or_else(|| PaperlessError::CustomFieldNotFound(n.to_string()));

        Ok(PaperlessFieldMap {
            total_amount_id: find("total_amount")?,
            litres_id: find("litres")?,                  // British spelling — see plan §1
            receipt_datetime_id: find("receipt_datetime")?,
        })
    }
}

#[cfg(test)]
#[path = "paperless_tests.rs"]
mod tests;
```

(`urlencoding` was added in Step 0a; no further dependency edits needed here.)

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core paperless_tests
```
Expected: 4 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/paperless.rs src-tauri/core/src/paperless_tests.rs src-tauri/core/src/lib.rs src-tauri/core/Cargo.toml
git commit -m "feat(paperless): tag + custom-field ID resolution with structured errors"
```

---

### Task 8: `paperless` module — fetch + parse documents

**Files:**
- Modify: [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)
- Modify: [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs)
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)
  (add `PaperlessInvoiceRow` if not added)

**Step 1: Write failing tests** (use the verified fixtures from §5 of this plan)

```rust
// paperless_tests.rs — add at bottom
#[tokio::test]
async fn fetch_documents_parses_real_fuel_doc_with_litres() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/documents/"))
        .and(query_param("tags__id__in", "51,59"))
        .and(query_param("page_size", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1, "next": null,
            "results": [{
                "id": 435, "title": "OMV Slovensko, s.r.o. - Scanned_20260427-1325",
                "tags": [54, 55, 51, 48], "created": "2026-04-27",
                "custom_fields": [
                    {"value": 113.95, "field": 1},
                    {"value": 63.34, "field": 5},
                    {"value": "2026-04-27T13:24:14", "field": 6}
                ]
            }]
        })))
        .mount(&mock).await;

    let client = super::PaperlessClient::new(mock.uri(), "tok".into());
    let map = super::PaperlessFieldMap { total_amount_id: 1, litres_id: 5, receipt_datetime_id: 6 };
    let docs = client.fetch_invoice_documents(51, 59, &map).await.unwrap();

    assert_eq!(docs.len(), 1);
    let d = &docs[0];
    assert_eq!(d.id, 435);
    assert_eq!(d.title, "OMV Slovensko, s.r.o. - Scanned_20260427-1325");
    assert_eq!(d.tag_ids, vec![54, 55, 51, 48]);
    assert_eq!(d.created, chrono::NaiveDate::from_ymd_opt(2026, 4, 27).unwrap());
    assert_eq!(d.total_amount, Some(113.95));
    assert_eq!(d.litres, Some(63.34));
    assert_eq!(d.receipt_datetime,
               chrono::NaiveDateTime::parse_from_str("2026-04-27T13:24:14", "%Y-%m-%dT%H:%M:%S").ok());
}

#[tokio::test]
async fn fetch_documents_parses_car_doc_with_no_litres() {
    let mock = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/documents/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 1, "next": null,
            "results": [{
                "id": 423, "title": "Hlavné mesto SR Bratislava - 1776180674432",
                "tags": [54, 55, 59, 48], "created": "2026-04-14",
                "custom_fields": [
                    {"value": 1.95, "field": 1},
                    {"value": "1776180674432", "field": 4},
                    {"value": "2026-04-14T15:31:00", "field": 6}
                ]
            }]
        })))
        .mount(&mock).await;

    let client = super::PaperlessClient::new(mock.uri(), "tok".into());
    let map = super::PaperlessFieldMap { total_amount_id: 1, litres_id: 5, receipt_datetime_id: 6 };
    let docs = client.fetch_invoice_documents(51, 59, &map).await.unwrap();
    assert_eq!(docs[0].litres, None, "car-tagged docs don't carry litres");
    assert_eq!(docs[0].total_amount, Some(1.95));
}

#[tokio::test]
async fn fetch_documents_follows_pagination_next_link() {
    let mock = MockServer::start().await;
    let mock_uri = mock.uri();

    // Page 1 → has next URL pointing to ?page=2
    Mock::given(method("GET")).and(path("/api/documents/"))
        .and(query_param("page_size", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 2,
            "next": format!("{}/api/documents/?page=2&page_size=100&tags__id__in=51%2C59", mock_uri),
            "results": [{
                "id": 1, "title": "p1", "tags": [51], "created": "2026-01-01",
                "custom_fields": [{"value": 10.0, "field": 1}, {"value": "2026-01-01T00:00:00", "field": 6}]
            }]
        })))
        .mount(&mock).await;

    Mock::given(method("GET")).and(path("/api/documents/")).and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "count": 2, "next": null,
            "results": [{
                "id": 2, "title": "p2", "tags": [59], "created": "2026-01-02",
                "custom_fields": [{"value": 20.0, "field": 1}, {"value": "2026-01-02T00:00:00", "field": 6}]
            }]
        })))
        .mount(&mock).await;

    let client = super::PaperlessClient::new(mock_uri, "tok".into());
    let map = super::PaperlessFieldMap { total_amount_id: 1, litres_id: 5, receipt_datetime_id: 6 };
    let docs = client.fetch_invoice_documents(51, 59, &map).await.unwrap();
    assert_eq!(docs.iter().map(|d| d.id).collect::<Vec<_>>(), vec![1, 2]);
}
```

**Step 2: Run, verify failures**

```powershell
cargo test -p kniha_jazd_core paperless_tests::fetch_documents
```
Expected: compile errors — `fetch_invoice_documents` undefined.

**Step 3: Implement**

```rust
// paperless.rs — add below
#[derive(Debug, Clone)]
pub struct PaperlessDoc {
    pub id: i64,
    pub title: String,
    pub tag_ids: Vec<i64>,
    pub created: chrono::NaiveDate,
    pub total_amount: Option<f64>,
    pub litres: Option<f64>,
    pub receipt_datetime: Option<chrono::NaiveDateTime>,
}

impl PaperlessClient {
    pub async fn fetch_invoice_documents(
        &self, fuel_id: i64, car_id: i64, fields: &PaperlessFieldMap,
    ) -> Result<Vec<PaperlessDoc>, PaperlessError> {
        #[derive(Deserialize)] struct CustomField { field: i64, value: serde_json::Value }
        #[derive(Deserialize)] struct Raw {
            id: i64, title: String, tags: Vec<i64>, created: String,
            #[serde(default)] custom_fields: Vec<CustomField>,
        }
        #[derive(Deserialize)] struct Page { next: Option<String>, results: Vec<Raw> }

        let mut url = format!(
            "{}/api/documents/?tags__id__in={},{}&page_size=100",
            self.base_url, fuel_id, car_id
        );

        let mut out = Vec::new();
        loop {
            let resp = self.http.get(&url).header("Authorization", self.auth()).send().await?;
            if !resp.status().is_success() { return Err(PaperlessError::Http(resp.status().as_u16())); }
            let page: Page = resp.json().await.map_err(|e| PaperlessError::Parse(e.to_string()))?;

            for r in page.results {
                let created = chrono::NaiveDate::parse_from_str(&r.created, "%Y-%m-%d")
                    .map_err(|e| PaperlessError::Parse(format!("created '{}': {}", r.created, e)))?;

                let mut total = None;
                let mut litres = None;
                let mut dt = None;
                for cf in r.custom_fields {
                    if cf.field == fields.total_amount_id {
                        total = cf.value.as_f64();
                    } else if cf.field == fields.litres_id {
                        litres = cf.value.as_f64();
                    } else if cf.field == fields.receipt_datetime_id {
                        if let Some(s) = cf.value.as_str() {
                            dt = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok();
                        }
                    }
                }

                out.push(PaperlessDoc {
                    id: r.id, title: r.title, tag_ids: r.tags, created,
                    total_amount: total, litres, receipt_datetime: dt,
                });
            }

            match page.next { Some(n) => url = n, None => break }
        }
        Ok(out)
    }
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core paperless_tests::fetch_documents
```
Expected: 3 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/paperless.rs src-tauri/core/src/paperless_tests.rs
git commit -m "feat(paperless): fetch + parse documents with litres support"
```

---

### Task 9: `get_paperless_invoices` command

Composes the client, applies year filter, joins with `paperless_trip_links`, returns
the flat row shape the doklady page renders.

**Files:**
- Create: [src-tauri/core/src/commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs)
- Modify: [src-tauri/core/src/commands_internal/mod.rs](../../src-tauri/core/src/commands_internal/mod.rs)
  (add `pub mod paperless_cmd;`)
- Modify: [src-tauri/core/src/models.rs](../../src-tauri/core/src/models.rs)
  (add `PaperlessInvoiceRow` if not yet)
- Modify: [src-tauri/core/src/commands_internal/commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs)
  (or new [paperless_cmd_tests.rs](../../src-tauri/core/src/commands_internal/paperless_cmd_tests.rs))

**Step 1: Write failing tests**

```rust
// paperless_cmd_tests.rs (new) — covers tag→assignment and year filter
#[test]
fn get_paperless_invoices_maps_fuel_only_to_fuel() {
    let row = super::test_helpers::make_doc(&[51]);
    let assigned = super::map_assignment(&row.tag_ids, 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Fuel);
}

#[test]
fn get_paperless_invoices_maps_car_only_to_other() {
    let row = super::test_helpers::make_doc(&[59]);
    let assigned = super::map_assignment(&row.tag_ids, 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Other);
}

#[test]
fn get_paperless_invoices_both_tags_priority_fuel() {
    let assigned = super::map_assignment(&[51, 59], 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Fuel);
}

#[test]
fn map_assignment_logs_warning_and_returns_other_when_neither_tag_present() {
    // Server-side filter should make this unreachable, but if it ever fails
    // (Paperless returning a stale tag set, race during custom-field migration),
    // the function must NOT panic — it logs a warning and returns Other so the
    // doklady page still renders the row instead of failing the whole sync.
    let assigned = super::map_assignment(&[1234], 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Other);
    // (We don't assert on the log output — that would require a log capture
    // harness; the test's role is to lock the don't-panic contract.)
}

#[test]
fn year_filter_uses_receipt_datetime_when_present() {
    let dt = chrono::NaiveDateTime::parse_from_str("2026-04-27T13:24:14", "%Y-%m-%dT%H:%M:%S").unwrap();
    let created = chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
    assert_eq!(super::doc_year(&Some(dt), &created), 2026);
}

#[test]
fn year_filter_falls_back_to_created_when_no_datetime() {
    let created = chrono::NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
    assert_eq!(super::doc_year(&None, &created), 2025);
}
```

**Step 2: Run, verify failures**

```powershell
cargo test -p kniha_jazd_core paperless_cmd
```

**Step 3: Implement**

```rust
// paperless_cmd.rs
use crate::app_state::AppState;
use crate::check_read_only;
use crate::db::Database;
use crate::models::{AssignmentType, PaperlessInvoiceRow};
use crate::paperless::{PaperlessClient, PaperlessError, PaperlessFieldMap, PaperlessDoc};
use crate::settings::LocalSettings;
use std::path::Path;

pub fn map_assignment(tag_ids: &[i64], fuel_id: i64, car_id: i64) -> AssignmentType {
    if tag_ids.contains(&fuel_id) {
        AssignmentType::Fuel
    } else if tag_ids.contains(&car_id) {
        AssignmentType::Other
    } else {
        // Unreachable in practice — Paperless server-side filter
        // (?tags__id__in=...) only returns docs that match. If we ever land
        // here it indicates a bug in the filter or a Paperless behavior change;
        // log loudly and degrade gracefully to Other so the doklady page still
        // renders the row instead of failing the whole sync.
        log::warn!(
            "paperless: doc has neither fuel ({}) nor car ({}) tag — got {:?}; \
             check server-side filter",
            fuel_id, car_id, tag_ids
        );
        AssignmentType::Other
    }
}

pub fn doc_year(dt: &Option<chrono::NaiveDateTime>, created: &chrono::NaiveDate) -> i32 {
    use chrono::Datelike;
    dt.as_ref().map(|d| d.year()).unwrap_or(created.year())
}

/// NOTE: `vehicle_id` is currently NOT used to filter results — Paperless docs
/// are not vehicle-scoped in this iteration. Single-vehicle users (the current
/// userbase) see the same set of invoices regardless. Multi-vehicle filtering is
/// deferred; see DECISIONS.md "BIZ — Paperless v1 is single-vehicle scoped" entry
/// (added in Task 16). The parameter is kept on the signature so the frontend
/// contract is forward-compatible — when filtering lands, no UI plumbing changes.
pub async fn get_paperless_invoices_internal(
    app_dir: &Path,
    db: &Database,
    vehicle_id: &str,
    year: i32,
) -> Result<Vec<PaperlessInvoiceRow>, PaperlessError> {
    let _ = vehicle_id;   // intentionally unused — see doc-comment above

    let settings = LocalSettings::load(app_dir);
    let (url, token) = match (settings.paperless_url, settings.paperless_api_token) {
        (Some(u), Some(t)) if !u.is_empty() && !t.is_empty() => (u, t),
        _ => return Err(PaperlessError::NotConfigured),
    };
    let base = url.trim_end_matches('/').to_string();

    let client = PaperlessClient::new(base.clone(), token);
    let fuel_id = client.resolve_tag_id("fuel").await?;
    let car_id  = client.resolve_tag_id("car").await?;
    let fmap    = client.resolve_field_map().await?;

    let docs: Vec<PaperlessDoc> = client.fetch_invoice_documents(fuel_id, car_id, &fmap).await?;
    let docs: Vec<PaperlessDoc> = docs.into_iter()
        .filter(|d| doc_year(&d.receipt_datetime, &d.created) == year)
        .collect();

    let doc_ids: Vec<i64> = docs.iter().map(|d| d.id).collect();
    let links = db.list_paperless_links_for_docs(&doc_ids)
        .map_err(|e| PaperlessError::Parse(e.to_string()))?;
    let link_map: std::collections::HashMap<i64, String> = links.into_iter().collect();

    Ok(docs.into_iter().map(|d| PaperlessInvoiceRow {
        paperless_url: format!("{}/documents/{}/", base, d.id),
        trip_id: link_map.get(&d.id).cloned(),
        assignment_type: map_assignment(&d.tag_ids, fuel_id, car_id),
        paperless_document_id: d.id, title: d.title,
        total_price_eur: d.total_amount, liters: d.litres,
        receipt_datetime: d.receipt_datetime, created_date: d.created,
    }).collect())
}

#[cfg(test)]
#[path = "paperless_cmd_tests.rs"]
mod tests;

#[cfg(test)]
pub(crate) mod test_helpers {
    pub fn make_doc(tag_ids: &[i64]) -> crate::paperless::PaperlessDoc {
        crate::paperless::PaperlessDoc {
            id: 0, title: "t".into(), tag_ids: tag_ids.to_vec(),
            created: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            total_amount: None, litres: None, receipt_datetime: None,
        }
    }
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core paperless_cmd
```
Expected: 6 passed.

**Step 5: Commit**

```powershell
git add src-tauri/core/src/commands_internal/ src-tauri/core/src/models.rs
git commit -m "feat(paperless): get_paperless_invoices command (mode-aware row builder)"
```

---

### Task 10: `assign_paperless_doc_to_trip` + `unassign_paperless_doc` commands

**Files:**
- Modify: [src-tauri/core/src/commands_internal/paperless_cmd.rs](../../src-tauri/core/src/commands_internal/paperless_cmd.rs)
- Modify: [src-tauri/core/src/commands_internal/paperless_cmd_tests.rs](../../src-tauri/core/src/commands_internal/paperless_cmd_tests.rs)

**Step 1: Write failing tests**

These mirror the trip-seeding helper from Task 6 (which lives in
[db_tests.rs](../../src-tauri/core/src/db_tests.rs) — re-export it as
`pub(crate) fn seed_test_trip` so this file can use it, OR copy the helper
verbatim if cross-module re-export is awkward).

```rust
// paperless_cmd_tests.rs — append below the map_assignment / doc_year tests

use crate::app_state::AppState;
use crate::db::Database;

#[test]
fn assign_paperless_doc_blocked_when_read_only() {
    let db = Database::in_memory().unwrap();
    let v = crate::db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = crate::db_tests::seed_test_trip(&db, &v.id.to_string());

    let app_state = AppState::new();
    app_state.enable_read_only("test");   // see app_state.rs:98

    let err = super::assign_paperless_doc_to_trip_internal(&app_state, &db, 435, &trip)
        .unwrap_err();
    assert!(err.to_lowercase().contains("read"));
}

#[test]
fn assign_paperless_doc_persists_link() {
    let db = Database::in_memory().unwrap();
    let v = crate::db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = crate::db_tests::seed_test_trip(&db, &v.id.to_string());

    let app_state = AppState::new();
    super::assign_paperless_doc_to_trip_internal(&app_state, &db, 435, &trip).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), Some(trip));
}

#[test]
fn unassign_paperless_doc_removes_link() {
    let db = Database::in_memory().unwrap();
    let v = crate::db_tests::create_test_vehicle("Test"); db.create_vehicle(&v).unwrap();
    let trip = crate::db_tests::seed_test_trip(&db, &v.id.to_string());

    let app_state = AppState::new();
    super::assign_paperless_doc_to_trip_internal(&app_state, &db, 435, &trip).unwrap();
    super::unassign_paperless_doc_internal(&app_state, &db, 435).unwrap();
    assert_eq!(db.get_paperless_link_for_doc(435).unwrap(), None);
}
```

Sub-prerequisite: in [db_tests.rs](../../src-tauri/core/src/db_tests.rs),
change the helper's visibility:

```rust
pub(crate) fn create_test_vehicle(name: &str) -> Vehicle { ... }
pub(crate) fn seed_test_trip(db: &Database, vehicle_id: &str) -> String { ... }
```

…and re-export the test module as a path-attribute mod in
[lib.rs](../../src-tauri/core/src/lib.rs) so `crate::db_tests::*` resolves under
`#[cfg(test)]`. (If the project already uses `#[path = "db_tests.rs"] mod db_tests;`
inside `db.rs`, just bump it to `pub(crate) mod tests;` — verify the existing
include pattern and follow it.)

**Step 2: Run, verify failures**

```powershell
cargo test -p kniha_jazd_core paperless_cmd_tests::assign paperless_cmd_tests::unassign
```

**Step 3: Implement**

```rust
// paperless_cmd.rs — append
pub fn assign_paperless_doc_to_trip_internal(
    app_state: &AppState, db: &Database,
    doc_id: i64, trip_id: &str,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.upsert_paperless_link(trip_id, doc_id).map_err(|e| e.to_string())
}

pub fn unassign_paperless_doc_internal(
    app_state: &AppState, db: &Database, doc_id: i64,
) -> Result<(), String> {
    check_read_only!(app_state);
    db.delete_paperless_link_for_doc(doc_id).map_err(|e| e.to_string())
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core paperless_cmd_tests
```
Expected: 9 passed (6 from task 9 + 3 new).

**Step 5: Commit**

```powershell
git add src-tauri/core/src/commands_internal/paperless_cmd.rs src-tauri/core/src/commands_internal/paperless_cmd_tests.rs
git commit -m "feat(paperless): assign/unassign trip↔doc commands (read-only-gated)"
```

---

### Task 11: Register all new Tauri commands in lib.rs

The codebase has **two parallel command paths** that both must be wired up:

1. **Desktop** — Tauri's `invoke_handler!` macro in
   [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs):290.
2. **Server (web mode)** — manual string-match dispatchers in
   [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs)
   (sync) and
   [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)
   (async). Every HA command appears in both files (3 sync + 2 async). Skipping
   this file means the Paperless feature silently breaks the `/web` build (Task
   33 in [_tasks/_done/](../_done/) — webapp gets "Unknown command" errors).

**Files:**
- Modify: [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs)
  (around line 290 — HA commands list)
- Modify: [src-tauri/desktop/src/commands.rs](../../src-tauri/desktop/src/commands.rs)
  (or wherever HA commands are wrapped — `test_ha_connection` is referenced at
  [lib.rs:293](../../src-tauri/desktop/src/lib.rs))
- Modify: [src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs)
  (sync — add 3 entries near
  [the existing HA "Integrations — sync only (3)" block at line 777](../../src-tauri/core/src/server/dispatcher.rs))
- Modify: [src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)
  (async — add 4 entries near
  [the existing HA "Integrations — async (2)" block at line 64](../../src-tauri/core/src/server/dispatcher_async.rs))

**Step 1: Desktop wrappers**

In
[src-tauri/desktop/src/commands.rs](../../src-tauri/desktop/src/commands.rs)
add Tauri command wrappers next to the HA wrappers (paste-adapt
`get_ha_settings`, `save_ha_settings`, `test_ha_connection` patterns):

```rust
#[tauri::command]
pub async fn get_paperless_settings(...) -> Result<PaperlessSettingsResponse, String> { ... }

#[tauri::command]
pub async fn save_paperless_settings(app_state: State<'_, AppState>, app_handle: AppHandle,
    url: Option<String>, token: Option<String>) -> Result<(), String> { ... }

#[tauri::command]
pub async fn test_paperless_connection(app_handle: AppHandle) -> Result<bool, String> { ... }

#[tauri::command]
pub async fn get_invoice_source_mode(app_handle: AppHandle) -> Result<InvoiceSourceMode, String> { ... }

#[tauri::command]
pub async fn get_paperless_invoices(app_handle: AppHandle, db: State<'_, Db>,
    vehicle_id: String, year: i32) -> Result<Vec<PaperlessInvoiceRow>, String> { ... }

#[tauri::command]
pub async fn assign_paperless_doc_to_trip(app_state: State<'_, AppState>, db: State<'_, Db>,
    doc_id: i64, trip_id: String) -> Result<(), String> { ... }

#[tauri::command]
pub async fn unassign_paperless_doc(app_state: State<'_, AppState>, db: State<'_, Db>,
    doc_id: i64) -> Result<(), String> { ... }
```

**Step 2: Add all 7 to `invoke_handler!`** in
[src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs):290:

```rust
commands::get_paperless_settings,
commands::save_paperless_settings,
commands::test_paperless_connection,
commands::get_invoice_source_mode,
commands::get_paperless_invoices,
commands::assign_paperless_doc_to_trip,
commands::unassign_paperless_doc,
```

**Step 3: Server-mode sync dispatcher (3 entries)**

In
[src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs),
under the existing `// Integrations — sync only (3)` block (around line 777),
append (paste-adapt `get_ha_settings` and `save_ha_settings` at lines 780-803):

```rust
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
    }
    let a: Args = parse_args(args)?;
    crate::commands_internal::integrations::save_paperless_settings_internal(
        &state.app_dir,
        &state.app_state,
        a.url,
        a.token,
    )?;
    Ok(serde_json::to_value(()).unwrap())
}
"get_invoice_source_mode" => {
    let v = crate::commands_internal::integrations::get_invoice_source_mode_internal(&state.app_dir)?;
    Ok(serde_json::to_value(v).unwrap())
}
```

Update the comment block header to say `// Integrations — sync only (6)` so the
HA + Paperless count is visible.

**Step 4: Server-mode async dispatcher (4 entries)**

In
[src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs),
under the existing `// Integrations — async (2)` block (around line 64), append
(paste-adapt `test_ha_connection` and `fetch_ha_odo` at lines 67-85):

```rust
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
    Some(result.map(|v| serde_json::to_value(v).unwrap())
                .map_err(|e| format!("{:?}", e)))
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
```

Update the comment block header to say `// Integrations — async (6)`.

> **Verification trick:** if `state.db` doesn't typecheck (the dispatcher's
> `state` struct may not yet expose `db: Arc<Database>` on the async path), look
> at how the existing `export_html` async block at
> [dispatcher_async.rs:90](../../src-tauri/core/src/server/dispatcher_async.rs)
> reaches the database — copy that pattern verbatim.

**Step 5: Verify build (both targets)**

```powershell
cd src-tauri
cargo build                     # desktop
cargo build -p kniha_jazd_core  # server lib only
```
Expected: clean builds.

**Step 6:** No dedicated unit test (wrappers are mechanical). Tier-2 test in
[Task 15](#task-15-tier-2-integration-test-with-mock-paperless-server) exercises
the desktop path end-to-end; the server-mode dispatchers are covered by the same
HA-style trust ("if the HA wrappers work via this path, ours do too").

**Step 7: Commit**

```powershell
git add src-tauri/desktop/src/ src-tauri/core/src/server/
git commit -m "feat(tauri): register paperless commands (desktop + server dispatchers)"
```

---

### Task 12: i18n keys (Slovak + English)

**Files:**
- Modify: [src/lib/i18n/sk/index.ts](../../src/lib/i18n/sk/index.ts)
- Modify: [src/lib/i18n/en/index.ts](../../src/lib/i18n/en/index.ts)

**Step 1:** Add the following key tree to both Slovak and English files (Slovak
first; mirror in English):

```ts
// sk/index.ts — under root
paperless: {
  // Settings
  sectionTitle: 'Paperless-ngx',
  description: 'Načítaj doklady priamo z Paperless-ngx (filtruje podľa tagov fuel/car).',
  url: 'URL',
  urlPlaceholder: 'https://documents.priklad.sk',
  apiToken: 'API token (PAT)',
  apiTokenPlaceholder: '40-znakový token z Paperless',
  testConnection: 'Test pripojenia',
  status: { idle: 'Pripravené', testing: 'Testujem…', connected: 'Pripojené', disconnected: 'Nepripojené' },
  errors: {
    urlInvalid: 'Neplatná URL',
    invalidToken: 'Neplatný token',
    tagMissing: 'Pridaj tag „{name}" v Paperless a označ ním invoice',
    fieldMissing: 'Vytvor custom field „{name}" v Paperless',
    network: 'Paperless nedostupný — skontroluj nastavenia',
  },
},
doklady: {
  paperless: {
    refresh: 'Obnoviť z Paperless',
    openInPaperless: 'Otvoriť v Paperless',
    noDate: '?',     // shown when receipt_datetime is missing
  },
},
```

```ts
// en/index.ts
paperless: {
  sectionTitle: 'Paperless-ngx',
  description: 'Read invoices directly from Paperless-ngx (filtered by fuel/car tags).',
  url: 'URL', urlPlaceholder: 'https://documents.example.com',
  apiToken: 'API token (PAT)', apiTokenPlaceholder: '40-character token from Paperless',
  testConnection: 'Test connection',
  status: { idle: 'Idle', testing: 'Testing…', connected: 'Connected', disconnected: 'Disconnected' },
  errors: {
    urlInvalid: 'Invalid URL', invalidToken: 'Invalid token',
    tagMissing: 'Add tag "{name}" in Paperless and tag your invoices with it',
    fieldMissing: 'Create custom field "{name}" in Paperless',
    network: 'Paperless unreachable — check your settings',
  },
},
doklady: { paperless: {
  refresh: 'Refresh from Paperless', openInPaperless: 'Open in Paperless', noDate: '?',
} },
```

**Step 2: Verify**

```powershell
npm run build
```
Expected: clean SvelteKit build (no missing-key TS errors from `typesafe-i18n`).

**Step 3: Commit**

```powershell
git add src/lib/i18n/
git commit -m "i18n: add paperless integration keys (sk + en)"
```

---

### Task 13: Settings page — Paperless section

**Files:**
- Modify: [src/routes/settings/+page.svelte](../../src/routes/settings/+page.svelte)
  (add a new section after the HA one; the HA section starts around the
  `testHaConnectionStatus` function at line 170 and template-side around line 460)
- Modify: [src/lib/api.ts](../../src/lib/api.ts) (add wrappers
  `testPaperlessConnection`, `getPaperlessSettings`, `savePaperlessSettings`,
  `getInvoiceSourceMode`)
- Modify: [src/lib/types.ts](../../src/lib/types.ts) (add `PaperlessSettings`,
  `InvoiceSourceMode`, `PaperlessInvoiceRow`. **Do NOT redefine
  `AssignmentType` — it already exists at
  [types.ts:189](../../src/lib/types.ts) as `'Fuel' | 'Other'`, which matches
  the Rust default-tagged serde output. Just import it where
  `PaperlessInvoiceRow` references it.**)

**Step 1: Lift the HA section as a template** — copy `testHaConnectionStatus`,
`saveHaSettingsNow`, `validateHaUrl`, the debounced wrapper, and the markup block,
then s/ha/paperless/g and adapt:

- `testHaConnection()` → `testPaperlessConnection()`
- `saveHaSettings(...)` → `savePaperlessSettings(...)`
- `Bearer` references in any frontend HA logging — Paperless docs always say `Token`.
  No frontend HTTP calls go straight to Paperless (all routed through Rust), so this
  is naming-only.

**Step 2:** UI mirrors HA exactly: input for URL, password input for PAT, "Test
connection" button with status indicator badge using `$LL.paperless.status.*`.
Status state machine: `IDLE → TESTING → CONNECTED | DISCONNECTED`.

**Step 3: Manual verification**

```powershell
npm run tauri dev
```
- Open Settings → scroll to Paperless section.
- Paste URL [https://documents.lacny.me](https://documents.lacny.me), paste the PAT
  from [.env](../../.env), blur.
- Auto-save fires after 800ms; status shows `TESTING` then `CONNECTED`.
- Clear the URL → status returns to `IDLE`.
- Paste an invalid PAT → `DISCONNECTED` with hover-tooltip "Neplatný token".

**Step 4:** No new unit tests (Tier-2 test in
[Task 15](#task-15-tier-2-integration-test-with-mock-paperless-server) exercises
the flow).

**Step 5: Commit**

```powershell
git add src/routes/settings/+page.svelte src/lib/api.ts src/lib/types.ts
git commit -m "feat(ui): paperless section in Settings page"
```

---

### Task 14: Doklady page — mode switch + Paperless renderer

**Files:**
- Modify: [src/routes/doklady/+page.svelte](../../src/routes/doklady/+page.svelte)
  (the existing 1472-line page)
- Modify: [src/lib/api.ts](../../src/lib/api.ts) (add `getPaperlessInvoices`,
  `assignPaperlessDocToTrip`, `unassignPaperlessDoc`)

**Step 1: Add the mode switch at page load**

```svelte
<script lang="ts">
  import { getInvoiceSourceMode, getPaperlessInvoices, assignPaperlessDocToTrip,
           unassignPaperlessDoc } from '$lib/api';
  import type { InvoiceSourceMode, PaperlessInvoiceRow } from '$lib/types';

  let mode: InvoiceSourceMode = 'local';
  let paperlessRows: PaperlessInvoiceRow[] = [];

  async function loadInvoices() {
    mode = await getInvoiceSourceMode();
    if (mode === 'paperless') {
      paperlessRows = await getPaperlessInvoices(currentVehicleId, currentYear);
    } else {
      // existing local-receipts loader, unchanged
    }
  }
</script>
```

**Step 2: Two render branches**

- **`mode === 'local'`:** existing markup, unchanged.
- **`mode === 'paperless'`:** new renderer that maps `paperlessRows[]` to a grid
  with columns: title, datetime (or `?`), assignment type chip (Fuel/Other),
  amount, liters (only Fuel rows), assigned-trip indicator, action buttons.

**Buttons in Paperless mode:**
| Button | Handler | Notes |
|---|---|---|
| Open in Paperless | `window.open(row.paperlessUrl, '_blank')` | Tauri opener plugin if needed for non-browser env |
| Assign | Opens existing `TripSelectorModal` → calls `assignPaperlessDocToTrip(doc_id, trip_id)` then `await loadInvoices()` | UPSERT semantics |
| Unassign | `unassignPaperlessDoc(doc_id)` then `await loadInvoices()` | Only shown when row.tripId is non-null |
| Edit, Reprocess, Remove | **Hidden** | No-op against Paperless |

**Step 3: "Refresh from Paperless" button** in the toolbar → calls
`loadInvoices()` again (forces a fresh fetch — backend doesn't cache documents).

**Step 4: Manual verification**

```powershell
npm run tauri dev
```
- Configure Paperless in Settings → return to Doklady.
- Verify ~20 rows render, fuel rows show liters, parking rows don't.
- Click "Open in Paperless" → opens the doc in your browser (you're logged in).
- Click "Assign" on doc 435 → pick any trip → row shows trip indicator.
- Click "Refresh from Paperless" → reload still shows the assignment.
- Disable Paperless in Settings (clear URL) → Doklady reverts to local-receipts grid.

**Step 5: Commit**

```powershell
git add src/routes/doklady/+page.svelte src/lib/api.ts src/lib/types.ts
git commit -m "feat(ui): doklady page paperless mode (Open + Assign + Refresh)"
```

---

### Task 15: Tier-2 integration test with mock Paperless server

**Files:**
- Create: [tests/integration/specs/_helpers/mock-paperless-server.ts](../../tests/integration/specs/_helpers/mock-paperless-server.ts)
  *(mirrors the existing HA mock helper; small Express/HTTP server returning the
   verified fixtures from §5)*
- Create: [tests/integration/specs/tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts)

**Step 1:** Write the mock helper. It listens on a free localhost port, exposes:
- `GET /api/ui_settings/` → 200 (with `Authorization: Token` check; 401 otherwise)
- `GET /api/tags/?name__iexact=fuel` → returns id 51
- `GET /api/tags/?name__iexact=car` → returns id 59
- `GET /api/custom_fields/` → returns the 3 required fields with stable IDs
- `GET /api/documents/?tags__id__in=51,59&page_size=100` → returns the 3 fixtures
  from §5 verbatim, with `next: null`

**Step 2: Write the spec** (matches the existing
[settings.spec.ts](../../tests/integration/specs/tier2/settings.spec.ts):1-60
shape — Mocha globals, no `vitest` imports; helpers from `utils/`)

```ts
/**
 * Tier 2: Paperless Integration
 *
 * Full flow: Settings → test connection → Doklady renders rows from mock
 * Paperless → Assign persists across refresh → toggling Paperless off restores
 * local view.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { invokeTauri } from '../../utils/db';
import { startMockPaperless, stopMockPaperless } from '../_helpers/mock-paperless-server';

describe('Tier 2: Paperless Integration', () => {
  let mockUrl: string;

  before(async () => {
    mockUrl = await startMockPaperless();
  });

  after(async () => {
    await stopMockPaperless();
  });

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  it('Settings → test → Doklady renders rows → Assign persists', async () => {
    // 1. Open Settings, enter URL + PAT, click Test
    await navigateTo('settings');
    await browser.pause(300);

    const urlInput = await $('input[data-test="paperless-url"]');
    await urlInput.setValue(mockUrl);

    const tokenInput = await $('input[data-test="paperless-token"]');
    await tokenInput.setValue('test-pat-123');

    await $('button[data-test="paperless-test-connection"]').click();
    await $('[data-test="paperless-status"]').waitForDisplayed({ timeout: 5000 });
    await expect($('[data-test="paperless-status"]')).toHaveText('Connected');

    // 2. Doklady → assert 3 rows render with the verified titles
    await navigateTo('doklady');
    await browser.pause(500);   // allow paperless fetch to settle

    const rows = await $$('[data-test="paperless-row"]');
    expect(rows.length).toBe(3);
    await expect($('[data-test="paperless-row"][data-doc-id="435"] [data-test=title]'))
      .toHaveText('OMV Slovensko, s.r.o. - Scanned_20260427-1325');
    await expect($('[data-test="paperless-row"][data-doc-id="435"] [data-test=liters]'))
      .toHaveText('63.34');
    // Parking doc has no liters cell value
    await expect($('[data-test="paperless-row"][data-doc-id="423"] [data-test=liters]'))
      .toHaveText('—');

    // 3. Assign doc 435 to a trip
    await $('[data-test="paperless-row"][data-doc-id="435"] [data-test="assign-btn"]').click();
    await $('[data-test="trip-option"]').waitForDisplayed({ timeout: 2000 });
    await ($$('[data-test="trip-option"]'))[0].click();
    await expect($('[data-test="paperless-row"][data-doc-id="435"] [data-test="trip-indicator"]'))
      .toBeDisplayed();

    // 4. Refresh — assignment survives the round-trip
    await $('button[data-test="paperless-refresh"]').click();
    await browser.pause(300);
    await expect($('[data-test="paperless-row"][data-doc-id="435"] [data-test="trip-indicator"]'))
      .toBeDisplayed();

    // 5. Disable Paperless via Tauri IPC (faster than UI), then verify
    //    Doklady reverts to local view (no paperless rows visible).
    await invokeTauri('save_paperless_settings', { url: '', token: '' });
    await navigateTo('trips');     // navigate away to force /doklady remount
    await navigateTo('doklady');
    await browser.pause(300);
    await expect($('[data-test="paperless-row"]')).not.toBeDisplayed();
  });
});
```

**Important:** The Tauri process is the HTTP client that hits `mockUrl`, NOT
the WDIO process. They share `localhost`, so a Node-launched mock on a free
port works — but the mock MUST bind to `127.0.0.1` (or `0.0.0.0`) and not e.g.
a Unix socket. Future maintainers should not move the mock into the Tauri
process.

**Step 3: Run**

```powershell
npm run test:integration:build   # rebuild debug binary
npx wdio run tests/integration/wdio.conf.ts --spec tests/integration/specs/tier2/paperless-integration.spec.ts
```
Expected: 1 passing.

**Step 4: Commit**

```powershell
git add tests/integration/specs/tier2/paperless-integration.spec.ts tests/integration/specs/_helpers/mock-paperless-server.ts
git commit -m "test(integration): tier-2 paperless integration end-to-end"
```

---

### Task 16: CHANGELOG, DECISIONS.md, feature doc

**Files:**
- Modify: [CHANGELOG.md](../../CHANGELOG.md) (under `## [Unreleased]` → `### Added`)
- Modify: [DECISIONS.md](../../DECISIONS.md) (two new entries)
- Create: [docs/features/paperless-integration.md](../../docs/features/paperless-integration.md)
  (template at [docs/CLAUDE.md](../../docs/CLAUDE.md))

**Step 1: CHANGELOG entry**

```markdown
## [Unreleased]

### Added
- **Paperless-ngx integration.** Optional invoice source: when Settings → Paperless
  is configured (URL + PAT), the Doklady page reads invoices directly from a
  Paperless-ngx instance instead of the local OCR scan, filtered by the `fuel` and
  `car` tags. Trip assignment works in both modes; existing local receipts and
  their assignments are preserved when Paperless is toggled off.
```

**Step 2: DECISIONS.md entries**

```markdown
### ADR — Paperless trip-link table is symmetric (`trip_id PRIMARY KEY`)
**Date:** 2026-05-03
**Context:** `paperless_trip_links` mirrors the receipt↔trip 1:1 relationship.
The existing `receipts` table uses `id PRIMARY KEY, trip_id UNIQUE` because
receipts carry their own metadata. Paperless docs live remotely; the link row
holds nothing but the IDs.
**Decision:** Use `trip_id TEXT PRIMARY KEY` and `paperless_document_id INTEGER UNIQUE`.
A separate surrogate `id` would add no information.
**Consequences:** UPSERT requires deleting both potential prior links (by trip_id
*and* by paperless_document_id) before inserting — encapsulated in
`db::upsert_paperless_link`.

### BIZ — Paperless DRF auth header is `Token`, not `Bearer`
**Date:** 2026-05-03
**Context:** Home Assistant integration uses `Authorization: Bearer <token>`.
Paperless-ngx uses Django REST Framework token authentication, which expects
`Authorization: Token <token>`.
**Decision:** Hardcode `Token` for Paperless requests; cover with explicit
regression test (`integrations_tests::test_paperless_connection_uses_token_auth_header_not_bearer`).
**Consequences:** Every new Paperless HTTP call must use the `Token` prefix.
Future Paperless-related issues should grep for `Authorization` first.

### BIZ — Paperless v1 is single-vehicle scoped (vehicle_id intentionally unused)
**Date:** 2026-05-03
**Context:** `get_paperless_invoices_internal(app_dir, db, vehicle_id, year)`
takes a `vehicle_id` parameter but does not filter Paperless results by it.
Paperless documents have no native vehicle dimension; the user's tagging scheme
uses only `fuel` / `car` for the kniha-jazd integration. Today the user has a
single primary vehicle, so this is invisible.
**Decision:** Keep `vehicle_id` on the signature for forward compatibility but
intentionally ignore it in v1. Document the deferral in code via
`let _ = vehicle_id;` and a doc-comment so it doesn't read as a bug.
**Consequences:** Multi-vehicle users see the same invoice list on every
vehicle's doklady page. Future work to scope by vehicle (e.g., a
`vehicle:{slug}` Paperless tag) is gated behind explicit user demand — current
single-vehicle user has no reason to bear that complexity yet.
```

**Step 3: Feature doc**

Use the template in [docs/CLAUDE.md](../../docs/CLAUDE.md). Cover:
- User flow (Settings → Doklady mode switch)
- Tag/field mapping table
- Schema additions
- The auth-header divergence (link to BIZ entry above)
- Out-of-scope (no fuzzy matching, no PAT keyring, no bulk assign)

**Step 4: Commit**

```powershell
git add CHANGELOG.md DECISIONS.md docs/features/paperless-integration.md
git commit -m "docs: paperless integration changelog, ADRs, feature doc"
```

---

### Task 17: /verify and ship

**Step 1: Run all backend tests**

```powershell
cd src-tauri
cargo test
```
Expected: full suite green (existing 195 + the new tests added in tasks 1, 2, 3, 4,
6, 7, 8, 9, 10).

**Step 2: Run integration tests (full sweep, last verification)**

```powershell
npm run test:integration:build
npm run test:integration   # full suite
```
Expected: green.

**Step 3: Format + lint**

```powershell
npm run lint
npm run format
```
Expected: no errors.

**Step 4: Manual smoke test**

```powershell
npm run tauri dev
```
- Verify Settings → Paperless flow against the live
  [https://documents.lacny.me](https://documents.lacny.me) instance using the PAT
  from [.env](../../.env).
- Verify Doklady shows fuel rows with liters, car rows without.
- Verify "Open in Paperless" opens the doc in browser.
- Verify Assign persists across refresh.

**Step 5: Move task folder to `_done/`**

```powershell
git mv _tasks/60-paperless-integration _tasks/_done/60-paperless-integration
# Update _tasks/index.md row to point at _done/60-paperless-integration/, status ✅
git add _tasks/index.md
git commit -m "docs(tasks): mark paperless integration complete"
```

**Step 6: Open PR**

```powershell
git push -u origin feat/paperless-integration
gh pr create --title "feat: paperless-ngx integration as alternative invoice source" \
  --body-file <(echo -e "## Summary\n- Adds Paperless-ngx as an alternative source for the Doklady page.\n- New table paperless_trip_links holds 1:1 trip↔doc links.\n- Tag mapping: fuel→Fuel, car→Other.\n- All conditional logic in Rust per ADR-008.\n\n## Test plan\n- [ ] Backend tests (cargo test)\n- [ ] Tier-2 integration test\n- [ ] Manual smoke against documents.lacny.me\n")
```

---

## Risks & rollback

| Risk | Mitigation |
|---|---|
| Live Paperless schema drift (field renamed) | Hard sync error with the offending field name surfaced to the UI; user fixes in Paperless |
| Paperless instance reachable but rate-limiting | 5s timeout per request; UX shows banner; assignment writes blocked |
| User toggles Paperless on/off and gets confused | Local view is byte-for-byte unchanged when off; Paperless-only data lives in its own table |
| Bug in tag→assignment mapping | Backed by 4 unit tests + integration test verifying real fixtures |

**Rollback:** `git revert` the merge; the
[paperless_trip_links](../../src-tauri/core/migrations/) table remains (no data
loss); the [down.sql](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/down.sql) migration is available for a clean schema reset.

---

## Out of scope (explicitly deferred)

Per [01-task.md § Out of scope](./01-task.md):
- Migrating local receipts into Paperless
- Fuzzy matching local↔Paperless docs
- Bulk multi-select assign
- Caching Paperless docs in SQLite
- Other tags beyond `fuel` / `car`
- PAT keyring encryption (revisit globally with HA)
