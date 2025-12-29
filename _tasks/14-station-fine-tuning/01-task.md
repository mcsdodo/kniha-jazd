# Task: Station Fine-tuning for Receipt Parsing

**Date:** 2025-12-29
**Subject:** Per-station learning to improve receipt OCR accuracy
**Status:** Deferred
**Source:** `_tasks/13-invoice-scanning/03-plan.md` - Phase 5

## Summary

Add per-station fine-tuning to improve Gemini receipt parsing accuracy over time. Each gas station (Slovnaft, OMV, Shell, etc.) has unique receipt formats - learn patterns from user corrections.

## Why Deferred

From the original plan:
> "This phase is optional enhancement. Implement only if Phase 1-4 proves station-specific parsing issues in practice."

Phase 1-4 (core receipt scanning) is now complete. Evaluate this feature after real-world usage identifies recurring parsing issues with specific stations.

## User Requirements

- Auto-detect station name from parsed receipt
- Store station-specific prompt hints (e.g., "Liters shown as 'Množstvo:'")
- Capture user corrections as few-shot examples
- Use examples to improve future parsing of same station's receipts
- Station profiles management UI in Settings

## Data Model

### StationProfile Entity

```rust
pub struct StationProfile {
    pub id: Uuid,
    pub name: String,                    // "Slovnaft", "OMV", "Shell"
    pub detection_keywords: Vec<String>, // ["SLOVNAFT", "MOL Group"]
    pub prompt_hints: Option<String>,    // "Liters shown as 'Množstvo:'"
    pub example_extractions: Vec<ExampleExtraction>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ExampleExtraction {
    pub raw_text_snippet: String,
    pub extracted_liters: Option<f64>,
    pub extracted_price: Option<f64>,
}
```

## Implementation Outline

### Task 1: StationProfile Model
- Add `StationProfile` and `ExampleExtraction` structs to `models.rs`
- DB migration for `station_profiles` table
- CRUD operations in `db.rs`

### Task 2: Auto-detect Station
- Parse station name from Gemini response (already returned)
- Match against known `StationProfile` by `detection_keywords`
- Append `prompt_hints` to Gemini request when matched

### Task 3: User Corrections Capture
- When user edits parsed values (liters, price), prompt "Save as example?"
- Store correction in `StationProfile.example_extractions`
- Use examples as few-shot prompts for future parsing

### Task 4: Station Management UI
- Add "Stanice" section to Settings page
- List profiles with example counts
- Edit hints, view/delete examples

## UI Mockup

```
┌─ Stanice (fine-tuning) ──────────────────────────┐
│ Slovnaft     [3 príklady] [Upraviť hinty]        │
│ OMV          [1 príklad]  [Upraviť hinty]        │
│ Shell        [0 príkladov] [Pridať príklad]      │
└──────────────────────────────────────────────────┘
```

## When to Implement

Consider implementing when:
1. Users report consistent parsing errors for specific stations
2. The same station fails parsing multiple times
3. Manual corrections become repetitive

## References

- Original plan: `_tasks/13-invoice-scanning/03-plan.md` (Phase 5)
- Design doc: `_tasks/13-invoice-scanning/02-design.md` (StationProfile section)
- Gemini client: `src-tauri/src/gemini.rs`
- Receipt processing: `src-tauri/src/receipts.rs`
