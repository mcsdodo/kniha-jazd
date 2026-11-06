<script lang="ts">
	// Task 96: the place list on the phone. Places belong to the whole
	// database, not to a vehicle, so there is no vehicle gate. The values
	// come from the backend (ADR-008).
	import { onMount } from 'svelte';
	import { pushState } from '$app/navigation';
	import { page } from '$app/state';
	import {
		listPlaces,
		renamePlace,
		setPlacePosition,
		setHomePlace,
		createPlace,
		deletePlace
	} from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Place } from '$lib/types';
	import { appModeStore } from '$lib/stores/appMode';
	import LL from '$lib/i18n/i18n-svelte';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import MobileBar from '$lib/components/mobile/MobileBar.svelte';
	import { coords5 } from '$lib/components/mobile/MobileFormat';
	import PlaceEditor, { type PlaceDraft } from '$lib/components/mobile/PlaceEditor.svelte';

	// The backend orders the list (unplaced first, then most-used, then by
	// name). Render it in the order it arrives.
	let places = $state<Place[]>([]);
	let loaded = $state(false);
	let loadError = $state<string | null>(null);
	let placeFilter = $state('');

	let placeFilterNeedle = $derived(placeFilter.trim().toLowerCase());
	// The same rule as the PC page: a query with diacritics matches `name`,
	// an ASCII query matches the backend's `normalisedName`. Do not add a
	// folding table here (ADR-008).
	let visiblePlaces = $derived(
		placeFilterNeedle
			? places.filter(
					(place) =>
						place.name.toLowerCase().includes(placeFilterNeedle) ||
						place.normalisedName.includes(placeFilterNeedle)
				)
			: places
	);

	// The open place lives in the history state of a shallow-routing entry, so
	// the back gesture closes the edit screen.
	let editId = $derived((page.state as { placeId?: string }).placeId ?? null);
	let editPlace = $derived(editId ? (places.find((p) => p.id === editId) ?? null) : null);
	let isNew = $derived((page.state as { newPlace?: boolean }).newPlace === true);
	let placeToDelete = $state<Place | null>(null);

	// The question belongs to the open editor: close it with the editor.
	$effect(() => {
		if (!editId) placeToDelete = null;
	});

	function openEditor(placeId: string) {
		pushState('', { placeId });
	}

	function openNew() {
		pushState('', { newPlace: true });
	}

	// Leave the edit screen after a write. If the user already made the back
	// gesture while the write ran, a second back would leave the list.
	function leaveEditor(stillOpen: () => boolean) {
		if (stillOpen()) history.back();
	}

	function closeEditor() {
		history.back();
	}

	// Write the changes of an existing place, step by step. The first error
	// stops the chain and keeps the screen open.
	async function saveExisting(id: string, draft: PlaceDraft) {
		let current: Place | undefined;
		try {
			current = (await listPlaces()).find((p) => p.id === id);
			if (!current) throw new Error('place not found');
		} catch (error) {
			toast.error($LL.places.saveError({ error: String(error) }));
			return;
		}
		const name = draft.name.trim();
		if (name !== current.name) {
			try {
				await renamePlace(id, name);
			} catch (error) {
				toast.error($LL.places.renameError({ error: String(error) }));
				return;
			}
		}
		if (draft.lat !== current.lat || draft.lon !== current.lon || draft.source !== current.source) {
			try {
				await setPlacePosition(id, draft.lat, draft.lon, draft.source);
			} catch (error) {
				toast.error($LL.places.saveError({ error: String(error) }));
				return;
			}
		}
		if (draft.isHome !== current.isHome) {
			try {
				await setHomePlace(draft.isHome ? id : null);
			} catch (error) {
				toast.error($LL.places.homeError({ error: String(error) }));
				return;
			}
		}
		leaveEditor(() => (page.state as { placeId?: string }).placeId === id);
		await loadPlaces();
		toast.success($LL.places.saved());
	}

	// Create a new place. After a successful create the screen always closes.
	async function saveNew(draft: PlaceDraft) {
		let created: Place;
		try {
			created = await createPlace(draft.name.trim(), draft.lat, draft.lon, draft.source);
		} catch (error) {
			toast.error($LL.places.saveError({ error: String(error) }));
			return;
		}
		let homeFailed: string | null = null;
		if (draft.isHome) {
			try {
				await setHomePlace(created.id);
			} catch (error) {
				homeFailed = String(error);
			}
		}
		leaveEditor(() => (page.state as { newPlace?: boolean }).newPlace === true);
		await loadPlaces();
		if (homeFailed !== null) {
			toast.error($LL.places.homeError({ error: homeFailed }));
		} else {
			toast.success($LL.places.saved());
		}
	}

	async function confirmDelete() {
		const target = placeToDelete;
		placeToDelete = null;
		if (!target) return;
		try {
			await deletePlace(target.id);
		} catch (error) {
			toast.error($LL.places.deleteError({ error: String(error) }));
			return;
		}
		leaveEditor(() => (page.state as { placeId?: string }).placeId === target.id);
		await loadPlaces();
		toast.success($LL.places.deleted());
	}

	onMount(loadPlaces);

	async function loadPlaces() {
		try {
			places = await listPlaces();
			loadError = null;
		} catch (error) {
			console.error('Failed to load places:', error);
			loadError = String(error);
		} finally {
			loaded = true;
		}
	}
