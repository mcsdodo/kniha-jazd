/**
 * Local mock release server for testing auto-update flow.
 *
 * Usage:
 *   node _test-releases/serve.js
 *
 * Then configure TAURI_UPDATER_ENDPOINT=http://localhost:3456/latest.json
 * before running `npm run tauri dev`
 */

const http = require('http');
const fs = require('fs');
const path = require('path');

const PORT = 3456;
const BASE_DIR = __dirname;

const MIME_TYPES = {
  '.json': 'application/json',
  '.exe': 'application/octet-stream',
  '.sig': 'text/plain',
  '.gz': 'application/gzip',
  '.tar.gz': 'application/gzip',
};

http.createServer((req, res) => {
  console.log(`${new Date().toISOString()} ${req.method} ${req.url}`);

  // Handle root as latest.json
  const urlPath = req.url === '/' ? '/latest.json' : req.url;
  const filePath = path.join(BASE_DIR, urlPath);

  if (!fs.existsSync(filePath)) {
    console.log(`  -> 404 Not Found`);
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
  1. Build a test release: npm run tauri build
  2. Copy installer to _test-releases/releases/v0.16.0/
  3. Update latest.json with correct signature
  4. Run app with: set TAURI_UPDATER_ENDPOINT=http://localhost:${PORT}/latest.json && npm run tauri dev

Press Ctrl+C to stop.
`);
});
