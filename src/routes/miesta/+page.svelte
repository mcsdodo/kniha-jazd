<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import PlaceModal from '$lib/components/PlaceModal.svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import type { Place, PlaceSource } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';

	// Reference data: the place book. The backend already orders the list
	// (unplaced first, then most-used, then by name) - render it in the order it
	// arrives, never re-sort here (ADR-008).
	let places = $state<Place[]>([]);
	let placeFilter = $state('');
	// Set by a row's edit button; drives the map dialog for an existing place.
	let editingPlace = $state<Place | null>(null);

	// Add flow: name first, then the map dialog for the position.
	let adding = $state(false);
	let addName = $state('');
	let newPlace = $state<{ name: string } | null>(null);

	// Rename flow: one row at a time.
	let renamingId = $state<string | null>(null);
	let renameValue = $state('');

	let placeToDelete = $state<Place | null>(null);

	// Display formatting of backend data, not business logic.
	let placedCount = $derived(places.filter(isPlaced).length);
	// Lowercase with the invariant rules - no locale argument. The only
	// locale-sensitive case-folding that matters is Turkish dotted/dotless i, and
	// there is no Turkish UI; for Slovak this is identical to a locale-aware fold.
	let placeFilterNeedle = $derived(placeFilter.trim().toLowerCase());
	// Match both spellings the row already carries. `normalisedName` is the
	// backend's `places::normalise` output - lowercased, diacritics folded,
	// whitespace collapsed - so an ASCII query finds a name written with
	// diacritics, which is the case that actually occurs: production data shows
	// users type "Kosice", not "Košice" (see the `normalisedName` field and `places::normalise`).
	// Folding the needle here instead would mean a second, divergent
	// copy of `normalise` in TypeScript, which ADR-008 forbids - so the needle
	// stays unfolded and the two spellings are reached by two routes: a query
	// typed with diacritics matches `name`, an ASCII one matches
	// `normalisedName`. That asymmetry is deliberate; do not "complete" it with a
	// JS folding table.
	let visiblePlaces = $derived(
		placeFilterNeedle
			? places.filter(
					(place) =>
						place.name.toLowerCase().includes(placeFilterNeedle) ||
						place.normalisedName.includes(placeFilterNeedle)
				)
			: places
	);

	onMount(loadPlaces);

	async function loadPlaces() {
		try {
			places = await api.listPlaces();
		} catch (error) {
			console.error('Failed to load places:', error);
		}
	}

	function isPlaced(place: Place): boolean {
		return place.lat !== null && place.lon !== null;
	}

	/** Three decimals is ~100 m - enough to recognise a place, short enough to read. */
	function formatCoordinates(place: Place): string {
		if (place.lat === null || place.lon === null) return '';
		return `${place.lat.toFixed(3)}, ${place.lon.toFixed(3)}`;
	}

	function openEditPlace(place: Place) {
		editingPlace = place;
	}

	function closePlaceModal() {
		editingPlace = null;
		newPlace = null;
	}

	/** The dialog hands back only the coordinate a human confirmed. The name
	 *  stored against it is the place's own name, never the geocoder's rendering
	 *  of the address (ADR-034). An existing place keeps its id and name; a new
	 *  one is created with the name typed in the add form. */
	async function handleSavePlace(coords: { lat: number; lon: number; source: PlaceSource }) {
		try {
			if (editingPlace) {
				await api.setPlacePosition(editingPlace.id, coords.lat, coords.lon, coords.source);
			} else if (newPlace) {
				await api.createPlace(newPlace.name, coords.lat, coords.lon, coords.source);
				adding = false;
				addName = '';
			} else {
				return;
			}
			closePlaceModal();
			await loadPlaces();
			toast.success($LL.places.saved());
		} catch (error) {
			console.error('Failed to save place:', error);
			toast.error($LL.places.saveError({ error: String(error) }));
		}
	}

	function startAdd() {
		adding = true;
		addName = '';
	}

	function cancelAdd() {
		adding = false;
		addName = '';
	}

	function continueAdd() {
		const name = addName.trim();
		if (!name) return;
		newPlace = { name };
	}

	function startRename(place: Place) {
		renamingId = place.id;
		renameValue = place.name;
	}

	function cancelRename() {
		renamingId = null;
		renameValue = '';
	}

	async function saveRename(place: Place) {
		const name = renameValue.trim();
		if (!name) return;
		try {
			await api.renamePlace(place.id, name);
			cancelRename();
			await loadPlaces();
			toast.success($LL.places.renamed());
		} catch (error) {
			// The backend message names the colliding place.
			console.error('Failed to rename place:', error);
			toast.error($LL.places.renameError({ error: String(error) }));
		}
	}

	async function confirmDelete() {
		const place = placeToDelete;
		if (!place) return;
		placeToDelete = null;
		try {
			await api.deletePlace(place.id);
			await loadPlaces();
			toast.success($LL.places.deleted());
		} catch (error) {
			console.error('Failed to delete place:', error);
			toast.error($LL.places.deleteError({ error: String(error) }));
		}
	}
