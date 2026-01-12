# Design: Additional Costs Invoice Recognition

**Date:** 2026-01-12
**Status:** Revised (user decisions applied)

## User Decisions (2026-01-12)

1. **Single cost per trip** - One invoice auto-assigned per trip only
2. **No type categories** - User writes description manually in `other_costs_note`
3. **Same folder** - All receipts (fuel + other) in same folder
4. **Binary classification** - AI detects: fuel (has liters) vs other (no liters)

---

## Simplified Architecture

### Key Insight: `liters != null` → Fuel, `liters == null` → Other Cost

No new `ReceiptType` enum needed. The existing `liters` field determines the type:
- **Fuel receipt**: `liters` is set → existing auto-match flow
- **Other cost receipt**: `liters` is null → manual assignment to `other_costs_*` fields

### Folder Structure

Single folder, AI auto-classifies:

```
doklady/
├── IMG_001.jpg  → AI: liters=45.2, price=72€  → FUEL
├── IMG_002.jpg  → AI: liters=null, price=15€  → OTHER COST
├── IMG_003.jpg  → AI: liters=null, price=8€   → OTHER COST
└── IMG_004.jpg  → AI: liters=null, price=50€  → OTHER COST
```

---

## Data Model Changes

### Receipt Model Extension (Minimal)

```rust
// Add to Receipt struct - only 2 new fields needed:
pub struct Receipt {
    // ... existing fields (liters, total_price_eur, receipt_date, etc.) ...

    // NEW: For non-fuel receipts (description for other_costs_note)
    pub vendor_name: Option<String>,      // Shop/service provider name
    pub cost_description: Option<String>, // Brief description of expense
}
```

**Notes:**
- `total_price_eur` is reused for other costs amount
- `liters == None` indicates this is an "other cost" receipt
- No `ReceiptType` enum - simplicity wins

### Database Migration

```sql
-- Minimal migration: just 2 new columns
-- File: migrations/YYYY-MM-DD-HHMMSS-add_receipt_cost_fields/up.sql
ALTER TABLE receipts ADD COLUMN vendor_name TEXT;
ALTER TABLE receipts ADD COLUMN cost_description TEXT;

-- down.sql (for rollback)
-- SQLite doesn't support DROP COLUMN easily, so this would need table recreation
-- For now, leave columns in place (additive migration)
```

---

## Gemini Prompt Changes

### Updated Extraction Prompt

```
Analyze this Slovak receipt/invoice image.

This could be either a FUEL receipt or OTHER expense (car wash, parking, service, etc.).

Extract fields as JSON:
{
  "receipt_date": "YYYY-MM-DD" or null,
  "total_price_eur": number or null,

  // FUEL-SPECIFIC (only if this is a gas station receipt):
  "liters": number or null,  // null if NOT a fuel receipt
  "station_name": string or null,
  "station_address": string or null,

  // OTHER COSTS (for non-fuel receipts):
  "vendor_name": string or null,      // Company/shop name
  "cost_description": string or null, // Brief description (e.g., "Umytie auta", "Parkovanie 2h")

  "confidence": {
    "liters": "high" | "medium" | "low" | "not_applicable",
    "total_price": "high" | "medium" | "low",
    "date": "high" | "medium" | "low"
  },
  "raw_text": "full OCR text"
}

Rules:
- If you see "L", "litrov", fuel types (Natural 95, Diesel, benzín, nafta) → it's FUEL, extract liters
- If NO liters/fuel indicators → it's OTHER COST, set liters=null
- For amounts: Look for "€", "EUR", "Spolu", "Celkom", "Total"
- Date formats: DD.MM.YYYY or DD.MM.YY
- Return null if field cannot be determined
```

**Key change:** The prompt now explicitly handles both fuel and non-fuel receipts, with `liters=null` being the indicator for "other cost".

---

## UI Changes

### 1. Doklady Page Enhancement

