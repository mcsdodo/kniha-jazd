# Task: Invoice/Receipt Integration Test Coverage

## Problem Statement

Integration tests for receipts/invoices are severely limited:
- **1 passing test** (IPC vehicle filtering - trivial)
- **5 skipped tests** (blocked on file seeding + LLM mocking)

The core issue: **receipts require actual files in the receipts folder** and **Gemini LLM calls cannot be mocked**. The app scans folders via `scan_receipts`/`sync_receipts` commands - there's no direct `create_receipt` command for testing.

## Today's Work (2026-01-20)

A real invoice was added to `tests/integration/data/`:
- `invoice.pdf` - Scanned Slovnaft fuel receipt (256KB)
- `invoice.json` - Expected extraction values for mocking LLM:
  ```json
  {
      "price": 91.32,
      "date": "2026-01-20",
      "litres": 63.680,
      "station": "Slovnaft, a.s."
  }
  ```

These files provide the foundation for a **mocked LLM approach** to enable receipt integration testing.

## Acceptance Criteria

1. Enable at least 3 of the 5 skipped receipt tests
2. Test the full receipt workflow: scan → parse (mocked) → assign to trip
3. Verify mismatch detection UI (recently added feature)
4. No flaky tests - use deterministic mocked data

## Related Code

- `src-tauri/src/receipts.rs` - Folder scanning, status tracking
- `src-tauri/src/gemini.rs` - LLM client for OCR extraction
- `src-tauri/src/commands.rs` - Receipt assignment, matching logic
- `tests/integration/specs/tier2/receipts.spec.ts` - Skipped tests
- `tests/integration/fixtures/receipts.ts` - Receipt factory functions
