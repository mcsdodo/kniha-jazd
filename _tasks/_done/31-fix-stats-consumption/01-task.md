**Date:** 2026-01-09
**Subject:** Fix stats average consumption to use only closed fill-up periods
**Status:** Planning

## Problem

The stats header "Spotreba" (average consumption) currently calculates:
```rust
let avg_consumption_rate = (total_fuel / total_km) * 100.0;
```

This includes trips in the current open period (after last fill-up), which:
1. Shows a different number than what table rows display
2. Can show misleadingly low consumption when driving after a fill-up

## Example

- Fill-up: 49.38L (closes previous period)
- 4 trips after: ~193km total (open period - no closing fill-up yet)
- Table shows: 5.1 L/100km (TP estimate for open period)
- Stats show: 4.97 L/100km (49.38L / 993km including open period)

User sees -2.6% deviation but table shows rates >= TP rate - confusing!

## Solution

Calculate average consumption only from **closed fill-up periods**:
- A period closes when a **full tank fill-up** occurs
- Trips after the last fill-up are in an "open" period and excluded
- If no closed periods exist, show `margin_percent: None`

## Assumptions

- Year starts with full tank
- Year ends with full tank
- No cross-year period calculations needed

## Acceptance Criteria

- [ ] Stats "Spotreba" matches weighted average of closed period rates
- [ ] Stats "Odchylka" is `None` when no closed periods
- [ ] Existing tests pass
- [ ] New tests cover the closed-period calculation