</script>

<MobileBar title={$LL.places.title()} backHref="/m" backLabel={$LL.mobile.places.back()} />

<div class="body" data-testid="m-miesta">
	<input
		type="search"
		class="filter"
		placeholder={$LL.places.filterPlaceholder()}
		bind:value={placeFilter}
		data-testid="m-miesta-filter"
	/>
	{#if !$appModeStore.isReadOnly}
		<button type="button" class="add" onclick={openNew}
			data-testid="m-miesta-add"
		>{$LL.places.add()}</button>
	{/if}

	{#if loadError}
		<p class="error">{loadError}</p>
	{:else if loaded && places.length === 0}
		<p class="note" data-testid="m-miesta-empty">{$LL.places.empty()}</p>
	{:else if loaded && visiblePlaces.length === 0}
		<p class="note" data-testid="m-miesta-no-matches">{$LL.places.noMatches()}</p>
	{/if}

	{#each visiblePlaces as place (place.id)}
		<div
			class="card"
			role="button"
			tabindex="0"
			onclick={() => openEditor(place.id)}
			onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && openEditor(place.id)}
			data-testid="m-miesta-card"
			data-place-id={place.id}
		>
			<div class="name-row">
				<span class="name" data-testid="m-miesta-card-name">{place.name}</span>
				{#if place.isHome}
					<svg
						xmlns="http://www.w3.org/2000/svg"
						width="18"
						height="18"
						viewBox="0 0 24 24"
						fill="currentColor"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
						class="home"
						data-testid="m-miesta-card-home"
						aria-hidden="true"
					><path d="M3 10.5 12 3l9 7.5" /><path d="M5 9.5V21h14V9.5" /></svg>
				{/if}
			</div>
			<div class="meta">
				<span data-testid="m-miesta-card-uses">{$LL.places.uses({ count: place.uses })}</span>
				<span data-testid="m-miesta-card-coords">{coords5(place.lat, place.lon)}</span>
			</div>
		</div>
	{/each}
</div>

{#if editId && editPlace}
	<div class="overlay">
		<MobileBar
			title={$LL.mobile.places.editTitle()}
			onback={closeEditor}
			backLabel={$LL.places.title()}
		/>
		{#key editPlace.id}
			<PlaceEditor
				place={editPlace}
				readOnly={$appModeStore.isReadOnly}
				onsave={(draft) => saveExisting(editPlace.id, draft)}
				ondelete={() => (placeToDelete = editPlace)}
			/>
		{/key}
	</div>
{:else if isNew}
	<div class="overlay">
		<MobileBar title={$LL.places.add()} onback={closeEditor} backLabel={$LL.places.title()} />
		<PlaceEditor place={null} readOnly={$appModeStore.isReadOnly} onsave={saveNew} ondelete={() => {}} />
	</div>
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
	.overlay {
		position: fixed;
		inset: 0;
		z-index: 50;
		overflow-y: auto;
		overscroll-behavior: contain;
		background: var(--bg-body);
	}

	.body {
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		max-width: 640px;
		margin: 0 auto;
	}

	.filter {
		min-height: 44px;
		padding: 0 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 10px;
		background: var(--bg-surface);
		color: var(--text-primary);
		font-size: 1rem;
		box-sizing: border-box;
	}

	.add {
		min-height: 44px;
		border-radius: 10px;
		border: 2px solid var(--accent-primary);
		background: var(--accent-primary);
		color: #fff;
		font-size: 1rem;
		font-weight: 600;
	}

	.card {
		min-height: 44px;
		padding: 0.7rem 0.9rem;
		border: 1px solid var(--border-default);
		border-radius: 12px;
		background: var(--bg-surface);
		box-sizing: border-box;
		cursor: pointer;
	}

	.name-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.name {
		font-weight: 600;
		overflow-wrap: anywhere;
	}

	.home {
		flex: none;
		color: var(--accent-primary);
	}

	.meta {
		display: flex;
		justify-content: space-between;
		gap: 0.75rem;
		margin-top: 0.2rem;
		font-size: 0.85rem;
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
	}

	.note {
		margin: 0;
		color: var(--text-secondary);
	}

	.error {
		margin: 0;
		color: var(--accent-danger);
	}
</style>
