# Invoice Scanning (Doklady) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable scanning fuel receipts from a folder, parsing with Gemini API, and assigning to trips.

**Architecture:** Local override file for dev settings → Receipt model in SQLite → Gemini API for OCR → Doklady page for management → Trip row integration for assignment.

**Tech Stack:** Rust (Tauri backend), SvelteKit (frontend), SQLite, Gemini 2.5 Flash Lite API, Playwright (E2E testing)

---

## Phase 1: Foundation

### Task 1.1: Local Settings Override File

**Files:**
- Create: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/lib.rs:1-10`
- Modify: `src-tauri/src/commands.rs` (add new command)
- Test: `src-tauri/src/settings.rs` (inline tests)

**Step 1: Write failing test for loading override file**

In `src-tauri/src/settings.rs`:

```rust
//! Local settings override file support
//! Priority: local.settings.json > database settings

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
}

impl LocalSettings {
    /// Load from local.settings.json in app data dir
    /// Returns default (empty) if file doesn't exist
    pub fn load(app_data_dir: &PathBuf) -> Self {
        let path = app_data_dir.join("local.settings.json");
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = tempdir().unwrap();
        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert!(settings.gemini_api_key.is_none());
        assert!(settings.receipts_folder_path.is_none());
    }

    #[test]
    fn test_load_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"{\"gemini_api_key\": \"test-key\", \"receipts_folder_path\": \"C:\\\\test\"}").unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.gemini_api_key, Some("test-key".to_string()));
        assert_eq!(settings.receipts_folder_path, Some("C:\\test".to_string()));
    }

    #[test]
    fn test_load_partial_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("local.settings.json");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(b"{\"gemini_api_key\": \"only-key\"}").unwrap();

        let settings = LocalSettings::load(&dir.path().to_path_buf());
        assert_eq!(settings.gemini_api_key, Some("only-key".to_string()));
        assert!(settings.receipts_folder_path.is_none());
    }
}
```

**Step 2: Run test to verify it passes**

```bash
cd src-tauri && cargo test settings::tests
```

Expected: 3 tests pass

**Step 3: Add tempfile dev dependency**

In `src-tauri/Cargo.toml`, add under `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = "3"
```

**Step 4: Register module in lib.rs**

In `src-tauri/src/lib.rs`, add after line 6:

```rust
mod settings;
```

**Step 5: Run all tests**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass

**Step 6: Commit**

```bash
git add src-tauri/src/settings.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(settings): add local override file support"
```

---

### Task 1.2: Add Receipt Model

**Files:**
- Modify: `src-tauri/src/models.rs`
- Modify: `src/lib/types.ts`

**Step 1: Add Receipt structs to models.rs**

In `src-tauri/src/models.rs`, add after TripGridData struct (around line 130):

```rust
/// Status of a scanned receipt
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReceiptStatus {
    Pending,      // File detected, not yet parsed
    Parsed,       // Successfully parsed with high confidence
    NeedsReview,  // Parsed but has uncertain fields
    Assigned,     // Linked to a trip
}

