<script lang="ts">
	import type { Evidence, Lane } from './archive';
	import { clock, compactDuration } from './format';
	import Icon from './Icon.svelte';

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
		onDrill
	}: {
		lanes: Lane[];
		fromMs: number;
		toMs: number;
		scope: 'day' | 'week' | 'month' | 'all';
		selected: number | null;
		onSelect: (blockId: number) => void;
		/** Walk down the ladder to the period containing `at`. The page turns that
		 *  into an offset; this component never owns the range. */
		onDrill: (next: 'day' | 'week' | 'month', at: number) => void;
	} = $props();

	const DAY = 86_400_000;
	const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

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
	 *  cold, and it was precisely when no key was drawn. */
	let present = $derived.by(() => {
		const seen = new Set<Evidence>();
		for (const rec of byDay.values()) for (const c of rec.classes.keys()) seen.add(c);
		return EVIDENCE.filter((e) => seen.has(e.id));
	});

	let categories = $derived.by(() => {
		const seen = new Set<string>();
		for (const lane of lanes) seen.add(lane.category);
		return [...seen];
	});

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
		const when = new Date(at).toLocaleDateString(undefined, {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			year: 'numeric'
		});
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

	let monthCells = $derived.by(() => {
		const first = new Date(startOfDay(fromMs));
		first.setDate(1);
		const out: (number | null)[] = [];
		// Monday-first, matching the axis's own week rule.
		for (let pad = (first.getDay() + 6) % 7; pad > 0; pad--) out.push(null);
		const cursor = new Date(first);
		while (cursor.getMonth() === first.getMonth() && cursor.getTime() < toMs) {
			out.push(startOfDay(cursor.getTime()));
			cursor.setDate(cursor.getDate() + 1);
		}
		return out;
	});

	/** Years or months, at the widest grain only.
	 *
	 *  Both are the same cell at the same grain; what differs is what the cells
	 *  are grouped under. Years hold the whole archive at once and read as
	 *  density; months are the unit a person actually remembers a stretch of work
	 *  by — "that was a February thing" — and cost the vertical room to say so. */
	let allShape = $state<'years' | 'months'>('years');

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
				label: cursor.toLocaleDateString(undefined, { month: 'short', year: '2-digit' }),
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
</script>

