<script lang="ts">
	import { toast } from '$lib/stores/toast';
	import { fly } from 'svelte/transition';
</script>

<div class="toast-container">
	{#each $toast as t (t.id)}
		<!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
		<div
			class="toast toast-{t.type}"
			transition:fly={{ y: -20, duration: 200 }}
			on:click={() => toast.dismiss(t.id)}
			on:keydown={(e) => e.key === 'Enter' && toast.dismiss(t.id)}
			role="alert"
			tabindex="0"
		>
			<span class="toast-icon">
				{#if t.type === 'success'}
					✓
				{:else if t.type === 'error'}
					✗
				{:else}
					ℹ
				{/if}
			</span>
			<span class="toast-message">{t.message}</span>
		</div>
	{/each}
</div>

<style>
	.toast-container {
		position: fixed;
		top: 1rem;
		right: 1rem;
		z-index: 9999;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-width: 400px;
	}

	.toast {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.875rem 1rem;
		border-radius: 6px;
		box-shadow: 0 4px 12px var(--shadow-default);
		cursor: pointer;
		font-size: 0.9rem;
	}

	.toast-success {
		background: var(--toast-success-bg);
		color: var(--toast-success-color);
		border: 1px solid var(--toast-success-border);
	}

	.toast-error {
		background: var(--toast-error-bg);
		color: var(--toast-error-color);
		border: 1px solid var(--toast-error-border);
	}

	.toast-info {
		background: var(--toast-info-bg);
		color: var(--toast-info-color);
		border: 1px solid var(--toast-info-border);
	}

	.toast-icon {
		font-size: 1.1rem;
		font-weight: bold;
	}

	.toast-message {
		flex: 1;
	}
</style>
