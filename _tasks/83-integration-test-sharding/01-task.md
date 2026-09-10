**Date:** 2026-09-10
**Subject:** Shard the integration specs by file across the CI matrix instead of by semantic tier
**Status:** Complete

**Depends on:** [Task 82](../82-integration-db-reset/) -- sharding reorders which specs share a database

## Goal

Cut the integration stage from 5m26 to about 2m30 by giving each parallel CI job
an equal share of the work. Today one job does 67% of it.

## Evidence

Step timings from run `34349086458` (a normal green push to `main`):

| Job | Fixed setup | Test step | Job total |
|---|---|---|---|
| Tier 1 (9 specs + 2 in `existing/`) | 46 s | 120 s | 2m46 |
| **Tier 2 (20 specs)** | 33 s | **293 s** | **5m26** |
| Tier 3 (4 specs) | 34 s | 22 s | 0m56 |
| Env-pinned (1 spec) | 44 s | 20 s | 1m04 |

The four jobs run in parallel, so the stage costs 5m26 while two of them finish
inside a minute. The fixed setup is the same everywhere: Setup Node (5 to 8 s),
Setup Chrome (9 to 11 s), npm install (5 to 8 s), Load Docker image (4 to 11 s).
Call it 40 s per job. That is the floor a new job cannot go below.

## Why the tiers are unbalanced

They are not a partition for speed. They group specs by meaning: tier 1 is the
critical path, tier 2 is features, tier 3 is edge cases
([getSpecs()](../../tests/integration/wdio.server.conf.ts), lines 58-89). Features
outnumber the rest, so tier 2 holds 20 spec files and tier 3 holds 4. The grouping
is right for local work, where `npm run test:integration:tier1` is the fast check.
It is the wrong axis for CI parallelism.

## Approach

Decouple the two. CI drops `TIER` and `PARALLEL_TIERS` and passes a shard index
instead; the tier npm scripts stay exactly as they are for local use, so invariant
I1 still holds through `test:integration:docker`. The env-pinned job is untouched,
because it needs its own container environment and cannot join the pool.

**Do not use WDIO's own `--shard`.** It slices the spec list *contiguously*
(`node_modules/@wdio/config/build/node/index.js:567-576`:
`specs.slice(current * specsPerShard - specsPerShard, end)`), and our spec list is
grouped by tier. At 4 shards, shard 2 would draw `odometer-cascade` (31.1 s),
`legal-compliance` (23.4 s), `copy-trip` (18.9 s), `column-visibility` (17.8 s) and
`datetime-is-order` (16.1 s) together: 3m16, worse than the target.

Shard round-robin instead (`index % total === current - 1`), which interleaves the
tiers. `getSpecs()` resolves the tier folders to files and returns only this shard's
files. See [02-plan.md](02-plan.md) for the implementation.

## Result

Measured on run [`34480532011`](https://github.com/mcsdodo/kniha-jazd/actions/runs/34480532011),
6 shards, all green, no retries. Full numbers in [03-results.md](03-results.md).

| Shard | Specs | Mocha time | Test step | Job total |
|---|---|---|---|---|
| 1 | 6 | 47.2 s | 74 s | 1m48 |
| 2 | 6 | 42.8 s | 81 s | 1m58 |
| 3 | 6 | 61.7 s | 91 s | 2m14 |
| 4 | 6 | 61.7 s | 87 s | 2m08 |
| **5** | 6 | **75.4 s** | **124 s** | **2m42** |
| 6 | 5 | 31.8 s | 58 s | 1m32 |

The integration stage went from 5m26 to 2m42, a fall of 50%. The predicted 2m16 was
not reached, for two measured reasons:

- Round-robin balances spec *count*, not cost. Shard 5 holds 75.4 s of work against a
  53.4 s ideal, because `odometer-cascade` alone (30.5 s) is as large as all of shard 6.
- The first worker of each job pays a Chrome cold start that the projection folded into
  a flat per-spec constant. It is per-job, not per-spec, and it varied from 8.5 s to
  28.7 s across the six. Shard 5 drew the worst one.

The projection said 2m16. The two corrections above account for the 26 s gap.

**Follow-up:** a duration-weighted split cuts the spread from 43.6 s to 0.9 s and the
slowest job to about 2m03. See [03-results.md](03-results.md), "The split needs
duration weighting".

Pipeline effect: this task does not touch the 4m33 Docker Image Build, so the stage
saving of 2m44 is the whole of the win.

## Risks

- **The split is by file count, not by duration.** With 35 files ranging from 0.9 s
  to 31.1 s, round-robin leaves a real gap: the heaviest shard is predicted at 79 s
  of test time against a 56 s average. Adding a spec can shift every assignment. If
  the gap widens, the fix is a duration-weighted split, not more shards.
- **Spec order changes.** Cross-spec leaks are order-dependent today:
  `datetime-is-order` fails under a full-tier run and passes alone, absorbed by
  `specFileRetries: 2`. Land [Task 82](../82-integration-db-reset/) first.
  *Outcome:* run `34480532011` passed all 35 specs with zero retries, so the new order
  surfaced no leak. That is one sample, not a proof. Keep the Task 82 dependency.
- Screenshot artifact names are keyed on `matrix.tier_name`
  ([test.yml:191](../../.github/workflows/test.yml)) and must follow the matrix.

## Acceptance criteria

- [x] The matrix runs shards, not tiers, and every spec file runs exactly once.
- [ ] Slowest integration job is under 2m30 on a green run (predicted 2m16).
      **Missed: 2m42.** See [03-results.md](03-results.md); duration weighting closes it.
- [x] `npm run test:integration:tier1` still works locally, unchanged.
- [x] Screenshot artifacts still upload with distinct names.
- [x] Verify shard balance from the step timings of the first green run. Done, and the
      formula needed a correction: the per-spec session cost is 2.4 s, not 2.9 s, and it
      is tight. The remainder is a *per-job* Chrome cold start of 8.5 s to 28.7 s that the
      2.9 s constant had absorbed. The skew is in duration, not in spec count, so the fix
      is a weighted split, not a reordered glob. See [03-results.md](03-results.md).

## Out of scope

- **Docker Image Build, 4m33.** After sharding it is about 65% of the pipeline and
  becomes the next thing worth attacking. It needs its own task: Rust layer
  caching, `cache-from`/`cache-to`, or a prebuilt base.
- The 232 `browser.pause()` calls (102 s total, 81.7 s of it in tier 2). Worth
  doing, but it edits spec bodies and belongs in its own task.
