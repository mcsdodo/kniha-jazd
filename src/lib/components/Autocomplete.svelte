<script lang="ts">
	export let value: string = '';
	export let suggestions: string[] = [];
	export let placeholder: string = '';
	export let onSelect: (value: string) => void;
	export let testId: string = '';

	let showDropdown = false;
	let filteredSuggestions: string[] = [];
	let selectedIndex = -1;

	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		value = target.value;
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
				event.stopPropagation(); // Don't trigger row cancel
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
		{value}
		{placeholder}
		data-testid={testId || undefined}
		on:input={handleInput}
		on:keydown={handleKeydown}
		on:blur={handleBlur}
		on:focus={updateSuggestions}
	/>
	{#if showDropdown}
		<div class="dropdown">
			{#each filteredSuggestions as suggestion, i}
				<button
					class="suggestion"
					class:selected={i === selectedIndex}
					on:mousedown|preventDefault={() => selectSuggestion(suggestion)}
					type="button"
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
		border: 1px solid #ddd;
		border-radius: 4px;
		font-size: 0.875rem;
	}

	input:focus {
		outline: none;
		border-color: #3498db;
	}

	.dropdown {
		position: absolute;
		top: 100%;
		left: 0;
		right: 0;
		background: white;
		border: 1px solid #ddd;
		border-top: none;
		border-radius: 0 0 4px 4px;
		max-height: 200px;
		overflow-y: auto;
		z-index: 1000;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.suggestion {
		display: block;
		width: 100%;
		padding: 0.5rem;
		border: none;
		background: white;
		text-align: left;
		cursor: pointer;
		font-size: 0.875rem;
	}

	.suggestion:hover,
	.suggestion.selected {
		background-color: #f0f0f0;
	}
</style>
