# Design: Invoice/Receipt Scanning (Doklady)

## Data Model

### New `Receipt` entity (Rust)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub id: Uuid,
    pub vehicle_id: Option<Uuid>,      // Set when assigned to a trip
    pub trip_id: Option<Uuid>,         // Set when assigned to a trip
    pub file_path: String,             // Original image path
    pub file_name: String,             // Just the filename for display
    pub scanned_at: DateTime<Utc>,

    // Parsed fields (None = uncertain/failed)
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,
    pub receipt_date: Option<NaiveDate>,
    pub station_name: Option<String>,
    pub station_address: Option<String>,

    // Status tracking
    pub status: ReceiptStatus,         // Pending, Parsed, NeedsReview, Assigned
    pub confidence_flags: Vec<String>, // ["liters_uncertain", "date_unclear"]
    pub raw_ocr_text: Option<String>,  // For debugging/manual review
    pub error_message: Option<String>, // If parsing failed

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReceiptStatus {
    Pending,      // File detected, not yet parsed
    Parsed,       // Successfully parsed with high confidence
    NeedsReview,  // Parsed but has uncertain fields
    Assigned,     // Linked to a trip
}
```

### New `StationProfile` entity (for fine-tuning)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StationProfile {
    pub id: Uuid,
    pub name: String,                    // "Slovnaft", "OMV", "Shell"
    pub detection_keywords: Vec<String>, // ["SLOVNAFT", "MOL Group"]
    pub prompt_hints: Option<String>,    // "Liters shown as 'Množstvo:'"
    pub example_extractions: Vec<ExampleExtraction>, // Few-shot examples
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleExtraction {
    pub raw_text_snippet: String,
    pub extracted_liters: Option<f64>,
    pub extracted_price: Option<f64>,
}
```

### Settings additions

```rust
// In Settings struct
pub gemini_api_key: Option<String>,
pub receipts_folder_path: Option<String>,
```

### Local Override File

Location: `%APPDATA%/com.notavailable.kniha-jazd/local.settings.json`

```json
{
  "gemini_api_key": "AIza...",
  "receipts_folder_path": "C:\\_dev\\_tmp\\doklady"
}
```

**Priority:** Override file > DB settings

**Behavior:**
- On app startup, check if override file exists
- If field is set in override file, use it (even if DB has a value)
- Override file is gitignored, used for local development
- UI shows "(override)" indicator if value comes from file

---

## Workflow: Folder Scanning

**Not real-time file watcher** - scan on demand for simplicity.

```
1. User opens Doklady page or clicks "Sync" button
2. Backend scans configured folder for images (.jpg, .png, .jpeg, .webp)
3. Compare against already-processed file paths in DB
4. For each new file:
   a. Create Receipt with status = Pending
   b. Send image to Gemini API with extraction prompt
   c. Parse response into Receipt fields
   d. Set confidence_flags for uncertain fields
   e. Update status = Parsed or NeedsReview
5. UI refreshes to show new receipts
```

**Why not real-time watching:**
- Simpler, more reliable across platforms
- User controls when parsing happens (and API costs)
- Avoids edge cases (file still being written, cloud sync delays)

---

## Gemini API Integration

### Prompt structure

```
Analyze this Slovak gas station receipt (bloček).
Extract the following fields as JSON:
{
  "liters": number or null,
  "total_price_eur": number or null,
  "receipt_date": "YYYY-MM-DD" or null,
  "station_name": string or null,
  "station_address": string or null,
  "raw_text": "full OCR text",
  "confidence": {
    "liters": "high" | "medium" | "low",
    "total_price": "high" | "medium" | "low",
    "date": "high" | "medium" | "low"
  }
}

Rules:
- Look for "L" or "litrov" near numbers for liters
- Look for "€" or "EUR" for price, usually the largest amount
- Date formats: DD.MM.YYYY or DD.MM.YY
- Return null if field cannot be determined
- Include any station name/address visible on receipt

{station_specific_hints}
```

### Station-specific fine-tuning

1. First scan: Gemini identifies station from receipt text
2. If known station in StationProfile: append prompt_hints and few-shot examples
3. User corrections feed back: when user fixes a parsed value, offer to save as example
4. Over time: each station builds up few-shot examples improving accuracy

### Error handling

- API timeout/network error → status = NeedsReview, store error_message
- Low confidence on any field → add to confidence_flags, status = NeedsReview
- All fields null → status = NeedsReview with "Nepodarilo sa rozpoznať"

### Cost estimate

- Gemini 2.5 Flash Lite: ~$0.075 per 1M input tokens
- One receipt image ≈ 1-2K tokens
- ~500 receipts per $0.10 - negligible

---

## UI Components

### 1. Settings page additions

```
┌─ Doklady (Receipts) ─────────────────────────────┐
│ Priečinok s dokladmi: [_________________] [Vybrať] │
│ Gemini API kľúč:      [_________________] [Test]   │
│                                                    │
│ ℹ️ Funkcia je neaktívna kým nie je nastavený       │
│   priečinok a API kľúč                             │
│                                                    │
│ (override) - hodnoty z local.settings.json        │
└────────────────────────────────────────────────────┘
```

### 2. Doklady page (new nav item)

