/**
 * Writes three Fuelio drives into a folder, in the format Fuelio backs up:
 * `route-<epoch ms>.data`, a zip with one headerless CSV `route-<epoch ms>.csv`
 * (timestamp ms, lat, lon, metres from the previous fix, speed m/s, altitude,
 * accuracy).
 *
 * The drives are dated 10 and 11 January of the CURRENT year, so the page shows
 * them with its default year and the fixture never goes stale. Two are about
 * 22 km due north from (48.0, 17.0) to (48.2, 17.0), above the page's default
 * 15 km filter. With no trips seeded, both are "missing in the logbook".
 *
 * The third is a 3 km fragment that starts at (48.2, 17.0) two hours after the
 * first drive. The default 15 km filter hides it. With a trip on the first
 * drive, it is a loose match of the same trip: one trip, two rows.
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

/**
 * A drive due north from `lat0`, longitude 17.0, in `steps` fixes of
 * `stepDeg` degrees of latitude (0.1 degree = 11.12 km), 20 m/s.
 */
function driveCsv(startMs, lat0, steps, stepDeg) {
  const segM = Math.round(stepDeg * 111_200);
  const rows = [];
  for (let i = 0; i <= steps; i++) {
    const seg = i === 0 ? 0 : segM;
    const t = startMs + i * Math.round((segM / 20) * 1000);
    rows.push(`${t},${(lat0 + i * stepDeg).toFixed(4)},17.0000,${seg},20.0,150,5`);
  }
  return rows.join('\n') + '\n';
}

/** The drives: epoch-ms ID (the start, UTC) and track, oldest first. */
export function fuelioDrives(year = new Date().getFullYear()) {
  return [
    { id: String(Date.UTC(year, 0, 10, 9, 0)), lat0: 48.0, steps: 2, stepDeg: 0.1 },
    { id: String(Date.UTC(year, 0, 10, 11, 0)), lat0: 48.2, steps: 1, stepDeg: 0.027 },
    { id: String(Date.UTC(year, 0, 11, 9, 0)), lat0: 48.0, steps: 2, stepDeg: 0.1 },
  ];
}

export function writeFuelioDrives(dir) {
  mkdirSync(dir, { recursive: true });
  for (const { id, lat0, steps, stepDeg } of fuelioDrives()) {
    const csv = Buffer.from(driveCsv(Number(id), lat0, steps, stepDeg));
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
  console.log(`Wrote ${fuelioDrives().length} Fuelio drives to ${dir}`);
}
