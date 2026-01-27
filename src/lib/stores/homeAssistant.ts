/**
 * Home Assistant ODO store with caching and periodic refresh.
 * Uses Rust backend for API calls to avoid CORS issues.
 */

import { writable, get } from 'svelte/store';
import type { HaOdoCache } from '$lib/types';
import { fetchHaOdo } from '$lib/api';

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
		 * Fetch ODO from Home Assistant via Rust backend and update cache.
		 */
		async fetchOdo(vehicleId: string, sensorId: string): Promise<number | null> {
			update((s) => ({ ...s, loading: true, error: null }));

			try {
				// Use Rust backend to avoid CORS issues
				const value = await fetchHaOdo(sensorId);

				if (value === null) {
					update((s) => ({ ...s, loading: false, error: 'Sensor unavailable' }));
					return null;
				}

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
				const errorMsg = error instanceof Error ? error.message : 'Unknown error';
				update((s) => ({ ...s, loading: false, error: errorMsg }));
				return null;
			}
		},

		/**
		 * Start periodic refresh for a vehicle.
		 */
		startPeriodicRefresh(vehicleId: string, sensorId: string) {
			// Stop any existing refresh
			this.stopPeriodicRefresh();

			currentVehicleId = vehicleId;
			currentSensorId = sensorId;

			// Fetch immediately
			this.fetchOdo(vehicleId, sensorId);

			// Then refresh every 5 minutes
			refreshInterval = setInterval(() => {
				if (currentVehicleId && currentSensorId) {
					this.fetchOdo(currentVehicleId, currentSensorId);
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