impl Default for ReceiptStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Typed confidence levels - prevents string inconsistencies
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ConfidenceLevel {
    #[default]
    Unknown,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldConfidence {
    pub liters: ConfidenceLevel,
    pub total_price: ConfidenceLevel,
    pub date: ConfidenceLevel,
}

/// A scanned fuel receipt (bloček)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub vehicle_id: Option<Uuid>,      // Set when assigned
    pub trip_id: Option<Uuid>,         // Set when assigned (UNIQUE when not null)
    pub file_path: String,             // Full path to image (UNIQUE)
    pub file_name: String,             // Just filename for display
    pub scanned_at: DateTime<Utc>,

    // Parsed fields (None = uncertain/failed)
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_date: Option<NaiveDate>,
    pub station_name: Option<String>,
    pub station_address: Option<String>,

    // Status tracking
    pub status: ReceiptStatus,
    pub confidence: FieldConfidence,   // Typed struct, not strings
    pub raw_ocr_text: Option<String>,  // For debugging (local only)
    pub error_message: Option<String>, // If parsing failed

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Receipt {
    pub fn new(file_path: String, file_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            vehicle_id: None,
            trip_id: None,
            file_path,
            file_name,
            scanned_at: now,
            liters: None,
            total_price_eur: None,
            receipt_date: None,
            station_name: None,
            station_address: None,
            status: ReceiptStatus::Pending,
            confidence: FieldConfidence::default(),
            raw_ocr_text: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_assigned(&self) -> bool {
        self.trip_id.is_some()
    }
}
```

**Step 2: Add TypeScript types**

In `src/lib/types.ts`, add at the end:

```typescript
export type ReceiptStatus = 'Pending' | 'Parsed' | 'NeedsReview' | 'Assigned';
export type ConfidenceLevel = 'Unknown' | 'High' | 'Medium' | 'Low';

export interface FieldConfidence {
	liters: ConfidenceLevel;
	total_price: ConfidenceLevel;
	date: ConfidenceLevel;
}

export interface Receipt {
	id: string;
	vehicle_id: string | null;
	trip_id: string | null;
	file_path: string;
	file_name: string;
	scanned_at: string;
	liters: number | null;
	total_price_eur: number | null;
	receipt_date: string | null;
	station_name: string | null;
	station_address: string | null;
	status: ReceiptStatus;
	confidence: FieldConfidence;
	raw_ocr_text: string | null;
	error_message: string | null;
	created_at: string;
	updated_at: string;
}

export interface ReceiptSettings {
	gemini_api_key: string | null;
	receipts_folder_path: string | null;
	gemini_api_key_from_override: boolean;
	receipts_folder_from_override: boolean;
}
```

**Step 3: Verify Rust compiles**

```bash
cd src-tauri && cargo check
```

**Step 4: Commit**

```bash
git add src-tauri/src/models.rs src/lib/types.ts
git commit -m "feat(models): add Receipt and ReceiptStatus types"
```

---

### Task 1.3: Database Migration for Receipts

**Files:**
- Modify: `src-tauri/src/db.rs`
- Test: `src-tauri/src/db.rs` (add tests)

**Step 1: Add receipts table migration**

In `src-tauri/src/db.rs`, in the `run_migrations` function, add after the full_tank migration (around line 70):

```rust
        // Add receipts table
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS receipts (
                id TEXT PRIMARY KEY,
                vehicle_id TEXT,
                trip_id TEXT UNIQUE,
                file_path TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                scanned_at TEXT NOT NULL,
                liters REAL,
                total_price_eur REAL,
                receipt_date TEXT,
                station_name TEXT,
                station_address TEXT,
                status TEXT NOT NULL DEFAULT 'Pending',
                confidence TEXT NOT NULL DEFAULT '{\"liters\":\"Unknown\",\"total_price\":\"Unknown\",\"date\":\"Unknown\"}',
                raw_ocr_text TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (vehicle_id) REFERENCES vehicles(id),
                FOREIGN KEY (trip_id) REFERENCES trips(id)
            );
            CREATE INDEX IF NOT EXISTS idx_receipts_status ON receipts(status);
            CREATE INDEX IF NOT EXISTS idx_receipts_trip ON receipts(trip_id);
            CREATE INDEX IF NOT EXISTS idx_receipts_date ON receipts(receipt_date);"
        )?;
```

**Step 2: Add Receipt types to imports at top of db.rs**

Change the import line:
```rust
use crate::models::{ConfidenceLevel, FieldConfidence, Receipt, ReceiptStatus, Route, Settings, Trip, Vehicle};
```

**Step 3: Add CRUD functions for receipts**

Add these functions to the Database impl (after existing CRUD functions):

```rust
    // ========================================================================
    // Receipt Operations
    // ========================================================================

    pub fn create_receipt(&self, receipt: &Receipt) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO receipts (id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                liters, total_price_eur, receipt_date, station_name, station_address,
                status, confidence, raw_ocr_text, error_message, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            (
                receipt.id.to_string(),
                receipt.vehicle_id.map(|id| id.to_string()),
                receipt.trip_id.map(|id| id.to_string()),
                &receipt.file_path,
                &receipt.file_name,
                receipt.scanned_at.to_rfc3339(),
                receipt.liters,
                receipt.total_price_eur,
                receipt.receipt_date.map(|d| d.to_string()),
                &receipt.station_name,
                &receipt.station_address,
                serde_json::to_string(&receipt.status).unwrap().trim_matches('"'),
                serde_json::to_string(&receipt.confidence).unwrap(),
                &receipt.raw_ocr_text,
                &receipt.error_message,
                receipt.created_at.to_rfc3339(),
                receipt.updated_at.to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    pub fn get_all_receipts(&self) -> Result<Vec<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                    liters, total_price_eur, receipt_date, station_name, station_address,
                    status, confidence, raw_ocr_text, error_message, created_at, updated_at
             FROM receipts ORDER BY scanned_at DESC"
        )?;

        let receipts = stmt.query_map([], |row| {
            Ok(Receipt {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                vehicle_id: row.get::<_, Option<String>>(1)?.map(|s| Uuid::parse_str(&s).unwrap()),
                trip_id: row.get::<_, Option<String>>(2)?.map(|s| Uuid::parse_str(&s).unwrap()),
                file_path: row.get(3)?,
                file_name: row.get(4)?,
                scanned_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                liters: row.get(6)?,
                total_price_eur: row.get(7)?,
                receipt_date: row.get::<_, Option<String>>(8)?
                    .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap()),
                station_name: row.get(9)?,
                station_address: row.get(10)?,
                status: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(11)?)).unwrap(),
                confidence: serde_json::from_str(&row.get::<_, String>(12)?).unwrap(),
                raw_ocr_text: row.get(13)?,
                error_message: row.get(14)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(16)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(receipts)
    }

    pub fn get_unassigned_receipts(&self) -> Result<Vec<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                    liters, total_price_eur, receipt_date, station_name, station_address,
                    status, confidence, raw_ocr_text, error_message, created_at, updated_at
             FROM receipts WHERE trip_id IS NULL ORDER BY receipt_date DESC, scanned_at DESC"
        )?;

        let receipts = stmt.query_map([], |row| {
            Ok(Receipt {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                vehicle_id: row.get::<_, Option<String>>(1)?.map(|s| Uuid::parse_str(&s).unwrap()),
                trip_id: row.get::<_, Option<String>>(2)?.map(|s| Uuid::parse_str(&s).unwrap()),
                file_path: row.get(3)?,
                file_name: row.get(4)?,
                scanned_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                liters: row.get(6)?,
                total_price_eur: row.get(7)?,
                receipt_date: row.get::<_, Option<String>>(8)?
                    .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap()),
                station_name: row.get(9)?,
                station_address: row.get(10)?,
                status: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(11)?)).unwrap(),
                confidence: serde_json::from_str(&row.get::<_, String>(12)?).unwrap(),
                raw_ocr_text: row.get(13)?,
                error_message: row.get(14)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(16)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(receipts)
    }

    pub fn update_receipt(&self, receipt: &Receipt) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE receipts SET
                vehicle_id = ?2, trip_id = ?3, liters = ?4, total_price_eur = ?5,
                receipt_date = ?6, station_name = ?7, station_address = ?8,
                status = ?9, confidence = ?10, raw_ocr_text = ?11,
                error_message = ?12, updated_at = ?13
             WHERE id = ?1",
            (
                receipt.id.to_string(),
                receipt.vehicle_id.map(|id| id.to_string()),
                receipt.trip_id.map(|id| id.to_string()),
                receipt.liters,
                receipt.total_price_eur,
                receipt.receipt_date.map(|d| d.to_string()),
                &receipt.station_name,
                &receipt.station_address,
                serde_json::to_string(&receipt.status).unwrap().trim_matches('"'),
                serde_json::to_string(&receipt.confidence).unwrap(),
                &receipt.raw_ocr_text,
                &receipt.error_message,
                Utc::now().to_rfc3339(),
            ),
        )?;
        Ok(())
    }

    pub fn delete_receipt(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM receipts WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_receipt_by_file_path(&self, file_path: &str) -> Result<Option<Receipt>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, vehicle_id, trip_id, file_path, file_name, scanned_at,
                    liters, total_price_eur, receipt_date, station_name, station_address,
                    status, confidence, raw_ocr_text, error_message, created_at, updated_at
             FROM receipts WHERE file_path = ?1"
        )?;

        stmt.query_row([file_path], |row| {
            Ok(Receipt {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                vehicle_id: row.get::<_, Option<String>>(1)?.map(|s| Uuid::parse_str(&s).unwrap()),
                trip_id: row.get::<_, Option<String>>(2)?.map(|s| Uuid::parse_str(&s).unwrap()),
                file_path: row.get(3)?,
                file_name: row.get(4)?,
                scanned_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                liters: row.get(6)?,
                total_price_eur: row.get(7)?,
                receipt_date: row.get::<_, Option<String>>(8)?
                    .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").unwrap()),
                station_name: row.get(9)?,
                station_address: row.get(10)?,
                status: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(11)?)).unwrap(),
                confidence: serde_json::from_str(&row.get::<_, String>(12)?).unwrap(),
                raw_ocr_text: row.get(13)?,
                error_message: row.get(14)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(15)?)
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(16)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        }).optional()
    }
