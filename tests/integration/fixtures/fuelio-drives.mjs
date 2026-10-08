/**
 * Writes two Fuelio drives into a folder, in the format Fuelio backs up:
 * `route-<epoch ms>.data`, a zip with one headerless CSV `route-<epoch ms>.csv`
 * (timestamp ms, lat, lon, metres from the previous fix, speed m/s, altitude,
 * accuracy).
 *
 * The drives are dated 10 and 11 January of the CURRENT year, so the page shows
 * them with its default year and the fixture never goes stale. Each is about
 * 22 km, above the page's default 15 km filter. With no trips seeded, both are
 * "missing in the logbook".
 *
 * Plain Node, no dependencies: CI runs it before `docker run` on Node 20
 * (`node tests/integration/fixtures/fuelio-drives.mjs data-env/fuelio`), and
 * wdio.server.conf.ts imports it for the spawned server.
 */

import { mkdirSync, writeFileSync } from 'fs';
import { join } from 'path';
import { pathToFileURL } from 'url';

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/** A zip with one stored (uncompressed) file. */
function zipOne(name, data) {
  const nameBuf = Buffer.from(name);
  const crc = crc32(data);
  const local = Buffer.alloc(30);
  local.writeUInt32LE(0x04034b50, 0);
  local.writeUInt16LE(20, 4); // version needed
  local.writeUInt32LE(crc, 14);
  local.writeUInt32LE(data.length, 18);
  local.writeUInt32LE(data.length, 22);
  local.writeUInt16LE(nameBuf.length, 26);
  const central = Buffer.alloc(46);
  central.writeUInt32LE(0x02014b50, 0);
  central.writeUInt16LE(20, 4); // version made by
  central.writeUInt16LE(20, 6); // version needed
  central.writeUInt32LE(crc, 16);
  central.writeUInt32LE(data.length, 20);
  central.writeUInt32LE(data.length, 24);
  central.writeUInt16LE(nameBuf.length, 28);
  const centralOffset = local.length + nameBuf.length + data.length;
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(1, 8);
  end.writeUInt16LE(1, 10);
  end.writeUInt32LE(central.length + nameBuf.length, 12);
  end.writeUInt32LE(centralOffset, 16);
  return Buffer.concat([local, nameBuf, data, central, nameBuf, end]);
}

/** About 22 km due north, one fix per 11.1 km (0.1 degree of latitude). */
function driveCsv(startMs) {
  const rows = [];
  for (let i = 0; i <= 2; i++) {
    const seg = i === 0 ? 0 : 11120;
    rows.push(`${startMs + i * 556_000},${(48.0 + i * 0.1).toFixed(4)},17.0000,${seg},20.0,150,5`);
  }
  return rows.join('\n') + '\n';
}

/** The epoch-ms IDs of the drives, oldest first. */
export function fuelioDriveIds(year = new Date().getFullYear()) {
  return [Date.UTC(year, 0, 10, 9, 0), Date.UTC(year, 0, 11, 9, 0)].map(String);
}

export function writeFuelioDrives(dir) {
  mkdirSync(dir, { recursive: true });
  for (const id of fuelioDriveIds()) {
    const csv = Buffer.from(driveCsv(Number(id)));
    writeFileSync(join(dir, `route-${id}.data`), zipOne(`route-${id}.csv`, csv));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const dir = process.argv[2];
  if (!dir) {
    console.error('usage: node fuelio-drives.mjs <folder>');
    process.exit(1);
  }
  writeFuelioDrives(dir);
  console.log(`Wrote ${fuelioDriveIds().length} Fuelio drives to ${dir}`);
}
