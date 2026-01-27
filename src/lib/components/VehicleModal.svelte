<script lang="ts">
	import type { Vehicle, VehicleType } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';

	export let vehicle: Vehicle | null = null;
	export let hasTrips = false; // Whether vehicle has trips (type change blocked)
	export let haConfigured = false; // Whether Home Assistant is configured
	export let onSave: (data: {
		name: string;
		licensePlate: string;
		initialOdometer: number;
		vehicleType: VehicleType;
		tankSizeLiters: number | null;
		tpConsumption: number | null;
		batteryCapacityKwh: number | null;
		baselineConsumptionKwh: number | null;
		initialBatteryPercent: number | null;
		vin: string | null;
		driverName: string | null;
		haOdoSensor: string | null;
	}) => void;
	export let onClose: () => void;

	let name = vehicle?.name || '';
	let licensePlate = vehicle?.licensePlate || '';
	let vehicleType: VehicleType = vehicle?.vehicleType || 'Ice';
	let tankSizeLiters = vehicle?.tankSizeLiters ?? 0;
	let tpConsumption = vehicle?.tpConsumption ?? 0;
	let batteryCapacityKwh = vehicle?.batteryCapacityKwh ?? 0;
	let baselineConsumptionKwh = vehicle?.baselineConsumptionKwh ?? 0;
	let initialBatteryPercent = vehicle?.initialBatteryPercent ?? 100;

	let initialOdometer = vehicle?.initialOdometer || 0;
	let vin = vehicle?.vin || '';
	let driverName = vehicle?.driverName || '';
	let haOdoSensor = vehicle?.haOdoSensor || '';

	// Show fuel fields for ICE and PHEV
	$: showFuelFields = vehicleType === 'Ice' || vehicleType === 'Phev';
	// Show battery fields for BEV and PHEV
	$: showBatteryFields = vehicleType === 'Bev' || vehicleType === 'Phev';

	function handleSave() {
		onSave({
			name,
			licensePlate,
			initialOdometer,
			vehicleType,
			tankSizeLiters: showFuelFields ? tankSizeLiters : null,
			tpConsumption: showFuelFields ? tpConsumption : null,
			batteryCapacityKwh: showBatteryFields ? batteryCapacityKwh : null,
			baselineConsumptionKwh: showBatteryFields ? baselineConsumptionKwh : null,
			initialBatteryPercent: showBatteryFields ? initialBatteryPercent : null,
			vin,
			driverName,
			haOdoSensor: haOdoSensor || null
		});
	}

	function handleBackgroundClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			onClose();
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}
</script>

