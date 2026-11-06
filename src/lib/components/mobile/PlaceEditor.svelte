<script lang="ts" module>
	import type { PlaceSource } from '$lib/types';

	/** What the editor hands to the page on save. The page owns the writes. */
	export type PlaceDraft = {
		name: string;
		lat: number;
		lon: number;
		source: PlaceSource;
		isHome: boolean;
	};
</script>

<script lang="ts">
	// Task 96: the edit screen of one place on the phone. The draft is local
	// until "Save": it is seeded once from `place`, so the page wraps this
	// component in {#key} on the place id. A geocoder answer never reaches the
	// database before the user presses Save (ADR-032).
	import { untrack } from 'svelte';
	import { geocodePlace } from '$lib/api';
	import type { GeocodeCandidate, Place } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import PlacePinMap from './PlacePinMap.svelte';
	import { coords5 } from './MobileFormat';

	let {
		place,
		readOnly,
		onsave,
		ondelete
	}: {
		/** null: a new place. */
		place: Place | null;
		readOnly: boolean;
		onsave: (draft: PlaceDraft) => Promise<void>;
		ondelete: () => void;
	} = $props();

	let name = $state(untrack(() => place?.name ?? ''));
	let lat = $state<number | null>(untrack(() => place?.lat ?? null));
	let lon = $state<number | null>(untrack(() => place?.lon ?? null));
	let source = $state<PlaceSource | null>(untrack(() => place?.source ?? null));
	let isHome = $state(untrack(() => place?.isHome ?? false));
	// True after a tap on the map or a drag of the pin.
	let pinnedByHand = $state(false);
	let saving = $state(false);

	let query = $state(untrack(() => place?.name ?? ''));
	let candidates = $state<GeocodeCandidate[]>([]);
	let searching = $state(false);
	let searched = $state(false);
	let searchFailed = $state(false);

	let locating = $state(false);
	let geoError = $state<string | null>(null);

	let position = $derived(lat !== null && lon !== null ? { lat, lon } : null);
	let canSave = $derived(
		!readOnly && !saving && name.trim() !== '' && lat !== null && lon !== null && source !== null
	);

	function onpick(newLat: number, newLon: number) {
		lat = newLat;
		lon = newLon;
		source = 'manual';
		pinnedByHand = true;
	}

	async function runSearch(event: SubmitEvent) {
		event.preventDefault();
		const trimmed = query.trim();
		if (!trimmed || searching) return;
		searching = true;
		searchFailed = false;
		searched = true;
		try {
			candidates = await geocodePlace(trimmed);
		} catch (error) {
			console.error('Failed to geocode place:', error);
			candidates = [];
			searchFailed = true;
		} finally {
			searching = false;
		}
	}

	function pickCandidate(candidate: GeocodeCandidate) {
		lat = candidate.lat;
		lon = candidate.lon;
		source = 'geocoder';
		pinnedByHand = false;
		candidates = [];
		searched = false;
	}

	function geoMessage(err: GeolocationPositionError): string {
		if (err.code === err.PERMISSION_DENIED) return $LL.mobile.add.picker.denied();
		if (err.code === err.TIMEOUT) return $LL.mobile.add.picker.timeout();
		return $LL.mobile.add.picker.unavailable();
	}

	// Geolocation works only in a secure context (HTTPS or localhost).
	function locate() {
		geoError = null;
		if (!window.isSecureContext || !('geolocation' in navigator)) {
			geoError = $LL.mobile.add.picker.insecure();
			return;
		}
		locating = true;
		navigator.geolocation.getCurrentPosition(
			(pos) => {
				locating = false;
				lat = pos.coords.latitude;
				lon = pos.coords.longitude;
				source = 'manual';
				pinnedByHand = false;
			},
			(err) => {
				locating = false;
				geoError = geoMessage(err);
			},
			{ enableHighAccuracy: true, timeout: 15000, maximumAge: 30000 }
		);
	}

	async function save() {
		if (!canSave || lat === null || lon === null || source === null) return;
		saving = true;
		try {
			await onsave({ name: name.trim(), lat, lon, source, isHome });
		} finally {
			saving = false;
		}
	}
</script>

