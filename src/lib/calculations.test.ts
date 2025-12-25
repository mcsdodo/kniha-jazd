/**
 * Unit tests for frontend calculation functions.
 * These tests mirror the Rust backend tests to ensure consistency.
 */

import { describe, it, expect } from 'vitest';
import {
	calculateConsumptionRates,
	calculateFuelRemaining,
	calculateDateWarnings,
	calculateConsumptionWarnings
} from './calculations';
import type { Trip } from './types';

// Helper to create test trips
function createTrip(overrides: Partial<Trip> & { id: string; date: string }): Trip {
	return {
		vehicle_id: 'test-vehicle',
		origin: 'Prague',
		destination: 'Brno',
		distance_km: 200,
		odometer: 50000,
		purpose: 'Business',
		fuel_liters: null,
		fuel_cost_eur: null,
		other_costs_eur: null,
		other_costs_note: null,
		full_tank: true,
		sort_order: 0,
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString(),
		...overrides
	};
}

describe('calculateConsumptionRates', () => {
	const TP_CONSUMPTION = 5.1;

	it('returns empty maps for empty trips array', () => {
		const { rates, estimated } = calculateConsumptionRates([], TP_CONSUMPTION);
		expect(rates.size).toBe(0);
		expect(estimated.size).toBe(0);
	});

	it('uses TP rate for trips with no fillups', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 100 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 150 })
		];

		const { rates, estimated } = calculateConsumptionRates(trips, TP_CONSUMPTION);

		expect(rates.get('1')).toBe(TP_CONSUMPTION);
		expect(rates.get('2')).toBe(TP_CONSUMPTION);
		expect(estimated.has('1')).toBe(true);
		expect(estimated.has('2')).toBe(true);
	});

	it('calculates rate from single full tank fillup', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 370 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 458, fuel_liters: 50.36, full_tank: true })
		];

		const { rates, estimated } = calculateConsumptionRates(trips, TP_CONSUMPTION);

		// Rate = 50.36 / (370 + 458) * 100 = 6.08 l/100km
		const expectedRate = (50.36 / 828) * 100;
		expect(rates.get('1')).toBeCloseTo(expectedRate, 2);
		expect(rates.get('2')).toBeCloseTo(expectedRate, 2);
		expect(estimated.has('1')).toBe(false);
		expect(estimated.has('2')).toBe(false);
	});

	it('all trips in same period share the same rate', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 100 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 200 }),
			createTrip({ id: '3', date: '2024-01-03', distance_km: 300 }),
			createTrip({ id: '4', date: '2024-01-04', distance_km: 200, fuel_liters: 48, full_tank: true })
		];

		const { rates } = calculateConsumptionRates(trips, TP_CONSUMPTION);

		// All trips should have the same rate
		const rate = rates.get('1');
		expect(rates.get('2')).toBe(rate);
		expect(rates.get('3')).toBe(rate);
		expect(rates.get('4')).toBe(rate);
	});

	it('partial fillup does not close period', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 300 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 200, fuel_liters: 20, full_tank: false }), // Partial
			createTrip({ id: '3', date: '2024-01-03', distance_km: 500 })
		];

		const { rates, estimated } = calculateConsumptionRates(trips, TP_CONSUMPTION);

		// No full tank fillup, so all use TP rate
		expect(rates.get('1')).toBe(TP_CONSUMPTION);
		expect(rates.get('2')).toBe(TP_CONSUMPTION);
		expect(rates.get('3')).toBe(TP_CONSUMPTION);
		expect(estimated.has('1')).toBe(true);
		expect(estimated.has('2')).toBe(true);
		expect(estimated.has('3')).toBe(true);
	});

	it('sums partial + full fillups in span', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 400 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 300, fuel_liters: 20, full_tank: false }), // Partial
			createTrip({ id: '3', date: '2024-01-03', distance_km: 300, fuel_liters: 30, full_tank: true })   // Full
		];

		const { rates, estimated } = calculateConsumptionRates(trips, TP_CONSUMPTION);

		// Rate = (20 + 30) / (400 + 300 + 300) * 100 = 5.0 l/100km
		const expectedRate = (50 / 1000) * 100;
		expect(rates.get('1')).toBeCloseTo(expectedRate, 2);
		expect(rates.get('2')).toBeCloseTo(expectedRate, 2);
		expect(rates.get('3')).toBeCloseTo(expectedRate, 2);
		expect(estimated.has('1')).toBe(false);
	});

	it('trips after last full tank use TP rate', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 400 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 400, fuel_liters: 40, full_tank: true }),
			createTrip({ id: '3', date: '2024-01-03', distance_km: 300 }), // After fillup
			createTrip({ id: '4', date: '2024-01-04', distance_km: 200 })
		];

		const { rates, estimated } = calculateConsumptionRates(trips, TP_CONSUMPTION);

		// Trips 1-2 have calculated rate
		expect(estimated.has('1')).toBe(false);
		expect(estimated.has('2')).toBe(false);

		// Trips 3-4 use TP rate
		expect(rates.get('3')).toBe(TP_CONSUMPTION);
		expect(rates.get('4')).toBe(TP_CONSUMPTION);
		expect(estimated.has('3')).toBe(true);
		expect(estimated.has('4')).toBe(true);
	});

	// Excel verification test - matches Rust test_excel_first_fillup_consumption_rate
	it('Excel verification: first fillup consumption rate', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-02', distance_km: 370, odometer: 66370 }),
			createTrip({ id: '2', date: '2024-01-03', distance_km: 458, odometer: 66828, fuel_liters: 50.36, full_tank: true })
		];

		const { rates } = calculateConsumptionRates(trips, 5.1);

		// Excel shows 6.08 l/100km
		expect(rates.get('2')).toBeCloseTo(6.08, 1);
	});
});

