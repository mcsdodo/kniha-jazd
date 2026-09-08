**Date:** 2026-09-08
**Subject:** What one trip ordering does to the real book, measured on two builds
**Status:** Code landed. The renumbering below needs your approval before the build reaches production.

## What this document is

Task 80 made one Rust comparator decide trip order. This document measures what
that does to your book. It has three parts:

1. The trips whose "Poradové číslo jazdy" changes, with date and route.
2. What does not change - odometers, distances, rates, margins.
3. What the new `recalculate_odometers` command would do, per year, from a dry
   run that wrote nothing.

**Nothing here was written to production.** All reads went to private copies.

## How every number was measured

| Item | Value |
|---|---|
| Old build | `kniha-jazd-web:pre80`, from commit `860a98f`, the commit before task 80 |
| New build | `kniha-jazd-web:local`, from commit `c818fe7`, the head of task 80 |
| App version, both | `0.44.0` |
| Database | One copy of the production book, made 2026-09-08 16:29, md5 `9fd6052e4247b1ad4dab9fe701bb8a23` |
| Copies | Three identical copies, one per container: `kj80-pre` (port 3471), `kj80-post` (3472), `kj80-dry` (3473) |
| Source of the numbers | `get_trip_grid_data`, `calculate_trip_stats`, `recalculate_odometers` with `dryRun` |
| Book size | 329 trips, one vehicle (BT014IN). The Tesla has no trips in any year. |

The book moved during this work. The copy above is later than the one
[02-correction.md](../79-odometer-span-inconsistency/02-correction.md) used: rows
107 and 108 of 2026 were edited by hand at 16:27 and 16:28 on 2026-09-08. Every
number below reads the later copy.

## Part 1 - The whole book, old build against new build

"Chain breaks" counts rows whose displayed "Km pred" does not equal the stored
odometer of the row above it, walking the grid in trip-number order. It must be
0: the column is derived from the row above.

| Year | Rows | Chain breaks OLD | Chain breaks NEW | Span warnings OLD | Span warnings NEW | Trip numbers changed | Km pred changed | Rates changed |
|---|---|---|---|---|---|---|---|---|
| 2023 | 69 | 34 | **0** | 1 (row 1) | 1 (row 1) | 24 | 0 | 0 |
| 2024 | 84 | 36 | **0** | 0 | 0 | 24 | 0 | 0 |
| 2025 | 68 | 19 | **0** | 0 | 0 | 14 | 0 | 0 |
| 2026 | 108 | 0 | **0** | 1 (row 106) | 1 (row 106) | 0 | 0 | 0 |

**The change is the order, not the values.** No odometer moves, no "Km pred" cell
moves, no rate moves. 62 rows get a different number, and the column that was
already correct per row now also reads correctly down the page.

Why the fix is free on this book: in the tied groups of 2023 to 2025 every row
carries the same `created_at`, because one import wrote them all. The comparator
therefore falls through to the odometer, which is the order the old odometer
chain already used. The old build numbered those rows by a different rule, so the
numbers moved and the odometers did not.

**2026 is untouched.** All 62 swaps sit in the imported years.

## Part 2 - The renumbering, row by row

This is the legal "Poradové číslo jazdy" column. Every swap is inside a group of
trips that share the same date and time. No date, route, distance or odometer
changes.

#### 2023 -- 24 of 69 rows renumbered

