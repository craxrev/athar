<script lang="ts">
	import type { Evidence, Lane } from './archive';
	import { clock, compactDuration } from './format';
	import { grainOf, type Mark, type Scope } from './grain';
	import Icon from './Icon.svelte';
	import { hueStyle } from './palette.svelte';

	/** The four ways a block's span can be evidenced, weakest last, named once.
	 *  The digest uses the same words: a split the timeline calls something else
	 *  is two vocabularies for one fact. */
	const EVIDENCE: { id: Evidence; label: string }[] = [
		{ id: 'sessions', label: 'from sessions' },
		{ id: 'commits', label: 'from commits' },
		{ id: 'saves', label: 'from saves' },
		{ id: 'bare', label: 'from records only' }
	];
	const evidenceLabel = new Map(EVIDENCE.map((e) => [e.id, e.label]));
	/** Strongest first. A window holding both a session and a save is a session
	 *  window: the same precedence the treatments themselves descend by. */
	const RANK: Evidence[] = ['sessions', 'commits', 'saves', 'bare'];

	let {
		lanes,
		fromMs,
		toMs,
		scope,
		selected,
		onSelect,
		onDrill,
		selectedPeriod,
		onSelectPeriod,
		allShape,
		onShape,
		marks = $bindable([])
	}: {
		lanes: Lane[];
		fromMs: number;
		toMs: number;
		scope: Scope;
		selected: number | null;
		onSelect: (blockId: number) => void;
		/** Walk down the ladder to the period containing `at`. The page turns that
		 *  into an offset; this component never owns the range. */
		onDrill: (next: 'day' | 'week' | 'month', at: number) => void;
		/** A tile, a cell and a month panel stand for a period, not a block, so
		 *  the grains that draw them select a period instead. The pane answers it;
		 *  going there is a separate, deliberate step. */
		selectedPeriod: number | null;
		onSelectPeriod: (at: number, kind: 'day' | 'month') => void;
		/** Owned by the page, because this component is remounted on every filter
		 *  change and a choice that resets that often is not a choice. */
		allShape: 'years' | 'months';
		onShape: (next: 'years' | 'months') => void;
		/** What is on screen, in the order the eye reads it, published so the
		 *  keyboard walks exactly what this rung drew. Bound out rather than
		 *  reassembled by the page: the page's own copy was right only while every
		 *  rung drew blocks, and after the ladder `j` at month grain selected a
		 *  block with no mark on screen. The component that draws the marks is the
		 *  one that knows which marks exist. */
		marks?: Mark[];
	} = $props();

	const DAY = 86_400_000;
	const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

	/** Built once. `toLocaleDateString` constructs a formatter per call, and the
	 *  months sheet asks for three thousand of them — enough to be felt on every
	 *  render and every scroll that brings a panel into view. */
	const DAY_TITLE = new Intl.DateTimeFormat(undefined, {
		weekday: 'long',
		day: 'numeric',
		month: 'long',
		year: 'numeric'
	});
	const MONTH_SHORT = new Intl.DateTimeFormat(undefined, { month: 'short', year: '2-digit' });
	const MONTH_LONG = new Intl.DateTimeFormat(undefined, { month: 'long', year: 'numeric' });
	const DAY_SHORT = new Intl.DateTimeFormat(undefined, {
		weekday: 'short',
		day: 'numeric',
		month: 'short'
	});

	function startOfDay(ms: number): number {
		const d = new Date(ms);
		d.setHours(0, 0, 0, 0);
		return d.getTime();
	}

	/** Never `key + DAY`. Twice a year a local day is twenty-three or twenty-five
	 *  hours long, and adding a fixed span lands an hour inside the next day or
	 *  an hour short of it — which files real work under the wrong date. A
	 *  product whose premise is exact timestamps does not get to be approximate
	 *  about which day they fell on. */
	function nextDay(dayStart: number): number {
		const d = new Date(dayStart);
		d.setDate(d.getDate() + 1);
		d.setHours(0, 0, 0, 0);
		return d.getTime();
	}
	function dayLength(dayStart: number): number {
		return nextDay(dayStart) - dayStart;
	}

	/** The archive indexed by the day it fell on.
	 *
	 *  A block can cross midnight, so it is split at the boundary rather than
	 *  filed under its start: a day row that drew time the day did not hold would
	 *  be the same kind of lie as a bar wider than its evidence. */
	type DayRecord = {
		ms: number;
		classes: Map<Evidence, number>;
		projects: Map<string, number>;
		/** Busiest first, and the category of the one at the front. Both are
		 *  derived once when the day is indexed rather than per cell: a sheet at
		 *  all time asks every day who owned it, and sorting a map inside a render
		 *  loop meant thousands of sorts on every range change. */
		names: string[];
		category: string;
		spans: {
			from: number;
			to: number;
			category: string;
			project: string;
			evidence: Evidence;
			blockId: number;
		}[];
	};

	let byDay = $derived.by(() => {
		const out = new Map<number, DayRecord>();
		for (const lane of lanes) {
			for (const bar of lane.bars) {
				let from = Math.max(bar.started_ms, fromMs);
				const end = Math.min(bar.ended_ms, toMs);
				while (from < end) {
					const key = startOfDay(from);
					const stop = Math.min(end, nextDay(key));
					let rec = out.get(key);
					if (!rec) {
						rec = {
							ms: 0,
							classes: new Map(),
							projects: new Map(),
							names: [],
							category: '',
							spans: []
						};
						out.set(key, rec);
					}
					rec.ms += stop - from;
					rec.classes.set(bar.evidence, (rec.classes.get(bar.evidence) ?? 0) + (stop - from));
					rec.projects.set(lane.project, (rec.projects.get(lane.project) ?? 0) + (stop - from));
					rec.spans.push({
						from,
						to: stop,
						category: lane.category,
						project: lane.project,
						evidence: bar.evidence,
						blockId: bar.block_id
					});
					from = stop;
				}
			}
		}
		// Rank once, here, where each day is touched exactly once.
		for (const rec of out.values()) {
			rec.names = [...rec.projects.entries()].sort((a, b) => b[1] - a[1]).map(([p]) => p);
			rec.category = categoryOf.get(rec.names[0]) ?? '';
		}
		return out;
	});

	let activeDays = $derived([...byDay.keys()].sort((a, b) => a - b));
	let rangeDays = $derived(Math.max(Math.round((toMs - fromMs) / DAY), 1));
	/** Which rung this range resolves to. Shared with the page rather than
	 *  branched on inline, so the marks the keyboard can reach and the marks the
	 *  template draws are decided by one rule. */
	let grain = $derived(grainOf(scope, fromMs, toMs));
	/** Scaled against the busiest day on screen, so ink means the same thing
	 *  everywhere in the view. Half the peak saturates, because a single
	 *  fourteen-hour outlier otherwise renders every ordinary day as nearly
	 *  empty. */
	let peakDay = $derived(
		[...byDay.values()].reduce((most, rec) => Math.max(most, rec.ms), 1) * 0.5
	);

	/** Only the classes this range actually holds — but always at least one.
	 *
	 *  Suppressing the key below two classes got this backwards: a day evidenced
	 *  only by file saves is precisely when someone meets an unfamiliar treatment
	 *  cold, and it was precisely when no key was drawn.
	 *
	 *  And only where the marks carry texture at all. Evidence rides the surface
	 *  of a span, and only the two rungs that draw spans have one: a tile's strip
	 *  and a cell carry hue and nothing else. Naming four treatments beside marks
	 *  that cannot show them teaches a vocabulary the screen is not speaking. */
	let present = $derived.by(() => {
		if (grain !== 'hours' && grain !== 'days') return [];
		const seen = new Set<Evidence>();
		for (const rec of byDay.values()) for (const c of rec.classes.keys()) seen.add(c);
		return EVIDENCE.filter((e) => seen.has(e.id));
	});

	let categories = $derived.by(() => {
		const seen = new Set<string>();
		for (const lane of lanes) seen.add(lane.category);
		return [...seen];
	});

	/** The hue key, drawn only where a mark's colour is the only thing naming its
	 *  class. The day rung heads every group with its category's swatch and name,
	 *  so a second key at the foot would restate what is already above each row. */
	let legendHues = $derived(grain === 'hours' || categories.length < 2 ? [] : categories);

	const categoryOf = $derived(new Map(lanes.map((l) => [l.project, l.category])));

	/** Two projects can share a leaf name in different trees. Showing both as the
	 *  same word makes the view unreadable, so a duplicate keeps its parent. */
	let duplicates = $derived.by(() => {
		const seen = new Map<string, number>();
		for (const l of lanes) seen.set(l.project, (seen.get(l.project) ?? 0) + 1);
		return seen;
	});

	function label(project: string): string {
		if ((duplicates.get(project) ?? 0) < 2) return project;
		const lane = lanes.find((l) => l.project === project);
		if (!lane) return project;
		return lane.project_path.split('/').filter(Boolean).slice(-2).join('/');
	}

	function spanTitle(s: DayRecord['spans'][number]): string {
		return [
			`${s.project}  ${clock(s.from)}–${clock(s.to)}`,
			compactDuration(s.to - s.from),
			evidenceLabel.get(s.evidence) ?? 'from records only'
		].join(' · ');
	}

	function dayTitle(at: number, rec: DayRecord | undefined): string {
		const when = DAY_TITLE.format(at);
		if (!rec) return `${when} · nothing archived`;
		return `${when} · ${compactDuration(rec.ms)} · ${rec.names.length} project${
			rec.projects.size === 1 ? '' : 's'
		}`;
	}

	/* ---- rung geometry ------------------------------------------------------
	   Each rung derives the days it draws from the range it was handed, never
	   from a date of its own: the rail owns which period is on screen. */

	let weekDays = $derived.by(() => {
		const out: number[] = [];
		for (let t = startOfDay(fromMs); t < toMs; t = nextDay(t)) out.push(t);
		return out.reverse();
	});

	/** The days of the range as tiles, grouped by the month each falls in.
	 *
	 *  Deriving the grid from the range rather than from one month is not a
	 *  refinement. This rung is reached at `all` whenever the archive is younger
	 *  than about six weeks — the first weeks of use — and a single-month walk
	 *  stopped at the end of the range's first month: on a 20 Jul – 16 Aug
	 *  archive it drew all of July and nothing of August, so sixteen of the
	 *  twenty-eight days in range never rendered, while nineteen days from
	 *  before the archive existed rendered as tiles reading "nothing archived".
	 *  Meanwhile the key below still counted the range, so it announced days the
	 *  grid could not show.
	 *
	 *  A day inside a drawn month but outside the range is a pad, never an empty
	 *  tile: the archive was never asked about it, and "nothing archived" is a
	 *  claim. */
	let tileMonths = $derived.by(() => {
		const first = startOfDay(fromMs);
		const out: { at: number; cells: (number | null)[] }[] = [];
		const cursor = new Date(first);
		cursor.setDate(1);
		while (cursor.getTime() < toMs) {
			const cells: (number | null)[] = [];
			// Monday-first, matching the axis's own week rule.
			for (let pad = (cursor.getDay() + 6) % 7; pad > 0; pad--) cells.push(null);
			const walk = new Date(cursor);
			while (walk.getMonth() === cursor.getMonth()) {
				const key = startOfDay(walk.getTime());
				cells.push(key >= first && key < toMs ? key : null);
				walk.setDate(walk.getDate() + 1);
			}
			out.push({ at: cursor.getTime(), cells });
			cursor.setMonth(cursor.getMonth() + 1);
		}
		return out;
	});

	/** The months of the range, newest first, each with its own week grid.
	 *
	 *  Years and months are the same cell at the same grain; what differs is what
	 *  the cells are grouped under. Years hold the whole archive at once and read
	 *  as density; months are the unit a person actually remembers a stretch of
	 *  work by — "that was a February thing" — and cost the vertical room to say
	 *  so. Which of the two is on screen is the page's to own, not this
	 *  component's; see `allShape` above. */
	let months = $derived.by(() => {
		const out: { at: number; label: string; cells: (number | null)[]; ms: number }[] = [];
		const last = new Date(toMs - 1);
		const cursor = new Date(last.getFullYear(), last.getMonth(), 1);
		const first = new Date(fromMs);
		while (cursor.getTime() >= new Date(first.getFullYear(), first.getMonth(), 1).getTime()) {
			const cells: (number | null)[] = [];
			const walk = new Date(cursor);
			for (let pad = (walk.getDay() + 6) % 7; pad > 0; pad--) cells.push(null);
			let ms = 0;
			while (walk.getMonth() === cursor.getMonth()) {
				const key = startOfDay(walk.getTime());
				cells.push(key);
				ms += byDay.get(key)?.ms ?? 0;
				walk.setDate(walk.getDate() + 1);
			}
			out.push({
				at: cursor.getTime(),
				label: MONTH_SHORT.format(cursor),
				cells,
				ms
			});
			cursor.setMonth(cursor.getMonth() - 1);
		}
		return out;
	});

	let years = $derived.by(() => {
		const out: { year: number; cells: (number | null)[]; ms: number }[] = [];
		const last = new Date(toMs - 1).getFullYear();
		for (let year = last; year >= new Date(fromMs).getFullYear(); year--) {
			const cells: (number | null)[] = [];
			const cursor = new Date(year, 0, 1);
			for (let pad = (cursor.getDay() + 6) % 7; pad > 0; pad--) cells.push(null);
			let ms = 0;
			while (cursor.getFullYear() === year) {
				const key = startOfDay(cursor.getTime());
				cells.push(key);
				ms += byDay.get(key)?.ms ?? 0;
				cursor.setDate(cursor.getDate() + 1);
			}
			out.push({ year, cells, ms });
		}
		return out;
	});

	/** One row per project for the day on screen, busiest first. */
	/** Projects for the day on screen, grouped under their category.
	 *
	 *  The grouping is the one thing the ladder took away and had to give back:
	 *  the rungs above this one draw days, so a category can only be a hue there,
	 *  and the per-category total went with it. Here rows are projects again, so
	 *  the heading that names the class and sums it belongs here too. Categories
	 *  arrive in the order their busiest project does, and projects stay busiest
	 *  first inside one — the same ordering the view has always used. */
	let dayGroups = $derived.by(() => {
		const rec = byDay.get(startOfDay(fromMs));
		if (!rec) return [];
		const out: {
			category: string;
			ms: number;
			rows: { project: string; ms: number; spans: DayRecord['spans'] }[];
		}[] = [];
		for (const project of rec.names) {
			const category = categoryOf.get(project) ?? '';
			const ms = rec.projects.get(project) ?? 0;
			let group = out.find((g) => g.category === category);
			if (!group) {
				group = { category, ms: 0, rows: [] };
				out.push(group);
			}
			group.ms += ms;
			group.rows.push({ project, ms, spans: rec.spans.filter((sp) => sp.project === project) });
		}
		return out;
	});

	let dayProjectCount = $derived(dayGroups.reduce((n, g) => n + g.rows.length, 0));

	let dayStartMs = $derived(startOfDay(fromMs));

	/* Positions are a share of the day's own length, not of a nominal
	   twenty-four hours: on the short day of the year the latter overflows the
	   track, on the long one it stops before the end. */
	function offsetIn(day: number, at: number): number {
		return ((at - day) / dayLength(day)) * 100;
	}
	function widthIn(day: number, s: { from: number; to: number }): number {
		// A block holding one record has no span; without a floor it is invisible.
		return Math.max(((s.to - s.from) / dayLength(day)) * 100, 0.3);
	}
	function weight(rec: DayRecord | undefined): number {
		if (!rec) return 0;
		return Math.min(rec.ms / peakDay, 1);
	}

	/* ---- what the keyboard can reach -----------------------------------------
	   Assembled from the same lists the template renders from, in the same order,
	   so the walk cannot reach a mark that was not drawn or skip one that was. */

	/** A block that crosses midnight is drawn on two day rows, and pressing `j`
	 *  onto the id already selected would toggle it off rather than advance. Two
	 *  marks, one selection: the second is dropped and the first keeps its
	 *  place. */
	function blockMarks(ids: number[]): Mark[] {
		const seen = new Set<number>();
		const out: Mark[] = [];
		for (const id of ids) {
			if (seen.has(id)) continue;
			seen.add(id);
			out.push({ kind: 'block', id });
		}
		return out;
	}

	let drawn = $derived.by((): Mark[] => {
		if (grain === 'hours')
			return blockMarks(
				dayGroups
					.flatMap((g) => g.rows)
					.flatMap((r) => r.spans)
					.map((s) => s.blockId)
			);
		if (grain === 'days')
			// Row by row, newest day first, and left to right inside a row — the order
			// the rung actually draws. A day's spans arrive project-major, which is
			// not how a strip reads; sorting every bar in the range by start time, as
			// the page used to, was wrong the other way and walked against the rows.
			return blockMarks(
				weekDays.flatMap((day) =>
					(byDay.get(day)?.spans ?? [])
						.slice()
						.sort((a, b) => a.from - b.from)
						.map((s) => s.blockId)
				)
			);
		if (grain === 'tiles')
			return tileMonths
				.flatMap((m) => m.cells)
				.filter((cell): cell is number => cell !== null && byDay.has(cell))
				.map((at) => ({ kind: 'day', at }));
		if (allShape === 'months') return months.map((m) => ({ kind: 'month', at: m.at }));
		return years
			.flatMap((y) => y.cells)
			.filter((cell): cell is number => cell !== null && byDay.has(cell))
			.map((at) => ({ kind: 'day', at }));
	});

	$effect(() => {
		marks = drawn;
	});

	/** The one cell in the year sheet that is in the tab order.
	 *
	 *  Every day the archive holds is a button here — 2,697 of them — so tabbing
	 *  from the sheet to the key beneath it was thousands of presses. One stop
	 *  gets you into the grid and `j`/`k` walk it, which is the same pair that
	 *  walks every other rung. Nothing is unreachable; the sheet just stops being
	 *  a wall. */
	let roving = $derived(
		selectedPeriod ?? (drawn[0]?.kind === 'day' ? drawn[0].at : null)
	);
