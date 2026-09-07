# Tech Debt: The integration suite is not type-checked by anything

**Date:** 2026-09-07
**Priority:** Low
**Effort:** Medium (2-8h)
**Component:** [tests/integration/](../../tests/integration/), [tsconfig.json](../../tsconfig.json), [package.json](../../package.json), [.github/workflows/test.yml](../../.github/workflows/test.yml)
**Status:** Open

## Problem

Nothing type-checks `tests/`. Not `npm run check`, not any other npm script, not any CI
job. The WebdriverIO suite is compiled only by the type-stripping loader wdio uses at run
time, which does not report type errors. It currently has **35 of them**.

Three facts, each verified in this tree:

1. **`npm run check` excludes the tests.** [tsconfig.json:3](../../tsconfig.json) is
   literally `"exclude": ["tests"],` and `check` is
   `svelte-check --tsconfig ./tsconfig.json`. Everything under `tests/` is outside the
   program svelte-check builds.

2. **No npm script type-checks the suite.** The full script list in
   [package.json](../../package.json) is `dev build preview prepare i18n check
   check:watch test:backend test:integration test:integration:tier1/2/3
   test:integration:docker test:integration:docker:env test:all`. The only checker is
   `check`, and per (1) it does not see `tests/`.

3. **No CI job type-checks anything at all.** `grep -rn "tsc\|svelte-check\|run
   check\|typecheck\|type-check" .github/workflows/` returns nothing.
   [test.yml](../../.github/workflows/test.yml) runs `cargo test`, a docker build, and
   `npm run test:integration:docker` across three tiers plus the env-pinned suite. Even
   the *frontend* is never type-checked in CI — but there at least `npm run check` exists
   and is green locally. For `tests/` there is no such command to run.

A [tests/integration/tsconfig.json](../../tests/integration/tsconfig.json) does exist
(strict, `include: ["./**/*.ts"]`, wdio + mocha + node types). It is well-formed and
nothing invokes it.

### Evidence

Measured on `2374a72`, clean tree:

```
npx tsc --noEmit -p tests/integration/tsconfig.json
```

**35 errors, 11 spec files.** No errors in `utils/`, `fixtures/`, `_helpers/` or
`wdio.server.conf.ts` — the harness is clean; the drift is entirely in specs.

| Spec | Errors |
|------|--------|
| [tier2/route-autocomplete.spec.ts](../../tests/integration/specs/tier2/route-autocomplete.spec.ts) | 8 |
| [tier2/datetime-is-order.spec.ts](../../tests/integration/specs/tier2/datetime-is-order.spec.ts) | 5 |
| [tier1/bev-trips.spec.ts](../../tests/integration/specs/tier1/bev-trips.spec.ts) | 5 |
| [tier2/receipt-settings.spec.ts](../../tests/integration/specs/tier2/receipt-settings.spec.ts) | 3 |
| [tier2/paperless-integration.spec.ts](../../tests/integration/specs/tier2/paperless-integration.spec.ts) | 3 |
| [tier2/legal-compliance.spec.ts](../../tests/integration/specs/tier2/legal-compliance.spec.ts) | 3 |
| [tier1/km-odo-bidirectional.spec.ts](../../tests/integration/specs/tier1/km-odo-bidirectional.spec.ts) | 3 |
| [env/env-managed-settings.spec.ts](../../tests/integration/specs/env/env-managed-settings.spec.ts) | 2 |
| [tier3/compensation.spec.ts](../../tests/integration/specs/tier3/compensation.spec.ts) | 1 |
| [tier2/date-prefill.spec.ts](../../tests/integration/specs/tier2/date-prefill.spec.ts) | 1 |
| [tier1/trip-management.spec.ts](../../tests/integration/specs/tier1/trip-management.spec.ts) | 1 |

Two earlier ad-hoc measurements during Task 75 reported ~25 and 40. The number drifts
with every commit precisely because nothing pins it — which is the item.

### The 35, categorised

They are not equal. Only the first group is a defect in the test.

**A. Genuine missing `await` — the check is not what it reads as. 12 errors (TS2801).**

All twelve are the same `waitUntil` predicate, in `route-autocomplete` (8),
`km-odo-bidirectional` (3) and `trip-management` (1):

```ts
// tests/integration/specs/tier2/route-autocomplete.spec.ts:105
await browser.waitUntil(
  async () => {
    const dropdown = await $('.autocomplete .dropdown');
    return dropdown.isExisting() && (await dropdown.isDisplayed());
  },
  { timeout: 10000 }
);
```

`isExisting()` returns a `Promise<boolean>`, which is always truthy, so the left operand
of the `&&` is dead. The guard reads "exists AND is displayed" and means "is displayed".
TypeScript says so exactly: *TS2801: This condition will always return true since this
'Promise<boolean>' is always defined.*