| Old # | New # | Date | Route | km | ODO |
|---|---|---|---|---|---|
| 7 | **6** | 2023-05-24 | Narcisova 44, Bratislava -> Narcisova 44, Bratislava | 65 | 3662 |
| 6 | **7** | 2023-05-24 | Narcisova 44, Bratislava -> Mlynske Nivy 14, Bratislava | 11 | 3673 |
| 11 | **9** | 2023-05-26 | Narcisova 44, Bratislava -> Mlynske Nivy 14, Bratislava | 7 | 3970 |
| 9 | **11** | 2023-05-26 | Brno Londynske Namesti -> Wag Payment solutions Na Vitezne Strani Praha | 201 | 4462 |
| 13 | **12** | 2023-05-27 | Wag Payment solutions Na Vitezne Strani Praha -> Narcisova 44, Bratislava | 340 | 4802 |
| 12 | **13** | 2023-05-27 | Narcisova 44, Bratislava -> Narcisova 44, Bratislava | 228 | 5030 |
| 28 | **27** | 2023-07-14 | Elektrarenska 2, Spisska Nova Ves -> Elektrarenska 2, Spisska Nova Ves | 220 | 9185 |
| 27 | **28** | 2023-07-14 | Elektrarenska 2, Spisska Nova Ves -> Motorgroup, Partizánska 4447/102, Poprad | 66 | 9251 |
| 34 | **33** | 2023-08-01 | Elektrarenska 2, Spisska Nova Ves -> Mlynske Nivy 14, Bratislava | 370 | 10343 |
| 33 | **34** | 2023-08-01 | Mlynske Nivy 14, Bratislava -> Stara Vajnorska 37A | 12 | 10355 |
| 36 | **35** | 2023-08-02 | Stara Vajnorska 37A -> Mlynske Nivy 14, Bratislava | 12 | 10367 |
| 35 | **36** | 2023-08-02 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 10737 |
| 43 | **42** | 2023-09-07 | Elektrarenska 2, Spisska Nova Ves -> Mlynske Nivy 14, Bratislava | 370 | 11622 |
| 42 | **43** | 2023-09-07 | Mlynske Nivy 14, Bratislava -> Brno Londynske Namesti | 266 | 11888 |
| 45 | **44** | 2023-09-08 | Mlynske Nivy 14, Bratislava -> Ivanska cesta 18, IKEA | 16 | 11904 |
| 44 | **45** | 2023-09-08 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 12274 |
| 53 | **52** | 2023-10-12 | Mlynske Nivy 14, Bratislava -> Mlynske Nivy 14, Bratislava | 105 | 13892 |
| 52 | **53** | 2023-10-12 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 14262 |
| 61 | **60** | 2023-11-15 | Mlynske Nivy 14, Bratislava -> Mlynske Nivy 14, Bratislava | 165 | 15638 |
| 60 | **61** | 2023-11-15 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 16008 |
| 65 | **63** | 2023-12-11 | Elektrarenska 2, Spisska Nova Ves -> Brno Londynske Namesti | 377 | 16453 |
| 63 | **65** | 2023-12-11 | Mlynske Nivy 14, Bratislava -> Solivarska 5, Bratislava | 4 | 16587 |
| 67 | **66** | 2023-12-12 | Solivarska 5, Bratislava -> Mlynske Nivy 14, Bratislava | 4 | 16591 |
| 66 | **67** | 2023-12-12 | Mlynske Nivy 14, Bratislava -> Mlynske Nivy 14, Bratislava | 71 | 16662 |

#### 2024 -- 24 of 84 rows renumbered

