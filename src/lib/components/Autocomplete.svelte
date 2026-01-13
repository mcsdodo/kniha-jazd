<script lang="ts">
	export let value: string = '';
	export let suggestions: string[] = [];
	export let placeholder: string = '';
	export let onSelect: (value: string) => void;
	export let testId: string = '';

	let showDropdown = false;
	let filteredSuggestions: string[] = [];
	let selectedIndex = -1;

	// Track previous value to detect changes (for reactive updates)
	let previousValue = value;

	// Reactive statement: update suggestions when value changes
	// This handles both user typing and programmatic value changes (e.g., WebDriverIO setValue)
	$: if (value !== previousValue) {
		previousValue = value;
		updateSuggestions();
	}

	function updateSuggestions() {
		if (value.trim() === '') {
			filteredSuggestions = [];
			showDropdown = false;
			return;
		}

		filteredSuggestions = suggestions.filter((s) =>
			s.toLowerCase().includes(value.toLowerCase())
		);
		showDropdown = filteredSuggestions.length > 0;
		selectedIndex = -1;
	}

	function selectSuggestion(suggestion: string) {
		value = suggestion;
		showDropdown = false;
		onSelect(suggestion);
	}

	function handleKeydown(event: KeyboardEvent) {
		// Tab should always close dropdown and move to next field
		if (event.key === 'Tab') {
			showDropdown = false;
			return; // Let Tab propagate normally
		}

		if (!showDropdown) return;

		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				selectedIndex = Math.min(selectedIndex + 1, filteredSuggestions.length - 1);
				break;
			case 'ArrowUp':
				event.preventDefault();
				selectedIndex = Math.max(selectedIndex - 1, -1);
				break;
			case 'Enter':
				event.preventDefault();
				event.stopPropagation(); // Don't trigger row save
				if (selectedIndex >= 0 && selectedIndex < filteredSuggestions.length) {
					selectSuggestion(filteredSuggestions[selectedIndex]);
				}
				break;
			case 'Escape':
				// Close dropdown but let event bubble up to cancel row edit
				showDropdown = false;
				break;
		}
	}

	function handleBlur() {
		// Delay to allow click on dropdown
		setTimeout(() => {
			showDropdown = false;
		}, 200);
	}
</script>

<div class="autocomplete">
	<input
		type="text"
		bind:value
		{placeholder}
		data-testid={testId || undefined}
		on:keydown={handleKeydown}
		on:blur={handleBlur}
		on:focus={updateSuggestions}
	/>
	{#if showDropdown}
		<div class="dropdown" tabindex="-1">
			{#each filteredSuggestions as suggestion, i}
				<button
					class="suggestion"
					class:selected={i === selectedIndex}
					on:mousedown|preventDefault={() => selectSuggestion(suggestion)}
					type="button"
					tabindex="-1"
				>
					{suggestion}
				</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.autocomplete {
		position: relative;
		width: 100%;
	}

	input {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid var(--border-input);
		border-radius: 4px;
		font-size: 0.875rem;
		background: var(--bg-surface);
		color: var(--text-primary);
	}

	input:focus {
		outline: none;
		border-color: var(--accent-primary);
	}

	.dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		background: var(--bg-surface);
		border: 1px solid var(--border-input);
		border-top: none;
		border-radius: 0 0 4px 4px;
		max-height: 200px;
		overflow-y: auto;
		z-index: 1000;
		box-shadow: 0 2px 4px var(--shadow-default);
	}

	.suggestion {
		display: block;
		width: 100%;
		padding: 0.5rem;
		border: none;
		background: var(--bg-surface);
		text-align: left;
		cursor: pointer;
		font-size: 0.875rem;
		color: var(--text-primary);
	}

	.suggestion:hover,
	.suggestion.selected {
		background-color: var(--bg-surface-alt);
	}
</style>
