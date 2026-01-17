# Task: Replace JavaScript alerts with styled modals

## Problem
The app uses native JavaScript `alert()` and `confirm()` dialogs which look inconsistent and unprofessional. The backup delete confirmation already uses a nice styled modal - all other dialogs should match this pattern.

## Current State
Found 19 `alert()` and 2 `confirm()` usages across 4 files:

### src/routes/settings/+page.svelte (13 alerts, 1 confirm)
- Vehicle save error
- Vehicle delete confirm + error
- Set active vehicle error
- Settings save success + error
- Export PDF placeholder
- Backup create success + error
- Backup info load error
- Backup restore success + error
- Backup delete error

### src/lib/components/TripGrid.svelte (4 alerts)
- Create trip error
- Update trip error
- Delete trip error
- Reorder trip error (2 places)

### src/lib/components/TripRow.svelte (1 confirm)
- Delete trip confirmation

### src/lib/components/CompensationBanner.svelte (1 alert)
- Add compensation trip error

### src/routes/+page.svelte (1 alert)
- Export error

## Goal
Replace all `alert()` and `confirm()` calls with:
1. **Toast notifications** - for success/error messages (non-blocking, auto-dismiss)
2. **Confirm modals** - for destructive action confirmations (like existing delete backup modal)

## Acceptance Criteria
- [ ] No native `alert()` or `confirm()` calls in codebase
- [ ] Toast notifications appear in top-right corner
- [ ] Success toasts auto-dismiss after 4 seconds
- [ ] Error toasts auto-dismiss after 6 seconds (longer to read)
- [ ] Confirm modals match existing backup delete modal style
- [ ] All UI remains in Slovak language
