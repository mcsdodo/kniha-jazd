<script lang="ts">
	import { setHiddenColumns } from '$lib/api';
	import LL from '$lib/i18n/i18n-svelte';

	export let hiddenColumns: string[] = [];
	export let onChange: (columns: string[]) => void;

	// Hideable columns configuration
	const hideableColumns = [
		{ id: 'tripNumber', labelKey: 'tripNumber' },
		{ id: 'startTime', labelKey: 'startTime' },
		{ id: 'endTime', labelKey: 'endTime' },
		{ id: 'driver', labelKey: 'driver' },
		{ id: 'odoStart', labelKey: 'odoStart' },
		{ id: 'time', labelKey: 'time' },
		{ id: 'fuelConsumed', labelKey: 'fuelConsumed' },
		{ id: 'fuelRemaining', labelKey: 'fuelRemaining' },
		{ id: 'otherCosts', labelKey: 'otherCosts' },
		{ id: 'otherCostsNote', labelKey: 'otherCostsNote' },
	] as const;

	let isOpen = false;

	function toggleDropdown() {
		isOpen = !isOpen;
	}

	function closeDropdown() {
		isOpen = false;
	}

	async function toggleColumn(columnId: string) {
		let newHiddenColumns: string[];
		if (hiddenColumns.includes(columnId)) {
			newHiddenColumns = hiddenColumns.filter(c => c !== columnId);
		} else {
			newHiddenColumns = [...hiddenColumns, columnId];
		}

		try {
			await setHiddenColumns(newHiddenColumns);
			onChange(newHiddenColumns);
		} catch (error) {
			console.error('Failed to save hidden columns:', error);
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			closeDropdown();
		}
	}

	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.column-visibility-dropdown')) {
			closeDropdown();
		}
	}

	$: hiddenCount = hiddenColumns.length;

	// Get label for column
	function getColumnLabel(labelKey: string): string {
		const labels = $LL.trips.columnVisibility;
		return (labels as Record<string, () => string>)[labelKey]?.() ?? labelKey;
	}
</script>

<svelte:window on:click={handleClickOutside} on:keydown={handleKeydown} />

<div class="column-visibility-dropdown">
	<button
		type="button"
		class="toggle-btn"
		class:has-hidden={hiddenCount > 0}
		on:click|stopPropagation={toggleDropdown}
		title={$LL.trips.columnVisibility.title()}
		data-testid="column-visibility-toggle"
	>
		<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
			<circle cx="12" cy="12" r="3"></circle>
		</svg>
		{#if hiddenCount > 0}
			<span class="badge">{hiddenCount}</span>
		{/if}
	</button>

	{#if isOpen}
		<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
		<div class="dropdown-menu" on:click|stopPropagation data-testid="column-visibility-menu">
			<div class="dropdown-header">{$LL.trips.columnVisibility.title()}</div>
			{#each hideableColumns as column}
				<label class="dropdown-item">
					<input
						type="checkbox"
						checked={!hiddenColumns.includes(column.id)}
						on:change={() => toggleColumn(column.id)}
						data-testid="column-toggle-{column.id}"
					/>
					<span class="checkmark"></span>
					<span class="label-text">{getColumnLabel(column.labelKey)}</span>
				</label>
			{/each}
		</div>
	{/if}
</div>

<style>
	.column-visibility-dropdown {
		position: relative;
		display: inline-block;
	}

	.toggle-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.25rem;
		padding: 0.5rem 0.75rem;
		background: var(--bg-surface-alt);
		border: 1px solid var(--border-default);
		border-radius: 6px;
		cursor: pointer;
		color: var(--text-secondary);
		transition: all 0.15s ease;
	}

	.toggle-btn:hover {
		background: var(--bg-surface);
		color: var(--text-primary);
		border-color: var(--border-input);
	}

	.toggle-btn.has-hidden {
		color: var(--accent-primary);
		border-color: var(--accent-primary);
	}

	.badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		background: var(--accent-primary);
		color: white;
		font-size: 0.75rem;
		font-weight: 600;
		border-radius: 9px;
	}

	.dropdown-menu {
		position: absolute;
		top: calc(100% + 4px);
		right: 0;
		min-width: 200px;
		background: var(--bg-surface);
		border: 1px solid var(--border-default);
		border-radius: 6px;
		box-shadow: 0 4px 12px var(--shadow-default);
		z-index: 1000;
		overflow: hidden;
	}

	.dropdown-header {
		padding: 0.625rem 0.875rem;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
		background: var(--bg-surface-alt);
		border-bottom: 1px solid var(--border-default);
	}

	.dropdown-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.625rem 0.875rem;
		cursor: pointer;
		transition: background-color 0.15s ease;
	}

	.dropdown-item:hover {
		background: var(--bg-surface-alt);
	}

	.dropdown-item input[type="checkbox"] {
		position: absolute;
		opacity: 0;
		cursor: pointer;
		height: 0;
		width: 0;
	}

	.checkmark {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 18px;
		height: 18px;
		border: 2px solid var(--border-input);
		border-radius: 4px;
		background: var(--bg-surface);
		transition: all 0.15s ease;
		flex-shrink: 0;
	}

	.dropdown-item input[type="checkbox"]:checked ~ .checkmark {
		background: var(--accent-primary);
		border-color: var(--accent-primary);
	}

	.dropdown-item input[type="checkbox"]:checked ~ .checkmark::after {
		content: '';
		display: block;
		width: 5px;
		height: 9px;
		border: solid white;
		border-width: 0 2px 2px 0;
		transform: rotate(45deg);
		margin-bottom: 2px;
	}

	.label-text {
		font-size: 0.875rem;
		color: var(--text-primary);
	}
</style>
