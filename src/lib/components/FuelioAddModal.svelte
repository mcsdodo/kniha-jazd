<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import type { CascadePlan, FuelioAddPreview, FuelioRow, Trip } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { toast } from '$lib/stores/toast';
	import OdometerCascadeModal from './OdometerCascadeModal.svelte';

	// Task 90: add a logbook trip from a "missing" Fuelio row. The backend
	// computes the values and the place order (ADR-008); this dialog lets the
	// user confirm the two ends and the purpose, and shows the odometer plan.

	interface Props {
		row: FuelioRow;
		vehicleId: string;
		year: number;
		onDone: () => void;
		onCancel: () => void;
	}
	let { row, vehicleId, year, onDone, onCancel }: Props = $props();

	let preview = $state<FuelioAddPreview | null>(null);
	let originId = $state('');
	let destinationId = $state('');
	let purpose = $state('');
	let withRoute = $state(true);
	let busy = $state(false);
	let error = $state<string | null>(null);
	let plan = $state<CascadePlan | null>(null);
	let yearTrips = $state<Trip[]>([]);

	onMount(async () => {
		try {
			preview = await api.getFuelioAddPreview(row.driveIds);
			// The backend sorts nearest first: preselect the nearest place.
			originId = preview.origin[0]?.id ?? '';
			destinationId = preview.destination[0]?.id ?? '';
		} catch (e) {
			error = String(e);
		}
	});

	// Display formatting of backend values.
	function dt(s: string | null): string {
		if (!s) return '-';
		const [date, time] = s.split('T');
		const [y, m, d] = date.split('-');
		return `${Number(d)}.${Number(m)}.${y} ${time.slice(0, 5)}`;
	}
	function km(m: number): string {
		return (m / 1000).toFixed(m < 10000 ? 2 : 0);
	}

	function call(dryRun: boolean) {
		return api.addFuelioTrip(
			vehicleId,
			row.driveIds,
			originId,
			destinationId,
			purpose,
			withRoute,
			dryRun
		);
	}

	async function proceed() {
		busy = true;
		error = null;
		try {
			const dry = await call(true);
			if (dry.plan.changes.length > 0 || dry.plan.nextYearChainBreaks) {
				// Later trips move: confirm in the dialog the logbook grid uses.
				yearTrips = await api.getTripsForYear(vehicleId, year);
				plan = dry.plan;
			} else {
				await commit();
			}
		} catch (e) {
			error = String(e);
		} finally {
			busy = false;
		}
	}

	async function commit() {
		busy = true;
		error = null;
		try {
			const r = await call(false);
			plan = null;
			// The trip exists even when its route failed: close and reload, so a
			// second click cannot add it again.
			if (r.routeError) {
				toast.error($LL.fuelio.add.routeFailed({ error: r.routeError }));
			} else {
				toast.success($LL.fuelio.add.done());
			}
			onDone();
		} catch (e) {
			plan = null;
			error = String(e);
		} finally {
			busy = false;
		}
	}
</script>

