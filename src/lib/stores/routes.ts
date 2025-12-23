// Routes store

import { writable } from 'svelte/store';
import type { Route } from '../types';

export const routesStore = writable<Route[]>([]);