<div class="editor" data-testid="m-place-editor">
	<label class="field">
		<span class="label">{$LL.places.addName()}</span>
		<input
			type="text"
			class="input"
			bind:value={name}
			disabled={readOnly}
			data-testid="m-place-name"
		/>
	</label>

	<div class="field">
		<span class="label">{$LL.mobile.places.position()}</span>
		<form class="search-row" onsubmit={runSearch}>
			<input
				type="text"
				class="input grow"
				bind:value={query}
				placeholder={$LL.places.searchPlaceholder()}
				aria-label={$LL.places.searchPlaceholder()}
				disabled={readOnly}
				data-testid="m-place-search"
			/>
			<button
				type="submit"
				class="btn secondary"
				disabled={readOnly || searching}
				data-testid="m-place-search-btn"
			>{searching ? $LL.places.searching() : $LL.places.search()}</button>
		</form>

		{#if searchFailed}
			<p class="status error">{$LL.places.searchError()}</p>
		{:else if searched && !searching && candidates.length === 0}
			<p class="status">{$LL.places.noResults()}</p>
		{/if}
		{#if candidates.length > 0}
			<ul class="candidates">
				{#each candidates as candidate (candidate.lat + ',' + candidate.lon + candidate.label)}
					<li>
						<button
							type="button"
							class="candidate"
							disabled={readOnly}
							onclick={() => pickCandidate(candidate)}
							data-testid="m-place-candidate"
						>{candidate.label}</button>
					</li>
				{/each}
			</ul>
		{/if}

		<PlacePinMap {position} {onpick} {readOnly} />
		<p class="status" data-testid="m-place-position">
			{#if pinnedByHand && lat !== null && lon !== null}
				{$LL.mobile.places.positionManual({ coords: coords5(lat, lon) })}
			{:else}
				{$LL.mobile.places.mapHint()}
			{/if}
		</p>

		<button
			type="button"
			class="btn secondary"
			disabled={readOnly || locating}
			onclick={locate}
			data-testid="m-place-gps"
		>{$LL.mobile.add.picker.myLocation()}</button>
		{#if geoError}
			<p class="status error" data-testid="m-place-gps-error">{geoError}</p>
		{/if}
	</div>

	<label class="check">
		<input type="checkbox" bind:checked={isHome} disabled={readOnly} data-testid="m-place-home" />
		<span>{$LL.places.markHome()}</span>
	</label>

	{#if !readOnly}
		<button
			type="button"
			class="btn primary"
			disabled={!canSave}
			onclick={save}
			data-testid="m-place-save"
		>{$LL.places.save()}</button>

		{#if place}
			<button
				type="button"
				class="btn danger"
				disabled={place.uses > 0}
				onclick={ondelete}
				data-testid="m-place-delete"
			>{$LL.places.delete()}</button>
			{#if place.uses > 0}
				<p class="status" data-testid="m-place-in-use">
					{$LL.places.deleteInUse({ count: place.uses })}
				</p>
			{/if}
		{/if}
	{/if}
</div>

<style>
	.editor {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.9rem;
		max-width: 640px;
		margin: 0 auto;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.label {
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--text-secondary);
	}

	.input {
		min-height: 44px;
		padding: 0 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 10px;
		background: var(--input-bg);
		color: var(--text-primary);
		font-size: 1rem;
		box-sizing: border-box;
	}

	.grow {
		flex: 1;
		min-width: 0;
	}

	.search-row {
		display: flex;
		gap: 0.5rem;
	}

	.btn {
		min-height: 44px;
		padding: 0 1rem;
		border-radius: 10px;
		border: 2px solid var(--accent-primary);
		font-size: 1rem;
		font-weight: 600;
	}

	.btn:disabled {
		opacity: 0.5;
	}

	.btn.primary {
		background: var(--accent-primary);
		color: #fff;
	}

	.btn.secondary {
		background: var(--bg-surface);
		color: var(--accent-primary);
	}

	.btn.danger {
		background: var(--bg-surface);
		border-color: var(--accent-danger);
		color: var(--accent-danger);
	}

	.candidates {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}

	.candidate {
		width: 100%;
		min-height: 44px;
		padding: 0.5rem 0.75rem;
		text-align: left;
		border: 1px solid var(--border-default);
		border-radius: 10px;
		background: var(--bg-surface);
		color: var(--text-primary);
		font-size: 0.95rem;
	}

	.status {
		margin: 0;
		font-size: 0.85rem;
		color: var(--text-secondary);
	}

	.status.error {
		color: var(--accent-danger);
	}

	.check {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		min-height: 44px;
	}

	.check input {
		width: 22px;
		height: 22px;
	}
</style>
