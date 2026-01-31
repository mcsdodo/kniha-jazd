import { writable } from 'svelte/store';
import { getThemePreference, setThemePreference } from '$lib/api';
import { THEME_MODES, type ThemeMode } from '$lib/constants';

function createThemeStore() {
    const { subscribe, set } = writable<ThemeMode>(THEME_MODES.SYSTEM);
    let mediaQueryCleanup: (() => void) | null = null;

    function applyTheme(mode: ThemeMode) {
        const isDark =
            mode === THEME_MODES.DARK ||
            (mode === THEME_MODES.SYSTEM && window.matchMedia('(prefers-color-scheme: dark)').matches);
        document.documentElement.dataset.theme = isDark ? 'dark' : 'light';
    }

    return {
        subscribe,
        init: async () => {
            const saved = await getThemePreference();
            set(saved);
            applyTheme(saved);

            // Listen for system preference changes (with cleanup tracking)
            const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
            const handler = () => {
                // Re-apply if in system mode
                getThemePreference().then((current) => {
                    if (current === THEME_MODES.SYSTEM) {
                        applyTheme(THEME_MODES.SYSTEM);
                    }
                });
            };
            mediaQuery.addEventListener('change', handler);
            mediaQueryCleanup = () => mediaQuery.removeEventListener('change', handler);
        },
        change: async (mode: ThemeMode) => {
            await setThemePreference(mode);
            set(mode);
            applyTheme(mode);
        },
        // Cleanup for proper resource management (call on app destroy if needed)
        destroy: () => {
            if (mediaQueryCleanup) {
                mediaQueryCleanup();
                mediaQueryCleanup = null;
            }
        }
    };
}

export const themeStore = createThemeStore();
