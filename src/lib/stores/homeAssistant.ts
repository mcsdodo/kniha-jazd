/**
 * Home Assistant ODO store with caching and periodic refresh.
 */

import { writable, get } from 'svelte/store';
import type { HaOdoCache } from '$lib/types';
import { fetchOdometer, HaAuthError, HaSensorNotFoundError, HaTimeoutError } from '$lib/services/homeAssistant';

const CACHE_KEY = 'kniha-jazd-ha-odo-cache';
const REFRESH_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

interface HaStoreState {
	cache: Map<string, HaOdoCache>; // vehicleId -> cache
	loading: boolean;
	error: string | null;
}

function createHaStore() {
	const { subscribe, set, update } = writable<HaStoreState>({
		cache: new Map(),
		loading: false,
		error: null
	});

	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let currentVehicleId: string | null = null;
	let currentSensorId: string | null = null;
	let currentUrl: string | null = null;
	let currentToken: string | null = null;

	// Load cache from localStorage
	function loadCache(): Map<string, HaOdoCache> {
		try {
			const stored = localStorage.getItem(CACHE_KEY);
			if (stored) {
				const parsed = JSON.parse(stored);
				// Convert object to Map
				return new Map(Object.entries(parsed));
			}
		} catch (e) {
			console.warn('Failed to load HA ODO cache:', e);
			localStorage.removeItem(CACHE_KEY);
		}
		return new Map();
	}

	// Save cache to localStorage
	function saveCache(cache: Map<string, HaOdoCache>) {
		try {
			// Convert Map to object for JSON serialization
			const obj = Object.fromEntries(cache);
			localStorage.setItem(CACHE_KEY, JSON.stringify(obj));
		} catch (e) {
			console.warn('Failed to save HA ODO cache:', e);
		}
	}

	// Initialize cache from localStorage
	const initialCache = loadCache();
	set({
		cache: initialCache,
		loading: false,
		error: null
	});

	return {
		subscribe,

		/**
		 * Get cached ODO for a vehicle.
		 */
		getCachedOdo(vehicleId: string): HaOdoCache | null {
			const state = get({ subscribe });
			return state.cache.get(vehicleId) || null;
		},

		/**
		 * Fetch ODO from Home Assistant and update cache.
		 */
		async fetchOdo(
			vehicleId: string,
			url: string,
			token: string,
			sensorId: string
		): Promise<number | null> {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				const value = await fetchOdometer(url, token, sensorId);
				const cacheEntry: HaOdoCache = {
					value,
					fetchedAt: Date.now()
				};

				update((s) => {
					const newCache = new Map(s.cache);
					newCache.set(vehicleId, cacheEntry);
					saveCache(newCache);
					return { ...s, cache: newCache, loading: false, error: null };
				});

				return value;
			} catch (error) {
				let errorMsg = 'Unknown error';

				if (error instanceof HaTimeoutError) {
					errorMsg = 'Connection timed out';
				} else if (error instanceof HaAuthError) {
					errorMsg = 'Invalid API token';
				} else if (error instanceof HaSensorNotFoundError) {
					errorMsg = 'Sensor not found';
				} else if (error instanceof Error) {
					errorMsg = error.message;
				}

				update((s) => ({ ...s, loading: false, error: errorMsg }));
				return null;
			}
		},

		/**
		 * Start periodic refresh for a vehicle.
		 */
		startPeriodicRefresh(
			vehicleId: string,
			url: string,
			token: string,
			sensorId: string
		) {
			// Stop any existing refresh
			this.stopPeriodicRefresh();

			currentVehicleId = vehicleId;
			currentUrl = url;
			currentToken = token;
			currentSensorId = sensorId;

			// Fetch immediately
			this.fetchOdo(vehicleId, url, token, sensorId);

			// Then refresh every 5 minutes
			refreshInterval = setInterval(() => {
				if (currentVehicleId && currentUrl && currentToken && currentSensorId) {
					this.fetchOdo(currentVehicleId, currentUrl, currentToken, currentSensorId);
				}
			}, REFRESH_INTERVAL_MS);
		},

		/**
		 * Stop periodic refresh.
		 */
		stopPeriodicRefresh() {
			if (refreshInterval) {
				clearInterval(refreshInterval);
				refreshInterval = null;
			}
			currentVehicleId = null;
			currentUrl = null;
			currentToken = null;
			currentSensorId = null;
		},

		/**
		 * Clear error state.
		 */
		clearError() {
			update((s) => ({ ...s, error: null }));
		},

		/**
		 * Clear all cached data.
		 */
		clearCache() {
			localStorage.removeItem(CACHE_KEY);
			update((s) => ({ ...s, cache: new Map() }));
		}
	};
}

export const haStore = createHaStore();
