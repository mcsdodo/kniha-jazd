**Date:** 2026-01-07
**Subject:** Comprehensive Integration Test Suite for Release Gate
**Status:** Planning

## Goal

Create a complete integration test suite (48 tests) that covers all user-facing functionality, to be run before every release. This ensures no regressions slip through when shipping new versions.

## Requirements

### Functional Coverage
- All 3 routes tested: Home (`/`), Settings (`/settings`), Receipts (`/doklady`)
- All vehicle types: ICE, BEV, PHEV
- All CRUD operations: vehicles, trips, receipts, backups
- Core business logic: consumption calculation, margin warnings, compensation suggestions
- Year handling: filtering, transitions, carryover

### Non-Functional
- Tests must be deterministic (no flaky tests)
- Mock external services (Gemini OCR) - use pre-seeded data
- Support tiered execution: Tier 1 (critical) can run separately for faster feedback
- Integrate with `/release` workflow

## Technical Notes

### Current State
- 10 existing integration tests in 2 spec files
- WebdriverIO + tauri-driver setup already working
- 108 Rust unit tests cover calculations (don't duplicate)

### Constraints
- Integration tests only run on Windows (tauri-driver limitation)
- Debug build required (`npm run tauri build -- --debug`)
- Receipt OCR must be mocked (no Gemini API in CI)

### Approach
- **Tier 1 (Critical):** 19 tests - Trip management, consumption/margin, year handling, export, BEV/PHEV trips
- **Tier 2 (Core):** 12 tests - Vehicle CRUD, receipts workflow, backups, settings
- **Tier 3 (Edge):** 7 tests - Empty states, compensation, multi-vehicle, validation

### Seeding Strategy
- **DB seeding** for preconditions (fast setup)
- **UI seeding** when testing the feature itself (validates forms)
