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