| Old # | New # | Date | Route | km | ODO |
|---|---|---|---|---|---|
| 2 | **1** | 2024-01-04 | Elektrarenska 2, Spisska Nova Ves -> Brno Londynske Namesti | 377 | 17416 |
| 1 | **2** | 2024-01-04 | Brno Londynske Namesti -> Wag Payment solutions Na Vitezne Strani Praha | 202 | 17618 |
| 4 | **3** | 2024-01-06 | Wag Payment solutions Na Vitezne Strani Praha -> Shell, Pri rieke 1110/4, Zilina | 413 | 18031 |
| 3 | **4** | 2024-01-06 | Shell, Pri rieke 1110/4, Zilina -> Elektrarenska 2, Spisska Nova Ves | 168 | 18199 |
| 8 | **6** | 2024-01-12 | Mlynske Nivy 14, Bratislava -> Mlynske Nivy 14, Bratislava | 177 | 18746 |
| 6 | **8** | 2024-01-12 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 19119 |
| 10 | **9** | 2024-01-22 | Elektrarenska 2, Spisska Nova Ves -> Mlynske Nivy 14, Bratislava | 260 | 19379 |
| 9 | **10** | 2024-01-22 | Mlynske Nivy 14, Bratislava -> Brno Londynske Namesti | 370 | 19749 |
| 14 | **12** | 2024-02-13 | Elektrarenska 2, Spisska Nova Ves -> Mlynske Nivy 14, Bratislava | 370 | 20489 |
| 12 | **14** | 2024-02-13 | Mlynske Nivy 14, Bratislava -> Mlynske Nivy 14, Bratislava | 45 | 20794 |
| 19 | **18** | 2024-03-11 | Mlynske Nivy 14, Bratislava -> Mlynske Nivy 14, Bratislava | 55 | 21570.5 |
| 18 | **19** | 2024-03-11 | Mlynske Nivy 14, Bratislava -> Brno Londynske Namesti | 260 | 21830.5 |
| 28 | **27** | 2024-04-11 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 23875.5 |
| 27 | **28** | 2024-04-11 | Elektrarenska 2, Spisska Nova Ves -> Elektrarenska 2, Spisska Nova Ves | 35 | 23910.5 |
| 31 | **30** | 2024-05-07 | Mlynske Nivy 14, Bratislava -> Wag Payment solutions Na Vitezne Strani Praha | 333 | 24613.5 |
| 30 | **31** | 2024-05-07 | Wag Payment solutions Na Vitezne Strani Praha -> Mlynske Nivy 14, Bratislava | 333 | 24946.5 |
| 36 | **35** | 2024-05-23 | Mlynske Nivy 14, Bratislava -> OMV D1 Zlate piesky | 10 | 25956.5 |
| 35 | **36** | 2024-05-23 | OMV D1 Zlate piesky -> Elektrarenska 2, Spisska Nova Ves | 360 | 26316.5 |
| 39 | **38** | 2024-06-10 | Mlynske Nivy 14, Bratislava -> Brno Londynske Namesti | 260 | 26936.5 |
| 38 | **39** | 2024-06-10 | Mlynske Nivy 14, Bratislava -> CS Shell Vajnorska | 11 | 26947.5 |
| 78 | **77** | 2024-11-27 | Mlynske Nivy 14, Bratislava -> OMV Tatranska cesta, Ruzomberok | 260 | 36799.5 |
| 77 | **78** | 2024-11-27 | OMV Tatranska cesta, Ruzomberok -> Elektrarenska 2, Spisska Nova Ves | 110 | 36909.5 |
| 83 | **82** | 2024-12-14 | Mlynske Nivy 14, Bratislava -> Orlen Prievozska, Bratislava | 1 | 37685.5 |
| 82 | **83** | 2024-12-14 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 38055.5 |

#### 2025 -- 14 of 68 rows renumbered

| Old # | New # | Date | Route | km | ODO |
|---|---|---|---|---|---|
| 23 | **22** | 2025-05-05 | Motorgroup, Partizánska 4447/102, Poprad -> Elektrarenska 2, Spisska Nova Ves | 32 | 43091 |
| 22 | **23** | 2025-05-05 | Elektrarenska 2, Spisska Nova Ves -> Elektrarenska 2, Spisska Nova Ves | 120 | 43211 |
| 28 | **27** | 2025-05-30 | Mlynske Nivy 14, Bratislava -> Mlynske Nivy 14, Bratislava | 134 | 44455 |
| 27 | **28** | 2025-05-30 | Mlynske Nivy 14, Bratislava -> Elektrarenska 2, Spisska Nova Ves | 370 | 44825 |
| 30 | **29** | 2025-06-02 | Elektrarenska 2, Spisska Nova Ves -> OMV Tatranska cesta, Ruzomberok | 100 | 44925 |
| 29 | **30** | 2025-06-02 | OMV Tatranska cesta, Ruzomberok -> Mlynske Nivy 14, Bratislava | 270 | 45195 |
| 51 | **48** | 2025-09-25 | Mlynske Nivy 14, Bratislava -> Wag Payment solutions Na Vitezne Strani Praha | 666 | 50122 |
| 50 | **49** | 2025-09-25 | Mlynske Nivy 14, Bratislava -> Bratislava - Slovnaft Pristavna | 1 | 50123 |
| 49 | **50** | 2025-09-25 | Bratislava - Slovnaft Pristavna -> Kamenny obrazok 26, Spisska Nova Ves | 370 | 50493 |
| 48 | **51** | 2025-09-25 | Kamenny obrazok 26, Spisska Nova Ves -> OMV Duklianska, Spisska Nova Ves | 2 | 50495 |
| 58 | **57** | 2025-10-22 | Mlynske Nivy 14, Bratislava -> WebEye, Fót, Akácos 0221/12 hrsz., 2151 HU | 213 | 52153 |
| 57 | **58** | 2025-10-22 | WebEye, Fót, Akácos 0221/12 hrsz., 2151 HU -> Mlynske Nivy 14, Bratislava | 213 | 52366 |
| 67 | **66** | 2025-12-12 | Mlynske Nivy 14, Bratislava -> Bratislava - Slovnaft Pristavna | 2 | 54456 |
| 66 | **67** | 2025-12-12 | Mlynske Nivy 14, Bratislava -> EuroWag - Malacky, Priemyselná 5851, 901 01 Malacky | 88 | 54544 |

