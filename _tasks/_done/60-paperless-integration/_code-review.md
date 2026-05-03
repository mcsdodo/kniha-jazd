# Code Review

**Target:** `feat/paperless-integration` (21 commits, cb8db20..c339a2d)
**Reference:** [03-plan.md](./03-plan.md)
**Started:** 2026-05-03
**Status:** Complete
**Focus:** Quality, correctness, best practices

**Baseline Test Status:** Pass (320 tests)

---

## Review Summary

**Status:** Complete
**Iterations:** 1
**Total Findings:** 0 Critical, 3 Important, 5 Minor
**Test Status:** Pass (320 tests)

### Recommendation

Ready to merge — no critical issues. 3 Important items worth addressing before merge.

---

## Iteration 1

### New Findings

#### Critical

None.

#### Important

- [Important] `get_paperless_invoices` uses `format!("{:?}", e)` (Debug) instead of `format!("{}", e)` (Display) when mapping `PaperlessError` to a string — [dispatcher_async.rs](../../../../src-tauri/core/src/server/dispatcher_async.rs) and [integrations.rs (desktop)](../../../../src-tauri/desktop/src/commands/integrations.rs). Users see `TagNotFound("fuel")` instead of `Tag 'fuel' not found in Paperless`. Fix: `.map_err(|e| e.to_string())` or `format!("{}", e)`.

- [Important] `resolve_field_map` fetches `/api/custom_fields/` with no `page_size` and no pagination loop — [paperless.rs](../../../../src-tauri/core/src/paperless.rs) lines 60-77. Paperless's default page size is 25; a user with >25 custom fields will get a confusing `CustomFieldNotFound` error even though the fields exist. Fix: add `?page_size=200` or add a pagination loop (same pattern as `fetch_invoice_documents`). The test at [paperless_tests.rs](../../../../src-tauri/core/src/paperless_tests.rs) always exercises the single-page path, so this is untested.

- [Important] Missing test: **trip-side uniqueness invariant** — [db_tests.rs](../../../../src-tauri/core/src/db_tests.rs). `upsert_paperless_link` removes both the old doc link and the old trip link before inserting. The existing `paperless_link_unique_doc_invariant` test verifies doc-side (re-linking same doc to new trip removes old trip link). There is no test verifying the trip-side direction: assigning a different doc to the same trip should remove the old doc's link. The code is correct but a future refactor could break this silently.

#### Minor

- [Minor] `window.open(row.paperlessUrl, '_blank')` in Tauri — [doklady/+page.svelte](../../../../src/routes/doklady/+page.svelte). Works in practice via Tauri's navigation policy, but `openPath()` from `@tauri-apps/plugin-opener` is already imported and used elsewhere in the same file for local receipts. Using it here would be consistent.

- [Minor] URL validation duplicated in frontend `validatePaperlessUrl` and Rust backend — [settings/+page.svelte](../../../../src/routes/settings/+page.svelte) vs [integrations.rs (core)](../../../../src-tauri/core/src/commands_internal/integrations.rs). **Not** an ADR-008 violation — this is UX pre-validation identical to the HA settings pattern. The backend remains authoritative. No change required.

- [Minor] The `null` = "keep token" vs `''` = "clear token" semantic in `savePaperlessSettings` is correct but implicit — [api.ts](../../../../src/lib/api.ts). The behavior is documented only in the integration test teardown comment. A one-line comment on the API function would make the contract explicit.

- [Minor] `receiptDatetime` serializes as a `NaiveDateTime` string without timezone suffix (e.g. `"2026-04-27T13:24:14"`) — [models.rs](../../../../src-tauri/core/src/models.rs) / [types.ts](../../../../src/lib/types.ts). Browsers parse this as local time, which can cause off-by-one-day display near midnight for UTC+N users. This is a pre-existing pattern shared with local receipt datetimes — not a new regression.

- [Minor] No test for `save_paperless_settings_internal` with `url=None, token=None` (keep-existing path) — [integrations_tests.rs](../../../../src-tauri/core/src/commands_internal/integrations_tests.rs). Low risk but would protect the keep-existing behavior from future regressions.

---

## All Findings (Consolidated)

#### Important

- [x] Use `e.to_string()` / `format!("{}", e)` instead of `format!("{:?}", e)` for `PaperlessError` → [dispatcher_async.rs](../../../../src-tauri/core/src/server/dispatcher_async.rs) + [integrations.rs (desktop)](../../../../src-tauri/desktop/src/commands/integrations.rs)
- [x] Add `page_size=200` (or pagination loop) to `resolve_field_map` → [paperless.rs](../../../../src-tauri/core/src/paperless.rs) line 65
- [x] Add missing trip-uniqueness invariant test → [db_tests.rs](../../../../src-tauri/core/src/db_tests.rs)

#### Minor

- [x] Replace `window.open()` with `openPath()` for the Open in Paperless button → [doklady/+page.svelte](../../../../src/routes/doklady/+page.svelte)
- [x] Comment on `savePaperlessSettings` documenting `null` = keep, `''` = clear → [api.ts](../../../../src/lib/api.ts)
- [x] Test for keep-existing `None/None` path in `save_paperless_settings_internal`
- [ ] (Pre-existing, not blocking) NaiveDateTime timezone display issue — tracked for future fix

## Resolution

**Addressed:** 6 findings
**Skipped:** 1 (NaiveDateTime timezone — pre-existing pattern, not introduced by this feature)
**Test Status:** 322 passing (+2 new tests added)
**Status:** Complete