```

**Step 4: Add tests for receipt CRUD**

Add at the end of the tests module in db.rs:

```rust
    #[test]
    fn test_receipt_crud() {
        let db = Database::in_memory().unwrap();

        // Create receipt
        let receipt = Receipt::new(
            "C:\\test\\receipt.jpg".to_string(),
            "receipt.jpg".to_string(),
        );
        db.create_receipt(&receipt).unwrap();

        // Get all receipts
        let receipts = db.get_all_receipts().unwrap();
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].file_name, "receipt.jpg");
        assert_eq!(receipts[0].status, ReceiptStatus::Pending);

        // Get by file path
        let found = db.get_receipt_by_file_path("C:\\test\\receipt.jpg").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, receipt.id);

        // Update receipt
        let mut updated = receipt.clone();
        updated.liters = Some(45.5);
        updated.total_price_eur = Some(72.50);
        updated.status = ReceiptStatus::Parsed;
        db.update_receipt(&updated).unwrap();

        let receipts = db.get_all_receipts().unwrap();
        assert_eq!(receipts[0].liters, Some(45.5));
        assert_eq!(receipts[0].status, ReceiptStatus::Parsed);

        // Delete receipt
        db.delete_receipt(&receipt.id.to_string()).unwrap();
        let receipts = db.get_all_receipts().unwrap();
        assert_eq!(receipts.len(), 0);
    }

    #[test]
    fn test_get_unassigned_receipts() {
        let db = Database::in_memory().unwrap();

        // Create two receipts
        let mut receipt1 = Receipt::new("path1.jpg".to_string(), "1.jpg".to_string());
        let mut receipt2 = Receipt::new("path2.jpg".to_string(), "2.jpg".to_string());

        // Assign one to a trip (fake trip_id)
        receipt2.trip_id = Some(Uuid::new_v4());
        receipt2.status = ReceiptStatus::Assigned;

        db.create_receipt(&receipt1).unwrap();
        db.create_receipt(&receipt2).unwrap();

        // Only unassigned should be returned
        let unassigned = db.get_unassigned_receipts().unwrap();
        assert_eq!(unassigned.len(), 1);
        assert_eq!(unassigned[0].file_name, "1.jpg");
    }
```

**Step 5: Run tests**

```bash
cd src-tauri && cargo test
```

Expected: All tests pass

**Step 6: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add receipts table and CRUD operations"
```

---

### Task 1.4: Playwright E2E Setup

**Files:**
- Create: `playwright.config.ts`
- Create: `tests/e2e/example.spec.ts`
- Modify: `package.json`
- Modify: `.gitignore`

**Step 1: Install Playwright**

```bash
npm install -D @playwright/test
npx playwright install chromium
```

**Step 2: Create Playwright config**

Create `playwright.config.ts`:

```typescript
import { defineConfig } from '@playwright/test';

export default defineConfig({
	testDir: './tests/e2e',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 1 : undefined,
	reporter: 'html',
	use: {
		baseURL: 'http://localhost:1420',
		trace: 'on-first-retry',
		screenshot: 'only-on-failure',
	},
	webServer: {
		command: 'npm run dev',
		url: 'http://localhost:1420',
		reuseExistingServer: !process.env.CI,
		timeout: 120000,
	},
});
```

**Step 3: Create example E2E test**

Create directory and file `tests/e2e/example.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

test('app loads and shows header', async ({ page }) => {
	await page.goto('/');

	// Check that the app header is visible
	await expect(page.locator('h1')).toContainText('Kniha Jázd');
});

test('navigation to settings works', async ({ page }) => {
	await page.goto('/');

	// Click settings link
	await page.click('a[href="/settings"]');

	// Verify we're on settings page
	await expect(page.locator('h1')).toContainText('Nastavenia');
});
```

**Step 4: Add npm scripts**

In `package.json`, add to scripts:

```json
"test:e2e": "playwright test",
"test:e2e:ui": "playwright test --ui"
```

**Step 5: Update .gitignore**

Add to `.gitignore`:

```
# Playwright
test-results/
playwright-report/
playwright/.cache/

# Local settings override (contains API keys)
local.settings.json
```

**Step 6: Run E2E tests**

Note: This requires the Tauri dev server. For initial setup, just verify config is valid:

```bash
npx playwright test --list
```

Expected: Shows list of tests without errors

**Step 7: Commit**

```bash
git add playwright.config.ts tests/e2e/example.spec.ts package.json .gitignore
git commit -m "feat(test): add Playwright E2E testing infrastructure"
```

---

## Phase 2: Gemini API Integration

### Task 2.1: Add reqwest dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add reqwest with blocking feature**

In `src-tauri/Cargo.toml`, add to dependencies:

```toml
reqwest = { version = "0.12", features = ["json", "blocking"] }
base64 = "0.22"
```

**Step 2: Verify it compiles**

```bash
cd src-tauri && cargo check
```

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore(deps): add reqwest and base64 for Gemini API"
```

---

### Task 2.2: Gemini API Client

**Files:**
- Create: `src-tauri/src/gemini.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create Gemini client with tests**

Create `src-tauri/src/gemini.rs`:

