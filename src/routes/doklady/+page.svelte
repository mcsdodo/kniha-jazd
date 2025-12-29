<script lang="ts">
	import { onMount } from 'svelte';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Receipt, ReceiptSettings } from '$lib/types';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';

	let receipts = $state<Receipt[]>([]);
	let settings = $state<ReceiptSettings | null>(null);
	let loading = $state(true);
	let syncing = $state(false);
	let filter = $state<'all' | 'unassigned' | 'needs_review'>('all');
	let receiptToDelete = $state<Receipt | null>(null);

	onMount(async () => {
		await loadSettings();
		await loadReceipts();
	});

	async function loadSettings() {
		try {
			settings = await api.getReceiptSettings();
		} catch (error) {
			console.error('Failed to load receipt settings:', error);
		}
	}

	async function loadReceipts() {
		loading = true;
		try {
			receipts = await api.getReceipts();
		} catch (error) {
			console.error('Failed to load receipts:', error);
			toast.error('Nepodarilo sa nacitat doklady');
		} finally {
			loading = false;
		}
	}

	async function handleSync() {
		if (!settings?.gemini_api_key || !settings?.receipts_folder_path) {
			toast.error('Najprv nastavte priecinok a API kluc v Nastaveniach');
			return;
		}

		syncing = true;
		try {
			const result = await api.syncReceipts();
			await loadReceipts();

			if (result.processed.length > 0) {
				if (result.errors.length > 0) {
					toast.success(`Nacitanych ${result.processed.length} dokladov (${result.errors.length} chyb)`);
				} else {
					toast.success(`Nacitanych ${result.processed.length} novych dokladov`);
				}
			} else {
				toast.success('Ziadne nove doklady');
			}
		} catch (error) {
			console.error('Failed to sync receipts:', error);
			toast.error('Nepodarilo sa synchronizovat: ' + error);
		} finally {
			syncing = false;
		}
	}

	function handleDeleteClick(receipt: Receipt) {
		receiptToDelete = receipt;
	}

	async function handleConfirmDelete() {
		if (!receiptToDelete) return;
		try {
			await api.deleteReceipt(receiptToDelete.id);
			await loadReceipts();
			toast.success('Doklad bol odstraneny');
		} catch (error) {
			console.error('Failed to delete receipt:', error);
			toast.error('Nepodarilo sa odstranit doklad');
		} finally {
			receiptToDelete = null;
		}
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return '--';
		try {
			const date = new Date(dateStr);
			return date.toLocaleDateString('sk-SK');
		} catch {
			return dateStr;
		}
	}

	function getStatusBadge(status: string): { text: string; class: string } {
		switch (status) {
			case 'Parsed':
				return { text: 'Spracovany', class: 'success' };
			case 'NeedsReview':
				return { text: 'Na kontrolu', class: 'warning' };
			case 'Assigned':
				return { text: 'Prideleny', class: 'info' };
			case 'Pending':
			default:
				return { text: 'Caka', class: 'neutral' };
		}
	}

	// Svelte 5: use $derived instead of $:
	let filteredReceipts = $derived(
		receipts.filter((r) => {
			if (filter === 'unassigned') return r.status !== 'Assigned';
			if (filter === 'needs_review') return r.status === 'NeedsReview';
			return true;
		})
	);

	let isConfigured = $derived(settings?.gemini_api_key && settings?.receipts_folder_path);
</script>

