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
