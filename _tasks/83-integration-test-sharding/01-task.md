**Date:** 2026-09-10
**Subject:** Shard the integration specs by file across the CI matrix instead of by semantic tier
**Status:** Planning

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

Decouple the two. WebdriverIO 9.23 shards natively, so `getSpecs()` needs no
change: CI drops `TIER` and `PARALLEL_TIERS`, `getSpecs()` returns all four tier
globs, and the matrix carries shard indices.

- [test.yml:107-116](../../.github/workflows/test.yml): the `tier` / `tier_name`
  matrix becomes `shard: ['1/4', '2/4', '3/4', '4/4']`.
- [test.yml:172-177](../../.github/workflows/test.yml): the run step becomes
  `npm run test:integration:docker -- --shard ${{ matrix.shard }}`, with the
  `TIER` and `PARALLEL_TIERS` env lines removed.
- The env-pinned job is untouched. It needs its own container environment and
  cannot join the pool.
- The tier npm scripts stay exactly as they are, for local use. Invariant I1 still
  holds through `test:integration:docker`.

## Projected result

435 s of test work to spread across the pool (tiers 1 to 3), 40 s fixed per job:

| Shards | Projected job | vs today (5m26) |
|---|---|---|
| 3 | 3m05 | -2m21 |
| **4** | **2m29** | **-2m57** |
| 5 | 2m07 | -3m19 |
| 6 | 1m53 | -3m33 |

Take 4. Returns flatten after that against two floors: the 40 s setup, and the
longest single spec file, which cannot be split (`odometer-cascade.spec.ts`,
31.1 s plus about 2.7 s of session startup).

Pipeline effect: 4m33 build + 5m26 tests today, against 4m33 + about 2m30 after.
About 10 minutes becomes about 7.

## Risks

- **Sharding splits by file count, not by duration.** WDIO divides the resolved
  spec list evenly by number of files. With 35 files ranging from 0.9 s to 31.1 s,
  one shard can draw several long specs. The worst plausible draw at 4 shards is
  about 97 s against a 109 s average, so it is tolerable. If it drifts, order the
  globs so the long specs spread across shards.
- **Spec order changes.** Cross-spec leaks are order-dependent today:
  `datetime-is-order` fails under a full-tier run and passes alone, absorbed by
  `specFileRetries: 2`. Land [Task 82](../82-integration-db-reset/) first.
- Screenshot artifact names are keyed on `matrix.tier_name`
  ([test.yml:191](../../.github/workflows/test.yml)) and must follow the matrix.

## Acceptance criteria

- [ ] The matrix runs shards, not tiers, and every spec file runs exactly once.
- [ ] Slowest integration job is under 3 minutes on a green run.
- [ ] `npm run test:integration:tier1` still works locally, unchanged.
- [ ] Screenshot artifacts still upload with distinct names.
- [ ] Verify shard balance from the step timings of the first green run, not from this
      estimate. A shard's step time should match `sum(its spec durations) + 2.9 x (its
      spec count)`. The 2.9 s is the per-spec Chrome session cost, derived from this run:
      tier 1 gives `A + 11s = 29.7`, tier 2 gives `A + 20s = 56.5`, so `s = 2.9` and the
      fixed wdio boot `A` is about zero. If a shard overshoots that formula, the skew is
      in spec count, and the fix is to reorder the globs, not to add a shard.

## Out of scope

- **Docker Image Build, 4m33.** After sharding it is about 65% of the pipeline and
  becomes the next thing worth attacking. It needs its own task: Rust layer
  caching, `cache-from`/`cache-to`, or a prebuilt base.
- The 232 `browser.pause()` calls (102 s total, 81.7 s of it in tier 2). Worth
  doing, but it edits spec bodies and belongs in its own task.