<div class="doklady-page">
	<div class="header">
		<h1>Doklady</h1>
		<div class="header-actions">
			<button class="button" onclick={handleSync} disabled={syncing || !isConfigured}>
				{syncing ? 'Synchronizujem...' : 'Sync'}
			</button>
		</div>
	</div>

	{#if !isConfigured}
		<div class="config-warning">
			<p>Funkcia dokladov nie je nakongurovana.</p>
			<p>
				Nastavte <strong>priecinok s dokladmi</strong> a <strong>Gemini API kluc</strong> v subore
				<code>local.settings.json</code> v priecinku aplikacie.
			</p>
		</div>
	{/if}

	<div class="filters">
		<button class="filter-btn" class:active={filter === 'all'} onclick={() => (filter = 'all')}>
			Vsetky ({receipts.length})
		</button>
		<button
			class="filter-btn"
			class:active={filter === 'unassigned'}
			onclick={() => (filter = 'unassigned')}
		>
			Nepridelene ({receipts.filter((r) => r.status !== 'Assigned').length})
		</button>
		<button
			class="filter-btn"
			class:active={filter === 'needs_review'}
			onclick={() => (filter = 'needs_review')}
		>
			Na kontrolu ({receipts.filter((r) => r.status === 'NeedsReview').length})
		</button>
	</div>

	{#if loading}
		<p class="placeholder">Nacitavam...</p>
	{:else if filteredReceipts.length === 0}
		<p class="placeholder">Ziadne doklady. Kliknite na Sync pre nacitanie novych.</p>
	{:else}
		<div class="receipts-list">
			{#each filteredReceipts as receipt}
				{@const badge = getStatusBadge(receipt.status)}
				<div class="receipt-card">
					<div class="receipt-header">
						<span class="file-name">{receipt.file_name}</span>
						<span class="badge {badge.class}">{badge.text}</span>
					</div>
					<div class="receipt-details">
						<div class="detail-row">
							<span class="label">Datum:</span>
							<span class="value">{formatDate(receipt.receipt_date)}</span>
						</div>
						<div class="detail-row">
							<span class="label">Litre:</span>
							<span class="value" class:uncertain={receipt.confidence.liters === 'Low'}>
								{receipt.liters != null ? `${receipt.liters.toFixed(2)} L` : '??'}
							</span>
						</div>
						<div class="detail-row">
							<span class="label">Cena:</span>
							<span class="value" class:uncertain={receipt.confidence.total_price === 'Low'}>
								{receipt.total_price_eur != null ? `${receipt.total_price_eur.toFixed(2)} EUR` : '??'}
							</span>
						</div>
						{#if receipt.station_name}
							<div class="detail-row">
								<span class="label">Stanica:</span>
								<span class="value">{receipt.station_name}</span>
							</div>
						{/if}
						{#if receipt.error_message}
							<div class="error-message">{receipt.error_message}</div>
						{/if}
					</div>
					<div class="receipt-actions">
						{#if receipt.status !== 'Assigned'}
							<button class="button-small">Pridelit k jazde</button>
						{/if}
						<button class="button-small danger" onclick={() => handleDeleteClick(receipt)}>
							Zmazat
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

{#if receiptToDelete}
	<ConfirmModal
		title="Odstranit doklad"
		message={`Naozaj chcete odstranit doklad "${receiptToDelete.file_name}"?`}
		confirmText="Odstranit"
		danger={true}
		onConfirm={handleConfirmDelete}
		onCancel={() => (receiptToDelete = null)}
	/>
{/if}

<style>
	.doklady-page {
		max-width: 800px;
		margin: 0 auto;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.header h1 {
		margin: 0;
		color: #2c3e50;
	}

	.config-warning {
		background: #fff3cd;
		border: 1px solid #ffc107;
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}

	.config-warning p {
		margin: 0.5rem 0;
	}

	.config-warning code {
		background: #f8f9fa;
		padding: 0.2rem 0.4rem;
		border-radius: 3px;
		font-size: 0.875rem;
	}

	.filters {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.filter-btn {
		padding: 0.5rem 1rem;
		border: 1px solid #ddd;
		background: white;
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.filter-btn:hover {
		background: #f5f5f5;
	}

	.filter-btn.active {
		background: #3498db;
		color: white;
		border-color: #3498db;
	}

	.receipts-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.receipt-card {
		background: white;
		border-radius: 8px;
		padding: 1rem;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.receipt-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
	}

	.file-name {
		font-weight: 500;
		color: #2c3e50;
	}

	.badge {
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-weight: 600;
	}

	.badge.success {
		background: #d4edda;
		color: #155724;
	}

	.badge.warning {
		background: #fff3cd;
		color: #856404;
	}

	.badge.info {
		background: #cce5ff;
		color: #004085;
	}

	.badge.neutral {
		background: #e9ecef;
		color: #495057;
	}

	.receipt-details {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.detail-row {
		display: flex;
		gap: 0.5rem;
	}

	.label {
		color: #7f8c8d;
		font-size: 0.875rem;
	}

	.value {
		font-weight: 500;
		color: #2c3e50;
	}

	.value.uncertain {
		color: #e67e22;
	}

	.error-message {
		grid-column: 1 / -1;
		color: #c0392b;
		font-size: 0.875rem;
		padding: 0.5rem;
		background: #fee;
		border-radius: 4px;
	}

	.receipt-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.placeholder {
		color: #7f8c8d;
		font-style: italic;
		text-align: center;
		padding: 2rem;
	}

	.button {
		padding: 0.75rem 1.5rem;
		background-color: #3498db;
		color: white;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button:hover:not(:disabled) {
		background-color: #2980b9;
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.button-small {
		padding: 0.5rem 1rem;
		background-color: #ecf0f1;
		color: #2c3e50;
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.button-small:hover {
		background-color: #d5dbdb;
	}

	.button-small.danger {
		background-color: #fee;
		color: #c0392b;
	}

	.button-small.danger:hover {
		background-color: #fdd;
	}
</style>
