// Store to trigger receipt indicator refresh from other pages
import { writable } from 'svelte/store';

// Increment this to trigger a refresh of the receipt indicator
export const receiptRefreshTrigger = writable(0);

export function triggerReceiptRefresh() {
	receiptRefreshTrigger.update(n => n + 1);
}
