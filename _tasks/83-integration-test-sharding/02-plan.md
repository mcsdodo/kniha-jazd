# Integration Test Sharding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the integration specs across 6 equal CI jobs instead of 3 unequal tier jobs, taking the slowest integration job from 5m26 to about 2m16.

**Architecture:** A pure helper assigns spec files to shards round-robin. `getSpecs()` in the WDIO config calls it when `WDIO_SHARD` is set, and returns that shard's files instead of the tier globs. The CI matrix carries shard indices. Local tier scripts do not change.

**Tech Stack:** TypeScript, WebdriverIO 9.23, `node --test` (built in, no new dependency), GitHub Actions.

**Spec:** [01-task.md](01-task.md)

**Depends on:** [Task 82](../82-integration-db-reset/). Sharding reorders which specs share a database. Do not merge this before 82 is green.

## Global Constraints

- Do not touch the `TIER` or `PARALLEL_TIERS` paths in `getSpecs()`. `npm run test:integration:tier1` must behave exactly as it does today.
- Do not add an npm dependency. `node --test` runs TypeScript directly on Node 24 (verified: `node --version` reports v24.15.0).
- Do not add an npm script. CI passes an env var, the same way it passes `TIER` today, so invariant I1 keeps holding through `test:integration:docker`.
- The env-pinned job (`integration-test-docker-env`) is out of scope and must not change.
- `shard.current` is one-based, matching WDIO's own convention.
- Slovak UI text and i18n are not involved in this task.

---

### Task 1: The `shardSpecs` helper

Round-robin assignment, as a pure function with its own test. WDIO's built-in `--shard` is deliberately not used: it slices contiguously (`node_modules/@wdio/config/build/node/index.js:567-576`), and our spec list is grouped by tier, so a contiguous slice draws whole clusters of slow tier 2 specs.

**Files:**
- Create: `tests/integration/utils/shard.ts`
- Test: `tests/integration/utils/shard.test.ts`

**Interfaces:**
- Consumes: nothing.
- Produces: `shardSpecs<T>(files: T[], current: number, total: number): T[]`. One-based `current`. Throws on a non-integer or out-of-range shard.

- [ ] **Step 1: Write the failing test**

Create `tests/integration/utils/shard.test.ts`:

```ts
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { shardSpecs } from './shard.ts';

const files = Array.from({ length: 35 }, (_, i) => `spec-${i}.spec.ts`);

test('every file lands in exactly one shard', () => {
  const all = [1, 2, 3, 4, 5, 6].flatMap((current) => shardSpecs(files, current, 6));
  assert.equal(all.length, files.length);
  assert.deepEqual([...new Set(all)].sort(), [...files].sort());
});

test('shards differ by at most one file', () => {
  const sizes = [1, 2, 3, 4, 5, 6].map((current) => shardSpecs(files, current, 6).length);
  assert.ok(Math.max(...sizes) - Math.min(...sizes) <= 1, `sizes: ${sizes.join(',')}`);
});

test('it interleaves rather than slicing contiguously', () => {
  // The contiguous slice WDIO would take is files 0 to 5. Round-robin must not.
  assert.deepEqual(shardSpecs(files, 1, 6).slice(0, 3), [
    'spec-0.spec.ts',
    'spec-6.spec.ts',
    'spec-12.spec.ts',
  ]);
});

test('a single shard returns everything', () => {
  assert.deepEqual(shardSpecs(files, 1, 1), files);
});

test('it rejects a shard outside the range', () => {
  assert.throws(() => shardSpecs(files, 0, 6), /out of range/);
  assert.throws(() => shardSpecs(files, 7, 6), /out of range/);
  assert.throws(() => shardSpecs(files, 1, 0), /out of range/);
});

test('it rejects a non-integer shard', () => {
  assert.throws(() => shardSpecs(files, 1.5, 6), /integers/);
  assert.throws(() => shardSpecs(files, Number.NaN, 6), /integers/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test tests/integration/utils/shard.test.ts`

Expected: FAIL. The error names the missing module, `Cannot find module ... shard.ts`.

