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
- **PAT for live testing** (already in [.env](../../.env)):
  `PAPERLESS_API_TOKEN=REDACTED`.
  Use only for manual verification; **all automated tests must use a mock HTTP
  server** so CI doesn't depend on the live Paperless instance.

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
    app_state.set_read_only(true, "test".into());
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
- Modify: [src-tauri/core/src/commands_internal/integrations.rs](../../src-tauri/core/src/commands_internal/integrations.rs)
- Modify: [src-tauri/core/src/commands_internal/integrations_tests.rs](../../src-tauri/core/src/commands_internal/integrations_tests.rs)
- Add `wiremock` to `[dev-dependencies]` in
  [src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml) if not already
  present (used by HA test infra — check first).

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

**Step 2: Run migration locally**

```powershell
cd src-tauri/core
# Use Diesel's standard runner — schema.rs is regenerated automatically
# (per existing project convention; see other migrations for reference).
diesel migration run --database-url ../../target/test-paperless.db
```
Expected: prints `Running migration 2026-05-03-100000_add_paperless_trip_links`.
[schema.rs](../../src-tauri/core/src/schema.rs) should now contain a
`paperless_trip_links` table block.

**Step 3: Verify schema.rs change**

```powershell
git diff src-tauri/core/src/schema.rs
```
Expected: a new `paperless_trip_links` block with the four columns listed.

**Step 4: Build**

```powershell
cd ../..
cargo build -p kniha_jazd_core
```
Expected: clean compile.

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

In [db_tests.rs](../../src-tauri/core/src/db_tests.rs) (or wherever the existing
CRUD tests live — search for `receipts` CRUD tests as a template):

```rust
#[test]
fn paperless_link_upsert_creates_then_replaces() {
    let conn = &mut crate::db::tests::test_connection();
    let trip_a = crate::db::tests::seed_trip(conn);
    let trip_b = crate::db::tests::seed_trip(conn);

    crate::db::upsert_paperless_link(conn, &trip_a, 435).unwrap();
    assert_eq!(crate::db::get_paperless_link_for_doc(conn, 435).unwrap(),
               Some(trip_a.clone()));

    // Reassigning the same doc to a different trip moves the link.
    crate::db::upsert_paperless_link(conn, &trip_b, 435).unwrap();
    assert_eq!(crate::db::get_paperless_link_for_doc(conn, 435).unwrap(),
               Some(trip_b.clone()));
    assert_eq!(crate::db::get_paperless_link_for_trip(conn, &trip_a).unwrap(), None);
}

#[test]
fn paperless_link_delete_is_idempotent() {
    let conn = &mut crate::db::tests::test_connection();
    let trip = crate::db::tests::seed_trip(conn);
    crate::db::upsert_paperless_link(conn, &trip, 435).unwrap();
    crate::db::delete_paperless_link_for_doc(conn, 435).unwrap();
    crate::db::delete_paperless_link_for_doc(conn, 435).unwrap(); // no-op, no error
    assert_eq!(crate::db::get_paperless_link_for_doc(conn, 435).unwrap(), None);
}

#[test]
fn paperless_link_unique_doc_invariant() {
    let conn = &mut crate::db::tests::test_connection();
    let trip_a = crate::db::tests::seed_trip(conn);
    let trip_b = crate::db::tests::seed_trip(conn);
    crate::db::upsert_paperless_link(conn, &trip_a, 435).unwrap();
    // Same doc on a second trip: UPSERT replaces, doesn't duplicate.
    crate::db::upsert_paperless_link(conn, &trip_b, 435).unwrap();
    let count: i64 = diesel::dsl::sql_query("SELECT COUNT(*) AS c FROM paperless_trip_links")
        .get_result::<crate::db::tests::CountRow>(conn).unwrap().c;
    assert_eq!(count, 1);
}
```

**Step 2: Run, verify failure**

```powershell
cd src-tauri
cargo test -p kniha_jazd_core paperless_link
```
Expected: compile errors — model + CRUD functions missing.

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

In [db.rs](../../src-tauri/core/src/db.rs) — UPSERT semantics: PRIMARY KEY conflict
on `trip_id` REPLACES; UNIQUE conflict on `paperless_document_id` requires explicit
delete-then-insert because SQLite UPSERT only handles one conflict target:

