# Design: Mocked LLM Receipt Testing

**Date:** 2026-01-20
**Updated:** 2026-01-21
**Related:** Task 42 (Receipt Mismatch Reasons) - ✅ Complete

## Two Mismatch Systems

The codebase has **two separate** mismatch detection systems:

| System | Location | Purpose | Format |
|--------|----------|---------|--------|
| **Receipt Verification** | `ReceiptVerification.mismatch_reason` | Show why receipt is unverified | `MismatchReason` enum (task 42 ✅) |
| **Trip Assignment** | `TripForAssignment.mismatch_reason` | Show why receipt can't attach to trip | `Option<String>` ("date", "liters", etc.) |

**Task 42 completed:** `MismatchReason` enum with `DateMismatch`, `LitersMismatch`, `PriceMismatch`, etc. for the verification UI.

**Task 41 focuses on:** Testing the trip assignment flow, which uses the string-based `mismatch_reason` in `TripForAssignment`.

## Architecture Analysis

### Current Flow (Production)
```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│  Receipt File   │ → │ scan_receipts │ → │ Pending Receipt │
│  (PDF/JPG)      │    │   command     │    │  in Database    │
└─────────────────┘    └──────────────┘    └────────┬────────┘
                                                     │
                                                     ▼
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│ Receipt Ready   │ ← │ process_receipt│ ← │  Gemini API     │
│ for Assignment  │    │ _with_gemini  │    │  (Real LLM)     │
└─────────────────┘    └──────────────┘    └─────────────────┘
```

### Proposed Test Flow
```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│  invoice.pdf    │ → │ scan_receipts │ → │ Pending Receipt │
│  (test fixture) │    │   command     │    │  in Test DB     │
└─────────────────┘    └──────────────┘    └────────┬────────┘
                                                     │
                                                     ▼
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│ Receipt Ready   │ ← │ process_receipt│ ← │  MOCK: Read     │
│ for Assignment  │    │ _with_gemini  │    │  invoice.json   │
└─────────────────┘    └──────────────┘    └─────────────────┘
```

## Implementation Approaches

### Option A: Environment Variable Mock (Recommended)

Add `KNIHA_JAZD_MOCK_GEMINI_DIR` env var. When set, `process_receipt_with_gemini()` reads mock JSON instead of calling Gemini API.

**Pros:**
- Minimal code changes
- Deterministic tests
- Uses real scanning logic
- Easy to add more test fixtures

**Cons:**
- Adds test-specific code path to production code

**Implementation:**
```rust
// In gemini.rs or receipts.rs
pub async fn extract_from_receipt(file_path: &Path) -> Result<ExtractedReceipt> {
    // Check for mock mode
    if let Ok(mock_dir) = std::env::var("KNIHA_JAZD_MOCK_GEMINI_DIR") {
        return load_mock_extraction(&mock_dir, file_path);
    }

    // Production: call Gemini API
    let client = GeminiClient::new(api_key)?;
    client.extract_from_image(file_path).await
}

fn load_mock_extraction(mock_dir: &str, file_path: &Path) -> Result<ExtractedReceipt> {
    // Look for {filename}.json in mock_dir
    let stem = file_path.file_stem().unwrap();
    let mock_file = Path::new(mock_dir).join(format!("{}.json", stem.to_string_lossy()));

    if mock_file.exists() {
        let json = std::fs::read_to_string(&mock_file)?;
        let mock: MockExtraction = serde_json::from_str(&json)?;
        return Ok(mock.into());
    }

    // No mock found - return "NeedsReview" result
    Ok(ExtractedReceipt::default())
}
```

### Option B: Direct Database Seeding

Add a `seed_receipt` test command that bypasses scanning and directly inserts parsed receipts.

**Pros:**
- No changes to production code
- Tests assignment logic directly

**Cons:**
- Doesn't test scanning or parsing
- Different code path than production
- Adds test-only Tauri command

