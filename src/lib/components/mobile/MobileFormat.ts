// Display formatting for the phone pages (Task 93). Formatting only: every
// value comes from the backend (ADR-008).

/** The BCP 47 tag for the app locale ('sk' or 'en'). */
export function intlTag(locale: string): string {
	return locale === 'en' ? 'en-GB' : 'sk-SK';
}

interface Parts {
	y: number;
	m: number;
	d: number;
	time: string;
}

// Split a NaiveDateTime string ("YYYY-MM-DDTHH:MM[:SS]") by hand. new Date()
// on a date-only string parses as UTC and can move the day.
function parts(s: string): Parts {
	const [date, time = ''] = s.split('T');
	const [y, m, d] = date.split('-').map((v) => parseInt(v, 10));
	return { y, m, d, time: time.slice(0, 5) };
}

/** "St 5.11." - the short weekday and the day and month. */
export function dayLabel(start: string, locale: string): string {
	const p = parts(start);
	const weekday = new Date(p.y, p.m - 1, p.d).toLocaleDateString(intlTag(locale), {
		weekday: 'short'
	});
	return `${capitalize(weekday)} ${p.d}.${p.m}.`;
}

/** "Ut 4.11.2026" - for the detail header. */
export function fullDayLabel(start: string, locale: string): string {
	return `${dayLabel(start, locale)}${parts(start).y}`;
}

/** "14:05 - 14:48". An end on another day also shows its date. */
export function timeRange(start: string, end: string | null): string {
	const s = parts(start);
	if (!end) return s.time;
	const e = parts(end);
	const sameDay = s.y === e.y && s.m === e.m && s.d === e.d;
	return sameDay ? `${s.time} - ${e.time}` : `${s.time} - ${e.d}.${e.m}. ${e.time}`;
}

/** "YYYY-MM" of a trip start, to group the list by month. */
export function monthKey(start: string): string {
	return start.slice(0, 7);
}

/** "November 2026" for a "YYYY-MM" key. */
export function monthLabel(key: string, locale: string): string {
	const [y, m] = key.split('-').map((v) => parseInt(v, 10));
	const label = new Date(y, m - 1, 1).toLocaleDateString(intlTag(locale), {
		month: 'long',
		year: 'numeric'
	});
	return capitalize(label);
}

/** "4.11.2026" for a "YYYY-MM-DD[THH:MM]" value. */
export function shortDate(s: string): string {
	const p = parts(s);
	return `${p.d}.${p.m}.${p.y}`;
}

/** A whole number with grouping, for km and the odometer. */
export function whole(n: number, locale: string): string {
	return n.toLocaleString(intlTag(locale), { maximumFractionDigits: 0 });
}

/** A number with a fixed count of decimals, as the PC grid shows it. */
export function fixed(n: number, digits = 1): string {
	return n.toFixed(digits);
}

function capitalize(s: string): string {
	return s.charAt(0).toUpperCase() + s.slice(1);
}
