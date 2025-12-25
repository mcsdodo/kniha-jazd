# Task: Support Partial Fillups

## Problem

The current consumption calculation assumes every fillup is a FULL tank (fill to 100%). When a partial fillup is recorded (e.g., 19.67L on 25.09), the system:

1. Calculates an impossibly low consumption rate (19.67L / 1037km = 1.89 l/100km)
2. Applies this rate retroactively to all trips in the period
3. Shows incorrect "zostatok" (remaining fuel) values
4. Caps at tank size, showing full tank when it shouldn't be

## Current Behavior

```
Rate = fuel_liters / km_since_last_fillup × 100
```

This only works when fillup = actual fuel consumed since last fillup (i.e., full tank).

## Desired Behavior

- Support both full tank and partial fillups
- Only calculate consumption rate from full tank fillups
- Use TP rate for periods without a full tank fillup
- Show accurate "zostatok" values

## Acceptance Criteria

1. User can mark a fillup as "full tank" or "partial"
2. Consumption rate calculation uses only full tank fillups
3. Zostatok calculation handles partial fillups correctly
4. Existing data continues to work (assume existing fillups are full tank)
