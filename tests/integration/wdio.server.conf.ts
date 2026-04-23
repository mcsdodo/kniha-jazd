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
 * Get specs based on TIER and PARALLEL_TIERS environment variables
 * (Reuses the same logic as wdio.conf.ts for consistency)
 */
function getSpecs(): string[] {
  const tier = process.env.TIER;
  const parallelMode = process.env.PARALLEL_TIERS === 'true';

  if (parallelMode) {
    switch (tier) {
      case '1':
        return ['./specs/tier1/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
      case '2':
        return ['./specs/tier2/**/*.spec.ts'];
      case '3':
        return ['./specs/tier3/**/*.spec.ts'];
      default:
        return ['./specs/**/*.spec.ts'];
    }
  }

  // Sequential mode (original behavior)
  if (tier === '1') {
    return ['./specs/tier1/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
  } else if (tier === '2') {
    return ['./specs/tier1/**/*.spec.ts', './specs/tier2/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
  }
  return ['./specs/**/*.spec.ts'];
}

/**
 * Get the path to the Tauri application binary based on platform.
 * CI can override via TAURI_BINARY env var.
 */
function getBinaryPath(): string {
  if (process.env.TAURI_BINARY) {
    return process.env.TAURI_BINARY;
  }

  const platform = process.platform;
  const base = join(__dirname, '../../src-tauri/target/debug');

  switch (platform) {
    case 'win32':
      return join(base, 'kniha-jazd.exe');
    case 'darwin':
      return join(base, 'bundle/macos/Kniha Jázd.app/Contents/MacOS/Kniha Jázd');
    case 'linux':
      return join(base, 'kniha-jazd');
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }
}

let tauriProcess: ChildProcess | null = null;
let testDataDir = '';
const SERVER_PORT = 3457; // Different from default 3456 to avoid conflicts
const SERVER_URL = `http://localhost:${SERVER_PORT}`;

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

// WebdriverIO configuration for server mode (Chrome browser against HTTP server)
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
   * Before all tests: Start Tauri binary with server auto-start, wait for HTTP ready.
   */
  onPrepare: async function () {
    // Create sandboxed temp directory for test data
    testDataDir = mkdtempSync(join(tmpdir(), 'kniha-jazd-server-test-'));
    process.env.KNIHA_JAZD_DATA_DIR = testDataDir;
    process.env.WDIO_SERVER_MODE = '1';
    process.env.WDIO_SERVER_URL = SERVER_URL;

    // Mock Gemini API: load JSON from mocks/ instead of calling API
    process.env.KNIHA_JAZD_MOCK_GEMINI_DIR = join(__dirname, 'data', 'mocks');

    // Create screenshots directory if it doesn't exist
    const screenshotsDir = join(__dirname, 'screenshots');
    if (!existsSync(screenshotsDir)) {
      mkdirSync(screenshotsDir, { recursive: true });
    }

    // Start Tauri binary with server auto-start
    const binaryPath = getBinaryPath();
    console.log(`Starting Tauri binary in server mode: ${binaryPath}`);
    console.log(`Server URL: ${SERVER_URL}`);
    console.log(`Test data dir: ${testDataDir}`);

    tauriProcess = spawn(binaryPath, [], {
      env: {
        ...process.env,
        KNIHA_JAZD_DATA_DIR: testDataDir,
        KNIHA_JAZD_SERVER_AUTOSTART: '1',
        KNIHA_JAZD_SERVER_PORT: String(SERVER_PORT),
        KNIHA_JAZD_MOCK_GEMINI_DIR: join(__dirname, 'data', 'mocks'),
      },
      stdio: 'ignore',
    });

    tauriProcess.on('error', (err) => {
      console.error(`Failed to start Tauri binary: ${err.message}`);
    });

    tauriProcess.on('exit', (code) => {
      if (code !== null && code !== 0) {
        console.error(`Tauri binary exited with code ${code}`);
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
    await browser.url(SERVER_URL);

    // Wait for DOM ready (no Tauri IPC needed in server mode)
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
   * Before each test: Reset database via RPC and refresh.
   *
   * Uses RPC to reset database instead of file deletion to avoid
   * Windows file locking issues.
   */
  beforeTest: async function () {
    // Wait for any pending operations to complete
    await new Promise(resolve => setTimeout(resolve, 800));

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

    // Reset database by deleting all vehicles (cascades to trips)
    // Uses existing RPC commands — no special reset command needed
    try {
      const rpc = async (cmd: string, args: Record<string, unknown> = {}) => {
        const resp = await fetch(`${SERVER_URL}/api/rpc`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', 'X-KJ-Client': '1' },
          body: JSON.stringify({ command: cmd, args }),
        });
        if (!resp.ok) throw new Error(`${cmd}: ${resp.status}`);
        return resp.json();
      };
      const vehicles = await rpc('get_vehicles') as Array<{ id: string }>;
      for (const v of vehicles) {
        try {
          await rpc('delete_vehicle', { id: v.id });
        } catch { /* ignore individual delete failures */ }
      }
    } catch (e) {
      console.warn('Database reset RPC failed:', e);
    }

    // Refresh the app to pick up fresh state
    await browser.refresh();

    // Wait for app to be ready again after refresh
    await browser.waitUntil(
      async () => {
        const header = await $('h1');
        return header.isDisplayed();
      },
      {
        timeout: 10000,
        timeoutMsg: 'App did not reload after DB reset'
      }
    );
  },

  /**
   * After all tests: Kill Tauri process and clean up temp directory.
   */
  onComplete: async function () {
    if (tauriProcess) {
      tauriProcess.kill();
      tauriProcess = null;
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
