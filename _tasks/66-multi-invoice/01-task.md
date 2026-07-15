**Date:** 2026-07-13
**Subject:** Multi-invoice support — 1 Fuel + N Other invoices per trip
**Status:** Planning

## Goal

Allow a trip to carry one Fuel invoice AND any number of Other-cost invoices
(parking, wash, toll…) at the same time. Today a trip can hold at most ONE
invoice total: `receipts.trip_id` is `UNIQUE` and `paperless_trip_links.trip_id`
is the PRIMARY KEY, so assigning a fuel-up blocks attaching the parking receipt
for the same ride.

## Requirements

Settled during brainstorming (2026-07-13):

1. **Cardinality:** max 1 Fuel invoice per trip (across BOTH sources — local
   receipt and Paperless combined), unlimited Other invoices per trip.
2. **Trip stays authoritative** (matches existing architecture: trip fields =
   what's reported/exported; invoices = attached proof). Export reads trip
   fields only — no export changes.
3. **Sum-on-assign for Other:** assigning an Other invoice adds its amount to
   `trip.other_costs_eur` (and appends the note); unassigning subtracts it.
4. **Manual overwrite allowed:** user can hand-edit `other_costs_eur` at any
   time; divergence from the sum of attached Other invoices is *surfaced* (grid
   warning), never prevented.
5. **Double-count guard:** if a trip has zero Other invoices and
   `other_costs_eur` already equals the invoice amount (cent-exact, per
   Requirement 6 — supersedes the ±0.01 epsilon used elsewhere today),
   assignment is link-only (the "manually pre-entered, now attaching proof"
   case — today's behavior).
6. **No floating-point errors in money math (HARD requirement from user):**
   all add/subtract on EUR amounts goes through integer-cent arithmetic
   (round-to-cents helper). Repeated assign/unassign cycles MUST yield exact
   results — dedicated unit tests required.
7. **Per-type indicators:** missing-invoice warning becomes per column (fuel /
   other costs), plus a NEW sum-mismatch warning when `other_costs_eur` ≠ sum
   of attached Other invoice amounts.
8. **Docs updated per use-case:** feature doc
   [docs/features/multi-invoice.md](../../docs/features/multi-invoice.md) (new),
   [CHANGELOG.md](../../CHANGELOG.md) entry,
   [DECISIONS.md](../../DECISIONS.md) entry (sum-on-assign is a business rule),
   and any existing feature docs describing single-invoice behavior.
9. **Tests updated accordingly:** backend unit tests for every use-case above,
   one integration spec for the UI flow. Existing tests asserting
   one-invoice-per-trip behavior must be updated, not deleted.

## Technical Notes

- Both blockers are SQLite column constraints → table rebuilds needed
  (SQLite cannot drop a column-level UNIQUE / change a PK in place).
- `paperless_trip_links` also gains `assignment_type` + `amount_eur`/`title`
  snapshots — the grid's sum-mismatch check must work offline
  (`get_trip_grid_data` cannot call the Paperless server).
- Backfill heuristic for existing links (they have no type/amount):
  `Fuel` if linked trip has `fuel_liters > 0` and no Fuel receipt attached,
  else `Other`; `amount_eur = NULL` (unknown → excluded from sum check).
- Unassign is already per-invoice on both paths (`InvoiceRef::Receipt(uuid)`,
  `InvoiceRef::Paperless(doc_id)`) — no change needed there.
- ADR-008: all new logic (sum math, mismatch detection, coverage sets) lives
  in Rust; frontend only renders flags.

See [02-design.md](./02-design.md) for the full design.
