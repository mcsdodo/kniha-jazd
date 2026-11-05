<script lang="ts">
	// The small top bar of the phone pages (Task 93): an optional back link
	// or button, a title, and an optional slot on the right.
	import type { Snippet } from 'svelte';

	let {
		title,
		subtitle = '',
		backHref = '',
		backLabel = '',
		onback,
		right
	}: {
		title: string;
		subtitle?: string;
		backHref?: string;
		backLabel?: string;
		onback?: () => void;
		right?: Snippet;
	} = $props();
</script>

<header class="bar" data-testid="mobile-bar">
	{#if onback}
		<button type="button" class="back" onclick={onback} data-testid="mobile-back">&lt; {backLabel}</button>
	{:else if backHref}
		<a class="back" href={backHref} data-testid="mobile-back">&lt; {backLabel}</a>
	{/if}
	<div class="titles">
		<span class="title">{title}</span>
		{#if subtitle}<small class="subtitle" data-testid="mobile-bar-subtitle">{subtitle}</small>{/if}
	</div>
	{#if right}<div class="right">{@render right()}</div>{/if}
</header>

<style>
	.bar {
		position: sticky;
		top: 0;
		z-index: 5;
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 52px;
		padding: 0.25rem 1rem;
		background: var(--bg-surface);
		border-bottom: 1px solid var(--border-default);
		box-sizing: border-box;
	}

	.back {
		display: inline-flex;
		align-items: center;
		min-height: 44px;
		padding: 0 0.25rem;
		margin-left: -0.25rem;
		border: none;
		background: none;
		color: var(--accent-primary);
		font: inherit;
		font-weight: 500;
		text-decoration: none;
		cursor: pointer;
		white-space: nowrap;
	}

	.titles {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		justify-content: space-between;
		gap: 0 0.5rem;
	}

	.title {
		font-weight: 600;
		font-size: 1.05rem;
	}

	.subtitle {
		color: var(--text-secondary);
		font-size: 0.85rem;
	}

	.right {
		flex: none;
	}
</style>