#### 2026 -- 0 of 108 rows renumbered

None.

Most groups are pairs. One group is larger: 2025-09-25 holds four trips
(old 48 to 51), and the new order reverses them.

### What stayed identical, measured

For every year, the old build and the new build return the same values for:
trips, stored odometers, `odometerStart` ("Km pred"), `rates`, `estimatedRates`,
`fuelConsumed`, `fuelRemaining`, `monthEndRows`, `odometerSpans`,
`consumptionWarnings`, `duplicateDatetimeWarnings` and `odometerSpanWarnings`.
Only the row numbers differ.

## Part 3 - The legal figures

`calculate_trip_stats` returns the worst period of the year. The two builds
return the same numbers, to the last digit.

| Year | Worst rate l/100km | Margin over TP 5.1 | Over the 20 % limit | Total km | Fuel L |
|---|---|---|---|---|---|
| 2023 | 6.109889 | 30.82 % | **yes** | 13914 | 850.13 |
| 2024 | 5.987772 | 24.17 % | **yes** | 21017.5 | 1258.48 |
| 2025 | 6.002670 | 19.78 % | no | 16857 | 1011.87 |
| 2026 | 5.964591 | 19.85 % | no | 14552 | 843.93 |

Task 80 moves none of this. The 2023 and 2024 books were already over the limit
before this work and are over it after, by the same amount.

This also corrects one line of
[02-correction.md](../79-odometer-span-inconsistency/02-correction.md): "No row
in any year is over the 20 % limit" holds for 2026, which is what that document
measured, but not for 2023 and 2024.

## Part 4 - The span warnings today

The plan predicted 3 span warnings in 2026. The book has 2 warnings in total
today, one in 2023 and one in 2026, on both builds:

| Year | Row | Trip | Date | Route | km | Span | Error |
|---|---|---|---|---|---|---|---|
| 2023 | 1 | `4d1273be` | 2023-04-25 | Horný dvor 4665, Senec -> Narcisova 44, Bratislava | 22 | 0 | -22 |
| 2026 | 106 | `03f46d80` | 2026-08-19 | Mlynske Nivy 14 -> OMV Strojnicka | 4 | 356 | +352 |

The two other 2026 rows stopped warning because they were edited by hand at
16:27 and 16:28 on 2026-09-08: row 107 went from 352 km to 353 km with odometer
69411 -> 69768, and row 108 from odometer 69465 -> 69818. Those two rows now
agree with their own distances. Row 106 still records a 4 km hop across a
356 km span, so the whole tail of 2026 sits 352 km high.

## Part 5 - `recalculate_odometers`, dry run per year

The command rewrites the year's stored odometers from the distances, in the one
book order. It has **no UI**. Nothing calls it on save. It is reached over RPC
only, and `dryRun: true` writes nothing (verified: the database file's md5 was
identical before and after all four dry runs).

