<script lang="ts" module>
	export interface PlacePickerOption {
		id: string;
		name: string;
		/** Metres from the GPS point. Null: the list has no GPS point (sorted by name). */
		distanceM: number | null;
	}
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import type { FuelioAddEnd } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';

	// Task 93: one end of a new trip on the phone. The user picks an existing
	// place, or names a new place. The backend sorts the places and decides the
	// preselect (ADR-008); this component only shows them.

	interface Props {
		/** Used for the element ids and the data-testids: "<testId>-select" and so on. */
		testId: string;
		label: string;
		options: PlacePickerOption[];
		/** The backend preselect (a place within the radius), or null. */
		preselectId?: string | null;
		/** "+ Nove miesto tu" is offered. With `locate`, only after a GPS position is known. */
		allowNew?: boolean;
		/** Shows the "Moja poloha" button. */
		locate?: boolean;
		/** Asks the GPS location on mount (needs `locate`). */
		autoLocate?: boolean;
		/** Where the new place goes, shown under the name input. */
		newHint?: string;
		value: FuelioAddEnd;
		/** The phone GPS position from "Moja poloha", for a new place. */
		position?: { lat: number; lon: number } | null;
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

	let newPossible = $derived(allowNew && (!locate || position !== null));
	// newPlaceName null: an existing place (or nothing yet). A string: new-place mode.
	let newMode = $derived(value.placeId === null && value.newPlaceName !== null);

	function applyPreselect(id: string | null) {
		if (id) {
			value = { placeId: id, newPlaceName: null };
		} else if (newPossible) {
			value = { placeId: null, newPlaceName: '' };
			noMatch = true;
		} else {
			value = { placeId: null, newPlaceName: null };
		}
	}

	onMount(() => {
		if (value.placeId === null && value.newPlaceName === null) applyPreselect(preselectId);
		if (locate && autoLocate) locateMe(true);
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
	// auto: the call on mount. A tap on "Moja poloha" always applies the result.
	function locateMe(auto = false) {
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
				position = { lat, lon };
				accuracyM = Math.round(pos.coords.accuracy);
				try {
					const r = await api.getPlacesNear(lat, lon);
					nearby = r.places;
					// A new GPS point: the backend preselect replaces the old choice.
					if (auto && touched) {
						// The user chose while the GPS was slow: keep the choice.
					} else if (r.preselectId) {
						noMatch = false;
						value = { placeId: r.preselectId, newPlaceName: null };
					} else if (allowNew) {
						noMatch = true;
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
</script>

<div class="picker" data-testid={testId}>
	<div class="head">
		<label for="{testId}-select">{label}</label>
		{#if locate}
			<button
				type="button"
				class="locate"
				onclick={() => locateMe()}
				disabled={locating}
				data-testid="{testId}-locate"
			>
				{locating ? $LL.mobile.add.picker.locating() : $LL.mobile.add.picker.myLocation()}
			</button>
		{/if}
	</div>

	{#if geoError}
		<p class="error" role="alert" data-testid="{testId}-geo-error">{geoError}</p>
	{:else if accuracyM !== null && nearby}
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
		{#if newHint}
			<p class="muted small">{newHint}</p>
		{/if}
		{#if list.length > 0}
			<button
				type="button"
				class="link"
				onclick={() => (value = { placeId: null, newPlaceName: null })}
				data-testid="{testId}-pick-existing">{$LL.mobile.add.picker.pickExisting()}</button
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
		{#if newPossible}
			<button type="button" class="link" onclick={startNew} data-testid="{testId}-new"
				>{$LL.mobile.add.picker.newHere()}</button
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
	}
	.locate:disabled {
		opacity: 0.6;
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
	.error {
		margin: 0;
		color: var(--accent-danger);
		font-size: 0.9rem;
	}
</style>