</script>

<div class="lanes" class:picked={selected !== null || selectedPeriod !== null}>
	{#if grain === 'hours'}
		<!-- Grain: one hour. The day is wide enough for projects to sit side by
		     side on a shared axis, which is the one range where comparing them is
		     legible — so this rung keeps the project rows the others give up. -->
		<div class="hours day" aria-hidden="true">
			<span></span>
			<div class="marks">
				{#each [0, 3, 6, 9, 12, 15, 18, 21] as h (h)}
					<span
						class="hour"
						class:first={h === 0}
						class:quarter={h % 6 !== 0}
						class:half={h % 12 !== 0}
						style="left: {(h / 24) * 100}%">{String(h).padStart(2, '0')}:00</span
					>
				{/each}
			</div>
			<span></span>
		</div>

		<div class="body">
			{#if dayProjectCount === 0}
				<p class="none">
					Nothing archived on this day. lore may not have been running, or nothing happened.
				</p>
			{/if}
			{#each dayGroups as group (group.category)}
				<div class="group">
					<div class="grouphead">
						<span class="swatch" style={hueStyle(group.category)} aria-hidden="true"></span>
						<h3>{group.category}</h3>
						<span class="num">{compactDuration(group.ms)}</span>
					</div>

					{#each group.rows as row, i (row.project)}
						<div class="prow" style="--stagger: {Math.min(i, 12) * 22}ms">
							<span class="project">{label(row.project)}</span>
							<div class="track">
								{#each [3, 6, 9, 12, 15, 18, 21] as h (h)}
									<span class="rule" style="left: {(h / 24) * 100}%"></span>
								{/each}
								{#each row.spans as s (s.blockId + '-' + s.from)}
									<button
										class="span"
										class:on={selected === s.blockId}
										data-evidence={s.evidence}
										data-block={s.blockId}
										style="left: {offsetIn(dayStartMs, s.from)}%; width: {widthIn(
											dayStartMs,
											s
										)}%; {hueStyle(s.category)}"
										title={spanTitle(s)}
										aria-label={spanTitle(s)}
										aria-pressed={selected === s.blockId}
										onclick={() => onSelect(s.blockId)}
									></button>
								{/each}
							</div>
							<span class="num total">{compactDuration(row.ms)}</span>
						</div>
					{/each}
				</div>
			{/each}
		</div>
	{:else if grain === 'days'}
		<!-- Grain: one day, as a row. Every hour of it is resolved, and an empty
		     day stays in the run drawn empty — a quiet week reads as quiet
		     rather than as a short one. -->
		<div class="hours week" aria-hidden="true">
			<span></span>
			<span></span>
			<div class="marks">
				{#each [0, 3, 6, 9, 12, 15, 18, 21] as h (h)}
					<span
						class="hour"
						class:first={h === 0}
						class:quarter={h % 6 !== 0}
						class:half={h % 12 !== 0}
						style="left: {(h / 24) * 100}%">{String(h).padStart(2, '0')}:00</span
					>
				{/each}
			</div>
			<span></span>
		</div>

		<div class="body">
			{#each weekDays as day, i (day)}
				{@const rec = byDay.get(day)}
				<div class="drow" class:void={!rec} style="--stagger: {Math.min(i, 12) * 22}ms">
					<span class="date"
						>{DAY_SHORT.format(day)}</span
					>
					<span class="num total">{rec ? compactDuration(rec.ms) : '—'}</span>
					<div class="strip">
						{#each [3, 6, 9, 12, 15, 18, 21] as h (h)}
							<span class="rule" style="left: {(h / 24) * 100}%"></span>
						{/each}
						{#each rec?.spans ?? [] as s (s.blockId + '-' + s.from)}
							<button
								class="span"
								class:on={selected === s.blockId}
								data-evidence={s.evidence}
								data-block={s.blockId}
								style="left: {offsetIn(day, s.from)}%; width: {widthIn(day, s)}%; {hueStyle(
									s.category
								)}"
								title={spanTitle(s)}
								aria-label={spanTitle(s)}
								aria-pressed={selected === s.blockId}
								onclick={() => onSelect(s.blockId)}
							></button>
						{/each}
					</div>
					<button
						class="open"
						title="Open {new Date(day).toDateString()}"
						aria-label="Open {new Date(day).toDateString()}"
						onclick={() => onDrill('day', day)}
					>
						<Icon name="chevron" size={15} />
					</button>
				</div>
			{/each}
		</div>
	{:else if grain === 'tiles'}
		<!-- Grain: one day, as a tile. Thirty days as rows is twenty-six pixels
		     each and a wasted axis; as tiles it is a page, and the tile is the
		     last rung where a day can still say a name. -->
		<div class="weekdays" aria-hidden="true">
			{#each WEEKDAYS as w (w)}<span>{w}</span>{/each}
		</div>

		<div class="body">
			{#each tileMonths as m (m.at)}
				<!-- Named only when the range crosses a boundary. At `by month` the range
				     bar above already says which month this is, and repeating it here
				     would be a heading over the only thing on screen. -->
				{#if tileMonths.length > 1}
					<h3 class="tilemonth">{MONTH_LONG.format(m.at)}</h3>
				{/if}
				<div class="month">
					{#each m.cells as cell, i (i)}
						{#if cell === null}
							<span class="tile pad"></span>
						{:else}
							{@const rec = byDay.get(cell)}
							{#if rec}
								{@const names = rec.names}
								<button
									class="tile"
									class:on={selectedPeriod === cell}
									aria-pressed={selectedPeriod === cell}
									data-period={cell}
									style="--stagger: {Math.min(i, 24) * 12}ms"
									title={dayTitle(cell, rec)}
									aria-label={dayTitle(cell, rec)}
									onclick={() => onSelectPeriod(cell, 'day')}
								>
									<span class="head">
										<span class="num day">{new Date(cell).getDate()}</span>
										<span class="num held">{compactDuration(rec.ms)}</span>
									</span>
									{#each names.slice(0, 2) as name (name)}
										<span class="who">
											<span class="swatch" style={hueStyle(categoryOf.get(name))} aria-hidden="true"
											></span>
											<span class="nm">{label(name)}</span>
										</span>
									{/each}
									{#if names.length > 2}
										<span class="who more">+{names.length - 2} more</span>
									{/if}
									<span class="micro" aria-hidden="true">
										{#each rec.spans as s (s.blockId + '-' + s.from)}
											<i
												style="left: {offsetIn(cell, s.from)}%; width: {Math.max(
													widthIn(cell, s),
													1.2
												)}%; {hueStyle(s.category)}"
											></i>
										{/each}
									</span>
								</button>
							{:else}
								<span class="tile empty" title={dayTitle(cell, undefined)}>
									<span class="head"><span class="num day">{new Date(cell).getDate()}</span></span>
								</span>
							{/if}
						{/if}
					{/each}
				</div>
			{/each}
		</div>
	{:else}
		<!-- Grain: one day, as a cell. The only arrangement that holds every day
		     the archive has and still resolves one of them. A cell carries real
		     hours and a real class; a date with nothing in it is the metaphor the
		     old anti-reference was written against. -->
		<div class="shape">
			<div class="modes" role="group" aria-label="Group days by">
				<button
					type="button"
					class:on={allShape === 'years'}
					aria-pressed={allShape === 'years'}
					onclick={() => onShape('years')}>Years</button
				>
				<button
					type="button"
					class:on={allShape === 'months'}
					aria-pressed={allShape === 'months'}
					onclick={() => onShape('months')}>Months</button
				>
			</div>
		</div>

		<div class="body">
			{#if allShape === 'months'}
				<div class="contact">
					{#each months as m (m.at)}
						<button
							class="mpanel"
							title="{MONTH_LONG.format(m.at)} · {m.ms
								? compactDuration(m.ms)
								: 'nothing archived'}"
							class:on={selectedPeriod === m.at}
							aria-pressed={selectedPeriod === m.at}
							data-period={m.at}
							onclick={() => onSelectPeriod(m.at, 'month')}
						>
							<span class="mhead">
								<span class="num mlabel">{m.label}</span>
								<span class="num mtot">{m.ms ? compactDuration(m.ms) : '—'}</span>
							</span>
							<span class="mgrid">
								{#each m.cells as cell, i (i)}
									{#if cell === null}
										<span class="mcell"></span>
									{:else}
										{@const rec = byDay.get(cell)}
										<span
											class="mcell"
											class:held={!!rec}
											style="--weight: {weight(rec)}; {hueStyle(rec?.category)}"
											title={dayTitle(cell, rec)}
										></span>
									{/if}
								{/each}
							</span>
						</button>
					{/each}
				</div>
			{:else}
			<div class="sheets">
				{#each years as y (y.year)}
					<div class="year">
						<span class="num yr">{y.year}</span>
						<div class="grid">
							{#each y.cells as cell, i (i)}
								{#if cell === null}
									<span class="cell"></span>
								{:else}
									{@const rec = byDay.get(cell)}
									{#if rec}
										<button
											class="cell held"
											class:on={selectedPeriod === cell}
											aria-pressed={selectedPeriod === cell}
											data-period={cell}
											tabindex={cell === roving ? 0 : -1}
											style="--weight: {weight(rec)}; {hueStyle(rec?.category)}"
											title={dayTitle(cell, rec)}
											aria-label={dayTitle(cell, rec)}
											onclick={() => onSelectPeriod(cell, 'day')}
										></button>
									{:else}
										<span class="cell"></span>
									{/if}
								{/if}
							{/each}
						</div>
						<span class="num ytot">{y.ms ? compactDuration(y.ms) : '—'}</span>
					</div>
				{/each}
			</div>
			{/if}
		</div>
	{/if}

	<!-- Each half is drawn only where it names something on screen: the treatments
	     where marks carry texture, the hues where a row is not already headed by
	     its category, the count wherever a range is more than one day. -->
	{#if present.length || legendHues.length || rangeDays > 1}
		<div class="key">
			{#if present.length || legendHues.length}
				<ul>
					{#each present as e (e.id)}
						<li>
							<span class="sample" data-evidence={e.id} aria-hidden="true"></span>
							{e.label}
						</li>
					{/each}
					{#each legendHues as c (c)}
						<li>
							<span class="swatch" style={hueStyle(c)} aria-hidden="true"></span>
							{c}
						</li>
					{/each}
				</ul>
			{/if}
			{#if rangeDays > 1}
				<p class="held">
					<b class="num">{activeDays.length}</b> of
					<b class="num">{rangeDays}</b> days hold something
				</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	.lanes {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		--edge: 16px;
		/* The two gutters every rung shares, so a date column and an hour axis
		   cannot drift apart between rungs. */
		--datecol: 118px;
		--totalcol: 52px;
		--rowgap: 12px;
		/* Declared once and consumed by both the row and the hour axis above it.
		   Two hand-kept copies is how the axis and the track drift apart, which
		   this file has already had to fix once. */
		--day-cols: 158px minmax(0, 1fr) var(--totalcol);
		--week-cols: var(--datecol) var(--totalcol) minmax(0, 1fr) 24px;
	}

	.body {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		padding-bottom: 20px;
	}

	.none {
		margin: 0;
		padding: 26px var(--edge);
		max-width: 62ch;
		font-size: var(--fs-min);
		line-height: 1.5;
		color: var(--text-faint);
	}

	/* ---- the hour axis, shared by the two rungs that resolve hours -------- */

	.hours {
		display: grid;
		gap: var(--rowgap);
		flex: none;
		height: 26px;
		padding: 0 var(--edge);
		border-bottom: 1px solid var(--line);
	}
	.hours.day {
		grid-template-columns: var(--day-cols);
	}
	.hours.week {
		grid-template-columns: var(--week-cols);
	}
	/* The only cell that carries ticks, sitting in the same column as the track
	   it labels. A container, so the labels below can thin themselves out
	   against the width they actually have rather than the window's — this pane
	   is resized by the side panes, not by the viewport. */
	.marks {
		position: relative;
		min-width: 0;
		container-type: inline-size;
	}
	/* Eight labels need about 58px each before `00:00` and `03:00` start
	   touching. Under that the three-hour marks go, then the six-hour ones,
	   leaving midnight and midday. */
	@container (max-width: 464px) {
		.hour.quarter {
			display: none;
		}
	}
	@container (max-width: 232px) {
		.hour.half {
			display: none;
		}
	}
	.hour {
		position: absolute;
		top: 7px;
		transform: translateX(-50%);
		font-family: var(--mono);
		font-size: var(--fs-min);
		font-weight: 560;
		color: var(--text-faint);
		white-space: nowrap;
	}
	.hour.first {
		transform: none;
	}

	/* ---- marks ------------------------------------------------------------
	   Hue says whose work it was; texture says how well the span is known. Two
	   questions, two channels — see DESIGN.md's Two Axes Rule. */

	.span {
		position: absolute;
		top: 2px;
		bottom: 2px;
		min-width: 3px;
		border: 0;
		padding: 0;
		border-radius: var(--radius-mark);
		background: var(--cat, var(--text-faint));
		cursor: pointer;
		transition: filter var(--motion-state);
		animation: grow 620ms cubic-bezier(0.16, 1, 0.3, 1) both;
		animation-delay: var(--stagger);
	}

	/* Exact at the commits, inferred between them by the idle-gap rule. The
	   hatch claims no positions, which is the point. */
	.span[data-evidence='commits'] {
		background-image: repeating-linear-gradient(
			45deg,
			var(--mark-cut) 0 2px,
			transparent 2px 5px
		);
	}
	/* Two mtimes and nothing known between: the ends are marked because the ends
	   are what the archive has. */
	.span[data-evidence='saves'] {
		box-shadow:
			inset 2px 0 0 var(--amber),
			inset -2px 0 0 var(--amber);
	}
	/* The span is real; nothing in it is describable, so it claims no ends. */
	.span[data-evidence='bare'] {
		background-image: repeating-linear-gradient(
			90deg,
			var(--mark-cut) 0 3px,
			transparent 3px 6px
		);
	}
	.span:hover {
		filter: brightness(1.22);
	}
	/* Selection recedes the rest of the timeline rather than recolouring one mark.
	   Every colour is already spoken for: hue says which project, texture says
	   how the span is known, and the accent is selection's own. Category hues
	   are drawn from a palette that excludes it, but a mark still cannot be
	   recoloured to mean chosen without spending a channel that is carrying
	   something. Dimming what was not chosen needs no colour at all — it
	   works the same on every category, and the selected mark keeps saying
	   exactly what it said before it was picked.

	   Brightness rather than opacity, because a translucent mark is not a
	   receded one: at 28% alpha the hour rules showed straight through every
	   bar, and two marks that touched blended into a third colour. Filtering
	   leaves the fill opaque, so a dimmed bar still hides what is behind it. */
	.picked .span:not(.on) {
		filter: brightness(0.42) saturate(0.75);
	}
	/* Above every other mark in the row. A day strip holds all of the day's
	   projects, and the archive has four hundred cross-project overlaps, so an
	   unselected block that shares time with the selected one was drawn straight
	   across it — the chosen mark came out striped with the marks it was chosen
	   over. Anything wholly inside the selection is unreachable here as a
	   result; the day rung gives each project its own row, which is where an
	   overlap gets untangled. */
	.span.on {
		filter: none;
		box-shadow: var(--lift-1);
		z-index: 1;
	}
	/* A dimmed bar still answers the pointer, just from further back. Spelled with
	   the same exclusion as the rule above it, so which of these wins does not
	   depend on which was written first. */
	.picked .span:not(.on):hover {
		filter: brightness(0.7) saturate(0.85);
	}
	.span.on:hover {
		filter: brightness(1.22);
	}
	/* The lift, without losing the amber end caps this treatment draws. */
	.span.on[data-evidence='saves'] {
		box-shadow:
			inset 2px 0 0 var(--amber),
			inset -2px 0 0 var(--amber),
			var(--lift-1);
	}

	/* ---- rung 1 · by day --------------------------------------------------- */

	/* More space above a heading than below it, and the first group needs less
	   than the ones that follow it: there is no row above it to separate from. */
	.group {
		padding-top: 22px;
	}
	.group:first-child {
		padding-top: 10px;
	}
	.grouphead {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 var(--edge) 8px;
	}
	.grouphead h3 {
		flex: 1;
		margin: 0;
		font-size: 13.5px;
		font-weight: 640;
		color: var(--text-dim);
		text-transform: capitalize;
	}
	.grouphead .num {
		color: var(--text-faint);
	}

	.prow {
		display: grid;
		grid-template-columns: var(--day-cols);
		align-items: center;
		gap: var(--rowgap);
		height: 42px;
		padding: 0 var(--edge);
		content-visibility: auto;
		contain-intrinsic-size: auto 42px;
	}
	.prow:hover {
		background: var(--wash);
	}
	.project {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 14px;
		font-weight: 540;
		color: var(--text-dim);
	}
	.prow:hover .project {
		color: var(--text);
	}
	.track {
		position: relative;
		height: 24px;
		border-radius: var(--radius-sm);
		background: var(--well);
	}
	.total {
		text-align: right;
		color: var(--text-faint);
	}

	/* ---- rung 2 · by week --------------------------------------------------- */

	.drow {
		display: grid;
		grid-template-columns: var(--week-cols);
		align-items: center;
		gap: var(--rowgap);
		height: 44px;
		padding: 0 var(--edge);
		content-visibility: auto;
		contain-intrinsic-size: auto 44px;
	}
	.drow:hover {
		background: var(--wash);
	}
	.date {
		font-size: 14px;
		font-weight: 540;
		color: var(--text-dim);
	}
	.drow.void .date,
	.drow.void .total {
		color: var(--text-faint);
		opacity: 0.62;
	}
	.strip {
		position: relative;
		height: 24px;
		border-radius: var(--radius-sm);
		background: var(--well);
		overflow: hidden;
	}
	.rule {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 1px;
		background: var(--line);
	}
	.open {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 24px;
		border: 0;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--text-faint);
		cursor: pointer;
		transition: background var(--motion-state), color var(--motion-state);
	}
	.open:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	/* ---- rung 3 · by month -------------------------------------------------- */

	.weekdays {
		display: grid;
		grid-template-columns: repeat(7, minmax(0, 1fr));
		gap: 4px;
		flex: none;
		padding: 7px var(--edge) 6px;
		border-bottom: 1px solid var(--line);
		font-family: var(--mono);
		font-size: var(--fs-min);
		font-weight: 560;
		color: var(--text-faint);
	}
	/* Only drawn when the range crosses a month. More space above it than below,
	   and none above the first: there is no grid before it to separate from. */
	.tilemonth {
		margin: 22px 0 0;
		padding: 0 var(--edge);
		font-size: 13.5px;
		font-weight: 640;
		color: var(--text-dim);
	}
	.tilemonth:first-child {
		margin-top: 4px;
	}
	.month {
		display: grid;
		grid-template-columns: repeat(7, minmax(0, 1fr));
		gap: 4px;
		padding: 10px var(--edge) 0;
	}
	/* Same reason as .mpanel: a button's flex children are centred by WebKit's UA
	   sheet, and the micro strip below has no intrinsic width to shrink to. */
	.tile {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 3px;
		min-height: 82px;
		padding: 6px 7px 7px;
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-family: inherit;
		text-align: left;
		cursor: pointer;
		animation: rise 480ms cubic-bezier(0.16, 1, 0.3, 1) both;
		animation-delay: var(--stagger);
	}
	.tile.pad {
		border: 0;
		background: none;
		min-height: 0;
		animation: none;
	}
	.tile.empty {
		background: var(--wash);
		border-style: dashed;
		border-color: var(--line);
		cursor: default;
		animation: none;
	}
	/* Chosen, at the grains where a day is the unit. The same recede the marks
	   use: what was not chosen steps back, and the chosen one is left as it was
	   so its hue and its hours still read.

	   The two rungs recede by different means, and the difference is not
	   arbitrary. A filter is what the spans need, because the hour rules run
	   behind them and a translucent bar lets them through; a tile and a cell sit
	   on bare ground with nothing behind either. So a cell — and there are 2,697
	   of them in a year sheet, against the eighty-nine panels whose filters
	   already had to be taken out for stutter — recedes on the one property it is
	   already spending: its own density alpha, scaled. Tiles keep the filter,
	   both because forty-five of them cost nothing and because their entrance
	   animates opacity and would win the cascade against it.

	   The chosen mark is excluded from the recede rather than exempted by a second
	   rule after it. `:not()` carries the specificity of its own argument, so
	   `.picked .tile:not(.pad):not(.empty)` weighs four classes against
	   `.picked .tile.on`'s three and won — the selected tile was dimmed along with
	   everything it was chosen over, which is precisely the state the Ink Rule
	   exists to produce the opposite of. Excluding `.on` in the rule that dims
	   leaves nothing to out-weigh, and nothing to break if these move. */
	.picked .tile:not(.pad):not(.empty):not(.on) {
		filter: brightness(0.5) saturate(0.8);
	}
	/* The chosen cell keeps the density alpha it had; only what it was chosen over
	   is scaled down. */
	.picked .cell.held:not(.on) {
		opacity: calc((0.72 + var(--weight) * 0.28) * 0.42);
	}
	.mpanel.on {
		border-color: var(--line-strong);
		background: var(--surface);
	}
	.tile.on {
		border-color: var(--line-strong);
		background: var(--surface-raised);
	}
	/* The accent, as everywhere else a thing is chosen. Hover took it and
	   selection took the brightest neutral in the system, which read the two
	   states in the wrong order — the transient one louder than the committed
	   one. */
	.cell.on {
		outline: 1px solid var(--accent);
		outline-offset: 1px;
	}

	.tile:not(.pad):not(.empty):hover {
		border-color: var(--line-strong);
		background: var(--surface-raised);
	}
	.head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 6px;
	}
	.day {
		font-weight: 620;
		color: var(--text-dim);
	}
	.tile.empty .day {
		opacity: 0.55;
	}
	.held {
		font-weight: 600;
		color: var(--text);
	}
	.who {
		display: flex;
		align-items: center;
		gap: 5px;
		min-width: 0;
		font-size: var(--fs-min);
		font-weight: 500;
		color: var(--text-faint);
	}
	.who.more {
		padding-left: 14px;
	}
	.nm {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.micro {
		position: relative;
		height: 5px;
		margin-top: auto;
		border-radius: var(--radius-mark);
		background: var(--well);
		overflow: hidden;
	}
	/* Square ends on purpose: the strip above clips its own corners, and the
	   narrowest segments here are two pixels wide — any radius at that size
	   rounds a mark into a dot and takes its shape with it. */
	.micro i {
		position: absolute;
		top: 0;
		bottom: 0;
		background: var(--cat, var(--text-faint));
	}

	/* ---- rung 4 · all time --------------------------------------------------- */

	/* The one choice this rung offers: the same cells, grouped by the unit you
	   think in. It sits above the sheet rather than in the range bar, because it
	   changes how the days are arranged and not which days they are. */
	.shape {
		display: flex;
		justify-content: flex-end;
		flex: none;
		padding: 8px var(--edge) 0;
	}
	.modes {
		display: flex;
		gap: 3px;
	}
	.modes button {
		padding: 4px 11px;
		border: 0;
		border-radius: var(--radius-pill);
		background: var(--fill-subtle);
		color: var(--text-faint);
		font-family: inherit;
		font-size: var(--fs-min);
		font-weight: 560;
		cursor: pointer;
		transition: background var(--motion-state), color var(--motion-state);
	}
	.modes button:hover {
		background: var(--surface-hover);
		color: var(--text-dim);
	}
	/* The tint, not the solid: this is the case the tint was minted for. The solid
	   accent on its own fill measures 4.48:1 — under the 4.5 floor by two
	   hundredths — and the tint clears it at 4.64. */
	.modes button.on {
		background: var(--accent-soft);
		color: var(--accent-tint);
	}

	/* Months are wider than they are tall at this size, so four across keeps a
	   panel's own week grid square rather than stretched. */
	.contact {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
		gap: 10px;
		padding: 12px var(--edge) 0;
	}
	/* `align-items` is explicit because WebKit's UA stylesheet centres a button's
	   flex children. The cells below are empty spans sized only by their aspect
	   ratio, so shrink-to-fit resolves their width to zero and the whole grid
	   collapses — the panel renders as a header over nothing. */
	.mpanel {
		/* Eighty-nine panels of thirty-seven cells each. Skipping the ones off
		   screen is the difference between a sheet that scrolls and one that
		   stutters. */
		content-visibility: auto;
		contain-intrinsic-size: auto 118px;
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: 7px;
		padding: 9px 10px 10px;
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-family: inherit;
		text-align: left;
		cursor: pointer;
		transition: background var(--motion-state), border-color var(--motion-state);
	}
	.mpanel:hover {
		border-color: var(--line-strong);
		background: var(--surface-raised);
	}
	.mhead {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
	}
	.mlabel {
		font-weight: 620;
		color: var(--text-dim);
	}
	.mtot {
		color: var(--text-faint);
	}
	.mgrid {
		display: grid;
		width: 100%;
		grid-template-columns: repeat(7, minmax(0, 1fr));
		gap: 2px;
	}
	.mcell {
		aspect-ratio: 1;
		border-radius: var(--radius-swatch);
		background: var(--well);
	}
	.mcell.held {
		background: color-mix(in oklab, var(--cat-tint, var(--text-dim)) calc(var(--weight) * 55%), var(--cat, var(--text-faint)));
		opacity: calc(0.72 + var(--weight) * 0.28);
	}

	/* Fifty-three columns cannot shrink below 636px, and the window's own 880px
	   minimum can leave this pane 508px. The sheet scrolls sideways as one piece
	   so every year stays column-aligned, and the year label pins to the left so
	   you never lose which row you are reading. */
	/* One cell size at every width. Sizing it to the pane meant a year looked
	   denser with the detail pane open than without, and the pane has nothing to
	   do with how big a day is — the sheet scrolls instead, on the platform's own
	   overlay scrollbar, which stays out of the way until it is used. */
	.sheets {
		padding: 18px var(--edge) 8px;
		overflow-x: auto;
		--cell: 10px;
		--cellgap: 2px;
	}
	/* The year leads and its total closes, as they did before — but both are
	   pinned to the edges of the scrollport, so neither is lost partway through a
	   sideways scroll of the sheet. */
	.year {
		content-visibility: auto;
		contain-intrinsic-size: auto 84px;
		display: grid;
		grid-template-columns: 42px max-content auto;
		align-items: center;
		gap: 12px;
		margin-bottom: 13px;
	}
	.yr,
	.ytot {
		position: sticky;
		z-index: 1;
		background: var(--ground);
	}
	.yr {
		left: 0;
		padding-right: 8px;
		font-weight: 620;
		color: var(--text);
	}
	.ytot {
		right: 0;
		padding-left: 8px;
		text-align: right;
		color: var(--text-faint);
	}
	.grid {
		display: grid;
		grid-auto-flow: column;
		grid-template-rows: repeat(7, var(--cell));
		gap: var(--cellgap);
		/* Without this the columns are auto-sized *and* stretched — a grid's
		   default `justify-content` behaves as stretch — so the spare width in the
		   row was shared between 53 tracks and the gaps came back uneven, worse
		   the wider the pane. The cells are a fixed size; the leftover space
		   belongs at the end of the row, not between the days. */
		justify-content: start;
	}
	.cell {
		width: var(--cell);
		height: var(--cell);
		padding: 0;
		border: 0;
		border-radius: var(--radius-swatch);
		background: var(--well);
	}
	/* The floor is 0.72 because nothing carrying a class may sit under 3:1, and
	   the palette's least luminous hue reaches it just above 0.70 alpha. Density
	   then rides the hue rather than the alpha, mixing toward that category's
	   own tint — the lighter sibling every palette entry carries, because a
	   solid hue is not legible small on a dark ground. A quiet day and a heavy
	   one stay a full step apart, and both clear the Legible Mark Rule. */
	.cell.held {
		cursor: pointer;
		background: color-mix(in oklab, var(--cat-tint, var(--text-dim)) calc(var(--weight) * 55%), var(--cat, var(--text-faint)));
		opacity: calc(0.72 + var(--weight) * 0.28);
	}
	/* Quieter than the accent ring that means chosen — the two sat the other way
	   round, with the pointer louder than the commitment — but still a neutral
	   with ink in it. A divider at 14% white measured 1.44:1 against ground, which
	   is under the 3:1 an indicator owes and is not a hover state, it is nothing. */
	.cell.held:hover {
		outline: 1px solid var(--text-faint);
		outline-offset: 1px;
	}
	.cell.on:hover {
		outline-color: var(--accent);
	}

	/* ---- the key ------------------------------------------------------------
	   Reads once, at the foot, and names only what the range holds. It carries
	   both channels now, because both are on the marks. */

	.key {
		display: flex;
		align-items: baseline;
		gap: 8px 18px;
		flex: none;
		padding: 9px var(--edge) 10px;
		border-top: 1px solid var(--line);
	}
	.key ul {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 16px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.key li {
		display: flex;
		align-items: center;
		gap: 7px;
		font-size: var(--fs-min);
		font-weight: 540;
		color: var(--text-faint);
	}
	.key .held {
		margin: 0 0 0 auto;
		flex: none;
		/* Pushed right by the legend beside it; on its own it leads the bar rather
		   than floating at the far edge of an otherwise empty rule. */
		font-size: var(--fs-min);
		font-weight: 500;
		color: var(--text-faint);
	}
	.key:not(:has(ul)) .held {
		margin-left: 0;
	}
	.key .held b {
		font-family: var(--mono);
		font-weight: 620;
		color: var(--text-dim);
	}
	.sample {
		position: relative;
		width: 26px;
		height: 12px;
		flex: none;
		border-radius: var(--radius-swatch);
		background: var(--text-faint);
	}
	.sample[data-evidence='commits'] {
		background-image: repeating-linear-gradient(
			45deg,
			var(--mark-cut) 0 2px,
			transparent 2px 5px
		);
	}
	.sample[data-evidence='saves'] {
		box-shadow:
			inset 2px 0 0 var(--amber),
			inset -2px 0 0 var(--amber);
	}
	.sample[data-evidence='bare'] {
		background-image: repeating-linear-gradient(
			90deg,
			var(--mark-cut) 0 3px,
			transparent 3px 6px
		);
	}

	.swatch {
		width: 9px;
		height: 9px;
		flex: none;
		border-radius: var(--radius-swatch);
		background: var(--cat, var(--text-faint));
	}

	/* One authored moment per surface: marks are revealed from their own start
	   along the axis, tiles settle in place. Both from an already-visible
	   default, neither deforming what it contains. */
	@keyframes grow {
		from {
			clip-path: inset(0 100% 0 0);
			opacity: 0.4;
		}
		to {
			clip-path: inset(0 0 0 0);
			opacity: 1;
		}
	}
	@keyframes rise {
		from {
			opacity: 0.55;
		}
		to {
			opacity: 1;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.span,
		.tile {
			animation: none;
		}
		/* State feedback keeps its colour change and loses only the ramp — a
		   blanket kill would take the feedback with it. */
		.span,
		.open,
		.tile,
		.mpanel,
		.modes button {
			transition: none;
		}
	}
</style>
