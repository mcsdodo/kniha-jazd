**Date:** 2026-09-08
**Subject:** How task 79 closed
**Status:** Closed

## What was applied

The user corrected the three 2026 rows by hand in the production book, at
17:13 and 17:14 UTC on 2026-09-08. The result matches
[02-correction.md](02-correction.md), and it also breaks the datetime tie that
made the order ambiguous.

| Trip | Field | Value in production now |
|---|---|---|
| `03f46d80` | `odometer` | 69063 |
| `a51cb498` | `odometer` | 69416 |
| `a51cb498` | `start_datetime` | `2026-08-19T15:21:00` |
| `32631e0e` | `odometer` | 69466, not edited |

The book now reads in the correct order: the 4 km hop to the OMV station comes
first, the 353 km leg home comes second.

## Measured after the correction

Source: `get_trip_grid_data` against `https://kniha-jazd.lacny.me`, vehicle
`c5c0b5d8-abaf-4e21-b6d4-fb289c26e854`, on 2026-09-08.

| Year | Odometer span warnings | Duplicate datetime warnings |
|---|---|---|
| 2023 | 1 (`0f477338`, span 3 km) | 26 |
| 2024 | 0 | 26 |
| 2025 | 0 | 14 |
| 2026 | **0** | **0** |

## The 2023 row stays as it is

The user decided to leave 2023: "the 2023 we leave, the error propagates and I
don't want to spend time on it". Neither Option A (`initial_odometer` 3147 ->
3125) nor Option B (the +22 cascade over 69 rows) was applied. The open row is
no longer row 1 (`4d1273be`) as
[80-one-trip-ordering/03-status.md](../80-one-trip-ordering/03-status.md)
recorded. It is now `0f477338`, with a 3 km span. The 2023 warning is expected
and needs no action.

## One operational note

Just after the hand edit, the production app returned the corrected odometers
and a stale span warning for `32631e0e` in the same response, with an empty
`odometerSpans` map. Two later reads returned no warning. The edit went into the
database file while the container held it open. Prefer the RPC write path, or a
container restart after a direct file edit, so a read cannot show a mixed state.
