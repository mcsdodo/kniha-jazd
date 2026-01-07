**Date:** 2026-01-07
**Subject:** Receipt filtering by vehicle - show unassigned + current car's receipts
**Status:** Planning

## Goal

When switching vehicles, filter receipts to show only:
1. Unassigned receipts (`vehicle_id = NULL`) — could belong to any car
2. Current car's receipts (`vehicle_id = selected_vehicle`) — already assigned

Additionally, fix UX issue where users can deselect to "no vehicle" when vehicles exist.

## Requirements

### Main Feature: Receipt Filtering

- **Scope:** Filter receipts everywhere based on context (Doklady page, ReceiptIndicator badge)
- **Filter Logic:** `WHERE vehicle_id IS NULL OR vehicle_id = ?`
- **Tab Interaction (Doklady page):** Layered filtering
  - "All" → unassigned + this car's receipts
  - "Unassigned" → only unassigned (no vehicle_id)
  - "Needs Review" → needs review AND (unassigned OR this car's)

### UX Fix: Vehicle Selector

- Remove "no vehicle" option when vehicles exist
- Auto-select first vehicle on app load if none persisted
- Handle edge case: if persisted active vehicle was deleted, auto-select first available

## Technical Notes

### Architecture Decision

**Backend Filtering (Option A)** chosen over frontend filtering:
- Aligns with ADR-008 (all business logic in Rust backend)
- Single source of truth for filtering logic
- Less data transferred over IPC
- Testable in Rust

### Components Affected

| Component | Change |
|-----------|--------|
| `src-tauri/src/commands.rs` | New `get_receipts_for_vehicle` command |
| `src-tauri/src/db.rs` | New query with vehicle filtering |
| `src-tauri/src/receipts_tests.rs` | 3 new tests |
| `src/lib/api.ts` | New `getReceiptsForVehicle` wrapper |
| `src/routes/doklady/+page.svelte` | Use new API |
| `src/lib/components/ReceiptIndicator.svelte` | Use vehicle-filtered count |
| `src/routes/+layout.svelte` | Remove empty option, auto-select logic |

### Existing Behavior (for reference)

- `get_receipts(year)` returns ALL receipts regardless of vehicle
- `verify_receipts(vehicle_id, year)` IS vehicle-aware (matches against trips)
- Assignment sets `vehicle_id` on receipt
