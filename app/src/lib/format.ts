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

export function relative(ms: number | null): string {
	if (ms === null) return 'never';
	const diff = Date.now() - ms;
	const mins = Math.round(diff / 60_000);
	if (mins < 1) return 'just now';
	if (mins < 60) return `${mins}m ago`;
	const hours = Math.round(mins / 60);
	if (hours < 24) return `${hours}h ago`;
	return `${Math.round(hours / 24)}d ago`;
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

export const startOfMonth = (ms: number): number => {
	const d = new Date(startOfDay(ms));
	d.setDate(1);
	return d.getTime();
};
