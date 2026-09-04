/* eslint-disable @typescript-eslint/no-explicit-any */
import { spawn, ChildProcess } from 'child_process';
import { mkdtempSync, rmSync, existsSync, mkdirSync } from 'fs';
import { tmpdir } from 'os';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// ES module equivalent of __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * WDIO_ENV_PINNED=1 runs only the env-managed-settings suite, with the fixture
 * variables exported into the spawned server. Those variables make settings
 * read-only app-wide, which would break specs that edit them — hence a separate run.
 */
const ENV_PINNED = process.env.WDIO_ENV_PINNED === '1';

/**
 * Environment variables that pin settings for the ENV_PINNED run. Values are
 * fixtures — the HA/Paperless hosts don't have to resolve, the UI assertions only
 * care that the fields are pinned.
 */
const ENV_PINNED_FIXTURE: Record<string, string> = {
  HA_URL: 'http://env-pinned-ha.test:8123',
  HA_API_TOKEN: 'env-pinned-ha-token',
  PAPERLESS_URL: 'https://env-pinned-paperless.test',
  PAPERLESS_API_TOKEN: 'env-pinned-paperless-token',
  PAPERLESS_ENABLED: 'true',
  KNIHA_JAZD_REVEAL_PIN: '4269',
};

/**
 * Blank out every overridable settings variable for normal runs.
 *
 * WebdriverIO auto-loads the repo's .env, and a developer with a real
 * PAPERLESS_API_TOKEN or GEMINI_API_KEY there would pin those settings in the
 * spawned server — making the setter guards reject writes and specs like
 * paperless-integration fail with "managed by the ... environment variable".
 * Empty values read as unset (see LocalSettings::apply_overrides), so this keeps
 * runs hermetic. Mirrors scrub_ambient_env() in settings.rs for the Rust tests.
 */
const SCRUBBED_ENV: Record<string, string> = Object.fromEntries(
  Object.keys(ENV_PINNED_FIXTURE)
    .concat('GEMINI_API_KEY')
    .map((key) => [key, ''])
);

/** Every tier folder, enumerated so `./specs/env/**` is never picked up by accident. */
const TIER_SPECS = [
  './specs/tier1/**/*.spec.ts',
  './specs/tier2/**/*.spec.ts',
  './specs/tier3/**/*.spec.ts',
  './specs/existing/**/*.spec.ts',
];

/**
 * Get specs based on TIER and PARALLEL_TIERS environment variables
 */
function getSpecs(): string[] {
  const tier = process.env.TIER;
  const parallelMode = process.env.PARALLEL_TIERS === 'true';

  // The env-pinned suite needs ENV_PINNED_FIXTURE set before the server starts,
  // so it gets its own run and is never swept into a normal one.
  if (ENV_PINNED) {
    return ['./specs/env/**/*.spec.ts'];
  }

  if (parallelMode) {
    switch (tier) {
      case '1':
        return ['./specs/tier1/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
      case '2':
        return ['./specs/tier2/**/*.spec.ts'];
      case '3':
        return ['./specs/tier3/**/*.spec.ts'];
      default:
        return TIER_SPECS;
    }
  }

  // Sequential mode (original behavior)
  if (tier === '1') {
    return ['./specs/tier1/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
  } else if (tier === '2') {
    return ['./specs/tier1/**/*.spec.ts', './specs/tier2/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
  }
  return TIER_SPECS;
}

/**
 * Get the path to the headless web server binary (`kniha-jazd-web`).
 * CI can override via the KJ_WEB_BINARY env var.
 */
function getBinaryPath(): string {
  if (process.env.KJ_WEB_BINARY) {
    return process.env.KJ_WEB_BINARY;
  }

  const base = join(__dirname, '../../src-tauri/target/debug');

  return process.platform === 'win32'
    ? join(base, 'kniha-jazd-web.exe')
    : join(base, 'kniha-jazd-web');
}

let serverProcess: ChildProcess | null = null;
let testDataDir = '';
const EXTERNAL_SERVER = process.env.WDIO_EXTERNAL_SERVER === '1';
// External mode (Docker) defaults to port 3456; spawned-server mode uses 3457
// to avoid colliding with a running app or container.
const DEFAULT_PORT = EXTERNAL_SERVER ? 3456 : 3457;
const SERVER_PORT = process.env.WDIO_SERVER_PORT
  ? Number(process.env.WDIO_SERVER_PORT)
  : DEFAULT_PORT;
const SERVER_URL = process.env.WDIO_SERVER_URL || `http://localhost:${SERVER_PORT}`;

/**
 * Poll a URL until it responds with 200 OK, or time out.
 */
async function waitForUrl(url: string, timeoutMs: number): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const resp = await fetch(url);
      if (resp.ok) return;
    } catch {
      // Server not ready yet
    }
    await new Promise(r => setTimeout(r, 500));
  }
  throw new Error(`Timed out waiting for ${url} after ${timeoutMs}ms`);
}

