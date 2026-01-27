/**
 * Home Assistant API service for fetching ODO sensor data.
 * This runs in the frontend because it's just HTTP fetch - no business logic.
 */

// Custom error types for specific error handling
export class HaTimeoutError extends Error {
	constructor() {
		super('Connection timed out');
		this.name = 'HaTimeoutError';
	}
}

export class HaAuthError extends Error {
	constructor() {
		super('Invalid or expired API token');
		this.name = 'HaAuthError';
	}
}

export class HaSensorNotFoundError extends Error {
	constructor(sensorId: string) {
		super(`Sensor not found: ${sensorId}`);
		this.name = 'HaSensorNotFoundError';
	}
}

export class HaInvalidResponseError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'HaInvalidResponseError';
	}
}

/**
 * Fetch ODO value from Home Assistant sensor.
 * @param url - Home Assistant base URL (e.g., "http://homeassistant.local:8123")
 * @param token - Long-lived access token
 * @param sensorId - Sensor entity ID (e.g., "sensor.car_odometer")
 * @returns ODO value in km
 * @throws {HaTimeoutError} If request times out (5s)
 * @throws {HaAuthError} If token is invalid (401)
 * @throws {HaSensorNotFoundError} If sensor doesn't exist (404)
 * @throws {HaInvalidResponseError} If response is not a valid number
 */
export async function fetchOdometer(
	url: string,
	token: string,
	sensorId: string
): Promise<number> {
	const apiUrl = `${url.replace(/\/$/, '')}/api/states/${sensorId}`;

	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), 5000); // 5s timeout

	try {
		const response = await fetch(apiUrl, {
			method: 'GET',
			headers: {
				Authorization: `Bearer ${token}`,
				'Content-Type': 'application/json'
			},
			signal: controller.signal
		});

		clearTimeout(timeout);

		if (response.status === 401) {
			throw new HaAuthError();
		}

		if (response.status === 404) {
			throw new HaSensorNotFoundError(sensorId);
		}

		if (!response.ok) {
			throw new HaInvalidResponseError(`HTTP ${response.status}: ${response.statusText}`);
		}

		const data = await response.json();

		// HA returns { state: "12345.6", attributes: {...}, ... }
		const state = data?.state;

		if (state === undefined || state === null || state === 'unavailable' || state === 'unknown') {
			throw new HaInvalidResponseError('Sensor state is unavailable');
		}

		const value = parseFloat(state);

		if (isNaN(value)) {
			throw new HaInvalidResponseError(`Invalid sensor value: ${state}`);
		}

		return value;
	} catch (error) {
		clearTimeout(timeout);

		if (error instanceof Error && error.name === 'AbortError') {
			throw new HaTimeoutError();
		}

		// Re-throw our custom errors
		if (
			error instanceof HaTimeoutError ||
			error instanceof HaAuthError ||
			error instanceof HaSensorNotFoundError ||
			error instanceof HaInvalidResponseError
		) {
			throw error;
		}

		// Wrap other errors
		throw new HaInvalidResponseError(error instanceof Error ? error.message : 'Unknown error');
	}
}

/**
 * Test connection to Home Assistant.
 * @param url - Home Assistant base URL
 * @param token - Long-lived access token
 * @returns true if connection successful
 */
export async function testConnection(url: string, token: string): Promise<boolean> {
	const apiUrl = `${url.replace(/\/$/, '')}/api/`;

	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), 5000);

	try {
		const response = await fetch(apiUrl, {
			method: 'GET',
			headers: {
				Authorization: `Bearer ${token}`,
				'Content-Type': 'application/json'
			},
			signal: controller.signal
		});

		clearTimeout(timeout);
		return response.ok;
	} catch {
		clearTimeout(timeout);
		return false;
	}
}
