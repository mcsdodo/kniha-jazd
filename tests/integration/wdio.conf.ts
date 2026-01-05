import type { Options } from '@wdio/types';
import { spawn, ChildProcess } from 'child_process';
import { mkdtempSync, rmSync, existsSync, unlinkSync } from 'fs';
import { tmpdir } from 'os';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// ES module equivalent of __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

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

export const config: Options.Testrunner = {
  runner: 'local',
  autoCompileOpts: {
    autoCompile: true,
    tsNodeOpts: {
      project: './tsconfig.json',
      transpileOnly: true,
    }
  },

  specs: ['./specs/**/*.spec.ts'],
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
    timeout: 60000,
  },

  /**
   * Before all tests: Set up sandboxed environment
   */
  onPrepare: async function () {
    // Create sandboxed temp directory for test data
    testDataDir = mkdtempSync(join(tmpdir(), 'kniha-jazd-test-'));
    process.env.KNIHA_JAZD_DATA_DIR = testDataDir;
    console.log(`Test data directory: ${testDataDir}`);

    // Find msedgedriver (required on Windows for WebView2)
    const edgeDriverPaths = [
      process.env.MSEDGEDRIVER_PATH,
      'msedgedriver.exe',
      'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedgedriver.exe',
      join(process.env.USERPROFILE || '', 'AppData', 'Local', 'Programs', 'msedgedriver', 'msedgedriver.exe'),
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
   * Before each test: Fresh database
   */
  beforeTest: async function () {
    // Delete existing test DB for clean slate
    const dbPath = join(testDataDir, 'kniha-jazd.db');
    if (existsSync(dbPath)) {
      unlinkSync(dbPath);
      console.log('Cleaned up test database');
    }
  },
};

