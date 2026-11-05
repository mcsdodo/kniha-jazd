<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import * as api from '$lib/api';
	import type {
		CascadePlan,
		FuelioAddEnd,
		ManualRoute,
		ManualTripEstimate,
		RouteProviderKind,
		Trip
	} from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { toast } from '$lib/stores/toast';
	import OdometerCascadeModal from '$lib/components/OdometerCascadeModal.svelte';
	import Autocomplete from '$lib/components/Autocomplete.svelte';
	import RoutePickerMap from '$lib/components/mobile/RoutePickerMap.svelte';
	import MobilePlacePicker, {
		type PickedPosition,
		type PlacePickerOption
	} from '$lib/components/mobile/MobilePlacePicker.svelte';

	// Task 93: add a trip by hand on the phone. Top to bottom: Odkiaľ, Kam,
	// then the times and the km. Each end is an existing place or a new place
	// with a position (phone GPS or address search). The end time starts at
	// now and the user can change it. With both ends known, the backend
	// computes the route: it fills the km and the start (ADR-008). The user
	// can change both.
	//
	// Task 94: the backend sends every route alternative. The map and the
	// chips show them; the chosen one fills the km and the start, and is
	// saved as the trip's route. With 2+ routing services the user picks one.

	// A datetime-local value: local time, "YYYY-MM-DDTHH:MM".
	function nowLocal(): string {
		const d = new Date();
		const p = (n: number) => String(n).padStart(2, '0');
		return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}T${p(d.getHours())}:${p(d.getMinutes())}`;
	}

	let end = $state(nowLocal());
	let start = $state('');
	// True while `start` holds the route estimate, not a typed value.
	let startFromEstimate = $state(false);
	let km = $state<number | null>(null);
	// True while `km` holds the route km, not a typed value.
	let kmFromEstimate = $state(false);
	let purpose = $state('');
	let purposeSuggestions = $state<string[]>([]);
	let allPlaces = $state<PlacePickerOption[]>([]);
	let placesError = $state<string | null>(null);

	let origin = $state<FuelioAddEnd>({ placeId: null, newPlaceName: null });
	let originPos = $state<PickedPosition | null>(null);
	let destination = $state<FuelioAddEnd>({ placeId: null, newPlaceName: null });
	let destinationPos = $state<PickedPosition | null>(null);
	// The origin the backend suggested: the destination of the last trip.
	let defaultOriginId = $state<string | null>(null);

	let estimate = $state<ManualTripEstimate | null>(null);
	let estimating = $state(false);
	// The index of the chosen alternative in estimate.routes.
	let activeRoute = $state(0);
	// The routing services the server offers, and the one the next request
	// uses. Null until get_route_providers answers: then the server default.
	let providers = $state<RouteProviderKind[]>([]);
	let provider = $state<RouteProviderKind | null>(null);
	let chosenRoute = $derived<ManualRoute | null>(estimate?.routes?.[activeRoute] ?? null);

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
		try {
			const info = await api.getRouteProviders();
			providers = info.available;
			provider ??= info.default;
		} catch (e) {
			console.error('Failed to load route providers:', e);
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

	// The origin default follows the end time. It replaces only an empty
	// origin or the old default, never a place the user chose.
	let defaultsSeq = 0;
	$effect(() => {
		const vehicle = $activeVehicleStore;
		const e = end;
		if (!vehicle || !e) return;
		const seq = ++defaultsSeq;
		api.getManualTripDefaults(vehicle.id, e).then(
			(d) => {
				if (seq !== defaultsSeq) return;
				const untouched =
					(origin.placeId === null && origin.newPlaceName === null) ||
					(origin.placeId !== null && origin.placeId === defaultOriginId);
				defaultOriginId = d.originPlaceId;
				if (untouched && d.originPlaceId) {
					origin = { placeId: d.originPlaceId, newPlaceName: null };
				}
			},
			(err) => console.error('Failed to load the trip defaults:', err)
		);
	});

	// One end for the route estimate: the existing place, or the position of a
	// new place. Null when the end has neither yet.
	function estimateEnd(v: FuelioAddEnd, pos: PickedPosition | null): api.EstimateEnd | null {
		if (v.placeId) return { placeId: v.placeId };
		if (v.newPlaceName !== null && pos) return { lat: pos.lat, lon: pos.lon };
		return null;
	}

	let originEnd = $derived(estimateEnd(origin, originPos));
	let destinationEnd = $derived(estimateEnd(destination, destinationPos));

	// The route, when both ends are known. Only the newest request may set the
	// values: a datetime-local input fires per field.
	let estimateSeq = 0;
	// The ends and the service of the routes on screen. A new end time keeps
	// the chosen alternative; new ends or a new service select route 1.
	let routesKey = '';
	$effect(() => {
		const o = originEnd;
		const d = destinationEnd;
		const e = end;
		const p = provider;
		const seq = ++estimateSeq;
		if (!o || !d || !e) {
			untrack(() => {
				estimate = null;
				estimating = false;
				dropEstimated();
			});
			return;
		}
		untrack(() => (estimating = true));
		const key = JSON.stringify([o, d, p]);
		api.getManualTripEstimate(o, d, e, p).then(
			(r) => {
				if (seq !== estimateSeq) return;
				estimating = false;
				estimate = r;
				const keep = key === routesKey && activeRoute < (r.routes?.length ?? 0);
				routesKey = key;
				if (!keep) activeRoute = 0;
				dropEstimated();
				fillFromRoute();
			},
			(err) => {
				if (seq !== estimateSeq) return;
				estimating = false;
				estimate = {
					routeKm: null,
					distanceKm: null,
					durationMin: null,
					startDatetime: null,
					error: String(err),
					provider: null,
					routes: []
				};
				dropEstimated();
			}
		);
	});

	// The chosen route fills the start and the km. A typed value stays.
	function fillFromRoute() {
		const r = chosenRoute;
		const startValue = r?.startDatetime ?? estimate?.startDatetime ?? null;
		const kmValue = r?.distanceKm ?? estimate?.distanceKm ?? null;
		if (startValue && (startFromEstimate || !start)) {
			start = startValue;
			startFromEstimate = true;
		}
		if (kmValue != null && (kmFromEstimate || km === null)) {
			km = kmValue;
			kmFromEstimate = true;
		}
	}

	function selectRoute(index: number) {
		if (index === activeRoute) return;
		activeRoute = index;
		fillFromRoute();
	}

	// An old estimate does not fit new ends: clear it. A typed value stays.
	function dropEstimated() {
		if (startFromEstimate) {
			start = '';
			startFromEstimate = false;
		}
		if (kmFromEstimate) {
			km = null;
			kmFromEstimate = false;
		}
	}

	function chosen(v: FuelioAddEnd): boolean {
		return !!v.placeId || !!v.newPlaceName?.trim();
	}

	function validate(): string | null {
		if (!chosen(origin)) return $LL.mobile.add.missingOrigin();
		if (!chosen(destination)) return $LL.mobile.add.missingDestination();
		if ((!origin.placeId && !originPos) || (!destination.placeId && !destinationPos)) {
			return $LL.mobile.add.noPosition();
		}
		if (!end) return $LL.mobile.add.missingEnd();
		if (!start) return $LL.mobile.add.missingStart();
		if (km === null || km === undefined || Number.isNaN(km)) return $LL.mobile.add.missingDistance();
		return null;
	}

	// An existing place id, or the id of the new place (created once).
	async function placeId(v: FuelioAddEnd, pos: PickedPosition | null): Promise<string> {
		if (v.placeId) return v.placeId;
		const name = v.newPlaceName!.trim();
		const key = name.toLowerCase();
		const known = created.get(key);
		if (known) return known;
		const place = await api.createPlace(name, pos!.lat, pos!.lon, pos!.source);
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
			const originId = await placeId(origin, originPos);
			const destinationId = await placeId(destination, destinationPos);
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
			const result = await call(false);
			plan = null;
			const route = chosenRoute;
			if (result.trip && route) {
				try {
					await api.saveManualTripRoute(
						result.trip.id,
						route.polyline,
						route.roadKm,
						estimate?.provider ?? null
					);
				} catch (e) {
					// The trip is in the book; only its map is missing.
					toast.error($LL.mobile.add.routeNotSaved({ error: String(e) }));
				}
			}
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
			{#if placesError}
				<p class="error" role="alert">{$LL.mobile.add.loadError({ error: placesError })}</p>
			{/if}

			<MobilePlacePicker
				testId="m-nova-origin"
				label={$LL.mobile.add.origin()}
				options={allPlaces}
				locate
				selectNote={origin.placeId !== null && origin.placeId === defaultOriginId
					? $LL.mobile.add.lastDestination()
					: ''}
				bind:value={origin}
				bind:position={originPos}
			/>
			<MobilePlacePicker
				testId="m-nova-destination"
				label={$LL.mobile.add.destination()}
				options={allPlaces}
				locate
				autoLocate
				bind:value={destination}
				bind:position={destinationPos}
			/>

			{#if providers.length > 1}
				<div class="field-group">
					<div class="seg" role="group" data-testid="m-nova-provider">
						{#each providers as value (value)}
							<button
								type="button"
								class:on={(provider ?? providers[0]) === value}
								aria-pressed={(provider ?? providers[0]) === value}
								onclick={() => (provider = value)}
								data-testid="m-nova-provider-{value}">{$LL.mobile.add.providerShort[value]()}</button
							>
						{/each}
					</div>
				</div>
			{/if}

			{#if estimate && estimate.routes.length > 0 && originEnd && destinationEnd}
				<RoutePickerMap routes={estimate.routes} active={activeRoute} onSelect={selectRoute} />
				{#if estimate.routes.length > 1}
					<div class="field-group">
						<span class="label">{$LL.mobile.add.alternatives()}</span>
						<div class="chips" data-testid="m-nova-alternatives">
							{#each estimate.routes as route, i (i)}
								<button
									type="button"
									class="chip"
									class:on={i === activeRoute}
									aria-pressed={i === activeRoute}
									onclick={() => selectRoute(i)}
									data-testid="m-nova-alternative-{i + 1}"
									>{$LL.mobile.add.alternative({
										n: i + 1,
										km: route.routeKm.toFixed(1),
										min: route.durationMin
									})}</button
								>
							{/each}
						</div>
					</div>
				{/if}
			{/if}

			<div class="box" data-testid="m-nova-times">
				<p class="muted small" data-testid="m-nova-route">
					{#if !originEnd || !destinationEnd}
						{$LL.mobile.add.startNeedsPlaces()}
					{:else if estimating}
						{$LL.mobile.add.estimating()}
					{:else if estimate?.error}
						<span class="error">{$LL.mobile.add.estimateFailed({ error: estimate.error })}</span>
					{:else if chosenRoute}
						{(estimate?.routes.length ?? 0) > 1
							? $LL.mobile.add.routeNumber({
									n: activeRoute + 1,
									km: chosenRoute.routeKm.toFixed(1),
									min: chosenRoute.durationMin
								})
							: $LL.mobile.add.routeSummary({
									km: chosenRoute.routeKm.toFixed(1),
									min: chosenRoute.durationMin
								})}
					{:else if estimate?.routeKm != null && estimate.durationMin != null}
						{$LL.mobile.add.routeSummary({
							km: estimate.routeKm.toFixed(1),
							min: estimate.durationMin
						})}
					{/if}
				</p>

				<div class="field-group">
					<label for="m-nova-end">{$LL.mobile.add.end()}</label>
					<div class="row">
						<input
							id="m-nova-end"
							type="datetime-local"
							class="field"
							bind:value={end}
							data-testid="m-nova-end"
						/>
						<button
							type="button"
							class="secondary"
							onclick={() => (end = nowLocal())}
							data-testid="m-nova-end-now">{$LL.mobile.add.now()}</button
						>
					</div>
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
					{#if startFromEstimate && (chosenRoute?.durationMin ?? estimate?.durationMin) != null}
						<p class="muted small" data-testid="m-nova-start-hint">
							{$LL.mobile.add.startFromRoute({
								min: (chosenRoute?.durationMin ?? estimate?.durationMin)!
							})}
						</p>
					{/if}
				</div>

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
						oninput={() => (kmFromEstimate = false)}
						data-testid="m-nova-km"
					/>
					{#if kmFromEstimate}
						<p class="muted small" data-testid="m-nova-km-hint">{$LL.mobile.add.kmFromRoute()}</p>
					{/if}
				</div>
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
	.label {
		font-weight: 600;
		font-size: 0.95rem;
	}
	.seg {
		display: flex;
		border: 1px solid var(--border-input);
		border-radius: 6px;
		overflow: hidden;
	}
	.seg button {
		flex: 1;
		min-height: 40px;
		border: none;
		background: var(--btn-secondary-bg);
		color: var(--text-primary);
		font-size: 0.95rem;
		cursor: pointer;
	}
	.seg button + button {
		border-left: 1px solid var(--border-input);
	}
	.seg button.on,
	.chip.on {
		background: color-mix(in srgb, var(--accent-primary) 15%, transparent);
		color: var(--accent-primary);
		font-weight: 700;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.5rem;
	}
	.chip {
		min-height: 40px;
		padding: 0.35rem 0.9rem;
		border: 1px solid var(--border-input);
		border-radius: 999px;
		background: var(--btn-secondary-bg);
		color: var(--text-primary);
		font-size: 0.9rem;
		cursor: pointer;
	}
	.chip.on {
		border-color: var(--accent-primary);
	}
	.box {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 8px;
	}
	.row {
		display: flex;
		gap: 0.5rem;
	}
	.row .field {
		flex: 1;
		min-width: 0;
	}
	.secondary {
		min-height: 44px;
		padding: 0.4rem 0.9rem;
		border: 1px solid var(--border-input);
		border-radius: 6px;
		background: var(--btn-secondary-bg);
		color: var(--text-primary);
		font-size: 0.95rem;
		cursor: pointer;
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
