// Store to trigger the nav invoice indicator from other pages.
//
// The badge counts unlinked Paperless fuel documents, and that count changes
// whenever a link is created or removed -- on the Doklady page (assign,
// unassign, clear override) and in the grid (a trip that gains or loses fuel
// changes what "unlinked" means). The indicator polls the Paperless API only
// every 5 minutes, so without this the badge can stay stale for that long right
// after the action that should have cleared it.
import { writable } from 'svelte/store';

export const invoiceRefreshTrigger = writable(0);

export function triggerInvoiceRefresh() {
	invoiceRefreshTrigger.update(n => n + 1);
}
