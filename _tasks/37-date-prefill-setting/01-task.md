# Task: Configurable Date Prefill for New Trip Entries

## Problem

When adding a new trip entry, the date field is prefilled with the previous trip's date + 1 day. This works well for batch entry (recording multiple past trips at once), but not for daily entry (recording the trip on the same day it happens).

Users with mixed workflows need to switch between these modes easily.

## Solution

Add a **segmented toggle** (`+1 | Dnes`) next to the "Nový záznam" button in the trip grid header:

- **+1** (default): Prefill with last trip date + 1 day (current behavior)
- **Dnes**: Prefill with today's date

The setting persists across sessions via `local.settings.json`.

## User Stories

1. As a user doing batch entry, I want new entries prefilled with previous +1 so I can quickly record sequential past trips
2. As a user doing daily entry, I want new entries prefilled with today's date so I don't have to change it every time
3. As a user, I want the toggle visible near the "New entry" button so I can switch modes with one click
4. As a user, I want my preference saved so I don't have to set it again after restarting the app

## Out of Scope

- Per-vehicle date prefill settings
- "Smart" auto-detection based on gap since last trip
- Date prefill for inserted rows (between existing trips)
