// Trips store

import { writable } from 'svelte/store';
import type { Trip } from '../types';
import { getTrips, getTripsForYear } from '../api';

export const tripsStore = writable<Trip[]>([]);

// Helper function to load trips
export async function loadTrips(vehicleId: string, year?: number): Promise<void> {
	try {
		const trips = year
			? await getTripsForYear(vehicleId, year)
			: await getTrips(vehicleId);
		tripsStore.set(trips);
	} catch (error) {
		console.error('Failed to load trips:', error);
		tripsStore.set([]);
	}
}