### Option C: Test-Only HTTP Interceptor

Mock the HTTP client used by Gemini API during tests.

**Pros:**
- Tests real Gemini client code
- No env var pollution

**Cons:**
- Complex setup
- Requires HTTP mocking library
- Harder to maintain

## Recommended Approach: Hybrid A + B

1. **Option A for E2E tests**: Test full scan → mock parse → assign flow
2. **Option B for unit integration**: Test assignment/mismatch logic with seeded data

This gives coverage of both the scanning pipeline AND the assignment business logic.

## Test Data Structure

```
tests/integration/data/
├── invoices/                    # Receipt files to be scanned
│   └── invoice.pdf              # Real Slovnaft receipt (existing)
├── mocks/                       # Mock LLM responses
│   └── invoice.json             # Expected extraction (existing, needs move)
└── README.md                    # Document the mock format
```

Mock JSON format (aligns with `ExtractedReceipt`):
```json
{
    "liters": 63.68,
    "total_price_eur": 91.32,
    "receipt_date": "2026-01-20",
    "station_name": "Slovnaft, a.s.",
    "station_address": "Prístavna ulica, Bratislava",
    "confidence": {
        "liters": "High",
        "totalPrice": "High",
        "date": "High"
    }
}
```

## Tests to Enable

### Existing Skipped Tests
| Test | Current Status | With Mock |
|------|----------------|-----------|
| Display pre-seeded receipts | Skipped | Enable |
| Filter by status | Skipped | Enable |
| Assign receipt to trip | Skipped | Enable |
| Delete receipt | Skipped | Enable |

### Mismatch Detection Testing

Per **ADR-008**: All calculations in Rust backend only.

#### What Task 42 Already Implemented (✅ Complete)

**Receipt Verification UI** (`verify_receipts_with_data`):
- `MismatchReason` enum in `models.rs`
- `ReceiptVerification.mismatch_reason` field
- Frontend displays: "Dátum 20.1. - jazda je 19.1." etc.
- i18n strings for all mismatch types
- **Unit tests added** to `commands_tests.rs`

#### What Still Needs Testing (Task 41)

**Trip Assignment** (`get_trips_for_receipt_assignment`):
- `TripForAssignment.mismatch_reason: Option<String>`
- String values: `"date"`, `"liters"`, `"price"`, `"date_and_liters"`, etc.
- Frontend assignment modal should display these

**Testing Strategy:**
| Test Type | Count | Location | Purpose |
|-----------|-------|----------|---------|
| **Rust unit tests** | 9 | `commands_tests.rs` | Trip assignment mismatch logic |
| **Integration test** | 1 | `receipts.spec.ts` | Verify assignment IPC + UI |

**Note:** Verification mismatch tests already exist from task 42. Assignment mismatch tests may partially exist - check before adding.

The trip assignment matching logic in `commands.rs:2256-2265`:

```rust
let mismatch = match (date_match, liters_match, price_match) {
    (false, false, false) => "all",
    (false, false, true) => "date_and_liters",
    (false, true, false) => "date_and_price",
    (false, true, true) => "date",
    (true, false, false) => "liters_and_price",
    (true, false, true) => "liters",
    (true, true, false) => "price",
    (true, true, true) => unreachable!(),
};
```

| # | Mismatch Reason | Status | Notes |
|---|-----------------|--------|-------|
| 1-7 | All string combinations | Check if exists | May overlap with task 42 tests |
| 8 | `"matches"` | Check if exists | `can_attach: true` |
| 9 | `"empty"` | Check if exists | Trip has no fuel |
| 10 | E2E integration | **Needed** | Assignment UI displays reason |

**Tolerance:** ±0.01 for liters and price (floating point comparison)

## Risk Assessment

- **Low risk**: Mock approach is isolated to test mode
- **Backward compatible**: No changes to production behavior
- **CI friendly**: Deterministic tests with no external API calls
