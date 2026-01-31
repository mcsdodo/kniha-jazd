# Task: Add Fuel Consumed Column to TripGrid

## Summary

Add a new calculated column "Spotr. (L)" to the TripGrid table showing fuel consumed per trip in liters.

## Requirements

- **Value**: Per-trip fuel consumed = `km × rate / 100`
- **Position**: Between "Cena €" and "l/100km" columns
- **Width**: 4%
- **Header**: "Spotr. (L)" (Slovak) / "Cons. (L)" (English)
- **Rate source**: Period consumption rate if fill-up period is closed (full tank), otherwise TP rate from vehicle documents
- **First record row**: Shows "0.0"
- **Decimal places**: 1 (e.g., "9.0")
- **Styling**: Same as other calculated columns (right-aligned, italic via `.number.calculated`)

## Acceptance Criteria

1. New column appears in correct position for ICE and PHEV vehicles
2. Values are calculated correctly using the appropriate consumption rate
3. Column is hidden for BEV vehicles (no fuel)
4. Backend unit tests cover all calculation scenarios
5. Integration test verifies column displays correctly
