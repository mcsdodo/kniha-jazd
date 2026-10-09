<script lang="ts">
	// Typed start/end of a trip in the edit row: "DD.MM HH:MM", the year comes
	// from the value the field holds. `value` stays "YYYY-MM-DDTHH:MM".
	//
	// Keyboard: like the native input, it edits one part at a time (day,
	// month, hour, minute). Tab / Shift+Tab move between the parts and leave
	// the field after the last one; arrows up/down change the selected part.
	// Typing "9.10 1430" still works: a separator or a full part moves on.
	import { createEventDispatcher } from 'svelte';
	import LL from '$lib/i18n/i18n-svelte';
	import {
		formatDatetimeInput,
		parseDatetimeInput,
		SEGMENT_RANGES,
		segmentAt,
		startSegments,
		segmentText,
		moveSegment,
		typeKey,
		stepSegment,
		type SegmentState
	} from '$lib/datetimeInput';

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
	let input: HTMLInputElement;
	// Part-by-part editing state while focused. Null = free text (the text is
	// invalid, or a paste or a script wrote it): `handleInput` parses it then.
	let segments: SegmentState | null = null;
	// Set on mousedown: the click then selects the part under the pointer.
	let pointerFocus = false;

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

	function selectSegment() {
		if (!segments || !input || document.activeElement !== input) return;
		const [from, to] = SEGMENT_RANGES[segments.seg];
		input.setSelectionRange(from, to);
	}

	function applySegments(next: SegmentState) {
		segments = next;
		text = segmentText(next);
		// Write the DOM now, so the selection below applies to the new text.
		input.value = text;
		const parsed = parseDatetimeInput(text, next.base, start);
		invalid = parsed === null;
		if (parsed) commit(parsed);
		selectSegment();
	}

	function handleFocus(event: FocusEvent) {
		focused = true;
		base = value;
		if (invalid) {
			segments = null;
			return;
		}
		// Shift+Tab from a later field selects the last part, as the native input does.
		const from = event.relatedTarget;
		const fromLater =
			from instanceof Node && (from.compareDocumentPosition(input) & Node.DOCUMENT_POSITION_PRECEDING) !== 0;
		segments = startSegments(value, fromLater ? 3 : 0);
		text = segmentText(segments);
		input.value = text;
		if (!pointerFocus) selectSegment();
	}

	function handleMouseDown() {
		pointerFocus = true;
	}

	function handleClick() {
		pointerFocus = false;
		if (!segments) return;
		// A drag selection is the user's own; leave it.
		if (input.selectionStart !== input.selectionEnd) return;
		segments = moveSegment(segments, segmentAt(input.selectionStart ?? 0));
		selectSegment();
	}

	function handleKeydown(event: KeyboardEvent) {
		if (!segments) return;
		// Mobile keyboards send "Unidentified": let the text go to handleInput.
		if (event.isComposing || event.key === 'Unidentified') return;
		if (event.ctrlKey || event.metaKey || event.altKey) return;
		const key = event.key;
		const allSelected = input.selectionStart === 0 && input.selectionEnd === input.value.length;

		if (key === 'Tab') {
			const next = segments.seg + (event.shiftKey ? -1 : 1);
			if (next < 0 || next > 3) return; // leave the field
			event.preventDefault();
			segments = moveSegment(segments, next);
			selectSegment();
		} else if (key === 'ArrowLeft' || key === 'ArrowRight') {
			event.preventDefault();
			segments = moveSegment(segments, segments.seg + (key === 'ArrowLeft' ? -1 : 1));
			selectSegment();
		} else if (key === 'ArrowUp' || key === 'ArrowDown') {
			event.preventDefault();
			applySegments(stepSegment(segments, key === 'ArrowUp' ? 1 : -1));
		} else if (key === 'Backspace' || key === 'Delete') {
			// Everything selected: switch to free text. Else start the part again.
			if (allSelected) {
				segments = null;
				return;
			}
			event.preventDefault();
			segments = { ...segments, buffer: '' };
			selectSegment();
		} else if (key.length === 1) {
			event.preventDefault();
			const from = allSelected ? moveSegment(segments, 0) : segments;
			applySegments(typeKey(from, key));
		}
	}

	// Paste: parse the whole text, then go on part by part.
	function handlePaste(event: ClipboardEvent) {
		const pasted = event.clipboardData?.getData('text') ?? '';
		event.preventDefault();
		const parsed = parseDatetimeInput(pasted, base, start);
		if (parsed) {
			commit(parsed);
			invalid = false;
			segments = startSegments(parsed, 0);
			text = segmentText(segments);
			input.value = text;
			selectSegment();
		} else {
			invalid = true;
			segments = null;
			text = pasted;
		}
	}

	// Free text: typing when the text was invalid, a mobile keyboard, or a
	// script that sets the value and dispatches `input`.
	function handleInput() {
		segments = null;
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
		segments = null;
		pointerFocus = false;
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
		bind:this={input}
		on:focus={handleFocus}
		on:mousedown={handleMouseDown}
		on:click={handleClick}
		on:keydown={handleKeydown}
		on:paste={handlePaste}
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
