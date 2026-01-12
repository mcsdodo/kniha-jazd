<script lang="ts">
	export let title: string = 'Potvrdiť';
	export let message: string;
	export let confirmText: string = 'Potvrdiť';
	export let cancelText: string = 'Zrušiť';
	export let danger: boolean = false;
	export let onConfirm: () => void;
	export let onCancel: () => void;
</script>

<div
	class="modal-overlay"
	on:click={onCancel}
	on:keydown={(e) => e.key === 'Escape' && onCancel()}
	role="button"
	tabindex="0"
>
	<div
		class="modal"
		on:click|stopPropagation
		on:keydown={() => {}}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<h2>{title}</h2>
		<div class="modal-content">
			<p>{message}</p>
		</div>
		<div class="modal-actions">
			<button class="button-small" on:click={onCancel}>{cancelText}</button>
			<button class="button-small" class:danger on:click={onConfirm}>{confirmText}</button>
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
		max-width: 400px;
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

	.button-small:hover {
		background-color: var(--btn-secondary-hover);
	}

	.button-small.danger {
		background-color: var(--accent-danger-bg);
		color: var(--accent-danger);
	}

	.button-small.danger:hover {
		background-color: var(--accent-danger-bg-hover);
	}
</style>
