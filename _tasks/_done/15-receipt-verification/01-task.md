**Date:** 2025-12-29
**Subject:** Receipt-to-Trip Verification System
**Status:** Planning

## Goal

Replace manual receipt assignment with automatic verification that matches invoices to trip fill-ups. The goal is 100% match between receipts in the folder and fuel entries in trips - ensuring accounting accuracy.

## Requirements

### Matching Criteria (Exact Match)
- Date: `trip.date == receipt.receipt_date`
- Liters: `trip.fuel_liters == receipt.liters`
- Price: `trip.fuel_cost_eur == receipt.total_price_eur`

### Doklady Page
- Summary bar showing verification status: "X/Y dokladov overených | Z neoverené"
- Matched receipts: green "Overený" badge + linked trip info (date, route)
- Unmatched receipts: red "Neoverený" badge + manual assign button
- Keep TripSelectorModal for manual assignment of unmatched receipts

### Jazdy Page
- Warning icon (⚠) next to fuel amount for trips without matching receipt
- Legend explaining all indicators:
  - `*` = čiastočné tankovanie (partial fillup)
  - `⚠` = bez dokladu (no receipt)
  - červený riadok = vysoká spotreba (high consumption)

### Cleanup
- Delete ReceiptPicker.svelte
- Remove ReceiptPicker from TripRow.svelte and TripGrid.svelte

## Technical Notes

- Verification runs per vehicle/year (same scope as trip grid)
- Backend returns verification status with each receipt
- Trip grid data extended to include `has_matching_receipt` flag
- Exact matching means OCR accuracy is critical - unmatched = OCR error or missing trip
