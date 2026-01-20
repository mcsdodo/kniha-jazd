# Task: Add Mismatch Reason to Unverified Receipt Status

**Date:** 2026-01-20
**Status:** ✅ Complete
**Commits:** `980a0a2`, `c971841`

## Problem Statement

When a receipt shows as "neoverena" (unverified), users have no idea WHY it's unverified. They see the red badge but don't know what action to take.

**Example scenario:**
- Trip dated 19.1.2026 with 63.68 L and 91.32 €
- Receipt dated 20.1.2026 with 63.68 L and 91.32 €
- Receipt shows as "neoverena" because dates don't match (off by 1 day)
- User thinks it might be related to consumption limit (it's not!)

## Current Behavior

The `ReceiptVerification` struct only has a boolean `matched` field:
```rust
pub struct ReceiptVerification {
    pub receipt_id: String,
    pub matched: bool,  // Just true/false, no reason
    pub matched_trip_id: Option<String>,
    pub matched_trip_date: Option<String>,
    pub matched_trip_route: Option<String>,
}
```

UI shows generic "Neoverený" badge with no explanation.

## Desired Behavior

Show specific, actionable reasons why a receipt is unverified:

| Scenario | Reason Shown (Slovak) |
|----------|----------------------|
| Receipt missing date/liters/price | "Chýbajú údaje na doklade" |
| No trip with fuel data exists | "Žiadna jazda s tankovaním" |
| Same liters+price, different date | "Dátum 20.1. - jazda je 19.1." |
| Same date+price, different liters | "63.68 L - jazda má 50.0 L" |
| Same date+liters, different price | "91.32 € - jazda má 85.0 €" |
| Other-cost receipt, no match | "Žiadna jazda s touto cenou" |

## Success Criteria

1. Users can see WHY a receipt is unverified
2. Reason is displayed inline (not just tooltip) for visibility
3. Reason text is actionable (tells user what to fix)
4. Works for both fuel receipts and other-cost receipts
5. i18n support (Slovak primary, English secondary)

## Out of Scope

- Auto-fixing mismatches
- Fuzzy date matching (e.g., ±1 day tolerance)
- Suggesting specific trips to assign to
