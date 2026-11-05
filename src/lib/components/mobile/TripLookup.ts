// Fast per-trip lookups into the backend grid data (Task 93). The sets only
// index the backend lists; nothing here computes a value (ADR-008).
import type { TripGridData, VehicleType } from '$lib/types';

/** The Paperless rows of the year: 'off' = Paperless is not configured. */
export type InvoiceState = 'loading' | 'ready' | 'error' | 'off';

export interface TripLookup {
	grid: TripGridData;
	showFuel: boolean;
	showEnergy: boolean;
	estimated: Set<string>;
	estimatedEnergy: Set<string>;
	overLimit: Set<string>;
	missingFuelInvoice: Set<string>;
	missingOtherInvoice: Set<string>;
	otherSumMismatch: Set<string>;
	fuelDatetime: Set<string>;
	otherDatetime: Set<string>;
	fuelOverride: Set<string>;
	otherOverride: Set<string>;
	duplicateDatetime: Set<string>;
	odometerSpan: Set<string>;
	routeMap: Set<string>;
}

export function buildTripLookup(grid: TripGridData, vehicleType: VehicleType): TripLookup {
	return {
		grid,
		showFuel: vehicleType !== 'Bev',
		showEnergy: vehicleType !== 'Ice',
		estimated: new Set(grid.estimatedRates),
		estimatedEnergy: new Set(grid.estimatedEnergyRates),
		overLimit: new Set(grid.consumptionWarnings),
		missingFuelInvoice: new Set(grid.missingFuelInvoices),
		missingOtherInvoice: new Set(grid.missingOtherInvoices),
		otherSumMismatch: new Set(grid.otherSumMismatches),
		fuelDatetime: new Set(grid.fuelDatetimeWarnings),
		otherDatetime: new Set(grid.otherDatetimeWarnings),
		fuelOverride: new Set(grid.fuelMismatchOverrides),
		otherOverride: new Set(grid.otherMismatchOverrides),
		duplicateDatetime: new Set(grid.duplicateDatetimeWarnings),
		odometerSpan: new Set(grid.odometerSpanWarnings),
		routeMap: new Set(grid.routeMapTripIds)
	};
}