```rust
pub fn upsert_paperless_link(
    conn: &mut SqliteConnection, trip_id: &str, doc_id: i64,
) -> QueryResult<()> {
    use crate::schema::paperless_trip_links::dsl as p;
    use diesel::prelude::*;
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

pub fn delete_paperless_link_for_doc(conn: &mut SqliteConnection, doc_id: i64) -> QueryResult<()> {
    use crate::schema::paperless_trip_links::dsl as p;
    diesel::delete(p::paperless_trip_links.filter(p::paperless_document_id.eq(doc_id)))
        .execute(conn).map(|_| ())
}

pub fn get_paperless_link_for_doc(conn: &mut SqliteConnection, doc_id: i64) -> QueryResult<Option<String>> {
    use crate::schema::paperless_trip_links::dsl as p;
    p::paperless_trip_links
        .filter(p::paperless_document_id.eq(doc_id))
        .select(p::trip_id)
        .first::<String>(conn).optional()
}

pub fn get_paperless_link_for_trip(conn: &mut SqliteConnection, trip_id: &str) -> QueryResult<Option<i64>> {
    use crate::schema::paperless_trip_links::dsl as p;
    p::paperless_trip_links
        .filter(p::trip_id.eq(trip_id))
        .select(p::paperless_document_id)
        .first::<i64>(conn).optional()
}

pub fn list_paperless_links_for_docs(
    conn: &mut SqliteConnection, doc_ids: &[i64],
) -> QueryResult<Vec<(i64, String)>> {
    use crate::schema::paperless_trip_links::dsl as p;
    p::paperless_trip_links
        .filter(p::paperless_document_id.eq_any(doc_ids))
        .select((p::paperless_document_id, p::trip_id))
        .load::<(i64, String)>(conn)
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
- Create: [src-tauri/core/src/paperless.rs](../../src-tauri/core/src/paperless.rs)
- Create: [src-tauri/core/src/paperless_tests.rs](../../src-tauri/core/src/paperless_tests.rs)
- Modify: [src-tauri/core/src/lib.rs](../../src-tauri/core/src/lib.rs)
  (add `pub mod paperless;`)

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

Add `urlencoding` to `[dependencies]` in
[src-tauri/core/Cargo.toml](../../src-tauri/core/Cargo.toml) if not present.

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
#[tokio::test]
async fn get_paperless_invoices_maps_fuel_only_to_fuel() {
    let row = super::test_helpers::make_doc(&[51], None);
    let assigned = super::map_assignment(&row.tag_ids, 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Fuel);
}

#[tokio::test]
async fn get_paperless_invoices_maps_car_only_to_other() {
    let row = super::test_helpers::make_doc(&[59], None);
    let assigned = super::map_assignment(&row.tag_ids, 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Other);
}

#[tokio::test]
async fn get_paperless_invoices_both_tags_priority_fuel() {
    let assigned = super::map_assignment(&[51, 59], 51, 59);
    assert_eq!(assigned, crate::models::AssignmentType::Fuel);
}

#[tokio::test]
async fn year_filter_uses_receipt_datetime_when_present() {
    let dt = chrono::NaiveDateTime::parse_from_str("2026-04-27T13:24:14", "%Y-%m-%dT%H:%M:%S").unwrap();
    let created = chrono::NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
    assert_eq!(super::doc_year(&Some(dt), &created), 2026);
}

#[tokio::test]
async fn year_filter_falls_back_to_created_when_no_datetime() {
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
use crate::models::{AssignmentType, PaperlessInvoiceRow};
use crate::paperless::{PaperlessClient, PaperlessError, PaperlessFieldMap, PaperlessDoc};
use crate::settings::LocalSettings;
use std::path::Path;

pub fn map_assignment(tag_ids: &[i64], fuel_id: i64, car_id: i64) -> AssignmentType {
    if tag_ids.contains(&fuel_id) { AssignmentType::Fuel }
    else if tag_ids.contains(&car_id) { AssignmentType::Other }
    else { AssignmentType::Other }   // unreachable in practice (server-side filter)
}

pub fn doc_year(dt: &Option<chrono::NaiveDateTime>, created: &chrono::NaiveDate) -> i32 {
    use chrono::Datelike;
    dt.as_ref().map(|d| d.year()).unwrap_or(created.year())
}

pub async fn get_paperless_invoices_internal(
    app_dir: &Path, conn: &mut diesel::SqliteConnection,
    _vehicle_id: &str, year: i32,
) -> Result<Vec<PaperlessInvoiceRow>, PaperlessError> {
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
    let links = crate::db::list_paperless_links_for_docs(conn, &doc_ids)
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
    pub fn make_doc(tag_ids: &[i64], _x: Option<()>) -> crate::paperless::PaperlessDoc {
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
Expected: 5 passed.

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

```rust
#[test]
fn assign_paperless_doc_blocked_when_read_only() {
    let conn = &mut crate::db::tests::test_connection();
    let trip = crate::db::tests::seed_trip(conn);
    let app_state = crate::app_state::AppState::new();
    app_state.set_read_only(true, "test".into());

    let err = super::assign_paperless_doc_to_trip_internal(&app_state, conn, 435, &trip)
        .unwrap_err();
    assert!(err.to_lowercase().contains("read"));
}

#[test]
fn assign_paperless_doc_persists_link() {
    let conn = &mut crate::db::tests::test_connection();
    let trip = crate::db::tests::seed_trip(conn);
    let app_state = crate::app_state::AppState::new();

    super::assign_paperless_doc_to_trip_internal(&app_state, conn, 435, &trip).unwrap();
    assert_eq!(crate::db::get_paperless_link_for_doc(conn, 435).unwrap(), Some(trip));
}

