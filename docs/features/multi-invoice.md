# Feature: Multi-Invoice Support (1 Fuel + N Other per Trip)

> A trip can carry one fuel invoice plus any number of other-cost invoices (parking, wash, toll…); assigning an Other invoice adds its amount to the trip's other costs, unassigning subtracts it — cent-exact, with a grid warning when the trip total diverges from the attached sum.

Before this feature a trip could hold at most ONE invoice total (`receipts.trip_id` was `UNIQUE`, `paperless_trip_links.trip_id` was the PRIMARY KEY) — assigning a fuel-up blocked attaching the parking receipt for the same ride.

## User Flow

### Fuel + parking on one ride

1. User drives, fuels up, and pays for parking — one trip, two documents.
2. On the Doklady page, assign the fuel invoice to the trip (Fuel type). The trip's fuel fields auto-populate if empty, as before.
3. Assign the parking invoice to the **same** trip (Other type) — no longer rejected. Its amount lands in the trip's other costs, its note is appended.
4. A second Fuel invoice for the same trip is rejected with „Jazda už má doklad o tankovaní" — across **both** sources (local receipt and Paperless combined). In the trip picker, fuel-covered trips are already greyed out (`can_attach = false`), so this error is only a backstop.

### Multiple Other invoices — sum-on-assign

1. Trip has other costs 10,00 € (one attached invoice). User assigns another Other invoice for 5,01 €.
2. The trip's other costs become exactly 15,01 € — money math goes through integer cents, so repeated assign/unassign cycles are bit-exact (no `10.009999…`).
3. The picker does **not** flag the second invoice as a price mismatch: once a trip already carries at least one Other invoice, the amount comparison is skipped (`✓ matches`) — the new invoice will be summed, there is nothing to "match" against.
4. Unassigning one invoice subtracts exactly what that invoice added; the rest stays.

### Manual overwrite → warning, never a block

1. User hand-edits the trip's other costs to a value different from the sum of attached Other invoices.
2. The edit is allowed (trip stays authoritative — manual entry without documents keeps working).
3. The grid shows a ⚠ in the other-costs column with tooltip „Suma iných nákladov nesedí so súčtom priradených dokladov ({total} € vs {sum} €)" — both numbers computed in the backend.
4. Trips where any attached Other invoice has an unknown amount (legacy Paperless links migrated without amount data) are excluded from this check entirely — no false warnings after updating.

### Double-count guard ("I typed it in first, now attaching proof")

1. Trip already has other costs 12,34 € entered manually, zero Other invoices attached.
2. User assigns an Other invoice for exactly 12,34 € (cent-exact comparison — no ±0,01 tolerance).
3. The invoice is linked **without** adding its amount again — total stays 12,34 €. Any other case (different amount, or an Other invoice already attached) adds the amount normally.

### Unassign — including after editing the invoice price

1. Each assignment stores a snapshot of the exact amount it added to the trip (`applied_amount_cents`).
2. Unassign subtracts **the snapshot**, never the invoice's live price — if the user edited the receipt's price after assigning, the trip total is restored exactly to its pre-assign value (the divergence was meanwhile surfaced by the sum-mismatch ⚠).
3. Link-only assignments (double-count guard, unknown-amount invoices, legacy pre-migration links) have no snapshot and never subtract anything.
4. Subtraction clamps at zero; a trip whose costs drop to 0 stores "no other costs", not "0,00 €". Fuel unassign never touches other costs. Unassigning a receipt whose trip was deleted just clears the link.

### Per-type grid warnings

| Indicator | Condition |
|---|---|
| Fuel column ⚠ missing | trip has fuel (> 0) but no Fuel invoice attached (either source) |
| Other column ⚠ missing | trip has other costs (> 0) but zero Other invoices |
| Other column ⚠ sum-mismatch | other costs ≠ sum of attached Other amounts (cent-exact); skipped when any amount is unknown |
| Datetime ⚠ / override ✓ | scoped per type — an out-of-range *second* Other receipt is detected too |

Zero-value costs (`0,00 €` / `0 l`) are intentionally **not** flagged as missing an invoice.

## Technical Implementation

### Schema (one atomic migration)

