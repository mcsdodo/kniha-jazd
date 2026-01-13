<script lang="ts">
	import { LL } from '$lib/i18n/i18n-svelte';

	export let version: string;
	export let releaseNotes: string | null;
	export let downloading: boolean = false;
	export let progress: number = 0;
	export let onUpdate: () => void;
	export let onLater: () => void;

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onLater();
		}
	}
</script>

<div
	class="modal-overlay"
	on:click={onLater}
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
		aria-labelledby="update-title"
		tabindex="-1"
	>
		<h2 id="update-title">{$LL.update.modalTitle()}</h2>
		<div class="modal-content">
			<p>{$LL.update.modalBody({ version })}</p>

			{#if releaseNotes}
				<div class="release-notes-section">
					<h3>{$LL.update.releaseNotes()}</h3>
					<div class="release-notes-content">
						{releaseNotes}
					</div>
				</div>
			{/if}

			{#if downloading}
				<div class="progress-section">
					<div class="progress-bar">
						<div class="progress-fill" style="width: {progress}%"></div>
					</div>
					<p class="progress-text">
						{$LL.update.downloadProgress({ percent: progress.toFixed(0) })}
					</p>
				</div>
			{/if}
		</div>

		<div class="modal-actions">
			<button
				class="button-small"
				on:click={onLater}
				disabled={downloading}
			>
				{$LL.update.buttonLater()}
			</button>
			<button
				class="button-small accent-primary"
				on:click={onUpdate}
				disabled={downloading}
			>
				{$LL.update.buttonUpdate()}
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
		max-height: 80vh;
		overflow-y: auto;
	}

	.modal h2 {
		margin: 0 0 1rem 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.modal h3 {
		margin: 1rem 0 0.5rem 0;
		font-size: 0.95rem;
		color: var(--text-primary);
		font-weight: 600;
	}

	.modal-content {
		margin-bottom: 1.5rem;
	}

	.modal-content p {
		margin: 0.5rem 0;
		color: var(--text-primary);
	}

	.release-notes-section {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-color);
	}

	.release-notes-content {
		white-space: pre-wrap;
		word-wrap: break-word;
		padding: 0.75rem;
		background: var(--bg-secondary);
		border-radius: 4px;
		font-size: 0.875rem;
		color: var(--text-secondary);
		max-height: 250px;
		overflow-y: auto;
	}

	.progress-section {
		margin-top: 1rem;
		padding-top: 1rem;
		border-top: 1px solid var(--border-color);
	}

	.progress-bar {
		width: 100%;
		height: 4px;
		background: var(--bg-secondary);
		border-radius: 2px;
		overflow: hidden;
		margin-bottom: 0.5rem;
	}

	.progress-fill {
		height: 100%;
		background: var(--accent-primary);
		border-radius: 2px;
		transition: width 0.3s ease;
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
