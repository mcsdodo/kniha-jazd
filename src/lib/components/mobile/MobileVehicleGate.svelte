<script lang="ts">
	// Shows the page body only when there is an active vehicle (Task 93).
	// The root layout sets the vehicle store after its startup, so an empty
	// store first means "still loading". The gate asks the backend once: no
	// vehicle at all -> a message with a link to the settings.
	import { onMount, type Snippet } from 'svelte';
	import { getVehicles } from '$lib/api';
	import { activeVehicleStore } from '$lib/stores/vehicles';
	import LL from '$lib/i18n/i18n-svelte';

	let { children }: { children: Snippet } = $props();

	let noVehicle = $state(false);

	onMount(async () => {
		try {
			noVehicle = (await getVehicles()).length === 0;
		} catch (error) {
			console.error('Failed to load vehicles:', error);
		}
	});
</script>

{#if $activeVehicleStore}
	{@render children()}
{:else if noVehicle}
	<div class="gate" data-testid="mobile-no-vehicle">
		<p>{$LL.mobile.home.noVehicle()}</p>
		<a class="link-button" href="/settings">{$LL.mobile.home.goToSettings()}</a>
	</div>
{:else}
	<p class="gate muted" data-testid="mobile-loading">{$LL.mobile.home.loading()}</p>
{/if}

<style>
	.gate {
		padding: 1.5rem 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		margin: 0;
	}

	.muted {
		color: var(--text-secondary);
	}

	.link-button {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 44px;
		padding: 0 1rem;
		border-radius: 10px;
		background: var(--accent-primary);
		color: #fff;
		font-weight: 600;
		text-decoration: none;
		align-self: flex-start;
	}
</style>
