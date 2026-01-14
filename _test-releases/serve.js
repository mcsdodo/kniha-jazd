/**
 * Local mock release server for testing auto-update flow.
 *
 * Usage:
 *   node _test-releases/serve.js
 *
 * Then run app with:
 *   set TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json && npm run tauri dev
 */

import http from 'http';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const PORT = 3456;
const BASE_DIR = __dirname;

const MIME_TYPES = {
  '.json': 'application/json',
  '.exe': 'application/octet-stream',
  '.sig': 'text/plain',
  '.gz': 'application/gzip',
};

http.createServer((req, res) => {
  console.log(`${new Date().toISOString()} ${req.method} ${req.url}`);

  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  // Handle preflight
  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    return res.end();
  }

  // Handle root as latest.json
  const urlPath = req.url === '/' ? '/latest.json' : decodeURIComponent(req.url);
  const filePath = path.join(BASE_DIR, urlPath);

  if (!fs.existsSync(filePath)) {
    console.log(`  -> 404 Not Found: ${filePath}`);
    res.writeHead(404);
    return res.end('Not found');
  }

  const ext = path.extname(filePath);
  const contentType = MIME_TYPES[ext] || 'application/octet-stream';
  const stat = fs.statSync(filePath);

  console.log(`  -> 200 OK (${contentType}, ${stat.size} bytes)`);

  res.writeHead(200, {
    'Content-Type': contentType,
    'Content-Length': stat.size,
  });

  fs.createReadStream(filePath).pipe(res);
}).listen(PORT, () => {
  console.log(`
============================================
  Mock Release Server running on port ${PORT}
============================================

Endpoints:
  http://localhost:${PORT}/latest.json
  http://localhost:${PORT}/releases/v0.16.0/...

To test updates:
  1. In another terminal, run:
     set TAURI_UPDATER_ENDPOINT=http://localhost:${PORT}/latest.json && npm run tauri dev

Press Ctrl+C to stop.
`);
});
