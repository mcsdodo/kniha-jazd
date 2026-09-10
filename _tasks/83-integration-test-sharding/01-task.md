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

## Projected result

435 s of test work to spread across the pool (tiers 1 to 3), 40 s fixed per job:

The floor assumes a perfect split. The second column is the real one: each spec
placed by round-robin, using its measured duration plus 2.9 s of session startup,
plus 40 s of job setup.

| Shards | Perfect split (floor) | Round-robin, computed | vs today (5m26) |
|---|---|---|---|
| 4 | 2m29 | 2m57 | -2m29 |
| 5 | 2m08 | 2m37 | -2m49 |
| **6** | **1m53** | **2m16** | **-3m10** |

Take 6. Round-robin does not balance perfectly, so the gap between the two columns
is the price of not maintaining a duration table. Two floors sit under both columns:
the 40 s job setup, and the longest single spec, which cannot be split
(`odometer-cascade.spec.ts`, 31.1 s plus session startup).

Pipeline effect: 4m33 build + 5m26 tests today, against 4m33 + 2m16 after. About
10 minutes becomes under 7.

## Risks

- **The split is by file count, not by duration.** With 35 files ranging from 0.9 s
  to 31.1 s, round-robin leaves a real gap: the heaviest shard is predicted at 79 s
  of test time against a 56 s average. Adding a spec can shift every assignment. If
  the gap widens, the fix is a duration-weighted split, not more shards.
- **Spec order changes.** Cross-spec leaks are order-dependent today:
  `datetime-is-order` fails under a full-tier run and passes alone, absorbed by
  `specFileRetries: 2`. Land [Task 82](../82-integration-db-reset/) first.
- Screenshot artifact names are keyed on `matrix.tier_name`
  ([test.yml:191](../../.github/workflows/test.yml)) and must follow the matrix.

## Acceptance criteria

- [ ] The matrix runs shards, not tiers, and every spec file runs exactly once.
- [ ] Slowest integration job is under 2m30 on a green run (predicted 2m16).
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