In this position the damage is bounded — the `isDisplayed()` half still gates, so the
predicate is weakened, not inert. **The same mistake in an `expect(...)` is fully inert:**

```ts
expect(el.isExisting()).toBe(true);   // compares a Promise object to true — cannot fail
```

A grep for that shape across [tests/integration/specs/](../../tests/integration/specs/) finds **none today**. That is
luck, not a guarantee: nothing would report it if one landed tomorrow.

**B. WebdriverIO 9 chainable typings — noise, no runtime effect. 11 errors
(5×TS2367, 2×TS2365, 2×TS2362, 2×TS2345).**

```ts
// tests/integration/specs/tier2/datetime-is-order.spec.ts:115
const editing = await $$('.trip-grid tbody tr.editing');
if (editing.length !== 0) return false;   // TS2367: 'Promise<number>' and '0' have no overlap
```

`ChainablePromiseArray` ([node_modules/webdriverio/build/types.d.ts:87-93](../../node_modules/webdriverio/build/types.d.ts)) declares no
`then`, so `await` on it is a *type-level* no-op and `.length` stays `Promise<number>` —
while at run time the chainable really is thenable and resolves to a real array, so the
comparison works. Same root for the two TS2345s in `env-managed-settings`, where an
`await $(...)` is passed to a helper annotated `Element`.

This is also the source of the hint-level *TS80007 `'await' has no effect on the type of
this expression`* an IDE shows on the suite's `await $(...)` house style. That hint is not
a defect, the suite deliberately uses that style, and `tsc --noEmit` does not even emit it
(it is suggestion-only — the 35 above contain zero TS80007).

**The trouble is that A and B are the same shape.** A real missing `await` and a wdio
typings artifact both surface as "you compared a Promise to a number". With eleven of the
latter sitting in the output permanently, nobody reads the output, so the twelve of the
former went unnoticed.

**C. Stale or under-narrowed test-side types — assertion still runs correctly. 9 errors
(3×TS2551, 5×TS2538, 1×TS2322).**

`receipt-settings` asserts `settings?.hasGeminiApiKey`, which is the real field name
(`has_gemini_api_key` in
[receipts_cmd.rs:36](../../src-tauri/core/src/commands_internal/receipts_cmd.rs),
`hasGeminiApiKey` in [src/lib/types.ts:249](../../src/lib/types.ts)); the spec's local
`ReceiptSettingsShape` is what is wrong. The `bev-trips` and `date-prefill` ones are an id
typed `string | undefined` used as an index or assigned to a `string`.

**D. Dead test input. 3 errors (TS2353).**

[legal-compliance.spec.ts:55,66,77](../../tests/integration/specs/tier2/legal-compliance.spec.ts)
seeds three trips with `time: '10:00'` and friends. `SeedTripData`
([tests/integration/utils/db.ts:169](../../tests/integration/utils/db.ts)) has no `time`
field and `seedTrip` builds its RPC args field-by-field, so the value is silently dropped.
Three trips read as if they set a time; none do — a leftover from the `startDatetime`
migration. Nothing asserts on it, so the spec still passes.

**The number that matters: 12 weakened guards, 0 currently-inert assertions, 3 dead seed
fields.** The other 20 are cosmetic.

## Impact

- **An unknown subset of the suite's assertions could be decorative and nobody can tell
  which.** Today the answer happens to be "none fully, twelve partially", but that was
  established by hand, once, and expires with the next commit.
- **A spec can go green against a completely broken page.** That is the specific failure
  mode of an un-awaited `expect` — no timeout, no error, a passing test.
- **The existing 35 errors are a noise floor that hides new ones.** Category B guarantees
  a non-empty error list forever, so "just run tsc on the tests" is not something anyone
  can usefully do ad hoc.
- **The only line of defence is an IDE.** Task 75 caught its two TS2801 conditions purely
  because the editor underlined them mid-edit. An agent editing through Bash sees nothing.
- Not blocking anything. The suite does catch real regressions — Task 75 leaned on it
  repeatedly and it failed honestly every time it should have.

## Root Cause

Historical layering, not a decision:

- The root `tsconfig.json` excludes `tests` because the SvelteKit-generated config it
  extends targets the app, and `tests/` needs a different `types` array
  (`@wdio/globals`, mocha) that would poison the app program. Excluding was the correct
  local move; giving the excluded tree its own checker was never done.
- `tests/integration/tsconfig.json` was added for editor support — an IDE picks up the
  nearest tsconfig automatically, so the suite *looks* type-checked while being edited.
  That masked the absence of a batch check.
- CI grew around `cargo test` and wdio runs. A type-check step was never part of that
  shape, so no job ever ran one.
- WebdriverIO 9's chainable typings then made the output permanently non-empty, which
  removed the last incentive to run it by hand.

## Recommended Solution

**Prerequisite for either option below: fix the 35 existing errors first.** A gate that
starts red is not a gate — it gets `continue-on-error` within a week. Sequence:

