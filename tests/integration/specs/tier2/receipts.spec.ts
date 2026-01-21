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
  syncReceipts,
  updateReceipt,
} from '../../utils/db';
import type { Receipt } from '../../fixtures/types';
import { createTestIceVehicle } from '../../fixtures/vehicles';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

// ESM workaround for __dirname
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

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

      // 4. Sync receipts - this both scans for new files AND processes them
      // (mock Gemini loads invoice.json instead of calling API)
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

      // 4. Sync receipts (scans and processes with mock Gemini)
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

      // 4. Sync receipts (scans and processes with mock Gemini)
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

  describe('Multi-Currency Receipts', () => {
    /**
     * Tests for multi-currency receipt support:
     * - CZK receipt should have NeedsReview status (no EUR conversion)
     * - After updating with EUR amount, status should change
     * - EUR receipt should auto-populate total_price_eur
     *
     * Uses invoice-czk.pdf with mock data from invoice-czk.json:
     * - original_amount: 250.0
     * - original_currency: "CZK"
     * - total_price_eur: null (needs conversion)
     */
    it('should create CZK receipt with NeedsReview status', async () => {
      // 1. Seed vehicle
      const vehicleData = createTestIceVehicle({
        name: 'Multi-Currency Test Vehicle',
        licensePlate: 'CURR-001',
        initialOdometer: 400000,
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

      // 2. Set receipts folder to test invoices directory
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
      await setReceiptsFolderPath(invoicesPath);

      // 3. Sync receipts - this scans for files and processes them with mock Gemini
      await syncReceipts();
      await browser.pause(500);

      // 4. Get receipts and find the CZK one
      const receipts = await getReceipts(2026);
      const czkReceipt = receipts.find(r => r.fileName === 'invoice-czk.pdf');

      expect(czkReceipt).toBeDefined();
      if (czkReceipt) {
        // Verify CZK receipt properties
        expect(czkReceipt.originalAmount).toBe(250.0);
        expect(czkReceipt.originalCurrency).toBe('CZK');
        expect(czkReceipt.totalPriceEur).toBeNull(); // Not converted yet
        expect(czkReceipt.status).toBe('NeedsReview'); // Foreign currency needs review
        expect(czkReceipt.vendorName).toBe('Parkoviště Praha');
        expect(czkReceipt.costDescription).toBe('Parkovné 2h');
      }
    });

    it('should update CZK receipt with EUR conversion', async () => {
      // 1. Get existing receipts (from previous test or fresh sync)
      const vehicleData = createTestIceVehicle({
        name: 'Currency Conversion Test Vehicle',
        licensePlate: 'CONV-001',
        initialOdometer: 500000,
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

      // 2. Set receipts folder and sync
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
      await setReceiptsFolderPath(invoicesPath);
      await syncReceipts();
      await browser.pause(500);

      // 3. Get receipts and find CZK one
      let receipts = await getReceipts(2026);
      const czkReceipt = receipts.find(r => r.fileName === 'invoice-czk.pdf');

      expect(czkReceipt).toBeDefined();
      if (!czkReceipt) return;

      // 4. Update receipt with EUR conversion (250 CZK ≈ 10 EUR)
      const updatedReceipt: Receipt = {
        ...czkReceipt,
        totalPriceEur: 10.0, // Manual EUR conversion
        status: 'Parsed', // Now it's properly parsed
      };

      await updateReceipt(updatedReceipt);
      await browser.pause(300);

      // 5. Verify the update
      receipts = await getReceipts(2026);
      const verifiedReceipt = receipts.find(r => r.id === czkReceipt.id);

      expect(verifiedReceipt).toBeDefined();
      if (verifiedReceipt) {
        expect(verifiedReceipt.totalPriceEur).toBe(10.0);
        expect(verifiedReceipt.originalAmount).toBe(250.0); // Original preserved
        expect(verifiedReceipt.originalCurrency).toBe('CZK'); // Currency preserved
        expect(verifiedReceipt.status).toBe('Parsed'); // Status updated
      }
    });

    it('should auto-populate total_price_eur for EUR receipts', async () => {
      // 1. Seed vehicle
      const vehicleData = createTestIceVehicle({
        name: 'EUR Receipt Test Vehicle',
        licensePlate: 'EURR-001',
        initialOdometer: 600000,
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

      // 2. Set receipts folder and sync
      const invoicesPath = join(__dirname, '..', '..', 'data', 'invoices');
      await setReceiptsFolderPath(invoicesPath);
      await syncReceipts();
      await browser.pause(500);

      // 3. Get receipts and find EUR one (invoice.pdf)
      const receipts = await getReceipts(2026);
      const eurReceipt = receipts.find(r => r.fileName === 'invoice.pdf');

      expect(eurReceipt).toBeDefined();
      if (eurReceipt) {
        // Verify EUR receipt auto-populated total_price_eur
        expect(eurReceipt.originalAmount).toBe(91.32);
        expect(eurReceipt.originalCurrency).toBe('EUR');
        expect(eurReceipt.totalPriceEur).toBe(91.32); // Auto-populated from original_amount
        expect(eurReceipt.status).toBe('Parsed'); // EUR receipts should be Parsed, not NeedsReview
      }
    });
  });
});
