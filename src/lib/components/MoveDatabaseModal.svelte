<script lang="ts">
	import { LL } from '$lib/i18n/i18n-svelte';

	export let targetPath: string;
	export let moving: boolean = false;
	export let onConfirm: () => void;
	export let onCancel: () => void;

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !moving) {
			onCancel();
		}
	}
</script>

<div
	class="modal-overlay"
	on:click={() => !moving && onCancel()}
	on:keydown={handleKeydown}
	role="button"
	tabindex="0"
>
	<div
		class="modal"
		on:click|stopPropagation
		on:keydown={() => {}}
		role="dialog"
		aria-modal="true"
		aria-labelledby="move-db-title"
		tabindex="-1"
	>
		<h2 id="move-db-title">{$LL.settings.dbLocationConfirmTitle()}</h2>
		<div class="modal-content">
			<p>{$LL.settings.dbLocationConfirmMessage()}</p>

			<div class="target-path-section">
				<code class="target-path">{targetPath}</code>
			</div>

			{#if moving}
				<div class="progress-section">
					<div class="progress-bar">
						<div class="progress-fill indeterminate"></div>
					</div>
					<p class="progress-text">{$LL.settings.dbLocationMoving()}</p>
				</div>
			{:else}
				<div class="warning-section">
					<p class="warning-text">
						<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
						{$LL.settings.dbLocationConfirmWarning()}
					</p>
				</div>
			{/if}
		</div>

		<div class="modal-actions">
			<button
				class="button-small"
				on:click={onCancel}
				disabled={moving}
			>
				{$LL.common.cancel()}
			</button>
			<button
				class="button-small accent-primary"
				on:click={onConfirm}
				disabled={moving}
			>
				{$LL.settings.dbLocationConfirmMove()}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: var(--bg-surface);
		padding: 1.5rem;
		border-radius: 8px;
		max-width: 500px;
		width: 90%;
	}

	.modal h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.modal-content {
		margin-bottom: 1.5rem;
	}

	.modal-content p {
		margin: 0.5rem 0;
		color: var(--text-primary);
	}

	.target-path-section {
		margin: 1rem 0;
		padding: 0.75rem;
		background: var(--bg-surface-alt);
		border-radius: 4px;
		border: 1px solid var(--border-default);
	}

	.target-path {
		font-family: monospace;
		font-size: 0.875rem;
		color: var(--text-primary);
		word-break: break-all;
	}

	.warning-section {
		margin-top: 1rem;
		padding: 0.75rem;
		background: var(--accent-warning-bg, rgba(251, 191, 36, 0.1));
		border-radius: 4px;
		border: 1px solid var(--accent-warning, #f59e0b);
	}

	.warning-text {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin: 0 !important;
		color: var(--accent-warning, #f59e0b);
		font-weight: 500;
		font-size: 0.875rem;
	}

	.warning-text svg {
		flex-shrink: 0;
	}

	.progress-section {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-default);
	}

	.progress-bar {
		width: 100%;
		height: 4px;
		background: var(--bg-surface-alt);
		border-radius: 2px;
		overflow: hidden;
		margin-bottom: 0.5rem;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent-primary);
		border-radius: 2px;
	}

	.progress-fill.indeterminate {
		width: 30%;
		animation: indeterminate 1.5s ease-in-out infinite;
	}

	@keyframes indeterminate {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(400%);
		}
	}

	.progress-text {
		margin: 0.5rem 0 0 0;
		font-size: 0.875rem;
		color: var(--text-secondary);
		text-align: center;
	}

	.modal-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.button-small {
		padding: 0.5rem 1rem;
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button-small:hover:not(:disabled) {
		background-color: var(--btn-secondary-hover);
	}

	.button-small.accent-primary {
		background-color: var(--accent-primary-bg);
		color: var(--accent-primary);
	}

	.button-small.accent-primary:hover:not(:disabled) {
		background-color: var(--accent-primary-bg-hover);
	}

	.button-small:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
