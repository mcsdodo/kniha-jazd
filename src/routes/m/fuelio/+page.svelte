<script lang="ts">
	import { untrack } from 'svelte';
	import * as api from '$lib/api';
	import type { CascadePlan, FuelioAddEnd, FuelioAddPreview, FuelioRow, Trip } from '$lib/types';
	import LL from '$lib/i18n/i18n-svelte';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import { toast } from '$lib/stores/toast';
	import OdometerCascadeModal from '$lib/components/OdometerCascadeModal.svelte';
	import Autocomplete from '$lib/components/Autocomplete.svelte';
	import MobilePlacePicker from '$lib/components/mobile/MobilePlacePicker.svelte';
	import FuelioTrackMap from '$lib/components/mobile/FuelioTrackMap.svelte';

	// Task 93: the Fuelio drives without a trip, on the phone. The backend syncs,
	// matches and writes (ADR-008); this page lists the missing rows and runs the
	// same add flow as FuelioAddModal.svelte.

	// The phone shows the drives of this year: the sync also copies this year.
	const year = new Date().getFullYear();

	let syncing = $state(false);
	let syncedAt = $state<string | null>(null);
	let syncFailed = $state(0);
	let syncError = $state<string | null>(null);
	let loading = $state(false);
	let loadError = $state<string | null>(null);
	let rows = $state<FuelioRow[] | null>(null);
	let purposeSuggestions = $state<string[]>([]);
	// The manual joins (BIZ-066), as on /fuelio. Not saved.
	let joins = $state<string[]>([]);
	// The filters of the PC page: one saved set for both pages. null until read.
	let filters = $state<api.FuelioFilters | null>(null);
	let filtersOpen = $state(false);

	// The add form for one row.
	let adding = $state<FuelioRow | null>(null);
	let preview = $state<FuelioAddPreview | null>(null);
	let origin = $state<FuelioAddEnd>({ placeId: null, newPlaceName: null });
	let destination = $state<FuelioAddEnd>({ placeId: null, newPlaceName: null });
	let purpose = $state('');
	let withRoute = $state(true);
	let busy = $state(false);
	let formError = $state<string | null>(null);
	let plan = $state<CascadePlan | null>(null);
	let yearTrips = $state<Trip[]>([]);

	// The layout loads the vehicle after this page mounts: start from the store.
	let started = false;
	$effect(() => {
		const vehicle = $activeVehicleStore;
		if (!vehicle) return;
		untrack(() => {
			api.getPurposes(vehicle.id).then(
				(p) => (purposeSuggestions = p),
				(e) => console.error('Failed to load purposes:', e)
			);
			if (!started) {
				started = true;
				syncThenLoad(vehicle.id);
			} else {
				load(vehicle.id);
			}
		});
	});

	// The local drives show at once. The sync then runs behind the list: it
	// downloads only the files that are not local yet, and the list loads
	// again only if a new file came. The list never goes blank for the sync.
	async function syncThenLoad(vehicleId: string) {
		syncing = true;
		syncError = null;
		const local = load(vehicleId);
		try {
			const r = await api.syncFuelioDropbox(year);
			syncFailed = r.failed.length;
			syncedAt = new Date().toTimeString().slice(0, 5);
			await local;
			if (r.downloaded > 0) await load(vehicleId);
		} catch (e) {
			// The local drives still show.
			syncError = String(e);
		} finally {
			syncing = false;
		}
	}

	// Only the newest load may set the rows.
	let loadSeq = 0;
	async function load(vehicleId: string) {
		const seq = ++loadSeq;
		loading = true;
		loadError = null;
		try {
			// Read once: a later load must not overwrite a field that the user edits.
			if (!filters) filters = await api.getFuelioFilters().catch(() => null);
			const r = await api.getFuelioCrosscheck(
				vehicleId,
				year,
				{
					maxGapMin: filters?.mergeGapMin ?? null,
					maxJumpKm: filters?.mergeJumpKm ?? null,
					looseMinKmPct: filters?.looseMinKmPct ?? null
				},
				[...joins]
			);
			// The backend sorts newest first.
			// The saved min-km filter hides short fragments, as on /fuelio.
			// Display filtering only.
			const minKm = filters?.minKm || 0;
			if (seq === loadSeq)
				rows = r.rows.filter(
					(row) => row.status === 'missing' && !row.ignored && (row.gpsKm ?? 0) >= minKm
				);
		} catch (e) {
			if (seq === loadSeq) loadError = String(e);
		} finally {
			if (seq === loadSeq) loading = false;
		}
	}

	// An empty number input binds as undefined: save it as null. Save first, so
	// the new report uses the saved values.
	async function saveFilters() {
		const vehicle = $activeVehicleStore;
		if (!filters || !vehicle) return;
		try {
			await api.setFuelioFilters({
				minKm: filters.minKm ?? null,
				mergeGapMin: filters.mergeGapMin ?? null,
				mergeJumpKm: filters.mergeJumpKm ?? null,
				looseMinKmPct: filters.looseMinKmPct ?? null
			});
		} catch (e) {
			toast.error(String(e));
			return;
		}
		await load(vehicle.id);
	}

	function shown(value: number | null | undefined): string {
		return value === null || value === undefined ? '-' : String(value);
	}

	function changeJoins(next: string[]) {
		joins = next;
		if ($activeVehicleStore) load($activeVehicleStore.id);
	}

	async function openAdd(row: FuelioRow) {
		adding = row;
		preview = null;
		formError = null;
		purpose = '';
		withRoute = true;
		origin = { placeId: null, newPlaceName: null };
		destination = { placeId: null, newPlaceName: null };
		window.scrollTo(0, 0);
		try {
			preview = await api.getFuelioAddPreview(row.driveIds);
		} catch (e) {
			formError = String(e);
		}
	}

	function closeAdd() {
		adding = null;
		preview = null;
		plan = null;
	}

	// Display formatting of backend values.
	function date(s: string | null): string {
		if (!s) return '-';
		const [y, m, d] = s.slice(0, 10).split('-');
		return `${Number(d)}.${Number(m)}.${y}`;
	}
	function time(s: string | null): string {
		return s ? s.slice(11, 16) : '-';
	}

	// The picker gives '' while the user types a new name: send a trimmed name.
	function end(v: FuelioAddEnd): FuelioAddEnd {
		return v.placeId ? { placeId: v.placeId, newPlaceName: null } : { placeId: null, newPlaceName: v.newPlaceName?.trim() ?? null };
	}
	function chosen(v: FuelioAddEnd): boolean {
		return !!v.placeId || !!v.newPlaceName?.trim();
	}

	function call(dryRun: boolean) {
		const vehicle = $activeVehicleStore!;
		return api.addFuelioTrip(vehicle.id, adding!.driveIds, end(origin), end(destination), purpose, withRoute, dryRun);
	}

	async function save() {
		if (!adding || !preview || !$activeVehicleStore) return;
		if (!chosen(origin)) {
			formError = $LL.mobile.add.missingOrigin();
			return;
		}
		if (!chosen(destination)) {
			formError = $LL.mobile.add.missingDestination();
			return;
		}
		busy = true;
		formError = null;
		try {
			const dry = await call(true);
			if (dry.plan.changes.length > 0 || dry.plan.nextYearChainBreaks) {
				// The trips of the year the drive ends in: the plan names them by id.
				yearTrips = await api.getTripsForYear($activeVehicleStore.id, Number(preview.end.slice(0, 4)));
				plan = dry.plan;
			} else {
				await commit();
			}
		} catch (e) {
			formError = String(e);
		} finally {
			busy = false;
		}
	}

	async function commit() {
		busy = true;
		formError = null;
		try {
			const r = await call(false);
			plan = null;
			// The trip exists even when its route failed: close and reload.
			if (r.routeError) {
				toast.error($LL.fuelio.add.routeFailed({ error: r.routeError }));
			} else {
				toast.success($LL.mobile.add.done());
			}
			closeAdd();
			if ($activeVehicleStore) await load($activeVehicleStore.id);
		} catch (e) {
			plan = null;
			formError = String(e);
		} finally {
			busy = false;
		}
	}
