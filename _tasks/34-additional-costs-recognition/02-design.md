# Design: Additional Costs Invoice Recognition

**Date:** 2026-01-12
**Status:** Draft

## Decision: Architecture Approach

### Recommended: Extend Receipt Model (Option A)

Extend the existing `Receipt` model with a `receipt_type` field rather than creating a parallel system.

**Rationale:**
- **DRY** - Reuse existing folder scanning, Gemini integration, UI components
- **Simpler** - One table, one workflow, one codebase path
- **Flexible** - Same infrastructure handles both types
- **Consistent UX** - Users learn one workflow

### Folder Structure Decision

**Recommended: Single folder with AI classification**

```
doklady/
├── IMG_001.jpg  → AI detects: fuel receipt (45.2L, 72€)
├── IMG_002.jpg  → AI detects: car wash (15€)
├── IMG_003.jpg  → AI detects: parking (8€)
└── IMG_004.jpg  → AI detects: highway toll (50€)
```

**Rationale:**
- Users don't have to manually sort invoices
- AI is already analyzing each image - adding classification is trivial
- Simpler folder management

---

## Data Model Changes

### Receipt Model Extension

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReceiptType {
    Fuel,           // Existing behavior
    CarWash,        // Umytie auta
    Parking,        // Parkovanie
    Toll,           // Diaľničná známka, mýto
    Service,        // Servis, opravy
    Other,          // Iné náklady
}

impl Default for ReceiptType {
    fn default() -> Self {
        Self::Fuel
    }
}

// Add to Receipt struct:
pub struct Receipt {
    // ... existing fields ...

    // NEW: Type classification
    pub receipt_type: ReceiptType,

    // NEW: For non-fuel receipts
    pub cost_amount_eur: Option<f64>,    // Amount for other costs
    pub cost_category: Option<String>,   // Detected category text
    pub cost_description: Option<String>, // Parsed description/note
    pub vendor_name: Option<String>,     // Shop/service provider name

    // Existing fuel fields (None for non-fuel)
    pub liters: Option<f64>,
    pub total_price_eur: Option<f64>,    // Also used for fuel cost
    pub receipt_date: Option<NaiveDate>,
    pub station_name: Option<String>,
    pub station_address: Option<String>,
    // ...
}
```

### Database Migration

```sql
-- Add new columns to receipts table
ALTER TABLE receipts ADD COLUMN receipt_type TEXT NOT NULL DEFAULT 'Fuel';
ALTER TABLE receipts ADD COLUMN cost_amount_eur REAL;
ALTER TABLE receipts ADD COLUMN cost_category TEXT;
ALTER TABLE receipts ADD COLUMN cost_description TEXT;
ALTER TABLE receipts ADD COLUMN vendor_name TEXT;
```

---

## Gemini Prompt Changes

### Updated Extraction Prompt

```
Analyze this Slovak receipt/invoice image.

First, classify the receipt type:
- "fuel" - Gas station receipt (benzín, nafta, LPG)
- "car_wash" - Car wash (umývanie, autoumyváreň)
- "parking" - Parking receipt (parkovanie)
- "toll" - Highway toll/sticker (diaľničná známka, mýto)
- "service" - Car service/repair (servis, oprava, STK)
- "other" - Other vehicle-related expense

Then extract fields as JSON:
{
  "receipt_type": "fuel" | "car_wash" | "parking" | "toll" | "service" | "other",
  "receipt_date": "YYYY-MM-DD" or null,

  // For fuel receipts:
  "liters": number or null,
  "total_price_eur": number or null,
  "station_name": string or null,
  "station_address": string or null,

  // For non-fuel receipts:
  "cost_amount_eur": number or null,
  "cost_category": string or null,  // e.g., "Umytie auta", "Parkovanie"
  "cost_description": string or null,  // Any relevant details
  "vendor_name": string or null,  // Company/shop name

  "confidence": {
    "type": "high" | "medium" | "low",
    "amount": "high" | "medium" | "low",
    "date": "high" | "medium" | "low"
  },
  "raw_text": "full OCR text"
}