/**
 * Reset the database via RPC. Trips must be deleted before vehicles because
 * SQLite enforces the trips.vehicle_id → vehicles.id FK.
 */
async function resetDatabase(serverUrl: string): Promise<void> {
  try {
    const rpc = async (cmd: string, args: Record<string, unknown> = {}) => {
      const resp = await fetch(`${serverUrl}/api/rpc`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-KJ-Client': '1' },
        body: JSON.stringify({ command: cmd, args }),
      });
      if (!resp.ok) throw new Error(`${cmd}: ${resp.status}`);
      return resp.json();
    };

    const vehicles = await rpc('get_vehicles') as Array<{ id: string }>;
    const currentYear = new Date().getFullYear();
    const yearsToCheck = [currentYear - 1, currentYear, currentYear + 1];

    for (const v of vehicles) {
      for (const year of yearsToCheck) {
        try {
          const trips = await rpc('get_trips_for_year', { vehicleId: v.id, year }) as Array<{ id: string }>;
          for (const trip of trips) {
            try {
              await rpc('delete_trip', { id: trip.id });
            } catch { /* ignore */ }
          }
        } catch { /* ignore */ }
      }
      try {
        await rpc('delete_vehicle', { id: v.id });
      } catch { /* ignore */ }
    }
  } catch (e) {
    console.warn('Database reset RPC failed:', e);
  }
}