[2026-07-15-100000_multi_invoice](../../src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql) rebuilds both tables in a **single migration directory = single transaction** — Diesel runs each directory in its own transaction with no outer one, so two directories could leave a half-migrated DB that panics on every launch. One directory makes the whole feature migration atomic.

- **`receipts`** — drops `trip_id UNIQUE`, adds `applied_amount_cents INTEGER` (snapshot, NULL = link-only/legacy), makes `mismatch_override NOT NULL` (with `COALESCE` on copy so a hand-edited NULL cannot abort the migration).
- **`paperless_trip_links`** — new PK `paperless_document_id` (one trip per doc, unchanged semantics); adds `assignment_type`, `amount_eur` + `title` snapshots (taken at assign time from the backend-fetched doc, so the grid's sum check works offline — the grid never calls the Paperless server), and `applied_amount_cents`.
- **One Fuel per trip** — partial unique indexes (`idx_receipts_trip_fuel`, `idx_paperless_links_trip_fuel`) enforce it within each store; the backend assign pre-check enforces it **across** stores.
- **Backfill heuristic for legacy Paperless links** (no type/amount available offline): `'Fuel'` if the linked trip has `fuel_liters > 0` and no Fuel receipt attached, else `'Other'`; `amount_eur = NULL` (unknown → excluded from the sum check, no false warnings); `applied_amount_cents = NULL` (unassigning a legacy link never mutates trip totals — today's behavior).
- **Orphan healing** — the bundled SQLite enforces foreign keys on every connection, so a hand-edited/restored DB with orphaned rows would abort the migration and brick startup. Orphaned receipts (trip deleted) are healed to unassigned (`trip_id`/`assignment_type` → NULL, everything else preserved); orphaned Paperless links are dropped (matches the `ON DELETE CASCADE` the table has always had).

The migration is covered by a dedicated data-integrity suite in [migration_tests.rs](../../src-tauri/core/src/migration_tests.rs): row/column byte-for-byte preservation, index recreation, backfill edge cases, orphan healing, single-transaction atomicity, and fresh-chain vs. migrated schema parity.

### Cent-exact money math

All EUR add/subtract goes through integer cents — never raw `f64` arithmetic (HARD requirement: repeated assign/unassign cycles must be bit-exact). Helpers in [calculations/mod.rs](../../src-tauri/core/src/calculations/mod.rs): `to_cents` / `from_cents` / `money_add` / `money_sub`. `money_sub` clamps at 0 → stored as "no value". All amount comparisons (double-count guard, sum-mismatch, picker compatibility) are done in cents, replacing the earlier ±0,01 epsilon — so the picker verdict and the assign behavior can never disagree on borderline values.

### Assignment semantics

In [commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) and [commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs), shared by both sources:

```
validate: amount, when present, must be finite and >= 0 (reject before any mutation)
idempotency: already assigned to SAME trip with same type -> no-op
             assigned elsewhere / different type -> reverse old contribution first
             (subtract applied_amount_cents from the old trip), then fresh assign
Fuel: reject if trip coverage shows a Fuel invoice in either store
Other decision table:
    amount unknown (NULL)                       -> link-only, no snapshot
    zero Others AND cents(total) == cents(amount) -> link-only (double-count guard)
    total empty AND zero Others                 -> populate trip, snapshot = amount
    else                                        -> money_add(total, amount), append note,
                                                   snapshot = amount
unassign: Other with snapshot -> money_sub(total, snapshot); 0 -> None
          no snapshot / Fuel / orphaned trip -> clear link only
          note segment stripped only when trivially identifiable
```

### Grid data (ADR-008 — no frontend math)

[TripGridData](../../src-tauri/core/src/models.rs) carries per-type sets: `missing_fuel_invoices`, `missing_other_invoices`, `other_sum_mismatches`, `fuel_datetime_warnings` / `other_datetime_warnings`, `fuel_mismatch_overrides` / `other_mismatch_overrides`, plus `other_invoice_sums` — the attached-sum (EUR) for each flagged trip, shipped pre-computed so the frontend tooltip can render „{total} € vs {sum} €" without doing arithmetic. Coverage comes from `get_trip_invoice_coverage` in [db.rs](../../src-tauri/core/src/db.rs) (union of receipts + Paperless links, per-type flags, sums in cents, unknown-amount flag); warnings computed in [statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs), which iterates **all** receipts of a trip (not just the first).

### Data Flow

```
Assign invoice (either source)
    → validate + idempotency + Fuel cross-source pre-check
    → Other decision table (cents)
    → trip updated (money_add) + snapshot stored on receipt row / paperless link
    → grid refetch: get_trip_grid_data → coverage → per-type warning sets
    → frontend renders flags + tooltip only
```

## Key Files

| File | Purpose |
|------|---------|
| [src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql](../../src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql) | Atomic double rebuild + backfill + orphan healing |
| [src-tauri/core/src/calculations/mod.rs](../../src-tauri/core/src/calculations/mod.rs) | Cent-exact money helpers |
| [src-tauri/core/src/commands_internal/invoices.rs](../../src-tauri/core/src/commands_internal/invoices.rs) | Unified assign/unassign dispatch, decision table |
| [src-tauri/core/src/commands_internal/receipts_cmd.rs](../../src-tauri/core/src/commands_internal/receipts_cmd.rs) | Receipt-path assignment |
| [src-tauri/core/src/invoice.rs](../../src-tauri/core/src/invoice.rs) | Multi-invoice-aware compatibility check + `can_attach` |
| [src-tauri/core/src/db.rs](../../src-tauri/core/src/db.rs) | `get_trip_invoice_coverage`, paperless link CRUD (`upsert_paperless_link`, `get_paperless_link`, `get_paperless_links_for_trip`) |
| [src-tauri/core/src/commands_internal/statistics.rs](../../src-tauri/core/src/commands_internal/statistics.rs) | Per-type warning sets + sum-mismatch |
| [src-tauri/core/src/migration_tests.rs](../../src-tauri/core/src/migration_tests.rs) | Migration data-integrity suite |
| [src/lib/components/TripRow.svelte](../../src/lib/components/TripRow.svelte) | Per-column warning rendering |
| [tests/integration/specs/tier2/multi-invoice.spec.ts](../../tests/integration/specs/tier2/multi-invoice.spec.ts) | UI flow: assign 2 Others, sum display, mismatch ⚠ |

## Design Decisions

- **Why does the trip stay authoritative?** — Manual entry without documents must keep working; export reads trip fields only and is untouched. Invoices are attached proof, not the source of the numbers. (See [BIZ-023](../../DECISIONS.md).)
- **Why a snapshot (`applied_amount_cents`) instead of subtracting the live price on unassign?** — The user may edit the invoice price after assigning; subtracting the live value would corrupt the trip total. The sum-mismatch indicator (which DOES use live values) is what surfaces such edits.
- **Why integer cents?** — `0.1 + 0.2 ≠ 0.3` in `f64`; repeated assign/unassign must round-trip exactly (see [BIZ-023](../../DECISIONS.md)).
- **Why one migration directory?** — Diesel gives one transaction per directory; splitting the two rebuilds risks a permanently half-migrated DB.
- **Why is the sum check skipped for unknown amounts?** — Legacy Paperless links have no amount data offline; a partial sum would produce false warnings for every upgraded user.
- **Why max 1 Fuel invoice?** — A trip has exactly one fill-up by design.

## Out of Scope

- Deriving `other_costs_eur` from invoices (rejected — manual entry must keep working).
- N Fuel invoices per trip.
- `mismatch_override` persistence for Paperless links (pre-existing gap, unchanged — [ADR-021](../../DECISIONS.md)).

## Related

- [BIZ-023 in DECISIONS.md](../../DECISIONS.md) — cardinality, sum-on-assign, cent-exact math, snapshot rule.
- [ADR-019 in DECISIONS.md](../../DECISIONS.md) — superseded: the paperless link table shape changed here.
- [unified-invoice-picker.md](./unified-invoice-picker.md) — the picker flow this feature extends.
- [paperless-integration.md](./paperless-integration.md) — Paperless invoice source.
- [_tasks/66-multi-invoice/](../../_tasks/66-multi-invoice/) — original planning docs.