</script>

<div class="page" data-testid="m-fuelio">
	<header class="bar">
		{#if adding}
			<button type="button" class="back" onclick={closeAdd} data-testid="m-fuelio-back">{$LL.mobile.add.back()}</button>
			<h1>{$LL.mobile.add.title()}</h1>
		{:else}
			<a class="back" href="/m" data-testid="m-fuelio-back">{$LL.mobile.add.back()}</a>
			<h1>{$LL.mobile.add.fuelioTitle()}</h1>
		{/if}
	</header>

	{#if !$activeVehicleStore}
		<p class="muted">{$LL.mobile.add.noVehicle()}</p>
	{:else if adding}
		<section class="form" data-testid="m-fuelio-add">
			{#if !preview && !formError}
				<p class="muted">{$LL.mobile.add.loading()}</p>
			{:else if preview}
				<div class="summary" data-testid="m-fuelio-add-summary">
					<strong>{date(preview.start)}</strong>
					<span>{time(preview.start)} - {time(preview.end)}</span>
					<strong>{preview.distanceKm} km</strong>
					<span class="muted">({$LL.mobile.add.gpsNote({ km: preview.gpsKm.toFixed(1) })})</span>
				</div>

				<FuelioTrackMap driveIds={adding.driveIds} />

				<MobilePlacePicker
					testId="m-fuelio-origin"
					label={$LL.mobile.add.origin()}
					options={preview.origin}
					preselectId={preview.originPreselectId}
					newHint={$LL.mobile.add.picker.pinFuelioStart()}
					bind:value={origin}
				/>
				<MobilePlacePicker
					testId="m-fuelio-destination"
					label={$LL.mobile.add.destination()}
					options={preview.destination}
					preselectId={preview.destinationPreselectId}
					newHint={$LL.mobile.add.picker.pinFuelioEnd()}
					bind:value={destination}
				/>

				<div class="field-group purpose">
					<label for="m-fuelio-purpose">{$LL.mobile.add.purpose()}</label>
					<Autocomplete
						id="m-fuelio-purpose"
						bind:value={purpose}
						suggestions={purposeSuggestions}
						onSelect={(v) => (purpose = v)}
						testId="m-fuelio-purpose"
					/>
				</div>

				<label class="check">
					<input type="checkbox" bind:checked={withRoute} data-testid="m-fuelio-route" />
					{$LL.mobile.add.route()}
				</label>
			{/if}

			{#if formError}
				<p class="error" role="alert" data-testid="m-fuelio-add-error">{formError}</p>
			{/if}

			<button
				type="button"
				class="primary"
				disabled={!preview || busy}
				onclick={save}
				data-testid="m-fuelio-save">{busy ? $LL.mobile.add.saving() : $LL.mobile.add.save()}</button
			>
		</section>
	{:else}
		<p class="status" data-testid="m-fuelio-sync">
			{#if syncing}
				{$LL.mobile.add.syncing()}
			{:else if syncError}
				<span class="error">{$LL.mobile.add.syncError({ error: syncError })}</span>
			{:else if syncedAt}
				{$LL.mobile.add.synced({ time: syncedAt })}
				{#if syncFailed > 0}
					<span class="error">{$LL.mobile.add.syncFailedFiles({ count: syncFailed })}</span>
				{/if}
			{/if}
		</p>

		{#if loadError}
			<p class="error" role="alert" data-testid="m-fuelio-error">{$LL.mobile.add.loadError({ error: loadError })}</p>
		{/if}

		{#if rows && rows.length > 0}
			<p class="muted small">{$LL.mobile.add.fuelioIntro()}</p>
		{/if}

		<!-- Outside the list branches: a filter that hides every row must not hide this. -->
		{#if filters}
			{#if filtersOpen}
				<section class="filters">
					<div class="filters-head">
						<button
							type="button"
							class="link-btn"
							onclick={() => (filtersOpen = false)}
							data-testid="m-fuelio-filters-toggle">{$LL.mobile.add.filtersHide()}</button
						>
					</div>
					<label class="filter">
						{$LL.fuelio.minKm()}
						<input type="number" min="0" inputmode="decimal" bind:value={filters.minKm} onchange={saveFilters} data-testid="m-fuelio-filter-min-km" />
					</label>
					<div class="two">
						<label class="filter">
							{$LL.fuelio.mergeGap()}
							<input type="number" min="0" step="5" inputmode="numeric" bind:value={filters.mergeGapMin} onchange={saveFilters} data-testid="m-fuelio-filter-gap" />
						</label>
						<label class="filter">
							{$LL.fuelio.mergeJump()}
							<input type="number" min="0" step="0.5" inputmode="decimal" bind:value={filters.mergeJumpKm} onchange={saveFilters} data-testid="m-fuelio-filter-jump" />
						</label>
					</div>
					<p class="muted small">{$LL.fuelio.mergeHint()}</p>
					<label class="filter">
						{$LL.fuelio.looseMin()}
						<input type="number" min="0" step="10" inputmode="decimal" bind:value={filters.looseMinKmPct} onchange={saveFilters} data-testid="m-fuelio-filter-loose" />
					</label>
					<p class="muted small">{$LL.fuelio.looseMinHint()}</p>
				</section>
			{:else}
				<div class="filters-line">
					<span class="muted small" data-testid="m-fuelio-filters-summary"
						>{$LL.mobile.add.filtersSummary({
							minKm: shown(filters.minKm),
							gap: shown(filters.mergeGapMin),
							jump: shown(filters.mergeJumpKm),
							loose: shown(filters.looseMinKmPct)
						})}</span
					>
					<button
						type="button"
						class="link-btn"
						onclick={() => (filtersOpen = true)}
						data-testid="m-fuelio-filters-toggle">{$LL.mobile.add.filtersEdit()}</button
					>
				</div>
			{/if}
		{/if}

		{#if rows === null}
			{#if loading}
				<p class="muted">{$LL.mobile.add.loading()}</p>
			{/if}
		{:else if rows.length === 0}
			<p class="muted" data-testid="m-fuelio-empty">{$LL.mobile.add.noMissing()}</p>
		{:else}
			<ul class="cards">
				{#each rows as row (row.driveIds.join(','))}
					<li>
						<button type="button" class="card" onclick={() => openAdd(row)} data-testid="m-fuelio-row">
							<span class="line">
								<strong>{date(row.gpsStart)}</strong>
								<span>{time(row.gpsStart)} - {time(row.gpsEnd)}</span>
								<strong class="km">{row.gpsKm !== null ? `${row.gpsKm.toFixed(1)} km` : '-'}</strong>
							</span>
							<span class="add">{$LL.mobile.add.addButton()} &gt;</span>
						</button>
						{#if row.canJoinNext || row.joined}
							<div class="join-actions">
								{#if row.canJoinNext}
									<button
										type="button"
										class="link-btn"
										onclick={() => changeJoins([...joins, row.driveIds[row.driveIds.length - 1]])}
										data-testid="m-fuelio-join-next">{$LL.fuelio.join.next()}</button
									>
								{/if}
								{#if row.joined}
									<button
										type="button"
										class="link-btn"
										onclick={() => changeJoins(joins.filter((id) => !row.driveIds.includes(id)))}
										data-testid="m-fuelio-split">{$LL.fuelio.join.split()}</button
									>
								{/if}
							</div>
						{/if}
					</li>
				{/each}
			</ul>
		{/if}
	{/if}
</div>

{#if plan}
	<OdometerCascadeModal
		kind="insert"
		{plan}
		trips={yearTrips}
		oldDistanceKm={0}
		onConfirm={commit}
		onCancel={() => (plan = null)}
	/>
{/if}

<style>
	.page {
		box-sizing: border-box;
		max-width: 640px;
		margin: 0 auto;
		padding: 0 16px 32px;
		color: var(--text-primary);
	}
	.bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		min-height: 52px;
		border-bottom: 1px solid var(--border-default);
		margin-bottom: 0.75rem;
	}
	.bar h1 {
		margin: 0;
		font-size: 1.15rem;
	}
	.back {
		display: inline-flex;
		align-items: center;
		min-height: 44px;
		padding: 0 0.5rem 0 0;
		border: none;
		background: none;
		color: var(--accent-primary);
		font-size: 1rem;
		text-decoration: none;
		cursor: pointer;
	}
	.status {
		min-height: 1.2rem;
		margin: 0 0 0.5rem;
		font-size: 0.9rem;
		color: var(--text-secondary);
	}
	.muted {
		color: var(--text-secondary);
	}
	.small {
		font-size: 0.85rem;
	}
	.error {
		color: var(--accent-danger);
		font-size: 0.9rem;
	}
	.cards {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
	}
	.card {
		box-sizing: border-box;
		width: 100%;
		min-height: 56px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		border: 1px solid var(--border-default);
		border-radius: 8px;
		background: var(--bg-surface);
		color: var(--text-primary);
		font-size: 1rem;
		text-align: left;
		cursor: pointer;
	}
	.filters-line {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		margin-bottom: 0.5rem;
	}
	.filters {
		display: flex;
		flex-direction: column;
		gap: 0.6rem;
		margin-bottom: 0.75rem;
		padding: 0.5rem 0.75rem 0.75rem;
		border: 1px solid var(--border-default);
		border-radius: 8px;
		background: var(--bg-surface);
	}
	.filters p {
		margin: 0;
	}
	.filters-head {
		display: flex;
		justify-content: flex-end;
	}
	.two {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
		align-items: end;
	}
	.filter {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		font-size: 0.9rem;
		font-weight: 600;
	}
	.filter input {
		box-sizing: border-box;
		width: 100%;
		min-height: 44px;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 6px;
		background: var(--input-bg);
		color: var(--text-primary);
		font-size: 1rem;
	}
	.join-actions {
		display: flex;
		justify-content: flex-end;
		gap: 1rem;
	}
	.link-btn {
		min-height: 44px;
		padding: 0 0.25rem;
		border: none;
		background: none;
		color: var(--accent-primary);
		font-size: 0.95rem;
		cursor: pointer;
	}
	.line {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem 0.75rem;
	}
	.add {
		color: var(--accent-primary);
		white-space: nowrap;
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.summary {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem 0.75rem;
		padding: 0.75rem 1rem;
		border-radius: 8px;
		background: var(--bg-surface-alt);
	}
	.field-group {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.field-group label {
		font-weight: 600;
		font-size: 0.95rem;
	}
	.purpose :global(input) {
		box-sizing: border-box;
		width: 100%;
		min-height: 44px;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--border-input);
		border-radius: 6px;
		background: var(--input-bg);
		color: var(--text-primary);
		font-size: 1rem;
	}
	.check {
		display: flex;
		align-items: center;
		gap: 0.6rem;
		min-height: 44px;
		font-size: 1rem;
	}
	.check input {
		width: 22px;
		height: 22px;
	}
	.primary {
		min-height: 48px;
		border: none;
		border-radius: 8px;
		background: var(--btn-active-primary-bg);
		color: var(--btn-active-primary-color);
		font-size: 1.05rem;
		font-weight: 600;
		cursor: pointer;
	}
	.primary:disabled {
		opacity: 0.5;
		cursor: default;
	}
</style>
