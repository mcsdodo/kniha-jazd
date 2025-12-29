import { writable } from 'svelte/store';

export type ToastType = 'success' | 'error' | 'info';

export interface Toast {
	id: number;
	message: string;
	type: ToastType;
}

function createToastStore() {
	const { subscribe, update } = writable<Toast[]>([]);
	let nextId = 1;

	function show(message: string, type: ToastType = 'info', duration = 4000) {
		const id = nextId++;
		update((toasts) => [...toasts, { id, message, type }]);

		if (duration > 0) {
			setTimeout(() => {
				dismiss(id);
			}, duration);
		}
	}

	function dismiss(id: number) {
		update((toasts) => toasts.filter((t) => t.id !== id));
	}

	return {
		subscribe,
		success: (message: string) => show(message, 'success'),
		error: (message: string) => show(message, 'error', 6000),
		info: (message: string) => show(message, 'info'),
		dismiss
	};
}

export const toast = createToastStore();
