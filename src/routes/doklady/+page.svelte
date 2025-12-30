<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Receipt, ReceiptSettings, ConfidenceLevel, Trip, VerificationResult, ReceiptVerification } from '$lib/types';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import TripSelectorModal from '$lib/components/TripSelectorModal.svelte';
	import { openPath } from '@tauri-apps/plugin-opener';
	import { appDataDir } from '@tauri-apps/api/path';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import LL from '$lib/i18n/i18n-svelte';

	interface ProcessingProgress {
		current: number;
		total: number;
		file_name: string;
	}

	let receipts = $state<Receipt[]>([]);
	let settings = $state<ReceiptSettings | null>(null);
	let loading = $state(true);
	let syncing = $state(false);
	let processing = $state(false);
	let processingProgress = $state<ProcessingProgress | null>(null);
	let filter = $state<'all' | 'unassigned' | 'needs_review'>('all');
	let receiptToDelete = $state<Receipt | null>(null);
	let reprocessingIds = $state<Set<string>>(new Set());
	let receiptToAssign = $state<Receipt | null>(null);
	let verification = $state<VerificationResult | null>(null);
	let configFolderPath = $state<string>('');
	let folderStructureWarning = $state<string | null>(null);

	let unlistenProgress: UnlistenFn | null = null;

	onMount(async () => {
		// Listen for processing progress events
		unlistenProgress = await listen<ProcessingProgress>('receipt-processing-progress', (event) => {
			processingProgress = event.payload;
		});

		// Get app data directory for config file location
		configFolderPath = await appDataDir();

		await loadSettings();
		await loadReceipts();
		await loadVerification();
	});

	onDestroy(() => {
		if (unlistenProgress) {
			unlistenProgress();
		}
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
			receipts = await api.getReceipts($selectedYearStore);
		} catch (error) {
			console.error('Failed to load receipts:', error);
			toast.error($LL.toast.errorLoadReceipts());
		} finally {
			loading = false;
		}
	}

	async function loadVerification() {
		const vehicle = $activeVehicleStore;
		if (!vehicle) return;

		try {
			verification = await api.verifyReceipts(vehicle.id, $selectedYearStore);
		} catch (error) {
			console.error('Failed to verify receipts:', error);
		}
	}

	function getVerificationForReceipt(receiptId: string): ReceiptVerification | null {
		return verification?.receipts.find(v => v.receipt_id === receiptId) ?? null;
	}

	async function handleSync() {
		if (!settings?.gemini_api_key || !settings?.receipts_folder_path) {
			toast.error($LL.toast.errorSetApiKeyFirst());
			return;
		}

		syncing = true;
		try {
			const result = await api.syncReceipts();
			await loadReceipts();
			await loadVerification();

			// Handle folder structure warning
			folderStructureWarning = result.warning;

			if (result.processed.length > 0) {
				if (result.errors.length > 0) {
					toast.success($LL.toast.receiptsLoadedWithErrors({ count: result.processed.length, errors: result.errors.length }));
				} else {
					toast.success($LL.toast.receiptsLoaded({ count: result.processed.length }));
				}
			} else {
				toast.success($LL.toast.noNewReceipts());
			}
		} catch (error) {
			console.error('Failed to sync receipts:', error);
			toast.error($LL.toast.errorSyncReceipts({ error: String(error) }));
		} finally {
			syncing = false;
		}
	}

	async function handleProcessPending() {
		if (!settings?.gemini_api_key) {
			toast.error($LL.toast.errorSetApiKeyOnlyFirst());
			return;
		}

		processing = true;
		processingProgress = null;
		try {
			const result = await api.processPendingReceipts();
			await loadReceipts();

			if (result.processed.length > 0) {
				if (result.errors.length > 0) {
					toast.success($LL.toast.receiptsProcessedWithErrors({ count: result.processed.length, errors: result.errors.length }));
				} else {
					toast.success($LL.toast.receiptsProcessed({ count: result.processed.length }));
				}
			} else {
				toast.success($LL.toast.noPendingReceipts());
			}
		} catch (error) {
			console.error('Failed to process pending receipts:', error);
			toast.error($LL.toast.errorProcessReceipts({ error: String(error) }));
		} finally {
			processing = false;
			processingProgress = null;
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
			toast.success($LL.toast.receiptDeleted());
		} catch (error) {
			console.error('Failed to delete receipt:', error);
			toast.error($LL.toast.errorDeleteReceipt());
		} finally {
			receiptToDelete = null;
		}
	}

	async function handleReprocess(receipt: Receipt) {
		reprocessingIds = new Set([...reprocessingIds, receipt.id]);
		try {
			await api.reprocessReceipt(receipt.id);
			await loadReceipts();
			toast.success($LL.toast.receiptReprocessed({ name: receipt.file_name }));
		} catch (error) {
			console.error('Failed to reprocess receipt:', error);
			toast.error($LL.toast.errorReprocessReceipt({ name: receipt.file_name, error: String(error) }));
		} finally {
			reprocessingIds = new Set([...reprocessingIds].filter((id) => id !== receipt.id));
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

	function getConfidenceInfo(level: ConfidenceLevel): { class: string; label: string } {
		switch (level) {
			case 'High':
				return { class: 'confidence-high', label: $LL.receipts.confidenceHigh() };
			case 'Medium':
				return { class: 'confidence-medium', label: $LL.receipts.confidenceMedium() };
			case 'Low':
				return { class: 'confidence-low', label: $LL.receipts.confidenceLow() };
			default:
				return { class: 'confidence-unknown', label: $LL.receipts.confidenceUnknown() };
		}
	}

	/**
	 * Check if a receipt has a date mismatch between source_year (folder) and receipt_date (OCR).
	 * Returns null if no mismatch, or { receiptYear, folderYear } if there's a mismatch.
	 */
	function getDateMismatch(receipt: Receipt): { receiptYear: number; folderYear: number } | null {
		if (!receipt.receipt_date || !receipt.source_year) {
			return null;
		}
		const receiptYear = new Date(receipt.receipt_date).getFullYear();
		if (receiptYear !== receipt.source_year) {
			return { receiptYear, folderYear: receipt.source_year };
		}
		return null;
	}

	async function handleOpenFile(filePath: string) {
		try {
			await openPath(filePath);
		} catch (error) {
			console.error('Failed to open file:', error);
			toast.error($LL.toast.errorOpenFile());
		}
	}

	function handleAssignClick(receipt: Receipt) {
		if (!$activeVehicleStore) {
			toast.error($LL.toast.errorSelectVehicleFirst());
			return;
		}
		receiptToAssign = receipt;
	}

	async function handleAssignToTrip(trip: Trip) {
		if (!receiptToAssign || !$activeVehicleStore) return;

		try {
			// Assign receipt to trip
			await api.assignReceiptToTrip(receiptToAssign.id, trip.id, $activeVehicleStore.id);

			// Update trip with fuel data from receipt if available
			if (receiptToAssign.liters != null || receiptToAssign.total_price_eur != null) {
				await api.updateTrip(
					trip.id,
					trip.date,
					trip.origin,
					trip.destination,
					trip.distance_km,
					trip.odometer,
					trip.purpose,
					receiptToAssign.liters,
					receiptToAssign.total_price_eur,
					trip.other_costs_eur ?? null,
					trip.other_costs_note ?? null,
					trip.full_tank
				);
			}

			await loadReceipts();
			receiptToAssign = null;
			toast.success($LL.toast.receiptAssigned());
		} catch (error) {
			console.error('Failed to assign receipt:', error);
			toast.error($LL.toast.errorAssignReceipt({ error: String(error) }));
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
	let pendingCount = $derived(receipts.filter((r) => r.status === 'Pending').length);
</script>

<div class="doklady-page">
	<div class="header">
		<h1>{$LL.receipts.title()}</h1>
		<div class="header-actions">
			<button class="button" onclick={handleSync} disabled={syncing || processing || !isConfigured}>
				{syncing ? $LL.receipts.syncing() : $LL.receipts.sync()}
			</button>
			{#if pendingCount > 0}
				<button
					class="button secondary"
					onclick={handleProcessPending}
					disabled={processing || syncing || !settings?.gemini_api_key}
				>
					{#if processing && processingProgress}
						{$LL.receipts.processingProgress({ current: processingProgress.current, total: processingProgress.total })}
					{:else if processing}
						{$LL.receipts.processing()}
					{:else}
						{$LL.receipts.processPending({ count: pendingCount })}
					{/if}
				</button>
			{/if}
		</div>
	</div>

	{#if !isConfigured}
		<div class="config-warning">
			<p>{$LL.receipts.notConfigured()}</p>
			<p>{$LL.receipts.configurePrompt()} <code class="filename">{$LL.receipts.configurePromptFile()}</code> {$LL.receipts.configurePromptSuffix()}</p>
			<pre class="config-sample">{`{
    "gemini_api_key": "YOUR_API_KEY_HERE",
    "receipts_folder_path": "C:\\\\Users\\\\YourUsername\\\\Documents\\\\Receipts"
}`}</pre>
			<p class="config-note">{$LL.receipts.configNote()}</p>
			<button class="config-path-btn" onclick={() => openPath(configFolderPath)}>
				<code>{configFolderPath}</code>
				<span class="open-icon">📂 {$LL.receipts.openConfigFolder()}</span>
			</button>
		</div>
	{/if}

	{#if folderStructureWarning}
		<div class="folder-structure-warning">
			<div class="warning-title">{$LL.receipts.folderStructureWarning()}</div>
			<div class="warning-details">{folderStructureWarning}</div>
			<div class="warning-hint">{$LL.receipts.folderStructureHint()}</div>
		</div>
	{/if}

	<div class="filters">
		<button class="filter-btn" class:active={filter === 'all'} onclick={() => (filter = 'all')}>
			{$LL.receipts.filterAll()} ({receipts.length})
		</button>
		<button
			class="filter-btn"
			class:active={filter === 'unassigned'}
			onclick={() => (filter = 'unassigned')}
		>
			{$LL.receipts.filterUnassigned()} ({verification?.unmatched ?? receipts.filter((r) => r.status !== 'Assigned').length})
		</button>
		<button
			class="filter-btn"
			class:active={filter === 'needs_review'}
			onclick={() => (filter = 'needs_review')}
		>
			{$LL.receipts.filterNeedsReview()} ({receipts.filter((r) => r.status === 'NeedsReview').length})
		</button>
	</div>

	{#if verification}
		<div class="verification-summary" class:all-matched={verification.unmatched === 0}>
			{#if verification.unmatched === 0}
				<span class="status-ok">✓ {$LL.receipts.allVerified({ count: verification.matched, total: verification.total })}</span>
			{:else}
				<span class="status-ok">✓ {$LL.receipts.verified({ count: verification.matched, total: verification.total })}</span>
				<span class="status-warning">⚠ {$LL.receipts.unverified({ count: verification.unmatched })}</span>
			{/if}
		</div>
	{/if}

	{#if loading}
		<p class="placeholder">{$LL.common.loading()}</p>
	{:else if filteredReceipts.length === 0}
		<p class="placeholder">{$LL.receipts.noReceipts()}</p>
	{:else}
		<div class="receipts-list">
			{#each filteredReceipts as receipt}
				{@const verif = getVerificationForReceipt(receipt.id)}
				{@const dateMismatch = getDateMismatch(receipt)}
				<div class="receipt-card">
					<div class="receipt-header">
						<span class="file-name">{receipt.file_name}</span>
						{#if verif?.matched}
							<span class="badge success">{$LL.receipts.statusVerified()}</span>
						{:else if receipt.status === 'NeedsReview'}
							<span class="badge warning">{$LL.receipts.statusNeedsReview()}</span>
						{:else}
							<span class="badge danger">{$LL.receipts.statusUnverified()}</span>
						{/if}
					</div>
					<div class="receipt-details">
						<div class="detail-row">
							<span class="label">{$LL.receipts.date()}</span>
							<span class="value-with-confidence">
								<span class="value">{formatDate(receipt.receipt_date)}</span>
								<span
									class="confidence-dot {getConfidenceInfo(receipt.confidence.date).class}"
									title={getConfidenceInfo(receipt.confidence.date).label}
								></span>
								{#if dateMismatch}
									<span
										class="date-mismatch-icon"
										title={$LL.receipts.dateMismatch({ receiptYear: dateMismatch.receiptYear, folderYear: dateMismatch.folderYear })}
									>⚠</span>
								{/if}
							</span>
						</div>
						<div class="detail-row">
							<span class="label">{$LL.receipts.liters()}</span>
							<span class="value-with-confidence">
								<span class="value" class:uncertain={receipt.confidence.liters === 'Low'}>
									{receipt.liters != null ? `${receipt.liters.toFixed(2)} L` : '??'}
								</span>
								<span
									class="confidence-dot {getConfidenceInfo(receipt.confidence.liters).class}"
									title={getConfidenceInfo(receipt.confidence.liters).label}
								></span>
							</span>
						</div>
						<div class="detail-row">
							<span class="label">{$LL.receipts.price()}</span>
							<span class="value-with-confidence">
								<span class="value" class:uncertain={receipt.confidence.total_price === 'Low'}>
									{receipt.total_price_eur != null ? `${receipt.total_price_eur.toFixed(2)} €` : '??'}
								</span>
								<span
									class="confidence-dot {getConfidenceInfo(receipt.confidence.total_price).class}"
									title={getConfidenceInfo(receipt.confidence.total_price).label}
								></span>
							</span>
						</div>
						{#if receipt.station_name}
							<div class="detail-row">
								<span class="label">{$LL.receipts.station()}</span>
								<span class="value">{receipt.station_name}</span>
							</div>
						{/if}
						{#if receipt.error_message}
							<div class="error-message">{receipt.error_message}</div>
						{/if}
						{#if verif?.matched}
							<div class="matched-trip">
								{$LL.receipts.trip()} {verif.matched_trip_date} | {verif.matched_trip_route}
							</div>
						{/if}
					</div>
					<div class="receipt-actions">
						<button class="button-small" onclick={() => handleOpenFile(receipt.file_path)}>
							{$LL.receipts.open()}
						</button>
						{#if !verif?.matched}
							<button
								class="button-small"
								onclick={() => handleReprocess(receipt)}
								disabled={reprocessingIds.has(receipt.id)}
							>
								{reprocessingIds.has(receipt.id) ? $LL.receipts.reprocessing() : $LL.receipts.reprocess()}
							</button>
							<button class="button-small" onclick={() => handleAssignClick(receipt)}>{$LL.receipts.assignToTrip()}</button>
						{/if}
						<button class="button-small danger" onclick={() => handleDeleteClick(receipt)}>
							{$LL.common.delete()}
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

{#if receiptToDelete}
	<ConfirmModal
		title={$LL.confirm.deleteReceiptTitle()}
		message={$LL.confirm.deleteReceiptMessage({ name: receiptToDelete.file_name })}
		confirmText={$LL.common.delete()}
		danger={true}
		onConfirm={handleConfirmDelete}
		onCancel={() => (receiptToDelete = null)}
	/>
{/if}

{#if receiptToAssign}
	<TripSelectorModal
		receipt={receiptToAssign}
		onSelect={handleAssignToTrip}
		onClose={() => (receiptToAssign = null)}
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

	.config-warning code.filename {
		background: #e9ecef;
		font-weight: 600;
		color: #495057;
	}

	.config-sample {
		background: #f8f9fa;
		color: #212529;
		border: 1px solid #dee2e6;
		padding: 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
		overflow-x: auto;
		margin: 0.75rem 0;
		font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
	}

	.config-note {
		font-size: 0.8rem;
		color: #856404;
		font-style: italic;
		margin: 0.25rem 0 0.5rem 0 !important;
	}

	.config-path-btn {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
		margin-top: 0.75rem;
		padding: 0.75rem 1rem;
		background: #fff;
		border: 1px solid #ddd;
		border-radius: 6px;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition: all 0.2s ease;
	}

	.config-path-btn:hover {
		background: #f0f0f0;
		border-color: #007bff;
	}

	.config-path-btn code {
		word-break: break-all;
		color: #666;
	}

	.config-path-btn .open-icon {
		color: #007bff;
		font-weight: 500;
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

	.badge.danger {
		background: #f8d7da;
		color: #721c24;
	}

	.verification-summary {
		display: flex;
		gap: 1rem;
		padding: 0.75rem 1rem;
		background: #f8f9fa;
		border-radius: 4px;
		margin-bottom: 1rem;
	}

	.verification-summary.all-matched {
		background: #d4edda;
	}

	.status-ok {
		color: #155724;
		font-weight: 500;
	}

	.status-warning {
		color: #856404;
		font-weight: 500;
	}

	.matched-trip {
		font-size: 0.875rem;
		color: #28a745;
		margin-top: 0.5rem;
		grid-column: 1 / -1;
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

	.value-with-confidence {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.value {
		font-weight: 500;
		color: #2c3e50;
	}

	.value.uncertain {
		color: #e67e22;
	}

	.confidence-dot {
		display: inline-block;
		width: 10px;
		height: 10px;
		min-width: 10px;
		min-height: 10px;
		border-radius: 50%;
		cursor: help;
		flex-shrink: 0;
		border: 1px solid rgba(0, 0, 0, 0.2);
	}

	.confidence-high {
		background-color: #27ae60;
	}

	.confidence-medium {
		background-color: #f39c12;
	}

	.confidence-low {
		background-color: #e74c3c;
	}

	.confidence-unknown {
		background-color: #95a5a6;
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

	.button.secondary {
		background-color: #27ae60;
	}

	.button.secondary:hover:not(:disabled) {
		background-color: #219a52;
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

	.folder-structure-warning {
		background: #f8d7da;
		border: 1px solid #f5c6cb;
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}

	.folder-structure-warning .warning-title {
		font-weight: 600;
		color: #721c24;
		margin-bottom: 0.5rem;
	}

	.folder-structure-warning .warning-details {
		color: #721c24;
		font-size: 0.875rem;
		margin-bottom: 0.5rem;
	}

	.folder-structure-warning .warning-hint {
		color: #856404;
		font-size: 0.8rem;
		font-style: italic;
	}

	.date-mismatch-icon {
		color: #e67e22;
		cursor: help;
		font-size: 0.875rem;
	}
</style>