1. **Fix category A (12)** — the actual bug. Mechanical:
   ```ts
   return (await editingRow.isExisting()) && (await editingRow.isDisplayed());
   ```
   [places.spec.ts:202](../../tests/integration/specs/tier2/places.spec.ts) already writes
   it this way; copy that.
2. **Fix category D (3)** — delete the three `time:` fields from `legal-compliance`, or
   add real datetime coverage if asserting on time was the intent.
3. **Fix category C (9)** — correct `ReceiptSettingsShape` to `hasGeminiApiKey`; narrow
   the ids at the seed call sites.
4. **Neutralise category B (11)** — the judgement call. Either use
   `(await $$(sel).getElements()).length`, which is the typed-correct form, or add one
   small helper in [tests/integration/utils/](../../tests/integration/utils/) (`countElements(sel): Promise<number>`) and
   route the call sites through it. Prefer the helper: one place to revisit when the wdio
   typings are fixed upstream, instead of scattered casts.
5. **Add the script** to [package.json](../../package.json):
   ```json
   "typecheck:tests": "tsc --noEmit -p tests/integration/tsconfig.json"
   ```
6. **Wire it into [test.yml](../../.github/workflows/test.yml)** as a step in a job that
   already runs `npm ci` — `integration-test-docker` (Tier 1) is the cheapest host, or a
   standalone `typecheck` job gated on `check-changes` if a clean signal is wanted. Runs
   in seconds; needs no docker image.

Note this leaves the *frontend* still un-type-checked in CI (`npm run check` exists but no
job runs it). Adding both in the same job is the obvious pairing, but the frontend has its
own baseline to establish first — out of scope here.

## Alternative Options (if any)

- **Fold `tests/` back into the root `tsconfig.json`** (drop the `"exclude"`, drop the
  nested config). One program, one command, `npm run check` covers everything, no second
  config to drift. Rejected as the primary recommendation: the two trees need different
  `types` arrays and different `moduleResolution` (`bundler` vs `node`), so the merge
  either loses `@wdio/globals` in the specs or leaks mocha/wdio globals into the app
  program. It also routes the specs through `svelte-check`, which buys nothing here. Worth
  reconsidering if the suite ever moves to a runner sharing the app's module resolution.
- **A `project references` setup** (root config referencing `tests/integration`).
  Technically the cleanest: one entry point without merging compiler options. Rejected on
  cost/benefit — it requires `composite: true` and emit plumbing across both configs for a
  repo with exactly two programs, where a second npm script is one line.
- **Status quo — rely on the IDE.** Defensible for a solo-maintained repo, and it is what
  caught the Task 75 errors. Rejected because agents editing via Bash have no IDE, and the
  suite is increasingly edited that way.
- **Fix the errors, skip the gate.** Half the value: the 35 come back, just more slowly.

## Related

- [_done/75-place-book/](../_done/75-place-book/) — where this surfaced. Its spec had two
  TS2801 conditions, caught only because an IDE underlined them mid-edit; a third error
  (`Promise.all(els.map(...))`) was a genuine runtime failure — WebdriverIO 9's
  `ElementArray.map()` is a chaining helper returning a promise, so it threw `object is
  not iterable`. See the comment at
  [places.spec.ts:427](../../tests/integration/specs/tier2/places.spec.ts).
- [tests/integration/specs/tier2/places.spec.ts](../../tests/integration/specs/tier2/places.spec.ts)
  — the one spec currently known to be clean under `tsc`, and the reference for the
  correct `(await x.isExisting()) && (await x.isDisplayed())` form.
- [tests/integration/tsconfig.json](../../tests/integration/tsconfig.json) — the config
  that exists and is never invoked.
- [.claude/rules/integration-tests.md](../../.claude/rules/integration-tests.md) — where a
  house rule about awaiting element queries belongs once the suite is clean.
- [07-integration-db-reset-broken.md](./07-integration-db-reset-broken.md) — same theme: a
  test-harness guarantee that silently did not hold.

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-09-07 | Created analysis | Surfaced during [Task 75](../_done/75-place-book/): the new spec's own missing `await`s were caught by an IDE, not by any command, which prompted checking whether *anything* type-checks `tests/`. Nothing does. Measured 35 errors across 11 specs on `2374a72`. |
| 2026-09-07 | Priority Low, not Medium | The suite demonstrably catches real regressions, and no fully-inert `expect(...)` exists in the tree today — only 12 weakened `waitUntil` guards. Raise to Medium the moment an inert assertion is found, since the class of bug is invisible by construction. |
| 2026-09-07 | Recommend `typecheck:tests` script + CI step over merging tsconfigs | The two trees need different `types` and `moduleResolution`; a second one-line script is cheaper than reconciling them. Fixing the 35 existing errors is a hard prerequisite either way. |
