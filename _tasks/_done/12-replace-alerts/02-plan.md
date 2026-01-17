# Plan: Replace JavaScript alerts with styled modals

## Phase 1: Create Toast Notification System

### 1.1 Create toast store
**File:** `src/lib/stores/toast.ts`

```typescript
- ToastType: 'success' | 'error' | 'info'
- Toast interface: { id, message, type }
- Store with methods: success(), error(), info(), dismiss()
- Auto-dismiss with configurable duration
```

### 1.2 Create Toast component
**File:** `src/lib/components/Toast.svelte`

```
- Fixed position top-right
- Animated entry/exit (fly transition)
- Color-coded by type (green/red/blue)
- Click to dismiss
- Icon per type (✓, ✗, ℹ)
```

### 1.3 Add Toast to app layout
**File:** `src/routes/+layout.svelte`

```
- Import and render <Toast /> component once at root
```

## Phase 2: Create Confirm Modal Component

### 2.1 Create reusable ConfirmModal
**File:** `src/lib/components/ConfirmModal.svelte`

```svelte
Props:
- title: string
- message: string
- confirmText: string (default: "Potvrdiť")
- cancelText: string (default: "Zrušiť")
- danger: boolean (red confirm button)
- onConfirm: () => void
- onCancel: () => void

Style: Match existing backup delete modal
```

## Phase 3: Replace alerts in settings page

**File:** `src/routes/settings/+page.svelte`

| Location | Current | Replace with |
|----------|---------|--------------|
| Vehicle save success | (none) | toast.success() |
| Vehicle save error | alert() | toast.error() |
| Vehicle delete confirm | confirm() | ConfirmModal |
| Vehicle delete error | alert() | toast.error() |
| Set active error | alert() | toast.error() |
| Settings save success | alert() | toast.success() |
| Settings save error | alert() | toast.error() |
| Export PDF placeholder | alert() | toast.info() |
| Backup create success | alert() | toast.success() |
| Backup create error | alert() | toast.error() |
| Backup info error | alert() | toast.error() |
| Backup restore success | alert() | toast.success() |
| Backup restore error | alert() | toast.error() |
| Backup delete error | alert() | toast.error() |

## Phase 4: Replace alerts in TripGrid

**File:** `src/lib/components/TripGrid.svelte`

| Location | Current | Replace with |
|----------|---------|--------------|
| Create trip error | alert() | toast.error() |
| Update trip error | alert() | toast.error() |
| Delete trip error | alert() | toast.error() |
| Reorder trip error | alert() | toast.error() |

## Phase 5: Replace confirm in TripRow

**File:** `src/lib/components/TripRow.svelte`

| Location | Current | Replace with |
|----------|---------|--------------|
| Delete trip confirm | confirm() | ConfirmModal |

Need to lift confirmation state to parent or use a store-based confirm pattern.

## Phase 6: Replace alerts in other components

**File:** `src/lib/components/CompensationBanner.svelte`
- Add trip error → toast.error()

**File:** `src/routes/+page.svelte`
- Export error → toast.error()

## Implementation Notes

1. **Toast store pattern**: Allows any component to trigger toasts without prop drilling
2. **Confirm modal**: Use either inline modal state or a promise-based store pattern
3. **Existing modals**: Keep restore/delete backup modals as-is (already styled)
4. **Testing**: Manual verification of each replaced dialog

## Files to Create
- `src/lib/stores/toast.ts`
- `src/lib/components/Toast.svelte`
- `src/lib/components/ConfirmModal.svelte`

## Files to Modify
- `src/routes/+layout.svelte`
- `src/routes/+page.svelte`
- `src/routes/settings/+page.svelte`
- `src/lib/components/TripGrid.svelte`
- `src/lib/components/TripRow.svelte`
- `src/lib/components/CompensationBanner.svelte`
