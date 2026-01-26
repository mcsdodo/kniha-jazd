<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let options: { value: string; label: string }[];
	export let value: string;
	export let size: 'small' | 'default' = 'default';
	export let title: string = '';

	const dispatch = createEventDispatcher<{ change: string }>();

	function handleClick(optionValue: string) {
		if (optionValue !== value) {
			dispatch('change', optionValue);
		}
	}
</script>

<div class="segmented-toggle size-{size}" {title}>
	{#each options as option}
		<button
			type="button"
			class="toggle-option"
			class:active={value === option.value}
			on:click={() => handleClick(option.value)}
		>
			{option.label}
		</button>
	{/each}
</div>

<style>
	.segmented-toggle {
		display: inline-flex;
		border-radius: 6px;
		background: var(--bg-surface-alt);
		border: 1px solid var(--border-default);
		padding: 2px;
		gap: 2px;
	}

	.toggle-option {
		border: none;
		background: transparent;
		cursor: pointer;
		font-family: inherit;
		font-weight: 500;
		color: var(--text-secondary);
		border-radius: 4px;
		transition: all 0.15s ease;
	}

	.toggle-option:hover:not(.active) {
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	.toggle-option.active {
		background: var(--accent-primary);
		color: white;
		box-shadow: 0 1px 2px var(--shadow-default);
	}

	/* Size variants */
	.size-default .toggle-option {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
	}

	.size-small .toggle-option {
		padding: 0.25rem 0.625rem;
		font-size: 0.75rem;
	}
</style>
