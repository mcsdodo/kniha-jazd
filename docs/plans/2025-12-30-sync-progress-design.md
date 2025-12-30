# Split Sync with Progress Feedback

**Date:** 2025-12-30
**Status:** Approved

## Problem

Current "Načítať" button does both folder scanning and OCR in one blocking operation. With many receipts, this takes minutes with no feedback - poor UX.

## Solution

Split into two explicit buttons with appropriate feedback.

## UI Design

### Button Layout

```
[Skenovať priečinok]  [Rozpoznať dáta (5)]
                                      ↑ badge showing pending count
```

### Button Specifications

| Button | Label | Action | Feedback |
|--------|-------|--------|----------|
| Scan | "Skenovať priečinok" | Scan folder, add new files to DB as "Pending" | Toast: "Nájdených X nových súborov" or "Žiadne nové súbory" |
| OCR | "Rozpoznať dáta" | Process pending receipts with Gemini OCR | Real-time progress: "Rozpoznávam 3/5..." |

### Badge Behavior

- Shows count of pending receipts: `(N)`
- Hidden when count is 0
- Updates after scan completes
- Clears when all processed

### Progress Display

During OCR processing, button text updates:
- "Rozpoznávam 1/12..."
- "Rozpoznávam 2/12..."
- ...
- "Rozpoznávam 12/12..."
- Returns to "Rozpoznať dáta"

## Backend Changes

### New Command: `scan_receipts`

```rust
#[tauri::command]
pub fn scan_receipts(app: AppHandle, db: State<Database>) -> Result<ScanResult, String>
```

Returns:
```rust
pub struct ScanResult {
    pub new_count: usize,
    pub warning: Option<String>,  // folder structure warning
}
```

- Synchronous (no async needed - just file system scan)
- No OCR calls
- Inserts new receipts with status "Pending"

### Existing: `process_pending_receipts`

Already has progress events - no changes needed to backend.

## Frontend Changes

### Doklady Page

1. Replace single "Načítať" button with two buttons
2. Add `pendingCount` state variable
3. Update `loadReceipts` to also fetch pending count
4. New `handleScan()` function for scan button
5. Existing `handleProcessPending()` already works

### i18n Additions (SK/EN)

```typescript
receipts: {
  scanFolder: 'Skenovať priečinok',
  scanning: 'Skenujem...',
  recognizeData: 'Rozpoznať dáta',
  recognizing: 'Rozpoznávam {current}/{total}...',
  foundNewReceipts: 'Nájdených {count} nových súborov',
  noNewReceipts: 'Žiadne nové súbory',
}
```

## User Flow

1. User opens Doklady page
2. Sees badge "(5)" on OCR button (5 pending from before)
3. Clicks "Skenovať priečinok"
4. Toast: "Nájdených 12 nových súborov"
5. Badge updates to "(17)"
6. Clicks "Rozpoznať dáta (17)"
7. Button shows: "Rozpoznávam 1/17...", "2/17...", etc.
8. Completes, badge clears
9. Receipts list refreshes with recognized data
