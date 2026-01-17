# Implementation Plan: Live Preview Fuel Calculations

**Date:** 2026-01-01
**Status:** Planning

---

## Phase 1: Backend - New Tauri Command

### 1.1 Add PreviewResult struct
- [ ] Add to `src-tauri/src/models.rs` or `commands.rs`:
  ```rust
  #[derive(Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct PreviewResult {
      pub zostatok: f64,
      pub consumption_rate: f64,
      pub margin_percent: f64,
      pub is_over_limit: bool,
      pub is_estimated_rate: bool,
  }
  ```

### 1.2 Implement preview_trip_calculation command
- [ ] Add to `src-tauri/src/commands.rs`:
  ```rust
  #[tauri::command]
  pub fn preview_trip_calculation(
      vehicle_id: String,
      year: i32,
      distance_km: i32,
      fuel_liters: Option<f64>,
      full_tank: bool,
      insert_at_sort_order: Option<i32>,
      editing_trip_id: Option<String>,
      state: State<'_, AppState>,
  ) -> Result<PreviewResult, String>
  ```
- [ ] Logic: Clone trips, insert/modify virtual trip, run calculation, extract result

### 1.3 Register command in main.rs
- [ ] Add `preview_trip_calculation` to `invoke_handler`

### 1.4 Tests
- [ ] Test: New trip preview calculates zostatok correctly
- [ ] Test: Preview with fuel recalculates consumption rate
- [ ] Test: Preview shows margin warning over 20%
- [ ] Test: Insert in middle affects subsequent fill-up rate

---

## Phase 2: Frontend - API Layer

### 2.1 Add TypeScript types
- [ ] Add to `src/lib/types.ts`:
  ```typescript
  export interface PreviewResult {
      zostatok: number;
      consumption_rate: number;
      margin_percent: number;
      is_over_limit: boolean;
      is_estimated_rate: boolean;
  }
  ```

### 2.2 Add API function
- [ ] Add to `src/lib/api.ts`:
  ```typescript
  export async function previewTripCalculation(
      vehicleId: string,
      year: number,
      distanceKm: number,
      fuelLiters: number | null,
      fullTank: boolean,
      insertAtSortOrder: number | null,
      editingTripId: string | null
  ): Promise<PreviewResult>
  ```

---

## Phase 3: Frontend - TripRow Component

### 3.1 Add preview props
- [ ] New props in `TripRow.svelte`:
  ```typescript
  export let previewData: PreviewResult | null = null;
  export let onPreviewRequest: (km: number, fuel: number | null, fullTank: boolean) => void = () => {};
  ```

### 3.2 Trigger preview on input changes
- [ ] Update `handleKmChange`: Call `onPreviewRequest(km, fuel, fullTank)`
- [ ] Update fuel_liters input: Call `onPreviewRequest` on change
- [ ] Update full_tank checkbox: Call `onPreviewRequest` on change

### 3.3 Display preview values
- [ ] Replace static consumption rate cell with preview-aware version:
  ```svelte
  <td class="number calculated" class:preview={previewData} class:over-limit={previewData?.is_over_limit}>
      {#if previewData}
          ~{previewData.consumption_rate.toFixed(2)}
          {#if previewData.margin_percent > 0}
              <span class:over-limit={previewData.is_over_limit}>
                  (+{previewData.margin_percent.toFixed(0)}%)
              </span>
          {/if}
      {:else}
          {consumptionRate.toFixed(2)}
      {/if}
  </td>
  ```
- [ ] Replace static zostatok cell with preview-aware version

### 3.4 Add CSS for preview state
- [ ] `.preview { opacity: 0.85; }`
- [ ] `.over-limit { color: #e74c3c; font-weight: 500; }`
- [ ] `tr.editing.consumption-warning` for background color

---

## Phase 4: Frontend - TripGrid Orchestration

### 4.1 Add preview state
- [ ] Add state variables:
  ```typescript
  let previewData: PreviewResult | null = null;
  let previewingForId: string | null = null;
  ```

### 4.2 Implement preview handler
- [ ] Add `handlePreviewRequest` function:
  - Calls `previewTripCalculation` API
  - Stores result in `previewData`
  - Tracks which row is previewing

### 4.3 Pass preview to TripRow
- [ ] Pass `previewData` prop (only to the row being edited)
- [ ] Pass `onPreviewRequest` callback

### 4.4 Clear preview on save/cancel
- [ ] Reset `previewData = null` in `handleEditEnd` and `handleSaveNew`

---

## Phase 5: Edge Cases

### 5.1 Handle empty/zero KM
- [ ] When KM is 0 or empty, show preview with no fuel consumed

### 5.2 Handle first trip ever
- [ ] Use tank_size as starting zostatok, tp_consumption as rate

### 5.3 Handle no full_tank fuel entries
- [ ] Mark rate as estimated (`is_estimated_rate: true`)

---

## Phase 6: Final

### 6.1 Testing
- [ ] Manual test: Edit existing trip, verify live preview
- [ ] Manual test: Add new trip at top, verify live preview
- [ ] Manual test: Insert trip in middle, verify rate recalculation
- [ ] Manual test: Margin warning appears when over 20%
- [ ] `cargo test` - all pass

### 6.2 Documentation
- [ ] Run `/changelog` to update [Unreleased]
- [ ] Consider `/decision` for ADR if calculation logic changes significantly

---

## File Changes Summary

| File | Changes |
|------|---------|
| `src-tauri/src/models.rs` | Add `PreviewResult` struct |
| `src-tauri/src/commands.rs` | Add `preview_trip_calculation` command |
| `src-tauri/src/main.rs` | Register new command |
| `src/lib/types.ts` | Add `PreviewResult` interface |
| `src/lib/api.ts` | Add `previewTripCalculation()` function |
| `src/lib/components/TripRow.svelte` | Preview display, input handlers |
| `src/lib/components/TripGrid.svelte` | Preview orchestration |
| `CHANGELOG.md` | Feature entry |
