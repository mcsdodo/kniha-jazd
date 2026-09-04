<script lang="ts">
	import type { Trip, Route, PreviewResult, VehicleType, SuggestedFillup, CopiedTripDefaults } from '$lib/types';
	import { getInferredTripTimeForRoute } from '$lib/api';
	import Autocomplete from './Autocomplete.svelte';
	import { confirmStore } from '$lib/stores/confirm';
	import { toast } from '$lib/stores/toast';
	import LL from '$lib/i18n/i18n-svelte';

	export let trip: Trip | null = null;
	export let vehicleId: string = '';
	export let routes: Route[] = [];
	export let purposeSuggestions: string[] = [];
	export let isNew: boolean = false;
	export let previousOdometer: number = 0;
	export let consumptionRate: number = 0;
	export let fuelConsumed: number = 0;
	export let fuelRemaining: number = 0;
	// Energy fields (BEV/PHEV)
	export let vehicleType: VehicleType = 'Ice';
	export let energyRate: number = 0;
	export let batteryRemainingKwh: number = 0;
	export let batteryRemainingPercent: number = 0;
	export let isEstimatedEnergyRate: boolean = false;
	export let hasSocOverride: boolean = false;

	export let defaultDate: string = new Date().toISOString().split('T')[0]; // For new rows

	// Convert ISO datetime string to datetime-local format "YYYY-MM-DDTHH:MM"
	function toDatetimeLocal(isoString: string | null | undefined): string {
		if (!isoString) return `${defaultDate}T00:00`;
		// isoString is like "2026-01-29T14:30:00", we need "2026-01-29T14:30"
		return isoString.slice(0, 16);
	}

	// Format datetime for display: "DD.MM HH:MM" (no year - it's in the dropdown)
	function formatDatetimeShort(isoString: string): string {
		const date = new Date(isoString);
		const day = date.getDate().toString().padStart(2, '0');
		const month = (date.getMonth() + 1).toString().padStart(2, '0');
		const hours = date.getHours().toString().padStart(2, '0');
		const minutes = date.getMinutes().toString().padStart(2, '0');
		return `${day}.${month}. ${hours}:${minutes}`;
	}

	// Format end datetime for display
	function formatEndDatetimeShort(endDatetime: string | null | undefined, startDatetime: string): string {
		// If no end datetime, show dash
		if (!endDatetime) {
			const startDate = new Date(startDatetime);
			const day = startDate.getDate().toString().padStart(2, '0');
			const month = (startDate.getMonth() + 1).toString().padStart(2, '0');
			return `${day}.${month}. -`;
		}
		return formatDatetimeShort(endDatetime);
	}
	export let onSave: (tripData: Partial<Trip>) => void;
	export let onCancel: () => void;
	export let onDelete: (id: string) => void;
	export let onInsertAbove: () => void = () => {};
	// Copy (Task 71) - duplicates this row's route into a new today-dated row
	export let onCopy: () => void = () => {};
	export let copyDisabled: boolean = false;
	// Set on a NEW row that was opened via another row's copy button. Seeds
	// formData below; null for an ordinary new row.
	export let copyFrom: CopiedTripDefaults | null = null;
	// Route map (Task 70)
	export let hasRouteMap: boolean = false;
	export let onOpenRouteMap: () => void = () => {};
	export let onEditStart: () => void = () => {};
	export let onEditEnd: () => void = () => {};
	export let hasConsumptionWarning: boolean = false;
	export let isEstimatedRate: boolean = false;
	// Per-type invoice warnings (Task 66: 1 Fuel + N Other invoices per trip)
	export let hasMatchingFuelInvoice: boolean = true;
	export let hasMatchingOtherInvoice: boolean = true;
	export let otherSumMismatch: boolean = false;
	export let otherInvoiceSum: number | null = null;
	export let fuelDatetimeWarning: boolean = false;
	export let otherDatetimeWarning: boolean = false;
	export let fuelMismatchOverride: boolean = false;
	export let otherMismatchOverride: boolean = false;
	// Live preview props
	export let previewData: PreviewResult | null = null;
	export let onPreviewRequest: (km: number, fuel: number | null, fullTank: boolean) => void = () => {};
	// Magic fill - pre-calculated suggestion for existing trips, callback for new trips
	export let suggestedFillup: SuggestedFillup | null = null;
	export let onMagicFill: (km: number, tripId: string | null) => Promise<number> = async () => 0;

	// Hidden columns
	export let hiddenColumns: string[] = [];

	// Legal compliance (2026)
	export let tripNumber: number = 0;
	export let odoStart: number = 0;
	export let driverName: string = '';

	// Derived: show fuel/energy fields based on vehicle type
	$: showFuelFields = vehicleType === 'Ice' || vehicleType === 'Phev';
	$: showEnergyFields = vehicleType === 'Bev' || vehicleType === 'Phev';

	let isEditing = isNew;
	let manualOdoEdit = false; // Track if user manually edited ODO
	let manualKmEdit = false; // Track if user manually edited KM (see tryAutoFillDistance)

	// Form state - use null for new rows to show placeholder
	const defaultStartDatetime = `${defaultDate}T00:00`;
	let formData = {
		startDatetime: trip ? toDatetimeLocal(trip.startDatetime) : defaultStartDatetime,
		endDatetime: trip ? toDatetimeLocal(trip.endDatetime) : defaultStartDatetime,
		origin: trip?.origin || '',
		destination: trip?.destination || '',
		distanceKm: trip?.distanceKm ?? (isNew ? null : 0),
		odometer: trip?.odometer ?? (isNew ? null : 0),
		purpose: trip?.purpose || '',
		// Fuel fields
		fuelLiters: trip?.fuelLiters || null,
		fuelCostEur: trip?.fuelCostEur || null,
		fullTank: trip?.fullTank ?? true, // Default to full tank
		// Energy fields
		energyKwh: trip?.energyKwh || null,
		energyCostEur: trip?.energyCostEur || null,
		fullCharge: trip?.fullCharge ?? false,
		socOverridePercent: trip?.socOverridePercent || null,
		// Other
		otherCostsEur: trip?.otherCostsEur || null,
		otherCostsNote: trip?.otherCostsNote || ''
	};

	// Tracks the start value the current endDatetime was calculated against, so
	// a start edit can shift the end by the same amount.
	let lastStartDatetime = formData.startDatetime;

	// Format a Date back into the "YYYY-MM-DDTHH:MM" a datetime-local expects.
	// Must be local-time components — toISOString() would shift by the offset.
	function toDatetimeLocalValue(d: Date): string {
		const p = (n: number) => n.toString().padStart(2, '0');
		return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
	}

	// For new trips: move the end along with the start, PRESERVING the duration.
	// A plain new row has end == start, so this still behaves as "end follows
	// start". A copied row may carry a real span — including the +1 day of an
	// overnight trip — and collapsing it onto the start would destroy the day
	// offset the backend deliberately computed (BIZ-024).
	function handleStartDatetimeChange() {
		if (!isNew) return;

		const prevStart = new Date(lastStartDatetime);
		const newStart = new Date(formData.startDatetime);
		const end = new Date(formData.endDatetime);
		const durationMs = end.getTime() - prevStart.getTime();

		if (
			!Number.isNaN(prevStart.getTime()) &&
			!Number.isNaN(newStart.getTime()) &&
			!Number.isNaN(end.getTime()) &&
			durationMs > 0
		) {
			formData.endDatetime = toDatetimeLocalValue(new Date(newStart.getTime() + durationMs));
		} else {
			formData.endDatetime = formData.startDatetime;
		}
		lastStartDatetime = formData.startDatetime;
	}

	// Get unique locations from routes
	$: locationSuggestions = Array.from(
		new Set([...routes.map((r) => r.origin), ...routes.map((r) => r.destination)])
	).sort();

	// Find matching route and auto-fill distance
	function tryAutoFillDistance() {
		if (!formData.origin || !formData.destination) return;

		const matchingRoute = routes.find(
			(r) => r.origin === formData.origin && r.destination === formData.destination
		);

		// Defensive guard: ignore routes with absurd stored distances (> 9999 km).
		// Such values are only plausible if a previous trip was saved with
		// corrupted KM — e.g., from an earlier delta-accumulation bug. Do not
		// propagate that corruption into new rows.
		// Auto-fill when the field is empty, or when it holds a value the user
		// did not type — a copy-seeded distance, or one this function filled in
		// for a route the user has since changed. Without the manualKmEdit arm,
		// a seeded row keeps the OLD route's km after the destination changes,
		// while the times re-infer around it: a 400 km journey silently saved
		// as 47 km, feeding the l/100km and margin calculations.
		if (
			matchingRoute &&
			(formData.distanceKm === null || !manualKmEdit) &&
			matchingRoute.distanceKm > 0 &&
			matchingRoute.distanceKm <= 9999
		) {
			// Round auto-filled distance to integer km. Routes can hold fractional
			// values (legacy data, future imports), but fractional km should only
			// arise from explicit manual edits — never from auto-fill, which would
			// otherwise propagate a stale fractional value into every new trip
			// on that route.
			const roundedKm = Math.round(matchingRoute.distanceKm);
			formData.distanceKm = roundedKm;
			// Also update ODO if not manually edited
			if (!manualOdoEdit) {
				formData.odometer = previousOdometer + roundedKm;
			}
			// Trigger live preview calculation for consumption/zostatok
			onPreviewRequest(roundedKm, formData.fuelLiters, formData.fullTank);
		}
	}

	// Track the (origin, destination) we already inferred times for, so a single
	// new row does not re-invoke the backend on every keystroke or re-render.
	let inferredKey = '';

	// Task 71: seed a copied row. Applied as an override AFTER the base
	// formData init above, so the fuel/energy/cost defaults there still hold —
	// those fields are deliberately not copied.
	if (copyFrom && isNew) {
		formData.startDatetime = copyFrom.startDatetime.slice(0, 16);
		formData.endDatetime = (copyFrom.endDatetime ?? copyFrom.startDatetime).slice(0, 16);
		// The end was computed against this start, so a later start edit shifts
		// it by the delta instead of collapsing an overnight span.
		lastStartDatetime = formData.startDatetime;
		formData.origin = copyFrom.origin;
		formData.destination = copyFrom.destination;
		// The backend zeroes an implausible distance rather than copying it;
		// null (not 0) leaves the field blank and lets auto-fill take over.
		formData.distanceKm = copyFrom.distanceKm > 0 ? copyFrom.distanceKm : null;
		formData.odometer = previousOdometer + (formData.distanceKm ?? 0);
		formData.purpose = copyFrom.purpose;
		// The copied times are explicit user intent. Marking this route pair as
		// already-inferred makes tryInferTimes() short-circuit, so the Task 56
		// jitter never overwrites them. Picking a DIFFERENT route changes the
		// key, so inference correctly resumes — and manualKmEdit stays false so
		// tryAutoFillDistance replaces the seeded km to match.
		inferredKey = `${copyFrom.origin}␟${copyFrom.destination}`;
		// Populate the live consumption/zostatok preview, matching what
		// tryAutoFillDistance does when it auto-fills km.
		onPreviewRequest(formData.distanceKm ?? 0, null, formData.fullTank);
	}

	// On a new row, infer start/end times from the most recent trip with the
	// same (vehicleId, origin, destination). Backend supplies the final ISO
	// datetimes (jitter is applied in Rust per ADR-008).
	async function tryInferTimes() {
		if (!isNew || !vehicleId) return;
		if (!formData.origin || !formData.destination) return;
		const key = `${formData.origin}\u241F${formData.destination}`;
		if (key === inferredKey) return;
		inferredKey = key;

		const rowDate = formData.startDatetime.slice(0, 10); // "YYYY-MM-DD"
		try {
			const result = await getInferredTripTimeForRoute(
				vehicleId, formData.origin, formData.destination, rowDate
			);
			if (result) {
				// Snapshot pre-overwrite values so undo can restore them.
				const previousStart = formData.startDatetime;
				const previousEnd = formData.endDatetime;

				// Apply the inferred times. Store as datetime-local
				// ("YYYY-MM-DDTHH:MM"), preserving the row's date.
				formData.startDatetime = result.startDatetime.slice(0, 16);
				formData.endDatetime = result.endDatetime.slice(0, 16);

				// Toast with undo action: lets the user revert to the typed
				// values and re-trigger inference if they change their mind.
				toast.withAction(
					$LL.trips.timeInferenceApplied(),
					$LL.trips.timeInferenceUndo(),
					() => {
						formData.startDatetime = previousStart;
						formData.endDatetime = previousEnd;
						inferredKey = ''; // allow re-trigger after undo
					}
				);
			}
		} catch (e) {
			// Inference is best-effort; failures must not block manual entry.
			console.warn('Time inference failed:', e);
		}
	}

	function handleOriginSelect(value: string) {
		formData.origin = value;
		tryAutoFillDistance();
		tryInferTimes();
	}

	function handleDestinationSelect(value: string) {
		formData.destination = value;
		tryAutoFillDistance();
		tryInferTimes();
	}

	// Auto-update ODO when km changes (unless user manually edited ODO)
	function handleKmChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		const km = inputValue === '' ? null : (parseFloat(inputValue) || 0);
		formData.distanceKm = km;
		// A typed distance outranks any route the user later picks — mirrors
		// manualOdoEdit. Clearing the field hands control back to auto-fill.
		manualKmEdit = km !== null;
		// Always auto-calculate ODO if not manually edited (previousOdometer can be 0)
		if (!manualOdoEdit && km !== null) {
			formData.odometer = previousOdometer + km;
		}
		// Request live preview calculation
		onPreviewRequest(km ?? 0, formData.fuelLiters, formData.fullTank);
	}

	// Request preview when fuel changes
	function handleFuelChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		formData.fuelLiters = inputValue === '' ? null : (parseFloat(inputValue) || null);
		onPreviewRequest(formData.distanceKm ?? 0, formData.fuelLiters, formData.fullTank);
	}

	// Request preview when fullTank changes
	function handleFullTankChange() {
		onPreviewRequest(formData.distanceKm ?? 0, formData.fuelLiters, formData.fullTank);
	}

	// Clamp ODO to (previousOdometer + 1) when finalised below the previous row's value.
	// Runs on `change` (blur / Enter) so mid-typing keystrokes are not snapped away.
	function handleOdoBlur() {
		if (
			previousOdometer > 0 &&
			formData.odometer !== null &&
			formData.odometer < previousOdometer
		) {
			formData.odometer = previousOdometer + 1;
			formData.distanceKm = 1;
			onPreviewRequest(formData.distanceKm, formData.fuelLiters, formData.fullTank);
		}
	}

	function handleOdoChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		const newOdo = inputValue === '' ? null : (parseFloat(inputValue) || 0);

		if (newOdo === formData.odometer) return;

		manualOdoEdit = true;
		formData.odometer = newOdo;

		if (newOdo === null) {
			formData.distanceKm = null;
			onPreviewRequest(0, formData.fuelLiters, formData.fullTank);
			return;
		}

		// KM is the gap between this row's ODO and the previous row's ODO.
		// Two guards protect against degenerate situations:
		//   1. previousOdometer === 0 means there is no meaningful baseline
		//      (fresh vehicle with no initialOdometer set). In that case the
		//      subtraction produces the raw ODO value — which looks to the
		//      user like "ODO ended up in the KM field". Skip auto-derivation
		//      and let the user type KM explicitly.
		//   2. Any single-trip distance > 9999 km is almost certainly the
		//      result of a missing baseline rather than a real trip.
		const candidate = newOdo - previousOdometer;
		if (previousOdometer > 0 && candidate >= 0 && candidate <= 9999) {
			formData.distanceKm = candidate;
			onPreviewRequest(candidate, formData.fuelLiters, formData.fullTank);
		}
	}

	function handleEdit() {
		isEditing = true;
		onEditStart();
		// Trigger preview immediately with current values
		onPreviewRequest(formData.distanceKm ?? 0, formData.fuelLiters, formData.fullTank);
	}

	async function handleMagicFill() {
		const currentKm = formData.distanceKm ?? 0;
		if (currentKm <= 0) return;

		let suggestedLiters: number;

		// For existing trips, use pre-calculated suggestion (no backend call)
		// For new trips, call backend
		if (trip?.id && suggestedFillup) {
			suggestedLiters = suggestedFillup.liters;
		} else {
			const tripId = trip?.id ?? null;
			suggestedLiters = await onMagicFill(currentKm, tripId);
		}

		if (suggestedLiters > 0) {
			formData.fuelLiters = suggestedLiters;
			formData.fullTank = true;
			// Trigger preview with new fuel value
			onPreviewRequest(currentKm, suggestedLiters, true);
		}
	}

	function handleSave() {
		// Final ODO clamp: never persist a value below the previous row's ODO.
		// Belt-and-suspenders behind the on:change clamp in handleOdoBlur.
		let odo = formData.odometer ?? 0;
		if (previousOdometer > 0 && odo < previousOdometer) {
			odo = previousOdometer + 1;
		}
		const km = odo === (formData.odometer ?? 0)
			? (formData.distanceKm ?? 0)
			: Math.max(1, odo - previousOdometer);
		const dataToSave = {
			...formData,
			distanceKm: km,
			odometer: odo
		};
		onSave(dataToSave);
		isEditing = false;
		if (!isNew) {
			onEditEnd();
		}
	}

	function handleCancel() {
		if (isNew) {
			onCancel();
		} else {
			// Reset form data
			const currentDate = new Date().toISOString().split('T')[0];
			formData = {
				startDatetime: trip ? toDatetimeLocal(trip.startDatetime) : `${currentDate}T00:00`,
				endDatetime: trip ? toDatetimeLocal(trip.endDatetime) : `${currentDate}T00:00`,
				origin: trip?.origin || '',
				destination: trip?.destination || '',
				distanceKm: trip?.distanceKm || 0,
				odometer: trip?.odometer || 0,
				purpose: trip?.purpose || '',
				fuelLiters: trip?.fuelLiters || null,
				fuelCostEur: trip?.fuelCostEur || null,
				fullTank: trip?.fullTank ?? true, // Default to full tank
				energyKwh: trip?.energyKwh || null,
				energyCostEur: trip?.energyCostEur || null,
				fullCharge: trip?.fullCharge ?? false,
				socOverridePercent: trip?.socOverridePercent || null,
				otherCostsEur: trip?.otherCostsEur || null,
				otherCostsNote: trip?.otherCostsNote || ''
			};
			isEditing = false;
			onEditEnd();
		}
	}

	function handleDeleteClick() {
		if (trip?.id) {
			confirmStore.show({
				title: $LL.confirm.deleteRecordTitle(),
				message: $LL.confirm.deleteRecordMessage(),
				confirmText: $LL.common.delete(),
				danger: true,
				onConfirm: () => onDelete(trip!.id)
			});
		}
	}

	// Single global keyboard handler for editing mode
	// ESC = cancel, Enter = submit (works regardless of focus)
	function handleGlobalKeydown(event: KeyboardEvent) {
		if (!isEditing) return;

		if (event.key === 'Escape') {
			// ESC always cancels editing
			event.preventDefault();
			handleCancel();
		} else if (event.key === 'Enter' && !event.shiftKey) {
			// Check if user is actively interacting with an autocomplete dropdown
			// Only defer to autocomplete if: (1) dropdown exists AND (2) an autocomplete input has focus
			// This avoids race condition with the 200ms blur delay that keeps dropdown in DOM
			const hasOpenDropdown = document.querySelector('.autocomplete .dropdown') !== null;
			const autocompleteHasFocus = document.activeElement?.closest('.autocomplete') !== null;
			if (hasOpenDropdown && autocompleteHasFocus) {
				// Let Autocomplete handle the selection first
				// Next Enter (after dropdown closes) will submit
				return;
			}
			event.preventDefault();
			handleSave();
		}
	}

