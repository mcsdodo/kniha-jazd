/**
 * Pure calculation functions for vehicle logbook.
 * Extracted from TripGrid.svelte for testability.
 */

import type { Trip } from './types';

/**
 * Sort trips chronologically by date, then by odometer for same-day trips.
 */
function sortChronologically(trips: Trip[]): Trip[] {
	return [...trips].sort((a, b) => {
		const dateDiff = new Date(a.date).getTime() - new Date(b.date).getTime();
		if (dateDiff !== 0) return dateDiff;
		return a.odometer - b.odometer;
	});
}

/**
 * Calculate consumption rates (l/100km) for each trip.
 *
 * Logic:
 * - Accumulates km and fuel across trips until a full tank fillup
 * - Only calculates actual rate when span ends with full_tank: true
 * - Partial fillups accumulate fuel but don't close the period
 * - Trips after last full tank use TP rate (marked as estimated)
 *
 * @param trips - Array of trips
 * @param tpConsumption - Vehicle's TP consumption rate (l/100km)
 * @returns rates map (tripId -> rate) and estimated set (tripIds using TP rate)
 */
export function calculateConsumptionRates(
	trips: Trip[],
	tpConsumption: number
): { rates: Map<string, number>; estimated: Set<string> } {
	const rates = new Map<string, number>();
	const estimated = new Set<string>();
	const chronological = sortChronologically(trips);

	const periods: { tripIds: string[]; rate: number; isEstimated: boolean }[] = [];
	let currentPeriodTrips: string[] = [];
	let kmInPeriod = 0;
	let fuelInPeriod = 0;

	for (const trip of chronological) {
		currentPeriodTrips.push(trip.id);
		kmInPeriod += trip.distance_km;

		if (trip.fuel_liters && trip.fuel_liters > 0) {
			fuelInPeriod += trip.fuel_liters;

			if (trip.full_tank && kmInPeriod > 0) {
				const rate = (fuelInPeriod / kmInPeriod) * 100;
				periods.push({ tripIds: [...currentPeriodTrips], rate, isEstimated: false });
				currentPeriodTrips = [];
				kmInPeriod = 0;
				fuelInPeriod = 0;
			}
		}
	}

	if (currentPeriodTrips.length > 0) {
		periods.push({ tripIds: currentPeriodTrips, rate: tpConsumption, isEstimated: true });
	}

	for (const period of periods) {
		for (const tripId of period.tripIds) {
			rates.set(tripId, period.rate);
			if (period.isEstimated) {
				estimated.add(tripId);
			}
		}
	}

	return { rates, estimated };
}

/**
 * Calculate remaining fuel (zostatok) after each trip.
 *
 * Logic:
 * - Starts with full tank
 * - Deducts fuel consumed: distance * rate / 100
 * - Full tank fillup: resets to tankSize
 * - Partial fillup: adds fuel directly (can exceed tank temporarily in calc)
 * - Clamps result to [0, tankSize]
 *
 * @param trips - Array of trips
 * @param rates - Consumption rates from calculateConsumptionRates
 * @param tankSize - Vehicle's tank capacity in liters
 * @returns Map of tripId -> fuel remaining after that trip
 */
export function calculateFuelRemaining(
	trips: Trip[],
	rates: Map<string, number>,
	tankSize: number
): Map<string, number> {
	const remaining = new Map<string, number>();
	const chronological = sortChronologically(trips);

	let zostatok = tankSize;
	for (const trip of chronological) {
		const rate = rates.get(trip.id) || 0;
		const spotreba = rate > 0 ? (trip.distance_km * rate) / 100 : 0;
		zostatok = zostatok - spotreba;

		if (trip.fuel_liters && trip.fuel_liters > 0) {
			if (trip.full_tank) {
				zostatok = tankSize;
			} else {
				zostatok = zostatok + trip.fuel_liters;
			}
		}

		if (zostatok < 0) zostatok = 0;
		if (zostatok > tankSize) zostatok = tankSize;
		remaining.set(trip.id, zostatok);
	}

	return remaining;
}

/**
 * Check if each trip's date is out of order relative to its neighbors.
 * Trips are expected to be sorted by sort_order (0 = newest at top).
 *
 * @param sortedTrips - Trips sorted by sort_order
 * @returns Set of trip IDs that have date ordering issues
 */
export function calculateDateWarnings(sortedTrips: Trip[]): Set<string> {
	const warnings = new Set<string>();

	for (let i = 0; i < sortedTrips.length; i++) {
		const trip = sortedTrips[i];
		const prevTrip = i > 0 ? sortedTrips[i - 1] : null;
		const nextTrip = i < sortedTrips.length - 1 ? sortedTrips[i + 1] : null;

		// sort_order 0 = newest (should have highest date)
		// Check: prevTrip.date >= trip.date >= nextTrip.date
		if (prevTrip && trip.date > prevTrip.date) {
			warnings.add(trip.id);
		}
		if (nextTrip && trip.date < nextTrip.date) {
			warnings.add(trip.id);
		}
	}

	return warnings;
}

/**
 * Check if any trip's consumption rate exceeds 120% of TP rate (legal limit).
 *
 * @param trips - Array of trips
 * @param rates - Consumption rates from calculateConsumptionRates
 * @param tpConsumption - Vehicle's TP consumption rate
 * @returns Set of trip IDs that exceed the legal limit
 */
export function calculateConsumptionWarnings(
	trips: Trip[],
	rates: Map<string, number>,
	tpConsumption: number
): Set<string> {
	const warnings = new Set<string>();
	const limit = tpConsumption * 1.2; // 120% of TP rate

	for (const trip of trips) {
		const rate = rates.get(trip.id);
		if (rate && rate > limit) {
			warnings.add(trip.id);
		}
	}

	return warnings;
}
