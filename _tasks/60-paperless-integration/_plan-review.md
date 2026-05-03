**Date:** 2026-05-03
**Subject:** Plan review — [03-plan.md](./03-plan.md)
**Status:** Needs Revisions
**Reviewer model:** Opus 4.7 (1M context) — review skill ran in main context (no Haiku fork available; logged for future iteration)

# Plan Review — Paperless-ngx Integration

Companion docs: [01-task.md](./01-task.md), [02-design.md](./02-design.md), [description.md](./description.md), [03-plan.md](./03-plan.md).

## Summary

| Severity | Count |
|---|---|
| Critical | 5 |
| Important | 6 |
| Minor | 4 |

**Recommendation: Needs Revisions.** The plan is well-structured and the design probing
work (live-API verified fixtures) is high-quality, but several test scaffolding
references rely on helpers that do not exist in this codebase, and a sizeable surface
area of the project (server-mode RPC dispatchers, missing test dependency) is omitted.
None of the issues are conceptually blocking — they're concrete fix-ups before
implementation can succeed.

---

## Critical Findings

### C1 — Plan invents [db::tests::test_connection](../../src-tauri/core/src/db.rs), [db::tests::seed_trip](../../src-tauri/core/src/db.rs), [db::tests::CountRow](../../src-tauri/core/src/db.rs) (Tasks 6, 10)

