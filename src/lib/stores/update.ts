import { writable } from 'svelte/store';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

const DISMISSED_VERSION_KEY = 'kniha-jazd-dismissed-update-version';

type BackupStep = 'pending' | 'in-progress' | 'done' | 'failed' | 'skipped';

interface UpdateState {
	checking: boolean;
	available: boolean;
	version: string | null;
	releaseNotes: string | null;
	dismissed: boolean;
	downloading: boolean;
	progress: number;
	error: string | null;
	// Backup step during update
	backupStep: BackupStep;
	backupError: string | null;
}

function getDismissedVersion(): string | null {
	try {
		return localStorage.getItem(DISMISSED_VERSION_KEY);
	} catch {
		return null;
	}
}

function setDismissedVersion(version: string | null): void {
	try {
		if (version) {
			localStorage.setItem(DISMISSED_VERSION_KEY, version);
		} else {
			localStorage.removeItem(DISMISSED_VERSION_KEY);
		}
	} catch {
		// Ignore localStorage errors
	}
}

const initialState: UpdateState = {
	checking: false,
	available: false,
	version: null,
	releaseNotes: null,
	dismissed: false,
	downloading: false,
	progress: 0,
	error: null,
	backupStep: 'pending',
	backupError: null
};

function createUpdateStore() {
	const { subscribe, set, update: updateState } = writable<UpdateState>(initialState);
	let updateObject: Awaited<ReturnType<typeof check>> | null = null;

	async function doCheck(respectDismissed: boolean) {
		updateState((state) => ({ ...state, checking: true, error: null }));
		try {
			const result = await check({ timeout: 5000 });
			if (result?.available) {
				updateObject = result;
				// Check if this version was previously dismissed (only for automatic checks)
				const dismissedVersion = getDismissedVersion();
				const isDismissed = respectDismissed && dismissedVersion === result.version;
				updateState((state) => ({
					...state,
					available: true,
					version: result.version,
					releaseNotes: result.body || null,
					checking: false,
					dismissed: isDismissed
				}));
			} else {
				// Clear any dismissed version if no update available
				setDismissedVersion(null);
				updateState((state) => ({
					...state,
					available: false,
					checking: false,
					dismissed: false
				}));
			}
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			updateState((state) => ({
				...state,
				checking: false,
				error: errorMsg
			}));
		}
	}

	return {
		subscribe,

		// Automatic check (respects dismissed state)
		check: () => doCheck(true),

		// Manual check (ignores dismissed state, always shows modal if update available)
		checkManual: () => doCheck(false),

		// Silent check (checks but auto-dismisses - for when auto-check is disabled)
		// This still sets available=true so the dot indicator shows
		checkSilent: async () => {
			updateState((state) => ({ ...state, checking: true, error: null }));
			try {
				const result = await check({ timeout: 5000 });
				if (result?.available) {
					updateObject = result;
					// Auto-dismiss but still mark as available (for dot indicator)
					updateState((state) => ({
						...state,
						available: true,
						version: result.version,
						releaseNotes: result.body || null,
						checking: false,
						dismissed: true  // Always dismissed in silent mode
					}));
				} else {
					updateState((state) => ({
						...state,
						available: false,
						checking: false,
						dismissed: false
					}));
				}
			} catch (err) {
				const errorMsg = err instanceof Error ? err.message : String(err);
				updateState((state) => ({
					...state,
					checking: false,
					error: errorMsg
				}));
			}
		},

		dismiss: () => {
			updateState((state) => {
				// Persist the dismissed version
				if (state.version) {
					setDismissedVersion(state.version);
				}
				return { ...state, dismissed: true };
			});
		},

		// Show the update modal (un-dismiss)
		showModal: () => {
			updateState((state) => ({ ...state, dismissed: false }));
		},

		install: async () => {
			if (!updateObject) {
				throw new Error('No update available to install');
			}

			// Step 1: Create backup
			updateState((state) => ({
				...state,
				backupStep: 'in-progress',
				backupError: null,
				error: null
			}));

			try {
				const { createBackupWithType } = await import('$lib/api');
				await createBackupWithType('pre-update', updateObject.version);
				updateState((state) => ({ ...state, backupStep: 'done' }));
			} catch (err) {
				const errorMsg = err instanceof Error ? err.message : String(err);
				updateState((state) => ({
					...state,
					backupStep: 'failed',
					backupError: errorMsg
				}));
				// Don't proceed - let UI handle the failed state
				return;
			}

			// Step 2: Download and install
			await performDownloadAndInstall();
		},

		// Continue after backup failure (user chose to proceed)
		continueWithoutBackup: async () => {
			updateState((state) => ({ ...state, backupStep: 'skipped' }));
			await performDownloadAndInstall();
		},

		reset: () => {
			set(initialState);
			updateObject = null;
		}
	};

	// Extracted download logic (used by install and continueWithoutBackup)
	async function performDownloadAndInstall() {
		if (!updateObject) {
			throw new Error('No update available to install');
		}

		updateState((state) => ({ ...state, downloading: true, error: null }));
		try {
			let contentLength = 0;
			let downloaded = 0;
			await updateObject.downloadAndInstall((event) => {
				if (event.event === 'Started') {
					contentLength = event.data.contentLength || 0;
					updateState((state) => ({ ...state, downloading: true }));
				} else if (event.event === 'Progress') {
					downloaded += event.data.chunkLength;
					const progress = contentLength > 0 ? Math.round((downloaded / contentLength) * 100) : 0;
					updateState((state) => ({ ...state, progress }));
				} else if (event.event === 'Finished') {
					updateState((state) => ({ ...state, downloading: false, progress: 100 }));
				}
			});

			// Relaunch the application after installation
			await relaunch();
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			updateState((state) => ({
				...state,
				downloading: false,
				error: errorMsg
			}));
		}
	}
}

export const updateStore = createUpdateStore();
