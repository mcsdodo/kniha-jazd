**Date:** 2026-09-08
**Subject:** The odometer corrections, with the measured legal consequence
**Status:** Waiting for your approval of the exact values

## What this document is

Task 79 asks you to approve exact numbers before anything is written to the
production book. This is that decision package. It has four parts:

1. What is wrong now, per row.
2. The exact new values.
3. What each change does to the consumption rate and the margin.
4. What the data cannot settle, and what stays broken.

**Nothing here was written to production.** All writes went to a private copy.

## How every number was measured

| Item | Value |
|---|---|
| Build | `kniha-jazd-web:local`, built from `b1af28e`, app version `0.44.0` |
| Database | A copy of the production book, taken 2026-09-08 |
| Container | `kniha-jazd-t79` on port 3468, `kniha-jazd-t79b` on port 3469 |
| Source of the numbers | `get_trip_grid_data`, `preview_trip_calculation`, `recalculate_odometers` with `dryRun` |

`b1af28e` is the head of task 80. One comparator now decides trip order:
`start_datetime`, then `created_at`, then `odometer`, then `id`. Every figure
below reads that order. Figures from the older task 79 file do not.

Each number below is labelled **measured** (the running build returned it) or
**arithmetic** (I calculated it and say so).

The three `curl` blocks in Part 5 were run verbatim against a fresh copy of the
book on a fourth container. All three succeeded, and the book then reported 0
span warnings in all four years (measured). They are not transcriptions.

## Part 1 - What is wrong now

The book raises 4 odometer span warnings. Measured with
`.superpowers/sdd/02-plan/measure-book.sh 3468`:

| Year | Rows | Chain breaks | Span warnings |
|---|---|---|---|
| 2023 | 69 | 0 | 1 |
| 2024 | 84 | 0 | 0 |
| 2025 | 68 | 0 | 0 |
| 2026 | 108 | 0 | 3 |

### The three 2026 rows

Vehicle BT014IN. "Start" is not stored. It is the previous row's stored ending
odometer. All values measured.

| # | Trip | Date | Route | km | Start | End | Span | Error |
|---|---|---|---|---|---|---|---|---|
| 106 | `03f46d80` | 2026-08-19 15:00 | Mlynske Nivy 14 -> OMV Strojnicka | 4 | 69059 | 69415 | **356** | +352 |
| 107 | `a51cb498` | 2026-08-19 15:00 | OMV Strojnicka -> Spisska Nova Ves | 352 | 69415 | 69411 | **-4** | -356 |
| 108 | `32631e0e` | 2026-08-27 15:00 | Spisska Nova Ves -> Ganovce | 50 | 69411 | 69465 | **54** | +4 |

A span of -4 km is impossible. The row ends lower than it starts.

Row 106 is the fill-up: 45.2 litres, 85.12 EUR, full tank. Row 108 is collateral
only. It starts from row 107's ending odometer, so it inherits row 107's error.

The two 2026-08-19 rows share the exact `start_datetime` `15:00:00`. The
comparator therefore uses `created_at`:

- `03f46d80`, the 4 km hop, created 2026-08-23 **12:50:19**. It sorts first.
- `a51cb498`, the 352 km leg, created 2026-08-23 **12:51:12**. It sorts second.

The places confirm that order. Row 106 ends at OMV Strojnicka and row 107 starts
there. So the order is right and the two stored odometers are wrong.

### The 2023 row

| # | Trip | Date | Route | km | Start | End | Span | Error |
|---|---|---|---|---|---|---|---|---|
| 1 | `4d1273be` | 2023-04-25 00:00 | Horny dvor 4665, Senec -> Narcisova 44, Bratislava | 22 | 3125 or 3147 | 3147 | **0** | -22 |

Purpose: `prevzatie auta (1 cesta)`. It is the first row of the whole book.

"Start" for row 1 is the vehicle's `initial_odometer`, which is **3147** today.
The row's stored ending odometer is also 3147. The span is therefore 0, but the
row records 22 km.

This warning is new. It is not in the task 79 file. That file reports a span of
-34910, because `initial_odometer` was 38057 then. You changed the field to 3147
on 2026-09-08 06:11 (`vehicles.updated_at`). That removed the absurd number but
left the row 22 km short.

## Part 2 - The exact new values

### 2026: two writes, in this order

