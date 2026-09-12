**Date:** 2026-09-11
**Subject:** Code review of task 84 (branch `feat/84-paperless-only-invoices`)
**Status:** Review complete; all findings resolved
**Range:** `2d3d6200..7cc5eb99` (12 commits, 98 files, +3888 / -16812)

# 84 -- Code Review

Three reviewers ran in parallel, split by behavior thread so each one spanned
Rust -> RPC -> Svelte. All claims below were verified against the code, or
against a read-only copy of the production database.

## Verification Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 683 passed, 0 failed |
| `npm run test:integration` (full) | 33 of 33 spec files passed, 5 min 54 s |
| `npm run check` (svelte-check) | 0 errors, 11 pre-existing CSS warnings |
| `npm run i18n` drift | `i18n-types.ts` current, not stale |
| Removed-symbol grep | no hits in code, tests, CI, compose, sample settings |
| Upgrade on a prod copy | all 3 migrations applied, server started, `integrity_check` ok, `foreign_key_check` clean |

## Production Upgrade Rehearsal

The production database was copied read-only and upgraded with the real
`kniha-jazd-web` binary. Nothing was written back.

| Check | Result |
|---|---|
| Migrations applied | `20260911120000`, `20260911125000`, `20260911130000` |
| `receipts` table | dropped |
| New link columns | `receipt_datetime`, `mismatch_override` present |
| Repair migration effect | Fuel links 7 -> 18 (11 repaired), exactly as predicted |
| False "missing fuel invoice" warnings | 67 -> 56 (11 removed) |
| Pre-migration backup | created automatically, holds all 68 receipt rows |
| Receipts lost by the drop | 51 of 68 (the unassigned ones); all 17 trip-assigned receipts survive as links |

The repair migration does what it claims on real data.

## Blocking Issue

### C1 -- The documented rescue path is false, and cannot run

Requirement 5 keeps [scripts/migrate_local_to_paperless.py](../../scripts/migrate_local_to_paperless.py) as the only escape
from an irreversible `DROP TABLE receipts`. Three defects, all verified:

1. **It never writes to Paperless.** The script has one HTTP call site
   ([scripts/migrate_local_to_paperless.py:53-54](../../scripts/migrate_local_to_paperless.py)), and it is a GET. Its own
   docstring states the real behavior: it finds a document that is *already in
   Paperless* and inserts `paperless_trip_links` rows. A user whose local
   receipts were never uploaded loses them either way.
2. **It crashes on Linux before parsing arguments.**
   `APPDATA = Path(os.environ["APPDATA"])` at module scope (line 32).
   `env -u APPDATA python3 scripts/migrate_local_to_paperless.py --help` raises
   `KeyError: 'APPDATA'`. Linux is the only supported deployment (ADR-030), and
   the production database is on a Linux host. `--db-path` cannot rescue it,
   because argparse never runs.
3. **Its INSERT silently writes zero rows.** Line 350 omits `assignment_type`,
   which `2026-07-15-100000_multi_invoice` made `TEXT NOT NULL` with no default.
   `INSERT OR IGNORE` swallows the violation. Reproduced on a scratch DB with
   the live schema: 0 rows written, exit 0, success message printed.

Defects 2 and 3 are pre-existing rot. Task 84 made them load-bearing.

**What is actually lost on production, measured.** The prod copy holds 68
receipt rows. 17 are assigned to a trip, and all 17 survive the upgrade as
`paperless_trip_links` rows -- 0 assigned receipts are unrepresented afterwards.
The loss is exactly the **51 unassigned receipts** (`trip_id IS NULL`).

**The script cannot save any of those 51, even fully repaired.** Its query is
`FROM trips t INNER JOIN receipts r ON r.trip_id = t.id`
([scripts/migrate_local_to_paperless.py:268-269](../../scripts/migrate_local_to_paperless.py)),
so it only ever considers trip-assigned receipts. It covers precisely the 17
rows that need no saving, and none of the 51 that are the real loss.

That is the sharpest form of this finding: the docs point the user at a rescue
that does not run on their platform, writes zero rows when it does, and by
construction does not cover the data that actually disappears.

Four documents written in this range claim a capability the script does not have:

| File | Claim |
|---|---|
| [CHANGELOG.md:23](../../CHANGELOG.md) | "skript ich prenesie do Paperless-ngx" |
| [docs/features/server-mode.md:73](../../docs/features/server-mode.md) | "so the rows reach Paperless first" |
| [docs/features/paperless-integration.md:109](../../docs/features/paperless-integration.md) | "which writes the old rows into Paperless" |
| [DECISIONS.md:18](../../DECISIONS.md) (ADR-050 item 4) | "moves old local receipts into Paperless" |

