<script lang="ts">
	import * as api from '$lib/api';
	import type { FuelioPlaceOption } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';

	// Task 97: one field for one end of the Fuelio add dialog. The text is the
	// value: a known place name is that place, other text is a new place. The
	// find_place lookup here only drives the display (the mark, the list). The
	// dialog resolves the text again when the user adds the trip.

	interface Props {
		id: string;
		testId: string;
		/** Places with coordinates, nearest to the GPS point first. */
		options: FuelioPlaceOption[];
		text: string;
		/** No known place near the GPS point (no preselect). */
		noNearby: boolean;
		/** Where a new place gets its position. */
		newHint: string;
	}
	let { id, testId, options, text = $bindable(), noNearby, newHint }: Props = $props();

	/** A known place. No distance: the place has no coordinates. */
	interface Known {
		id: string;
		name: string;
		distanceM: number | null;
	}
	type Row = { kind: 'place'; place: Known } | { kind: 'create' };

	let hit = $state<Known | null>(null);
	// The text that `hit` belongs to. Another text means the lookup is not back yet.
	let lookedUp = $state<string | null>(null);
	let open = $state(false);
	// True after the user typed: the list filters. False after a pick or on
	// focus: the list shows every place.
	let filtering = $state(false);
	let active = $state(-1);

	const trimmed = $derived(text.trim());
	const exact = $derived(options.find((o) => o.name === trimmed) ?? null);
	const pending = $derived(trimmed !== '' && !exact && lookedUp !== trimmed);
	const known = $derived<Known | null>(exact ?? (lookedUp === trimmed ? hit : null));
	const isNew = $derived(trimmed !== '' && !known && !pending);

	// Text in another case or without diacritics: the fold stays in Rust (ADR-008).
	$effect(() => {
		const t = trimmed;
		if (t === '' || exact) return;
		const timer = setTimeout(async () => {
			try {
				const p = await api.findPlace(t);
				if (t !== text.trim()) return;
				hit = p
					? { id: p.id, name: p.name, distanceM: options.find((o) => o.id === p.id)?.distanceM ?? null }
					: null;
				lookedUp = t;
			} catch (e) {
				console.error('find_place failed:', e);
			}
		}, 150);
		return () => clearTimeout(timer);
	});

	const rows = $derived.by((): Row[] => {
		if (!filtering || trimmed === '') return options.map((place) => ({ kind: 'place', place }));
		const q = trimmed.toLowerCase();
		const list: Known[] = options.filter((o) => o.name.toLowerCase().includes(q));
		if (known) {
			const rest = list.filter((o) => o.id !== known.id);
			list.splice(0, list.length, known, ...rest);
		}
		const out: Row[] = list.map((place) => ({ kind: 'place', place }));
		if (isNew) out.push({ kind: 'create' });
		return out;
	});

	function km(m: number): string {
		return (m / 1000).toFixed(m < 10000 ? 2 : 0);
	}

	function pick(row: Row) {
		if (row.kind === 'place') text = row.place.name;
		filtering = false;
		open = false;
		active = -1;
	}

	function onInput() {
		filtering = true;
		open = true;
		active = -1;
	}

	function onKeydown(e: KeyboardEvent) {
		switch (e.key) {
			case 'ArrowDown':
				e.preventDefault();
				open = true;
				active = Math.min(active + 1, rows.length - 1);
				break;
			case 'ArrowUp':
				e.preventDefault();
				active = Math.max(active - 1, -1);
				break;
			case 'Enter':
				if (open && active >= 0 && active < rows.length) {
					e.preventDefault();
					pick(rows[active]);
				}
				break;
			case 'Escape':
				// Close the list only, not the dialog.
				if (open) {
					e.stopPropagation();
					open = false;
				}
				break;
			case 'Tab':
				open = false;
				break;
		}
	}
</script>

<div class="field">
	<div class="box">
		<input
			{id}
			type="text"
			bind:value={text}
			placeholder={$LL.fuelio.add.placeholder()}
			autocomplete="off"
			role="combobox"
			aria-expanded={open && rows.length > 0}
			aria-controls="{id}-list"
			aria-autocomplete="list"
			oninput={onInput}
			onfocus={() => {
				filtering = false;
				open = true;
			}}
			onclick={() => (open = true)}
			onblur={() => (open = false)}
			onkeydown={onKeydown}
			data-testid={testId}
		/>
		{#if known && known.distanceM !== null}
			<span class="dist">{$LL.fuelio.add.distanceFrom({ km: km(known.distanceM) })}</span>
		{:else if isNew}
			<span class="new-mark" data-testid="{testId}-new-mark">{$LL.fuelio.add.newMark()}</span>
		{/if}
	</div>
	{#if open && rows.length > 0}
		<ul id="{id}-list" class="list" role="listbox">
			{#each rows as row, i (row.kind === 'place' ? row.place.id : '__create__')}
				<li
					role="option"
					aria-selected={i === active}
					class:active={i === active}
					class:create={row.kind === 'create'}
					onmousedown={(e) => {
						e.preventDefault();
						pick(row);
					}}
					data-testid={row.kind === 'place' ? `${testId}-option` : `${testId}-create`}
				>
					{#if row.kind === 'place'}
						<span>{row.place.name}</span>
						{#if row.place.distanceM !== null}
							<span class="dist">{$LL.fuelio.add.distanceShort({ km: km(row.place.distanceM) })}</span>
						{/if}
					{:else}
						<span>{$LL.fuelio.add.createPlace({ name: trimmed })}</span>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
	{#if (noNearby && !known) || isNew}
		<span class="hint">
			{#if noNearby && !known}<span data-testid="{testId}-no-nearby">{$LL.fuelio.add.noNearby()}</span>{/if}
			{#if isNew}{newHint}{/if}
		</span>
	{/if}
</div>

<style>
	.field {
		position: relative;
	}
	.box {
		position: relative;
		display: flex;
		align-items: center;
	}
	input {
		box-sizing: border-box;
		width: 100%;
		padding: 0.35rem 0.5rem;
		padding-right: 9rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		background: var(--input-bg);
		color: var(--text-primary);
		font-size: 0.875rem;
	}
	.box > .dist,
	.box > .new-mark {
		position: absolute;
		right: 0.5rem;
		pointer-events: none;
		white-space: nowrap;
	}
	.dist {
		color: var(--text-secondary);
		font-size: 0.8rem;
	}
	.new-mark {
		font-size: 0.72rem;
		font-weight: 600;
		color: var(--accent-warning-dark);
		background: var(--consumption-warn-bg);
		border-radius: 4px;
		padding: 1px 6px;
	}
	.list {
		position: absolute;
		left: 0;
		right: 0;
		z-index: 10;
		margin: 2px 0 0;
		padding: 0;
		list-style: none;
		max-height: 14rem;
		overflow-y: auto;
		background: var(--bg-surface);
		border: 1px solid var(--border-input);
		border-radius: 4px;
		box-shadow: 0 4px 10px var(--shadow-default);
	}
	li {
		display: flex;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.3rem 0.6rem;
		cursor: pointer;
		font-size: 0.85rem;
	}
	li:hover,
	li.active {
		background: var(--accent-primary);
		color: #fff;
	}
	li:hover .dist,
	li.active .dist {
		color: inherit;
	}
	li.create {
		border-top: 1px solid var(--border-default);
		font-weight: 600;
	}
	.hint {
		display: block;
		font-size: 0.8rem;
		margin-top: 0.2rem;
		color: var(--text-secondary);
	}
</style>
