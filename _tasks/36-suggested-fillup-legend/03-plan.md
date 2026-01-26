# Implementation Plan: Suggested Fillup Legend

## Step 1: Backend - Add SuggestedFillup to TripData

**Files:** `src-tauri/src/commands.rs`

- [ ] Add `SuggestedFillup` struct with `liters` and `consumption_rate`
- [ ] Add `suggested_fillup: Option<SuggestedFillup>` to `TripData`
- [ ] In `get_trip_grid_data`, calculate suggestions for trips in open period
- [ ] Skip calculation for BEV vehicles (no fuel)

**Tests:** `src-tauri/src/commands_tests.rs`
- [ ] Test open period trips get suggestions
- [ ] Test closed period trips get None
- [ ] Test consumption rate calculation
- [ ] Test BEV vehicles skipped

## Step 2: Frontend Types

**Files:** `src/lib/types.ts`

- [ ] Add `SuggestedFillup` interface
- [ ] Add `suggestedFillup?: SuggestedFillup` to `TripData`

## Step 3: i18n Translations

**Files:** `src/lib/i18n/sk/index.ts`, `src/lib/i18n/en/index.ts`

- [ ] Add `suggestedFillup` translation with `{liters}` and `{rate}` params

## Step 4: Legend Display

**Files:** `src/lib/components/TripGrid.svelte`

- [ ] Derive `suggestedFillup` from last trip with suggestion
- [ ] Add legend item with green styling
- [ ] Use lightbulb icon (💡) as indicator

## Step 5: Magic Button Simplification

**Files:** `src/lib/components/TripRow.svelte`

- [ ] Remove `calculateMagicFillLiters` import and async call
- [ ] Use `trip.suggestedFillup.liters` directly
- [ ] Keep magic button hidden if no suggestion available

## Step 6: Cleanup

**Files:** `src/lib/api.ts`, `src-tauri/src/commands.rs`

- [ ] Remove `calculateMagicFillLiters` API function (no longer called from frontend)
- [ ] Keep backend function for potential future use OR remove entirely

## Step 7: Integration Test

**Files:** `tests/integration/specs/tier1/` or `tier2/`

- [ ] Test legend appears with open period
- [ ] Test legend hidden with closed period
- [ ] Test magic button uses pre-calculated value

## Verification

```bash
npm run test:backend        # Rust unit tests
npm run test:integration:tier1  # Integration tests
```
