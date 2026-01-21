/**
 * Tier 2: Receipts Workflow Integration Tests
 *
 * Tests the receipt management functionality including:
 * - Receipt scanning and processing (with mock Gemini)
 * - Mismatch detection for trip assignment
 * - Viewing receipts list
 *
 * NOTE: These tests use mock Gemini responses from tests/integration/data/mocks/
 * See tests/integration/data/README.md for mock file format.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import {
  seedVehicle,
  seedTrip,
  setActiveVehicle,
  getReceipts,
  getReceiptsForVehicle,
  getTripsForReceiptAssignment,
  setReceiptsFolderPath,
  triggerReceiptScan,
  syncReceipts,
} from '../../utils/db';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { join } from 'path';

describe('Tier 2: Receipts Workflow', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  describe('Receipt Vehicle Filtering', () => {
    it('should get receipts for a specific vehicle via IPC', async () => {
      // Seed two vehicles
      const vehicle1Data = createTestIceVehicle({
        name: 'Vehicle 1 for Receipts',
        licensePlate: 'VEH1-001',
        initialOdometer: 50000,
        tpConsumption: 7.0,
        tankSizeLiters: 50,
      });

      const vehicle2Data = createTestIceVehicle({
        name: 'Vehicle 2 for Receipts',
        licensePlate: 'VEH2-001',
        initialOdometer: 60000,
        tpConsumption: 7.5,
        tankSizeLiters: 55,
      });

      const vehicle1 = await seedVehicle({
        name: vehicle1Data.name,
        licensePlate: vehicle1Data.licensePlate,
        initialOdometer: vehicle1Data.initialOdometer,
        vehicleType: vehicle1Data.vehicleType,
        tankSizeLiters: vehicle1Data.tankSizeLiters,
        tpConsumption: vehicle1Data.tpConsumption,
      });

      const vehicle2 = await seedVehicle({
        name: vehicle2Data.name,
        licensePlate: vehicle2Data.licensePlate,
        initialOdometer: vehicle2Data.initialOdometer,
        vehicleType: vehicle2Data.vehicleType,
        tankSizeLiters: vehicle2Data.tankSizeLiters,
        tpConsumption: vehicle2Data.tpConsumption,
      });

      const year = new Date().getFullYear();

      // Get receipts for vehicle 1 via IPC - should return empty or existing receipts
      const vehicle1Receipts = await getReceiptsForVehicle(vehicle1.id as string, year);
      const vehicle2Receipts = await getReceiptsForVehicle(vehicle2.id as string, year);

      // Receipts arrays should be defined (even if empty)
      expect(vehicle1Receipts).toBeDefined();
      expect(vehicle2Receipts).toBeDefined();
      expect(Array.isArray(vehicle1Receipts)).toBe(true);
      expect(Array.isArray(vehicle2Receipts)).toBe(true);
    });
  });

  describe('Mismatch Detection E2E', () => {
    /**
     * This test verifies the full flow:
     * 1. Seed a vehicle and trip
     * 2. Set receipts folder to test data
     * 3. Scan for receipts (finds invoice.pdf)
     * 4. Sync receipts (mock Gemini returns invoice.json data)
     * 5. Call get_trips_for_receipt_assignment
     * 6. Verify mismatch_reason is returned correctly
     *
     * Invoice mock data (from tests/integration/data/mocks/invoice.json):
     * - liters: 63.68
     * - total_price_eur: 91.32
     * - receipt_date: "2026-01-20"
     */
    it('should return mismatch_reason via IPC when trip data differs', async () => {
      // 1. Seed vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Mismatch Test Vehicle',
        licensePlate: 'MISM-001',
        initialOdometer: 100000,
        tpConsumption: 7.0,
        tankSizeLiters: 70,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      // 2. Seed trip with DIFFERENT liters (40.0 vs receipt's 63.68)
      // This should trigger "liters" mismatch
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2026-01-20', // Same date as receipt
        origin: 'Bratislava',
        destination: 'Trnava',
        distanceKm: 65,
        odometer: 100065,
        purpose: 'Služobná cesta',
        fuelLiters: 40.0, // Different from receipt's 63.68
        fuelCostEur: 91.32, // Same as receipt
        fullTank: true,
      });

      // 3. Set receipts folder to test invoices directory
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
      await setReceiptsFolderPath(invoicesPath);

      // 4. Scan for receipts (should find invoice.pdf)
      await triggerReceiptScan();
      await browser.pause(500);

      // 5. Sync receipts (mock Gemini loads invoice.json)
      await syncReceipts();
      await browser.pause(500);

      // 6. Get receipts and verify we have one
      const receipts = await getReceipts(2026);
      expect(receipts.length).toBeGreaterThan(0);

      const receipt = receipts[0];
      expect(receipt).toBeDefined();

      // 7. Get trips for receipt assignment and check mismatch
      const tripsForAssignment = await getTripsForReceiptAssignment(
        receipt.id,
        vehicle.id as string,
        2026
      );

      // Should have at least one trip
      expect(tripsForAssignment.length).toBeGreaterThan(0);

      // Find our seeded trip in the results
      const tripMatch = tripsForAssignment.find(t => t.trip.id === trip.id);
      expect(tripMatch).toBeDefined();

      if (tripMatch) {
        // Verify mismatch detection
        expect(tripMatch.attachmentStatus).toBe('differs');
        expect(tripMatch.mismatchReason).toBe('liters');
        expect(tripMatch.canAttach).toBe(false);
      }
    });

    it('should return "matches" when trip data matches receipt exactly', async () => {
      // 1. Seed vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Match Test Vehicle',
        licensePlate: 'MTCH-001',
        initialOdometer: 200000,
        tpConsumption: 7.0,
        tankSizeLiters: 70,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      // 2. Seed trip with MATCHING data
      // Receipt: liters=63.68, price=91.32, date=2026-01-20
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2026-01-20',
        origin: 'Bratislava',
        destination: 'Košice',
        distanceKm: 400,
        odometer: 200400,
        purpose: 'Služobná cesta',
        fuelLiters: 63.68, // Exact match
        fuelCostEur: 91.32, // Exact match
        fullTank: true,
      });

      // 3. Set receipts folder
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
      await setReceiptsFolderPath(invoicesPath);

      // 4. Scan and sync
      await triggerReceiptScan();
      await browser.pause(500);
      await syncReceipts();
      await browser.pause(500);

      // 5. Get receipts
      const receipts = await getReceipts(2026);
      expect(receipts.length).toBeGreaterThan(0);

      const receipt = receipts[0];

      // 6. Get trips for assignment
      const tripsForAssignment = await getTripsForReceiptAssignment(
        receipt.id,
        vehicle.id as string,
        2026
      );

      const tripMatch = tripsForAssignment.find(t => t.trip.id === trip.id);
      expect(tripMatch).toBeDefined();

      if (tripMatch) {
        // Verify exact match
        expect(tripMatch.attachmentStatus).toBe('matches');
        expect(tripMatch.mismatchReason).toBeNull();
        expect(tripMatch.canAttach).toBe(true);
      }
    });

    it('should return "empty" when trip has no fuel data', async () => {
      // 1. Seed vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Empty Trip Test Vehicle',
        licensePlate: 'EMTY-001',
        initialOdometer: 300000,
        tpConsumption: 7.0,
        tankSizeLiters: 70,
      });

      const vehicle = await seedVehicle({
        name: vehicleData.name,
        licensePlate: vehicleData.licensePlate,
        initialOdometer: vehicleData.initialOdometer,
        vehicleType: vehicleData.vehicleType,
        tankSizeLiters: vehicleData.tankSizeLiters,
        tpConsumption: vehicleData.tpConsumption,
      });

      await setActiveVehicle(vehicle.id as string);

      // 2. Seed trip WITHOUT fuel data
      const trip = await seedTrip({
        vehicleId: vehicle.id as string,
        date: '2026-01-20',
        origin: 'Nitra',
        destination: 'Žilina',
        distanceKm: 150,
        odometer: 300150,
        purpose: 'Služobná cesta',
        // No fuelLiters, no fuelCostEur
      });

      // 3. Set receipts folder
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
      await setReceiptsFolderPath(invoicesPath);

      // 4. Scan and sync
      await triggerReceiptScan();
      await browser.pause(500);
      await syncReceipts();
      await browser.pause(500);

      // 5. Get receipts
      const receipts = await getReceipts(2026);
      expect(receipts.length).toBeGreaterThan(0);

      const receipt = receipts[0];

      // 6. Get trips for assignment
      const tripsForAssignment = await getTripsForReceiptAssignment(
        receipt.id,
        vehicle.id as string,
        2026
      );

      const tripMatch = tripsForAssignment.find(t => t.trip.id === trip.id);
      expect(tripMatch).toBeDefined();

      if (tripMatch) {
        // Trip has no fuel data - can attach freely
        expect(tripMatch.attachmentStatus).toBe('empty');
        expect(tripMatch.mismatchReason).toBeNull();
        expect(tripMatch.canAttach).toBe(true);
      }
    });
  });

  describe('Receipt Display', () => {
    it('should display receipts page', async () => {
      // Navigate to receipts page
      await navigateTo('doklady');
      await browser.pause(500);

      // Verify we're on the receipts page (doklady = receipts in Slovak)
      const url = await browser.getUrl();
      expect(url).toContain('doklady');

      // The page should load without errors
      const body = await $('body');
      const pageText = await body.getText();
      expect(pageText).toBeDefined();
    });
  });
});