describe('calculateFuelRemaining', () => {
	const TANK_SIZE = 66;

	it('starts with full tank', () => {
		const trips = [createTrip({ id: '1', date: '2024-01-01', distance_km: 100 })];
		const rates = new Map([['1', 6.0]]);

		const remaining = calculateFuelRemaining(trips, rates, TANK_SIZE);

		// 66 - (100 * 6 / 100) = 66 - 6 = 60
		expect(remaining.get('1')).toBeCloseTo(60, 1);
	});

	it('full tank fillup resets to tank size', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 500 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 100, fuel_liters: 50, full_tank: true })
		];
		const rates = new Map([['1', 6.0], ['2', 6.0]]);

		const remaining = calculateFuelRemaining(trips, rates, TANK_SIZE);

		// After trip 1: 66 - 30 = 36
		// After trip 2: full tank = 66
		expect(remaining.get('2')).toBe(TANK_SIZE);
	});

	it('partial fillup adds fuel directly', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', distance_km: 200 }),
			createTrip({ id: '2', date: '2024-01-02', distance_km: 100, fuel_liters: 20, full_tank: false })
		];
		const rates = new Map([['1', 6.0], ['2', 6.0]]);

		const remaining = calculateFuelRemaining(trips, rates, TANK_SIZE);

		// After trip 1: 66 - 12 = 54
		// After trip 2: 54 - 6 + 20 = 68, capped to 66
		expect(remaining.get('2')).toBe(TANK_SIZE);
	});

	it('clamps zostatok to 0 when negative', () => {
		const trips = [createTrip({ id: '1', date: '2024-01-01', distance_km: 2000 })];
		const rates = new Map([['1', 6.0]]);

		const remaining = calculateFuelRemaining(trips, rates, TANK_SIZE);

		// 66 - 120 would be negative, clamped to 0
		expect(remaining.get('1')).toBe(0);
	});

	// Excel verification - matches Rust test_zostatok_equals_tank_after_fillup
	it('Excel verification: zostatok equals tank after fillup', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-02', distance_km: 370 }),
			createTrip({ id: '2', date: '2024-01-03', distance_km: 458, fuel_liters: 50.36, full_tank: true })
		];
		// Using the rate that would be calculated
		const rate = (50.36 / 828) * 100;
		const rates = new Map([['1', rate], ['2', rate]]);

		const remaining = calculateFuelRemaining(trips, rates, TANK_SIZE);

		// After full tank fillup, zostatok should be exactly tank size
		expect(remaining.get('2')).toBe(TANK_SIZE);
	});
});

describe('calculateDateWarnings', () => {
	it('returns empty set for correctly ordered trips', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-03', sort_order: 0 }), // Newest first
			createTrip({ id: '2', date: '2024-01-02', sort_order: 1 }),
			createTrip({ id: '3', date: '2024-01-01', sort_order: 2 })  // Oldest last
		];

		const warnings = calculateDateWarnings(trips);
		expect(warnings.size).toBe(0);
	});

	it('detects date out of order', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', sort_order: 0 }), // Wrong: oldest at top
			createTrip({ id: '2', date: '2024-01-03', sort_order: 1 }), // Wrong: newer below older
			createTrip({ id: '3', date: '2024-01-02', sort_order: 2 })
		];

		const warnings = calculateDateWarnings(trips);
		expect(warnings.size).toBeGreaterThan(0);
	});

	it('no warning for single trip', () => {
		const trips = [createTrip({ id: '1', date: '2024-01-01', sort_order: 0 })];

		const warnings = calculateDateWarnings(trips);
		expect(warnings.size).toBe(0);
	});

	it('no warning for same dates', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01', sort_order: 0 }),
			createTrip({ id: '2', date: '2024-01-01', sort_order: 1 })
		];

		const warnings = calculateDateWarnings(trips);
		expect(warnings.size).toBe(0);
	});
});

describe('calculateConsumptionWarnings', () => {
	const TP_CONSUMPTION = 5.1;

	it('no warning when rate is under 120% of TP', () => {
		const trips = [createTrip({ id: '1', date: '2024-01-01' })];
		const rates = new Map([['1', 5.5]]); // 108% of TP

		const warnings = calculateConsumptionWarnings(trips, rates, TP_CONSUMPTION);
		expect(warnings.has('1')).toBe(false);
	});

	it('no warning when rate is exactly at 120% of TP', () => {
		const trips = [createTrip({ id: '1', date: '2024-01-01' })];
		const limit = TP_CONSUMPTION * 1.2; // 6.12
		const rates = new Map([['1', limit]]);

		const warnings = calculateConsumptionWarnings(trips, rates, TP_CONSUMPTION);
		expect(warnings.has('1')).toBe(false);
	});

	it('warning when rate exceeds 120% of TP', () => {
		const trips = [createTrip({ id: '1', date: '2024-01-01' })];
		const rates = new Map([['1', 7.0]]); // 137% of TP

		const warnings = calculateConsumptionWarnings(trips, rates, TP_CONSUMPTION);
		expect(warnings.has('1')).toBe(true);
	});

	it('multiple trips with mixed rates', () => {
		const trips = [
			createTrip({ id: '1', date: '2024-01-01' }),
			createTrip({ id: '2', date: '2024-01-02' }),
			createTrip({ id: '3', date: '2024-01-03' })
		];
		const rates = new Map([
			['1', 5.0],  // OK
			['2', 7.5],  // Over limit
			['3', 6.0]   // OK
		]);

		const warnings = calculateConsumptionWarnings(trips, rates, TP_CONSUMPTION);
		expect(warnings.has('1')).toBe(false);
		expect(warnings.has('2')).toBe(true);
		expect(warnings.has('3')).toBe(false);
	});
});
