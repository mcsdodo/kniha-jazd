// Vehicles store

import { writable } from 'svelte/store';
import type { Vehicle } from '../types';

export const vehiclesStore = writable<Vehicle[]>([]);
export const activeVehicleStore = writable<Vehicle | null>(null);