```rust
//! Gemini API client for receipt OCR and extraction

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedReceipt {
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_date: Option<String>, // YYYY-MM-DD format
    pub station_name: Option<String>,
    pub station_address: Option<String>,
    pub raw_text: Option<String>,
    pub confidence: ExtractionConfidence,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionConfidence {
    pub liters: String,      // "high", "medium", "low"
    pub total_price: String,
    pub date: String,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "responseMimeType")]
    response_mime_type: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

const EXTRACTION_PROMPT: &str = r#"Analyze this Slovak gas station receipt (bloček).
Extract the following fields as JSON:
{
  "liters": number or null,
  "total_price_eur": number or null,
  "receipt_date": "YYYY-MM-DD" or null,
  "station_name": string or null,
  "station_address": string or null,
  "raw_text": "full OCR text from receipt",
  "confidence": {
    "liters": "high" | "medium" | "low",
    "total_price": "high" | "medium" | "low",
    "date": "high" | "medium" | "low"
  }
}

Rules:
- Look for "L" or "litrov" near numbers for liters
- Look for "€" or "EUR" for total price, usually the largest amount
- Date formats on Slovak receipts: DD.MM.YYYY or DD.MM.YY
- Return null if a field cannot be determined with reasonable confidence
- Include station name and address if visible
- For confidence: "high" = clearly visible, "medium" = partially visible/guessed, "low" = very uncertain

Return ONLY valid JSON, no other text."#;

pub struct GeminiClient {
    api_key: String,
    client: reqwest::blocking::Client,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Extract receipt data from an image file
    pub fn extract_from_image(&self, image_path: &Path) -> Result<ExtractedReceipt, String> {
        // Read and encode image
        let image_data = fs::read(image_path)
            .map_err(|e| format!("Failed to read image: {}", e))?;
        let base64_image = STANDARD.encode(&image_data);

        // Determine mime type from extension
        let mime_type = match image_path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            _ => "image/jpeg", // Default
        };

        // Build request
        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![
                    Part::Text { text: EXTRACTION_PROMPT.to_string() },
                    Part::InlineData {
                        inline_data: InlineData {
                            mime_type: mime_type.to_string(),
                            data: base64_image,
                        },
                    },
                ],
            }],
            generation_config: GenerationConfig {
                response_mime_type: "application/json".to_string(),
            },
        };

        // Call Gemini API
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent?key={}",
            self.api_key
        );

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().unwrap_or_default();
            return Err(format!("API error {}: {}", status, error_text));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        // Extract JSON from response
        let text = gemini_response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or("No response text from API")?;

        // Parse extracted data
        let extracted: ExtractedReceipt = serde_json::from_str(&text)
            .map_err(|e| format!("Failed to parse extraction result: {} - Raw: {}", e, text))?;

        Ok(extracted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_prompt_is_valid() {
        // Just verify the prompt compiles and has expected content
        assert!(EXTRACTION_PROMPT.contains("liters"));
        assert!(EXTRACTION_PROMPT.contains("total_price_eur"));
        assert!(EXTRACTION_PROMPT.contains("YYYY-MM-DD"));
    }

    #[test]
    fn test_extracted_receipt_deserialization() {
        let json = r#"{
            "liters": 45.5,
            "total_price_eur": 72.50,
            "receipt_date": "2024-12-15",
            "station_name": "Slovnaft",
            "station_address": "Bratislava",
            "raw_text": "some text",
            "confidence": {
                "liters": "high",
                "total_price": "high",
                "date": "medium"
            }
        }"#;

        let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
        assert_eq!(extracted.liters, Some(45.5));
        assert_eq!(extracted.total_price_eur, Some(72.50));
        assert_eq!(extracted.receipt_date, Some("2024-12-15".to_string()));
        assert_eq!(extracted.confidence.liters, "high");
    }

    #[test]
    fn test_extracted_receipt_with_nulls() {
        let json = r#"{
            "liters": null,
            "total_price_eur": 50.00,
            "receipt_date": null,
            "station_name": null,
            "station_address": null,
            "raw_text": "blurry text",
            "confidence": {
                "liters": "low",
                "total_price": "medium",
                "date": "low"
            }
        }"#;

        let extracted: ExtractedReceipt = serde_json::from_str(json).unwrap();
        assert!(extracted.liters.is_none());
        assert_eq!(extracted.total_price_eur, Some(50.00));
        assert!(extracted.receipt_date.is_none());
    }
}
```

**Step 2: Register module in lib.rs**

In `src-tauri/src/lib.rs`, add after settings module:

```rust
mod gemini;
```

**Step 3: Run tests**

```bash
cd src-tauri && cargo test gemini
```

Expected: 3 tests pass

**Step 4: Commit**

```bash
git add src-tauri/src/gemini.rs src-tauri/src/lib.rs
git commit -m "feat(gemini): add Gemini API client for receipt extraction"
```

---

### Task 2.3: Receipt Scanning Service

**Files:**
- Create: `src-tauri/src/receipts.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Create receipt scanning service**

Create `src-tauri/src/receipts.rs`:

```rust
//! Receipt folder scanning and processing service

use crate::db::Database;
use crate::gemini::{ExtractedReceipt, GeminiClient};
use crate::models::{Receipt, ReceiptStatus};
use crate::settings::LocalSettings;
use chrono::NaiveDate;
use std::path::Path;

const SUPPORTED_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];

/// Scan folder for new receipt images and return count of new files found
pub fn scan_folder_for_new_receipts(
    folder_path: &str,
    db: &Database,
) -> Result<Vec<Receipt>, String> {
    let path = Path::new(folder_path);
    if !path.exists() {
        return Err(format!("Folder does not exist: {}", folder_path));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", folder_path));
    }

    let mut new_receipts = Vec::new();

    let entries = std::fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries.flatten() {
        let file_path = entry.path();

        // Skip non-files
        if !file_path.is_file() {
            continue;
        }

        // Check extension
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        if !extension.map(|e| SUPPORTED_EXTENSIONS.contains(&e.as_str())).unwrap_or(false) {
            continue;
        }

        let file_path_str = file_path.to_string_lossy().to_string();

        // Check if already processed
        if db.get_receipt_by_file_path(&file_path_str)
            .map_err(|e| e.to_string())?
            .is_some()
        {
            continue;
        }

        // Create new receipt record
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let receipt = Receipt::new(file_path_str, file_name);
        db.create_receipt(&receipt).map_err(|e| e.to_string())?;
        new_receipts.push(receipt);
    }

    Ok(new_receipts)
}

