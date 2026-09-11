<script lang="ts">
	import * as api from '$lib/api';
	import { toast } from '$lib/stores/toast';
	import type { PaperlessInvoiceRow, Trip } from '$lib/types';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';
	import TripSelectorModal from '$lib/components/TripSelectorModal.svelte';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { selectedYearStore } from '$lib/stores/year';
	import LL from '$lib/i18n/i18n-svelte';

	let paperlessRows = $state<PaperlessInvoiceRow[]>([]);
	let loading = $state(true);
	let paperlessError = $state<string | null>(null);
	let needsPaperlessSetup = $state(false);
	let invoiceToAssign = $state<PaperlessInvoiceRow | null>(null);
	let invoiceToUnassign = $state<PaperlessInvoiceRow | null>(null);

	// Load and reload invoices whenever year or vehicle changes (including initial load).
	// This runs after the layout populates the stores, fixing the race where the page
	// mounts before activeVehicleStore is set.
	$effect(() => {
		const _year = $selectedYearStore;
		const currentVehicle = $activeVehicleStore;
		if (currentVehicle) {
			loadInvoices();
		} else {
			loading = false;
		}
	});

	async function loadInvoices() {
		loading = true;
		paperlessError = null;
		needsPaperlessSetup = false;
		try {
			const vehicle = $activeVehicleStore;
			if (!vehicle) {
				paperlessRows = [];
				return;
			}
			paperlessRows = await api.getPaperlessInvoices(vehicle.id, $selectedYearStore);
		} catch (error) {
			paperlessError = String(error);
			paperlessRows = [];
			needsPaperlessSetup =
				paperlessError.includes('NotConfigured') || paperlessError.includes('not configured');
		} finally {
			loading = false;
		}
	}

	function formatDatetime(datetimeStr: string | null): string {
		if (!datetimeStr) return '--';
		try {
			const date = new Date(datetimeStr);
			const hasTime = datetimeStr.includes('T') && !datetimeStr.endsWith('T00:00:00');
			if (hasTime) {
				return date.toLocaleString('sk-SK', {
					day: '2-digit',
					month: '2-digit',
					year: 'numeric',
					hour: '2-digit',
					minute: '2-digit'
				});
			}
			return date.toLocaleDateString('sk-SK');
		} catch {
			return datetimeStr;
		}
	}

	function handleOpenPaperless(url: string) {
		window.open(url, '_blank', 'noopener');
	}

	function handleAssignClick(row: PaperlessInvoiceRow) {
		if (!$activeVehicleStore) {
			toast.error($LL.toast.errorSelectVehicleFirst());
			return;
		}
		invoiceToAssign = row;
	}

	async function handleAssignInvoice(result: {
		trip: Trip;
		assignmentType: 'Fuel' | 'Other';
		mismatchOverride: boolean;
	}) {
		if (!invoiceToAssign || !$activeVehicleStore) return;
		const row = invoiceToAssign;
		try {
			await api.assignPaperlessInvoice(
				row.paperlessDocumentId,
				result.trip.id,
				$activeVehicleStore.id,
				result.assignmentType,
				result.mismatchOverride,
			);
			await loadInvoices();
			invoiceToAssign = null;
			toast.success($LL.doklady.paperless.assignedToast());
		} catch (error) {
			console.error('Failed to assign invoice:', error);
			toast.error($LL.doklady.paperless.assignError({ error: String(error) }));
		}
	}

	function handleUnassignClick(row: PaperlessInvoiceRow) {
		invoiceToUnassign = row;
	}

	async function handleConfirmUnassign() {
		if (!invoiceToUnassign) return;
		const row = invoiceToUnassign;
		try {
			await api.unassignPaperlessInvoice(row.paperlessDocumentId);
			await loadInvoices();
			toast.success($LL.doklady.paperless.unassignedToast());
		} catch (error) {
			console.error('Failed to unassign invoice:', error);
			toast.error($LL.doklady.paperless.unassignError());
		} finally {
			invoiceToUnassign = null;
		}
	}
</script>