[README.md:128-131](../../README.md) and [README.en.md:127-129](../../README.en.md) say only "run it BEFORE
upgrading" and are accurate.

**Fix, either option:**

- Repair the script: guard the `APPDATA` read behind a platform check, add
  `assignment_type` to the INSERT, drop `OR IGNORE` so a constraint failure is
  loud. Then correct the four claims to say it *relinks documents already in
  Paperless*.
- Or leave the script and correct all four sites to state plainly that it only
  relinks already-uploaded documents and runs on Windows only.

Either way, fix [CHANGELOG.md:23](../../CHANGELOG.md) before `/release` -- that entry reaches users.

**The pre-migration backup is the only real recovery path, and it is not a
restore.** `Database::new`
([src-tauri/core/src/db.rs:71](../../src-tauri/core/src/db.rs)) writes
`<DATA_DIR>/backups/kniha-jazd-backup-*-pre-migration-*.db` whenever migrations
are pending, and `get_cleanup_candidates`
([src-tauri/core/src/commands_internal/backup.rs:151-155](../../src-tauri/core/src/commands_internal/backup.rs))
never prunes it. The rehearsal confirms it is created and holds all 68 rows.

But restoring it into a post-84 build does **not** bring the receipts back. I
tested this: the restored file has 68 receipts, and after the post-84 server
opens it once, `receipts` is gone again -- the three migrations are absent from
`__diesel_schema_migrations`, so diesel simply re-runs them and re-drops the
table.

So document the backup as a file to read offline with `sqlite3`, never as a
restore path. Do not write "restore the backup to get your receipts back" -- that
would add a fourth false rescue claim to the three this finding is about.

## Important

### I1 -- The nav badge is never invalidated after assign or unassign

[src/lib/components/InvoiceIndicator.svelte:11-21](../../src/lib/components/InvoiceIndicator.svelte) refreshes on mount, on a
300 s interval, and on vehicle/year change. At base, `triggerReceiptRefresh()`
was called from `doklady/+page.svelte:131` and `TripGrid.svelte:516,596`;
`src/lib/stores/receipts.ts` was deleted with no replacement.

The poll also went 30 s -> 300 s, so the stale window is ten times wider.
Assigning the last unlinked fuel invoice leaves a red badge for up to five
minutes -- the exact workflow the badge exists to close.

This is a plan gap, not a coding slip: `03-plan.md:1052-1100` specifies the
component as implemented, and the plan deletes the store without naming a
replacement.

**Fix:** keep a one-line `invoiceRefreshTrigger` store and fire it from
`handleAssignInvoice`, `handleConfirmUnassign`, and `handleClearOverride`.

### I2 -- The repair migration is not hardened against a second candidate link

`migrations/2026-09-11-125000_retype_fuel_links_from_receipts/up.sql:27-46`
uses an uncorrelated `trip_id IN (SELECT ...)`, so every qualifying `Other` link
on a matching trip is promoted at once. The partial unique index
`idx_paperless_links_trip_fuel` (`2026-07-15-100000_multi_invoice/up.sql:150`)
then rejects the second row, and `db.rs:90` calls `.expect(...)` -- a startup
panic.

**This does not fire on production.** The diagnostic returned no trip with more
than one candidate link, and the rehearsal upgraded cleanly. The shape needs a
restored or hand-edited DB, because the pre-Task-66 table had
`trip_id TEXT PRIMARY KEY`.

So this is hardening, not a live crash. But the test that claims to cover it is
vacuous: `migration_tests.rs:500 repair_does_not_add_a_second_fuel_link` seeds
one link, so it cannot fail and does not test its own name. Fix the test using
`open_db_legacy_before("2026-09-11-125000")` (`db.rs:1256` was parameterised for
exactly this), and add a `MIN(paperless_document_id)` predicate to the SQL.

### I3 -- Migration 125000 panics on a DB built inside a four-hour window

`125000/up.sql:39` reads `receipts`; `130000/up.sql:5` drops it. The drop landed
in `7769ba9` and the repair in `e708b78`, later the same day. A database built
from a commit in between has 130000 applied and 125000 pending, and diesel then
runs the repair against a missing table: `no such table: receipts`.

Production runs `:main` and has never seen 130000, and CI starts from an empty
volume, so only scratch dev DBs are affected.

