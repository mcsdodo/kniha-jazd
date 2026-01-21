<script lang="ts">
	import type { Receipt, ReceiptCurrency } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';

	export let receipt: Receipt;
	export let onSave: (data: {
		receiptDate: string | null;
		liters: number | null;
		originalAmount: number | null;
		originalCurrency: ReceiptCurrency | null;
		totalPriceEur: number | null;
		stationName: string | null;
		vendorName: string | null;
		costDescription: string | null;
	}) => void;
	export let onClose: () => void;

	// Form state
	let receiptDate = receipt.receiptDate ?? '';
	let liters: number | null = receipt.liters;
	let originalAmount: number | null = receipt.originalAmount;
	let originalCurrency: ReceiptCurrency | null = receipt.originalCurrency;
	let totalPriceEur: number | null = receipt.totalPriceEur;
	let stationName = receipt.stationName ?? '';
	let vendorName = receipt.vendorName ?? '';
	let costDescription = receipt.costDescription ?? '';

	// Derived state
	$: isFuelReceipt = liters !== null;
	$: isForeignCurrency = originalCurrency !== null && originalCurrency !== 'EUR';

	// When currency changes to EUR, sync totalPriceEur with originalAmount
	$: if (originalCurrency === 'EUR' && originalAmount !== null) {
		totalPriceEur = originalAmount;
	}

	// Validation
	$: eurAmountRequired = isForeignCurrency && totalPriceEur === null;
	$: canSave = !eurAmountRequired;

	function handleCurrencyChange(event: Event) {
		const select = event.target as HTMLSelectElement;
		const value = select.value;
		originalCurrency = value === '' ? null : (value as ReceiptCurrency);

		// If switching to EUR, auto-copy original amount
		if (originalCurrency === 'EUR' && originalAmount !== null) {
			totalPriceEur = originalAmount;
		}
		// If switching to foreign currency, clear EUR amount so user must enter it
		if (originalCurrency !== null && originalCurrency !== 'EUR') {
			totalPriceEur = null;
		}
	}

	function handleSave() {
		if (!canSave) return;

		onSave({
			receiptDate: receiptDate || null,
			liters: liters,
			originalAmount: originalAmount,
			originalCurrency: originalCurrency,
			totalPriceEur: totalPriceEur,
			stationName: stationName || null,
			vendorName: vendorName || null,
			costDescription: costDescription || null,
		});
	}

	function handleBackgroundClick(event: MouseEvent) {
		if (event.target === event.currentTarget) {
			onClose();
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			onClose();
		}
	}
</script>

