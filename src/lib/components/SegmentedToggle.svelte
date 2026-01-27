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
		border-radius: 4px;
		background: transparent;
		border: none;
		padding: 0;
		gap: 0;
	}

	.toggle-option {
		border: none;
		background: transparent;
		cursor: pointer;
		font-family: inherit;
		font-weight: 500;
		color: var(--text-secondary);
		border-radius: 0;
		transition: all 0.15s ease;
		border: 1px solid var(--border-default);
		margin-left: -1px;
	}

	.toggle-option:first-child {
		border-radius: 4px 0 0 4px;
		margin-left: 0;
	}

	.toggle-option:last-child {
		border-radius: 0 4px 4px 0;
	}

	.toggle-option:hover:not(.active) {
		background: var(--bg-surface-alt);
		color: var(--text-primary);
	}

	.toggle-option.active {
		background: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		border-color: var(--btn-active-primary-bg);
		z-index: 1;
		position: relative;
	}

	/* Size variants */
	.size-default .toggle-option {
		padding: 0.5rem 1rem;
		font-size: 0.875rem;
	}

	.size-small .toggle-option {
		padding: 0.625rem 1rem;
		font-size: 0.875rem;
	}
</style>