/// Process a pending receipt with Gemini API
pub fn process_receipt_with_gemini(
    receipt: &mut Receipt,
    api_key: &str,
) -> Result<(), String> {
    let client = GeminiClient::new(api_key.to_string());
    let path = Path::new(&receipt.file_path);

    match client.extract_from_image(path) {
        Ok(extracted) => {
            apply_extraction_to_receipt(receipt, extracted);
            Ok(())
        }
        Err(e) => {
            receipt.status = ReceiptStatus::NeedsReview;
            receipt.error_message = Some(e.clone());
            Err(e)
        }
    }
}

fn apply_extraction_to_receipt(receipt: &mut Receipt, extracted: ExtractedReceipt) {
    receipt.liters = extracted.liters;
    receipt.total_price_eur = extracted.total_price_eur;
    receipt.receipt_date = extracted.receipt_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
    receipt.station_name = extracted.station_name;
    receipt.station_address = extracted.station_address;
    receipt.raw_ocr_text = extracted.raw_text;

    // Determine status based on confidence
    let mut flags = Vec::new();

    if extracted.confidence.liters == "low" || extracted.liters.is_none() {
        flags.push("liters_uncertain".to_string());
    }
    if extracted.confidence.total_price == "low" || extracted.total_price_eur.is_none() {
        flags.push("price_uncertain".to_string());
    }
    if extracted.confidence.date == "low" || receipt.receipt_date.is_none() {
        flags.push("date_uncertain".to_string());
    }

    receipt.confidence = flags.clone();

    // Set status: NeedsReview if any uncertainty, otherwise Parsed
    if flags.is_empty() {
        receipt.status = ReceiptStatus::Parsed;
    } else {
        receipt.status = ReceiptStatus::NeedsReview;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ReceiptStatus;

    #[test]
    fn test_apply_extraction_high_confidence() {
        let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
        let extracted = ExtractedReceipt {
            liters: Some(45.5),
            total_price_eur: Some(72.50),
            receipt_date: Some("2024-12-15".to_string()),
            station_name: Some("Slovnaft".to_string()),
            station_address: Some("Bratislava".to_string()),
            raw_text: Some("OCR text".to_string()),
            confidence: crate::gemini::ExtractionConfidence {
                liters: "high".to_string(),
                total_price: "high".to_string(),
                date: "high".to_string(),
            },
        };

        apply_extraction_to_receipt(&mut receipt, extracted);

        assert_eq!(receipt.liters, Some(45.5));
        assert_eq!(receipt.total_price_eur, Some(72.50));
        assert_eq!(receipt.status, ReceiptStatus::Parsed);
        assert!(receipt.confidence.is_empty());
    }

    #[test]
    fn test_apply_extraction_low_confidence() {
        let mut receipt = Receipt::new("test.jpg".to_string(), "test.jpg".to_string());
        let extracted = ExtractedReceipt {
            liters: None,
            total_price_eur: Some(50.00),
            receipt_date: None,
            station_name: None,
            station_address: None,
            raw_text: Some("blurry".to_string()),
            confidence: crate::gemini::ExtractionConfidence {
                liters: "low".to_string(),
                total_price: "medium".to_string(),
                date: "low".to_string(),
            },
        };

        apply_extraction_to_receipt(&mut receipt, extracted);

        assert_eq!(receipt.status, ReceiptStatus::NeedsReview);
        assert!(receipt.confidence.contains(&"liters_uncertain".to_string()));
        assert!(receipt.confidence.contains(&"date_uncertain".to_string()));
    }
}
```

**Step 2: Register module in lib.rs**

In `src-tauri/src/lib.rs`, add after gemini module:

```rust
mod receipts;
```

**Step 3: Run tests**

```bash
cd src-tauri && cargo test receipts
```

Expected: 2 tests pass

**Step 4: Commit**

```bash
git add src-tauri/src/receipts.rs src-tauri/src/lib.rs
git commit -m "feat(receipts): add folder scanning and Gemini processing"
```

---

### Task 2.4: Receipt Tauri Commands

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/api.ts`

**Step 1: Add receipt commands to commands.rs**

Add at the end of `src-tauri/src/commands.rs`:

```rust
// ============================================================================
// Receipt Commands
// ============================================================================

use crate::models::{Receipt, ReceiptStatus};
use crate::receipts::{process_receipt_with_gemini, scan_folder_for_new_receipts};
use crate::settings::LocalSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSettings {
    pub gemini_api_key: Option<String>,
    pub receipts_folder_path: Option<String>,
    pub gemini_api_key_from_override: bool,
    pub receipts_folder_from_override: bool,
}

#[tauri::command]
pub fn get_receipt_settings(app: tauri::AppHandle) -> Result<ReceiptSettings, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let local = LocalSettings::load(&app_dir);

    // For now, only local settings are supported
    // TODO: Add DB settings and merge with override priority
    Ok(ReceiptSettings {
        gemini_api_key: local.gemini_api_key.clone(),
        receipts_folder_path: local.receipts_folder_path.clone(),
        gemini_api_key_from_override: local.gemini_api_key.is_some(),
        receipts_folder_from_override: local.receipts_folder_path.is_some(),
    })
}

#[tauri::command]
pub fn get_receipts(db: State<Database>) -> Result<Vec<Receipt>, String> {
    db.get_all_receipts().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_unassigned_receipts(db: State<Database>) -> Result<Vec<Receipt>, String> {
    db.get_unassigned_receipts().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_receipts(app: tauri::AppHandle, db: State<Database>) -> Result<Vec<Receipt>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings = LocalSettings::load(&app_dir);

    let folder_path = settings.receipts_folder_path
        .ok_or("Receipts folder not configured")?;

    let api_key = settings.gemini_api_key
        .ok_or("Gemini API key not configured")?;

    // Scan for new files
    let mut new_receipts = scan_folder_for_new_receipts(&folder_path, &db)?;

    // Process each new receipt with Gemini
    for receipt in &mut new_receipts {
        if let Err(e) = process_receipt_with_gemini(receipt, &api_key) {
            log::warn!("Failed to process receipt {}: {}", receipt.file_name, e);
        }
        // Update in DB regardless of success/failure
        db.update_receipt(receipt).map_err(|e| e.to_string())?;
    }

    Ok(new_receipts)
}

#[tauri::command]
pub fn update_receipt(db: State<Database>, receipt: Receipt) -> Result<(), String> {
    db.update_receipt(&receipt).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_receipt(db: State<Database>, id: String) -> Result<(), String> {
    db.delete_receipt(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn assign_receipt_to_trip(
    db: State<Database>,
    receipt_id: String,
    trip_id: String,
    vehicle_id: String,
) -> Result<Receipt, String> {
    let mut receipts = db.get_all_receipts().map_err(|e| e.to_string())?;
    let receipt = receipts
        .iter_mut()
        .find(|r| r.id.to_string() == receipt_id)
        .ok_or("Receipt not found")?;

    receipt.trip_id = Some(Uuid::parse_str(&trip_id).map_err(|e| e.to_string())?);
    receipt.vehicle_id = Some(Uuid::parse_str(&vehicle_id).map_err(|e| e.to_string())?);
    receipt.status = ReceiptStatus::Assigned;

    db.update_receipt(receipt).map_err(|e| e.to_string())?;

    Ok(receipt.clone())
}
```

**Step 2: Register commands in lib.rs**

In `src-tauri/src/lib.rs`, update the invoke_handler to add the new commands:

```rust
      commands::get_receipt_settings,
      commands::get_receipts,
      commands::get_unassigned_receipts,
      commands::sync_receipts,
      commands::update_receipt,
      commands::delete_receipt,
      commands::assign_receipt_to_trip,
```

**Step 3: Add API functions in api.ts**

Add at the end of `src/lib/api.ts`:

```typescript
// Receipt commands
export async function getReceiptSettings(): Promise<ReceiptSettings> {
	return await invoke('get_receipt_settings');
}

export async function getReceipts(): Promise<Receipt[]> {
	return await invoke('get_receipts');
}

export async function getUnassignedReceipts(): Promise<Receipt[]> {
	return await invoke('get_unassigned_receipts');
}

export async function syncReceipts(): Promise<Receipt[]> {
	return await invoke('sync_receipts');
}

export async function updateReceipt(receipt: Receipt): Promise<void> {
	return await invoke('update_receipt', { receipt });
}

export async function deleteReceipt(id: string): Promise<void> {
	return await invoke('delete_receipt', { id });
}

export async function assignReceiptToTrip(
	receiptId: string,
	tripId: string,
	vehicleId: string
): Promise<Receipt> {
	return await invoke('assign_receipt_to_trip', { receiptId, tripId, vehicleId });
}
```

**Step 4: Add ReceiptSettings import to api.ts**

Update the import line:

```typescript
import type { Vehicle, Trip, Route, CompensationSuggestion, Settings, TripStats, BackupInfo, TripGridData, Receipt, ReceiptSettings } from './types';
```

**Step 5: Verify compilation**

```bash
cd src-tauri && cargo check
```

**Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src/lib/api.ts
git commit -m "feat(commands): add receipt Tauri commands and API"
```

---

## Phase 3: Doklady UI

### Task 3.1: Doklady Page

**Files:**
- Create: `src/routes/doklady/+page.svelte`
- Modify: `src/routes/+layout.svelte`

**Step 1: Create Doklady page**

Create directory and file `src/routes/doklady/+page.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Receipt, ReceiptSettings } from '$lib/types';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';

	let receipts = $state<Receipt[]>([]);
	let settings = $state<ReceiptSettings | null>(null);
	let loading = $state(true);
	let syncing = $state(false);
	let filter = $state<'all' | 'unassigned' | 'needs_review'>('all');
	let receiptToDelete = $state<Receipt | null>(null);

	onMount(async () => {
		await loadSettings();
		await loadReceipts();
	});

	async function loadSettings() {
		try {
			settings = await api.getReceiptSettings();
		} catch (error) {
			console.error('Failed to load receipt settings:', error);
		}
	}

	async function loadReceipts() {
		loading = true;
		try {
			receipts = await api.getReceipts();
		} catch (error) {
			console.error('Failed to load receipts:', error);
			toast.error('Nepodarilo sa načítať doklady');
		} finally {
			loading = false;
		}
	}

	async function handleSync() {
		if (!settings?.gemini_api_key || !settings?.receipts_folder_path) {
			toast.error('Najprv nastavte priečinok a API kľúč v Nastaveniach');
			return;
		}

		syncing = true;
		try {
			const newReceipts = await api.syncReceipts();
			await loadReceipts();
			if (newReceipts.length > 0) {
				toast.success(`Načítaných ${newReceipts.length} nových dokladov`);
			} else {
				toast.success('Žiadne nové doklady');
			}
		} catch (error) {
			console.error('Failed to sync receipts:', error);
			toast.error('Nepodarilo sa synchronizovať: ' + error);
		} finally {
			syncing = false;
		}
	}

	function handleDeleteClick(receipt: Receipt) {
		receiptToDelete = receipt;
	}

	async function handleConfirmDelete() {
		if (!receiptToDelete) return;
		try {
			await api.deleteReceipt(receiptToDelete.id);
			await loadReceipts();
			toast.success('Doklad bol odstránený');
		} catch (error) {
			console.error('Failed to delete receipt:', error);
			toast.error('Nepodarilo sa odstrániť doklad');
		} finally {
			receiptToDelete = null;
		}
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return '--';
		try {
			const date = new Date(dateStr);
			return date.toLocaleDateString('sk-SK');
		} catch {
			return dateStr;
		}
	}

	function getStatusBadge(status: string): { text: string; class: string } {
		switch (status) {
			case 'Parsed':
				return { text: 'Spracovaný', class: 'success' };
			case 'NeedsReview':
				return { text: 'Na kontrolu', class: 'warning' };
			case 'Assigned':
				return { text: 'Pridelený', class: 'info' };
			case 'Pending':
			default:
				return { text: 'Čaká', class: 'neutral' };
		}
	}

	$: filteredReceipts = receipts.filter((r) => {
		if (filter === 'unassigned') return r.status !== 'Assigned';
		if (filter === 'needs_review') return r.status === 'NeedsReview';
		return true;
	});

	$: isConfigured = settings?.gemini_api_key && settings?.receipts_folder_path;
