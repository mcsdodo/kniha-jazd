<script lang="ts">
	import type { Trip, Route, PreviewResult, VehicleType, SuggestedFillup } from '$lib/types';
	import { extractTime } from '$lib/types';
	import Autocomplete from './Autocomplete.svelte';
	import { confirmStore } from '$lib/stores/confirm';
	import LL from '$lib/i18n/i18n-svelte';

	export let trip: Trip | null = null;
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
	export let onSave: (tripData: Partial<Trip>) => void;
	export let onCancel: () => void;
	export let onDelete: (id: string) => void;
	export let onInsertAbove: () => void = () => {};
	export let onEditStart: () => void = () => {};
	export let onEditEnd: () => void = () => {};
	export let onMoveUp: () => void = () => {};
	export let onMoveDown: () => void = () => {};
	export let canMoveUp: boolean = false;
	export let canMoveDown: boolean = false;
	export let hasDateWarning: boolean = false;
	export let hasConsumptionWarning: boolean = false;
	export let isEstimatedRate: boolean = false;
	export let hasMatchingReceipt: boolean = true;
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
	export let isMonthEnd: boolean = false;

	// Derived: show fuel/energy fields based on vehicle type
	$: showFuelFields = vehicleType === 'Ice' || vehicleType === 'Phev';
	$: showEnergyFields = vehicleType === 'Bev' || vehicleType === 'Phev';

	let isEditing = isNew;
	let manualOdoEdit = false; // Track if user manually edited ODO

	// Form state - use null for new rows to show placeholder
	let formData = {
		date: trip?.date || defaultDate,
		time: trip?.datetime ? extractTime(trip.datetime) : '00:00',
		endTime: trip?.endTime || '',
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

		if (matchingRoute && formData.distanceKm === null) {
			formData.distanceKm = matchingRoute.distanceKm;
			// Also update ODO if not manually edited
			if (!manualOdoEdit) {
				formData.odometer = previousOdometer + matchingRoute.distanceKm;
			}
			// Trigger live preview calculation for consumption/zostatok
			onPreviewRequest(matchingRoute.distanceKm, formData.fuelLiters, formData.fullTank);
		}
	}

	function handleOriginSelect(value: string) {
		formData.origin = value;
		tryAutoFillDistance();
	}

	function handleDestinationSelect(value: string) {
		formData.destination = value;
		tryAutoFillDistance();
	}

	// Auto-update ODO when km changes (unless user manually edited ODO)
	function handleKmChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		const km = inputValue === '' ? null : (parseFloat(inputValue) || 0);
		formData.distanceKm = km;
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

	function handleOdoChange(event: Event) {
		const inputValue = (event.target as HTMLInputElement).value;
		const newOdo = inputValue === '' ? null : (parseFloat(inputValue) || 0);

		// Only process if value actually changed
		if (newOdo === formData.odometer) return;

		manualOdoEdit = true;
		const oldOdo = formData.odometer;
		formData.odometer = newOdo;

		// Bidirectional: recalculate KM from ODO
		// Use the expected previous ODO (current ODO - current KM) to handle edits correctly
		if (newOdo !== null && oldOdo !== null && formData.distanceKm !== null) {
			// Calculate the delta and apply it to KM
			const delta = newOdo - oldOdo;
			formData.distanceKm = Math.max(0, (formData.distanceKm ?? 0) + delta);
			// Trigger live preview with updated KM
			onPreviewRequest(formData.distanceKm, formData.fuelLiters, formData.fullTank);
		} else if (newOdo !== null) {
			// Fallback: calculate from previousOdometer (for new trips or when values are null)
			formData.distanceKm = Math.max(0, newOdo - previousOdometer);
			onPreviewRequest(formData.distanceKm, formData.fuelLiters, formData.fullTank);
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
		// Ensure numeric fields have proper values (convert null to 0)
		const dataToSave = {
			...formData,
			distanceKm: formData.distanceKm ?? 0,
			odometer: formData.odometer ?? 0
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
			formData = {
				date: trip?.date || new Date().toISOString().split('T')[0],
				time: trip?.datetime ? extractTime(trip.datetime) : '00:00',
				endTime: trip?.endTime || '',
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
		<td class="col-date">
			<input type="date" bind:value={formData.date} data-testid="trip-date" />
		</td>
		{#if !hiddenColumns.includes('time')}
			<td class="col-time">
				<input type="time" bind:value={formData.time} data-testid="trip-time" />
			</td>
			<td class="col-end-time">
				<input type="time" bind:value={formData.endTime} data-testid="trip-end-time" />
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
			<input type="number" value={formData.odometer} on:input={handleOdoChange} step="1" min="0" placeholder="0" data-testid="trip-odometer" />
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
		class:date-warning={hasDateWarning}
		class:consumption-warning={hasConsumptionWarning}
		class:month-end-trip={isMonthEnd}
	>
		{#if !hiddenColumns.includes('tripNumber')}
			<td class="col-trip-number number">{tripNumber}</td>
		{/if}
		<td class="col-date">{new Date(trip.date).toLocaleDateString('sk-SK')}</td>
		{#if !hiddenColumns.includes('time')}
			<td class="col-time">{extractTime(trip.datetime)}</td>
			<td class="col-end-time">{trip.endTime || ''}</td>
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
					{#if !hasMatchingReceipt}
						<span class="no-receipt-indicator" title={$LL.trips.noReceipt()}>⚠</span>
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
			<td class="col-other-costs number">{trip.otherCostsEur?.toFixed(2) || ''}</td>
		{/if}
		{#if !hiddenColumns.includes('otherCostsNote')}
			<td class="col-other-costs-note">{trip.otherCostsNote || ''}</td>
		{/if}
		<td class="col-actions actions">
			<span class="icon-actions">
				<button
					class="icon-btn move-up"
					on:click|stopPropagation={onMoveUp}
					title={$LL.trips.moveUp()}
					disabled={!canMoveUp}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="18 15 12 9 6 15"></polyline>
					</svg>
				</button>
				<button
					class="icon-btn move-down"
					on:click|stopPropagation={onMoveDown}
					title={$LL.trips.moveDown()}
					disabled={!canMoveDown}
				>
					<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
						<polyline points="6 9 12 15 18 9"></polyline>
					</svg>
				</button>
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

	tr.date-warning {
		background-color: var(--danger-bg); /* light red */
	}

	tr.date-warning:hover:not(.editing) {
		background-color: var(--danger-bg-hover); /* slightly darker red on hover */
	}

	tr.consumption-warning {
		background-color: var(--warning-bg); /* light orange */
	}

	tr.consumption-warning:hover:not(.editing) {
		background-color: var(--warning-border); /* slightly darker orange on hover */
	}

	/* If both warnings apply, date warning takes priority */
	tr.date-warning.consumption-warning {
		background-color: var(--danger-bg);
	}

	/* Month-end trip highlighting (legal compliance 2026) */
	tr.month-end-trip {
		background: #e8f4fc;
		border-bottom: 2px solid #4a90d9;
	}

	:global(.dark) tr.month-end-trip {
		background: #1a3a4a;
	}

	tr.month-end-trip:hover:not(.editing) {
		background: #d8e8f5;
	}

	:global(.dark) tr.month-end-trip:hover:not(.editing) {
		background: #254a5a;
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

	.icon-btn.move-up:hover:not(:disabled),
	.icon-btn.move-down:hover:not(:disabled) {
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
	.no-receipt-indicator {
		color: var(--accent-warning-dark);
		margin-left: 0.25rem;
		cursor: help;
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
