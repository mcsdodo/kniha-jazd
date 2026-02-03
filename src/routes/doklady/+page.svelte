<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { Receipt, ReceiptSettings, ConfidenceLevel, Trip, VerificationResult, ReceiptVerification, ReceiptMismatchReason } from '$lib/types';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import TripSelectorModal from '$lib/components/TripSelectorModal.svelte';
	import ReceiptEditModal from '$lib/components/ReceiptEditModal.svelte';
	import { openPath } from '@tauri-apps/plugin-opener';
	import { appDataDir } from '@tauri-apps/api/path';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import { triggerReceiptRefresh } from '$lib/stores/receipts';
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
	let typeFilter = $state<'all' | 'fuel' | 'other'>('all');
	let receiptToDelete = $state<Receipt | null>(null);
	let receiptToEdit = $state<Receipt | null>(null);
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

	// Reload receipts when year OR vehicle changes
	let previousYear = $state<number | null>(null);
	let previousVehicleId = $state<string | null>(null);
	$effect(() => {
		const currentYear = $selectedYearStore;
		const currentVehicle = $activeVehicleStore;
		const currentVehicleId = currentVehicle?.id ?? null;

		const yearChanged = previousYear !== null && previousYear !== currentYear;
		const vehicleChanged = previousVehicleId !== null && previousVehicleId !== currentVehicleId;

		if (yearChanged || vehicleChanged) {
			loadReceipts();
			loadVerification();
		}

		previousYear = currentYear;
		previousVehicleId = currentVehicleId;
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
			const vehicle = $activeVehicleStore;
			if (vehicle) {
				receipts = await api.getReceiptsForVehicle(vehicle.id, $selectedYearStore);
			} else {
				receipts = [];
			}
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

	// Refresh all receipt data and update nav badge
	async function refreshReceiptData() {
		await loadReceipts();
		await loadVerification();
		triggerReceiptRefresh();
	}

	function getVerificationForReceipt(receiptId: string): ReceiptVerification | null {
		return verification?.receipts.find(v => v.receiptId === receiptId) ?? null;
	}

	async function handleScan() {
		if (!settings?.receiptsFolderPath) {
			toast.error($LL.toast.errorSetApiKeyFirst());
			return;
		}

		syncing = true;
		try {
			const result = await api.scanReceipts();
			await refreshReceiptData();

			// Handle folder structure warning
			folderStructureWarning = result.warning;

			if (result.newCount > 0) {
				toast.success($LL.toast.foundNewReceipts({ count: result.newCount }));
			} else {
				toast.success($LL.toast.noNewReceipts());
			}
		} catch (error) {
			console.error('Failed to scan receipts:', error);
			toast.error($LL.toast.errorSyncReceipts({ error: String(error) }));
		} finally {
			syncing = false;
		}
	}

	async function handleProcessPending() {
		if (!settings?.geminiApiKey) {
			toast.error($LL.toast.errorSetApiKeyOnlyFirst());
			return;
		}

		processing = true;
		processingProgress = null;
		try {
			const result = await api.processPendingReceipts();
			await refreshReceiptData();

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
			await refreshReceiptData();
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
			await refreshReceiptData();
			toast.success($LL.toast.receiptReprocessed({ name: receipt.fileName }));
		} catch (error) {
			console.error('Failed to reprocess receipt:', error);
			toast.error($LL.toast.errorReprocessReceipt({ name: receipt.fileName, error: String(error) }));
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

	function formatDatetime(datetimeStr: string | null): string {
		if (!datetimeStr) return '--';
		try {
			const date = new Date(datetimeStr);
			// Check if time component is present (not 00:00:00)
			const hasTime = datetimeStr.includes('T') && !datetimeStr.endsWith('T00:00:00');
			if (hasTime) {
				return date.toLocaleString('sk-SK', {
					day: '2-digit',
					month: '2-digit',
					year: 'numeric',
					hour: '2-digit',
					minute: '2-digit'
				});
			} else {
				return date.toLocaleDateString('sk-SK');
			}
		} catch {
			return datetimeStr;
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
	 * Check if a receipt has a date mismatch between sourceYear (folder) and receiptDatetime (OCR).
	 * Returns null if no mismatch, or { receiptYear, folderYear } if there's a mismatch.
	 */
	function getDateMismatch(receipt: Receipt): { receiptYear: number; folderYear: number } | null {
		if (!receipt.receiptDatetime || !receipt.sourceYear) {
			return null;
		}
		const receiptYear = new Date(receipt.receiptDatetime).getFullYear();
		if (receiptYear !== receipt.sourceYear) {
			return { receiptYear, folderYear: receipt.sourceYear };
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

	async function handleAssignToTrip(result: { trip: Trip; assignmentType: 'Fuel' | 'Other'; mismatchOverride: boolean }) {
		if (!receiptToAssign || !$activeVehicleStore) return;

		try {
			await api.assignReceiptToTrip(
				receiptToAssign.id,
				result.trip.id,
				$activeVehicleStore.id,
				result.assignmentType,
				result.mismatchOverride
			);
			await refreshReceiptData();
			receiptToAssign = null;
			toast.success($LL.toast.receiptAssigned());
		} catch (error) {
			console.error('Failed to assign receipt:', error);
			toast.error($LL.toast.errorAssignReceipt({ error: String(error) }));
		}
	}

	function handleEditClick(receipt: Receipt) {
		receiptToEdit = receipt;
	}

	async function handleSaveReceipt(data: {
		receiptDatetime: string | null;
		liters: number | null;
		originalAmount: number | null;
		originalCurrency: import('$lib/types').ReceiptCurrency | null;
		totalPriceEur: number | null;
		stationName: string | null;
		vendorName: string | null;
		costDescription: string | null;
	}) {
		if (!receiptToEdit) return;

		try {
			// Normalize datetime format: datetime-local gives "YYYY-MM-DDTHH:mm" but backend expects "YYYY-MM-DDTHH:mm:ss"
			let normalizedDatetime = data.receiptDatetime;
			if (normalizedDatetime && !normalizedDatetime.includes(':00', normalizedDatetime.length - 3)) {
				// Add seconds if missing (datetime-local format is YYYY-MM-DDTHH:mm)
				if (normalizedDatetime.match(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/)) {
					normalizedDatetime += ':00';
				}
			}

			// Build updated receipt object
			const updatedReceipt: Receipt = {
				...receiptToEdit,
				receiptDatetime: normalizedDatetime,
				liters: data.liters,
				originalAmount: data.originalAmount,
				originalCurrency: data.originalCurrency,
				totalPriceEur: data.totalPriceEur,
				stationName: data.stationName,
				vendorName: data.vendorName,
				costDescription: data.costDescription,
				// Clear NeedsReview if we now have EUR value
				status: data.totalPriceEur != null && receiptToEdit.status === 'NeedsReview'
					? 'Parsed'
					: receiptToEdit.status,
			};

			await api.updateReceipt(updatedReceipt);
			await refreshReceiptData();
			receiptToEdit = null;
			toast.success($LL.toast.receiptUpdated());
		} catch (error) {
			console.error('Failed to update receipt:', error);
			toast.error($LL.toast.errorAssignReceipt({ error: String(error) }));
		}
	}

	// Helper to check if receipt is assigned to a trip (design spec v7: trip_id based)
	function isReceiptAssigned(receipt: Receipt): boolean {
		return receipt.tripId != null;
	}

	// Helper to check if receipt is fuel or other cost
	function isFuelReceipt(receipt: Receipt): boolean {
		return receipt.liters !== null;
	}

	// Helper to check if receipt has foreign currency (needs EUR conversion)
	function isForeignCurrency(receipt: Receipt): boolean {
		return receipt.originalCurrency != null && receipt.originalCurrency !== 'EUR';
	}

	// Helper to check if foreign currency receipt has been converted
	function hasEurConversion(receipt: Receipt): boolean {
		return isForeignCurrency(receipt) && receipt.totalPriceEur != null;
	}

	// Helper to format price display based on currency
	function formatPriceDisplay(receipt: Receipt): string {
		if (isForeignCurrency(receipt)) {
			// Foreign currency receipt
			const originalPart = receipt.originalAmount != null
				? `${receipt.originalAmount.toFixed(2)} ${receipt.originalCurrency}`
				: `?? ${receipt.originalCurrency}`;
			if (receipt.totalPriceEur != null) {
				// Has EUR conversion
				return `${originalPart} → ${receipt.totalPriceEur.toFixed(2)} €`;
			} else {
				// Needs conversion
				return `${originalPart} → ⚠️`;
			}
		} else {
			// EUR or no currency specified
			return receipt.totalPriceEur != null ? `${receipt.totalPriceEur.toFixed(2)} €` : '??';
		}
	}

	// Helper to format mismatch reason for display
	function formatMismatchReason(reason: ReceiptMismatchReason | undefined): string {
		if (!reason || reason.type === 'none') return '';
		switch (reason.type) {
			case 'missingReceiptData':
				return $LL.receipts.mismatchMissingData();
			case 'noFuelTripFound':
				return $LL.receipts.mismatchNoFuelTrip();
			case 'dateMismatch':
				return $LL.receipts.mismatchDate({
					receiptDate: reason.receiptDate,
					tripDate: reason.closestTripDate
				});
			case 'datetimeOutOfRange':
				return $LL.receipts.mismatchDatetimeOutOfRange({
					receiptTime: reason.receiptTime,
					tripStart: reason.tripStart,
					tripEnd: reason.tripEnd
				});
			case 'litersMismatch':
				return $LL.receipts.mismatchLiters({
					receiptLiters: reason.receiptLiters,
					tripLiters: reason.tripLiters
				});
			case 'priceMismatch':
				return $LL.receipts.mismatchPrice({
					receiptPrice: reason.receiptPrice,
					tripPrice: reason.tripPrice
				});
			case 'noOtherCostMatch':
				return $LL.receipts.mismatchNoOtherCost();
			default:
				return '';
		}
	}

	// Svelte 5: use $derived instead of $:
	// Apply type filter only
	let typeFilteredReceipts = $derived(
		receipts.filter((r) => {
			if (typeFilter === 'fuel' && !isFuelReceipt(r)) return false;
			if (typeFilter === 'other' && isFuelReceipt(r)) return false;
			return true;
		})
	);

	// Split into unassigned and assigned sections (design spec v7)
	let unassignedReceipts = $derived(
		typeFilteredReceipts.filter((r) => {
			if (filter === 'needs_review' && r.status !== 'NeedsReview') return false;
			return !isReceiptAssigned(r);
		})
	);
	let assignedReceipts = $derived(
		typeFilteredReceipts.filter((r) => {
			if (filter === 'needs_review') return false; // needs_review only shows unassigned
			if (filter === 'unassigned') return false; // unassigned filter hides assigned
			return isReceiptAssigned(r);
		})
	);

	// Counts for filter badges
	let unassignedCount = $derived(receipts.filter((r) => !isReceiptAssigned(r)).length);
	let needsReviewCount = $derived(receipts.filter((r) => r.status === 'NeedsReview').length);
	let fuelCount = $derived(receipts.filter((r) => isFuelReceipt(r)).length);
	let otherCount = $derived(receipts.filter((r) => !isFuelReceipt(r)).length);

	let isConfigured = $derived(settings?.geminiApiKey && settings?.receiptsFolderPath);
	let pendingCount = $derived(receipts.filter((r) => r.status === 'Pending').length);
</script>

<div class="doklady-page">
	<div class="header">
		<h1>{$LL.receipts.title()}</h1>
		<div class="header-actions">
			<button class="button" onclick={handleScan} disabled={syncing || processing || !settings?.receiptsFolderPath}>
				{syncing ? $LL.receipts.scanning() : $LL.receipts.scanFolder()}
			</button>
			<button
				class="button secondary"
				onclick={handleProcessPending}
				disabled={processing || syncing || !settings?.geminiApiKey || pendingCount === 0}
			>
				{#if processing && processingProgress}
					{$LL.receipts.recognizing({ current: processingProgress.current, total: processingProgress.total })}
				{:else if processing}
					{$LL.receipts.processing()}
				{:else}
					{$LL.receipts.recognizeData()}{#if pendingCount > 0} ({pendingCount}){/if}
				{/if}
			</button>
		</div>
	</div>

	{#if !isConfigured}
		<div class="config-warning">
			<div class="warning-icon">⚠</div>
			<h3>{$LL.receipts.notConfiguredTitle()}</h3>
			<p>{$LL.receipts.notConfiguredDescription()}</p>
			<ul class="requirements-list">
				<li>{$LL.receipts.notConfiguredApiKey()}</li>
				<li>{$LL.receipts.notConfiguredFolder()}</li>
			</ul>
			<button class="button" onclick={() => goto('/settings#receipt-scanning')}>
				{$LL.receipts.goToSettings()}
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
		<div class="filter-group">
			<button class="filter-btn" class:active={filter === 'all'} onclick={() => (filter = 'all')}>
				{$LL.receipts.filterAll()} ({receipts.length})
			</button>
			<button
				class="filter-btn"
				class:active={filter === 'unassigned'}
				onclick={() => (filter = 'unassigned')}
			>
				{$LL.receipts.filterUnassigned()} ({unassignedCount})
			</button>
			<button
				class="filter-btn"
				class:active={filter === 'needs_review'}
				onclick={() => (filter = 'needs_review')}
			>
				{$LL.receipts.filterNeedsReview()} ({needsReviewCount})
			</button>
		</div>
		<select class="type-filter" bind:value={typeFilter}>
			<option value="all">{$LL.receipts.filterAll()}</option>
			<option value="fuel">{$LL.receipts.filterFuel()} ({fuelCount})</option>
			<option value="other">{$LL.receipts.filterOther()} ({otherCount})</option>
		</select>
	</div>

	{#if verification}
		<div class="verification-summary" class:all-matched={verification.unmatched === 0}>
			{#if verification.unmatched === 0}
				<span class="status-ok">✓ {$LL.receipts.allAssigned({ count: verification.matched, total: verification.total })}</span>
			{:else}
				<span class="status-ok">✓ {$LL.receipts.assigned({ count: verification.matched, total: verification.total })}</span>
				<span class="status-warning">⚠ {$LL.receipts.unassigned({ count: verification.unmatched })}</span>
			{/if}
		</div>
	{/if}

	{#if loading}
		<p class="placeholder">{$LL.common.loading()}</p>
	{:else if receipts.length === 0}
		<p class="placeholder">{$LL.receipts.noReceipts()}</p>
	{:else}
		<!-- Design spec v7: Two sections - Unassigned and Assigned -->

		{#if unassignedReceipts.length > 0}
			<div class="receipts-section">
				<h2 class="section-header unassigned">
					<span class="section-icon">🔴</span>
					{$LL.receipts.sectionUnassigned()} ({unassignedReceipts.length})
				</h2>
				<div class="receipts-list">
					{#each unassignedReceipts as receipt}
						{@const verif = getVerificationForReceipt(receipt.id)}
						{@const dateMismatch = getDateMismatch(receipt)}
						<div class="receipt-card unmatched">
							<div class="receipt-header">
								<span class="file-name">
									<span class="receipt-type-icon" title={isFuelReceipt(receipt) ? $LL.receipts.filterFuel() : $LL.receipts.otherCost()}>
										{isFuelReceipt(receipt) ? '\u26FD' : '\uD83D\uDCC4'}
									</span>
									{receipt.fileName}
								</span>
								<div class="header-badges">
									{#if receipt.status === 'NeedsReview'}
										<span class="badge warning">{$LL.receipts.statusNeedsReview()}</span>
									{:else}
										<span class="badge danger">{$LL.receipts.statusUnassigned()}</span>
									{/if}
								</div>
							</div>
							<div class="receipt-details">
								<div class="detail-row">
									<span class="label">{$LL.receipts.date()}</span>
									<span class="value-with-confidence">
										<span class="value">{formatDatetime(receipt.receiptDatetime)}</span>
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
								{#if isFuelReceipt(receipt)}
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
											<span
												class="value"
												class:uncertain={receipt.confidence.totalPrice === 'Low'}
												class:needs-conversion={isForeignCurrency(receipt) && !hasEurConversion(receipt)}
											>
												{formatPriceDisplay(receipt)}
											</span>
											<span
												class="confidence-dot {getConfidenceInfo(receipt.confidence.totalPrice).class}"
												title={getConfidenceInfo(receipt.confidence.totalPrice).label}
											></span>
										</span>
									</div>
									{#if receipt.stationName}
										<div class="detail-row">
											<span class="label">{$LL.receipts.station()}</span>
											<span class="value">{receipt.stationName}</span>
										</div>
									{/if}
								{:else}
									<div class="detail-row">
										<span class="label">{$LL.receipts.price()}</span>
										<span class="value-with-confidence">
											<span
												class="value"
												class:uncertain={receipt.confidence.totalPrice === 'Low'}
												class:needs-conversion={isForeignCurrency(receipt) && !hasEurConversion(receipt)}
											>
												{formatPriceDisplay(receipt)}
											</span>
											<span
												class="confidence-dot {getConfidenceInfo(receipt.confidence.totalPrice).class}"
												title={getConfidenceInfo(receipt.confidence.totalPrice).label}
											></span>
										</span>
									</div>
									{#if receipt.vendorName}
										<div class="detail-row">
											<span class="label">{$LL.receipts.vendor()}</span>
											<span class="value">{receipt.vendorName}</span>
										</div>
									{/if}
									{#if receipt.costDescription}
										<div class="detail-row full-width">
											<span class="label">{$LL.receipts.description()}</span>
											<span class="value">{receipt.costDescription}</span>
										</div>
									{/if}
								{/if}
								{#if receipt.errorMessage}
									<div class="error-message">{receipt.errorMessage}</div>
								{/if}
							</div>
							<div class="receipt-actions">
								<button class="button-small" onclick={() => handleOpenFile(receipt.filePath)}>
									{$LL.receipts.open()}
								</button>
								<button class="button-small" onclick={() => handleEditClick(receipt)}>
									{$LL.common.edit()}
								</button>
								<button
									class="button-small"
									onclick={() => handleReprocess(receipt)}
									disabled={reprocessingIds.has(receipt.id)}
								>
									{reprocessingIds.has(receipt.id) ? $LL.receipts.reprocessing() : $LL.receipts.reprocess()}
								</button>
								<button class="button-small primary" onclick={() => handleAssignClick(receipt)}>{$LL.receipts.assignToTrip()}</button>
								<button class="button-small danger" onclick={() => handleDeleteClick(receipt)}>
									{$LL.common.delete()}
								</button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if assignedReceipts.length > 0}
			<div class="receipts-section">
				<h2 class="section-header assigned">
					<span class="section-icon">🟢</span>
					{$LL.receipts.sectionAssigned()} ({assignedReceipts.length})
				</h2>
				<div class="receipts-list">
					{#each assignedReceipts as receipt}
						{@const verif = getVerificationForReceipt(receipt.id)}
						{@const dateMismatch = getDateMismatch(receipt)}
						<div class="receipt-card">
							<div class="receipt-header">
								<span class="file-name">
									<span class="receipt-type-icon" title={isFuelReceipt(receipt) ? $LL.receipts.filterFuel() : $LL.receipts.otherCost()}>
										{isFuelReceipt(receipt) ? '\u26FD' : '\uD83D\uDCC4'}
									</span>
									{receipt.fileName}
								</span>
								<div class="header-badges">
									{#if receipt.assignmentType}
										<span class="badge {receipt.assignmentType === 'Fuel' ? 'fuel' : 'other'}">
											{receipt.assignmentType === 'Fuel' ? $LL.receipts.assignedAsFuel() : $LL.receipts.assignedAsOther()}
										</span>
										{#if receipt.mismatchOverride}
											<span class="badge override" title={$LL.receipts.overrideConfirmed()}>✓</span>
										{/if}
									{/if}
									<span class="badge success">{$LL.receipts.statusAssigned()}</span>
								</div>
							</div>
							<div class="receipt-details">
								<div class="detail-row">
									<span class="label">{$LL.receipts.date()}</span>
									<span class="value-with-confidence">
										<span class="value">{formatDatetime(receipt.receiptDatetime)}</span>
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
								{#if isFuelReceipt(receipt)}
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
											<span
												class="value"
												class:uncertain={receipt.confidence.totalPrice === 'Low'}
												class:needs-conversion={isForeignCurrency(receipt) && !hasEurConversion(receipt)}
											>
												{formatPriceDisplay(receipt)}
											</span>
											<span
												class="confidence-dot {getConfidenceInfo(receipt.confidence.totalPrice).class}"
												title={getConfidenceInfo(receipt.confidence.totalPrice).label}
											></span>
										</span>
									</div>
									{#if receipt.stationName}
										<div class="detail-row">
											<span class="label">{$LL.receipts.station()}</span>
											<span class="value">{receipt.stationName}</span>
										</div>
									{/if}
								{:else}
									<div class="detail-row">
										<span class="label">{$LL.receipts.price()}</span>
										<span class="value-with-confidence">
											<span
												class="value"
												class:uncertain={receipt.confidence.totalPrice === 'Low'}
												class:needs-conversion={isForeignCurrency(receipt) && !hasEurConversion(receipt)}
											>
												{formatPriceDisplay(receipt)}
											</span>
											<span
												class="confidence-dot {getConfidenceInfo(receipt.confidence.totalPrice).class}"
												title={getConfidenceInfo(receipt.confidence.totalPrice).label}
											></span>
										</span>
									</div>
									{#if receipt.vendorName}
										<div class="detail-row">
											<span class="label">{$LL.receipts.vendor()}</span>
											<span class="value">{receipt.vendorName}</span>
										</div>
									{/if}
									{#if receipt.costDescription}
										<div class="detail-row full-width">
											<span class="label">{$LL.receipts.description()}</span>
											<span class="value">{receipt.costDescription}</span>
										</div>
									{/if}
								{/if}
								{#if receipt.errorMessage}
									<div class="error-message">{receipt.errorMessage}</div>
								{/if}
								{#if verif?.matched}
									<div class="matched-trip">
										{$LL.receipts.trip()} {verif.matchedTripDatetime} | {verif.matchedTripRoute}
									</div>
									{#if verif.datetimeWarning && verif.matchedTripTimeRange}
										<div class="datetime-warning-row">
											<span class="warning-icon">⚠</span>
											<span class="warning-text">{$LL.trips.receiptDatetimeMismatchWithRange({ timeRange: verif.matchedTripTimeRange })}</span>
										</div>
									{/if}
								{/if}
							</div>
							<div class="receipt-actions">
								<button class="button-small" onclick={() => handleOpenFile(receipt.filePath)}>
									{$LL.receipts.open()}
								</button>
								<button class="button-small" onclick={() => handleEditClick(receipt)}>
									{$LL.common.edit()}
								</button>
								<button
									class="button-small"
									onclick={() => handleReprocess(receipt)}
									disabled={reprocessingIds.has(receipt.id)}
								>
									{reprocessingIds.has(receipt.id) ? $LL.receipts.reprocessing() : $LL.receipts.reprocess()}
								</button>
								<button class="button-small danger" onclick={() => handleDeleteClick(receipt)}>
									{$LL.common.delete()}
								</button>
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if unassignedReceipts.length === 0 && assignedReceipts.length === 0}
			<p class="placeholder">{$LL.receipts.noReceipts()}</p>
		{/if}
	{/if}
</div>

{#if receiptToDelete}
	<ConfirmModal
		title={$LL.confirm.deleteReceiptTitle()}
		message={$LL.confirm.deleteReceiptMessage({ name: receiptToDelete.fileName })}
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

{#if receiptToEdit}
	<ReceiptEditModal
		receipt={receiptToEdit}
		onSave={handleSaveReceipt}
		onClose={() => (receiptToEdit = null)}
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
		color: var(--text-primary);
	}

	.config-warning {
		background: var(--warning-bg);
		border: 1px solid var(--warning-border);
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}

	.config-warning h3 {
		margin: 0 0 0.5rem 0;
		color: var(--warning-color);
		font-size: 1.1rem;
	}

	.config-warning .warning-icon {
		font-size: 2rem;
		margin-bottom: 0.5rem;
	}

	.config-warning .requirements-list {
		margin: 0.75rem 0;
		padding-left: 1.5rem;
		color: var(--text-primary);
	}

	.config-warning .requirements-list li {
		margin-bottom: 0.25rem;
	}

	.config-warning p {
		margin: 0.5rem 0;
	}

	.config-warning code {
		background: var(--bg-surface-alt);
		padding: 0.2rem 0.4rem;
		border-radius: 3px;
		font-size: 0.875rem;
	}

	.config-warning code.filename {
		background: var(--bg-surface-alt);
		font-weight: 600;
		color: var(--text-primary);
	}

	.config-sample {
		background: var(--bg-surface-alt);
		color: var(--text-primary);
		border: 1px solid var(--border-default);
		padding: 1rem;
		border-radius: 6px;
		font-size: 0.875rem;
		overflow-x: auto;
		margin: 0.75rem 0;
		font-family: var(--font-mono);
	}

	.config-note {
		font-size: 0.8rem;
		color: var(--warning-color);
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
		background: var(--bg-surface);
		border: 1px solid var(--border-input);
		border-radius: 6px;
		cursor: pointer;
		text-align: left;
		width: 100%;
		transition: all 0.2s ease;
	}

	.config-path-btn:hover {
		background: var(--bg-surface-alt);
		border-color: var(--accent-primary);
	}

	.config-path-btn code {
		word-break: break-all;
		color: var(--text-secondary);
	}

	.config-path-btn .open-icon {
		color: var(--accent-primary);
		font-weight: 500;
	}

	.filters {
		display: flex;
		gap: 1rem;
		margin-bottom: 1.5rem;
		align-items: center;
		justify-content: space-between;
	}

	.filter-group {
		display: flex;
		gap: 0.5rem;
	}

	.type-filter {
		padding: 0.5rem 1rem;
		border: 1px solid var(--border-input);
		background: var(--bg-surface);
		color: var(--text-primary);
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.875rem;
	}

	.type-filter:hover {
		border-color: var(--accent-primary);
	}

	.filter-btn {
		padding: 0.5rem 1rem;
		border: 1px solid var(--border-input);
		background: var(--bg-surface);
		color: var(--text-primary);
		border-radius: 4px;
		cursor: pointer;
		transition: all 0.2s;
	}

	.filter-btn:hover {
		background: var(--bg-surface-alt);
	}

	.filter-btn.active {
		background: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		border-color: var(--btn-active-primary-bg);
	}

	.receipts-section {
		margin-bottom: 2rem;
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 1.1rem;
		font-weight: 600;
		margin-bottom: 1rem;
		padding-bottom: 0.5rem;
		border-bottom: 2px solid var(--border-muted);
	}

	.section-header.unassigned {
		border-bottom-color: var(--color-danger);
	}

	.section-header.assigned {
		border-bottom-color: var(--color-success);
	}

	.section-icon {
		font-size: 0.9rem;
	}

	.receipts-list {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.receipt-card {
		background: var(--bg-surface);
		border-radius: 8px;
		padding: 1rem;
		box-shadow: 0 1px 3px var(--shadow-default);
	}

	.receipt-card.unmatched {
		border-left: 3px solid var(--accent-danger);
		background: var(--bg-surface-alt);
	}

	.receipt-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
	}

	.file-name {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 500;
		color: var(--text-primary);
	}

	.receipt-type-icon {
		font-size: 1rem;
	}

	.badge {
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
		font-size: 0.75rem;
		font-weight: 600;
	}

	.badge.success {
		background: var(--toast-success-bg);
		color: var(--toast-success-color);
	}

	.badge.warning {
		background: var(--warning-bg);
		color: var(--warning-color);
	}

	.badge.info {
		background: var(--toast-info-bg);
		color: var(--toast-info-color);
	}

	.badge.neutral {
		background: var(--bg-surface-alt);
		color: var(--text-primary);
	}

	.badge.danger {
		background: var(--toast-error-bg);
		color: var(--toast-error-color);
	}

	.badge.fuel {
		background: var(--accent-primary-light-bg, #e0f2fe);
		color: var(--accent-primary, #0284c7);
	}

	.badge.other {
		background: var(--bg-surface-alt);
		color: var(--text-secondary);
	}

	.badge.override {
		background: var(--warning-bg);
		color: var(--warning-color);
		padding: 0.15rem 0.35rem;
	}

	.header-badges {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.verification-summary {
		display: flex;
		gap: 1rem;
		padding: 0.75rem 1rem;
		background: var(--bg-surface-alt);
		border-radius: 4px;
		margin-bottom: 1rem;
	}

	.verification-summary.all-matched {
		background: var(--toast-success-bg);
	}

	.status-ok {
		color: var(--toast-success-color);
		font-weight: 500;
	}

	.status-warning {
		color: var(--warning-color);
		font-weight: 500;
	}

	.matched-trip {
		font-size: 0.875rem;
		color: var(--accent-success);
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

	.detail-row.full-width {
		grid-column: 1 / -1;
	}

	.label {
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.value-with-confidence {
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.value {
		font-weight: 500;
		color: var(--text-primary);
	}

	.value.uncertain {
		color: var(--accent-warning-dark);
	}

	.value.needs-conversion {
		color: var(--accent-danger);
		font-style: italic;
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
		background-color: var(--accent-success);
	}

	.confidence-medium {
		background-color: #f39c12;
	}

	.confidence-low {
		background-color: #e74c3c;
	}

	.confidence-unknown {
		background-color: var(--text-muted);
	}

	.error-message {
		grid-column: 1 / -1;
		color: var(--accent-danger);
		font-size: 0.875rem;
		padding: 0.5rem;
		background: var(--accent-danger-bg);
		border-radius: 4px;
	}

	.receipt-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.placeholder {
		color: var(--text-secondary);
		font-style: italic;
		text-align: center;
		padding: 2rem;
	}

	.button {
		padding: 0.75rem 1.5rem;
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: background-color 0.2s;
	}

	.button:hover:not(:disabled) {
		background-color: var(--btn-active-primary-hover);
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.button.secondary {
		background-color: var(--btn-active-success-bg);
		color: var(--btn-active-success-color);
	}

	.button.secondary:hover:not(:disabled) {
		background-color: var(--btn-active-success-hover);
	}

	.button-small {
		padding: 0.5rem 1rem;
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
		border: none;
		border-radius: 4px;
		font-size: 0.875rem;
		cursor: pointer;
	}

	.button-small:hover {
		background-color: var(--btn-secondary-hover);
	}

	.button-small.danger {
		background-color: var(--accent-danger-bg);
		color: var(--accent-danger);
	}

	.button-small.danger:hover {
		background-color: var(--accent-danger-hover-bg);
	}

	.folder-structure-warning {
		background: var(--toast-error-bg);
		border: 1px solid var(--toast-error-border);
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}

	.folder-structure-warning .warning-title {
		font-weight: 600;
		color: var(--toast-error-color);
		margin-bottom: 0.5rem;
	}

	.folder-structure-warning .warning-details {
		color: var(--toast-error-color);
		font-size: 0.875rem;
		margin-bottom: 0.5rem;
	}

	.folder-structure-warning .warning-hint {
		color: var(--warning-color);
		font-size: 0.8rem;
		font-style: italic;
	}

	.date-mismatch-icon {
		color: var(--accent-warning-dark);
		cursor: help;
		font-size: 0.875rem;
	}

	.datetime-warning-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--accent-danger);
	}

	.datetime-warning-row .warning-icon {
		font-size: 0.875rem;
	}

	.datetime-warning-row .warning-text {
		font-style: italic;
	}
</style>
