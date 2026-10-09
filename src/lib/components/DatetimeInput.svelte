<script lang="ts">
	// Typed start/end of a trip in the edit row: "DD.MM HH:MM", the year comes
	// from the value the field holds. `value` stays "YYYY-MM-DDTHH:MM".
	import { createEventDispatcher } from 'svelte';
	import LL from '$lib/i18n/i18n-svelte';
	import { formatDatetimeInput, parseDatetimeInput } from '$lib/datetimeInput';

	export let value: string;
	// Set for the END of a trip: gives the year and the lower limit.
	export let start: string | null = null;
	export let showPicker = true;
	export let testid: string;
	// True while the typed text is not a valid date and time. The row must not save.
	export let invalid = false;

	const dispatch = createEventDispatcher<{ change: string }>();

	let text = formatDatetimeInput(value);
	let shownValue = value;
	let focused = false;
	// The value when the user started to type. Parse against it, not against
	// each partial result: "15" (a time) on the way to "15.3" (a date) must not
	// leave 15:00 behind as the time.
	let base = value;
	let pickerInput: HTMLInputElement;

	// Other code also writes the value (end follows start, copy row, time
	// inference). Show it, unless the user types in this field now.
	$: if (value !== shownValue) {
		shownValue = value;
		if (!focused) {
			text = formatDatetimeInput(value);
			invalid = false;
		}
	}

	function commit(next: string) {
		if (next === value) return;
		shownValue = next;
		value = next;
		dispatch('change', next);
	}

	function handleFocus() {
		focused = true;
		base = value;
	}

	function handleInput() {
		const parsed = parseDatetimeInput(text, focused ? base : value, start);
		invalid = parsed === null;
		if (parsed) commit(parsed);
	}

	// Blur, and a `change` without focus (a script set the value): show the
	// normal form. Invalid text stays, so the user can correct it.
	function handleChange() {
		if (!invalid) text = formatDatetimeInput(value);
	}

	function handleBlur() {
		focused = false;
		handleChange();
	}

	function openPicker() {
		pickerInput.value = value;
		try {
			pickerInput.showPicker();
		} catch {
			pickerInput.click();
		}
	}

	function handlePicked() {
		if (!pickerInput.value) return;
		text = formatDatetimeInput(pickerInput.value);
		invalid = false;
		commit(pickerInput.value);
	}
</script>

<div class="datetime-input">
	<input
		type="text"
		inputmode="numeric"
		autocomplete="off"
		placeholder="DD.MM HH:MM"
		bind:value={text}
		on:focus={handleFocus}
		on:input={handleInput}
		on:change={handleChange}
		on:blur={handleBlur}
		class:invalid
		aria-invalid={invalid}
		data-testid={testid}
		data-value={value}
	/>
	{#if showPicker}
		<button
			type="button"
			class="picker-button"
			tabindex="-1"
			title={$LL.trips.pickDatetime()}
			aria-label={$LL.trips.pickDatetime()}
			on:click={openPicker}
			data-testid="{testid}-picker"
		>
			<svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<rect x="3" y="4" width="18" height="18" rx="2" />
				<line x1="16" y1="2" x2="16" y2="6" />
				<line x1="8" y1="2" x2="8" y2="6" />
				<line x1="3" y1="10" x2="21" y2="10" />
			</svg>
		</button>
		<!-- Not display:none: showPicker() needs an element that is rendered. -->
		<input
			type="datetime-local"
			class="picker-native"
			tabindex="-1"
			aria-hidden="true"
			bind:this={pickerInput}
			on:change={handlePicked}
		/>
	{/if}
</div>

<style>
	.datetime-input {
		position: relative;
		display: flex;
		align-items: center;
		gap: 2px;
		margin: 0 1px;
	}

	input[type='text'] {
		flex: 1;
		min-width: 0;
		padding: 0.5rem 0.125rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 0.875rem;
		font-variant-numeric: tabular-nums;
		box-sizing: border-box;
	}

	input.invalid {
		border-color: var(--accent-danger);
		outline: 1px solid var(--accent-danger);
	}

	.picker-button {
		flex: none;
		padding: 0 0.125rem;
		border: none;
		background: none;
		cursor: pointer;
		color: var(--text-secondary);
		line-height: 0;
	}

	.picker-button:hover {
		color: var(--accent-primary);
	}

	.picker-native {
		position: absolute;
		left: 0;
		bottom: 0;
		width: 1px;
		height: 1px;
		padding: 0;
		border: 0;
		opacity: 0;
		pointer-events: none;
	}
</style>
