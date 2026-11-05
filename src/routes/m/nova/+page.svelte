<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import * as api from '$lib/api';
	import type { CascadePlan, FuelioAddEnd, ManualTripEstimate, Trip } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { toast } from '$lib/stores/toast';
	import OdometerCascadeModal from '$lib/components/OdometerCascadeModal.svelte';
	import Autocomplete from '$lib/components/Autocomplete.svelte';
	import MobilePlacePicker, { type PlacePickerOption } from '$lib/components/mobile/MobilePlacePicker.svelte';

	// Task 93: add a trip by hand on the phone. The end is now, the destination
	// comes from the phone GPS, the user types the km. The backend computes the
	// start from the route drive time (ADR-008). For a new place there is no
	// route before the save, so the user types the start.

	// "Now" as a datetime-local value: local time, "YYYY-MM-DDTHH:MM".
	function nowLocal(): string {
		const d = new Date();
		const p = (n: number) => String(n).padStart(2, '0');
		return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
	}

	let end = $state(nowLocal());
	let start = $state('');
	// True while `start` holds the backend estimate, not a typed value.
	let startFromEstimate = $state(false);
	let km = $state<number | null>(null);
	let purpose = $state('');
	let purposeSuggestions = $state<string[]>([]);
	let allPlaces = $state<PlacePickerOption[]>([]);
	let placesError = $state<string | null>(null);

	let origin = $state<FuelioAddEnd>({ placeId: null, newPlaceName: null });
	let originPos = $state<{ lat: number; lon: number } | null>(null);
	let destination = $state<FuelioAddEnd>({ placeId: null, newPlaceName: null });
	let destinationPos = $state<{ lat: number; lon: number } | null>(null);

	let estimate = $state<ManualTripEstimate | null>(null);
	let estimating = $state(false);

	let busy = $state(false);
	let formError = $state<string | null>(null);
	let plan = $state<CascadePlan | null>(null);
	let yearTrips = $state<Trip[]>([]);
	// Places this page created on an earlier save (key: lower-case name). A save
	// that the user cancels in the odometer dialog must not create them again.
	const created = new Map<string, string>();

	onMount(async () => {
		try {
			const places = await api.listPlaces();
			allPlaces = places
				.map((p) => ({ id: p.id, name: p.name, distanceM: null }))
				.sort((a, b) => a.name.localeCompare(b.name, 'sk'));
		} catch (e) {
			placesError = String(e);
		}
	});

	// The layout loads the vehicle after this page mounts.
	$effect(() => {
		const vehicle = $activeVehicleStore;
		if (!vehicle) return;
		api.getPurposes(vehicle.id).then(
			(p) => (purposeSuggestions = p),
			(e) => console.error('Failed to load purposes:', e)
		);
	});

	// One end for the route estimate: the existing place, or the GPS point of a
	// new place. Null when the end has neither yet.
	function estimateEnd(
		v: FuelioAddEnd,
		pos: { lat: number; lon: number } | null
	): api.EstimateEnd | null {
		if (v.placeId) return { placeId: v.placeId };
		if (v.newPlaceName !== null && pos) return { lat: pos.lat, lon: pos.lon };
		return null;
	}

	// The start from the route, when both ends are known (a place or a GPS
	// point). Only the newest request may set it: a datetime-local input fires
	// per field.
	let estimateSeq = 0;
	$effect(() => {
		const o = estimateEnd(origin, originPos);
		const d = estimateEnd(destination, destinationPos);
		const e = end;
		const seq = ++estimateSeq;
		if (!o || !d || !e) {
			untrack(() => {
				estimate = null;
				estimating = false;
				dropEstimatedStart();
			});
			return;
		}
		untrack(() => (estimating = true));
		api.getManualTripEstimate(o, d, e).then(
			(r) => {
				if (seq !== estimateSeq) return;
				estimating = false;
				estimate = r;
				if (r.startDatetime) {
					start = r.startDatetime;
					startFromEstimate = true;
				} else {
					dropEstimatedStart();
				}
			},
			(err) => {
				if (seq !== estimateSeq) return;
				estimating = false;
				estimate = { routeKm: null, durationMin: null, startDatetime: null, error: String(err) };
				dropEstimatedStart();
			}
		);
	});

	// An old estimate does not fit new ends: clear it, the user types the start.
	// A start the user typed stays.
	function dropEstimatedStart() {
		if (startFromEstimate) {
			start = '';
			startFromEstimate = false;
		}
	}

	let hasNewEnd = $derived(origin.newPlaceName !== null || destination.newPlaceName !== null);

	function chosen(v: FuelioAddEnd): boolean {
		return !!v.placeId || !!v.newPlaceName?.trim();
	}

	function validate(): string | null {
		if (!end) return $LL.mobile.add.missingEnd();
		if (!chosen(destination)) return $LL.mobile.add.missingDestination();
		if (!chosen(origin)) return $LL.mobile.add.missingOrigin();
		if ((!origin.placeId && !originPos) || (!destination.placeId && !destinationPos)) {
			return $LL.mobile.add.noPosition();
		}
		if (!start) return $LL.mobile.add.missingStart();
		if (km === null || km === undefined || Number.isNaN(km)) return $LL.mobile.add.missingDistance();
		return null;
	}

	// An existing place id, or the id of the new place (created once).
	async function placeId(v: FuelioAddEnd, pos: { lat: number; lon: number } | null): Promise<string> {
		if (v.placeId) return v.placeId;
		const name = v.newPlaceName!.trim();
		const key = name.toLowerCase();
		const known = created.get(key);
		if (known) return known;
		const place = await api.createPlace(name, pos!.lat, pos!.lon, 'manual');
		created.set(key, place.id);
		return place.id;
	}

	let ids: { origin: string; destination: string } | null = null;

	function call(dryRun: boolean) {
		return api.createTripCascade(
			$activeVehicleStore!.id,
			start,
			end,
			ids!.origin,
			ids!.destination,
			km!,
			purpose,
			null,
			null,
			null,
			null,
			null,
			null,
			null,
			null,
			null,
			dryRun
		);
	}

	async function save() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) return;
		const invalid = validate();
		if (invalid) {
			formError = invalid;
			return;
		}
		busy = true;
		formError = null;
		try {
			// Destination first: it is the GPS end and the usual new place.
			const destinationId = await placeId(destination, destinationPos);
			const originId = await placeId(origin, originPos);
			ids = { origin: originId, destination: destinationId };
			const dry = await call(true);
			if (dry.plan.changes.length > 0 || dry.plan.nextYearChainBreaks) {
				yearTrips = await api.getTripsForYear(vehicle.id, Number(end.slice(0, 4)));
				plan = dry.plan;
			} else {
				await commit();
			}
		} catch (e) {
			formError = String(e);
		} finally {
			busy = false;
		}
	}

	async function commit() {
		busy = true;
		formError = null;
		try {
			await call(false);
			plan = null;
			toast.success($LL.mobile.add.done());
			await goto('/m');
		} catch (e) {
			plan = null;
			formError = String(e);
		} finally {
			busy = false;
		}
	}
