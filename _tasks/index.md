# Task Index

Quick overview of all tasks and their status.

**Last updated:** 2026-01-31

## Active Tasks

| # | Task | Status | Notes |
|---|------|--------|-------|
| 47 | [Datetime Consolidation](47-datetime-consolidation/) | ✅ Complete | Single datetime inputs, ADR-012 forward-only migrations |
| 42 | [Commands Module Split](42-commands-module-split/) | 🟡 In Progress | ADR-011: Split 3908-line commands.rs into modules |
| 41 | [Integration Test Speedup](41-integration-test-speedup/) | 📋 Planning | IPC-based DB reset for faster tests |
| 38 | [Magic Strings Refactoring](38-magic-strings-refactoring/) | 📋 Planning | Replace hardcoded strings with constants |
| 33 | [Web Deployment](33-web-deployment/) | 📋 Planning | Web version feasibility |
| 32 | [Portable CSV Backup](32-portable-csv-backup/) | 📋 Planning | Cross-platform backup format |

## Completed Tasks

| # | Task | Completed |
|---|------|-----------|
| 48 | end_datetime Cleanup | 2026-01-31 |
| 46 | Legal Requirements Updates | 2026-01-31 |
| 37 | Date Prefill Setting | 2026-01-31 |
| 36 | Suggested Fillup Legend | 2026-01-31 |
| 35 | Fuel Consumed Column | 2026-01-31 |
| 40 | Home Assistant ODO | 2026-01-29 |
| 39 | Trip Time + Hideable Columns | 2026-01-29 |
| 45 | DB Backup When Updating | 2026-01-24 |
| 44 | Multi-Currency Receipts | 2026-01-21 |
| 43 | ODO Recalculation Bug | 2026-01-21 |
| 42 | Receipt Mismatch Reasons | 2026-01-21 |
| 41 | Invoice Integration Tests | 2026-01-17 |
| 40 | Editable Receipt Settings | 2026-01-17 |
| 39 | Custom DB Location | 2026-01-17 |
| 38 | Auto-Update | 2026-01-15 |
| 37 | Dead Code Cleanup | 2026-01-13 |
| 36 | Dark Theme Overhaul | 2026-01-13 |
| 35 | Dark Theme | 2026-01-12 |
| 34 | Additional Costs Recognition | 2026-01-12 |
| 34 | Feature Docs Review | 2026-01-28 |
| 31 | Fix Stats Consumption | 2026-01-10 |
| 19 | Electric Vehicles | 2026-01-13 (partial - BEV done, PHEV pending) |
| ... | (older tasks in `_done/`) | ... |

> **Note:** Task numbers 35-41 were reused after earlier tasks with same numbers moved to `_done/`.
> See `_done/` folder for the original tasks with these numbers. New tasks should start from **46**.

## Tech Debt

| # | Item | Priority | Status |
|---|------|----------|--------|
| 03 | [Dead Code & Warnings](_TECH_DEBT/03-dead-code-and-warnings.md) | Low | Resolved (Task 37) |
| 02 | [PHEV Compensation](_TECH_DEBT/02-phev-compensation-suggestions.md) | Low | Open |
| 01 | [Skill Command Conflict](_TECH_DEBT/01-skill-command-name-conflict.md) | Low | Open |

## Legend

| Icon | Meaning |
|------|---------|
| 📋 | Planning |
| 🟡 | Partial / In Progress |
| ✅ | Complete |
| ❌ | Blocked / On Hold |