- [x] **Status:** open  (see [Resolution](#resolution))

Tasks 6 and 10 build their failing tests on these helpers:

```rust
let conn = &mut crate::db::tests::test_connection();
let trip_a = crate::db::tests::seed_trip(conn);
let count = ... .get_result::<crate::db::tests::CountRow>(conn).unwrap().c;
```

None of these exist. The actual db test pattern in
[src-tauri/core/src/db_tests.rs](../../src-tauri/core/src/db_tests.rs) is:

```rust
let db = Database::in_memory().expect(...);     // not test_connection()
db.create_trip(&trip)?;                         // not seed_trip(conn)
```

The codebase has a [Database struct](../../src-tauri/core/src/db.rs) (line 48) that
owns [Mutex<SqliteConnection>](../../src-tauri/core/src/db.rs) and exposes typed CRUD
methods. There is no [tests submodule](../../src-tauri/core/src/db.rs) on
[db](../../src-tauri/core/src/db.rs), no
[seed_trip](../../src-tauri/core/src/db.rs) in the fixtures, and the only
[CountRow](../../src-tauri/core/src/commands_internal/backup.rs) lives privately inside
[commands_internal/backup.rs](../../src-tauri/core/src/commands_internal/backup.rs):330.

**Fix:** Either (a) add the helpers as a Task 5.5
([pub mod tests](../../src-tauri/core/src/db.rs) in
[db.rs](../../src-tauri/core/src/db.rs) with
[pub fn test_connection()](../../src-tauri/core/src/db.rs),
[pub fn seed_trip(conn)](../../src-tauri/core/src/db.rs),
[pub struct CountRow](../../src-tauri/core/src/db.rs)), or
(b) rewrite Tasks 6 and 10 tests to use
[Database::in_memory()](../../src-tauri/core/src/db.rs) and methods on it
(matching the existing convention). Option (b) is much closer to the existing style and
should be preferred.

### C2 — [wiremock](https://docs.rs/wiremock/) is not in [Cargo.toml](../../src-tauri/core/Cargo.toml) and the plan handwaves "if not already present" (Tasks 3, 7, 8)

- [x] **Status:** open  (see [Resolution](#resolution))

Tasks 3, 7, and 8 import [wiremock::matchers::*](https://docs.rs/wiremock/) and
[wiremock::{Mock, MockServer, ...}](https://docs.rs/wiremock/).
[Cargo.toml](../../src-tauri/core/Cargo.toml)
[\[dev-dependencies\]](../../src-tauri/core/Cargo.toml) block contains only
[tempfile = "3"](https://docs.rs/tempfile/). A repo-wide grep for
[wiremock](https://docs.rs/wiremock/) finds **zero** hits outside the plan itself. This
is not "extend HA test infra" — there is no HA test infra that uses
[wiremock](https://docs.rs/wiremock/); HA is tested manually + via the integration spec.

**Fix:** Add a concrete sub-step at the top of Task 3 (the first
[wiremock](https://docs.rs/wiremock/)-using task):

```toml
# src-tauri/core/Cargo.toml [dev-dependencies]
wiremock = "0.6"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }   # for #[tokio::test]
```

Note: [tokio](https://docs.rs/tokio/) is in
[\[dependencies\]](../../src-tauri/core/Cargo.toml) already (with the right features),
but it is **not** automatically available in tests; the
[#\[tokio::test\]](https://docs.rs/tokio/) macro requires it in
[\[dev-dependencies\]](../../src-tauri/core/Cargo.toml) too, **or** the
[dep:](../../src-tauri/core/Cargo.toml) section needs a feature gate. Confirm by
running [cargo test test_paperless_connection](../../src-tauri/core/Cargo.toml) after
Task 3 step 1 — if you get "cannot find attribute
[tokio::test](https://docs.rs/tokio/) in this scope," that's why.

### C3 — Plan adds 7 Tauri commands but never registers them in the server-mode RPC dispatchers ([dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs) / [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs)) (Task 11)

- [x] **Status:** open  (see [Resolution](#resolution))

The codebase has a parallel command path for the web-deployed server mode. It lives in
[src-tauri/core/src/server/dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs):780
and
[src-tauri/core/src/server/dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs):67.
Every HA command is dispatched in **both** places (3 sync + 2 async). The plan only
touches the desktop
[invoke_handler!](../../src-tauri/desktop/src/lib.rs), so the entire Paperless feature
will be missing from server mode (silently — the web frontend will get
"Unknown command" errors).

**Fix:** Extend Task 11 to also add 7 dispatcher entries:

- Sync (3) → [dispatcher.rs](../../src-tauri/core/src/server/dispatcher.rs):
  [get_paperless_settings](../../src-tauri/core/src/server/dispatcher.rs),
  [save_paperless_settings](../../src-tauri/core/src/server/dispatcher.rs),
  [get_invoice_source_mode](../../src-tauri/core/src/server/dispatcher.rs).
- Async (4) → [dispatcher_async.rs](../../src-tauri/core/src/server/dispatcher_async.rs):
  [test_paperless_connection](../../src-tauri/core/src/server/dispatcher_async.rs),
  [get_paperless_invoices](../../src-tauri/core/src/server/dispatcher_async.rs),
  [assign_paperless_doc_to_trip](../../src-tauri/core/src/server/dispatcher_async.rs),
  [unassign_paperless_doc](../../src-tauri/core/src/server/dispatcher_async.rs).

Pattern for async with args (mirroring
[fetch_ha_odo](../../src-tauri/core/src/server/dispatcher_async.rs):72):

```rust
"get_paperless_invoices" => {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Args { vehicle_id: String, year: i32 }
    let a: Args = match parse_args(args) { Ok(a) => a, Err(e) => return Some(Err(e)) };
    // ... await internal, map to Value
}
```

This is a 30-line addition but it's load-bearing for
[/web](../../_tasks/_done/33-web-deployment/) (Task 33 in
[_done/](../../_tasks/_done/)).

### C4 — Tier-2 spec uses [Vitest](https://vitest.dev/) framework imports; the project uses [WebdriverIO](https://webdriver.io/) + [Mocha](https://mochajs.org/) (Task 15)

- [x] **Status:** open  (see [Resolution](#resolution))

Plan line 1789:

```ts
import { describe, it, expect, before, after } from 'vitest';
```

This will not compile under the integration test runner.
[package.json](../../package.json):

```json
"@wdio/mocha-framework": "^9.23.0"
```

All existing tier-2 specs use **[Mocha](https://mochajs.org/) globals** (
[describe](https://mochajs.org/), [it](https://mochajs.org/),
[beforeEach](https://mochajs.org/), etc. — no import) and
[expect](https://webdriver.io/docs/api/expect-webdriverio) comes from
[WebdriverIO](https://webdriver.io/) ([expect-webdriverio](https://webdriver.io/docs/api/expect-webdriverio),
available as a global on
[$](https://webdriver.io/docs/api/browser/$)/[browser](https://webdriver.io/docs/api/browser)).
See
[tests/integration/specs/tier2/settings.spec.ts](../../tests/integration/specs/tier2/settings.spec.ts):55
for the canonical pattern.

**Fix:** Drop the [Vitest](https://vitest.dev/) import; use the existing test utilities:

```ts
import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { invokeTauri } from '../../utils/db';

describe('Tier 2: Paperless Integration', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  it('Settings → test → Doklady renders rows → Assign persists', async () => {
    // ...
  });
});
```

Also: the plan's spec runs the mock Paperless server inside the
[WDIO](https://webdriver.io/) process. That's fine in principle, but for the
Tauri-driven integration test the Tauri process is the HTTP client — it must reach the
mock from its **own** process. Using
[mockUrl](../../tests/integration/specs/tier2/) (typically
[http://localhost:NNNN](http://localhost) chosen by Node) works only because both
processes share [localhost](http://localhost). Worth a sentence acknowledging this so
future maintainers don't break it by moving the mock into the Tauri process.

### C5 — Migration command/path inconsistency: [diesel migration run](https://diesel.rs/) against a stray [target/test-paperless.db](../../target/), but this project uses embedded migrations on [Database::in_memory()](../../src-tauri/core/src/db.rs) (Task 5)

- [x] **Status:** open  (see [Resolution](#resolution))

Plan line 677:

```powershell
diesel migration run --database-url ../../target/test-paperless.db
```

[db.rs](../../src-tauri/core/src/db.rs):58 runs migrations via
[conn.run_pending_migrations(MIGRATIONS)](../../src-tauri/core/src/db.rs) (embedded).
The [MIGRATIONS](../../src-tauri/core/src/db.rs) constant is declared via
[embed_migrations!()](https://docs.rs/diesel_migrations/) and **all** migration folders
live alongside it (see directory listing of
[src-tauri/core/migrations/](../../src-tauri/core/migrations/)).
That means:

1. The [diesel migration run](https://diesel.rs/) step is **optional in this project** —
   it only generates [schema.rs](../../src-tauri/core/src/schema.rs) from a real DB.
   Tests pick up the new migration via [embed_migrations!](https://docs.rs/diesel_migrations/)
   automatically once the folder exists.
2. Pointing it at [../../target/test-paperless.db](../../target/) creates a throwaway
   file in the workspace target dir that's neither
   [.gitignore](../../.gitignore)-d nor cleaned up by anything; this is a small mess
   but not what the project's prior migrations have done (look at the 16 existing
   [migrations](../../src-tauri/core/migrations/) — none of them required a
   [target/](../../target/) artifact).
3. The plan says "[schema.rs](../../src-tauri/core/src/schema.rs) is regenerated
   automatically" but [Diesel](https://diesel.rs/) only regenerates it via
   [diesel print-schema > schema.rs](https://diesel.rs/) against the DB;
   [diesel migration run](https://diesel.rs/) alone does not update
   [schema.rs](../../src-tauri/core/src/schema.rs). (Unless there's a project-specific
   [diesel.toml](../../src-tauri/core/diesel.toml) that wires
   [print_schema.file = "src/schema.rs"](../../src-tauri/core/diesel.toml) — confirm.)

**Fix:** Replace Step 2-3 of Task 5 with the actual project workflow. Either:

- **Manual edit:** Append the [paperless_trip_links](../../src-tauri/core/src/schema.rs)
  table block to [schema.rs](../../src-tauri/core/src/schema.rs) by hand (mirroring an
  existing small table, e.g., [vehicles](../../src-tauri/core/src/schema.rs)), since
  that's the source-of-truth file the codebase reads.
- **Or document the regeneration command actually used:** check
  [diesel.toml](../../src-tauri/core/diesel.toml) for
  [print_schema](https://diesel.rs/) config; if present, the command is something like
  [diesel database setup --database-url ... && diesel print-schema > src/schema.rs](https://diesel.rs/).

Either way, drop the [target/test-paperless.db](../../target/) artifact — it's not the
way.

---

## Important Findings

### I1 — [urlencoding](https://docs.rs/urlencoding/) crate is not in [Cargo.toml](../../src-tauri/core/Cargo.toml) (Task 7)

- [x] **Status:** open  (see [Resolution](#resolution))

Plan line 1017 calls [urlencoding::encode(name)](https://docs.rs/urlencoding/).
[urlencoding](https://docs.rs/urlencoding/) is not in
[Cargo.toml](../../src-tauri/core/Cargo.toml). The plan says "Add
[urlencoding](https://docs.rs/urlencoding/) to
[\[dependencies\]](../../src-tauri/core/Cargo.toml)... if not present" (line 1049) but
never makes it a concrete step.

**Fix:** Add [urlencoding = "2"](https://docs.rs/urlencoding/) to
[\[dependencies\]](../../src-tauri/core/Cargo.toml) as Task 7 step 0, OR drop
[urlencoding](https://docs.rs/urlencoding/) and use
[reqwest::Url](https://docs.rs/reqwest/)'s built-in query-builder
([url.query_pairs_mut().append_pair("name__iexact", name)](https://docs.rs/url/)),
which is already in the dep tree via [reqwest](https://docs.rs/reqwest/).

### I2 — [app_state.set_read_only(true, "test".into())](../../src-tauri/core/src/app_state.rs) signature is unverified

- [x] **Status:** open  (see [Resolution](#resolution))

Plan lines 322 and 1430 use:

```rust
app_state.set_read_only(true, "test".into());
```

Whether this signature matches the actual
[AppState](../../src-tauri/core/src/app_state.rs) API in this codebase is unverified;
the plan inherited it from an HA-style assumption. Existing calls to read-only checks
in the codebase use
[check_read_only!(app_state)](../../src-tauri/core/src/app_state.rs) (a macro).
**Confirm** the setter signature matches before locking these test stubs in.

**Fix:** Either confirm by reading
[app_state.rs](../../src-tauri/core/src/app_state.rs) first, or write the read-only
test against whatever helper is already used in
[commands_tests.rs](../../src-tauri/core/src/commands_internal/commands_tests.rs) for
HA's analogous
[save_ha_settings_blocked_by_read_only](../../src-tauri/core/src/commands_internal/commands_tests.rs)
test (the plan claims to mirror HA — find the HA equivalent and copy that exact
pattern).

### I3 — [ReceiptStatus](../../src-tauri/core/src/models.rs) / [AssignmentType](../../src-tauri/core/src/models.rs) priority logic edge case ("neither" branch is unreachable but mapped to [Other](../../src-tauri/core/src/models.rs))

- [x] **Status:** open  (see [Resolution](#resolution))

In Task 9 (line 1340):

```rust
pub fn map_assignment(tag_ids: &[i64], fuel_id: i64, car_id: i64) -> AssignmentType {
    if tag_ids.contains(&fuel_id) { AssignmentType::Fuel }
    else if tag_ids.contains(&car_id) { AssignmentType::Other }
    else { AssignmentType::Other }   // unreachable in practice (server-side filter)
}
```

The "unreachable" branch defaulting to
[Other](../../src-tauri/core/src/models.rs) is silently lossy if the server-side
filter ever fails ([Paperless API](https://docs.paperless-ngx.com/api/) returns an
unexpected tag set, race condition on custom-field migration, etc.). Since the test
suite includes
[fuel_only_to_fuel](./03-plan.md),
[car_only_to_other](./03-plan.md),
[both_tags_priority_fuel](./03-plan.md), but NOT
[neither_tag_should_panic_or_log](./03-plan.md), the regression is invisible.

**Fix:** Either (a) panic in dev with a clear message ("server-side filter failed; doc
{} has tags {:?}"), or (b) [log::error!](https://docs.rs/log/) and continue with
[Other](../../src-tauri/core/src/models.rs). Add a test for the "neither" case
explicitly so future maintainers see the choice.

### I4 — [paperless](../../src-tauri/core/src/) module is not added to [lib.rs](../../src-tauri/core/src/lib.rs) [pub mod](../../src-tauri/core/src/lib.rs) list before tests run (Task 7)

- [x] **Status:** open  (see [Resolution](#resolution))

Plan line 877 says: "Modify [lib.rs](../../src-tauri/core/src/lib.rs) (add
[pub mod paperless;](../../src-tauri/core/src/lib.rs))" but the test code
in step 1 already calls
[super::PaperlessClient::new(...)](../../src-tauri/core/src/) etc. — so the new module
must be [pub mod paperless](../../src-tauri/core/src/lib.rs) AND
[#\[cfg(test)\] #\[path = ...\] mod tests;](../../src-tauri/core/src/) before
[cargo test](https://doc.rust-lang.org/cargo/) will find anything. Adding the
[lib.rs](../../src-tauri/core/src/lib.rs) edit as a numbered substep (instead of in a
side note) prevents the "tests fail with [cannot find module](../../src-tauri/core/src/lib.rs)" loop.

**Fix:** In Task 7, make the file edits a sequenced list:

1. Create empty [paperless.rs](../../src-tauri/core/src/paperless.rs) with a doc comment.
2. Edit [lib.rs](../../src-tauri/core/src/lib.rs) to add
   [pub mod paperless;](../../src-tauri/core/src/lib.rs).
3. [cargo build -p kniha_jazd_core](https://doc.rust-lang.org/cargo/) (catches the
   empty module).
4. Then write the failing tests + implementation as currently described.

### I5 — [AssignmentType](../../src-tauri/core/src/models.rs) is [Copy + Eq](https://doc.rust-lang.org/std/marker/trait.Copy.html) and serializes as "Fuel" / "Other"; ensure frontend [PaperlessInvoiceRow.assignment_type](./03-plan.md) deserializes the same way

- [x] **Status:** open  (see [Resolution](#resolution))

[models.rs](../../src-tauri/core/src/models.rs):465 defines
[#\[derive(...Serialize, Deserialize)\]](https://serde.rs/) with default tag (no
[#\[serde(tag = ...)\]](https://serde.rs/)), which gives "Fuel" / "Other" as plain JSON
strings (string enum). The plan's
[PaperlessInvoiceRow](./03-plan.md) keeps
[assignment_type: AssignmentType](./03-plan.md) — fine. Just confirm the
TypeScript side declares it the same way to avoid the
"I got [{ Fuel: null }](https://serde.rs/)" trap that bites people with
[serde](https://serde.rs/) tagged enums.

**Fix:** In Task 13's [src/lib/types.ts](../../src/lib/types.ts) edits, declare:

```ts
export type AssignmentType = 'Fuel' | 'Other';   // matches Rust serde default
```

Add a test asserting the JSON shape if you don't trust the round-trip.

### I6 — Hardcoded fuel/car tag lookup ignores the existence of multi-vehicle setups ([vehicle_id](./03-plan.md) is shadowed [_vehicle_id](./03-plan.md))

- [x] **Status:** open  (see [Resolution](#resolution))

Task 9, line 1351:

```rust
pub async fn get_paperless_invoices_internal(
    app_dir: &Path, conn: &mut diesel::SqliteConnection,
    _vehicle_id: &str, year: i32,
) -> Result<Vec<PaperlessInvoiceRow>, PaperlessError> {
```

The frontend passes [currentVehicleId](./03-plan.md) (Task 14, line 1722), but the
backend ignores it. Today this is fine because all tagged invoices in
[Paperless](https://docs.paperless-ngx.com/) apply across the user's single vehicle,
but the UI expects per-vehicle filtering (consistent with the local receipt grid).
When the user has two cars, every fuel doc shows up on both cars'
[doklady](../../src/routes/doklady/+page.svelte) pages.

**Fix:** Either (a) document explicitly in Task 9 that vehicle scoping is deferred and
the parameter is currently unused, with a TODO and a note in
[DECISIONS.md](../../DECISIONS.md), or (b) gate the param with a tag like
[vehicle:{name}](https://docs.paperless-ngx.com/) and handle the lookup; at minimum,
add a test for this so the next maintainer doesn't think it's a bug. The current plan
silently swallows the param without saying so.

---

## Minor Findings

### M1 — [_tasks/index.md](../index.md) already lists task 60 as "Planning"; Task 0 step 2 condition "if not already present" is a no-op

- [x] **Status:** open  (see [Resolution](#resolution))

[_tasks/index.md](../index.md):12 currently shows "Planning". Task 0 step 2 says
"add a row if not present, with status In Progress." The reasonable change is
**not** "if not present, add" — it's "update the existing row's status to In Progress".
Tighten the wording.

### M2 — Manual smoke test step in Task 17 lists [documents.lacny.me](https://documents.lacny.me) PAT in plan; [.env](../../.env) is the right source, double-check it's gitignored

- [x] **Status:** open  (see [Resolution](#resolution))

The plan helpfully points implementers to [.env](../../.env) for the PAT. Confirm
[.env](../../.env) is [.gitignore](../../.gitignore)-d (likely yes given file naming
convention). Worth a one-line "DO NOT commit [.env](../../.env)" reminder in Task 17
for the next reader.

### M3 — Risks section understates "[Paperless](https://docs.paperless-ngx.com/) instance reachable but very slow"

- [x] **Status:** open  (see [Resolution](#resolution))

The 5-second timeout is the same as HA but [Paperless](https://docs.paperless-ngx.com/)
serves a 4.7 KB JSON for [/api/ui_settings/](https://docs.paperless-ngx.com/api/) and
a much heavier paginated list for [/api/documents/](https://docs.paperless-ngx.com/api/).
Worth capturing a second risk: "Sync of >100 docs hits the 5s timeout per page → user
sees DISCONNECTED falsely." Either bump the timeout for
[fetch_invoice_documents](./03-plan.md) or document the 100-doc-per-page page-load as
the design ceiling for v1.

### M4 — PR-create command in Task 17 step 6 uses [bash](https://www.gnu.org/software/bash/) heredoc syntax in [PowerShell](https://learn.microsoft.com/en-us/powershell/)

- [x] **Status:** open  (see [Resolution](#resolution))

```powershell
gh pr create --title "..." --body-file <(echo -e "## Summary\n...")
```

The [<(...)](https://www.gnu.org/software/bash/) process substitution is
[bash](https://www.gnu.org/software/bash/)-only, not
[PowerShell](https://learn.microsoft.com/en-us/powershell/). [CLAUDE.md](../../CLAUDE.md)
indicates the shell is [PowerShell](https://learn.microsoft.com/en-us/powershell/).
On Windows this command will fail with "the system cannot find the file specified."

**Fix:** Either use
[--body](https://cli.github.com/manual/gh_pr_create) with a
[PowerShell](https://learn.microsoft.com/en-us/powershell/) here-string:

```powershell
gh pr create --title "feat: paperless..." --body @"
## Summary
- Adds Paperless-ngx as an alternative source...
"@
```

Or write the body to a real file with
[Set-Content](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/set-content)
first and pass [--body-file body.md](https://cli.github.com/manual/gh_pr_create), then
delete it.

---

## What's Good

The plan is unusually rigorous in several respects, worth calling out so the revision
preserves them:

- **Live-API verification** of fixtures, tag IDs, custom-field IDs, and the auth-header
  divergence is excellent — this is the kind of work that prevents the most painful
  bugs (the [Bearer](https://datatracker.ietf.org/doc/html/rfc6750) vs
  [Token](https://www.django-rest-framework.org/api-guide/authentication/) regression
  test in Task 3 is gold).
- **Bite-sized commits** following [TDD](../../.claude/rules/rust-backend.md) ordering
  match the project's
  [superpowers:writing-plans](../../.claude/skills/) discipline.
- **[DECISIONS.md](../../DECISIONS.md)** entries (ADR + BIZ) capture the right "why"
  rationale for future agents.
- **Mode-switch pattern** keeps [ADR-008](../../DECISIONS.md) honest: a single Rust
  function answers "are we in [Paperless](https://docs.paperless-ngx.com/) mode?",
  frontend doesn't branch on raw settings.
- **Dependent-side
  [trip_id PRIMARY KEY](../../src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql)**
  pattern correctly mirrors the existing
  [receipts.trip_id UNIQUE](../../src-tauri/core/migrations/2026-01-08-095218-0000_baseline/up.sql)
  shape.

---

## Iteration log

- **Round 1 (this review):** Identified 5 critical, 6 important, 4 minor findings.
  No further iteration performed in this review pass.

---

## Resolution

Phase 2 applied 2026-05-03 — all 11 critical + important findings patched in
[03-plan.md](./03-plan.md). Minor findings (M1–M4) deferred per user's "fix all
Cs and Is" instruction.

| Finding | Action | Notes |
|---|---|---|
| [C1](#c1--plan-invents-dbteststest_connection-dbtestsseed_trip-dbtestscountrow-tasks-6-10) | ✅ Fixed | Tasks 6 + 10 rewritten to use [Database::in_memory()](../../src-tauri/core/src/db.rs):68; CRUD turned into methods on `Database`; new `seed_test_trip` helper inlined into [db_tests.rs](../../src-tauri/core/src/db_tests.rs) mirroring [create_test_vehicle](../../src-tauri/core/src/db_tests.rs):15. |
| [C2](#c2--wiremock-is-not-in-cargotoml-and-the-plan-handwaves-if-not-already-present-tasks-3-7-8) | ✅ Fixed | Added `wiremock = "0.6"` to `[dev-dependencies]` as Task 3 Step 0; pre-flight section documents that `tokio` is already in `[dependencies]` with `macros` enabled. |
| [C3](#c3--plan-adds-7-tauri-commands-but-never-registers-them-in-the-server-mode-rpc-dispatchers-dispatcherrs--dispatcher_asyncrs-task-11) | ✅ Fixed | Task 11 expanded with explicit code blocks for [dispatcher.rs:777](../../src-tauri/core/src/server/dispatcher.rs) (3 sync) and [dispatcher_async.rs:64](../../src-tauri/core/src/server/dispatcher_async.rs) (4 async). |
| [C4](#c4--tier-2-spec-uses-vitest-framework-imports-the-project-uses-webdriverio--mocha-task-15) | ✅ Fixed | Task 15 spec rewritten to drop `'vitest'` import; uses Mocha globals + utils helpers ([waitForAppReady](../../tests/integration/utils/), [navigateTo](../../tests/integration/utils/), [invokeTauri](../../tests/integration/utils/db.ts)) matching [settings.spec.ts:1-60](../../tests/integration/specs/tier2/settings.spec.ts). |
| [C5](#c5--migration-commandpath-inconsistency-diesel-migration-run-against-a-stray-targettest-paperlessdb-but-this-project-uses-embedded-migrations-on-databasein_memory-task-5) | ✅ Fixed | Task 5 Step 2 replaced: manually edit [schema.rs](../../src-tauri/core/src/schema.rs) (project pattern); optional CLI alternative documented using `$env:TEMP` instead of stray `target/`. Step 4 verifies via `cargo test test_database_creation` since [embed_migrations!()](../../src-tauri/core/src/db.rs):23 runs the migration through `Database::in_memory()`. |
| [I1](#i1--urlencoding-crate-is-not-in-cargotoml-task-7) | ✅ Fixed | `urlencoding = "2"` added in Task 7 Step 0a as a concrete substep. Pre-flight table at top of plan also lists it. |
| [I2](#i2--app_stateset_read_onlytrue-testinto-signature-is-unverified) | ✅ Fixed | Tasks 2 and 10 use [app_state.enable_read_only("test")](../../src-tauri/core/src/app_state.rs):98 (the actual API). Pre-flight section documents the correct signature. |
| [I3](#i3--receiptstatus--assignmenttype-priority-logic-edge-case-neither-branch-is-unreachable-but-mapped-to-other) | ✅ Fixed | `map_assignment` now calls `log::warn!(...)` on the unreachable branch; new test `map_assignment_logs_warning_and_returns_other_when_neither_tag_present` locks the don't-panic contract. |
| [I4](#i4--paperless-module-is-not-added-to-librs-pub-mod-list-before-tests-run-task-7) | ✅ Fixed | Task 7 Step 0b creates the empty module + `pub mod paperless;` in [lib.rs](../../src-tauri/core/src/lib.rs) BEFORE Step 1 writes failing tests, with a `cargo build` verification gate. |
| [I5](#i5--assignmenttype-is-copy--eq-and-serializes-as-fuel--other-ensure-frontend-paperlessinvoicerowassignment_type-deserializes-the-same-way) | ✅ Fixed | Task 13's [types.ts](../../src/lib/types.ts) line now explicitly says "Do NOT redefine `AssignmentType` — already exists at [types.ts:189](../../src/lib/types.ts) as `'Fuel' \| 'Other'`." |
| [I6](#i6--hardcoded-fuelcar-tag-lookup-ignores-the-existence-of-multi-vehicle-setups-vehicle_id-is-shadowed-_vehicle_id) | ✅ Fixed | Task 9 renames `_vehicle_id` → `vehicle_id`, adds `let _ = vehicle_id;` with a doc-comment explaining the deferral. Task 16 records a third "BIZ — Paperless v1 is single-vehicle scoped" entry for [DECISIONS.md](../../DECISIONS.md). |
| [M1](#m1--_tasksindexmd-already-lists-task-60-as-planning-task-0-step-2-condition-if-not-already-present-is-a-no-op) | ⏭ Deferred | Per user instruction "fix all Cs and Is" — minor findings out of scope. |
| [M2](#m2--manual-smoke-test-step-in-task-17-lists-documentslacnyme-pat-in-plan-env-is-the-right-source-double-check-its-gitignored) | ⏭ Partial | Pre-flight section now notes [.env](../../.env) is gitignored and adds "Do NOT commit [.env](../../.env)" reminder; full M2 wording in Task 17 deferred. |
| [M3](#m3--risks-section-understates-paperless-instance-reachable-but-very-slow) | ⏭ Deferred | Per user instruction — minor finding out of scope. |
| [M4](#m4--pr-create-command-in-task-17-step-6-uses-bash-heredoc-syntax-in-powershell) | ⏭ Deferred | Per user instruction — minor finding out of scope. |
