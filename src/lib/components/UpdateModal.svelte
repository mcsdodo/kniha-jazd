<script lang="ts">
	import { LL } from '$lib/i18n/i18n-svelte';

	type BackupStep = 'pending' | 'in-progress' | 'done' | 'failed' | 'skipped';

	export let version: string;
	export let releaseNotes: string | null;
	export let downloading: boolean = false;
	export let progress: number = 0;
	export let backupStep: BackupStep = 'pending';
	export let backupError: string | null = null;
	export let onUpdate: () => void;
	export let onLater: () => void;
	export let onContinueWithoutBackup: () => void;

	$: isUpdating = backupStep !== 'pending' || downloading;
	$: showBackupFailed = backupStep === 'failed';

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape' && !isUpdating) {
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

			{#if releaseNotes && !isUpdating}
				<div class="release-notes-section">
					<h3>{$LL.update.releaseNotes()}</h3>
					<div class="release-notes-content">
						{releaseNotes}
					</div>
				</div>
			{/if}

			{#if isUpdating}
				<div class="update-steps">
					<!-- Backup Step -->
					<div class="step" class:active={backupStep === 'in-progress'} class:done={backupStep === 'done' || backupStep === 'skipped'} class:failed={backupStep === 'failed'}>
						<span class="step-icon">
							{#if backupStep === 'in-progress'}⏳{:else if backupStep === 'done'}✓{:else if backupStep === 'skipped'}⏭{:else if backupStep === 'failed'}✗{:else}○{/if}
						</span>
						<span class="step-label">
							{#if backupStep === 'in-progress'}
								{$LL.update.backupInProgress()}
							{:else if backupStep === 'done'}
								{$LL.update.backupStep()}
							{:else if backupStep === 'skipped'}
								{$LL.update.backupStep()} (skipped)
							{:else if backupStep === 'failed'}
								{$LL.update.backupFailed()}
							{:else}
								{$LL.update.backupStep()}
							{/if}
						</span>
					</div>

					<!-- Download Step -->
					<div class="step" class:active={downloading} class:done={progress >= 100}>
						<span class="step-icon">
							{#if downloading && progress < 100}⏳{:else if progress >= 100}✓{:else}○{/if}
						</span>
						<span class="step-label">
							{#if downloading}
								{$LL.update.downloadProgress({ percent: progress.toFixed(0) })}
							{:else}
								{$LL.update.downloading()}
							{/if}
						</span>
					</div>

					<!-- Install Step -->
					<div class="step" class:active={progress >= 100 && !downloading}>
						<span class="step-icon">
							{#if progress >= 100}⏳{:else}○{/if}
						</span>
						<span class="step-label">{$LL.update.installing()}</span>
					</div>
				</div>

				{#if downloading}
					<div class="progress-section">
						<div class="progress-bar">
							<div class="progress-fill" style="width: {progress}%"></div>
						</div>
					</div>
				{/if}
			{/if}

			{#if showBackupFailed}
				<div class="backup-failed-warning">
					<p>{$LL.update.backupFailedMessage()}</p>
					{#if backupError}
						<p class="error-detail">{backupError}</p>
					{/if}
				</div>
			{/if}
		</div>

		<div class="modal-actions">
			{#if showBackupFailed}
				<button
					class="button-small"
					on:click={onLater}
				>
					{$LL.common.cancel()}
				</button>
				<button
					class="button-small accent-primary"
					on:click={onContinueWithoutBackup}
				>
					{$LL.update.continueWithoutBackup()}
				</button>
			{:else}
				<button
					class="button-small"
					on:click={onLater}
					disabled={isUpdating}
				>
					{$LL.update.buttonLater()}
				</button>
				<button
					class="button-small accent-primary"
					on:click={onUpdate}
					disabled={isUpdating}
				>
					{$LL.update.buttonUpdate()}
				</button>
			{/if}
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

	.update-steps {
		margin-top: 1rem;
		padding: 1rem;
		background: var(--bg-secondary);
		border-radius: 4px;
	}

	.step {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 0;
		color: var(--text-secondary);
	}

	.step.active {
		color: var(--text-primary);
		font-weight: 500;
	}

	.step.done {
		color: var(--success, #4caf50);
	}

	.step.failed {
		color: var(--danger, #f44336);
	}

	.step-icon {
		font-size: 1rem;
		width: 1.5rem;
		text-align: center;
	}

	.backup-failed-warning {
		margin-top: 1rem;
		padding: 1rem;
		background: var(--danger-light, #ffebee);
		border: 1px solid var(--danger, #f44336);
		border-radius: 4px;
	}

	.backup-failed-warning p {
		margin: 0;
		color: var(--danger, #f44336);
	}

	.error-detail {
		margin-top: 0.5rem !important;
		font-size: 0.875rem;
		opacity: 0.8;
	}

	.progress-section {
		margin-top: 1rem;
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
