# Task: Additional Costs Invoice Recognition

**Date:** 2026-01-12
**Subject:** Recognition and assignment of additional cost invoices (like fillup receipts)
**Status:** Planning

## Background

The app currently supports:
- **Fuel receipts** - scanned from a folder, parsed with Gemini AI, assigned to trips (fuel fields)
- **Other costs on trips** - manual entry of `other_costs_eur` and `other_costs_note` fields

The user wants the same scanning/recognition/assignment workflow for additional cost invoices:
- Car wash receipts
- Parking receipts
- Toll/highway sticker receipts
- Service/maintenance invoices
- Any other vehicle-related expenses

## User Story

> As a user, I want to put additional cost invoices in a folder and have them automatically recognized and assignable to trips, just like fuel receipts work today.

## Requirements

### Functional
1. **Folder-based input** - User puts invoices in designated folder (same or separate from fuel receipts)
2. **Automatic recognition** - App scans and parses invoices using Gemini AI
3. **Extract key fields**:
   - Amount (EUR)
   - Date
   - Type/category (car wash, parking, toll, service, etc.)
   - Description/note
   - Vendor name (optional)
4. **Assignment to trips** - Link invoice to a trip (populates `other_costs_eur` and `other_costs_note`)
5. **Management UI** - View, edit, assign, delete invoices (similar to Doklady page)

### Non-functional
1. **Reuse existing infrastructure** - Gemini API client, folder scanning, Receipt model patterns
2. **Consistent UX** - Same workflow as fuel receipts
3. **Distinguish from fuel** - Clear visual distinction between fuel receipts and other cost invoices

## Open Questions

1. **Same folder or separate?**
   - Option A: Same folder, AI distinguishes fuel from other
   - Option B: Separate folder for other costs
   - Option C: Subfolder structure (receipts/fuel/, receipts/other/)

2. **Same table or new table?**
   - Option A: Extend Receipt model with `receipt_type` field
   - Option B: New `CostInvoice` table (parallel to Receipt)

3. **Multiple costs per trip?**
   - Current: Trip has single `other_costs_eur` + `other_costs_note`
   - Question: Should we support multiple other costs per trip?

4. **Categories/types predefined or free-form?**
   - Predefined: car_wash, parking, toll, service, other
   - Free-form: User-entered text

## Related

- **Existing implementation**: `_tasks/13-invoice-scanning/` (fuel receipts)
- **Tech debt**: None currently
- **Code files**:
  - `src-tauri/src/receipts.rs` - Folder scanning
  - `src-tauri/src/gemini.rs` - AI parsing
  - `src-tauri/src/models.rs` - Receipt model
  - `src/routes/doklady/+page.svelte` - Receipts UI