**Fix while the branch is unmerged:** fold the UPDATE into
`2026-09-11-130000_drop_receipts/up.sql` immediately before `DROP TABLE`, and
delete the 125000 folder. This does not break the "never edit historical
migrations" rule, because no shipped build contains it.

### I4 -- The repair can retype an Other document as the fuel invoice

`125000/up.sql:38-41` keys on "the trip has a Fuel receipt", not on "this link
is the fuel document". A trip with both a Fuel and an Other receipt produced two
candidate inserts in the old script, and the old `trip_id` primary key let only
the first win.

**This does not fire on production either:** the ambiguous subset is 0 rows.
Add `AND NOT EXISTS (... r2.assignment_type='Other')` to skip ambiguous trips,
or record the accepted risk in the migration comment.

### I5 -- `get_invoice_source_mode` survives, though the design says it was removed

`server/dispatcher.rs:820-821` -> `commands_internal/integrations.rs:391-402`,
plus seven `InvoiceSourceMode::Local` assertions in `integrations_tests.rs:332-382`.

Confirmed dead: nothing in `src/` calls it, and the only Rust caller is the
dispatcher arm itself. The Doklady page probes configured state through
`get_paperless_settings` (`api.ts:431`). The `Local` variant now means only
"Paperless is not configured", which its name contradicts.

**Fix:** delete the arm, the enum, both helpers, and the seven assertions -- or
amend the design's Command Surface table if it is kept deliberately.

### I6 -- Requirement 9's losses table is incomplete

`02-design.md:84-100` lists six accepted losses. Three more are real and
unstated: badge freshness (I1); historical link state (no backfill, so every
pre-existing link keeps `receipt_datetime = NULL` and `mismatch_override = 0`
forever -- and migration 125000 proves the join was feasible in that window); and
no in-app remedy for a datetime mismatch (below).

### I7 -- An out-of-range datetime on an empty trip warns with no way to dismiss

`check_paperless_trip_compatibility` never returns `Differs` when
`!trip_has_fuel` (`invoice.rs:66-77`), and the modal gates the override buttons
on `attachmentStatus === 'differs'` (`TripSelectorModal.svelte:296`). But
`calculate_invoice_datetime_warnings` flags the link on the snapshot alone
(`statistics.rs:1419-1427`).

The common case -- fill up at 20:00 after a trip that ended at 17:00 -- warns
permanently. `ReceiptEditModal.svelte` was the old in-app fix and is deleted, so
the only remedy is edit-in-Paperless, unassign, reassign.

### I8 -- CHANGELOG and ADR-050 omit the third migration and the override feature

ADR-050 item 3 and [docs/features/multi-invoice.md:94-98,165-166](../../docs/features/multi-invoice.md) describe a
two-migration sequence; there are three, and the order is load-bearing. The
repair is also a user-visible data correction (11 false warnings disappear on
production) with no `### Opravené` entry.

`### Pridané` says nothing about the persisted mismatch override, its revert
control, or datetime warnings on Paperless invoices -- all new user-visible
behavior that ADR-050 itself calls out as closing the ADR-021 gap.

## Minor

- Three i18n keys are now dead: `trips.noReceipt`, `trips.receiptDatetimeMismatch`,
  `trips.receiptDatetimeMismatchWithRange`, `trips.legend.noReceipt`. Remove from
  `sk` and `en`, then run `npm run i18n`.
- Superseded ADRs carry no inline marker. The file shows the convention three
  times (`:168`, `:232`, `:658`). ADR-021 now reads as current when ADR-050 just
  built it.
- [_tasks/index.md:76](../../_tasks/index.md) shows tech-debt 05 as Resolved; its header says Obsolete.
- [CHANGELOG.md](../../CHANGELOG.md) section order: `Odstránené` sits before `Opravené`. The skill
  fixes the order as Pridané -> Zmenené -> Opravené -> Odstránené.
- `invoices.rs:110-115`: the idempotent early return discards a changed
  `mismatch_override`. Not reachable from the UI, but an RPC-surface inconsistency.
- `InvoiceIndicator.svelte`: a vehicle switch can issue up to three full Paperless
  fetches (`+layout.svelte:92-108` writes the store three times); no in-flight
  sequence guard, so counts can resolve out of order; `catch { count = 0 }` makes
  an outage look like "all linked" -- at least log it.
- `statistics.rs:1418,1440`: `l.trip_id == trip.id.to_string()` allocates inside
  the inner loop. The receipt version compared `Uuid` to `Uuid`. Hoist it.
