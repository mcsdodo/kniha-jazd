<script lang="ts">
	import { onMount } from 'svelte';
	import LL from '$lib/i18n/i18n-svelte';
	import type { CascadePlan, Trip } from '$lib/types';

	export let plan: CascadePlan;
	export let trips: Trip[];
	/** The distance the edited row recorded before this change. Unused for
	 *  'insert' and 'delete', where there is no before-and-after distance. */
	export let oldDistanceKm: number = 0;
	/** Which write is being confirmed. It picks the summary and the breakdown
	 *  line, and it is the only thing that differs between the three. */
	export let kind: 'edit' | 'insert' | 'delete' = 'edit';
	export let onConfirm: () => void;
	export let onCancel: () => void;

	// The plan names rows by id only. The date and the route come from the
	// trips the grid already holds, so the backend does not repeat them.
	$: byId = new Map(trips.map((t) => [t.id, t]));

	// Nothing else moves focus into the dialog, so without this a keydown
	// still targets whatever had focus before the modal opened - typically
	// the row being saved - and neither keydown handler below ever sees it
	// (task 81, fix round 2).
	let modalEl: HTMLDivElement;
	// The element the modal stole focus from on mount (task 81, fix round 3)
	// -- usually the Save/Delete button or whatever input the user was in.
	// Restored on both Cancel and Confirm so the caret does not land on
	// <body>, forcing the user to click back into the row.
	let previouslyFocused: HTMLElement | null = null;
	onMount(() => {
		previouslyFocused = document.activeElement instanceof HTMLElement ? document.activeElement : null;
		modalEl?.focus();
	});

	function restoreFocus() {
		previouslyFocused?.focus();
	}

	// Every path that closes the modal - the Cancel button, the overlay
	// click, and Escape - must go through this, not the bare `onCancel`
	// prop, or focus stays on the (about to be destroyed) modal.
	function handleCancel() {
		restoreFocus();
		onCancel();
	}

	function handleConfirm() {
		restoreFocus();
		onConfirm();
	}

	// Stops here unconditionally: TripRow's window-level ESC/Enter handler
	// must never see a key pressed while this modal owns the decision, or
	// ESC would revert the row's form while the modal stays armed and a
	// later Confirm would still write it.
	function handleKeydown(event: KeyboardEvent) {
		event.stopPropagation();
		if (event.key === 'Escape') {
			handleCancel();
		}
	}

	function signed(value: number): string {
		return `${value >= 0 ? '+' : ''}${value.toFixed(1)}`;
	}

	function shortDate(trip: Trip | undefined): string {
		if (!trip) return '';
		const [year, month, day] = trip.startDatetime.slice(0, 10).split('-');
		return `${day}.${month}.${year}`;
	}

	// The repair line is shown only when the edited row was already broken.
	$: showRepair = Math.abs(plan.deltaFromRepair) >= 0.001;
</script>

<div
	class="modal-overlay"
	on:click={handleCancel}
	on:keydown={handleKeydown}
	role="button"
	tabindex="0"
>
	<div
		class="modal"
		bind:this={modalEl}
		on:click|stopPropagation
		on:keydown={handleKeydown}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		data-testid="cascade-modal"
	>
		<h2>{$LL.trips.cascade.title()}</h2>
		<div class="modal-content">
			<p class="summary" data-testid="cascade-summary">
				{#if kind === 'insert'}
					{$LL.trips.cascade.summaryInsert({
						count: plan.changes.length,
						delta: signed(plan.delta)
					})}
				{:else if kind === 'delete'}
					{$LL.trips.cascade.summaryDelete({
						count: plan.changes.length,
						delta: signed(plan.delta)
					})}
				{:else}
					{$LL.trips.cascade.summary({
						count: plan.changes.length,
						delta: signed(plan.delta)
					})}
				{/if}
			</p>
			<ul class="breakdown">
				<li>
					{#if kind === 'insert'}
						{$LL.trips.cascade.fromInsert({ delta: signed(plan.deltaFromDistance) })}
					{:else if kind === 'delete'}
						{$LL.trips.cascade.fromDelete({ delta: signed(plan.deltaFromDistance) })}
					{:else}
						{$LL.trips.cascade.fromDistance({
							delta: signed(plan.deltaFromDistance),
							oldKm: oldDistanceKm.toFixed(0),
							newKm: plan.newDistanceKm.toFixed(0)
						})}
					{/if}
				</li>
				{#if showRepair}
					<li data-testid="cascade-repair">
						{plan.repairCrossesYear
							? $LL.trips.cascade.fromRepairCrossYear({ delta: signed(plan.deltaFromRepair) })
							: $LL.trips.cascade.fromRepair({ delta: signed(plan.deltaFromRepair) })}
					</li>
				{/if}
			</ul>
			{#if plan.nextYearChainBreaks}
				<p class="year-end" data-testid="cascade-year-end">
					{$LL.trips.cascade.yearEndWarning()}
				</p>
			{/if}
			<div class="changes">
				<table>
					<thead>
						<tr>
							<th>{$LL.trips.cascade.columnTrip()}</th>
							<th>{$LL.trips.cascade.columnDate()}</th>
							<th>{$LL.trips.cascade.columnRoute()}</th>
							<th class="number">{$LL.trips.cascade.columnOld()}</th>
							<th class="number">{$LL.trips.cascade.columnNew()}</th>
						</tr>
					</thead>
					<tbody>
						{#each plan.changes as change (change.tripId)}
							<tr>
								<td>{change.tripNumber}</td>
								<td>{shortDate(byId.get(change.tripId))}</td>
								<td class="route">
									{byId.get(change.tripId)?.origin ?? ''} -&gt;
									{byId.get(change.tripId)?.destination ?? ''}
								</td>
								<td class="number">{change.oldOdometer.toFixed(0)}</td>
								<td class="number">{change.newOdometer.toFixed(0)}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
		<div class="modal-actions">
			<button class="button-small" on:click={handleCancel} data-testid="cascade-cancel">
				{$LL.trips.cascade.cancel()}
			</button>
			<button class="button-small" on:click={handleConfirm} data-testid="cascade-confirm">
				{kind === 'delete' ? $LL.trips.cascade.confirmDelete() : $LL.trips.cascade.confirm()}
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

	.summary {
		font-weight: 600;
	}
	.breakdown {
		margin: 0 0 0.75rem 0;
		padding-left: 1.25rem;
	}
	.year-end {
		margin: 0 0 0.75rem 0;
	}
	.changes {
		max-height: 40vh;
		overflow-y: auto;
	}
	.changes table {
		width: 100%;
		border-collapse: collapse;
	}
	.changes th,
	.changes td {
		padding: 0.25rem 0.5rem;
		text-align: left;
	}
	.changes .number {
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.changes .route {
		max-width: 20rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
