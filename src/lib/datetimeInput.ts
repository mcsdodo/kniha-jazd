// Text form of a trip's start/end in the edit row: "DD.MM HH:MM".
//
// The year is not typed. It comes from the value the field already holds,
// which is in the year selected on the page. This is input formatting, not
// business logic (ADR-008): the stored value stays "YYYY-MM-DDTHH:MM".
//
// No imports on purpose: `npm run test:unit` loads this file with plain
// Node (node:test), without Vite or the $lib alias.

const ISO = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/;
const DATE = /^(\d{1,2})\.(\d{1,2})\.?$/;
const TIME_SEPARATED = /^(\d{1,2})[:.](\d{2})$/;
const TIME_DIGITS = /^\d{1,4}$/;

const pad = (n: number) => n.toString().padStart(2, '0');

function daysInMonth(year: number, month: number): number {
	return new Date(year, month, 0).getDate();
}

function isoParts(value: string) {
	const m = ISO.exec(value);
	if (!m) return null;
	const [year, month, day, hour, minute] = m.slice(1).map(Number);
	return { year, month, day, hour, minute };
}

function build(year: number, month: number, day: number, hour: number, minute: number): string | null {
	if (month < 1 || month > 12) return null;
	if (day < 1 || day > daysInMonth(year, month)) return null;
	if (hour > 23 || minute > 59) return null;
	return `${year}-${pad(month)}-${pad(day)}T${pad(hour)}:${pad(minute)}`;
}

// "1430" -> 14:30, "930" -> 09:30, "9" -> 09:00. With a separator, ":" or ".".
function parseTime(token: string, allowDot: boolean): { hour: number; minute: number } | null {
	const sep = TIME_SEPARATED.exec(token);
	if (sep && (allowDot || token.includes(':'))) {
		return { hour: Number(sep[1]), minute: Number(sep[2]) };
	}
	if (!TIME_DIGITS.test(token)) return null;
	if (token.length <= 2) return { hour: Number(token), minute: 0 };
	return { hour: Number(token.slice(0, -2)), minute: Number(token.slice(-2)) };
}

/** "YYYY-MM-DDTHH:MM" -> "DD.MM HH:MM". An unknown value gives "". */
export function formatDatetimeInput(value: string): string {
	const p = isoParts(value);
	if (!p) return '';
	return `${pad(p.day)}.${pad(p.month)} ${pad(p.hour)}:${pad(p.minute)}`;
}

/**
 * Parse typed text into "YYYY-MM-DDTHH:MM", or null if the text is not valid.
 *
 * Accepted: "DD.MM HH:MM" and short forms ("9.10 1430", "9.10 14.30"),
 * a time alone (keeps the date of `current`), a date alone (keeps the time
 * of `current`), and the full ISO form.
 *
 * `start` is given for the END of a trip. The end then takes the year of the
 * start, moves to the next year only for a December start with a January end,
 * and is invalid when it is earlier than the start.
 */
export function parseDatetimeInput(text: string, current: string, start: string | null = null): string | null {
	const cur = isoParts(current);
	if (!cur) return null;
	const trimmed = text.trim();

	const result = ISO.test(trimmed) ? parseIso(trimmed) : parseTyped(trimmed, cur, start);
	// Same fixed-width format, so string order is time order.
	if (result && start && result < start) return null;
	return result;
}

function parseIso(text: string): string | null {
	const p = isoParts(text)!;
	return build(p.year, p.month, p.day, p.hour, p.minute);
}

function parseTyped(
	trimmed: string,
	cur: NonNullable<ReturnType<typeof isoParts>>,
	start: string | null
): string | null {
	const tokens = trimmed.split(/\s+/).filter(Boolean);
	let day = cur.day;
	let month = cur.month;
	let hour = cur.hour;
	let minute = cur.minute;
	let dateTyped = false;

	if (tokens.length === 2) {
		const d = DATE.exec(tokens[0]);
		const t = parseTime(tokens[1], true);
		if (!d || !t) return null;
		[day, month] = [Number(d[1]), Number(d[2])];
		({ hour, minute } = t);
		dateTyped = true;
	} else if (tokens.length === 1) {
		// One token with a dot is a date ("14.30" is not a time here).
		const d = DATE.exec(tokens[0]);
		const t = parseTime(tokens[0], false);
		if (d) {
			[day, month] = [Number(d[1]), Number(d[2])];
			dateTyped = true;
		} else if (t) {
			({ hour, minute } = t);
		} else {
			return null;
		}
	} else {
		return null;
	}

	let year = cur.year;
	const s = start ? isoParts(start) : null;
	if (s && dateTyped) {
		year = s.month === 12 && month === 1 ? s.year + 1 : s.year;
	}

	return build(year, month, day, hour, minute);
}