| Trip | Field | Old | New |
|---|---|---|---|
| `03f46d80-8fda-4bf4-b0ae-1364f967507b` | `odometer` | 69415 | **69063** |
| `a51cb498-bf29-4875-b0c6-5079a93d16e9` | `odometer` | 69411 | **69415** |

`32631e0e` needs no edit. Its start becomes 69415, and 69465 - 69415 = 50, which
matches its recorded distance.

**The task 79 arithmetic is confirmed.** The proof is independent of me: on an
untouched copy, `recalculate_odometers` for 2026 with `dryRun: true` returns
exactly these two rows and exactly these two values, and no others (measured).

**Order matters.** After the first write alone, the book still warns: `a51cb498`
then spans 348 against 352 recorded (measured). Apply both, or neither.

After both writes, 2026 raises 0 span warnings and 0 chain breaks (measured).

### 2023: one field, but you must choose

Two values satisfy the invariant. They make opposite claims about the book.

**Option A - `initial_odometer` 3147 -> 3125.** This says the handover reading at
Senec was 3125, the car reached Narcisova at 3147, and every stored odometer in
the book is correct.

Measured effect of Option A on the copy:

- 2023 span warning cleared. All years then raise 0 span warnings.
- `yearStartOdometer` for 2023 moves 3147 -> 3125.
- Row 1's "Km pred" cell moves 3147 -> 3125.
- The three synthetic month-end rows for January, February and March 2023 move
  3147 -> 3125. Those months have no trips.
- **Nothing else changes.** Rates, estimated rates, fuel remaining, fuel
  consumed, trip numbers, every stored trip odometer, and the years 2024, 2025
  and 2026 are byte-identical before and after (measured by diffing the full
  `get_trip_grid_data` response for all four years).

**Option B - row 1 ending odometer 3147 -> 3169.** This says 3147 is the handover
reading, so the whole book is 22 km low from row 1 onward.

Measured cost of Option B on a second copy:

- 69 rows rewritten in 2023, each +22 km.
- That immediately creates a **new** span warning at the 2023/2024 boundary
  (measured: 2024 goes from 0 warnings to 1).
- To clear it: 84 more rows in 2024, then 68 in 2025, then 2 in 2026.
- Total: **223 legal odometer values rewritten** to fix one row.

### Why the data leans to Option A, and what it cannot prove

The evidence in the database:

- The imported chain is self-consistent from row 1's **end** onward. Row 2 starts
  at 3147 and ends at 3172, which is exactly its 25 recorded km. Row 3 is 3172 to
  3274, exactly 102 km. The import carried a coherent chain.
- `initial_odometer` was never part of that import. Its imported value was 38057,
  which is not a plausible handover reading for a book that starts at 3147. It
  looks like a value copied from somewhere else.
- All 69 rows of 2023 carry the identical `created_at` of
  `2025-12-26T18:52:02.172293Z`. They came from one import, not from typing.

So the book's own numbers say row 1 ends at 3147. Only the anchor is unproven.

**The database cannot settle it.** The handover reading is not stored anywhere.
`receipts` has no odometer column. There is no service record in the schema. The
one fact that would decide it is outside the database:

- The dealer's handover protocol from 2023-04-25, if you kept it. Does it say
  3125 or 3147?
- Failing that, the real odometer against the book. Under Option A the book ends
  at **69465** on 2026-08-27 (measured). Under Option B, fully applied, it ends
  at about **69487** (arithmetic: 69465 + 22, less the 0.5 km of Part 4 if that
  is absorbed too, so 69486.5). The 22 km gap discriminates either way. Your
  vehicle has `sensor.bt014in_odometer` configured in Home Assistant, so a
  historical reading near 2026-08-27 would decide it. I did not query Home
  Assistant.

**My reading:** Option A is far more likely and costs one field. Option B would
rewrite 223 values of a legal record on the strength of one number that was
itself typed by hand this morning. But I cannot prove Option A from the data, and
I will not present a guess as a fact on a legal record. **You decide.**

If you are not sure, leaving 2023 alone is a defensible third choice. The warning
is a sign on one cell. It changes no consumption figure.

## Part 3 - The legal consequence, measured

### The affected 2026 period

The fill-up on `03f46d80` closes a consumption period. That period is trips 102
to 106, from 2026-08-07 to 2026-08-19 (measured).

