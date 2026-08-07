import { openPath } from '@tauri-apps/plugin-opener';
import { IS_TAURI } from './api-adapter';

/**
 * Open an external URL.
 *
 * Desktop (Tauri): unchanged behaviour — hands the URL to the OS default browser
 * via the opener plugin.
 * Server/browser mode: the opener plugin is unavailable (no `__TAURI_INTERNALS__`),
 * so open a new tab instead.
 */
export async function openExternal(url: string): Promise<void> {
	if (IS_TAURI) {
		await openPath(url);
		return;
	}
	window.open(url, '_blank', 'noopener,noreferrer');
}