Rules:
- For fuel: Look for "L", "litrov", fuel type (Natural 95, Diesel)
- For amounts: Look for "€", "EUR", "Spolu", "Celkom", "Total"
- Date formats: DD.MM.YYYY or DD.MM.YY
- Return null if field cannot be determined
```

---

## UI Changes

### 1. Doklady Page Enhancement

Add filter by type + visual distinction:

```
┌─ Doklady ──────────────────────── [🔄 Sync] ─┐
│ Filter: [Všetky ▾] [Typ: Všetky ▾] [Status ▾] │
├───────────────────────────────────────────────┤
│ ⛽ IMG_001.jpg    15.12.2024    TANKOVANIE    │
│    45.2 L  |  72.50 €  |  OMV Bratislava      │
│    ✅ Pridelené k jazde 15.12.                 │
├───────────────────────────────────────────────┤
│ 🚿 IMG_002.jpg    18.12.2024    UMYTIE        │
│    15.00 €  |  AutoWash Express               │
│    [Prideliť k jazde]                         │
├───────────────────────────────────────────────┤
│ 🅿️ IMG_003.jpg    20.12.2024    PARKOVANIE    │
│    8.00 €  |  City Parking BA                 │
│    [Prideliť k jazde]                         │
├───────────────────────────────────────────────┤
│ 🛣️ IMG_004.jpg    01.01.2025    DIAĽNICA      │
│    50.00 €  |  eZnamka.sk                     │
│    ⚠️ Neurčený dátum jazdy                     │
└───────────────────────────────────────────────┘
```

**Icons by type:**
- ⛽ Fuel (tankovanie)
- 🚿 Car wash (umytie)
- 🅿️ Parking (parkovanie)
- 🛣️ Toll (diaľnica)
- 🔧 Service (servis)
- 📄 Other (iné)

### 2. Assignment Flow

**For fuel receipts** (existing):
- Assigns to `fuel_liters`, `fuel_cost_eur` fields

**For other costs** (new):
- Assigns to `other_costs_eur`, `other_costs_note` fields
- Note format: `{category}: {description}` (e.g., "Umytie: AutoWash Express")

### 3. Trip Row Integration

Extend trip row to show assigned other costs:

```
│ Dátum   │ Trasa          │ km  │ ODO   │ Účel      │ Tankovanie │ Iné náklady │
├─────────┼────────────────┼─────┼───────┼───────────┼────────────┼─────────────┤
│ 15.12.  │ BA → KE        │ 400 │ 55000 │ služobná  │ 45L / 72€  │ 🚿 15€      │
│         │                │     │       │           │ 📄         │ 📄          │
```

Small document icon (📄) indicates assigned receipt - click to view.

### 4. Floating Indicator Update

Show count by type or combined:

```
┌─ Jazdy ──────────── [⛽ 2] [🚿 1] [🅿️ 3] nepridelené ─┐
```

Or simplified:
```
┌─ Jazdy ──────────────────────── [📄 6 nepridelených] ─┐
```

---

## Implementation Phases

### Phase 1: Data Model & Backend
- [ ] Add `ReceiptType` enum to models
- [ ] Database migration for new columns
- [ ] Update Gemini prompt for classification
- [ ] Update receipt parsing to handle both types
- [ ] Add tests for new receipt types

### Phase 2: UI Updates
- [ ] Add type filter to Doklady page
- [ ] Visual distinction (icons, colors) by type
- [ ] Update ReceiptCard component for non-fuel display
- [ ] Assignment flow for other costs → trip.other_costs fields

### Phase 3: Trip Integration
- [ ] Show assigned other costs in trip row
- [ ] Other costs picker in trip edit mode
- [ ] Update floating indicator

### Phase 4: Polish
- [ ] i18n for new strings
- [ ] E2E tests for other costs flow
- [ ] Update changelog

---

## Files to Modify

### Backend (Rust)
| File | Changes |
|------|---------|
| `models.rs` | Add `ReceiptType` enum, new Receipt fields |
| `db.rs` | Update Receipt CRUD for new fields |
| `gemini.rs` | Update prompt, parse new response fields |
| `receipts.rs` | Handle both receipt types in scanning |
| `commands.rs` | Update assignment command for other costs |

### Frontend (Svelte)
| File | Changes |
|------|---------|
| `types.ts` | Add `ReceiptType`, new Receipt fields |
| `doklady/+page.svelte` | Type filter, visual distinction |
| `ReceiptCard.svelte` | Display both fuel and other costs |
| `TripRow.svelte` | Show other costs, picker integration |

### Database
| File | Changes |
|------|---------|
| `migrations/` | New migration for receipt_type and cost fields |

---

## Testing Strategy

### Unit Tests (Rust)
- Receipt type parsing from Gemini response
- Receipt type enum serialization
- Assignment to other_costs fields

### Integration Tests
- Scan folder with mixed receipt types
- Assign fuel receipt → fuel fields
- Assign other cost → other_costs fields
- Filter by receipt type

### E2E Tests
- Full flow: scan → view → assign other cost → verify on trip

---

## Open Items for User Decision

1. **Should we support multiple other costs per trip?**
   - Current model: single `other_costs_eur` + `other_costs_note` per trip
   - If multiple needed: requires Trip model change (array of costs)

2. **Category list - predefined or extensible?**
   - Predefined: Easier filtering, consistent
   - Extensible: More flexible, user can add custom

3. **Same folder or configurable?**
   - Option: Add `other_costs_folder_path` to settings
   - Or: Use same `receipts_folder_path` (recommended)