<div class="modal-backdrop" on:click={handleBackgroundClick} on:keydown={handleKeydown} role="button" tabindex="-1">
	<div class="modal-content">
		<div class="modal-header">
			<h2>{$LL.receiptEdit.title()}</h2>
			<button class="close-button" on:click={onClose}>&times;</button>
		</div>

		<div class="modal-body">
			<!-- File name (read-only info) -->
			<div class="file-info">
				<span class="file-icon">{isFuelReceipt ? '\u26FD' : '\uD83D\uDCC4'}</span>
				<span class="file-name">{receipt.fileName}</span>
			</div>

			<!-- Date -->
			<div class="form-group">
				<label for="receipt-date">{$LL.receiptEdit.date()}</label>
				<input
					type="date"
					id="receipt-date"
					bind:value={receiptDate}
				/>
			</div>

			<!-- Liters (fuel receipts only) -->
			{#if isFuelReceipt || receipt.liters !== null}
				<div class="form-group">
					<label for="liters">{$LL.receiptEdit.liters()}</label>
					<input
						type="number"
						id="liters"
						bind:value={liters}
						step="0.01"
						min="0"
						placeholder="0.00"
					/>
				</div>
			{/if}

			<!-- Currency Section -->
			<div class="section-header">{$LL.receiptEdit.amountSection()}</div>

			<div class="form-row">
				<!-- Original Amount -->
				<div class="form-group flex-1">
					<label for="original-amount">{$LL.receiptEdit.originalAmount()}</label>
					<input
						type="number"
						id="original-amount"
						bind:value={originalAmount}
						step="0.01"
						min="0"
						placeholder="0.00"
					/>
				</div>

				<!-- Currency -->
				<div class="form-group currency-select">
					<label for="currency">{$LL.receiptEdit.currency()}</label>
					<select id="currency" value={originalCurrency ?? ''} on:change={handleCurrencyChange}>
						<option value="">--</option>
						<option value="EUR">EUR</option>
						<option value="CZK">CZK</option>
						<option value="HUF">HUF</option>
						<option value="PLN">PLN</option>
					</select>
				</div>
			</div>

			<!-- EUR Amount (for foreign currency) -->
			{#if isForeignCurrency}
				<div class="form-group">
					<label for="total-price-eur" class:required={eurAmountRequired}>
						{$LL.receiptEdit.eurAmount()}
					</label>
					<input
						type="number"
						id="total-price-eur"
						bind:value={totalPriceEur}
						step="0.01"
						min="0"
						placeholder="0.00"
						class:error={eurAmountRequired}
					/>
					{#if eurAmountRequired}
						<span class="hint error">{$LL.receiptEdit.eurAmountRequired()}</span>
					{/if}
				</div>
			{/if}

			<!-- Station/Vendor info -->
			{#if isFuelReceipt}
				<div class="form-group">
					<label for="station-name">{$LL.receiptEdit.stationName()}</label>
					<input
						type="text"
						id="station-name"
						bind:value={stationName}
						placeholder={$LL.receiptEdit.stationNamePlaceholder()}
					/>
				</div>
			{:else}
				<div class="form-group">
					<label for="vendor-name">{$LL.receiptEdit.vendorName()}</label>
					<input
						type="text"
						id="vendor-name"
						bind:value={vendorName}
						placeholder={$LL.receiptEdit.vendorNamePlaceholder()}
					/>
				</div>
				<div class="form-group">
					<label for="cost-description">{$LL.receiptEdit.costDescription()}</label>
					<input
						type="text"
						id="cost-description"
						bind:value={costDescription}
						placeholder={$LL.receiptEdit.costDescriptionPlaceholder()}
					/>
				</div>
			{/if}
		</div>

		<div class="modal-footer">
			<button class="button button-secondary" on:click={onClose}>{$LL.common.cancel()}</button>
			<button class="button button-primary" on:click={handleSave} disabled={!canSave}>
				{$LL.common.save()}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background-color: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-content {
		background: var(--bg-surface);
		border-radius: 8px;
		width: 90%;
		max-width: 450px;
		max-height: 90vh;
		overflow-y: auto;
		box-shadow: 0 4px 12px var(--shadow-default);
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border-default);
	}

	.modal-header h2 {
		margin: 0;
		font-size: 1.25rem;
		color: var(--text-primary);
	}

	.close-button {
		background: none;
		border: none;
		font-size: 2rem;
		color: var(--text-secondary);
		cursor: pointer;
		padding: 0;
		width: 2rem;
		height: 2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		line-height: 1;
	}

	.close-button:hover {
		color: var(--text-primary);
	}

	.modal-body {
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.file-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.75rem;
		background: var(--bg-surface-alt);
		border-radius: 4px;
		font-size: 0.875rem;
	}

	.file-icon {
		font-size: 1.25rem;
	}

	.file-name {
		color: var(--text-primary);
		font-weight: 500;
		word-break: break-all;
	}

	.section-header {
		font-weight: 600;
		color: var(--text-primary);
		font-size: 0.9rem;
		margin-top: 0.5rem;
		padding-bottom: 0.25rem;
		border-bottom: 1px solid var(--border-default);
	}

	.form-row {
		display: flex;
		gap: 1rem;
	}

	.flex-1 {
		flex: 1;
	}

	.currency-select {
		width: 100px;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.form-group label {
		font-weight: 500;
		color: var(--text-primary);
		font-size: 0.875rem;
	}

	.form-group label.required::after {
		content: ' *';
		color: var(--accent-danger);
	}

	.form-group input,
	.form-group select {
		padding: 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 1rem;
		font-family: inherit;
		background-color: var(--bg-surface);
		color: var(--text-primary);
	}

	.form-group input:focus,
	.form-group select:focus {
		outline: none;
		border-color: var(--accent-primary);
		box-shadow: 0 0 0 3px var(--input-focus-shadow);
	}

	.form-group input.error {
		border-color: var(--accent-danger);
	}

	.hint {
		font-size: 0.75rem;
		color: var(--text-secondary);
		font-style: italic;
	}

	.hint.error {
		color: var(--accent-danger);
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.75rem;
		padding: 1.5rem;
		border-top: 1px solid var(--border-default);
	}

	.button {
		padding: 0.75rem 1.5rem;
		border: none;
		border-radius: 4px;
		font-weight: 500;
		cursor: pointer;
		transition: all 0.2s;
		font-size: 1rem;
	}

	.button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.button-secondary {
		background-color: var(--btn-secondary-bg);
		color: var(--text-primary);
	}

	.button-secondary:hover:not(:disabled) {
		background-color: var(--btn-secondary-hover);
	}

	.button-primary {
		background-color: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
	}

	.button-primary:hover:not(:disabled) {
		background-color: var(--btn-active-primary-hover);
	}
</style>