- [ ] **Step 3: Write the implementation**

Create `tests/integration/utils/shard.ts`:

```ts
/**
 * Round-robin spec sharding for CI.
 *
 * WDIO's own `--shard` slices the spec list contiguously (see
 * node_modules/@wdio/config/build/node/index.js:567-576). Our spec list is
 * grouped by tier, so a contiguous slice draws whole clusters of slow tier 2
 * specs: at 4 shards one of them would take odometer-cascade, legal-compliance,
 * copy-trip, column-visibility and datetime-is-order together. Round-robin
 * interleaves the tiers instead.
 *
 * `current` is one-based, matching WDIO's own convention.
 */
export function shardSpecs<T>(files: T[], current: number, total: number): T[] {
  if (!Number.isInteger(current) || !Number.isInteger(total)) {
    throw new Error(`shard must be given as integers, got ${current}/${total}`);
  }
  if (total < 1 || current < 1 || current > total) {
    throw new Error(`shard ${current}/${total} is out of range`);
  }
  return files.filter((_, index) => index % total === current - 1);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test tests/integration/utils/shard.test.ts`

Expected: PASS, `# pass 6`, `# fail 0`.

- [ ] **Step 5: Commit**

```bash
git add tests/integration/utils/shard.ts tests/integration/utils/shard.test.ts
git commit -m "test(83): add round-robin spec sharding helper"
```

---

### Task 2: Wire `WDIO_SHARD` into the config

`getSpecs()` returns globs today. When `WDIO_SHARD` is set it must resolve those folders to files and return only this shard's files. Everything else in `getSpecs()` stays untouched.

**Files:**
- Modify: `tests/integration/wdio.server.conf.ts:3` (add `readdirSync` to the existing `fs` import)
- Modify: `tests/integration/wdio.server.conf.ts:50-89` (`TIER_SPECS`, `getSpecs`)

**Interfaces:**
- Consumes: `shardSpecs(files, current, total)` from Task 1.
- Produces: the `WDIO_SHARD` env contract, `"<current>/<total>"`, one-based, for example `WDIO_SHARD=3/6`. Task 3 sets it.

- [ ] **Step 1: Add the import**

In `tests/integration/wdio.server.conf.ts`, extend the existing `fs` import on line 3 and add the helper import next to it:

```ts
import { mkdtempSync, rmSync, existsSync, mkdirSync, readdirSync } from 'fs';
```

Then, below the other imports:

```ts
import { shardSpecs } from './utils/shard';
```

Note the asymmetry, it is deliberate and both forms were checked. The config uses the
extensionless specifier, matching every other import in the suite (for example
`import { seedVehicle } from '../../utils/db'`); the WDIO loader accepts it. The test
file in Task 1 must write `./shard.ts` instead, because `node --test` resolves through
plain Node ESM, where an extensionless specifier fails with `ERR_MODULE_NOT_FOUND`.

- [ ] **Step 2: Add the folder list and the resolver**

Directly below the existing `TIER_SPECS` declaration (line 50-55), add:

```ts
/**
 * The spec folders, in the order the shard split walks them. The order decides
 * which shard a file lands in, so keep it stable: changing it reshuffles every
 * shard. `./specs/env/**` is excluded on purpose, it has its own CI job.
 */
const SHARD_FOLDERS = ['tier1', 'tier2', 'tier3', 'existing'];

/** Resolve the tier folders to spec files, sorted, so the split is deterministic. */
function resolveAllSpecFiles(): string[] {
  return SHARD_FOLDERS.flatMap((folder) => {
    const dir = join(__dirname, 'specs', folder);
    return readdirSync(dir)
      .filter((file) => file.endsWith('.spec.ts'))
      .sort()
      .map((file) => join(dir, file));
  });
}
```

- [ ] **Step 3: Return the shard from `getSpecs()`**

In `getSpecs()`, add this block immediately after the `if (ENV_PINNED)` block and before `if (parallelMode)`:

```ts
  // CI shards by file. Local runs keep using TIER, which is untouched below.
  const shard = process.env.WDIO_SHARD;
  if (shard) {
    const [current, total] = shard.split('/').map(Number);
    const files = shardSpecs(resolveAllSpecFiles(), current, total);
    console.log(`Shard ${current}/${total}: ${files.length} spec files`);
    return files;
  }