| Item | Before | After | Method |
|---|---|---|---|
| Period distance | 782 km | 782 km | measured, `get_trip_grid_data` |
| Period fuel | 45.2 L | 45.2 L | measured |
| Consumption rate | 5.780051 l/100km | 5.780051 l/100km | measured, `rates` map |
| Margin over TP 5.1 | 13.3343 % | 13.3343 % | measured, `preview_trip_calculation` |
| Over the 20 % limit | no | no | measured, `isOverLimit` |

**The correction does not move the rate or the margin at all.**

That is a real finding, and it corrects the task 79 file. That file says "the
wrong spans have been feeding the consumption figures". They were not. A period's
distance comes from the `distance_km` column of each row, never from the
odometer (`calculate_closed_period_totals`,
`src-tauri/core/src/calculations/mod.rs:96-121`, and the rate loop at
`src-tauri/core/src/commands_internal/statistics.rs:113-128`). The odometer is
not an input to any consumption number.

What did skew this period was the **ordering**, and task 80 already fixed it.
Under the old rules the 352 km leg was counted inside the period, which read
45.2 L over 1134 km, that is 3.986 l/100km (arithmetic, from the figures in the
task brief). Under the canonical order the leg falls after the fill-up and the
period reads 45.2 L over 782 km, that is 5.780 l/100km (measured). Both are under
the limit. The book is now on the correct figure whether or not you apply this
correction.

### The 2023 change

No consumption effect. Measured: the `rates`, `estimatedRates`, `fuelRemaining`
and `fuelConsumed` maps for 2023 are identical before and after the
`initial_odometer` change. 2023 raises no consumption warning either way.

### Whole book

No row in any year is over the 20 % limit before or after. Measured:
`consumptionWarnings` is empty for 2026 in both states.

## Part 4 - What stays broken

- **The 2025 half kilometre.** Row 1 of 2025, 2025-01-12, spans 88.5 against 88
  recorded (measured). It sits under the 1 km warning tolerance, so it raises no
  sign. Its source is the last row of 2024, 2024-12-31, which ends at 38056.5.
  Every one of the 68 rows of 2025 is 0.5 km above the distance-derived total
  because of it. Nothing in this package touches it. Do not run
  `recalculate_odometers` on 2025 unless you want 68 legal values moved by
  -0.5 km.
- **If you apply the 2026 correction but not the 2023 one**, the book still
  raises 1 span warning, on 2023 row 1.
- **If you apply neither**, all 4 warnings stand.
- **The `00:00` default start time** is unchanged. 30 of the 31 tied timestamp
  groups in the book sit at `00:00`. That is what makes ties common. It is out of
  scope here.

Measured end state of my copy, after the two 2026 writes and Option A:

| Year | Rows | Chain breaks | Drift rows | Span warnings |
|---|---|---|---|---|
| 2023 | 69 | 0 | 0 | 0 |
| 2024 | 84 | 0 | 0 | 0 |
| 2025 | 68 | 0 | 68 | 0 |
| 2026 | 108 | 0 | 0 | 0 |

"Drift rows" counts rows whose stored odometer differs from the running total of
the distances. The 68 in 2025 are the half kilometre above.

## Part 5 - How to apply it

### Apply by RPC, not in the browser

`update_trip` replaces the whole row. A payload that omits `fuelLiters` would
erase the 45.2 litres on `03f46d80` and destroy the period. Every payload below
is complete and was executed on the copy. After each write I read the row back
and confirmed `fuelLiters` is still 45.2 and `fullTank` is still true (measured).

The frontend no longer recalculates the year on save. `recalculateAllOdo` is gone
from the Svelte code as of `4f451ee`, and no page calls `recalculate_odometers`.
So editing in the UI would not cascade on this build. Even so, raw RPC is the
safer path: it writes exactly what you send.

**Back up `/data/kniha-jazd.db` before any write.**

### The three calls

Replace `HOST` with the target. Run them in this order.

Write 1 - `03f46d80`, odometer 69415 -> 69063:

