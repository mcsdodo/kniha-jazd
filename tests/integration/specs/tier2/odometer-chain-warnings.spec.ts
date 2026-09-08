/**
 * Tier 2: odometer chain warnings (Task 79)
 *
 * The grid marks two shapes that break the odometer chain:
 * - two trips that share an exact start datetime, because the row order then
 *   follows the entry order and not the travel order
 * - a row whose odometer span contradicts its recorded distance
 *
 * The arithmetic of both rules belongs to the backend unit tests. This spec
 * only proves that the grid asks the backend and shows the answer.
 */

import { waitForAppReady, navigateTo } from '../../utils/app';
import { ensureLanguage } from '../../utils/language';
import { seedVehicle, seedTrip, setActiveVehicle } from '../../utils/db';
import { waitForTripGrid } from '../../utils/assertions';

const TRIP_ROW_SELECTOR =
  '.trip-grid tbody tr:not(.synthetic-row):not(.editing):not(.empty)';

/** Read the warning signs of one column across all visible trip rows. */
async function chainIndicators(column: string): Promise<string[]> {
  const rows = await $$(TRIP_ROW_SELECTOR);
  const out: string[] = [];
  for (const row of rows) {
    const marks = await row.$$(`${column} .chain-indicator`);
    for (const mark of marks) {
      out.push(await mark.getAttribute('title'));
    }
  }
  return out;
}

/** The full legend text, or an empty string when no legend is shown. */
async function legendText(): Promise<string> {
  const legend = await $('.table-legend');
  if (!(await legend.isExisting())) return '';
  return (await legend.getText()).trim();
}

describe('Tier 2: Odometer chain warnings', () => {
  let vehicleId: string;
  const year = new Date().getFullYear();

  beforeEach(async () => {
    await waitForAppReady();
    await ensureLanguage('en');

    const vehicle = await seedVehicle({
      name: 'Chain Warning Vehicle',
      licensePlate: 'CHAIN-01',
      initialOdometer: 50000,
      tankSizeLiters: 50,
      tpConsumption: 6.5,
    });
    vehicleId = vehicle.id as string;
    await setActiveVehicle(vehicleId);
  });

  it('marks both trips that share a start datetime', async () => {
    // 50000 -> 50025 -> 50055. The chain is clean, only the times collide.
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-10T15:00`,
      origin: 'Bratislava',
      destination: 'OMV Strojnicka',
      distanceKm: 25,
      odometer: 50025,
      purpose: 'Fuel stop',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-10T15:00`,
      origin: 'OMV Strojnicka',
      destination: 'Trnava',
      distanceKm: 30,
      odometer: 50055,
      purpose: 'Client visit',
    });

    // Refresh so the grid mounts with the seeded data
    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    const marks = await chainIndicators('.col-start-datetime');
    expect(marks.length).toBe(2);
    expect(marks[0]).toContain('same date and time');

    expect(await legendText()).toContain('same date and time');
    // The odometers chain correctly, so no span warning joins it.
    expect(await chainIndicators('.col-odo')).toHaveLength(0);
  });

  it('marks a row whose odometer contradicts its distance', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-10T08:00`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 50,
      odometer: 50050,
      purpose: 'Clean row',
    });
    // 50 km recorded, but the odometer moves 100 km.
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-11T08:00`,
      origin: 'Trnava',
      destination: 'Nitra',
      distanceKm: 50,
      odometer: 50150,
      purpose: 'Broken row',
    });

    // Refresh so the grid mounts with the seeded data
    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    const marks = await chainIndicators('.col-odo');
    expect(marks.length).toBe(1);
    expect(marks[0]).toContain('100');
    expect(marks[0]).toContain('50');

    expect(await legendText()).toContain('odometer does not match the distance');
    // The datetimes differ, so no duplicate warning joins it.
    expect(await chainIndicators('.col-start-datetime')).toHaveLength(0);
  });

  it('marks nothing when the chain and the datetimes are clean', async () => {
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-10T08:00`,
      origin: 'Bratislava',
      destination: 'Trnava',
      distanceKm: 50,
      odometer: 50050,
      purpose: 'First',
    });
    await seedTrip({
      vehicleId,
      startDatetime: `${year}-03-11T09:00`,
      origin: 'Trnava',
      destination: 'Nitra',
      distanceKm: 70,
      odometer: 50120,
      purpose: 'Second',
    });

    // Refresh so the grid mounts with the seeded data
    await browser.refresh();
    await waitForAppReady();
    await navigateTo('trips');
    await waitForTripGrid();
    await browser.pause(500);

    expect(await chainIndicators('.col-start-datetime')).toHaveLength(0);
    expect(await chainIndicators('.col-odo')).toHaveLength(0);

    const legend = await legendText();
    expect(legend).not.toContain('same date and time');
    expect(legend).not.toContain('odometer does not match the distance');
  });
});
