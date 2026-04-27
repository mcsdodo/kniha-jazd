import { writable } from 'svelte/store';
import { TOAST_TYPES, type ToastType } from '$lib/constants';

export type { ToastType };

export interface Toast {
	id: number;
	message: string;
	type: ToastType;
	action?: { label: string; onClick: () => void };
}

function createToastStore() {
	const { subscribe, update } = writable<Toast[]>([]);
	let nextId = 1;

	function show(message: string, type: ToastType = TOAST_TYPES.INFO, duration = 4000) {
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

	function withAction(
		message: string,
		actionLabel: string,
		onAction: () => void,
		duration = 6000
	) {
		const id = nextId++;
		update((toasts) => [
			...toasts,
			{
				id,
				message,
				type: TOAST_TYPES.INFO,
				action: {
					label: actionLabel,
					onClick: () => {
						onAction();
						dismiss(id);
					}
				}
			}
		]);
		if (duration > 0) {
			setTimeout(() => dismiss(id), duration);
		}
	}

	return {
		subscribe,
		success: (message: string) => show(message, TOAST_TYPES.SUCCESS),
		error: (message: string) => show(message, TOAST_TYPES.ERROR, 6000),
		info: (message: string) => show(message, TOAST_TYPES.INFO),
		withAction,
		dismiss
	};
}

export const toast = createToastStore();