</script>

<svelte:window on:keydown={handleGlobalKeydown} />

{#if isEditing}
	<tr class="editing">
		{#if !hiddenColumns.includes('tripNumber')}
			<td class="col-trip-number number">{isNew ? '-' : tripNumber}</td>
		{/if}
		<td class="col-start-datetime">
			<input
				type="datetime-local"
				bind:value={formData.startDatetime}
				on:change={handleStartDatetimeChange}
				data-testid="trip-start-datetime"
				required
			/>
		</td>
		{#if !hiddenColumns.includes('time')}
			<td class="col-end-datetime">
				<input
					type="datetime-local"
					bind:value={formData.endDatetime}
					data-testid="trip-end-datetime"
					required
				/>
			</td>
		{/if}
		<td class="col-origin">
			<Autocomplete
				bind:value={formData.origin}
				suggestions={locationSuggestions}
				placeholder={$LL.trips.originPlaceholder()}
				onSelect={handleOriginSelect}
				testId="trip-origin"
			/>
		</td>
		<td class="col-destination">
			<Autocomplete
				bind:value={formData.destination}
				suggestions={locationSuggestions}
				placeholder={$LL.trips.destinationPlaceholder()}
				onSelect={handleDestinationSelect}
				testId="trip-destination"
			/>
		</td>
		<td class="col-km">
			<input type="number" value={formData.distanceKm} on:input={handleKmChange} step="1" min="0" placeholder="0" data-testid="trip-distance" />
		</td>
		{#if !hiddenColumns.includes('odoStart')}
			<td class="col-odo-start number">{isNew ? '-' : odoStart.toFixed(0)}</td>
		{/if}
		<td class="col-odo">
			<input type="number" value={formData.odometer} on:input={handleOdoChange} on:change={handleOdoBlur} step="1" min="0" placeholder="0" data-testid="trip-odometer" />
		</td>
		<td class="col-purpose">
			<Autocomplete
				bind:value={formData.purpose}
				suggestions={purposeSuggestions}
				placeholder={$LL.trips.purposePlaceholder()}
				onSelect={(value) => (formData.purpose = value)}
				testId="trip-purpose"
			/>
		</td>
		{#if !hiddenColumns.includes('driver')}
			<td class="col-driver">{driverName}</td>
		{/if}
		{#if showFuelFields}
			<td class="col-fuel-liters fuel-cell">
				<input
					type="number"
					value={formData.fuelLiters}
					on:input={handleFuelChange}
					step="0.01"
					min="0"
					placeholder="0.00"
					data-testid="trip-fuel-liters"
				/>
				{#if formData.fuelLiters}
					<label class="full-tank-label">
						<input type="checkbox" bind:checked={formData.fullTank} on:change={handleFullTankChange} data-testid="trip-full-tank" />
						<span class="checkmark"></span>
						<span class="label-text">{$LL.trips.fullTank()}</span>
					</label>
				{/if}
			</td>
			<td class="col-fuel-cost">
				<input
					type="number"
					bind:value={formData.fuelCostEur}
					step="0.01"
					min="0"
					placeholder="0.00"
					data-testid="trip-fuel-cost"
				/>
			</td>
			{#if !hiddenColumns.includes('fuelConsumed')}
				<td class="col-fuel-consumed number calculated" class:preview={previewData}>
					{#if previewData}
						~{((formData.distanceKm || 0) * previewData.consumptionRate / 100).toFixed(2)}
					{:else}
						{fuelConsumed.toFixed(2)}
					{/if}
				</td>
			{/if}
			<td class="col-consumption-rate number calculated" class:preview={previewData} class:over-limit={previewData?.isOverLimit}>
				{#if previewData}
					~{previewData.consumptionRate.toFixed(2)}
					<span class="margin" class:over-limit={previewData.isOverLimit} class:within-limit={!previewData.isOverLimit}>
						({previewData.marginPercent >= 0 ? '+' : ''}{previewData.marginPercent.toFixed(0)}%)
					</span>
				{:else}
					{consumptionRate.toFixed(2)}
				{/if}
			</td>
			{#if !hiddenColumns.includes('fuelRemaining')}
				<td class="col-fuel-remaining number calculated" class:preview={previewData}>
					{#if previewData}
						~{previewData.fuelRemaining.toFixed(1)}
					{:else}
						{fuelRemaining.toFixed(1)}
					{/if}
				</td>
			{/if}
		{/if}
		{#if showEnergyFields}
			<td class="col-energy-kwh energy-cell">
				<input
					type="number"
					bind:value={formData.energyKwh}
					step="0.1"
					min="0"
					placeholder="0.0"
					data-testid="trip-energy-kwh"
				/>
				{#if formData.energyKwh}
					<label class="full-charge-label">
						<input type="checkbox" bind:checked={formData.fullCharge} data-testid="trip-full-charge" />
						<span class="checkmark"></span>
						<span class="label-text">{$LL.trips.fullCharge()}</span>
					</label>
				{/if}
			</td>
			<td class="col-energy-cost">
				<input
					type="number"
					bind:value={formData.energyCostEur}
					step="0.01"
					min="0"
					placeholder="0.00"
					data-testid="trip-energy-cost"
				/>
			</td>
			<td class="col-energy-rate number calculated">
				{energyRate.toFixed(2)}
			</td>
			<td class="col-battery-remaining number calculated soc-cell">
				{batteryRemainingKwh.toFixed(1)} kWh
				<span class="battery-percent">({batteryRemainingPercent.toFixed(0)}%)</span>
				{#if !isNew}
					<details class="soc-override-details">
						<summary title={$LL.trips.socOverrideHint()}>⚡</summary>
						<div class="soc-override-input">
							<input
								type="number"
								bind:value={formData.socOverridePercent}
								step="1"
								min="0"
								max="100"
								placeholder="%"
								data-testid="trip-soc-override"
							/>
							<span class="soc-hint">{$LL.trips.socOverrideHint()}</span>
						</div>
					</details>
				{/if}
			</td>
		{/if}
		{#if !hiddenColumns.includes('otherCosts')}
			<td class="col-other-costs">
				<input
					type="number"
					bind:value={formData.otherCostsEur}
					step="0.01"
					min="0"
					placeholder="0.00"
					data-testid="trip-other-costs"
				/>
			</td>
		{/if}
		{#if !hiddenColumns.includes('otherCostsNote')}
			<td class="col-other-costs-note">
				<input
					type="text"
					bind:value={formData.otherCostsNote}
					placeholder=""
					data-testid="trip-other-costs-note"
				/>
			</td>
		{/if}
		<td class="col-actions actions editing-actions">
			<button class="icon-btn magic" on:click={handleMagicFill} title={$LL.trips.magicFill()}>
				<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="m12 3-1.9 5.8a2 2 0 0 1-1.3 1.3L3 12l5.8 1.9a2 2 0 0 1 1.3 1.3L12 21l1.9-5.8a2 2 0 0 1 1.3-1.3L21 12l-5.8-1.9a2 2 0 0 1-1.3-1.3Z"></path>
				</svg>
			</button>
			<button class="icon-btn save" on:click={handleSave} title={$LL.common.save()}>
				<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<polyline points="20 6 9 17 4 12"></polyline>
				</svg>
			</button>
			<button class="icon-btn cancel" on:click={handleCancel} title={$LL.common.cancel()}>
				<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<line x1="18" y1="6" x2="6" y2="18"></line>
					<line x1="6" y1="6" x2="18" y2="18"></line>
				</svg>
			</button>
		</td>
	</tr>
{:else if trip}
	<tr
		on:dblclick={handleEdit}
		class:consumption-warning={hasConsumptionWarning}
	>
		{#if !hiddenColumns.includes('tripNumber')}
			<td class="col-trip-number number">{tripNumber}</td>
		{/if}
		<td class="col-start-datetime">{formatDatetimeShort(trip.startDatetime)}</td>
		{#if !hiddenColumns.includes('time')}
			<td class="col-end-datetime">{formatEndDatetimeShort(trip.endDatetime, trip.startDatetime)}</td>
		{/if}
		<td class="col-origin">{trip.origin}</td>
		<td class="col-destination">{trip.destination}</td>
		<td class="col-km number">{trip.distanceKm.toFixed(0)}</td>
		{#if !hiddenColumns.includes('odoStart')}
			<td class="col-odo-start number">{odoStart.toFixed(0)}</td>
		{/if}
		<td class="col-odo number">{trip.odometer.toFixed(0)}</td>
		<td class="col-purpose">{trip.purpose}</td>
		{#if !hiddenColumns.includes('driver')}
			<td class="col-driver">{driverName}</td>
		{/if}
		{#if showFuelFields}
			<td class="col-fuel-liters number">
				{#if trip.fuelLiters}
					{trip.fuelLiters.toFixed(2)}
					{#if !trip.fullTank}
						<span class="partial-indicator" title={$LL.trips.partialFillup()}>*</span>
					{/if}
					{#if !hasMatchingFuelInvoice}
						<span class="receipt-indicator missing" title={$LL.trips.legend.missingFuelInvoice()}>⚠</span>
					{:else if fuelDatetimeWarning && !fuelMismatchOverride}
						<span class="receipt-indicator mismatch" title={$LL.trips.legend.dataMismatch()}>⚠</span>
					{/if}
				{/if}
			</td>
			<td class="col-fuel-cost number">{trip.fuelCostEur?.toFixed(2) || ''}</td>
			{#if !hiddenColumns.includes('fuelConsumed')}
				<td class="col-fuel-consumed number calculated">{fuelConsumed.toFixed(2)}</td>
			{/if}
			<td class="col-consumption-rate number calculated" class:estimated={isEstimatedRate}>
				{consumptionRate.toFixed(2)}
				{#if isEstimatedRate}
					<span class="estimated-indicator" title={$LL.trips.estimatedRate()}>~</span>
				{/if}
			</td>
			{#if !hiddenColumns.includes('fuelRemaining')}
				<td class="col-fuel-remaining number calculated">{fuelRemaining.toFixed(1)}</td>
			{/if}
		{/if}
		{#if showEnergyFields}
			<td class="col-energy-kwh number">
				{#if trip.energyKwh}
					{trip.energyKwh.toFixed(1)}
					{#if !trip.fullCharge}
						<span class="partial-indicator" title={$LL.trips.partialCharge()}>*</span>
					{/if}
				{/if}
			</td>
			<td class="col-energy-cost number">{trip.energyCostEur?.toFixed(2) || ''}</td>
			<td class="col-energy-rate number calculated" class:estimated={isEstimatedEnergyRate}>
				{energyRate.toFixed(2)}
				{#if isEstimatedEnergyRate}
					<span class="estimated-indicator" title={$LL.trips.estimatedRate()}>~</span>
				{/if}
			</td>
			<td class="col-battery-remaining number calculated" class:soc-override={hasSocOverride}>
				{batteryRemainingKwh.toFixed(1)} kWh
				<span class="battery-percent">({batteryRemainingPercent.toFixed(0)}%)</span>
				{#if hasSocOverride}
					<span class="soc-indicator" title={$LL.trips.socOverride()}>⚡</span>
				{/if}
			</td>
		{/if}
		{#if !hiddenColumns.includes('otherCosts')}
			<td class="col-other-costs number">
				{trip.otherCostsEur?.toFixed(2) || ''}
				{#if trip.otherCostsEur || otherSumMismatch}
					{#if !hasMatchingOtherInvoice}
						<span class="receipt-indicator missing" title={$LL.trips.legend.missingOtherInvoice()}>⚠</span>
					{:else if otherSumMismatch}
						<span class="receipt-indicator mismatch" title={$LL.trips.legend.otherSumMismatch({ total: (trip.otherCostsEur ?? 0).toFixed(2), sum: (otherInvoiceSum ?? 0).toFixed(2) })}>⚠</span>
					{:else if otherDatetimeWarning && !otherMismatchOverride}
						<span class="receipt-indicator mismatch" title={$LL.trips.legend.dataMismatch()}>⚠</span>
					{/if}
				{/if}
			</td>
		{/if}
		{#if !hiddenColumns.includes('otherCostsNote')}
			<td class="col-other-costs-note">{trip.otherCostsNote || ''}</td>
		{/if}
		<td class="col-actions actions">
			<span class="icon-actions">
				<button
					class="icon-btn insert"
					on:click|stopPropagation={onInsertAbove}
					title={$LL.trips.insertAbove()}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<line x1="12" y1="5" x2="12" y2="19"></line>
						<line x1="5" y1="12" x2="19" y2="12"></line>
					</svg>
				</button>
				<button
					class="icon-btn copy"
					on:click|stopPropagation={onCopy}
					disabled={copyDisabled}
					title={$LL.trips.copyRecord()}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
						<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
					</svg>
				</button>
				<button
					class="icon-btn map"
					class:has-map={hasRouteMap}
					on:click|stopPropagation={onOpenRouteMap}
					title={hasRouteMap ? $LL.routeMap.viewMap() : $LL.routeMap.addMap()}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill={hasRouteMap ? 'currentColor' : 'none'} stroke="currentColor" stroke-width="2">
						<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"></path>
						<circle cx="12" cy="10" r="3"></circle>
					</svg>
				</button>
				<button
					class="icon-btn delete"
					on:click|stopPropagation={handleDeleteClick}
					title={$LL.trips.deleteRecord()}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="3 6 5 6 21 6"></polyline>
						<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
					</svg>
				</button>
			</span>
		</td>
	</tr>
{/if}

<style>
	tr {
		cursor: default;
		transition: background-color 0.2s;
	}

	tr:hover:not(.editing) {
		background-color: var(--bg-surface-alt);
		cursor: pointer;
	}

	tr.editing {
		background-color: var(--editing-row-bg);
		cursor: default;
	}

	tr.editing td input,
	tr.editing td :global(.autocomplete) {
		margin: 0 1px;
		width: calc(100% - 2px);
	}

	tr.consumption-warning {
		background-color: var(--warning-bg); /* light orange */
	}

	tr.consumption-warning:hover:not(.editing) {
		background-color: var(--warning-border); /* slightly darker orange on hover */
	}

	td {
		padding: 0.5rem;
		border-bottom: 1px solid var(--border-default);
	}

	tr:not(.editing) td {
		padding-left: 0.9rem;
	}

	td.number {
		text-align: right;
	}

	td.calculated {
		color: var(--text-secondary);
		font-style: italic;
	}

	/* Live preview styling */
	td.preview {
		opacity: 0.85;
	}

	td.over-limit {
		background-color: var(--warning-bg);
	}

	.margin {
		font-size: 0.75rem;
		margin-left: 0.25rem;
	}

	.margin.over-limit {
		color: var(--accent-danger);
		font-weight: 500;
	}

	.margin.within-limit {
		color: var(--accent-success);
	}

	td.actions {
		text-align: right;
		white-space: nowrap;
	}

	input {
		width: 100%;
		padding: 0.5rem 0.125rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 0.875rem;
		box-sizing: border-box;
	}

	tr.editing input[type='number'] {
		text-align: right;
		padding-right: 0.25rem;
	}

	button {
		padding: 0.375rem 0.75rem;
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
		margin: 0 0.25rem;
	}

	.editing-actions {
		display: flex;
		gap: 0.25rem;
		justify-content: flex-end;
		align-items: center;
	}

	.icon-actions {
		display: flex;
		gap: 0.25rem;
		justify-content: flex-end;
		align-items: center;
	}

	.icon-btn {
		background: none;
		border: none;
		padding: 0.25rem;
		cursor: pointer;
		color: var(--text-muted);
		border-radius: 4px;
		transition: color 0.2s, background-color 0.2s;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		margin: 0;
	}

	.icon-btn:hover {
		background-color: var(--icon-btn-hover-bg);
	}

	.icon-btn.insert:hover {
		color: var(--accent-primary);
	}

	.icon-btn.copy:hover {
		color: var(--accent-primary);
	}

	/* Route map (Task 70): saved maps are tinted so a filled pin reads as
	   "map exists" even before hover. */
	.icon-btn.map:hover {
		color: var(--accent-primary);
	}

	.icon-btn.map.has-map {
		color: var(--accent-primary);
	}

	.icon-btn.delete:hover {
		color: var(--accent-danger);
		background-color: var(--accent-danger-bg);
	}

	.icon-btn.save:hover {
		color: var(--accent-success);
		background-color: var(--accent-success-bg);
	}

	.icon-btn.cancel:hover {
		color: var(--accent-warning);
	}

	.icon-btn.magic:hover {
		color: var(--accent-primary);
	}

	.icon-btn:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	/* Fuel cell with checkbox */
	.fuel-cell {
		position: relative;
	}

	.full-tank-label {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.full-tank-label input[type='checkbox'] {
		width: auto;
		margin: 0;
		cursor: pointer;
	}

	.full-tank-label .label-text {
		white-space: nowrap;
	}

	/* Partial fillup indicator */
	.partial-indicator {
		color: var(--accent-warning);
		font-weight: bold;
		margin-left: 0.25rem;
	}

	/* No receipt indicator */
	/* Receipt status indicators - Task 51 */
	.receipt-indicator {
		margin-left: 0.25rem;
		cursor: help;
		font-weight: bold;
	}

	/* 🔴 Missing receipt - red */
	.receipt-indicator.missing {
		color: var(--accent-danger);
	}

	/* 🟡 Data mismatch - yellow/orange warning */
	.receipt-indicator.mismatch {
		color: var(--accent-warning-dark);
	}

	/* 🟠 User confirmed override - orange */
	.receipt-indicator.override {
		color: #f97316; /* Orange */
	}

	/* Estimated rate styling */
	td.estimated {
		color: var(--text-muted);
	}

	.estimated-indicator {
		color: var(--text-muted);
		margin-left: 0.125rem;
	}

	/* Energy cell with checkbox */
	.energy-cell {
		position: relative;
	}

	.full-charge-label {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.full-charge-label input[type='checkbox'] {
		width: auto;
		margin: 0;
		cursor: pointer;
	}

	.full-charge-label .label-text {
		white-space: nowrap;
	}

	/* Battery percent display */
	.battery-percent {
		font-size: 0.75rem;
		color: var(--text-secondary);
		margin-left: 0.125rem;
	}

	/* SoC override styling */
	td.soc-override {
		color: var(--accent-primary);
	}

	.soc-indicator {
		color: var(--accent-primary);
		margin-left: 0.125rem;
		cursor: help;
	}

	/* SoC override input (expandable) */
	.soc-cell {
		position: relative;
	}

	.soc-override-details {
		display: inline-block;
		margin-left: 0.25rem;
	}

	.soc-override-details summary {
		cursor: pointer;
		color: var(--text-secondary);
		font-size: 0.875rem;
		list-style: none;
	}

	.soc-override-details summary::-webkit-details-marker {
		display: none;
	}

	.soc-override-details[open] summary {
		color: var(--accent-primary);
	}

	.soc-override-input {
		position: absolute;
		top: 100%;
		right: 0;
		background: var(--bg-surface);
		border: 1px solid var(--border-input);
		border-radius: 4px;
		padding: 0.5rem;
		box-shadow: 0 2px 8px var(--shadow-default);
		z-index: 10;
		min-width: 160px;
	}

	.soc-override-input input {
		width: 60px;
		margin-bottom: 0.25rem;
	}

	.soc-hint {
		display: block;
		font-size: 0.7rem;
		color: var(--text-secondary);
		line-height: 1.2;
	}
</style>
