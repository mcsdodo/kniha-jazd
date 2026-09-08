**Date:** 2026-09-08
**Subject:** How task 80 closed
**Status:** Closed

## The three decisions of Part 6

1. **The renumbering.** Approved by the build that runs in production. The
   container runs the image built at 16:33 UTC on 2026-09-08 from commit
   `c818fe7`, which carries the one comparator. The 62 renumbered rows of 2023
   to 2025 are live.
2. **The 2026 odometer correction.** Applied by hand, not by
   `recalculate_odometers`. See
   [79-odometer-span-inconsistency/03-closed.md](../79-odometer-span-inconsistency/03-closed.md).
   Production now raises 0 span warnings and 0 duplicate datetime warnings for
   2026.
3. **The 2023 row 1 warning.** Left open on purpose. The user does not want the
   +22 cascade and does not want to spend time on the one field.

## What is still true and still parked

Part 7 of [03-status.md](03-status.md) stays open:

- `handleSave` clamps the odometer only downward. ADR-042 accepts that: the
  chain warns, it never blocks.
- `places.spec.ts` and `route-autocomplete.spec.ts` fail on `main` because
  `resetDatabase` does not clear the place book. That is a harness defect, not a
  task 80 defect.

`recalculate_odometers` shipped with no UI. It is reached over RPC only. Do not
run it on 2023 or 2025: the table in Part 5 shows what it would rewrite.