```

- [ ] **Step 4: Verify every spec lands in exactly one shard**

Run:

```bash
for i in 1 2 3 4 5 6; do
  WDIO_SHARD="$i/6" node -e "import('./tests/integration/wdio.server.conf.ts').then(m => console.log(m.config.specs.join('\n')))"
done | grep -c '\.spec\.ts$'
```

Expected: `35`.

Then confirm there are no duplicates:

```bash
for i in 1 2 3 4 5 6; do
  WDIO_SHARD="$i/6" node -e "import('./tests/integration/wdio.server.conf.ts').then(m => console.log(m.config.specs.join('\n')))"
done | grep '\.spec\.ts$' | sort | uniq -d
```

Expected: no output.

- [ ] **Step 5: Verify the tier path still works**

Run:

```bash
TIER=1 node -e "import('./tests/integration/wdio.server.conf.ts').then(m => console.log(JSON.stringify(m.config.specs)))"
```

Expected, unchanged from today:

```
["./specs/tier1/**/*.spec.ts","./specs/existing/**/*.spec.ts"]
```

- [ ] **Step 6: Run one shard end to end**

Run: `WDIO_SHARD=5/6 npx wdio run tests/integration/wdio.server.conf.ts`

Expected: PASS. Shard 5 is the predicted heaviest: `phev-trips`, `column-visibility`, `odometer-cascade`, `route-autocomplete`, `vehicle-management`, `vehicle-setup`. WDIO spawns its own server on port 3457, so no container is needed.

- [ ] **Step 7: Commit**

```bash
git add tests/integration/wdio.server.conf.ts
git commit -m "test(83): shard specs by file when WDIO_SHARD is set"
```

---

### Task 3: Switch the CI matrix from tiers to shards

**Files:**
- Modify: `.github/workflows/test.yml:103` (job name), `:107-116` (matrix), `:172-177` (run step), `:191` (artifact name)

**Interfaces:**
- Consumes: the `WDIO_SHARD` contract from Task 2.
- Produces: 6 jobs named `Integration Tests (Docker/Chrome - Shard N of 6)`. The job id `integration-test-docker` does not change, so the `needs:` list of `publish-main-image` (line 302-310) keeps working untouched.

- [ ] **Step 1: Replace the job name and the matrix**

Replace lines 103-116:

```yaml
    name: Integration Tests (Docker/Chrome - ${{ matrix.tier_name }})
    needs: [check-changes, integration-build-docker]
    if: needs.check-changes.outputs.run_tests == 'true'
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        include:
          - tier: '1'
            tier_name: 'Tier 1'
          - tier: '2'
            tier_name: 'Tier 2'
          - tier: '3'
            tier_name: 'Tier 3'
```

with:

```yaml
    name: Integration Tests (Docker/Chrome - Shard ${{ matrix.shard }} of 6)
    needs: [check-changes, integration-build-docker]
    if: needs.check-changes.outputs.run_tests == 'true'
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        # Shards, not tiers. The tiers group specs by meaning and are unbalanced:
        # tier 2 ran 293 s of tests while tier 3 ran 22 s. See _tasks/83-*/01-task.md.
        shard: [1, 2, 3, 4, 5, 6]
```

- [ ] **Step 2: Replace the run step**

Replace lines 172-177:

```yaml
      - name: Run Docker integration tests (${{ matrix.tier_name }})
        env:
          TIER: ${{ matrix.tier }}
          WDIO_EXTERNAL_SERVER: '1'
          PARALLEL_TIERS: 'true'
        run: npm run test:integration:docker
```

with:

```yaml
      - name: Run Docker integration tests (shard ${{ matrix.shard }})
        env:
          WDIO_SHARD: ${{ matrix.shard }}/6
          WDIO_EXTERNAL_SERVER: '1'
        run: npm run test:integration:docker
