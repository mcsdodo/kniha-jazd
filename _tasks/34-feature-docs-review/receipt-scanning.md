# Review: receipt-scanning.md

## Convention Compliance

**Overall Assessment:** Partial compliance. The document follows the template structure well but violates the key rule about code embedding.

**What it does well:**
- Clear user flow section with numbered steps
- Good use of ASCII diagrams (Receipt Status Lifecycle on lines 126-150)
- Tables are well-formatted and informative
- Design Decisions section explains the "why" effectively
- Key Files table is comprehensive

**Convention violations:**
- Embeds full struct definition instead of using line references
- Embeds function logic instead of pointing to source

## Issues Found

### Issue 1: Embedded `ExtractedReceipt` struct (Lines 84-98)

**Problem:** Full Rust struct definition is embedded in the documentation.

```rust
pub struct ExtractedReceipt {
    pub liters: Option<f64>,           // Fuel only
    pub station_name: Option<String>,   // Fuel only
    ...
}
```

**Risk:** Documentation drift. When struct fields change in code, this copy becomes stale.

**Location in source:** `gemini.rs:L14`

### Issue 2: Embedded `check_receipt_trip_compatibility` function (Lines 191-196)

**Problem:** Function signature and inline comments embedded.

```rust
fn check_receipt_trip_compatibility(receipt: &Receipt, trip: &Trip) -> CompatibilityResult {
    // Trip has no fuel → can attach (empty)
    // Receipt has fuel data → must match trip's fuel exactly (matches/differs)
    // Receipt is other cost → can attach to any trip (empty)
}
```

**Risk:** Logic comments may diverge from actual implementation.

**Location in source:** `commands.rs:L2609`

### Issue 3: Key Files table lacks line references

**Minor issue:** The Key Files table lists files but doesn't specify which line numbers contain the key logic. Template suggests `commands.rs:L###` format.

## Recommendations

### Replace embedded struct with reference

**Before (lines 84-98):**
```markdown
**Response Schema Fields**:
```rust
pub struct ExtractedReceipt {
    pub liters: Option<f64>,           // Fuel only
    ...
}
```

**After:**
```markdown
**Response Schema:** See `ExtractedReceipt` struct in `gemini.rs:L14` for full field definitions.

Key fields:
- `liters`, `station_name`, `station_address` — Fuel receipts only
- `vendor_name`, `cost_description` — Other costs only
- `original_amount`, `original_currency` — Raw OCR values (EUR/CZK/HUF/PLN)
- `confidence` — Per-field extraction confidence levels
```

### Replace embedded function with reference

**Before (lines 191-196):**
```markdown
**Assignment Compatibility Check**:
```rust
fn check_receipt_trip_compatibility(receipt: &Receipt, trip: &Trip) -> CompatibilityResult {
    // Trip has no fuel → can attach (empty)
    ...
}
```

**After:**
```markdown
**Assignment Compatibility Check** (`commands.rs:L2609`):
- Trip has no fuel data → receipt can be attached
- Fuel receipt → must match trip's fuel fields exactly
- Other-cost receipt → can attach to any trip
```

### Enhance Key Files table with line numbers (optional)

Current table is acceptable, but could be improved:

| File | Purpose | Key Location |
|------|---------|--------------|
| `gemini.rs` | Gemini API client, extraction prompt | `ExtractedReceipt` at L14, `process_receipt_with_gemini` at L## |
| `commands.rs` | Receipt commands | `check_receipt_trip_compatibility` at L2609 |

## Summary

| Check | Status |
|-------|--------|
| Follows template structure | Pass |
| User flow is clear | Pass |
| No embedded code | **FAIL** (2 instances) |
| Uses line references | Partial (tables but not inline) |
| Math formulas (stable) | N/A |
| ASCII diagrams | Pass |
| Design rationale explained | Pass |

**Priority:** Medium. The embedded code is a maintenance burden but the overall document quality is good.
