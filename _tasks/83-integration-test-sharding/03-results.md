**Date:** 2026-09-10
**Subject:** Measured shard balance after the matrix change
**Status:** Complete

# Result

Run [`34480532011`](https://github.com/mcsdodo/kniha-jazd/actions/runs/34480532011)
(PR [#7](https://github.com/mcsdodo/kniha-jazd/pull/7)), 6 shards, all green.
No spec retried, so every timing below is a first-attempt measurement.

| Shard | Specs | Mocha time | Test step | Job total |
|---|---|---|---|---|
| 1 | 6 | 47.2 s | 74 s | 1m48 |
| 2 | 6 | 42.8 s | 81 s | 1m58 |
| 3 | 6 | 61.7 s | 91 s | 2m14 |
| 4 | 6 | 61.7 s | 87 s | 2m08 |
| **5** | 6 | **75.4 s** | **124 s** | **2m42** |
| 6 | 5 | 31.8 s | 58 s | 1m32 |

All 7 integration jobs started at `13:06:41Z`, the same second, so the pool ran
concurrently. No job queued.

## Speed, before and after

| | Before (run `34349086458`) | After (run `34480532011`) |
|---|---|---|
| Slowest integration job | 5m26 (Tier 2) | **2m42 (Shard 5)** |
| Fastest integration job | 0m56 (Tier 3) | 1m32 (Shard 6) |
| Integration stage wall time | 5m26 | **2m42** |

The integration stage costs 164 s less, a fall of 50%.

**Do not read the full-pipeline numbers as a win from this change.** The whole run
went from 641 s to 213 s, but 235 s of that is the Docker Image Build, which fell
from 273 s to 38 s on a warm `type=gha` cache. This task does not touch the build.
The honest number is the integration stage: 5m26 to 2m42.

## The balance formula

The plan predicted `sum(durations) + 2.9 x specs`. The measurement splits that
constant in two:

- **Steady state is 2.4 s per spec**, and it is very stable. Across all 30 non-first
  workers the wall-clock minus mocha time sits between 2.2 s and 2.7 s.
- **The first worker of a run costs much more**, and it varies by runner: 8.5 s,
  11.1 s, 12.5 s, 12.6 s, 21.7 s, 28.7 s. This is Chrome and chromedriver cold start.
  It is a fixed per-job cost, not a per-spec one, and no split can remove it.

Shard 5 drew the worst of the six (28.7 s against a 12.6 s median). Its first spec,
`phev-trips` at 3.6 s of mocha time, took 32.3 s of wall clock.

## Two acceptance criteria, checked

| Criterion | Result |
|---|---|
| The matrix runs shards, and every spec runs exactly once | **Pass.** 35 files across 6 shards, no duplicates. |
| Slowest integration job under 2m30 | **Miss.** 2m42. |
| `npm run test:integration:tier1` unchanged | **Pass.** The `TIER` globs are byte-identical. |
| Screenshot artifacts have distinct names | **Pass.** Keyed on `matrix.shard`. |

## The split needs duration weighting

The plan set the decision rule: if the formula holds but the spread across shards is
wide, weight by duration. Both halves are now measured.

The formula holds (2.4 s per spec, tight). The spread does not: mocha time per shard
runs 31.8 s to 75.4 s, a spread of 43.6 s against a 53.4 s ideal. That is 18 times the
per-spec noise. Round-robin split the *count* evenly and the *cost* badly, because
`odometer-cascade` alone (30.5 s) is as large as the whole of shard 6.

A greedy longest-processing-time split over the same 35 specs gives:

| Split | Per-shard mocha time | Spread | Worst job at a median cold start |
|---|---|---|---|
| Round-robin (shipped) | 47.2, 42.8, 61.7, 61.7, 75.4, 31.8 | 43.6 s | about 2m25 |
| Duration-weighted (LPT) | 52.9, 53.4, 53.8, 53.6, 53.2, 53.7 | 0.9 s | about 2m03 |

Weighting is worth about 22 s more, and it would put the slowest job under the 2m30
target with margin against a bad cold start. It costs a duration table that a new spec
must be added to. That is a separate task, not a change to this one: the sharding
mechanism is the same either way, only the assignment function changes.

The floor stays where the plan put it: 88 s, being the longest single spec
(`odometer-cascade`, 30.5 s) plus its session cost, cold start and 40 s of job setup.
