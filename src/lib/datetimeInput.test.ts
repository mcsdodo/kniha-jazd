// Run with: npm run test:unit (node:test, Node >= 22.18 strips the types).
import { test } from 'node:test';
import assert from 'node:assert/strict';

import { formatDatetimeInput, parseDatetimeInput } from './datetimeInput.ts';

const CURRENT = '2026-10-09T08:00';

// --- formatDatetimeInput ---

test('format shows DD.MM HH:MM without the year', () => {
	assert.equal(formatDatetimeInput('2026-10-09T14:30'), '09.10 14:30');
});

test('format of an empty or broken value is empty', () => {
	assert.equal(formatDatetimeInput(''), '');
	assert.equal(formatDatetimeInput('garbage'), '');
});

// --- parseDatetimeInput: full and short forms ---

test('parse the full display form', () => {
	assert.equal(parseDatetimeInput('09.10 14:30', CURRENT), '2026-10-09T14:30');
});

test('parse single-digit day, month and hour', () => {
	assert.equal(parseDatetimeInput('9.1 7:05', CURRENT), '2026-01-09T07:05');
});

test('parse a trailing dot after the month', () => {
	assert.equal(parseDatetimeInput('9.10. 14:30', CURRENT), '2026-10-09T14:30');
});

test('parse the time as digits only', () => {
	assert.equal(parseDatetimeInput('9.10 1430', CURRENT), '2026-10-09T14:30');
	assert.equal(parseDatetimeInput('9.10 930', CURRENT), '2026-10-09T09:30');
	assert.equal(parseDatetimeInput('9.10 9', CURRENT), '2026-10-09T09:00');
});

test('parse a dot as the time separator when a date comes first', () => {
	assert.equal(parseDatetimeInput('9.10 14.30', CURRENT), '2026-10-09T14:30');
});

test('a time alone keeps the current date', () => {
	assert.equal(parseDatetimeInput('1430', CURRENT), '2026-10-09T14:30');
	assert.equal(parseDatetimeInput('14:30', CURRENT), '2026-10-09T14:30');
});

test('a date alone keeps the current time', () => {
	assert.equal(parseDatetimeInput('3.5', CURRENT), '2026-05-03T08:00');
});

test('the year comes from the current value', () => {
	assert.equal(parseDatetimeInput('1.2 10:00', '2025-07-01T00:00'), '2025-02-01T10:00');
});

test('extra spaces are ignored', () => {
	assert.equal(parseDatetimeInput('  9.10   14:30 ', CURRENT), '2026-10-09T14:30');
});

test('the ISO form is accepted as is', () => {
	assert.equal(parseDatetimeInput('2025-03-15T08:00', CURRENT), '2025-03-15T08:00');
});

// --- parseDatetimeInput: invalid input ---

test('invalid input returns null', () => {
	for (const text of [
		'',
		'abc',
		'32.10 10:00',
		'31.11 10:00',
		'29.2 10:00', // 2026 is not a leap year
		'9.13 10:00',
		'9.10 24:00',
		'9.10 10:60',
		'9.10 12345',
		'9.10 10:00 extra',
		'14.30', // one token with a dot is a date, and month 30 does not exist
		'2026-02-30T10:00'
	]) {
		assert.equal(parseDatetimeInput(text, CURRENT), null, `"${text}" must be invalid`);
	}
});

test('29 February is valid in a leap year', () => {
	assert.equal(parseDatetimeInput('29.2 10:00', '2028-01-01T00:00'), '2028-02-29T10:00');
});

// --- parseDatetimeInput: the end of a trip (start given) ---

test('the end takes the year of the start', () => {
	assert.equal(
		parseDatetimeInput('10.5 18:00', '2026-01-01T00:00', '2026-05-10T08:00'),
		'2026-05-10T18:00'
	);
});

test('an end in January after a December start moves to the next year', () => {
	assert.equal(
		parseDatetimeInput('1.1 02:00', '2026-12-31T22:00', '2026-12-31T22:00'),
		'2027-01-01T02:00'
	);
});

test('an end before the start is invalid (no silent move to the next year)', () => {
	assert.equal(parseDatetimeInput('10.5 13:00', '2026-05-10T14:00', '2026-05-10T14:00'), null);
	assert.equal(parseDatetimeInput('1.5 13:00', '2026-05-10T14:00', '2026-05-10T14:00'), null);
});

test('an end equal to the start is valid', () => {
	assert.equal(
		parseDatetimeInput('10.5 14:00', '2026-05-10T14:00', '2026-05-10T14:00'),
		'2026-05-10T14:00'
	);
});

test('a time alone for the end keeps the current end date', () => {
	assert.equal(
		parseDatetimeInput('0230', '2027-01-01T01:00', '2026-12-31T22:00'),
		'2027-01-01T02:30'
	);
});

test('an ISO end before the start is invalid too', () => {
	assert.equal(parseDatetimeInput('2026-05-10T13:00', '2026-05-10T14:00', '2026-05-10T14:00'), null);
});
