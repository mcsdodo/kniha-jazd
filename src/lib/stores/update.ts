import { writable } from 'svelte/store';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

interface UpdateState {
	checking: boolean;
	available: boolean;
	version: string | null;
	releaseNotes: string | null;
	dismissed: boolean;
	downloading: boolean;
	progress: number;
	error: string | null;
}

const initialState: UpdateState = {
	checking: false,
	available: false,
	version: null,
	releaseNotes: null,
	dismissed: false,
	downloading: false,
	progress: 0,
	error: null
};

function createUpdateStore() {
	const { subscribe, set, update: updateState } = writable<UpdateState>(initialState);
	let updateObject: Awaited<ReturnType<typeof check>> | null = null;

	return {
		subscribe,

		check: async () => {
			updateState((state) => ({ ...state, checking: true, error: null }));
			try {
				const result = await check({ timeout: 5000 });
				if (result?.available) {
					updateObject = result;
					updateState((state) => ({
						...state,
						available: true,
						version: result.version,
						releaseNotes: result.body || null,
						checking: false
					}));
				} else {
					updateState((state) => ({
						...state,
						available: false,
						checking: false
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
			updateState((state) => ({ ...state, dismissed: true }));
		},

		install: async () => {
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
		},

		reset: () => {
			set(initialState);
			updateObject = null;
		}
	};
}

export const updateStore = createUpdateStore();