Measured on the untouched copy, 2026-09-08:

| Year | Rows it would change | Change | What it means |
|---|---|---|---|
| 2023 | **69 of 69** | every row +22 km | The 22 km of row 1 pushed through the whole year. This is Option B of the task 79 package, which costs 223 legal values across the book. |
| 2024 | **0** | - | The year already agrees with its distances. |
| 2025 | **68 of 68** | every row -0.5 km | Chases a half kilometre carried over the 2024/2025 boundary. The 1 km warning tolerance ignores it on purpose. |
| 2026 | **3** (rows 106, 107, 108) | each -352 km | Corrects row 106 and carries the correction through the two rows after it. |

The 2026 rows in full:

| Row | Trip | Date | Old odometer | New odometer |
|---|---|---|---|---|
| 106 | `03f46d80` | 2026-08-19 | 69415 | **69063** |
| 107 | `a51cb498` | 2026-08-19 | 69768 | **69416** |
| 108 | `32631e0e` | 2026-08-27 | 69818 | **69466** |

### The operator procedure

Run the dry run first, always. If it returns rows or values other than the ones
above, **stop**: the book moved since this document was written.

```bash
# 1. Dry run. Writes nothing.
curl -s -X POST HOST/api/rpc -H 'Content-Type: application/json' -d '{
  "command": "recalculate_odometers",
  "args": {"vehicleId": "c5c0b5d8-abaf-4e21-b6d4-fb289c26e854", "year": 2026, "dryRun": true}
}'

# 2. Back up /data/kniha-jazd.db.

# 3. Apply, only after you approve the list the dry run returned.
curl -s -X POST HOST/api/rpc -H 'Content-Type: application/json' -d '{
  "command": "recalculate_odometers",
  "args": {"vehicleId": "c5c0b5d8-abaf-4e21-b6d4-fb289c26e854", "year": 2026, "dryRun": false}
}'
```

The reply is a list of `{tripId, tripNumber, oldOdometer, newOdometer}`, one
entry per row that moves. An empty list means the year already agrees with its
distances. The command refuses to write when the app is in read-only mode; the
dry run still answers.

**Do not run it on 2023 or 2025.** See the table above.

## Part 6 - What you decide

1. **The renumbering in Part 2.** It arrives with the build. Nothing runs it and
   nothing can undo it row by row: it is what the one comparator computes from
   the data you already have. Approve it before `:main` reaches the homelab.
2. **The 2026 odometer correction.** Three rows, -352 km each, from Part 5. It
   changes no rate and no margin, as
   [02-correction.md](../79-odometer-span-inconsistency/02-correction.md)
   measured for the same period.
3. **The 2023 row 1 warning.** Still open, still the choice in
   [02-correction.md](../79-odometer-span-inconsistency/02-correction.md):
   `initial_odometer` 3147 -> 3125 (one field), the +22 cascade over 69 rows, or
   leave it.

## Part 7 - Parked, not fixed here

### The save clamp only fires downward

`handleSave` ([TripRow.svelte:482-511](../../src/lib/components/TripRow.svelte))
clamps the odometer only when it is **below** the row's anchor. A stale odometer
at or above the anchor is saved next to a new distance. That produces a visible
span warning on the row, which is what ADR-042 asks for: the chain warns, it
never blocks. It is a follow-up, not a task 80 defect.

### Two integration specs fail on `main` today, independently of task 80

`tests/integration/specs/*/places.spec.ts` and `route-autocomplete.spec.ts` fail
on `main`. The cause is the harness, not this task: `resetDatabase`
([wdio.server.conf.ts:135-175](../../tests/integration/wdio.server.conf.ts))
deletes trips and vehicles only, so the place book survives the whole run and
those two specs depend on which specs ran before them. Cause confirmed by
reading the code. The failure itself was observed during task 4 and was not
re-run here, because a single focused spec cannot reproduce a cross-spec effect.
It will look like a task 80 regression to the next person who runs the suite.
