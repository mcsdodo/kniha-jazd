import { writable } from 'svelte/store';
import { getAppMode, type AppModeInfo } from '$lib/api';

function createAppModeStore() {
    const { subscribe, set } = writable<AppModeInfo>({
        is_read_only: false,
        reason: null,
    });

    return {
        subscribe,
        refresh: async () => {
            try {
                const mode = await getAppMode();
                set(mode);
            } catch (error) {
                console.error('Failed to get app mode:', error);
            }
        },
    };
}

export const appModeStore = createAppModeStore();
