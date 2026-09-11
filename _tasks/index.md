# Task Index

Quick overview of all tasks and their status.

**Last updated:** 2026-09-11 ([Task 84](84-paperless-only-invoices/) is implemented on `feat/84-paperless-only-invoices` and in progress, not merged: local receipts + Gemini OCR removed, Paperless is the only invoice source, the `receipts` table is dropped on upgrade. Tech debt [09](_TECH_DEBT/09-paperless-ocr-capability-gaps.md) records the six accepted capability losses. [Task 83](_done/83-integration-test-sharding/) archived: the shard matrix is on `main` and the integration stage fell from 5m26 to 2m37. It merged ahead of [Task 82](82-integration-db-reset/), which was declared to block it, so 82 is now live correctness risk rather than a gate.)

## Active Tasks

| # | Task | Status | Notes |
|---|------|--------|-------|
| 84 | [Paperless-Only Invoices](84-paperless-only-invoices/) | 🟡 In Progress | Implemented on branch `feat/84-paperless-only-invoices`; removes local receipts + Gemini OCR, makes Paperless the only invoice source, drops the `receipts` table. Not merged yet |
| 82 | [Integration DB Reset](82-integration-db-reset/) | 📋 Planning | One guarded backend command resets every table + local.settings.json; correctness, not speed. [Task 83](_done/83-integration-test-sharding/) shipped without it, so the order risk is live on `main` |
| 57 | [Invoice to Trip](57-invoice-to-trip/) | 📋 Planning | Create trip from fuel invoice (mid-trip split helper) + origin auto-fill |
| 32 | [Portable CSV Backup](32-portable-csv-backup/) | 📋 Planning | Cross-platform backup format |

## Completed Tasks

