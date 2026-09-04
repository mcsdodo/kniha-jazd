# Task Index

Quick overview of all tasks and their status.

**Last updated:** 2026-09-04 (Task 74 completed)

## Active Tasks

| # | Task | Status | Notes |
|---|------|--------|-------|
| 72 | [Route Map Origin/Destination](72-route-map-origin-destination/) | 📋 Planning | Geocode the row's origin/destination, route A→B, alternatives + manual waypoint editing |
| 57 | [Invoice to Trip](57-invoice-to-trip/) | 📋 Planning | Create trip from fuel invoice (mid-trip split helper) + origin auto-fill |
| 51 | [Receipt State Model](51-receipt-state-model/) | 📋 Planning | Explicit assignment, user picks FUEL/OTHER |
| 41 | [Integration Test Speedup](41-integration-test-speedup/) | 📋 Planning | IPC-based DB reset for faster tests |
| 32 | [Portable CSV Backup](32-portable-csv-backup/) | 📋 Planning | Cross-platform backup format |

## Completed Tasks

| # | Task | Completed |
|---|------|-----------|
| 74 | [Main-Branch Image Channel](./_done/74-main-branch-image-channel/) | 2026-09-04 |
| 73 | [Web-First Migration](./_done/73-web-first-migration/) | 2026-09-04 |
| 71 | [Copy Trip Row](./_done/71-copy-trip-row/) | 2026-09-03 |
| 70 | [Route Map Integration](./_done/70-route-map-integration/) | 2026-08-10 |
| 67 | [Online Always-On Runner](./_done/67-online-always-on-runner/) | 2026-09-03 |
| 61 | [Route Map POC](./_done/61-route-map-poc/) — graduated by [Task 70](./_done/70-route-map-integration/) | 2026-08-10 |
| 69 | [PIN-Gated Secret Reveal](./_done/69-pin-gated-secret-reveal/) | 2026-08-10 |
| 68 | [Env-Managed Settings UI](./_done/68-env-managed-settings-ui/) | 2026-08-10 |
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
| 07 | [Integration DB Reset Broken](./_TECH_DEBT/07-integration-db-reset-broken.md) | Medium | ✅ Moot ([Task 73](./_done/73-web-first-migration/) deleted wdio.conf.ts; cross-spec sharing → Task 41) |
| 06 | [Tauri Feature Gating](./_TECH_DEBT/06-tauri-feature-gating.md) | Medium | ✅ Moot ([Task 73](./_done/73-web-first-migration/) deleted the Tauri crate) |
| 05 | [Receipt State Model](_TECH_DEBT/05-receipt-trip-state-model.md) | Medium | → Task 51 |
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
