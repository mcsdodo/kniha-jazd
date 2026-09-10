**Date:** 2026-09-10
**Subject:** Measured shard balance after the matrix change
**Status:** Complete

# Result

Two green runs on PR [#7](https://github.com/mcsdodo/kniha-jazd/pull/7), 6 shards each,
70 spec executions, **zero retries**. Every timing is a first-attempt measurement.
Run A is [`34480532011`](https://github.com/mcsdodo/kniha-jazd/actions/runs/34480532011),
run B is [`34481357652`](https://github.com/mcsdodo/kniha-jazd/actions/runs/34481357652).

| Shard | Specs | Mocha time A / B | Cold start A / B | Job total A / B |
|---|---|---|---|---|
| 1 | 6 | 47.2 / 48.6 s | 11.1 / 12.4 s | 1m48 / 2m00 |
| 2 | 6 | 42.8 / 42.8 s | 21.7 / 10.6 s | 1m58 / 1m46 |
| 3 | 6 | 61.7 / 60.3 s | 12.5 / **31.1** s | 2m14 / **2m24** |
| 4 | 6 | 61.7 / 64.0 s | 8.5 / 8.9 s | 2m08 / 2m10 |
| **5** | 6 | **75.4 / 75.7 s** | **28.7** / 3.5 s | **2m42** / 2m07 |
| 6 | 5 | 31.8 / 31.8 s | 12.6 / 7.2 s | 1m32 / 1m32 |

All 7 integration jobs of run A started at `13:06:41Z`, the same second, so the pool
ran concurrently. No job queued.

The two runs separate the two effects cleanly:

- **Mocha time is deterministic.** Every shard reproduces within 2.3 s. Shard 5 really
  does hold the most work (75.4 then 75.7 s), and shard 6 the least (31.8 s both times).
- **The cold start is runner noise.** It moved from 28.7 s to 3.5 s on shard 5, and from
  12.5 s to 31.1 s on shard 3. Range across 12 samples: 3.5 s to 31.1 s.

So the slowest job is not a fixed number. It is 2m42 in run A and 2m24 in run B, because
the bad cold-start draw landed on different shards.

## Speed, before and after

| | Before (`34349086458`) | After, run A | After, run B |
|---|---|---|---|
| Slowest integration job | 5m26 (Tier 2) | **2m42** (Shard 5) | **2m24** (Shard 3) |
| Fastest integration job | 0m56 (Tier 3) | 1m32 (Shard 6) | 1m32 (Shard 6) |
| Integration stage wall time | 5m26 | 2m42 | 2m24 |

The integration stage costs 164 s less in the worse of the two runs, a fall of 50%.

**Do not read the full-pipeline numbers as a win from this change.** The whole run
went from 641 s to 213 s, but 235 s of that is the Docker Image Build, which fell
from 273 s to 38 s on a warm `type=gha` cache. This task does not touch the build.
The honest number is the integration stage: 5m26 to 2m42.

## The balance formula

The plan predicted `sum(durations) + 2.9 x specs`. The measurement splits that
constant in two:

- **Steady state is 2.4 s per spec**, and it is very stable. Across all 58 non-first
  workers in the two runs, the wall clock minus mocha time sits between 1.9 s and 2.7 s.
- **The first worker of a job costs much more**, and it varies by runner: 3.5, 7.2, 8.5,
  8.9, 10.6, 11.1, 12.4, 12.5, 12.6, 21.7, 28.7, 31.1 s. This is Chrome and chromedriver
  cold start. It is a per-job cost, not a per-spec one, and no split can remove it.

The plan's flat 2.9 s per spec absorbed this per-job cost into a per-spec constant. That
is the model error behind the 2m16 projection.

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

Both "worst job" figures assume a median 12 s cold start and 40 s of fixed job setup.
The 40 s is measured on this run, not carried over: shard 5 ran a 162 s job around a
124 s test step (38 s), and shard 6 a 92 s job around a 58 s step (34 s).

Weighting matters most in the worst case, not the median. The bad cold-start draw of
about 31 s can land on any shard, so the slowest job is `heaviest shard + 31 s`:

| Split | Heaviest shard | Worst-case job |
|---|---|---|
| Round-robin (shipped) | 75.7 s | about 2m42, which is what run A measured |
| Duration-weighted (LPT) | 53.8 s | about 2m20 |

Round-robin therefore *cannot* hold under 2m30 when the bad draw hits shard 5. Weighting
holds under it on every draw. It costs a duration table that a new spec must be added to.
That is a separate task, not a change to this one: the sharding mechanism is the same
either way, only the assignment function changes.

The floor stays where the plan put it: 88 s, being the longest single spec
(`odometer-cascade`, 30.5 s) plus its session cost, cold start and 40 s of job setup.