| # | Task | Completed |
|---|------|-----------|
| 83 | [Integration Test Sharding](./_done/83-integration-test-sharding/) -- the CI matrix shards the specs six ways instead of by tier; integration stage 5m26 -> 2m37 on `main`. The under-2m30 criterion was not met and is closed as unattainable by a round-robin split; the duration-weighted split is the open follow-up, see [03-results.md](./_done/83-integration-test-sharding/03-results.md) | 2026-09-11 |
| 41 | [Integration Test Speedup](./_done/41-integration-test-speedup/) -- archived unbuilt: written for the Tauri harness that [Task 73](./_done/73-web-first-migration/) deleted, and its speedup premise measured false (the reset it replaced costs 17 ms per test); the surviving work is [Task 82](82-integration-db-reset/), see [04-superseded.md](./_done/41-integration-test-speedup/04-superseded.md) | 2026-09-09 |
| 78 | [Round Trip Legs and Distance Write-Back](./_done/78-round-trip-legs-and-distance-writeback/) -- a round trip routes as two independent legs with a picker each, and the routed distance can be written back to the trip behind a fuel-period and legal-margin warning; see [docs/features/route-maps.md](../docs/features/route-maps.md) | 2026-09-09 |
| 81 | [Odometer Cascade On Save](./_done/81-odometer-cascade-on-save/) -- an edit, an insert and a delete cascade the odometer to every later row of the year, behind a confirmation modal; see [ADR-046](../DECISIONS.md#adr-046-a-save-cascades-the-odometer-by-delta-a-rebase-never-runs-on-its-own) | 2026-09-09 |
| 80 | [One Trip Ordering](./_done/80-one-trip-ordering/) -- the renumbering is live in production, see [04-closed.md](./_done/80-one-trip-ordering/04-closed.md) | 2026-09-08 |
| 79 | [Odometer Span Inconsistency](./_done/79-odometer-span-inconsistency/) -- the 2026 rows are corrected in production, 2023 stays as it is, see [03-closed.md](./_done/79-odometer-span-inconsistency/03-closed.md) | 2026-09-08 |
| 72 | [Route Map Origin/Destination](./_done/72-route-map-origin-destination/) — see [docs/features/route-maps.md](../docs/features/route-maps.md) | 2026-09-07 |
| 77 | [Linux Dev Environment](./_done/77-linux-dev-environment/) | 2026-09-07 |
| 75 | [Place Book](./_done/75-place-book/) — see [docs/features/place-book.md](../docs/features/place-book.md) | 2026-09-07 |
| 76 | [Route Usage Counter Drift](./_done/76-route-usage-counter-drift/) | 2026-09-07 |
| 74 | [Main-Branch Image Channel](./_done/74-main-branch-image-channel/) | 2026-09-04 |
| 73 | [Web-First Migration](./_done/73-web-first-migration/) | 2026-09-04 |
| 71 | [Copy Trip Row](./_done/71-copy-trip-row/) | 2026-09-03 |
| 70 | [Route Map Integration](./_done/70-route-map-integration/) | 2026-08-10 |
| 67 | [Online Always-On Runner](./_done/67-online-always-on-runner/) | 2026-09-03 |
| 61 | [Route Map POC](./_done/61-route-map-poc/) — graduated by [Task 70](./_done/70-route-map-integration/) | 2026-08-10 |
| 69 | [PIN-Gated Secret Reveal](./_done/69-pin-gated-secret-reveal/) | 2026-08-10 |
| 68 | [Env-Managed Settings UI](./_done/68-env-managed-settings-ui/) | 2026-08-10 |
| 52 | [HA Suggested Fillup Push](./_done/52-ha-suggested-fillup-push/) | 2026-02-11 |
| 66 | [Multi-Invoice Support](./_done/66-multi-invoice/) | 2026-07-15 |
| 65 | [Datetime Is Order](./_done/65-datetime-is-order/) | 2026-05-21 |
| 64 | [Unified Invoice Picker](./_done/64-unified-invoice-picker/) | 2026-05-04 |
| 63 | [Paperless Configurable Fields](./_done/63-paperless-configurable-fields/) | 2026-05-04 |
| 62 | [Paperless Toggle](./_done/62-paperless-toggle/) | 2026-05-04 |
| 60 | [Paperless Integration](./_done/60-paperless-integration/) | 2026-05-03 |
| 59 | [Time Inference Toggle](./_done/59-time-inference-toggle/) | 2026-04-27 |
| 58 | [Tauri Workspace Split](./_done/58-tauri-workspace-split/) | 2026-04-26 |
| 33 | [Web Deployment](_done/33-web-deployment/) | 2026-04-26 |
| 55 | [Server Mode](_done/55-server-mode/) | 2026-04-25 |
| 56 | Smart Trip Defaults | 2026-04-16 |
| 54 | Fix Odometer Recalculation Bugs | 2026-03-04 |
| 53 | HA Real Fuel Level | 2026-02-12 |
| 50 | [Receipt Datetime Validation](50-receipt-datetime-validation/) | 2026-02-11 |
| 51 | [Receipt State Model](./_done/51-receipt-state-model/) -- explicit assignment replaced the auto-matching: the user picks FUEL or OTHER, `trip_id` alone means assigned, and a data mismatch is a warning the user can confirm; shipped in 0.29.0, archived 2026-09-09 | 2026-02-04 |
| 49 | [Claude Rules Restructuring](49-claude-rules-restructuring/) | 2026-02-01 |
| 48 | end_datetime Cleanup | 2026-01-31 |
| 47 | [Datetime Consolidation](47-datetime-consolidation/) | 2026-02-11 |
| 46 | Legal Requirements Updates | 2026-01-31 |
| 45 | DB Backup When Updating | 2026-01-24 |
| 44 | Multi-Currency Receipts | 2026-01-21 |
| 43 | ODO Recalculation Bug | 2026-01-21 |
| 42 | [Commands Module Split](42-commands-module-split/) | 2026-02-11 |
| 40 | Home Assistant ODO | 2026-01-29 |
| 39 | Trip Time + Hideable Columns | 2026-01-29 |
| ... | (older tasks in [_done/](./_done/)) | ... |

> **Note:** Task numbers can be reused. Check BOTH [_tasks/](.) and [_tasks/_done/](./_done/) folders to find the next available number.

## Tech Debt

| # | Item | Priority | Status |
|---|------|----------|--------|
| 09 | [Paperless-Only Invoice Capability Gaps](./_TECH_DEBT/09-paperless-ocr-capability-gaps.md) | Low | Open (six Gemini-path capabilities accepted as lost in [Task 84](84-paperless-only-invoices/)) |
| 08 | [Integration Suite Not Type-Checked](./_TECH_DEBT/08-integration-suite-not-type-checked.md) | Low | Open (35 tsc errors, 11 specs; 12 weakened `waitUntil` guards) |
| 07 | [Integration DB Reset Broken](./_TECH_DEBT/07-integration-db-reset-broken.md) | Medium | 🟡 Partly moot ([Task 73](./_done/73-web-first-migration/) deleted wdio.conf.ts; cross-spec sharing open -> [Task 82](82-integration-db-reset/)) |
| 06 | [Tauri Feature Gating](./_TECH_DEBT/06-tauri-feature-gating.md) | Medium | ✅ Moot ([Task 73](./_done/73-web-first-migration/) deleted the Tauri crate) |
| 05 | [Receipt State Model](_TECH_DEBT/05-receipt-trip-state-model.md) | Medium | ✅ Obsolete (Task 84 deleted the local receipt store the model described) |
| 04 | [Backup Restore Versioning](_TECH_DEBT/04-backup-restore-versioning.md) | Low | Open |
| 03 | Dead Code & Warnings | Low | ✅ Resolved (Task 37, file archived) |
| 02 | PHEV Compensation | Low | Open (see Task 19 status for context) |
| 01 | [Skill Command Conflict](_TECH_DEBT/01-skill-command-name-conflict.md) | Low | Open |

## Legend

| Icon | Meaning |
|------|---------|
| 📋 | Planning |
| 🟡 | Partial / In Progress |
| ✅ | Complete |
| ❌ | Blocked / On Hold |
