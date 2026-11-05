<script lang="ts" module>
	import type { PlaceSource } from '$lib/types';

	export interface PlacePickerOption {
		id: string;
		name: string;
		/** Metres from the GPS point. Null: the list has no GPS point (sorted by name). */
		distanceM: number | null;
	}

	/** The position of a new place, and where it came from. */
	export interface PickedPosition {
		lat: number;
		lon: number;
		source: PlaceSource;
	}
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import type { FuelioAddEnd, GeocodeCandidate } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';

	// Task 93: one end of a new trip on the phone. The user picks an existing
	// place, or names a new place. The backend sorts the places and decides the
	// preselect (ADR-008); this component only shows them.
	//
	// Two modes:
	// - Fuelio (no `locate`): the position of a new place is the GPS point of
	//   the drive. The parent knows it; this component asks only the name.
	// - Manual (`locate`): "Moja poloha" reads the phone GPS, sorts the list
	//   and selects a place within the radius. A new place gets its position
	//   from "Moja poloha" or from an address search.

	interface Props {
		/** Used for the element ids and the data-testids: "<testId>-select" and so on. */
		testId: string;
		label: string;
		options: PlacePickerOption[];
		/** The backend preselect (a place within the radius), or null. */
		preselectId?: string | null;
		/** The new-place option is offered. */
		allowNew?: boolean;
		/** Manual mode: the GPS button and the position choice for a new place. */
		locate?: boolean;
		/** Asks the GPS location on mount (needs `locate`). */
		autoLocate?: boolean;
		/** Fuelio mode: where the new place goes, shown under the name input. */
		newHint?: string;
		/** A note under the list, for example why this place is selected. */
		selectNote?: string;
		value: FuelioAddEnd;
		/** Manual mode: the position of a new place. */
		position?: PickedPosition | null;
	}

	let {
		testId,
		label,
		options,
		preselectId = null,
		allowNew = true,
		locate = false,
		autoLocate = false,
		newHint = '',
		selectNote = '',
		value = $bindable(),
		position = $bindable(null)
	}: Props = $props();

	// The list from "Moja poloha" replaces the options of the parent.
	let nearby = $state<PlacePickerOption[] | null>(null);
	let list = $derived(nearby ?? options);
	let locating = $state(false);
	let accuracyM = $state<number | null>(null);
	let geoError = $state<string | null>(null);
	// True when new-place mode came from "no place within the radius", not from a tap.
	let noMatch = $state(false);
	// The user changed the value: a late automatic GPS answer must not replace it.
	let touched = false;

	// Address search for the position of a new place.
	let searchOpen = $state(false);
	let query = $state('');
	let searching = $state(false);
	let searched = $state(false);
	let searchError = $state<string | null>(null);
	let candidates = $state<GeocodeCandidate[]>([]);
	let addressLabel = $state<string | null>(null);

	// newPlaceName null: an existing place (or nothing yet). A string: new-place mode.
	let newMode = $derived(value.placeId === null && value.newPlaceName !== null);

	onMount(() => {
		if (value.placeId === null && value.newPlaceName === null) {
			if (preselectId) {
				value = { placeId: preselectId, newPlaceName: null };
			} else if (allowNew && !locate) {
				// Fuelio: no place within the radius of the GPS point.
				value = { placeId: null, newPlaceName: '' };
				noMatch = true;
			}
		}
		if (locate && autoLocate) locate_(true, false);
	});

	function onSelect(e: Event) {
		const id = (e.currentTarget as HTMLSelectElement).value;
		touched = true;
		value = { placeId: id || null, newPlaceName: null };
	}

	function startNew() {
		touched = true;
		noMatch = false;
		value = { placeId: null, newPlaceName: '' };
	}

	function pickExisting() {
		touched = true;
		noMatch = false;
		value = { placeId: null, newPlaceName: null };
	}

	function onName(e: Event) {
		touched = true;
		value = { placeId: null, newPlaceName: (e.currentTarget as HTMLInputElement).value };
	}

	function dist(m: number): string {
		return m < 1000
			? $LL.mobile.add.picker.distM({ m: Math.round(m) })
			: $LL.mobile.add.picker.distKm({ km: (m / 1000).toFixed(1) });
	}

	function geoMessage(err: GeolocationPositionError): string {
		if (err.code === err.PERMISSION_DENIED) return $LL.mobile.add.picker.denied();
		if (err.code === err.TIMEOUT) return $LL.mobile.add.picker.timeout();
		return $LL.mobile.add.picker.unavailable();
	}

	// Geolocation works only in a secure context (HTTPS or localhost). Every
	// failure shows a message: the user then picks from the list.
	// auto: the call on mount. A tap always applies the result.
	// pinOnly: the button in the new-place panel. It sets the position of the
	// new place and keeps the new-place mode.
	function locate_(auto: boolean, pinOnly: boolean) {
		geoError = null;
		if (!window.isSecureContext || !('geolocation' in navigator)) {
			geoError = $LL.mobile.add.picker.insecure();
			return;
		}
		locating = true;
		navigator.geolocation.getCurrentPosition(
			async (pos) => {
				const lat = pos.coords.latitude;
				const lon = pos.coords.longitude;
				// Keep the fix first: a new place can use it even if the list fails.
				position = { lat, lon, source: 'manual' };
				addressLabel = null;
				accuracyM = Math.round(pos.coords.accuracy);
				if (pinOnly) {
					searchOpen = false;
					locating = false;
					return;
				}
				try {
					const r = await api.getPlacesNear(lat, lon);
					nearby = r.places;
					if (auto && touched) {
						// The user chose while the GPS was slow: keep the choice.
					} else if (r.preselectId) {
						noMatch = false;
						value = { placeId: r.preselectId, newPlaceName: null };
					} else if (allowNew) {
						noMatch = true;
						searchOpen = false;
						value = { placeId: null, newPlaceName: value.newPlaceName ?? '' };
					}
				} catch (e) {
					geoError = $LL.mobile.add.picker.nearError({ error: String(e) });
				} finally {
					locating = false;
				}
			},
			(err) => {
				locating = false;
				geoError = geoMessage(err);
			},
			{ enableHighAccuracy: true, timeout: 15000, maximumAge: 30000 }
		);
	}

	async function search() {
		const q = query.trim();
		if (!q || searching) return;
		searching = true;
		searchError = null;
		searched = true;
		try {
			candidates = await api.geocodePlace(q);
		} catch (e) {
			// Old matches under an error invite a position from an old query.
			candidates = [];
			searchError = String(e);
		} finally {
			searching = false;
		}
	}

	function pickCandidate(c: GeocodeCandidate) {
		position = { lat: c.lat, lon: c.lon, source: 'geocoder' };
		addressLabel = c.label;
		accuracyM = null;
		searchOpen = false;
		candidates = [];
		searched = false;
	}
