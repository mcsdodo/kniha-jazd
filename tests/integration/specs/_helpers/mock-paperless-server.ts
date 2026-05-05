/**
 * Mock Paperless-ngx HTTP server for integration tests.
 *
 * Spawns a tiny Node `http` server on a free loopback port and serves the
 * minimal subset of the Paperless API the backend hits during the Tier-2
 * integration test:
 *
 *   - GET /api/ui_settings/         (auth probe used by test_paperless_connection)
 *   - GET /api/tags/?name__iexact=  (resolve fuel/car tag IDs)
 *   - GET /api/custom_fields/       (resolve total_price_eur / liters / receipt_datetime IDs)
 *   - GET /api/documents/           (returns 3 fixture invoices: doc 435 fuel, 423 + 391 car)
 *
 * Tauri runs in a separate process but shares loopback, so binding to
 * `127.0.0.1` is reachable from the backend HTTP client.
 */

import http from 'http';
import { URL } from 'url';

export const MOCK_PAPERLESS_TOKEN = 'paperless-test-token';

let server: http.Server | null = null;
let baseUrl = '';

export async function startMockPaperless(): Promise<string> {
  if (server && baseUrl) return baseUrl;

  server = http.createServer((req, res) => {
    const auth = req.headers['authorization'];
    if (auth !== `Token ${MOCK_PAPERLESS_TOKEN}`) {
      res.writeHead(401, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ detail: 'Unauthorized' }));
      return;
    }

    // baseUrl is set after listen() resolves; until then use a placeholder
    // so URL parsing works for the very first request (shouldn't happen,
    // but defensive).
    const url = new URL(req.url || '', baseUrl || 'http://127.0.0.1');

    const send = (body: unknown, status = 200): void => {
      res.writeHead(status, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(body));
    };

    if (req.method === 'GET' && url.pathname === '/api/ui_settings/') {
      return send({});
    }

    if (req.method === 'GET' && url.pathname === '/api/tags/') {
      const name = url.searchParams.get('name__iexact');
      if (name === 'fuel') return send({ count: 1, results: [{ id: 51, name: 'fuel' }] });
      if (name === 'car') return send({ count: 1, results: [{ id: 59, name: 'car' }] });
      return send({ count: 0, results: [] });
    }

    if (req.method === 'GET' && url.pathname === '/api/custom_fields/') {
      return send({
        count: 3,
        results: [
          { id: 1, name: 'total_price_eur',  data_type: 'float'  },
          { id: 5, name: 'liters',           data_type: 'float'  },
          { id: 6, name: 'receipt_datetime', data_type: 'string' },
        ],
      });
    }

    const DOCS: Record<number, unknown> = {
      435: {
        id: 435,
        title: 'OMV Slovensko, s.r.o. - Scanned_20260427-1325',
        tags: [54, 55, 51, 48], // includes fuel (51)
        created: '2026-04-27',
        custom_fields: [
          { value: 113.95, field: 1 }, // total_price_eur
          { value: 63.34,  field: 5 }, // liters
          { value: '2026-04-27T13:24:14', field: 6 }, // receipt_datetime
        ],
      },
      423: {
        id: 423,
        title: 'Hlavné mesto SR Bratislava - 1776180674432',
        tags: [54, 55, 59, 48], // includes car (59)
        created: '2026-04-14',
        custom_fields: [
          { value: 1.95, field: 1 }, // total_price_eur
          { value: '1776180674432', field: 4 }, // unrelated field
          { value: '2026-04-14T15:31:00', field: 6 }, // receipt_datetime
        ],
      },
      391: {
        id: 391,
        title: 'Mataso s.r.o. - 0003',
        tags: [44, 55, 59, 48], // includes car (59)
        created: '2026-03-27',
        custom_fields: [
          { value: 110.0, field: 1 }, // total_price_eur
          { value: '0003', field: 4 }, // unrelated field
          { value: '2026-03-27T14:41:00', field: 6 }, // receipt_datetime
        ],
      },
    };

    if (req.method === 'GET' && url.pathname === '/api/documents/') {
      return send({ count: Object.keys(DOCS).length, next: null, results: Object.values(DOCS) });
    }

    // Single-document fetch: GET /api/documents/{id}/
    const singleDocMatch = url.pathname.match(/^\/api\/documents\/(\d+)\/$/);
    if (req.method === 'GET' && singleDocMatch) {
      const doc = DOCS[parseInt(singleDocMatch[1], 10)];
      if (doc) return send(doc);
      return send({ detail: 'Not found' }, 404);
    }

    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ detail: 'Not found' }));
  });

  await new Promise<void>((resolve, reject) => {
    server!.once('error', reject);
    server!.listen(0, '127.0.0.1', () => resolve());
  });

  const addr = server.address();
  if (!addr || typeof addr === 'string') {
    throw new Error('Mock Paperless server failed to bind');
  }
  baseUrl = `http://127.0.0.1:${addr.port}`;
  return baseUrl;
}

export async function stopMockPaperless(): Promise<void> {
  if (!server) return;
  await new Promise<void>((resolve, reject) => {
    server!.close((err) => (err ? reject(err) : resolve()));
  });
  server = null;
  baseUrl = '';
}