```
┌─ Doklady ───────────────────────────── [🔄 Sync] ─┐
│ Filter: [Všetky ▾] [Nepridelené ▾]                │
├───────────────────────────────────────────────────┤
│ 📄 IMG_001.jpg    15.12.2024                      │
│    45.2 L  |  72.50 €  |  OMV Bratislava          │
│    ✅ Pridelené k jazde 15.12.                     │
├───────────────────────────────────────────────────┤
│ 📄 IMG_002.jpg    18.12.2024   ⚠️ Na kontrolu      │
│    ?? L   |  65.00 €  |  --                       │
│    [Upraviť] [Prideliť k jazde] [Zmazať]          │
├───────────────────────────────────────────────────┤
│ 📄 IMG_003.jpg    --           ❌ Chyba            │
│    Nepodarilo sa rozpoznať                        │
│    [Zadať manuálne] [Zmazať]                      │
└───────────────────────────────────────────────────┘
```

### 3. Floating indicator (in Trips view header)

```
┌─ Jazdy ──────────────────────── [📄 3 nepridelené] ┐
```

Click → quick-assign modal or jump to Doklady page.

### 4. Trip row integration

When entering fuel on a trip, show picker with smart filtering:

```
Tankovanie: [45.2] L  [72.50] €  [📄 Vybrať doklad ▾]
                                  ├─ IMG_001.jpg (15.12, 45.2L) ← date match
                                  ├─ IMG_002.jpg (18.12, ??L)
                                  └─ Zadať manuálne
```

**Smart filtering logic:**
- Show receipts within ±3 days of trip date first
- If station address available, boost receipts near trip origin/destination
- Sort by date proximity

### 5. Station profiles management (Settings)

```
┌─ Stanice (fine-tuning) ──────────────────────────┐
│ Slovnaft     [3 príklady] [Upraviť hinty]        │
│ OMV          [1 príklad]  [Upraviť hinty]        │
│ Shell        [0 príkladov] [Pridať príklad]      │
└──────────────────────────────────────────────────┘
```

---

## E2E Testing with Playwright

### Test structure

```
tests/
├── e2e/
│   ├── fixtures/
│   │   └── sample-receipts/     # Test images (various quality/formats)
│   ├── doklady.spec.ts          # Receipt management tests
│   ├── trip-assignment.spec.ts  # Assign receipt to trip
│   └── settings.spec.ts         # API key & folder config
├── playwright.config.ts
```

### Key test scenarios

1. **Settings configuration**
   - Configure receipts folder path
   - Configure API key
   - Test API key validation
   - Override file takes priority indicator

2. **Doklady page**
   - Sync button fetches new receipts
   - Receipt displays parsed data correctly
   - Filter by status works
   - Edit uncertain fields on NeedsReview receipt
   - Delete receipt

3. **Assignment flow**
   - Assign receipt to trip from Doklady page
   - Assign receipt from Trip row dropdown
   - Receipt status changes to Assigned
   - Trip fuel fields populated from receipt

4. **Floating indicator**
   - Shows correct unassigned count
   - Updates when receipts assigned
   - Click navigates to Doklady

5. **Error handling**
   - Failed parse shows NeedsReview with message
   - Manual entry fallback works

### Tauri + Playwright setup

- Mock Gemini API responses in tests (avoid real API calls)
- Use test fixtures with known receipt images
- Mock file system for folder scanning

---

## Implementation Phases

### Phase 1: Foundation
- [ ] Local settings override file (`local.settings.json`)
- [ ] Add Settings fields (API key, folder path) + UI
- [ ] Create Receipt model and DB migration
- [ ] Set up Playwright E2E testing infrastructure
- [ ] Add `.gitignore` entry for override file

### Phase 2: Core Parsing
- [ ] Gemini API client (Rust backend)
- [ ] Folder scanning logic
- [ ] Basic Doklady page (list receipts, show parsed data)
- [ ] Sync button
- [ ] E2E tests for sync & display

### Phase 3: Assignment Flow
- [ ] Assign receipt to trip from Doklady page
- [ ] Trip row dropdown "Vybrať doklad"
- [ ] Floating indicator in Trips view
- [ ] E2E tests for assignment flows

### Phase 4: Smart Matching
- [ ] Date proximity filtering in dropdown
- [ ] Station address cross-check with trip locations
- [ ] Confidence flags UI (highlight uncertain fields)
- [ ] Edit/correct parsed values

### Phase 5: Station Fine-tuning
- [ ] StationProfile model + migration
- [ ] Auto-detect station from receipt
- [ ] Store user corrections as examples
- [ ] Station hints management UI

---

## Files to Create/Modify

### Rust Backend
- `src-tauri/src/models.rs` - Add Receipt, ReceiptStatus, StationProfile
- `src-tauri/src/db.rs` - CRUD for receipts, station profiles
- `src-tauri/src/gemini.rs` - NEW: Gemini API client
- `src-tauri/src/receipts.rs` - NEW: Folder scanning, parsing orchestration
- `src-tauri/src/settings.rs` - NEW: Override file loading
- `src-tauri/src/lib.rs` - Register new Tauri commands
- `src-tauri/migrations/` - New migration for receipts table

### Frontend
- `src/routes/doklady/+page.svelte` - NEW: Receipts management page
- `src/lib/components/ReceiptCard.svelte` - NEW: Receipt display/edit
- `src/lib/components/ReceiptPicker.svelte` - NEW: Dropdown for trip row
- `src/lib/components/FloatingIndicator.svelte` - NEW: Unassigned count
- `src/routes/settings/+page.svelte` - Add receipts settings section
- `src/lib/components/TripRow.svelte` - Add receipt picker integration

### Testing
- `tests/e2e/` - NEW: Playwright test suite
- `playwright.config.ts` - NEW: Playwright configuration
- `tests/e2e/fixtures/sample-receipts/` - Test receipt images

### Config
- `.gitignore` - Add `local.settings.json` pattern
