/* eslint-disable @typescript-eslint/no-explicit-any */
import { spawn, ChildProcess } from 'child_process';
import { mkdtempSync, rmSync, existsSync, unlinkSync, mkdirSync } from 'fs';
import { tmpdir } from 'os';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// ES module equivalent of __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Get specs based on TIER environment variable
 * - TIER=1: Run tier1 + existing (fast, critical tests for PRs)
 * - TIER=2: Run tier1 + tier2 + existing
 * - TIER=3 or unset: Run all tiers
 */
function getSpecs(): string[] {
  const tier = process.env.TIER;

  if (tier === '1') {
    return ['./specs/tier1/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
  } else if (tier === '2') {
    return ['./specs/tier1/**/*.spec.ts', './specs/tier2/**/*.spec.ts', './specs/existing/**/*.spec.ts'];
  }
  // Default: run all specs
  return ['./specs/**/*.spec.ts'];
}

/**
 * Get the path to the test database
 */
export function getTestDbPath(): string {
  return join(testDataDir, 'kniha-jazd.db');
}

let tauriDriver: ChildProcess | null = null;
let testDataDir: string = '';

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

/**
 * Wait for a port to be available (tauri-driver ready)
 */
async function waitForPort(port: number, timeout = 10000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/status`);
      if (response.ok) return;
    } catch {
      // Port not ready yet
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`Timeout waiting for port ${port}`);
}

// WebdriverIO configuration
// Note: Using 'any' type as WebdriverIO types don't fully cover all valid options
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

  maxInstances: 1, // Tauri apps run one at a time

  // Connect to tauri-driver running on port 4444
  hostname: '127.0.0.1',
  port: 4444,
  path: '/',

  capabilities: [{
    // tauri-driver uses these capabilities
    'tauri:options': {
      application: getBinaryPath(),
    }
  }],

  logLevel: 'info',
  bail: 0,
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,

  framework: 'mocha',
  reporters: ['spec'],

  mochaOpts: {
    ui: 'bdd',
    timeout: 30000, // 30s per test - most tests should complete within 10s
  },

  /**
   * Before all tests: Set up sandboxed environment
   */
  onPrepare: async function () {
    // Create sandboxed temp directory for test data
    testDataDir = mkdtempSync(join(tmpdir(), 'kniha-jazd-test-'));
    process.env.KNIHA_JAZD_DATA_DIR = testDataDir;
    console.log(`Test data directory: ${testDataDir}`);

    // Mock Gemini API to avoid real API calls during tests
    process.env.KNIHA_JAZD_MOCK_GEMINI = 'true';

    // Create screenshots directory if it doesn't exist
    const screenshotsDir = join(__dirname, 'screenshots');
    if (!existsSync(screenshotsDir)) {
      mkdirSync(screenshotsDir, { recursive: true });
    }

    // Find msedgedriver (required on Windows for WebView2)
    const edgeDriverPaths = [
      process.env.MSEDGEDRIVER_PATH,
      'msedgedriver.exe',
      'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedgedriver.exe',
      join(process.env.USERPROFILE || '', 'AppData', 'Local', 'Programs', 'msedgedriver', 'msedgedriver.exe'),
      // WinGet installation path
      join(process.env.LOCALAPPDATA || '', 'Microsoft', 'WinGet', 'Packages', 'Microsoft.EdgeDriver_Microsoft.Winget.Source_8wekyb3d8bbwe', 'msedgedriver.exe'),
    ].filter(Boolean) as string[];

    let nativeDriverArg: string[] = [];
    if (process.platform === 'win32') {
      const edgeDriverPath = edgeDriverPaths.find(p => existsSync(p));
      if (edgeDriverPath) {
        nativeDriverArg = ['--native-driver', edgeDriverPath];
        console.log(`Using Edge WebDriver: ${edgeDriverPath}`);
      } else {
        console.warn('WARNING: msedgedriver.exe not found. Download from:');
        console.warn('https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/');
        console.warn('Set MSEDGEDRIVER_PATH env var or add to PATH');
      }
    }

    // Start tauri-driver
    const tauriDriverPath = process.platform === 'win32' ? 'tauri-driver.exe' : 'tauri-driver';
    tauriDriver = spawn(tauriDriverPath, nativeDriverArg, {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, KNIHA_JAZD_DATA_DIR: testDataDir }
    });

    tauriDriver.stdout?.on('data', (data) => {
      console.log(`tauri-driver: ${data}`);
    });

    tauriDriver.stderr?.on('data', (data) => {
      console.error(`tauri-driver error: ${data}`);
    });

    // Wait for tauri-driver to be ready
    await waitForPort(4444);
    console.log('tauri-driver is ready');
  },

  /**
   * After all tests: Clean up
   */
  onComplete: async function () {
    // Stop tauri-driver
    if (tauriDriver) {
      tauriDriver.kill();
      tauriDriver = null;
    }

    // Clean up temp directory
    if (testDataDir && existsSync(testDataDir)) {
      rmSync(testDataDir, { recursive: true, force: true });
      console.log(`Cleaned up test data directory: ${testDataDir}`);
    }
  },

  /**
   * Before all tests in a worker: Wait for app to be ready
   * This runs once per spec file (each spec gets a fresh session)
   */
  before: async function () {
    // Wait for DOM to be ready
    await browser.waitUntil(
      async () => {
        const header = await $('h1');
        return header.isDisplayed();
      },
      {
        timeout: 30000,
        timeoutMsg: 'App did not load within 30 seconds'
      }
    );

    // Wait for Tauri v2 IPC bridge to be available
    // In Tauri v2 with withGlobalTauri: true, API is at window.__TAURI__.core.invoke
    await browser.waitUntil(
      async () => {
        return browser.execute(() => {
          return typeof (window as any).__TAURI__ !== 'undefined' &&
                 typeof (window as any).__TAURI__.core !== 'undefined' &&
                 typeof (window as any).__TAURI__.core.invoke === 'function';
        });
      },
      {
        timeout: 10000,
        timeoutMsg: 'Tauri IPC bridge did not initialize within 10 seconds'
      }
    );

    console.log('App ready for testing');
  },

  /**
   * Before each test: Fresh database
   */
  beforeTest: async function () {
    // Wait for any pending operations to complete
    await new Promise(resolve => setTimeout(resolve, 500));

    // Delete existing test DB for clean slate with retry logic
    const dbPath = getTestDbPath();
    const maxRetries = 3;

    for (let i = 0; i < maxRetries; i++) {
      try {
        if (existsSync(dbPath)) {
          unlinkSync(dbPath);
          console.log('Cleaned up test database');
        }
        // Also clean up the WAL and SHM files if they exist
        const walPath = dbPath + '-wal';
        const shmPath = dbPath + '-shm';
        if (existsSync(walPath)) {
          unlinkSync(walPath);
        }
        if (existsSync(shmPath)) {
          unlinkSync(shmPath);
        }
        break;
      } catch (e) {
        if (i === maxRetries - 1) {
          console.error(`Failed to clean up test database after ${maxRetries} retries:`, e);
          throw e;
        }
        console.log(`Retry ${i + 1}/${maxRetries} cleaning up test database...`);
        await new Promise(r => setTimeout(r, 200));
      }
    }

    // After DB cleanup, refresh the app to pick up fresh state
    await browser.refresh();

    // Wait for app to be ready again after refresh
    await browser.waitUntil(
      async () => {
        const header = await $('h1');
        return header.isDisplayed();
      },
      {
        timeout: 10000,
        timeoutMsg: 'App did not reload after DB cleanup'
      }
    );

    // Wait for Tauri v2 IPC to be available
    await browser.waitUntil(
      async () => {
        return browser.execute(() => {
          return typeof (window as any).__TAURI__ !== 'undefined' &&
                 typeof (window as any).__TAURI__.core !== 'undefined' &&
                 typeof (window as any).__TAURI__.core.invoke === 'function';
        });
      },
      {
        timeout: 5000,
        timeoutMsg: 'Tauri IPC not ready after DB cleanup'
      }
    );
  },
};