<div class="lanes">
	{#if scope === 'day'}
		<!-- Grain: one hour. The day is wide enough for projects to sit side by
		     side on a shared axis, which is the one range where comparing them is
		     legible — so this rung keeps the project rows the others give up. -->
		<div class="hours day" aria-hidden="true">
			<span></span>
			<div class="marks">
				{#each [0, 3, 6, 9, 12, 15, 18, 21] as h (h)}
					<span class="hour" class:first={h === 0} style="left: {(h / 24) * 100}%"
						>{String(h).padStart(2, '0')}:00</span
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
						<span class="swatch" data-category={group.category} aria-hidden="true"></span>
						<h3>{group.category}</h3>
						<span class="num">{compactDuration(group.ms)}</span>
					</div>

					{#each group.rows as row, i (row.project)}
						<div class="prow" style="--stagger: {Math.min(i, 12) * 22}ms">
							<span class="project">{label(row.project)}</span>
							<div class="track">
								{#each row.spans as s (s.blockId + '-' + s.from)}
									<button
										class="span"
										class:on={selected === s.blockId}
										data-evidence={s.evidence}
										data-category={s.category}
										style="left: {offsetIn(dayStartMs, s.from)}%; width: {widthIn(dayStartMs, s)}%"
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
	{:else if scope === 'week' || rangeDays <= 10}
		<!-- Grain: one day, as a row. Every hour of it is resolved, and an empty
		     day stays in the run drawn empty — a quiet week reads as quiet
		     rather than as a short one. -->
		<div class="hours week" aria-hidden="true">
			<span></span>
			<span></span>
			<div class="marks">
				{#each [0, 3, 6, 9, 12, 15, 18, 21] as h (h)}
					<span class="hour" class:first={h === 0} style="left: {(h / 24) * 100}%"
						>{String(h).padStart(2, '0')}:00</span
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
						>{new Date(day).toLocaleDateString(undefined, {
							weekday: 'short',
							day: 'numeric',
							month: 'short'
						})}</span
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
								data-category={s.category}
								style="left: {offsetIn(day, s.from)}%; width: {widthIn(day, s)}%"
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
	{:else if scope === 'month' || rangeDays <= 45}
		<!-- Grain: one day, as a tile. Thirty days as rows is twenty-six pixels
		     each and a wasted axis; as tiles it is a page, and the tile is the
		     last rung where a day can still say a name. -->
		<div class="weekdays" aria-hidden="true">
			{#each WEEKDAYS as w (w)}<span>{w}</span>{/each}
		</div>

		<div class="body">
			<div class="month">
				{#each monthCells as cell, i (i)}
					{#if cell === null}
						<span class="tile pad"></span>
					{:else}
						{@const rec = byDay.get(cell)}
						{#if rec}
							{@const names = rec.names}
							<button
								class="tile"
								style="--stagger: {Math.min(i, 24) * 12}ms"
								title={dayTitle(cell, rec)}
								aria-label={dayTitle(cell, rec)}
								onclick={() => onDrill('day', cell)}
							>
								<span class="head">
									<span class="num day">{new Date(cell).getDate()}</span>
									<span class="num held">{compactDuration(rec.ms)}</span>
								</span>
								{#each names.slice(0, 2) as name (name)}
									<span class="who">
										<span class="swatch" data-category={categoryOf.get(name)} aria-hidden="true"
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
											data-category={s.category}
											style="left: {offsetIn(cell, s.from)}%; width: {Math.max(
												widthIn(cell, s),
												1.2
											)}%"
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
					onclick={() => (allShape = 'years')}>Years</button
				>
				<button
					type="button"
					class:on={allShape === 'months'}
					aria-pressed={allShape === 'months'}
					onclick={() => (allShape = 'months')}>Months</button
				>
			</div>
		</div>

		<div class="body">
			{#if allShape === 'months'}
				<div class="contact">
					{#each months as m (m.at)}
						<button
							class="mpanel"
							title="{new Date(m.at).toLocaleDateString(undefined, {
								month: 'long',
								year: 'numeric'
							})} · {m.ms ? compactDuration(m.ms) : 'nothing archived'}"
							onclick={() => onDrill('month', m.at)}
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
											data-category={rec?.category ?? ''}
											style="--weight: {weight(rec)}"
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
											data-category={rec?.category ?? ''}
											style="--weight: {weight(rec)}"
											title={dayTitle(cell, rec)}
											aria-label={dayTitle(cell, rec)}
											onclick={() => onDrill('month', cell)}
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

	{#if present.length}
		<div class="key">
			<ul>
				{#each present as e (e.id)}
					<li>
						<span class="sample" data-evidence={e.id} aria-hidden="true"></span>
						{e.label}
					</li>
				{/each}
				{#if categories.length > 1 && scope !== 'day'}
					{#each categories as c (c)}
						<li>
							<span class="swatch" data-category={c} aria-hidden="true"></span>
							{c}
						</li>
					{/each}
				{/if}
			</ul>
			<p class="held">
				<b class="num">{activeDays.length}</b> of
				<b class="num">{rangeDays}</b> days hold something
			</p>
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
	   it labels. */
	.marks {
		position: relative;
		min-width: 0;
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
		background: var(--text-faint);
		cursor: pointer;
		animation: grow 620ms cubic-bezier(0.16, 1, 0.3, 1) both;
		animation-delay: var(--stagger);
	}
	.span[data-category='work'] {
		background: var(--cat-work);
	}
	.span[data-category='research'] {
		background: var(--cat-research);
	}
	.span[data-category='personal'] {
		background: var(--cat-personal);
	}
	.span[data-category='freelance'] {
		background: var(--cat-freelance);
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
	/* Selection takes the ink, never the shape: a selected saves-only span is
	   still visibly a pair of end marks. */
	.span.on {
		background-color: var(--accent);
		box-shadow: var(--lift-1);
	}
	.span.on[data-evidence='saves'] {
		box-shadow:
			inset 2px 0 0 var(--on-accent),
			inset -2px 0 0 var(--on-accent),
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
		border-radius: 2px;
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
		background: var(--text-faint);
	}
	.micro i[data-category='work'] {
		background: var(--cat-work);
	}
	.micro i[data-category='research'] {
		background: var(--cat-research);
	}
	.micro i[data-category='personal'] {
		background: var(--cat-personal);
	}
	.micro i[data-category='freelance'] {
		background: var(--cat-freelance);
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
	.modes button.on {
		background: var(--accent-soft);
		color: var(--accent);
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
		background: var(--text-faint);
		opacity: calc(0.72 + var(--weight) * 0.28);
	}
	.mcell.held[data-category='work'] {
		background: color-mix(in oklab, var(--cat-work-tint) calc(var(--weight) * 55%), var(--cat-work));
	}
	.mcell.held[data-category='research'] {
		background: color-mix(in oklab, var(--cat-research-tint) calc(var(--weight) * 55%), var(--cat-research));
	}
	.mcell.held[data-category='personal'] {
		background: color-mix(in oklab, var(--cat-personal-tint) calc(var(--weight) * 55%), var(--cat-personal));
	}
	.mcell.held[data-category='freelance'] {
		background: color-mix(in oklab, var(--cat-freelance-tint) calc(var(--weight) * 55%), var(--cat-freelance));
	}

	/* Fifty-three columns cannot shrink below 636px, and the window's own 880px
	   minimum can leave this pane 508px. The sheet scrolls sideways as one piece
	   so every year stays column-aligned, and the year label pins to the left so
	   you never lose which row you are reading. */
	.sheets {
		padding: 18px var(--edge) 8px;
		overflow-x: auto;
		scrollbar-width: thin;
		scrollbar-color: var(--line-strong) transparent;
	}
	.year {
		display: grid;
		grid-template-columns: 42px minmax(0, max-content) auto;
		align-items: center;
		gap: 12px;
		margin-bottom: 13px;
	}
	.yr {
		position: sticky;
		left: 0;
		z-index: 1;
		padding-right: 6px;
		background: var(--ground);
		font-weight: 620;
		color: var(--text);
	}
	.grid {
		display: grid;
		grid-auto-flow: column;
		grid-template-rows: repeat(7, 10px);
		gap: 2px;
	}
	.cell {
		width: 10px;
		height: 10px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-swatch);
		background: var(--well);
	}
	/* The floor is 0.72 because magenta is the worst case: solid `cat-research`
	   over ground reaches 3:1 at 0.70 alpha and nothing that carries a class may
	   sit under it. Density then rides the hue rather than the alpha, mixing
	   toward the category's own tint — the token that exists precisely because a
	   solid hue is not legible small on a dark ground. A quiet day and a heavy
	   one stay a full step apart, and both clear the Legible Mark Rule. */
	.cell.held {
		cursor: pointer;
		background: var(--text-faint);
		opacity: calc(0.72 + var(--weight) * 0.28);
	}
	.cell.held[data-category='work'] {
		background: color-mix(in oklab, var(--cat-work-tint) calc(var(--weight) * 55%), var(--cat-work));
	}
	.cell.held[data-category='research'] {
		background: color-mix(in oklab, var(--cat-research-tint) calc(var(--weight) * 55%), var(--cat-research));
	}
	.cell.held[data-category='personal'] {
		background: color-mix(in oklab, var(--cat-personal-tint) calc(var(--weight) * 55%), var(--cat-personal));
	}
	.cell.held[data-category='freelance'] {
		background: color-mix(in oklab, var(--cat-freelance-tint) calc(var(--weight) * 55%), var(--cat-freelance));
	}
	.cell.held:hover {
		outline: 1px solid var(--accent);
		outline-offset: 1px;
	}
	.ytot {
		color: var(--text-faint);
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
		font-size: var(--fs-min);
		font-weight: 500;
		color: var(--text-faint);
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
		background: var(--text-faint);
	}
	.swatch[data-category='work'] {
		background: var(--cat-work);
	}
	.swatch[data-category='research'] {
		background: var(--cat-research);
	}
	.swatch[data-category='personal'] {
		background: var(--cat-personal);
	}
	.swatch[data-category='freelance'] {
		background: var(--cat-freelance);
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
		.open,
		.tile,
		.mpanel,
		.modes button {
			transition: none;
		}
	}
</style>
