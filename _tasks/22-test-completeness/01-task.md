# Task: Test Completeness for Business Logic

**Date:** 2026-01-05
**Status:** Complete
**Goal:** Ensure all business rules from DECISIONS.md are verified through tests

## Background

Analysis of the codebase revealed:
- **93 backend tests** pass across 8 modules
- Core calculations (`calculations.rs`) are well-tested (28 tests)
- Integration test infrastructure exists (`tests/integration/`) but only has proof-of-concept tests
- Several business rules lack explicit test verification

## Problem

While test coverage is good numerically, some business logic from DECISIONS.md lacks explicit verification:

1. **Partial fill-up handling** (Task 06) - `full_tank=false` should NOT close a consumption period
2. **Warning calculations** - date ordering and consumption limit warnings are untested
3. **Year carryover** - simple logic but undocumented through tests
4. **Integration tests** - infrastructure ready but only 1 spec implemented

## Goals

1. Add targeted unit tests for untested business rules
2. Complete planned integration test cases from `_tasks/20-e2e-testing/`
3. Document business behavior through tests (tests as documentation)

## Non-Goals

- Achieving arbitrary coverage percentages
- Writing filler tests for CRUD operations
- Testing trivial code paths

## Acceptance Criteria

- [x] Partial fill-up logic has explicit test
- [x] Warning calculations have tests
- [x] Year carryover has 2-case test (3 tests added)
- [ ] ~~At least 2 more integration test specs implemented~~ (Deferred - see `_review.md`)
- [x] All tests pass on CI (108 tests pass)
