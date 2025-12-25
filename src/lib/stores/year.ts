import { writable } from 'svelte/store';

// Initialize to current calendar year
export const selectedYearStore = writable<number>(new Date().getFullYear());

// Helper to reset to current year (used when switching vehicles)
export function resetToCurrentYear(): void {
	selectedYearStore.set(new Date().getFullYear());
}
