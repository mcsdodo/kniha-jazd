/**
 * Skip helpers for dual-mode integration tests.
 *
 * Some tests exercise Tauri-only features (e.g., native file dialogs,
 * window management) that have no equivalent in server mode, or filesystem
 * paths that aren't mounted into the Docker container.
 *
 * Use the `describeIn{X}Mode` helpers as a `describe`-replacement:
 *
 * @example
 * import { describeNotInDockerMode } from '../../utils/skip';
 *
 * describeNotInDockerMode('Mismatch Detection E2E', () => {
 *   it('matches receipt to trip', async () => { ... });
 * });
 */

const IS_SERVER_MODE = process.env.WDIO_SERVER_MODE === '1';
const IS_DOCKER_MODE = process.env.WDIO_EXTERNAL_SERVER === '1';

/** describe-block alias that skips when running in server mode (Tauri-only features). */
export const describeNotInServerMode: Mocha.SuiteFunction =
  (IS_SERVER_MODE ? describe.skip : describe) as Mocha.SuiteFunction;

/** describe-block alias that skips when running in Tauri mode (server-only features). */
export const describeNotInTauriMode: Mocha.SuiteFunction =
  (!IS_SERVER_MODE ? describe.skip : describe) as Mocha.SuiteFunction;

/**
 * describe-block alias that skips when running against an external server (Docker).
 *
 * Use for tests that need backend access to host filesystem paths
 * (receipts folder scanning, Gemini mock JSON files). In Docker mode the
 * container can't see the host's `tests/integration/data/...` directories
 * unless they're explicitly mounted, which we don't do for the production-shaped
 * compose file. These tests still run in spawned-Tauri server mode.
 */
export const describeNotInDockerMode: Mocha.SuiteFunction =
  (IS_DOCKER_MODE ? describe.skip : describe) as Mocha.SuiteFunction;
