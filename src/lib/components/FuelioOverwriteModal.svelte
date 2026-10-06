<script lang="ts">
	import { untrack } from 'svelte';
	import * as api from '$lib/api';
	import type { DistanceWriteback, FuelioFields, FuelioRow, Trip } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { toast } from '$lib/stores/toast';
	import OdometerCascadeModal from './OdometerCascadeModal.svelte';

	// Task 90: overwrite a matched trip with the GPS values of its Fuelio
	// drives. The backend computes and writes everything (ADR-008); this
	// dialog only collects the field choice and shows the dry-run plan.

	interface Props {
		row: FuelioRow;
		vehicleId: string;
		year: number;
		onDone: () => void;
		onCancel: () => void;
	}
	let { row, vehicleId, year, onDone, onCancel }: Props = $props();

	let partial = $derived(row.flags.includes('partialGps'));
	let loose = $derived(row.flags.includes('looseMatch'));
	// A complete match: everything on. A partial or loose one: the user opts in.
	// Read once: the defaults must not reset the user's choice later.
	const preselect = untrack(() => !partial && !loose);
	let fields = $state<FuelioFields>({
		start: preselect,
		end: preselect,
		distance: preselect,
		route: preselect
	});
	let busy = $state(false);
	let error = $state<string | null>(null);
	let writeback = $state<DistanceWriteback | null>(null);
	let yearTrips = $state<Trip[]>([]);

	let anySelected = $derived(fields.start || fields.end || fields.distance || fields.route);

	// Display formatting of backend values.
	function dt(s: string | null): string {
		if (!s) return '-';
		const [date, time] = s.split('T');
		const [y, m, d] = date.split('-');
		return `${Number(d)}.${Number(m)}.${y} ${time.slice(0, 5)}`;
	}

	async function proceed() {
		if (!row.tripId) return;
		busy = true;
		error = null;
		try {
			const dry = await api.applyFuelioToTrip(row.tripId, row.driveIds, fields, true);
			if (dry.writeback?.changesTrip) {
				// The odometer chain or the margin moves: confirm in the same
				// dialog that /mapa uses for a distance write-back.
				yearTrips = await api.getTripsForYear(vehicleId, year);
				writeback = dry.writeback;
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
		if (!row.tripId) return;
		busy = true;
		error = null;
		try {
			const r = await api.applyFuelioToTrip(row.tripId, row.driveIds, fields, false);
			writeback = null;
			const changed =
				r.routeWritten ||
				r.startBefore !== r.startAfter ||
				r.endBefore !== r.endAfter ||
				r.distanceBefore !== r.distanceAfter;
			if (changed) {
				toast.success($LL.fuelio.overwrite.done());
			} else {
				toast.info($LL.fuelio.overwrite.nothing());
			}
			onDone();
		} catch (e) {
			writeback = null;
			error = String(e);
		} finally {
			busy = false;
		}
	}
</script>

{#if writeback}
	<OdometerCascadeModal
		kind="writeback"
		plan={writeback.plan}
		margin={writeback.margin}
		trips={yearTrips}
		oldDistanceKm={writeback.distanceBefore}
		onConfirm={commit}
		onCancel={() => (writeback = null)}
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
			data-testid="fuelio-overwrite-modal"
		>
			<h2>{$LL.fuelio.overwrite.title()}</h2>
			<p class="trip">
				{dt(row.tripStart)} - {row.origin} &rarr; {row.destination}
			</p>
			<p class="muted">{$LL.fuelio.overwrite.intro()}</p>
			{#if partial}
				<p class="warn">{$LL.fuelio.overwrite.partialWarning()}</p>
			{/if}
			{#if loose}
				<p class="warn">{$LL.fuelio.overwrite.looseWarning()}</p>
			{/if}

			<table>
				<tbody>
					<tr>
						<td><label><input type="checkbox" bind:checked={fields.start} /> {$LL.fuelio.overwrite.start()}</label></td>
						<td>{dt(row.tripStart)}</td>
						<td>&rarr;</td>
						<td><strong>{dt(row.gpsStart)}</strong></td>
					</tr>
					<tr>
						<td><label><input type="checkbox" bind:checked={fields.end} /> {$LL.fuelio.overwrite.end()}</label></td>
						<td>{dt(row.tripEnd)}</td>
						<td>&rarr;</td>
						<td><strong>{dt(row.gpsEnd)}</strong></td>
					</tr>
					<tr>
						<td>
							<label><input type="checkbox" bind:checked={fields.distance} /> {$LL.fuelio.overwrite.distance()}</label>
						</td>
						<td>{row.tripKm ?? '-'} km</td>
						<td>&rarr;</td>
						<td>
							<strong>{row.gpsKm?.toFixed(1)} km</strong>
							<div class="muted small">{$LL.fuelio.overwrite.distanceNote()}</div>
						</td>
					</tr>
					<tr>
						<td><label><input type="checkbox" bind:checked={fields.route} /> {$LL.fuelio.overwrite.route()}</label></td>
						<td colspan="3" class="muted small">{$LL.fuelio.overwrite.routeNew()}</td>
					</tr>
				</tbody>
			</table>

			{#if error}
				<p class="error" data-testid="fuelio-overwrite-error">{error}</p>
			{/if}

			<div class="modal-actions">
				<button class="button-small" onclick={onCancel}>{$LL.fuelio.overwrite.cancel()}</button>
				<button
					class="button-small primary"
					disabled={!anySelected || busy}
					onclick={proceed}
					data-testid="fuelio-overwrite-continue">{$LL.fuelio.overwrite.continue()}</button
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
		max-width: 560px;
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
	.trip {
		font-weight: 500;
	}
	.muted {
		color: var(--text-secondary);
	}
	.small {
		font-size: 0.8rem;
	}
	.warn {
		background: var(--warning-bg);
		color: var(--warning-color);
		padding: 0.4rem 0.6rem;
		border-radius: 4px;
		font-size: 0.875rem;
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
		vertical-align: top;
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