{#if plan}
	<OdometerCascadeModal
		kind="insert"
		{plan}
		trips={yearTrips}
		oldDistanceKm={0}
		onConfirm={commit}
		onCancel={() => (plan = null)}
	/>
{:else}
	<div
		class="modal-overlay"
		onclick={onCancel}
		onkeydown={(e) => e.key === 'Escape' && onCancel()}
		role="button"
		tabindex="0"
	>
		<div
			class="modal"
			onclick={(e) => e.stopPropagation()}
			onkeydown={() => {}}
			role="dialog"
			aria-modal="true"
			tabindex="-1"
			data-testid="fuelio-add-modal"
		>
			<h2>{$LL.fuelio.add.title()}</h2>
			<p class="muted">{$LL.fuelio.add.intro()}</p>

			{#if !preview && !error}
				<p>{$LL.fuelio.add.loading()}</p>
			{:else if preview}
				<table>
					<tbody>
						<tr>
							<td>{$LL.fuelio.add.start()}</td>
							<td><strong>{dt(preview.start)}</strong></td>
						</tr>
						<tr>
							<td>{$LL.fuelio.add.end()}</td>
							<td><strong>{dt(preview.end)}</strong></td>
						</tr>
						<tr>
							<td>{$LL.fuelio.add.distance()}</td>
							<td>
								<strong>{preview.distanceKm} km</strong>
								<span class="muted small"
									>{$LL.fuelio.add.distanceNote({ gps: preview.gpsKm.toFixed(1) })}</span
								>
							</td>
						</tr>
						<tr>
							<td><label for="fuelio-add-origin">{$LL.fuelio.add.origin()}</label></td>
							<td>
								<select id="fuelio-add-origin" bind:value={originId} data-testid="fuelio-add-origin">
									{#each preview.origin as p (p.id)}
										<option value={p.id}
											>{p.name} - {$LL.fuelio.add.distanceFrom({ km: km(p.distanceM) })}</option
										>
									{/each}
								</select>
							</td>
						</tr>
						<tr>
							<td><label for="fuelio-add-destination">{$LL.fuelio.add.destination()}</label></td>
							<td>
								<select
									id="fuelio-add-destination"
									bind:value={destinationId}
									data-testid="fuelio-add-destination"
								>
									{#each preview.destination as p (p.id)}
										<option value={p.id}
											>{p.name} - {$LL.fuelio.add.distanceFrom({ km: km(p.distanceM) })}</option
										>
									{/each}
								</select>
							</td>
						</tr>
						<tr>
							<td><label for="fuelio-add-purpose">{$LL.fuelio.add.purpose()}</label></td>
							<td>
								<input
									id="fuelio-add-purpose"
									class="text-input"
									type="text"
									bind:value={purpose}
									data-testid="fuelio-add-purpose"
								/>
							</td>
						</tr>
						<tr>
							<td colspan="2">
								<label><input type="checkbox" bind:checked={withRoute} /> {$LL.fuelio.add.route()}</label>
							</td>
						</tr>
					</tbody>
				</table>
				{#if preview.origin.length === 0}
					<p class="error">{$LL.fuelio.add.noPlaces()}</p>
				{/if}
			{/if}

			{#if error}
				<p class="error" data-testid="fuelio-add-error">{error}</p>
			{/if}

			<div class="modal-actions">
				<button class="button-small" onclick={onCancel}>{$LL.fuelio.add.cancel()}</button>
				<button
					class="button-small primary"
					disabled={!preview || !originId || !destinationId || busy}
					onclick={proceed}
					data-testid="fuelio-add-continue">{$LL.fuelio.add.continue()}</button
				>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}
	.modal {
		background: var(--bg-surface);
		padding: 1.5rem;
		border-radius: 8px;
		max-width: 620px;
		width: 92%;
		color: var(--text-primary);
	}
	h2 {
		margin: 0 0 0.5rem;
		font-size: 1.25rem;
	}
	p {
		margin: 0.4rem 0;
	}
	.muted {
		color: var(--text-secondary);
	}
	.small {
		font-size: 0.8rem;
		margin-left: 0.4rem;
	}
	.error {
		color: var(--accent-danger);
		font-size: 0.875rem;
	}
	table {
		width: 100%;
		border-collapse: collapse;
		margin: 0.75rem 0 1rem;
		font-size: 0.875rem;
	}
	td {
		padding: 0.35rem 0.4rem;
		border-bottom: 1px solid var(--border-default);
		vertical-align: middle;
	}
	td:first-child {
		white-space: nowrap;
		width: 1%;
	}
	select,
	.text-input {
		box-sizing: border-box;
		width: 100%;
		padding: 0.35rem 0.5rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		background: var(--input-bg);
		color: var(--text-primary);
		font-size: 0.875rem;
	}
	.modal-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}
	.button-small {
		padding: 0.5rem 1rem;
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
	}
	.button-small:hover {
		background-color: var(--btn-secondary-hover);
	}
	.button-small.primary {
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
	}
	.button-small:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
