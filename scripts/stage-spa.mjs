// Copies the SvelteKit build output into src-tauri/desktop/spa/ so Tauri's
// bundle.resources glob can pick it up. Run as `beforeBundleCommand`; see
// src-tauri/desktop/src/static_dir.rs for the consuming side.

import { cp, rm, mkdir, stat, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..');
const src = resolve(repoRoot, 'build');
const dest = resolve(repoRoot, 'src-tauri', 'desktop', 'spa');

try {
	await stat(src);
} catch {
	console.error(
		`stage-spa: source ${src} does not exist — did 'vite build' run?`
	);
	process.exit(1);
}

await rm(dest, { recursive: true, force: true });
await mkdir(dest, { recursive: true });
await cp(src, dest, { recursive: true });
// Re-create .gitkeep so a clean clone (where spa/ doesn't exist) can still
// satisfy tauri-build's compile-time glob check via the tracked placeholder.
await writeFile(resolve(dest, '.gitkeep'), '');
console.log(`stage-spa: ${src} -> ${dest}`);