</script>

<div class="places-page">
	<section class="places-section" data-testid="places-section">
		<h2 class="places-heading">
			<span>{$LL.places.title()}</span>
			<span class="places-counter" data-testid="places-counter">
				{$LL.places.placed({ count: placedCount, total: places.length })}
			</span>
		</h2>

		<div class="add-bar">
			{#if adding}
				<input
					type="text"
					class="text-input"
					data-testid="place-add-name"
					bind:value={addName}
					placeholder={$LL.places.addName()}
					aria-label={$LL.places.addName()}
					onkeydown={(e) => {
						if (e.key === 'Enter') continueAdd();
						if (e.key === 'Escape') cancelAdd();
					}}
				/>
				<button
					class="button-small"
					data-testid="place-add-next"
					disabled={addName.trim() === ''}
					onclick={continueAdd}
				>
					{$LL.places.addNext()}
				</button>
				<button class="button-small" onclick={cancelAdd}>{$LL.common.cancel()}</button>
			{:else}
				<button class="button-small" data-testid="place-add" onclick={startAdd}>
					{$LL.places.add()}
				</button>
			{/if}
		</div>

		{#if places.length > 0}
			<input
				type="text"
				class="text-input places-filter"
				data-testid="places-filter"
				bind:value={placeFilter}
				placeholder={$LL.places.filterPlaceholder()}
				aria-label={$LL.places.filterPlaceholder()}
			/>
		{/if}

		<div class="place-list" data-testid="places-list">
			{#each visiblePlaces as place (place.id)}
				<div
					class="place-row"
					data-testid="place-row"
					data-place-id={place.id}
					data-place-placed={isPlaced(place)}
				>
					<div class="place-info">
						{#if renamingId === place.id}
							<div class="rename-bar">
								<input
									type="text"
									class="text-input"
									data-testid="place-rename-input"
									bind:value={renameValue}
									aria-label={$LL.places.rename()}
									onkeydown={(e) => {
										if (e.key === 'Enter') saveRename(place);
										if (e.key === 'Escape') cancelRename();
									}}
								/>
								<button
									class="button-small"
									data-testid="place-rename-save"
									disabled={renameValue.trim() === ''}
									onclick={() => saveRename(place)}
								>
									{$LL.places.renameSave()}
								</button>
								<button class="button-small" onclick={cancelRename}>
									{$LL.common.cancel()}
								</button>
							</div>
						{:else}
							<strong>
								{#if !isPlaced(place)}
									<span
										class="unplaced-icon"
										data-testid="place-unplaced-icon"
										title={$LL.places.needsPosition()}
									>&#9888;</span>
								{/if}
								<span data-testid="place-name">{place.name}</span>
							</strong>
						{/if}
						<span class="details" data-testid="place-uses">
							{$LL.places.uses({ count: place.uses })}
						</span>
					</div>
					<div class="place-actions">
						{#if isPlaced(place)}
							<span class="place-coords" data-testid="place-coords">
								{formatCoordinates(place)}
							</span>
						{:else}
							<span class="place-coords missing" data-testid="place-coords">-</span>
						{/if}
						<button
							class="button-small"
							data-testid="place-edit"
							onclick={() => openEditPlace(place)}
						>
							{$LL.common.edit()}
						</button>
						<button
							class="button-small"
							data-testid="place-rename"
							onclick={() => startRename(place)}
						>
							{$LL.places.rename()}
						</button>
						<button
							class="button-small"
							data-testid="place-delete"
							disabled={place.uses > 0}
							title={place.uses > 0 ? $LL.places.deleteInUse({ count: place.uses }) : ''}
							onclick={() => (placeToDelete = place)}
						>
							{$LL.places.delete()}
						</button>
					</div>
				</div>
			{:else}
				{#if places.length > 0}
					<!-- Distinct from places-empty: there are places, the filter just
					     matches none of them. -->
					<p class="placeholder" data-testid="places-no-matches">
						{$LL.places.noMatches()}
					</p>
				{:else}
					<p class="placeholder" data-testid="places-empty">{$LL.places.empty()}</p>
				{/if}
			{/each}
		</div>
	</section>
</div>

{#if editingPlace || newPlace}
	<!-- The key is what forces a fresh dialog when it is re-targeted, so the
	     dialog cannot keep a previous place's pin. PlaceModal seeds its pending
	     coordinate from the prop exactly once, so swapping the prop under a live
	     instance would leave place A's coordinate in it under place B's name -
	     the wrong-pin outcome ADR-032 exists to prevent. The #if above does not
	     prevent that on its own: nothing traps focus, so a keyboard user can tab
	     to another row's edit button behind the open dialog, and editingPlace
	     goes straight from A to B without ever being null. The place id is the
	     row identity - the same key the list's #each uses. -->
	{#key editingPlace?.id ?? 'new'}
		<PlaceModal
			place={editingPlace ?? newPlace!}
			onSave={handleSavePlace}
			onClose={closePlaceModal}
		/>
	{/key}
{/if}

{#if placeToDelete}
	<ConfirmModal
		title={$LL.places.deleteConfirmTitle()}
		message={$LL.places.deleteConfirmMessage({ name: placeToDelete.name })}
		confirmText={$LL.places.delete()}
		cancelText={$LL.common.cancel()}
		danger={true}
		onConfirm={confirmDelete}
		onCancel={() => (placeToDelete = null)}
	/>
{/if}

<style>
	.places-page {
		max-width: 800px;
		margin: 0 auto;
	}

	.places-section {
		background: var(--bg-surface);
		padding: 1.5rem;
		border-radius: 8px;
		box-shadow: 0 1px 3px var(--shadow-default);
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.places-heading {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 1rem;
		margin: 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.places-counter {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text-secondary);
	}

	.add-bar,
	.rename-bar {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.text-input {
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 0.875rem;
		font-family: inherit;
		background-color: var(--input-bg);
		color: var(--text-primary);
	}

	.add-bar .text-input,
	.rename-bar .text-input {
		flex: 1;
		min-width: 0;
	}

	.text-input:focus {
		outline: none;
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	.place-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.place-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 1rem;
		padding: 0.625rem 1rem;
		border: 1px solid var(--border-default);
		border-radius: 4px;
		background: var(--bg-surface-alt);
	}

	.place-info {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		min-width: 0;
		flex: 1;
	}

	.place-info strong {
		font-size: 0.9375rem;
		font-weight: 500;
		color: var(--text-primary);
	}

	.details {
		font-size: 0.8125rem;
		color: var(--text-secondary);
	}

	.unplaced-icon {
		color: var(--warning-highlight);
	}

	.place-actions {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		flex-shrink: 0;
	}

	.place-coords {
		font-size: 0.8125rem;
		font-variant-numeric: tabular-nums;
		color: var(--text-secondary);
		white-space: nowrap;
	}

	.place-coords.missing {
		color: var(--text-muted);
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

	.button-small:hover:not(:disabled) {
		background-color: var(--btn-secondary-hover);
	}

	.button-small:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.placeholder {
		margin: 0;
		color: var(--text-muted);
	}
</style>
