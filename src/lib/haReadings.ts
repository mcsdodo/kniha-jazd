import type { HaOdoCache } from '$lib/types';

// The Home Assistant readings next to the year stats. The PC page and the
// phone start page both use this helper, so they show the same numbers.
// The sensor values arrive in the browser (haStore), so this display step
// stays in the frontend (ADR-013).

/** The real odometer is this many km over the book: show a warning. */
export const HA_ODO_WARNING_KM = 50;

export interface HaReadings {
	/** The real odometer in km, or null without a reading. */
	odo: number | null;
	/** Real odometer minus the highest odometer in the book. Positive: km missing in the book. */
	odoDelta: number | null;
	odoWarning: boolean;
	/** The real fuel in the tank, in liters, or null without a reading or a tank size. */
	fuelLiters: number | null;
	/** Book fuel remaining minus the real fuel. Positive: liters to refill. */
	fuelRefill: number | null;
}

export function haReadings(
	cache: HaOdoCache | null | undefined,
	maxLoggedOdo: number | null,
	fuelRemainingLiters: number | null,
	tankSizeLiters: number | null | undefined
): HaReadings {
	const odo = cache ? cache.value : null;
	const odoDelta = odo !== null && maxLoggedOdo !== null ? odo - maxLoggedOdo : null;
	const percent = cache?.fuelLevelPercent ?? null;
	const fuelLiters = percent !== null && tankSizeLiters ? (percent * tankSizeLiters) / 100 : null;
	const fuelRefill =
		fuelLiters !== null && fuelRemainingLiters !== null ? fuelRemainingLiters - fuelLiters : null;
	return {
		odo,
		odoDelta,
		odoWarning: odoDelta !== null && odoDelta >= HA_ODO_WARNING_KM,
		fuelLiters,
		fuelRefill
	};
}