```

- [ ] **Step 3: Fix the artifact name**

The artifact name cannot contain the `/` of a shard string, so it uses the bare index. Replace line 191:

```yaml
          name: integration-test-screenshots-docker-${{ matrix.tier_name }}
```

with:

```yaml
          name: integration-test-screenshots-docker-shard-${{ matrix.shard }}
```

- [ ] **Step 4: Verify the workflow parses and no tier reference is left**

Run:

```bash
python3 -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/test.yml')); j=d['jobs']['integration-test-docker']; print('shards:', j['strategy']['matrix']['shard']); print('needs of publish:', d['jobs']['publish-main-image']['needs'])"
grep -n "matrix.tier\|TIER:\|PARALLEL_TIERS" .github/workflows/test.yml
```

Expected: `shards: [1, 2, 3, 4, 5, 6]`, the publish `needs` list still contains `integration-test-docker`, and the `grep` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/test.yml
git commit -m "ci(83): shard the integration matrix six ways instead of by tier"
```

---

### Task 4: Verify the balance on a real run

The projection is arithmetic, not a measurement. This task replaces it with the measured numbers and records them.

**Files:**
- Modify: `_tasks/83-integration-test-sharding/01-task.md` (the projection table, replaced by measured values)
- Create: `_tasks/83-integration-test-sharding/03-results.md`

**Interfaces:**
- Consumes: a green CI run of the sharded matrix.
- Produces: the measured per-shard timings, and a decision on whether the split needs weighting.

- [ ] **Step 1: Push the branch and wait for the run**

```bash
git push -u origin HEAD
RUN=$(gh run list --workflow test.yml --limit 1 --json databaseId --jq '.[0].databaseId')
gh run watch "$RUN"
```

Expected: all 6 shard jobs conclude `success`.

- [ ] **Step 2: Read the per-shard step timings**

```bash
gh run view "$RUN" --json jobs --jq '.jobs[] | select(.name|test("Shard")) | "\(.name)\t\(.steps[] | select(.name|test("Run Docker")) | ((.completedAt|fromdate)-(.startedAt|fromdate)))s"'
```

Expected: every shard's test step under 100 s, giving a job under 2m30 once the 40 s of setup is added. Predicted heaviest is shard 5 at about 96 s of test step, 2m16 as a job.

- [ ] **Step 3: Check each shard against the balance formula**

For each shard, its test step should be close to `sum(its spec durations) + 2.9 x (its spec count)`. The 2.9 s is the per-spec Chrome session cost derived in [01-task.md](01-task.md). If a shard overshoots the formula, the skew is in spec count. If the formula holds but the spread across shards is wide, the skew is in duration, and the fix is a duration-weighted split, not more shards.

- [ ] **Step 4: Write the results file**

Create `_tasks/83-integration-test-sharding/03-results.md`, filling every bracket with a
measured value:

```markdown
**Date:** YYYY-MM-DD
**Subject:** Measured shard balance after the matrix change
**Status:** Complete

# Result

Run `[run id]`, [N] shards.

| Shard | Specs | Test step | Job total | Formula: sum(durations) + 2.9 x specs |
|---|---|---|---|---|
| 1 | [n] | [x] s | [m]m[s] | [y] s |

Slowest job: [m]m[s], against 5m26 before. Pipeline: [m]m[s] against about 10m.

The split [does / does not] need duration weighting, because [the spread across shards
is [x] s, which is [within / beyond] the 2.9 s per-spec noise].
```

- [ ] **Step 5: Replace the projection in `01-task.md` with the measurement**

Change the "Projected result" heading to "Result", replace the projected table with the measured one, and link to `03-results.md`.

- [ ] **Step 6: Commit**

```bash
git add _tasks/83-integration-test-sharding/
git commit -m "docs(83): record the measured shard balance"
```

---

## Rollback

One revert of Task 3's commit restores the tier matrix. Tasks 1 and 2 are inert without `WDIO_SHARD`, so they can stay.