// WebdriverIO configuration: Chrome browser against the headless HTTP server
export const config: any = {
  runner: 'local',
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tsconfig.json',
      transpileOnly: true,
    }
  },

  specs: getSpecs(),
  exclude: [],

  // Run one at a time — server mode shares a single backend instance
  maxInstances: 1,

  capabilities: [{
    browserName: 'chrome',
    'goog:chromeOptions': {
      args: ['--no-sandbox', '--disable-gpu'],
    },
  }],

  // Retry flaky tests up to 2 times before failing
  specFileRetries: 2,
  specFileRetriesDelay: 1,
  specFileRetriesDeferred: false,

  logLevel: 'info',
  bail: 0,
  baseUrl: SERVER_URL,
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,

  framework: 'mocha',
  reporters: ['spec'],

  mochaOpts: {
    ui: 'bdd',
    timeout: 30000,
  },

  /**
   * Before all tests: Start the headless web server binary, wait for HTTP ready.
   * If WDIO_EXTERNAL_SERVER=1 is set (Docker mode), skip the spawn — the server is
   * already running externally and we just wait for it to respond.
   */
  onPrepare: async function () {
    process.env.WDIO_SERVER_URL = SERVER_URL;

    // Mock Gemini API: load JSON from mocks/ instead of calling API
    process.env.KNIHA_JAZD_MOCK_GEMINI_DIR = join(__dirname, 'data', 'mocks');

    // Create screenshots directory if it doesn't exist
    const screenshotsDir = join(__dirname, 'screenshots');
    if (!existsSync(screenshotsDir)) {
      mkdirSync(screenshotsDir, { recursive: true });
    }

    // ENV_PINNED + EXTERNAL_SERVER is valid: CI starts a second container with
    // ENV_PINNED_FIXTURE passed as -e flags. The values below must stay in sync
    // with the `Start env-pinned container` step in .github/workflows/test.yml.

    if (EXTERNAL_SERVER) {
      console.log(`Connecting to external server at ${SERVER_URL}`);
      await waitForUrl(`${SERVER_URL}/health`, 30000);
      console.log('External server is ready');
      return;
    }

    // Spawned-server mode: create temp data dir, launch binary, wait for HTTP
    testDataDir = mkdtempSync(join(tmpdir(), 'kniha-jazd-server-test-'));
    process.env.KNIHA_JAZD_DATA_DIR = testDataDir;

    const binaryPath = getBinaryPath();
    console.log(`Starting web server binary: ${binaryPath}`);
    console.log(`Server URL: ${SERVER_URL}`);
    console.log(`Test data dir: ${testDataDir}`);

    if (ENV_PINNED) {
      console.log(`Pinning settings via env: ${Object.keys(ENV_PINNED_FIXTURE).join(', ')}`);
    }

    serverProcess = spawn(binaryPath, [], {
      env: {
        ...process.env,
        KNIHA_JAZD_DATA_DIR: testDataDir,
        DATABASE_PATH: join(testDataDir, 'kniha-jazd.db'),
        STATIC_DIR: join(__dirname, '../../build'),
        PORT: String(SERVER_PORT),
        KNIHA_JAZD_MOCK_GEMINI_DIR: join(__dirname, 'data', 'mocks'),
        ...SCRUBBED_ENV,
        ...(ENV_PINNED ? ENV_PINNED_FIXTURE : {}),
      },
      stdio: 'ignore',
    });

    serverProcess.on('error', (err) => {
      console.error(`Failed to start web server binary: ${err.message}`);
    });

    serverProcess.on('exit', (code) => {
      if (code !== null && code !== 0) {
        console.error(`Web server binary exited with code ${code}`);
      }
    });

    // Wait for HTTP server to be ready
    await waitForUrl(`${SERVER_URL}/health`, 30000);
    console.log('Server is ready');
  },

  /**
   * Before all tests in a worker: Navigate to server URL, wait for app to load.
   */
  before: async function () {
    // Clear any leftover data from previous runs (Docker volume / spawned-server temp dir).
    await resetDatabase(SERVER_URL);

    await browser.url(SERVER_URL);

    // Wait for the SPA to boot — the <h1> only renders once the bundle has run.
    await browser.waitUntil(
      async () => {
        const header = await $('h1');
        return header.isDisplayed();
      },
      { timeout: 15000, timeoutMsg: 'App did not load in server mode' }
    );

    console.log('App ready for testing (server mode)');
  },

  /**
   * Before each test: set locale and refresh the page so any stale UI state from
   * the previous test (open dialogs, edited form rows) is cleared. Do NOT reset the
   * database here: WDIO's `beforeTest` runs AFTER the spec's `beforeEach`, so a
   * database reset here would wipe out vehicles the spec just seeded. Database
   * cleanup runs in `afterTest` instead — the next test then starts with an empty DB.
   */
  beforeTest: async function () {
    // Set locale to English for consistent test output
    for (let i = 0; i < 3; i++) {
      try {
        await browser.execute(() => {
          localStorage.setItem('kniha-jazd-locale', 'en');
        });
        break;
      } catch (e) {
        if (i === 2) {
          console.warn('Could not set locale in localStorage:', e);
        } else {
          await new Promise(r => setTimeout(r, 500));
        }
      }
    }

    await browser.refresh();
    await browser.waitUntil(
      async () => {
        const header = await $('h1');
        return header.isDisplayed();
      },
      { timeout: 10000, timeoutMsg: 'App did not reload between tests' }
    );
  },

  /**
   * After each test: reset the database so the next test's `beforeEach`
   * starts from a clean state.
   */
  afterTest: async function () {
    await resetDatabase(SERVER_URL);
  },

  /**
   * After all tests: Kill the web server process and clean up temp directory.
   * In external server mode, the container/server is managed by the user — skip cleanup.
   */
  onComplete: async function () {
    if (EXTERNAL_SERVER) {
      console.log('External server mode — skipping process cleanup');
      return;
    }

    if (serverProcess) {
      serverProcess.kill();
      serverProcess = null;
    }

    if (testDataDir && existsSync(testDataDir)) {
      try {
        rmSync(testDataDir, { recursive: true, force: true });
        console.log(`Cleaned up test data directory: ${testDataDir}`);
      } catch {
        // Ignore cleanup errors — temp dir will be cleaned by OS eventually
      }
    }
  },
};
