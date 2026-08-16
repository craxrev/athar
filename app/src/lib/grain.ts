/** The grain ladder, in one place.
 *
 *  The rail selects a *unit* and the range gives a *length*, and the two do not
 *  always agree: "all time" on a two-week-old archive is a week of days, not
 *  seven years of cells. Which rung a range resolves to therefore has to be
 *  computed, and it has to be computed once — the timeline draws by it and the
 *  keyboard walks by it, and a second copy is how those two start disagreeing
 *  about which marks exist.
 */
export type Scope = 'day' | 'week' | 'month' | 'all';

/** Named for what a day *is* at that rung, which is the ladder's own vocabulary:
 *  a day is an hour-resolved row of projects, a resolved row of its own, a tile,
 *  or a cell. */
export type Grain = 'hours' | 'days' | 'tiles' | 'cells';

const DAY = 86_400_000;

export function grainOf(scope: Scope, fromMs: number, toMs: number): Grain {
	if (scope === 'day') return 'hours';
	const days = Math.max(Math.round((toMs - fromMs) / DAY), 1);
	if (scope === 'week' || days <= 10) return 'days';
	if (scope === 'month' || days <= 45) return 'tiles';
	return 'cells';
}

/** One thing the reader can select, in the order the timeline drew it.
 *
 *  The two halves are exclusive by construction: a rung draws blocks or it draws
 *  periods, never both, which is why the pane answering a selection can switch
 *  on this and never has to reconcile the two. */
export type Mark =
	| { kind: 'block'; id: number }
	| { kind: 'day'; at: number }
	| { kind: 'month'; at: number };

/** The attribute the marks carry, so a keyboard-chosen one can be brought into
 *  view. Written here rather than spelled out at each call site: a selector and
 *  the attribute it looks for drifting apart is silent, and has happened once
 *  already in this build. */
export function markSelector(mark: Mark): string {
	return mark.kind === 'block'
		? `[data-block="${mark.id}"]`
		: `[data-period="${mark.at}"]`;
}
