/**
 * Home Assistant store with caching and periodic refresh.
 * Supports ODO sensor and fuel level sensor with independent error tracking.
 * Uses Rust backend for API calls to avoid CORS issues.
 */

import { writable, get } from 'svelte/store';
import type { HaOdoCache } from '$lib/types';
import { fetchHaOdo } from '$lib/api';

const CACHE_KEY = 'kniha-jazd-ha-odo-cache';
const REFRESH_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

interface HaStoreState {
	cache: Map<string, HaOdoCache>; // vehicleId -> cache (ODO + optional fuel level)
	loading: boolean;
	odoError: string | null;
	fuelError: string | null;
}

function createHaStore() {
	const { subscribe, set, update } = writable<HaStoreState>({
		cache: new Map(),
		loading: false,
		odoError: null,
		fuelError: null
	});

	let refreshInterval: ReturnType<typeof setInterval> | null = null;
	let currentVehicleId: string | null = null;
	let currentOdoSensorId: string | null = null;
	let currentFuelSensorId: string | null = null;

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
			console.warn('Failed to load HA cache:', e);
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
			console.warn('Failed to save HA cache:', e);
		}
	}

	// Initialize cache from localStorage
	const initialCache = loadCache();
	set({
		cache: initialCache,
		loading: false,
		odoError: null,
		fuelError: null
	});

	return {
		subscribe,

		/**
		 * Get cached entry for a vehicle.
		 */
		getCachedOdo(vehicleId: string): HaOdoCache | null {
			const state = get({ subscribe });
			return state.cache.get(vehicleId) || null;
		},

		/**
		 * Fetch ODO from Home Assistant via Rust backend and update cache.
		 */
		async fetchOdo(vehicleId: string, sensorId: string): Promise<number | null> {
			update((s) => ({ ...s, loading: true, odoError: null }));

			try {
				// Use Rust backend to avoid CORS issues
				const value = await fetchHaOdo(sensorId);

				if (value === null) {
					update((s) => ({ ...s, loading: false, odoError: 'Sensor unavailable' }));
					return null;
				}

				update((s) => {
					const newCache = new Map(s.cache);
					const existing = newCache.get(vehicleId);
					const cacheEntry: HaOdoCache = {
						value,
						fetchedAt: Date.now(),
						// Preserve existing fuel level data
						fuelLevelPercent: existing?.fuelLevelPercent,
						fuelFetchedAt: existing?.fuelFetchedAt
					};
					newCache.set(vehicleId, cacheEntry);
					saveCache(newCache);
					return { ...s, cache: newCache, loading: false, odoError: null };
				});

				return value;
			} catch (error) {
				const errorMsg = error instanceof Error ? error.message : 'Unknown error';
				update((s) => ({ ...s, loading: false, odoError: errorMsg }));
				return null;
			}
		},

		/**
		 * Fetch fuel level from Home Assistant via Rust backend and update cache.
		 * Reuses the same fetch_ha_odo backend command (generic sensor fetcher).
		 */
		async fetchFuelLevel(vehicleId: string, sensorId: string): Promise<number | null> {
			update((s) => ({ ...s, fuelError: null }));

			try {
				const value = await fetchHaOdo(sensorId);

				if (value === null) {
					update((s) => ({ ...s, fuelError: 'Sensor unavailable' }));
					return null;
				}

				update((s) => {
					const newCache = new Map(s.cache);
					const existing = newCache.get(vehicleId);
					if (existing) {
						// Update fuel level on existing cache entry
						existing.fuelLevelPercent = value;
						existing.fuelFetchedAt = Date.now();
						newCache.set(vehicleId, { ...existing });
					} else {
						// Create entry with just fuel level (no ODO yet)
						newCache.set(vehicleId, {
							value: 0,
							fetchedAt: 0,
							fuelLevelPercent: value,
							fuelFetchedAt: Date.now()
						});
					}
					saveCache(newCache);
					return { ...s, cache: newCache, fuelError: null };
				});

				return value;
			} catch (error) {
				const errorMsg = error instanceof Error ? error.message : 'Unknown error';
				update((s) => ({ ...s, fuelError: errorMsg }));
				return null;
			}
		},

		/**
		 * Start periodic refresh for a vehicle's HA sensors.
		 */
		startPeriodicRefresh(vehicleId: string, odoSensorId: string, fuelSensorId?: string) {
			// Stop any existing refresh
			this.stopPeriodicRefresh();

			currentVehicleId = vehicleId;
			currentOdoSensorId = odoSensorId;
			currentFuelSensorId = fuelSensorId || null;

			// Fetch immediately (parallel)
			this.fetchOdo(vehicleId, odoSensorId);
			if (fuelSensorId) {
				this.fetchFuelLevel(vehicleId, fuelSensorId);
			}

			// Then refresh every 5 minutes
			refreshInterval = setInterval(() => {
				if (currentVehicleId && currentOdoSensorId) {
					this.fetchOdo(currentVehicleId, currentOdoSensorId);
				}
				if (currentVehicleId && currentFuelSensorId) {
					this.fetchFuelLevel(currentVehicleId, currentFuelSensorId);
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
			currentOdoSensorId = null;
			currentFuelSensorId = null;
		},

		/**
		 * Clear error state.
		 */
		clearError() {
			update((s) => ({ ...s, odoError: null, fuelError: null }));
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
