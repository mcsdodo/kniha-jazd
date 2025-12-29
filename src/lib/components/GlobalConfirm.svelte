<script lang="ts">
	import { confirmStore } from '$lib/stores/confirm';
	import ConfirmModal from './ConfirmModal.svelte';
</script>

{#if $confirmStore.isOpen}
	<ConfirmModal
		title={$confirmStore.title}
		message={$confirmStore.message}
		confirmText={$confirmStore.confirmText}
		danger={$confirmStore.danger}
		onConfirm={confirmStore.confirm}
		onCancel={confirmStore.close}
	/>
{/if}
