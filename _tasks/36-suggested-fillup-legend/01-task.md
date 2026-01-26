# Task 36: Suggested Fillup Legend Indicator

## Summary

Show a suggested fillup indicator in the table legend when there's an unclosed fuel period. Displays the recommended liters and resulting consumption rate that the magic fill button would calculate.

## User Story

As a user with trips in an open fuel period (no full tank closure), I want to see a suggested fillup amount in the legend so I know what to log next time I fill up, without having to click edit on a trip.

## Requirements

1. **Trigger**: Show when `open_period_km > 0` (trips without a closing full tank)
2. **Display**: "Návrh tankovania: 38 L → 5.78 l/100km" (Slovak primary)
3. **Color**: Green (success) - `var(--accent-success)`
4. **Location**: In the table legend alongside existing indicators
5. **Interaction**: None - purely informational
6. **Vehicle types**: ICE and PHEV only (skip for pure BEV)

## Technical Approach

- Pre-calculate `suggestedFillup` per-trip in `TripData` during `get_trip_grid_data`
- Legend displays the LAST trip's suggestion (most actionable)
- Magic fill button simplified to use pre-calculated value (no backend call)
- Values stable until next data refresh (deterministic within session)

## Out of Scope

- Clicking the indicator to take action
- Caching across sessions
- User preference to hide the indicator