Add filter for fuel vs other + visual distinction:

```
┌─ Doklady ──────────────────────── [🔄 Sync] ─┐
│ Filter: [Všetky ▾] [Typ: Všetky ▾] [Status ▾] │
├───────────────────────────────────────────────┤
│ ⛽ IMG_001.jpg    15.12.2024                   │
│    45.2 L  |  72.50 €  |  OMV Bratislava      │
│    ✅ Pridelené k jazde 15.12.                 │
├───────────────────────────────────────────────┤
│ 📄 IMG_002.jpg    18.12.2024                   │
│    15.00 €  |  AutoWash Express               │
│    [Prideliť k jazde]                         │
├───────────────────────────────────────────────┤
│ 📄 IMG_003.jpg    20.12.2024                   │
│    8.00 €  |  City Parking BA                 │
│    [Prideliť k jazde]                         │
└───────────────────────────────────────────────┘
```

**Icons (simple binary):**
- ⛽ Fuel (has liters)
- 📄 Other cost (no liters)

### 2. Assignment Flow

**For fuel receipts** (existing - unchanged):
- `liters != null` → auto-match by date + liters + price
- Assigns to `fuel_liters`, `fuel_cost_eur` fields

**For other costs** (new):
- `liters == null` → manual assignment by user
- Assigns to `other_costs_eur`, `other_costs_note` fields
- Note format: `{vendor_name}: {cost_description}` or user-edited text

### 3. Assignment Collision Handling

**If trip already has `other_costs_eur` set:**
- Block automatic assignment
- Show warning: "Jazda už má iné náklady"
- User can manually overwrite via trip edit

### 4. Floating Indicator Update

Combined count (simplified):
```
┌─ Jazdy ──────────────────────── [📄 6 nepridelených] ─┐
```

---

## Implementation Phases (Simplified)

### Phase 1: Backend (~3h)
- [ ] Add 2 new Receipt fields: `vendor_name`, `cost_description`
- [ ] Database migration (2 columns)
- [ ] Update Gemini prompt for dual recognition
- [ ] Update `ExtractedReceipt` struct parsing
- [ ] Update `assign_receipt_to_trip` for other costs
- [ ] Add tests

### Phase 2: Frontend (~2h)
- [ ] Update TypeScript types (2 new fields)
- [ ] Add binary filter (fuel/other) to Doklady page
- [ ] Visual distinction (⛽ vs 📄 icon)
- [ ] Assignment flow: if no liters → populate other_costs fields

### Phase 3: Polish (~1h)
- [ ] i18n for new strings
- [ ] Integration tests
- [ ] Update changelog

**Total: ~6 hours** (reduced from ~13h original estimate)

---

## Files to Modify

### Backend (Rust)
| File | Changes |
|------|---------|
| `models.rs` | Add `vendor_name`, `cost_description` to Receipt |
| `db.rs` | Update Receipt CRUD for 2 new fields |
| `gemini.rs` | Update prompt, parse new response fields |
| `commands.rs` | Update `assign_receipt_to_trip` for other costs |

### Frontend (Svelte)
| File | Changes |
|------|---------|
| `types.ts` | Add `vendorName`, `costDescription` to Receipt |
| `doklady/+page.svelte` | Binary filter, icon distinction |

### Database
| File | Changes |
|------|---------|
| `migrations/YYYY-MM-DD-HHMMSS-add_receipt_cost_fields/` | `up.sql` + `down.sql` |

---

## Testing Strategy

### Unit Tests (Rust)
- Gemini parsing with `liters=null` (other cost)
- Gemini parsing with `liters=45.2` (fuel)
- Assignment to `other_costs_*` fields

### Integration Tests
- Scan folder with mixed receipts
- Assign fuel receipt → fuel fields (existing)
- Assign other cost → other_costs fields (new)

---

## Resolved Questions

All open questions have been decided (see User Decisions section above).
