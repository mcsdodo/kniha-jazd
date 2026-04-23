import { writable } from 'svelte/store';
import { IS_TAURI } from '$lib/api-adapter';

interface Capabilities {
    mode: 'desktop' | 'server';
    readOnly: boolean;
    features: {
        fileDialogs: boolean;
        updater: boolean;
        openExternal: boolean;
        restoreBackup: boolean;
        moveDatabase: boolean;
    };
}

const defaultDesktop: Capabilities = {
    mode: 'desktop',
    readOnly: false,
    features: {
        fileDialogs: true,
        updater: true,
        openExternal: true,
        restoreBackup: true,
        moveDatabase: true,
    },
};

export const capabilities = writable<Capabilities>(defaultDesktop);

export async function loadCapabilities(): Promise<void> {
    if (IS_TAURI) {
        const { apiCall } = await import('$lib/api-adapter');
        try {
            const mode = await apiCall<{ mode: string; isReadOnly: boolean }>('get_app_mode');
            capabilities.set({
                ...defaultDesktop,
                readOnly: mode.isReadOnly,
            });
        } catch {
            capabilities.set(defaultDesktop);
        }
        return;
    }

    try {
        const resp = await fetch('/api/capabilities');
        const data = await resp.json();
        capabilities.set({
            mode: 'server',
            readOnly: data.read_only,
            features: {
                fileDialogs: data.features.file_dialogs,
                updater: data.features.updater,
                openExternal: data.features.open_external,
                restoreBackup: data.features.restore_backup,
                moveDatabase: data.features.move_database,
            },
        });
    } catch {
        capabilities.set(defaultDesktop);
    }
}