#[test]
fn unassign_paperless_doc_removes_link() {
    let conn = &mut crate::db::tests::test_connection();
    let trip = crate::db::tests::seed_trip(conn);
    let app_state = crate::app_state::AppState::new();

    super::assign_paperless_doc_to_trip_internal(&app_state, conn, 435, &trip).unwrap();
    super::unassign_paperless_doc_internal(&app_state, conn, 435).unwrap();
    assert_eq!(crate::db::get_paperless_link_for_doc(conn, 435).unwrap(), None);
}
```

**Step 2: Run, verify failures**

```powershell
cargo test -p kniha_jazd_core paperless_cmd_tests::assign paperless_cmd_tests::unassign
```

**Step 3: Implement**

```rust
// paperless_cmd.rs — append
pub fn assign_paperless_doc_to_trip_internal(
    app_state: &AppState, conn: &mut diesel::SqliteConnection,
    doc_id: i64, trip_id: &str,
) -> Result<(), String> {
    check_read_only!(app_state);
    crate::db::upsert_paperless_link(conn, trip_id, doc_id).map_err(|e| e.to_string())
}

pub fn unassign_paperless_doc_internal(
    app_state: &AppState, conn: &mut diesel::SqliteConnection, doc_id: i64,
) -> Result<(), String> {
    check_read_only!(app_state);
    crate::db::delete_paperless_link_for_doc(conn, doc_id).map_err(|e| e.to_string())
}
```

**Step 4: Run, verify pass**

```powershell
cargo test -p kniha_jazd_core paperless_cmd_tests
```
Expected: 8 passed (5 from task 9 + 3 new).

**Step 5: Commit**

```powershell
git add src-tauri/core/src/commands_internal/paperless_cmd.rs src-tauri/core/src/commands_internal/paperless_cmd_tests.rs
git commit -m "feat(paperless): assign/unassign trip↔doc commands (read-only-gated)"
```

---

### Task 11: Register all new Tauri commands in lib.rs

**Files:**
- Modify: [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs)
  (around line 290 — HA commands list)
- Modify: [src-tauri/desktop/src/commands.rs](../../src-tauri/desktop/src/commands.rs)
  (or wherever HA commands are wrapped — search for `test_ha_connection` to find
  the file; on probe it appears in
  [src-tauri/desktop/src/lib.rs](../../src-tauri/desktop/src/lib.rs):293)

**Step 1:** Add Tauri command wrappers next to the HA wrappers (paste-adapt
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

**Step 2:** Add all 7 to the `invoke_handler!` list in
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

**Step 3: Verify build**

```powershell
cd src-tauri
cargo build
```
Expected: clean build.

**Step 4:** No dedicated unit test (wrappers are mechanical). Tier-2 test in
[Task 15](#task-15-tier-2-integration-test-with-mock-paperless-server) exercises
them end-to-end.

**Step 5: Commit**

```powershell
git add src-tauri/desktop/src/
git commit -m "feat(tauri): register paperless commands"
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
  `InvoiceSourceMode`)

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

**Step 2: Write the spec**

```ts
import { describe, it, expect, before, after } from 'vitest';
import { startMockPaperless, stopMockPaperless } from '../_helpers/mock-paperless-server';

describe('paperless integration — full flow', () => {
  let mockUrl: string;

  before(async () => { mockUrl = await startMockPaperless(); });
  after(async () => { await stopMockPaperless(); });

  it('Settings → test → Doklady renders rows → Assign persists', async () => {
    // 1. Open Settings, enter URL + PAT, click Test
    await $('a[href="/settings"]').click();
    await $('input[data-test="paperless-url"]').setValue(mockUrl);
    await $('input[data-test="paperless-token"]').setValue('test-pat-123');
    await $('button[data-test="paperless-test-connection"]').click();
    await expect($('[data-test="paperless-status"]')).toHaveText('Pripojené');

    // 2. Doklady → assert 3 rows render with the verified titles
    await $('a[href="/doklady"]').click();
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
    await $('[data-test="trip-option"]').first().click();
    await expect($('[data-test="paperless-row"][data-doc-id="435"] [data-test="trip-indicator"]'))
      .toBeDisplayed();

    // 4. Refresh — assignment survives
    await $('button[data-test="paperless-refresh"]').click();
    await expect($('[data-test="paperless-row"][data-doc-id="435"] [data-test="trip-indicator"]'))
      .toBeDisplayed();

    // 5. Disable Paperless — doklady reverts to local view; the row no longer appears
    await $('a[href="/settings"]').click();
    await $('input[data-test="paperless-url"]').setValue('');
    // (token may also be cleared depending on existing UX — match the HA section's behavior)
    await $('a[href="/doklady"]').click();
    await expect($('[data-test="paperless-row"]')).not.toBeDisplayed();
  });
});
```

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