```bash
curl -s -X POST HOST/api/rpc -H 'Content-Type: application/json' -d '{
  "command": "update_trip",
  "args": {
    "id": "03f46d80-8fda-4bf4-b0ae-1364f967507b",
    "startDatetime": "2026-08-19T15:00:00",
    "endDatetime": "2026-08-19T15:20:00",
    "origin": "Mlynske Nivy 14, Bratislava",
    "destination": "OMV Strojnicka, Bratislava",
    "distanceKm": 4.0,
    "odometer": 69063.0,
    "purpose": "tankovanie",
    "fuelLiters": 45.2,
    "fuelCostEur": 85.12,
    "fullTank": true,
    "energyKwh": null,
    "energyCostEur": null,
    "fullCharge": false,
    "socOverridePercent": null,
    "otherCostsEur": null,
    "otherCostsNote": ""
  }
}'
```

Write 2 - `a51cb498`, odometer 69411 -> 69415:

```bash
curl -s -X POST HOST/api/rpc -H 'Content-Type: application/json' -d '{
  "command": "update_trip",
  "args": {
    "id": "a51cb498-bf29-4875-b0c6-5079a93d16e9",
    "startDatetime": "2026-08-19T15:00:00",
    "endDatetime": "2026-08-19T18:00:00",
    "origin": "OMV Strojnicka, Bratislava",
    "destination": "Kamenny obrazok 26, Spisska Nova Ves",
    "distanceKm": 352.0,
    "odometer": 69415.0,
    "purpose": "navrat SC",
    "fuelLiters": null,
    "fuelCostEur": null,
    "fullTank": true,
    "energyKwh": null,
    "energyCostEur": null,
    "fullCharge": false,
    "socOverridePercent": null,
    "otherCostsEur": null,
    "otherCostsNote": ""
  }
}'
```

Write 3 - Option A only, `initial_odometer` 3147 -> 3125. `update_vehicle` takes
the whole vehicle, so every field is present. `updatedAt` is required and is
stored exactly as you send it (measured), so put the current UTC time there
instead of the value below if you want the record to show when you changed it:

```bash
curl -s -X POST HOST/api/rpc -H 'Content-Type: application/json' -d '{
  "command": "update_vehicle",
  "args": {
    "vehicle": {
      "id": "c5c0b5d8-abaf-4e21-b6d4-fb289c26e854",
      "name": "Mercedes Benz C All-Terrain",
      "licensePlate": "BT014IN",
      "vin": "W1KAH1EB0PF066297",
      "driverName": "Jozef Lačný",
      "vehicleType": "Ice",
      "tpConsumption": 5.1,
      "tankSizeLiters": 66.0,
      "initialOdometer": 3125.0,
      "isActive": true,
      "batteryCapacityKwh": null,
      "baselineConsumptionKwh": null,
      "initialBatteryPercent": null,
      "haOdoSensor": "sensor.bt014in_odometer",
      "haFuelLevelSensor": "sensor.bt014in_fuel_level",
      "haFillupSensor": "input_text.bt014in_suggested_fillup",
      "createdAt": "2025-12-23T08:08:38.302174600Z",
      "updatedAt": "2026-09-08T06:11:27.544Z"
    }
  }
}'
```

### Alternative for the 2026 pair

`recalculate_odometers` produces the same two writes in one call. Run the dry run
first and check that it returns exactly two rows and the values above. If it
returns more, stop: the book moved since this package was written.

```bash
curl -s -X POST HOST/api/rpc -H 'Content-Type: application/json' \
  -d '{"command":"recalculate_odometers","args":{"vehicleId":"c5c0b5d8-abaf-4e21-b6d4-fb289c26e854","year":2026,"dryRun":true}}'
```

Do **not** run this command on 2023 or 2025. Measured dry runs on the current
book: 2023 would rewrite 69 rows, 2025 would rewrite 68, and 2024 would rewrite
0. The 2023 run is Option B in disguise, and it cascades.

### Verify after applying

```bash
curl -s -X POST HOST/api/rpc -H 'Content-Type: application/json' \
  -d '{"command":"get_trip_grid_data","args":{"vehicleId":"c5c0b5d8-abaf-4e21-b6d4-fb289c26e854","year":2026}}' \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["odometerSpanWarnings"])'
```

Expect `[]` for 2026, and `[]` for 2023 if you took Option A.

## Decisions you need to make

1. Apply the two 2026 odometer writes? They fix an impossible -4 km span and
   change no consumption figure.
2. For 2023: Option A (`initial_odometer` -> 3125, one field), Option B (rewrite
   223 rows), or leave it?
3. Leave the 2025 half kilometre alone? Recommended, unless you want 68 rows
   moved by -0.5 km.
