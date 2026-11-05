/**
 * Tier 2: the year stats on the phone start page (Task 93)
 *
 * The start page at /m shows the stats row of the PC page and its
 * suggestions: the compensation banner and the suggested fill-up. The
 * numbers come from the backend; this spec checks only that the page shows
 * them (the math is in the backend tests).
 */

import { waitForAppReady } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle, getTripGridData } from '../../utils/db';
import { SlovakCities, TripPurposes } from '../../fixtures/trips';

describe('Tier 2: phone start page stats', () => {
  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');
  });

  it('shows the year stats, the compensation banner and the suggested fill-up', async () => {
    const vehicle = await seedVehicle({
      name: 'Phone Stats Test',
      licensePlate: 'MOB-001',
      initialOdometer: 40000,
      vehicleType: 'Ice',
      tankSizeLiters: 50,
      tpConsumption: 7.0,
    });
    const vehicleId = vehicle.id as string;
    const year = new Date().getFullYear();

    // 100 km with 9 l: 9.0 l/100 km, over the limit of 8.4 (7.0 * 1.2).
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-01T08:00`,
      origin: SlovakCities.bratislava,
      destination: SlovakCities.trnava,
      distanceKm: 50,
      odometer: 40050,
      purpose: TripPurposes.business,
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-05T08:00`,
      origin: SlovakCities.trnava,
      destination: SlovakCities.nitra,
      distanceKm: 50,
      odometer: 40100,
      purpose: TripPurposes.business,
      fuelLiters: 9.0,
      fuelCostEur: 13.5,
      fullTank: true,
    });
    // A trip after the last fill-up opens a period: the backend suggests a fill-up.
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-08T08:00`,
      origin: SlovakCities.nitra,
      destination: SlovakCities.bratislava,
      distanceKm: 30,
      odometer: 40130,
      purpose: TripPurposes.business,
    });

    const grid = await getTripGridData(vehicleId, year);
    expect(grid.legendSuggestedFillup).not.toBeNull();

    await setActiveVehicle(vehicleId);
    await browser.url('/m');

    const stats = await $('[data-testid="mobile-home-stats"]');
    await stats.waitForDisplayed({ timeout: 15000 });

    expect(await $('[data-testid="mobile-stat-km"]').getText()).toBe('130 km');
    expect(await $('[data-testid="mobile-stat-fuel"]').getText()).toContain('9.0 l');
    expect(await $('[data-testid="mobile-stat-consumption"]').getText()).toContain('l/100 km');
    await expect($('[data-testid="mobile-stat-deviation"]')).toBeDisplayed();

    const banner = await $('[data-testid="mobile-home-compensation"] .compensation-banner');
    await expect(banner).toBeDisplayed();

    const fillup = await $('[data-testid="mobile-stat-suggested-fillup"]');
    await expect(fillup).toBeDisplayed();
    expect(await fillup.getText()).toContain(grid.legendSuggestedFillup!.liters.toFixed(2));
  });
});
