/**
 * Resolve a path under tests/integration/data/ into whatever the *backend* can see.
 *
 * The specs hand absolute paths to the backend over RPC (receipts folder scanning,
 * Gemini mock JSON). In spawned-server mode the backend is a local process and sees
 * host paths. In Docker mode it is a container that only sees what we mounted, so the
 * same logical location has a different absolute path on the other side of the RPC.
 *
 * Keep the mount targets in sync with the `-v` flags in the `Start container` step of
 * .github/workflows/test.yml and the local `docker run` in 03-plan.md. NOT with
 * docker-compose.web.yml — that is the production-shaped deployment file and has no
 * business mounting test fixtures.
 *
 * There are TWO mappings, and conflating them is the main way to waste an hour here:
 *
 *   fixtures  read-only   repo tests/integration/data  ->  /testdata
 *   workdir   read-write  host $PWD/data               ->  /data
 *
 * Fixtures are committed inputs (invoice PDFs, Gemini mock JSON). The workdir is where
 * the running instance keeps its database and where a spec that *creates* files for the
 * backend to find must write.
 */
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const IS_DOCKER_MODE = process.env.WDIO_EXTERNAL_SERVER === '1';

/** Where tests/integration/data/ is mounted inside the container (read-only). */
export const CONTAINER_FIXTURE_ROOT = '/testdata';

/** Where the writable data dir is mounted inside the container. */
export const CONTAINER_WORK_ROOT = '/data';

/** Host path to tests/integration/data/ — for reads the *test process* performs. */
export const HOST_FIXTURE_ROOT = join(__dirname, '..', 'data');

/**
 * Host path to the writable data dir the backend also sees.
 * Docker: the bind-mount source. Spawned mode: the temp dir wdio exported.
 */
export function hostWorkDir(): string {
  if (IS_DOCKER_MODE) return join(process.cwd(), 'data');
  const dir = process.env.KNIHA_JAZD_DATA_DIR;
  if (!dir) {
    throw new Error('KNIHA_JAZD_DATA_DIR not set — spawned-server mode should export it');
  }
  return dir;
}

/**
 * A committed fixture path as the BACKEND sees it.
 * Use for any fixture path sent over RPC; use HOST_FIXTURE_ROOT for fs calls in the spec.
 */
export function backendFixturePath(...segments: string[]): string {
  return IS_DOCKER_MODE
    ? [CONTAINER_FIXTURE_ROOT, ...segments].join('/')
    : join(HOST_FIXTURE_ROOT, ...segments);
}

/**
 * A path under the writable work dir as the BACKEND sees it.
 * Pair every call with hostWorkDir() for the write the test process performs.
 */
export function backendWorkPath(...segments: string[]): string {
  return IS_DOCKER_MODE
    ? [CONTAINER_WORK_ROOT, ...segments].join('/')
    : join(hostWorkDir(), ...segments);
}