</script>

<div class="page" data-testid="m-nova">
	<header class="bar">
		<a class="back" href="/m" data-testid="m-nova-back">{$LL.mobile.add.back()}</a>
		<h1>{$LL.mobile.add.title()}</h1>
	</header>

	{#if !$activeVehicleStore}
		<p class="muted">{$LL.mobile.add.noVehicle()}</p>
	{:else}
		<section class="form">
			<div class="field-group">
				<label for="m-nova-end">{$LL.mobile.add.end()}</label>
				<input id="m-nova-end" type="datetime-local" class="field" bind:value={end} data-testid="m-nova-end" />
			</div>

			{#if placesError}
				<p class="error" role="alert">{$LL.mobile.add.loadError({ error: placesError })}</p>
			{/if}

			<MobilePlacePicker
				testId="m-nova-destination"
				label={$LL.mobile.add.destination()}
				options={allPlaces}
				locate
				autoLocate
				newHint={$LL.mobile.add.picker.pinGps()}
				bind:value={destination}
				bind:position={destinationPos}
			/>
			<MobilePlacePicker
				testId="m-nova-origin"
				label={$LL.mobile.add.origin()}
				options={allPlaces}
				locate
				newHint={$LL.mobile.add.picker.pinGps()}
				bind:value={origin}
				bind:position={originPos}
			/>

			<div class="field-group">
				<label for="m-nova-km">{$LL.mobile.add.distance()}</label>
				<input
					id="m-nova-km"
					type="number"
					inputmode="decimal"
					min="0"
					step="any"
					class="field"
					bind:value={km}
					data-testid="m-nova-km"
				/>
				{#if estimate?.routeKm != null}
					<p class="muted small" data-testid="m-nova-route-km">
						{$LL.mobile.add.routeHint({ km: estimate.routeKm.toFixed(1) })}
					</p>
				{/if}
			</div>

			<div class="field-group">
				<label for="m-nova-start">{$LL.mobile.add.start()}</label>
				<input
					id="m-nova-start"
					type="datetime-local"
					class="field"
					bind:value={start}
					oninput={() => (startFromEstimate = false)}
					data-testid="m-nova-start"
				/>
				<p class="muted small" data-testid="m-nova-start-hint">
					{#if !estimateEnd(origin, originPos) || !estimateEnd(destination, destinationPos)}
						{hasNewEnd ? $LL.mobile.add.startNewPlace() : $LL.mobile.add.startNeedsPlaces()}
					{:else if estimating}
						{$LL.mobile.add.estimating()}
					{:else if estimate?.error}
						<span class="error">{$LL.mobile.add.estimateFailed({ error: estimate.error })}</span>
					{:else if estimate?.durationMin != null && startFromEstimate}
						{$LL.mobile.add.startFromRoute({ min: estimate.durationMin })}
					{/if}
				</p>
			</div>

			<div class="field-group purpose">
				<label for="m-nova-purpose">{$LL.mobile.add.purpose()}</label>
				<Autocomplete
					id="m-nova-purpose"
					bind:value={purpose}
					suggestions={purposeSuggestions}
					onSelect={(v) => (purpose = v)}
					testId="m-nova-purpose"
				/>
			</div>

			{#if formError}
				<p class="error" role="alert" data-testid="m-nova-error">{formError}</p>
			{/if}

			<button type="button" class="primary" disabled={busy} onclick={save} data-testid="m-nova-save"
				>{busy ? $LL.mobile.add.saving() : $LL.mobile.add.save()}</button
			>
		</section>
	{/if}
</div>

{#if plan}
	<OdometerCascadeModal
		kind="insert"
		{plan}
		trips={yearTrips}
		oldDistanceKm={0}
		onConfirm={commit}
		onCancel={() => (plan = null)}
	/>
{/if}

<style>
	.page {
		box-sizing: border-box;
		max-width: 640px;
		margin: 0 auto;
		padding: 0 16px 32px;
		color: var(--text-primary);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 52px;
		border-bottom: 1px solid var(--border-default);
		margin-bottom: 0.75rem;
	}
	.bar h1 {
		margin: 0;
		font-size: 1.15rem;
	}
	.back {
		display: inline-flex;
		align-items: center;
		min-height: 44px;
		padding: 0 0.5rem 0 0;
		color: var(--accent-primary);
		font-size: 1rem;
		text-decoration: none;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.field-group {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.field-group label {
		font-weight: 600;
		font-size: 0.95rem;
	}
	.field,
	.purpose :global(input) {
		box-sizing: border-box;
		width: 100%;
		min-height: 44px;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 6px;
		background: var(--input-bg);
		color: var(--text-primary);
		/* 16px stops the iOS zoom on focus */
		font-size: 1rem;
	}
	.muted {
		color: var(--text-secondary);
	}
	.small {
		font-size: 0.85rem;
		margin: 0;
	}
	.error {
		color: var(--accent-danger);
		font-size: 0.9rem;
	}
	.primary {
		min-height: 48px;
		border: none;
		border-radius: 8px;
		background: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		font-size: 1.05rem;
		font-weight: 600;
		cursor: pointer;
	}
	.primary:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
