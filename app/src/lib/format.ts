/** Formatting shared across the panes. Durations and counts are data, so they
 *  are set in the numeric face; prose is not. */

const clockFmt = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' });
const dayFmt = new Intl.DateTimeFormat(undefined, { weekday: 'short', day: 'numeric', month: 'short' });
const fullFmt = new Intl.DateTimeFormat(undefined, {
	weekday: 'long',
	day: 'numeric',
	month: 'long',
	year: 'numeric'
});

export function clock(ms: number | null): string {
	return ms === null ? '--:--' : clockFmt.format(new Date(ms));
}

/** Times alone imply the same day. When a session was resumed later, the dates
 *  have to show or the range reads as a contradiction against its own span. */
export function clockRange(fromMs: number | null, toMs: number | null): string {
	if (fromMs === null || toMs === null) return '--:--';
	const sameDay = new Date(fromMs).toDateString() === new Date(toMs).toDateString();
	return sameDay
		? `${clock(fromMs)}–${clock(toMs)}`
		: `${day(fromMs)} ${clock(fromMs)} → ${day(toMs)} ${clock(toMs)}`;
}

export function day(ms: number | null): string {
	return ms === null ? '—' : dayFmt.format(new Date(ms));
}

export function fullDay(ms: number | null): string {
	return ms === null ? '—' : fullFmt.format(new Date(ms));
}

export function dayKey(ms: number): string {
	const d = new Date(ms);
	return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

/** A block can hold a single record and have no span; `0m` would read as nothing
 *  happening when something did. */
export function duration(ms: number): string {
	const mins = Math.floor(ms / 60_000);
	if (mins < 1) return '<1m';
	if (mins < 60) return `${mins}m`;
	return `${Math.floor(mins / 60)}h ${String(mins % 60).padStart(2, '0')}m`;
}

export function compactDuration(ms: number): string {
	const mins = Math.round(ms / 60_000);
	if (mins < 1) return '<1m';
	if (mins < 60) return `${mins}m`;
	const hours = mins / 60;
	return hours < 10 ? `${hours.toFixed(1)}h` : `${Math.round(hours)}h`;
}

export function tokens(n: number): string {
	if (n < 1000) return String(n);
	if (n < 1_000_000) return `${(n / 1000).toFixed(n < 10_000 ? 1 : 0)}k`;
	return `${(n / 1_000_000).toFixed(1)}M`;
}

/** Elapsed time, against a clock the caller supplies.
 *
 *  `now` is a parameter rather than a `Date.now()` inside, because a call to the
 *  real clock is invisible to the framework: the text was recomputed only when
 *  the timestamp changed, so "4m ago" stayed on screen an hour later and then
 *  jumped. Passing a ticking value makes the elapsing part reactive. */
export function relative(ms: number | null, now: number = Date.now()): string {
	if (ms === null) return 'never';
	const diff = now - ms;
	const mins = Math.round(diff / 60_000);
	if (mins < 1) return 'just now';
	if (mins < 60) return `${mins}m ago`;
	const hours = Math.round(mins / 60);
	if (hours < 24) return `${hours}h ago`;
	return `${Math.round(hours / 24)}d ago`;
}

/** How long a source keeps its own history before deleting it. lore has to run at
 *  least once inside this window or that stretch is gone from every source at once. */
export const RETENTION_DAYS = 30;

export type Retention = {
	state: 'never' | 'lapsed' | 'alarm' | 'ok';
	daysSince: number | null;
	daysLeft: number | null;
};

/** Derived in one place because two surfaces show it: the rail footer, and the
 *  range bar for when the rail is folded — which below 1120px it folds by itself,
 *  and does on every visit to the reader or settings. A second copy of this rule
 *  is how the two start disagreeing about the one number the product rests on. */
export function retention(lastScanMs: number | null, now: number): Retention {
	if (lastScanMs === null) return { state: 'never', daysSince: null, daysLeft: null };
	const daysSince = Math.floor((now - lastScanMs) / 86_400_000);
	const daysLeft = RETENTION_DAYS - daysSince;
	return {
		state: daysLeft <= 0 ? 'lapsed' : daysLeft <= 7 ? 'alarm' : 'ok',
		daysSince,
		daysLeft
	};
}

/** Keeps the tail, which is the part that identifies a file. */
export function shortPath(path: string, segments = 2): string {
	const parts = path.split('/').filter(Boolean);
	return parts.length <= segments ? path : parts.slice(-segments).join('/');
}

export const startOfDay = (ms: number): number => {
	const d = new Date(ms);
	d.setHours(0, 0, 0, 0);
	return d.getTime();
};

/** Weeks start Monday, which is how a working week is read. */
export const startOfWeek = (ms: number): number => {
	const d = new Date(startOfDay(ms));
	const shift = (d.getDay() + 6) % 7;
	d.setDate(d.getDate() - shift);
	return d.getTime();
};

/** Calendar arithmetic, not millisecond arithmetic: adding 7 * 86_400_000 across a
 *  daylight-saving boundary lands an hour off, and a month is not a fixed length
 *  at all. Both go through Date so the steps stay on real boundaries. */
export const addDays = (ms: number, n: number): number => {
	const d = new Date(ms);
	d.setDate(d.getDate() + n);
	return d.getTime();
};

export const addMonths = (ms: number, n: number): number => {
	const d = new Date(ms);
	d.setMonth(d.getMonth() + n);
	return d.getTime();
};

const monthFmt = new Intl.DateTimeFormat(undefined, { month: 'long', year: 'numeric' });
const dayShortFmt = new Intl.DateTimeFormat(undefined, { weekday: 'short', day: 'numeric' });

/** What range is actually on screen, said plainly. Four presets anchored to `now`
 *  never needed this; a range you can step does. */
export function rangeLabel(scope: 'day' | 'week' | 'month' | 'all', fromMs: number, toMs: number): string {
	if (scope === 'all') return 'Everything archived';
	if (scope === 'month') return monthFmt.format(new Date(fromMs));
	if (scope === 'day') return fullDay(fromMs);
	// A week: name both ends, and carry the month only where it changes.
	const last = new Date(toMs - 1);
	return `${dayShortFmt.format(new Date(fromMs))} – ${day(last.getTime())}`;
}

export const startOfMonth = (ms: number): number => {
	const d = new Date(startOfDay(ms));
	d.setDate(1);
	return d.getTime();
};