- `doklady/+page.svelte:45-46` detects the empty state by substring-matching an
  error message; only the `'not configured'` clause ever matches. Rewording
  `paperless.rs:16-17` silently turns the setup state into a raw error banner.
- `TripSelectorModal.svelte:36`: `hasMismatch` is a `$derived` returning a
  function, referenced nowhere. Always truthy if someone uses it. Delete.
- `migration_tests.rs:466-511` does not cover the ambiguous case, the `created_at`
  boundary, or a negative proving a post-Task-66 link with snapshots is never
  promoted -- that guard is the migration's main safety property.
- [Dockerfile.web:35](../../Dockerfile.web): the `touch` is load-bearing (it defeats the cached stub
  layer's fingerprint) but reads as redundant. Add a comment so nobody removes it.
- Stale doc counts: `server-mode.md:150` says "74 sync and 15 async" (actual 63
  and 14); [ARCHITECTURE.md:88](../../ARCHITECTURE.md) says 68 sync.
- [tests/integration/README.md:107-116](../../tests/integration/README.md) and the invariant I1 paragraph in
  [CLAUDE.md](../../CLAUDE.md) still describe `TIER` env vars and a Windows `set VAR=` form; CI
  now uses `WDIO_SHARD` with `cross-env` (task 83 drift).
- [local.settings.json.sample](../../local.settings.json.sample) collapsed to `{}`; [DECISIONS.md:531](../../DECISIONS.md) still links
  to it as the settings example. Add the live optional keys.

## What Was Done Well

- The deletion is complete and consistent. The abstraction, models, schema, RPCs,
  env vars, and test wiring went away together. Grep finds no dangling symbol,
  and only one dead dispatcher arm survives.
- `upsert_paperless_link` (`db.rs:863`) is the single writer and always writes
  both new columns, so the override cannot be silently dropped. Re-assign
  overwrites rather than carrying a stale confirmation forward.
- Both new commands are read-only gated (`invoices.rs:298-305`, `:310`).
- NULL snapshots cannot produce a false warning: the calculator skips `None`
  (`statistics.rs:1419`), the migration defaults existing rows, and an
  unparseable stored datetime degrades to `None` (`db.rs:1216-1218`).
- The Fuel/Other split survives end to end, all four `TripGridData` fields
  intact through to `TripRow.svelte`.
- The repair migration's guard set is precisely "written by the Task 66
  backfill", and its comment block explains the reasoning honestly.
- No ADR-008 violation, and no N+1 against the Paperless API.
- Tech-debt 09 records all six design losses with a Paperless-side path.
- Shard balance holds: 33 spec files over 6 shards, enumerated dynamically, no
  orphaned spec.
- The Dockerfile stub layer is correct: `touch` dirties both crate roots,
  `rm -rf core/src web/src` runs in the same `RUN`, and dropping `|| true` makes
  a broken stub fail loudly.

## Verdict

**Ready to merge: with fixes.**

The engineering is sound and both test gates are green. The repair migration was
the main risk and it is verified safe on the real database: it applies cleanly,
fixes 11 false warnings, and leaves the DB integrity-clean.

One item blocks, and it is a documentation defect rather than a code defect: the
rescue path documented for an irreversible `DROP TABLE` does not run on Linux,
writes zero rows when it does, and by construction covers only trip-assigned
receipts -- while the 51 rows actually lost on production are the unassigned
ones. Four documents claim it moves receipts into Paperless; it never writes to
Paperless at all.

Nothing here requires a code change to ship safely. It requires the docs to
describe what the upgrade really does: 51 unassigned local receipts are
discarded, the script does not save them, and the pre-migration backup is a file
to read offline. [CHANGELOG.md:23](../../CHANGELOG.md) must be corrected before
`/release`, because that entry is what reaches users.

If the user wants those 51 rows preserved, that is a separate decision to make
before the drop ships -- export them to CSV, or upload the documents to
Paperless first.

Suggested order: C1, then I3 (it changes where the I2 and I4 edits land), then
I1, then the CHANGELOG/DECISIONS pass (I8, I6), then fold the Minor items into
one cleanup commit.

## Resolution (2026-09-11)

Every finding above was fixed on the branch. Commits `ba9decf`, `949fae9`,
`ebae65a`, `5e36299`, `62e93af`.

| Finding | Resolution |
|---|---|
| C1 rescue script | **Script deleted** at the user's direction, not repaired. All six documents now state the real upgrade path: 51 unassigned receipts are discarded, with the export command. The pre-migration backup is documented as an offline `sqlite3` read, never a restore. |
| I1 nav badge | `src/lib/stores/invoices.ts` restores the trigger; fires on assign, unassign, trip save and trip delete. |
| I2 repair UNIQUE violation | `MIN(paperless_document_id)` predicate added. The vacuous test now stands at the repair's own boundary and was proven to fail without the guard (`UniqueViolation`). |
| I3 migration ordering | Repair folded into `2026-09-11-130000_drop_receipts`; the `125000` folder is gone. |
| I4 ambiguous retype | Trips carrying both a Fuel and an Other receipt are skipped. |
| I5 dead command | `get_invoice_source_mode`, both helpers, the enum and 7 tests removed. |
| I6 losses table | Gap item 10 added to the design; tech-debt item 7 added. Two of the three found losses were fixed rather than accepted. |
| I7 undismissable warning | Both empty-trip branches share `datetime_only_result`; the picker gates the override on the mismatch reason, so `matches_date` survives and is confirmable. |
| I8 CHANGELOG / ADR | Entries added for the persisted override, Paperless datetime warnings and the link repair; section order fixed; 4 superseded ADRs marked inline. |
| Minor items | All applied: dead i18n keys, `_tasks/index.md` row, settings sample, stale command counts, override-on-reassign, allocation hoist, `NotConfigured` marker, dead `hasMismatch` rune, `Dockerfile.web` comment, task-83 sharding drift. |

### Verification after the fixes

| Gate | Result |
|---|---|
| `cargo test --workspace` | 681 passed, 0 failed |
| `npm run test:integration` (full) | 33 of 33 spec files, no retries |
| `npm run check` | 0 errors |
| `npm run i18n` | idempotent, no drift |

### Production rehearsal, on the real database

Synced with [data/sync-from-prod.sh](../../data/sync-from-prod.sh) and run under
[docker-compose.web.yml](../../docker-compose.web.yml).

| Check | Result |
|---|---|
| Container start | healthy, no migration panic |
| Migrations applied | `20260911120000`, `20260911130000` |
| `receipts` | dropped |
| Repair effect | Fuel links 7 -> 18; false "missing fuel invoice" 67 -> 56 |
| `integrity_check` / `foreign_key_check` | ok / clean |
| Pre-migration backup | `sha256 f5090af4...`, byte-identical to the synced prod file |
| App over RPC | v0.44.0, Normal mode, grid renders 111 trips (2026) and 84 (2024) |
| `count_unlinked_paperless_fuel_invoices` | returns 1 against the live Paperless API |

The backup is an untouched copy. The migrated database differs from it in exactly
the three intended ways: two new columns, 11 links retyped, `receipts` gone.

## CodeRabbit review on PR #9 (2026-09-12)

Three findings, all rated Major. Two are real. Each one was checked against the
code before any edit.

### R1 -- the repair keyed on a calendar date (valid, minimal fix)

`2026-09-11-130000_drop_receipts/up.sql` selected backfilled links with
`created_at < '2026-07-16'`. That predicate is wrong, for the reason CodeRabbit
gives: the multi-invoice backfill **copies** the old link timestamp
([2026-07-15-100000_multi_invoice/up.sql:141](../../src-tauri/core/migrations/2026-07-15-100000_multi_invoice/up.sql)),
it does not stamp the time it runs. A database that applies `multi_invoice` late
carries backfilled rows with a recent `created_at`; the repair skipped them, and
the `DROP TABLE receipts` below then took the trip's fuel coverage with it.

CodeRabbit asked for a new durable marker and called it a heavy lift. One
already exists: **`title IS NULL`**.

- The backfill writes `NULL` into the title slot (`multi_invoice/up.sql:139`).
- `upsert_paperless_link` ([db.rs:880](../../src-tauri/core/src/db.rs)) always
  writes the document title, and it is the only INSERT into the table.
- Nothing later nulls it: `set_paperless_override` updates two columns, unassign
  deletes the row.

So the predicate is now `title IS NULL`, with the amount columns kept as a second
guard. No new column, no schema change.

The amount guard alone cannot do this job: a document Paperless read no amount
from leaves `amount_eur` and `applied_amount_cents` NULL exactly like the
backfill (`apply_other_amount` returns `None`), so the date was the only thing
separating those rows. The title separates them durably.

Two tests, both proven to fail without the fix:

| Test | Proves |
|---|---|
| `delayed_upgrade_still_repairs_a_backfilled_link` | an August link on the old schema is still repaired after both migrations run. Fails with the date predicate restored (`Other`, expected `Fuel`). |
| `repair_skips_an_app_written_link_without_amounts` | a link the app wrote, with no amount snapshots, is never retyped. Fails with no marker at all (`Fuel`, expected `Other`). |

`repair_covers_a_link_created_on_the_multi_invoice_day` and
`repair_skips_a_link_created_after_the_backfill` asserted the calendar semantics
and were replaced by those two.

**This had to land before the merge.** A follow-up migration cannot do the
repair: it reads `receipts`, which this migration drops. That is the same trap as
I3 above. Production runs `:main` and has seen neither migration, so the in-place
edit is safe.

### R2 -- `MIN()` resolves an ambiguous candidate set (no change)

CodeRabbit asks to skip trips with more than one candidate instead of taking the
lowest document id. Both branches are unreachable: the pre-multi-invoice table
was `trip_id TEXT PRIMARY KEY`
([2026-05-03-100000_add_paperless_trip_links/up.sql:2](../../src-tauri/core/migrations/2026-05-03-100000_add_paperless_trip_links/up.sql)),
so the backfill can emit at most one legacy link per trip, and that stays true
under `title IS NULL`. The production diagnostic found no such trip (I2 above).

`MIN()` is kept because it is already proven to prevent the `UniqueViolation`
that aborts the whole upgrade, and `repair_does_not_add_a_second_fuel_link`
asserts it. The only argument for the alternative is a failure mode neither
version can reach.

### R3 -- the datetime mismatch reason was still missing in two branches (valid)

I7 above was fixed for the `!trip_has_fuel` and `!trip_has_other_costs` pair, but
two branches in `check_paperless_trip_compatibility` still returned
`mismatch_reason: None` without looking at the datetime:

- `coverage.has_other` is true -- a second parking or toll document on one trip.
- `doc.total_amount` is `None` while the trip has other costs -- a document
  Paperless read no amount from.

Both are reachable, and the invariant holds for them too:
[invoices.rs:178](../../src-tauri/core/src/commands_internal/invoices.rs)
snapshots the datetime on every assign, and
[statistics.rs:1424](../../src-tauri/core/src/commands_internal/statistics.rs)
warns on `Other` links as well as `Fuel`. The picker gates the confirm button on
the reason ([TripSelectorModal.svelte:292](../../src/lib/components/TripSelectorModal.svelte)),
so the grid warning could not be confirmed away.

Both branches now route through `datetime_only_result`, which keeps the
amount-comparison skip. Four tests were added, two of which failed before the
fix. The PR description claimed this was already fixed; it was corrected.

One picker label changes with it: a second Other document carrying **no**
datetime used to show the green "sedí s dokladom" tick, an agreement that was
never checked (the amount comparison is skipped in that branch and there was no
datetime to compare). It now shows no badge, like the three sibling branches with
nothing to compare. No grid warning and no stored value is affected -- nothing is
snapshotted when the document has no datetime.

### R4 -- flaky spec found while verifying (fixed)

`paperless-integration.spec.ts:194` read the trip list immediately after the
click that opens the picker. The modal fetches its trips over RPC after it
opens, so the list starts empty and the assertion failed on an empty list. The
full sweep hid it behind a retry.

Measured, with the picker fix reverted to prove it is not a side effect of R3:
**3 failures in 6 runs** on the unchanged code. With a `waitUntil` on the first
row: **8 passes in 8 runs**. Every other step in the spec already waits this way.

### Verification after the CodeRabbit fixes

| Gate | Result |
|---|---|
| `cargo test --workspace` | 685 passed, 0 failed |
| `npm run check` | 0 errors |
| `npm run test:integration` (full) | 33 of 33 spec files, 1 retry -- the R4 flake, fixed after the run |
| `paperless-integration` spec, 8 consecutive runs | 8 passing, no retry |

Rehearsal repeated against a fresh production copy (`sha256 f5090af4...`,
byte-identical to the one used on 2026-09-11), so the numbers below describe the
new predicate and not the old one:

| Check | Before | After |
|---|---|---|
| Fuel links | 7 | 18 |
| False "missing fuel invoice" | 67 | 56 |
| `receipts` | 68 rows | dropped |
| `integrity_check` / `foreign_key_check` | -- | ok / clean |

Backfilled links on production carry `created_at` before the old cutoff
(`title IS NULL AND created_at >= '2026-07-16'` selects 0 rows), so both
predicates repair the same 11 rows there. The change matters only for a database
that upgrades late.
