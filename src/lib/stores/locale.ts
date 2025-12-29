import { writable } from 'svelte/store';
import type { Locales } from '$lib/i18n/i18n-types';
import { setLocale } from '$lib/i18n/i18n-svelte';
import { loadLocale } from '$lib/i18n/i18n-util.sync';

const LOCALE_STORAGE_KEY = 'kniha-jazd-locale';
const DEFAULT_LOCALE: Locales = 'sk';

function createLocaleStore() {
	const { subscribe, set } = writable<Locales>(DEFAULT_LOCALE);

	return {
		subscribe,
		/**
		 * Initialize locale from localStorage or browser detection
		 */
		init: () => {
			let locale: Locales;

			// Check localStorage first
			const saved = localStorage.getItem(LOCALE_STORAGE_KEY);
			if (saved === 'sk' || saved === 'en') {
				locale = saved;
			} else {
				// Detect from browser
				const browserLang = navigator.language.toLowerCase();
				locale = browserLang.startsWith('sk') ? 'sk' : 'en';
			}

			// Load and set the locale
			loadLocale(locale);
			setLocale(locale);
			set(locale);

			return locale;
		},
		/**
		 * Change locale and persist to localStorage
		 */
		change: (newLocale: Locales) => {
			loadLocale(newLocale);
			setLocale(newLocale);
			set(newLocale);

			// Persist to localStorage
			localStorage.setItem(LOCALE_STORAGE_KEY, newLocale);
		}
	};
}

export const localeStore = createLocaleStore();
