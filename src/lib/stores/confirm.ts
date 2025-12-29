import { writable, get } from 'svelte/store';

export interface ConfirmState {
	isOpen: boolean;
	title: string;
	message: string;
	confirmText: string;
	danger: boolean;
	onConfirm: () => void;
}

const defaultState: ConfirmState = {
	isOpen: false,
	title: 'Potvrdiť',
	message: '',
	confirmText: 'Potvrdiť',
	danger: false,
	onConfirm: () => {}
};

function createConfirmStore() {
	const { subscribe, set } = writable<ConfirmState>(defaultState);

	function show(options: {
		title?: string;
		message: string;
		confirmText?: string;
		danger?: boolean;
		onConfirm: () => void;
	}) {
		set({
			isOpen: true,
			title: options.title || 'Potvrdiť',
			message: options.message,
			confirmText: options.confirmText || 'Potvrdiť',
			danger: options.danger ?? false,
			onConfirm: options.onConfirm
		});
	}

	function close() {
		set(defaultState);
	}

	function handleConfirm() {
		const state = get({ subscribe });
		state.onConfirm();
		close();
	}

	return {
		subscribe,
		show,
		close,
		confirm: handleConfirm
	};
}

export const confirmStore = createConfirmStore();