</script>

<div class="picker" data-testid={testId}>
	<div class="head">
		<label for="{testId}-select">{label}</label>
		{#if locate && !newMode}
			<button
				type="button"
				class="locate"
				onclick={() => locate_(false, false)}
				disabled={locating}
				data-testid="{testId}-locate"
			>
				{locating ? $LL.mobile.add.picker.locating() : $LL.mobile.add.picker.myLocation()}
			</button>
		{/if}
	</div>

	{#if geoError}
		<p class="error" role="alert" data-testid="{testId}-geo-error">{geoError}</p>
	{:else if accuracyM !== null && nearby && !newMode}
		<p class="muted small">{$LL.mobile.add.picker.located({ m: accuracyM })}</p>
	{/if}

	{#if newMode}
		{#if noMatch}
			<p class="muted small" data-testid="{testId}-no-nearby">{$LL.mobile.add.picker.noNearby()}</p>
		{/if}
		<input
			id="{testId}-select"
			type="text"
			class="field"
			placeholder={$LL.mobile.add.picker.newName()}
			value={value.newPlaceName ?? ''}
			oninput={onName}
			autocomplete="off"
			data-testid="{testId}-new-name"
		/>
		{#if locate}
			<div class="pos" data-testid="{testId}-position">
				<p class="small">
					{#if position && addressLabel}
						{$LL.mobile.add.picker.positionAddress({ label: addressLabel })}
					{:else if position && accuracyM !== null}
						{$LL.mobile.add.picker.positionGps({ m: accuracyM })}
					{:else if position}
						{$LL.mobile.add.picker.positionGpsPlain()}
					{:else}
						<span class="warn">{$LL.mobile.add.picker.positionNone()}</span>
					{/if}
				</p>
				<div class="pos-btns">
					<button
						type="button"
						class="locate"
						onclick={() => locate_(false, true)}
						disabled={locating}
						data-testid="{testId}-pin-gps"
						>{locating ? $LL.mobile.add.picker.locating() : $LL.mobile.add.picker.myLocation()}</button
					>
					<button
						type="button"
						class="locate"
						class:on={searchOpen}
						onclick={() => (searchOpen = !searchOpen)}
						data-testid="{testId}-pin-search"
						>{$LL.mobile.add.picker.searchAddress()}</button
					>
				</div>
				{#if searchOpen}
					<form
						class="search"
						onsubmit={(e) => {
							e.preventDefault();
							void search();
						}}
					>
						<input
							type="search"
							class="field"
							placeholder={$LL.mobile.add.picker.addressPlaceholder()}
							bind:value={query}
							autocomplete="off"
							data-testid="{testId}-address"
						/>
						<button type="submit" class="locate" disabled={searching || !query.trim()}
							>{searching ? $LL.mobile.add.picker.searching() : $LL.mobile.add.picker.search()}</button
						>
					</form>
					{#if searchError}
						<p class="error" role="alert">{$LL.mobile.add.picker.searchError({ error: searchError })}</p>
					{:else if searched && !searching && candidates.length === 0}
						<p class="muted small">{$LL.mobile.add.picker.noResults()}</p>
					{/if}
					{#if candidates.length > 0}
						<ul class="cands" data-testid="{testId}-candidates">
							{#each candidates as c, i (i)}
								<li><button type="button" onclick={() => pickCandidate(c)}>{c.label}</button></li>
							{/each}
						</ul>
					{/if}
				{/if}
			</div>
		{:else if newHint}
			<p class="muted small">{newHint}</p>
		{/if}
		{#if list.length > 0}
			<button type="button" class="link" onclick={pickExisting} data-testid="{testId}-pick-existing"
				>{$LL.mobile.add.picker.pickExisting()}</button
			>
		{/if}
	{:else}
		<select
			id="{testId}-select"
			class="field"
			value={value.placeId ?? ''}
			onchange={onSelect}
			data-testid="{testId}-select"
		>
			{#if !value.placeId}
				<option value="" disabled>{$LL.mobile.add.picker.placeholder()}</option>
			{/if}
			{#each list as p (p.id)}
				<option value={p.id}>{p.distanceM === null ? p.name : `${p.name} - ${dist(p.distanceM)}`}</option>
			{/each}
		</select>
		{#if selectNote}
			<p class="muted small" data-testid="{testId}-note">{selectNote}</p>
		{/if}
		{#if allowNew}
			<button type="button" class="link" onclick={startNew} data-testid="{testId}-new"
				>{locate ? $LL.mobile.add.picker.newPlace() : $LL.mobile.add.picker.newHere()}</button
			>
		{/if}
	{/if}
</div>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		min-height: 44px;
	}
	label {
		font-weight: 600;
		font-size: 0.95rem;
	}
	.field {
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
	.locate {
		min-height: 44px;
		padding: 0.4rem 0.9rem;
		border: 1px solid var(--border-input);
		border-radius: 6px;
		background: var(--btn-secondary-bg);
		color: var(--text-primary);
		font-size: 0.95rem;
		cursor: pointer;
		white-space: nowrap;
	}
	.locate:disabled {
		opacity: 0.6;
	}
	.locate.on {
		border-color: var(--accent-primary);
		color: var(--accent-primary);
	}
	.pos {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
		padding: 0.6rem 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 8px;
	}
	.pos-btns {
		display: flex;
		gap: 0.5rem;
	}
	.pos-btns .locate {
		flex: 1;
	}
	.search {
		display: flex;
		gap: 0.5rem;
	}
	.cands {
		list-style: none;
		margin: 0;
		padding: 0;
		border: 1px solid var(--border-default);
		border-radius: 6px;
	}
	.cands li + li {
		border-top: 1px solid var(--border-default);
	}
	.cands button {
		width: 100%;
		min-height: 44px;
		padding: 0.5rem 0.75rem;
		border: none;
		background: none;
		color: var(--text-primary);
		font-size: 0.95rem;
		text-align: left;
		cursor: pointer;
	}
	.link {
		align-self: flex-start;
		min-height: 44px;
		padding: 0.4rem 0;
		border: none;
		background: none;
		color: var(--accent-primary);
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
	.warn {
		color: var(--accent-warning, var(--accent-danger));
	}
	.error {
		margin: 0;
		color: var(--accent-danger);
		font-size: 0.9rem;
	}
</style>