<div class="doklady-page">
	<div class="header">
		<h1>{$LL.app.nav.receipts()}</h1>
		<div class="header-actions">
			<button
				type="button"
				data-test="paperless-refresh"
				class="button"
				onclick={loadInvoices}
				disabled={loading}
			>
				{$LL.doklady.paperless.refresh()}
			</button>
		</div>
	</div>

	{#if loading}
		<p class="placeholder">{$LL.common.loading()}</p>
	{:else if needsPaperlessSetup}
		<div class="empty-state">
			<p>{$LL.doklady.paperless.notConfigured()}</p>
			<a class="button" href="/settings">{$LL.doklady.paperless.openSettings()}</a>
		</div>
	{:else}
		{#if paperlessError}
			<div class="config-warning" data-test="paperless-error">
				<div class="warning-icon">⚠</div>
				<p>{paperlessError}</p>
			</div>
		{/if}

		{#if paperlessRows.length === 0}
			<p class="placeholder">{$LL.doklady.paperless.noInvoices()}</p>
		{:else}
			<div class="receipts-section">
				<div class="receipts-list">
					{#each paperlessRows as row (row.paperlessDocumentId)}
						<div
							class="receipt-card"
							class:unmatched={row.tripId === null}
							data-test="paperless-row"
							data-doc-id={row.paperlessDocumentId}
						>
							<div class="receipt-header">
								<span class="file-name" data-test="title">
									<span class="receipt-type-icon">
										{row.assignmentType === 'Fuel' ? '⛽' : '📄'}
									</span>
									{row.title}
								</span>
								<div class="header-badges">
									{#if row.tripId}
										<span class="badge success" data-test="trip-indicator">{$LL.doklady.paperless.assigned()}</span>
									{:else}
										<span class="badge danger">{$LL.doklady.paperless.unassigned()}</span>
									{/if}
								</div>
							</div>
							<div class="receipt-details">
								<div class="detail-row">
									<span class="label">{$LL.doklady.paperless.date()}</span>
									<span class="value">
										{row.receiptDatetime ? formatDatetime(row.receiptDatetime) : $LL.doklady.paperless.noDate()}
									</span>
								</div>
								<div class="detail-row">
									<span class="label">{$LL.doklady.paperless.price()}</span>
									<span class="value">
										{row.totalPriceEur != null ? `${row.totalPriceEur.toFixed(2)} €` : '-'}
									</span>
								</div>
								<div class="detail-row">
									<span class="label">{$LL.doklady.paperless.liters()}</span>
									<span class="value" data-test="liters">
										{#if row.assignmentType === 'Fuel' && row.liters != null}
											{row.liters.toFixed(2)} L
										{:else}
											-
										{/if}
									</span>
								</div>
							</div>
							<div class="receipt-actions">
								<button
									type="button"
									class="button-small"
									onclick={() => handleOpenPaperless(row.paperlessUrl)}
								>
									{$LL.doklady.paperless.openInPaperless()}
								</button>
								{#if row.tripId}
									<button
										type="button"
										class="button-small"
										onclick={() => handleUnassignClick(row)}
									>
										{$LL.doklady.paperless.unassign()}
									</button>
								{:else}
									<button
										type="button"
										data-test="assign-btn"
										class="button-small primary"
										onclick={() => handleAssignClick(row)}
									>
										{$LL.doklady.paperless.assignToTrip()}
									</button>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>

{#if invoiceToUnassign}
	<ConfirmModal
		title={$LL.doklady.paperless.unassignTitle()}
		message={$LL.doklady.paperless.unassignMessage({ name: invoiceToUnassign.title })}
		confirmText={$LL.doklady.paperless.unassign()}
		danger={true}
		onConfirm={handleConfirmUnassign}
		onCancel={() => (invoiceToUnassign = null)}
	/>
{/if}

{#if invoiceToAssign}
	<TripSelectorModal
		invoice={invoiceToAssign}
		onSelect={handleAssignInvoice}
		onClose={() => (invoiceToAssign = null)}
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

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 1rem;
		padding: 3rem 1rem;
		text-align: center;
		color: var(--text-secondary);
	}

	.empty-state .button {
		text-decoration: none;
	}

	.config-warning {
		background: var(--warning-bg);
		border: 1px solid var(--warning-border);
		padding: 1rem;
		border-radius: 8px;
		margin-bottom: 1.5rem;
	}

	.config-warning .warning-icon {
		font-size: 2rem;
		margin-bottom: 0.5rem;
	}

	.config-warning p {
		margin: 0.5rem 0;
	}

	.receipts-section {
		margin-bottom: 2rem;
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

	.header-badges {
		display: flex;
		gap: 0.5rem;
		align-items: center;
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

	.badge.danger {
		background: var(--toast-error-bg);
		color: var(--toast-error-color);
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
		color: var(--text-secondary);
		font-size: 0.875rem;
	}

	.value {
		font-weight: 500;
		color: var(--text-primary);
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

	.button-small.primary {
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
	}

	.button-small.primary:hover {
		background-color: var(--btn-active-primary-hover);
	}
</style>