</script>

<div class="doklady-page">
	<div class="header">
		<h1>Doklady</h1>
		<div class="header-actions">
			<button class="button" onclick={handleSync} disabled={syncing || !isConfigured}>
				{syncing ? 'Synchronizujem...' : '🔄 Sync'}
			</button>
		</div>
	</div>

	{#if !isConfigured}
		<div class="config-warning">
			<p>⚠️ Funkcia dokladov nie je nakonfigurovaná.</p>
			<p>
				Nastavte <strong>priečinok s dokladmi</strong> a <strong>Gemini API kľúč</strong> v súbore
				<code>local.settings.json</code> v priečinku aplikácie.
			</p>
		</div>
	{/if}

	<div class="filters">
		<button class="filter-btn" class:active={filter === 'all'} onclick={() => (filter = 'all')}>
			Všetky ({receipts.length})
		</button>
		<button
			class="filter-btn"
			class:active={filter === 'unassigned'}
			onclick={() => (filter = 'unassigned')}
		>
			Nepridelené ({receipts.filter((r) => r.status !== 'Assigned').length})
		</button>
		<button
			class="filter-btn"
			class:active={filter === 'needs_review'}
			onclick={() => (filter = 'needs_review')}
		>
			Na kontrolu ({receipts.filter((r) => r.status === 'NeedsReview').length})
		</button>
	</div>

	{#if loading}
		<p class="placeholder">Načítavam...</p>
	{:else if filteredReceipts.length === 0}
		<p class="placeholder">Žiadne doklady. Kliknite na Sync pre načítanie nových.</p>
	{:else}
		<div class="receipts-list">
			{#each filteredReceipts as receipt}
				{@const badge = getStatusBadge(receipt.status)}
				<div class="receipt-card">
					<div class="receipt-header">
						<span class="file-name">📄 {receipt.file_name}</span>
						<span class="badge {badge.class}">{badge.text}</span>
					</div>
					<div class="receipt-details">
						<div class="detail-row">
							<span class="label">Dátum:</span>
							<span class="value">{formatDate(receipt.receipt_date)}</span>
						</div>
						<div class="detail-row">
							<span class="label">Litre:</span>
							<span class="value" class:uncertain={receipt.confidence.includes('liters_uncertain')}>
								{receipt.liters != null ? `${receipt.liters.toFixed(2)} L` : '??'}
							</span>
						</div>
						<div class="detail-row">
							<span class="label">Cena:</span>
							<span class="value" class:uncertain={receipt.confidence.includes('price_uncertain')}>
								{receipt.total_price_eur != null ? `${receipt.total_price_eur.toFixed(2)} €` : '??'}
							</span>
						</div>
						{#if receipt.station_name}
							<div class="detail-row">
								<span class="label">Stanica:</span>
								<span class="value">{receipt.station_name}</span>
							</div>
						{/if}
						{#if receipt.error_message}
							<div class="error-message">❌ {receipt.error_message}</div>
						{/if}
					</div>
					<div class="receipt-actions">
						{#if receipt.status !== 'Assigned'}
							<button class="button-small">Prideliť k jazde</button>
						{/if}
						<button class="button-small danger" onclick={() => handleDeleteClick(receipt)}>
							Zmazať
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

{#if receiptToDelete}
	<ConfirmModal
		title="Odstrániť doklad"
		message={`Naozaj chcete odstrániť doklad "${receiptToDelete.file_name}"?`}
		confirmText="Odstrániť"
		danger={true}
		onConfirm={handleConfirmDelete}
		onCancel={() => (receiptToDelete = null)}
	/>
{/if}

<style>
	.doklady-page {
		max-width: 800px;
		margin: 0 auto;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.header h1 {
		margin: 0;
		color: #2c3e50;
	}

	.config-warning {
		background: #fff3cd;
		border: 1px solid #ffc107;
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}

	.config-warning p {
		margin: 0.5rem 0;
	}

	.config-warning code {
		background: #f8f9fa;
		padding: 0.2rem 0.4rem;
		border-radius: 3px;
		font-size: 0.875rem;
	}

	.filters {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.filter-btn {
		padding: 0.5rem 1rem;
		border: 1px solid #ddd;
		background: white;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.filter-btn:hover {
		background: #f5f5f5;
	}

	.filter-btn.active {
		background: #3498db;
		color: white;
		border-color: #3498db;
	}

	.receipts-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.receipt-card {
		background: white;
		border-radius: 8px;
		padding: 1rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.receipt-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
	}

	.file-name {
		font-weight: 500;
		color: #2c3e50;
	}

	.badge {
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-weight: 600;
	}

	.badge.success {
		background: #d4edda;
		color: #155724;
	}

	.badge.warning {
		background: #fff3cd;
		color: #856404;
	}

	.badge.info {
		background: #cce5ff;
		color: #004085;
	}

	.badge.neutral {
		background: #e9ecef;
		color: #495057;
	}

	.receipt-details {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.detail-row {
		display: flex;
		gap: 0.5rem;
	}

	.label {
		color: #7f8c8d;
		font-size: 0.875rem;
	}

	.value {
		font-weight: 500;
		color: #2c3e50;
	}

	.value.uncertain {
		color: #e67e22;
	}

	.error-message {
		grid-column: 1 / -1;
		color: #c0392b;
		font-size: 0.875rem;
		padding: 0.5rem;
		background: #fee;
		border-radius: 4px;
	}

	.receipt-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.placeholder {
		color: #7f8c8d;
		font-style: italic;
		text-align: center;
		padding: 2rem;
	}

	.button {
		padding: 0.75rem 1.5rem;
		background-color: #3498db;
		color: white;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button:hover:not(:disabled) {
		background-color: #2980b9;
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.button-small {
		padding: 0.5rem 1rem;
		background-color: #ecf0f1;
		color: #2c3e50;
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.button-small:hover {
		background-color: #d5dbdb;
	}

	.button-small.danger {
		background-color: #fee;
		color: #c0392b;
	}

	.button-small.danger:hover {
		background-color: #fdd;
	}
</style>
```

**Step 2: Add navigation link in layout.svelte**

In `src/routes/+layout.svelte`, find the nav section and add Doklady link after Kniha jázd:

```svelte
<a href="/doklady" class="nav-link" class:active={$page.url.pathname === '/doklady'}>Doklady</a>
```

**Step 3: Verify app runs**

```bash
npm run tauri dev
```

Navigate to /doklady and verify page loads.

**Step 4: Commit**

```bash
git add src/routes/doklady/+page.svelte src/routes/+layout.svelte
git commit -m "feat(ui): add Doklady page for receipt management"
```

---

### Task 3.2: Floating Indicator Component

**Files:**
- Create: `src/lib/components/ReceiptIndicator.svelte`
- Modify: `src/routes/+layout.svelte`

**Step 1: Create indicator component**

Create `src/lib/components/ReceiptIndicator.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { goto } from '$app/navigation';

	let unassignedCount = $state(0);
	let loading = $state(true);

	onMount(async () => {
		await loadCount();
		// Refresh count every 30 seconds
		const interval = setInterval(loadCount, 30000);
		return () => clearInterval(interval);
	});

	async function loadCount() {
		try {
			const receipts = await api.getUnassignedReceipts();
			unassignedCount = receipts.length;
		} catch (error) {
			console.error('Failed to load unassigned receipts:', error);
		} finally {
			loading = false;
		}
	}

	function handleClick() {
		goto('/doklady?filter=unassigned');
	}
</script>

{#if !loading && unassignedCount > 0}
	<button class="indicator" onclick={handleClick} title="Nepridelené doklady">
		📄 {unassignedCount}
	</button>
{/if}

<style>
	.indicator {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.5rem 0.75rem;
		background: #e74c3c;
		color: white;
		border: none;
		border-radius: 20px;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.2s;
	}

	.indicator:hover {
		background: #c0392b;
	}
</style>
```

**Step 2: Add to layout**

In `src/routes/+layout.svelte`, add the import:

```svelte
import ReceiptIndicator from '$lib/components/ReceiptIndicator.svelte';
```

And add the component in the header-right div, before vehicle-selector:

```svelte
<div class="header-right">
	<ReceiptIndicator />
	<div class="vehicle-selector">
```

**Step 3: Verify it shows**

Run the app and verify the indicator appears when there are unassigned receipts.

**Step 4: Commit**

```bash
git add src/lib/components/ReceiptIndicator.svelte src/routes/+layout.svelte
git commit -m "feat(ui): add floating indicator for unassigned receipts"
```

---

### Task 3.3: E2E Tests for Doklady

**Files:**
- Create: `tests/e2e/doklady.spec.ts`

**Step 1: Create E2E test**

Create `tests/e2e/doklady.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

test.describe('Doklady page', () => {
	test('navigates to doklady page', async ({ page }) => {
		await page.goto('/');
		await page.click('a[href="/doklady"]');
		await expect(page.locator('h1')).toContainText('Doklady');
	});

	test('shows configuration warning when not configured', async ({ page }) => {
		await page.goto('/doklady');
		await expect(page.locator('.config-warning')).toBeVisible();
		await expect(page.locator('.config-warning')).toContainText('nie je nakonfigurovaná');
	});

	test('sync button is disabled when not configured', async ({ page }) => {
		await page.goto('/doklady');
		const syncButton = page.locator('button:has-text("Sync")');
		await expect(syncButton).toBeDisabled();
	});

	test('filter buttons work', async ({ page }) => {
		await page.goto('/doklady');

		// All filters should be visible
		await expect(page.locator('.filter-btn:has-text("Všetky")')).toBeVisible();
		await expect(page.locator('.filter-btn:has-text("Nepridelené")')).toBeVisible();
		await expect(page.locator('.filter-btn:has-text("Na kontrolu")')).toBeVisible();

		// Click unassigned filter
		await page.click('.filter-btn:has-text("Nepridelené")');
		await expect(page.locator('.filter-btn:has-text("Nepridelené")')).toHaveClass(/active/);
	});
});
```

**Step 2: Run E2E tests**

```bash
npm run test:e2e
```

Note: Tests require Tauri dev server running. If tests fail due to app not running, that's expected - they're designed to run with the full app.

**Step 3: Commit**

```bash
git add tests/e2e/doklady.spec.ts
git commit -m "test(e2e): add Doklady page tests"
```

---

## Phase 4: Trip Integration (Future)

The remaining tasks for Phase 4 and 5 cover:

### Task 4.1: Receipt Picker for Trip Row
- Add dropdown to TripRow.svelte for selecting a receipt when entering fuel
- Filter by date proximity (±3 days)
- Auto-fill liters and price from selected receipt

### Task 4.2: Assign from Doklady Page
- Add "Prideliť k jazde" button that opens trip selector modal
- Show trips for active vehicle, filter by date
- Link receipt to trip and update trip fuel fields

### Task 4.3: Smart Date Matching
- Sort receipts by date proximity to trip date
- Highlight best matches in picker

### Task 5.1: Station Profiles (Future Enhancement)
- Add StationProfile model
- Auto-detect station from receipt
- Store user corrections as few-shot examples
- Station management UI

---

## Summary of Commits

After completing all tasks, the commits should be:

1. `feat(settings): add local override file support`
2. `feat(models): add Receipt and ReceiptStatus types`
3. `feat(db): add receipts table and CRUD operations`
4. `feat(test): add Playwright E2E testing infrastructure`
5. `chore(deps): add reqwest and base64 for Gemini API`
6. `feat(gemini): add Gemini API client for receipt extraction`
7. `feat(receipts): add folder scanning and Gemini processing`
8. `feat(commands): add receipt Tauri commands and API`
9. `feat(ui): add Doklady page for receipt management`
10. `feat(ui): add floating indicator for unassigned receipts`
11. `test(e2e): add Doklady page tests`

---

## Test Data Setup

For development testing, create `C:\_dev\_tmp\doklady` folder and place some sample receipt images there.

Create `%APPDATA%\com.notavailable.kniha-jazd\local.settings.json`:

```json
{
  "gemini_api_key": "YOUR_API_KEY_HERE",
  "receipts_folder_path": "C:\\_dev\\_tmp\\doklady"
}
```