<div class="modal-backdrop" on:click={handleBackgroundClick} on:keydown={handleKeydown} role="button" tabindex="-1">
	<div class="modal-content">
		<div class="modal-header">
			<h2>{vehicle ? $LL.vehicleModal.editTitle() : $LL.vehicleModal.addTitle()}</h2>
			<button class="close-button" on:click={onClose}>&times;</button>
		</div>

		<div class="modal-body">
			<div class="form-group">
				<label for="name">{$LL.vehicleModal.nameLabel()}</label>
				<input type="text" id="name" bind:value={name} placeholder={$LL.vehicleModal.namePlaceholder()} />
			</div>

			<div class="form-group">
				<label for="license-plate">{$LL.vehicleModal.licensePlateLabel()}</label>
				<input
					type="text"
					id="license-plate"
					bind:value={licensePlate}
					placeholder={$LL.vehicleModal.licensePlatePlaceholder()}
				/>
			</div>

			<div class="form-group">
				<label for="vehicle-type">{$LL.vehicleModal.vehicleTypeLabel()}</label>
				<select id="vehicle-type" bind:value={vehicleType} disabled={hasTrips}>
					<option value="Ice">{$LL.vehicle.vehicleTypeIce()}</option>
					<option value="Bev">{$LL.vehicle.vehicleTypeBev()}</option>
					<option value="Phev">{$LL.vehicle.vehicleTypePhev()}</option>
				</select>
				{#if hasTrips}
					<span class="hint">{$LL.vehicle.typeChangeBlocked()}</span>
				{/if}
			</div>

			<div class="form-group">
				<label for="initial-odometer">{$LL.vehicleModal.initialOdometerLabel()}</label>
				<input
					type="number"
					id="initial-odometer"
					bind:value={initialOdometer}
					step="0.1"
					min="0"
					placeholder={$LL.vehicleModal.initialOdometerPlaceholder()}
				/>
			</div>

			<div class="form-group">
				<label for="vin">{$LL.vehicleModal.vinLabel()}</label>
				<input type="text" id="vin" bind:value={vin} placeholder={$LL.vehicleModal.vinPlaceholder()} />
			</div>

			<div class="form-group">
				<label for="driver-name">{$LL.vehicleModal.driverLabel()}</label>
				<input
					type="text"
					id="driver-name"
					bind:value={driverName}
					placeholder={$LL.vehicleModal.driverPlaceholder()}
				/>
			</div>

			{#if haConfigured}
				<div class="form-group">
					<label for="ha-odo-sensor">{$LL.homeAssistant.sensorLabel()}</label>
					<input
						type="text"
						id="ha-odo-sensor"
						bind:value={haOdoSensor}
						placeholder={$LL.homeAssistant.sensorPlaceholder()}
					/>
					<span class="hint">{$LL.homeAssistant.sensorHint()}</span>
				</div>
			{/if}

			{#if showFuelFields}
				<div class="section-header">{$LL.vehicleModal.fuelSection()}</div>
				<div class="form-group">
					<label for="tank-size">{$LL.vehicleModal.tankSizeLabel()}</label>
					<input
						type="number"
						id="tank-size"
						bind:value={tankSizeLiters}
						step="0.1"
						min="0"
						placeholder={$LL.vehicleModal.tankSizePlaceholder()}
					/>
				</div>

				<div class="form-group">
					<label for="tp-consumption">{$LL.vehicleModal.tpConsumptionLabel()}</label>
					<input
						type="number"
						id="tp-consumption"
						bind:value={tpConsumption}
						step="0.1"
						min="0"
						placeholder={$LL.vehicleModal.tpConsumptionPlaceholder()}
					/>
				</div>
			{/if}

			{#if showBatteryFields}
				<div class="section-header">{$LL.vehicleModal.batterySection()}</div>
				<div class="form-group">
					<label for="battery-capacity">{$LL.vehicleModal.batteryCapacityLabel()}</label>
					<input
						type="number"
						id="battery-capacity"
						bind:value={batteryCapacityKwh}
						step="0.1"
						min="0"
						placeholder={$LL.vehicleModal.batteryCapacityPlaceholder()}
					/>
				</div>

				<div class="form-group">
					<label for="baseline-consumption">{$LL.vehicleModal.baselineConsumptionLabel()}</label>
					<input
						type="number"
						id="baseline-consumption"
						bind:value={baselineConsumptionKwh}
						step="0.1"
						min="0"
						placeholder={$LL.vehicleModal.baselineConsumptionPlaceholder()}
					/>
				</div>

				<div class="form-group">
					<label for="initial-battery">{$LL.vehicleModal.initialBatteryLabel()}</label>
					<input
						type="number"
						id="initial-battery"
						bind:value={initialBatteryPercent}
						step="1"
						min="0"
						max="100"
						placeholder={$LL.vehicleModal.initialBatteryPlaceholder()}
					/>
				</div>
			{/if}
		</div>

		<div class="modal-footer">
			<button class="button button-secondary" on:click={onClose}>{$LL.common.cancel()}</button>
			<button class="button button-primary" on:click={handleSave}>{$LL.common.save()}</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-content {
		background: var(--bg-surface);
		border-radius: 8px;
		width: 90%;
		max-width: 500px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 4px 12px var(--shadow-default);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-default);
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.close-button {
		background: none;
		border: none;
		font-size: 2rem;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		line-height: 1;
	}

	.close-button:hover {
		color: var(--text-primary);
	}

	.modal-body {
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.section-header {
		font-weight: 600;
		color: var(--text-primary);
		font-size: 0.9rem;
		margin-top: 0.5rem;
		padding-bottom: 0.25rem;
		border-bottom: 1px solid var(--border-default);
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.form-group label {
		font-weight: 500;
		color: var(--text-primary);
		font-size: 0.875rem;
	}

	.form-group input,
	.form-group select {
		padding: 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 1rem;
		font-family: inherit;
		background-color: var(--bg-surface);
		color: var(--text-primary);
	}

	.form-group input:focus,
	.form-group select:focus {
		outline: none;
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	.form-group select:disabled {
		background-color: var(--bg-surface-alt);
		color: var(--text-muted);
		cursor: not-allowed;
	}

	.hint {
		font-size: 0.75rem;
		color: var(--text-secondary);
		font-style: italic;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid var(--border-default);
	}

	.button {
		padding: 0.75rem 1.5rem;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 1rem;
	}

	.button-secondary {
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
	}

	.button-secondary:hover {
		background-color: var(--btn-secondary-hover);
	}

	.button-primary {
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
	}

	.button-primary:hover {
		background-color: var(--btn-active-primary-hover);
	}
</style>
