<script lang="ts">
	import type { Bar, Evidence, Lane } from './archive';
	import { clock, compactDuration } from './format';

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

	let {
		lanes,
		fromMs,
		toMs,
		scope,
		selected,
		onSelect
	}: {
		lanes: Lane[];
		fromMs: number;
		toMs: number;
		scope: 'day' | 'week' | 'month' | 'all';
		selected: number | null;
		onSelect: (blockId: number) => void;
	} = $props();

	/** Measured, because label collision is a pixel problem: the same 6% of track
	 *  is comfortable at 1000px and unreadable at 110px. */
	let trackWidth = $state(600);
	let axisTrack = $state<HTMLElement | null>(null);
	$effect(() => {
		if (!axisTrack) return;
		const observer = new ResizeObserver(([entry]) => {
			trackWidth = entry.contentRect.width || 1;
		});
		observer.observe(axisTrack);
		return () => observer.disconnect();
	});

	const span = $derived(Math.max(toMs - fromMs, 1));
	const left = (ms: number) => ((Math.max(ms, fromMs) - fromMs) / span) * 100;
	// A block holding one record has no span; without a floor it would be invisible.
	const width = (b: Bar) =>
		Math.max(((Math.min(b.ended_ms, toMs) - Math.max(b.started_ms, fromMs)) / span) * 100, 0.45);

	/** Below this the treatments stop being treatments: a hatch reads as noise and
	 *  a pair of end marks touch. Narrow bars keep their class colour and give up
	 *  the texture, which is the part that was never going to survive anyway. */
	const narrow = (b: Bar) => (width(b) / 100) * trackWidth < 14;

	/** Only the classes this range actually holds. A key that teaches a code the
	 *  view is not using is a puzzle, not a legend. */
	let present = $derived.by(() => {
		const seen = new Set<string>();
		for (const lane of lanes) for (const bar of lane.bars) seen.add(bar.evidence);
		return EVIDENCE.filter((e) => seen.has(e.id));
	});

	/** Ticks follow the range's own unit, so the axis never reads as a calendar
	 *  grid: hours within a day, days within a week or month, months beyond. */
	let ticks = $derived.by(() => {
		const out: { at: number; label: string; major: boolean }[] = [];
		if (scope === 'day') {
			for (let h = 0; h <= 24; h += 3) {
				const t = fromMs + h * 3_600_000;
				if (t > toMs) break;
				out.push({ at: left(t), label: `${String(h).padStart(2, '0')}:00`, major: h % 6 === 0 });
			}
			// A brand-new archive can hold a single day, and "all time" over it drew one
			// lone label with nothing to measure against. A short span reads as days
			// whatever the scope is called.
		} else if (scope === 'week' || scope === 'month' || toMs - fromMs <= 62 * 86_400_000) {
			const days = (toMs - fromMs) / 86_400_000;
			const step = days <= 9 ? 1 : days <= 40 ? 7 : 14;
			const cursor = new Date(fromMs);
			while (cursor.getTime() <= toMs) {
				const d = new Date(cursor);
				out.push({
					at: left(cursor.getTime()),
					// Weekday plus day number. The month belongs to the range, not to
					// every tick, and repeating it collided the labels.
					label: `${d.toLocaleDateString(undefined, { weekday: 'short' })} ${d.getDate()}`,
					major: d.getDay() === 1
				});
				cursor.setDate(cursor.getDate() + step);
			}
		} else {
			// The step follows the span rather than assuming years. Fixed at twelve
			// months, an archive spanning less than two calendar years drew exactly
			// one tick at position zero — the scope built for the long view had no
			// usable axis on the only data that exists yet.
			const months = Math.max(
				1,
				Math.round((toMs - fromMs) / (30.44 * 86_400_000))
			);
			const step = [1, 2, 3, 6, 12, 24, 60].find((n) => months / n <= 10) ?? 120;
			const cursor = new Date(fromMs);
			// Snap to the start of a period so labels land on round dates.
			cursor.setDate(1);
			cursor.setHours(0, 0, 0, 0);
			if (step >= 12) cursor.setMonth(0);
			else cursor.setMonth(Math.floor(cursor.getMonth() / step) * step);
			while (cursor.getTime() <= toMs) {
				const y = cursor.getFullYear();
				out.push({
					at: left(cursor.getTime()),
					label:
						step >= 12
							? String(y)
							: cursor.getMonth() === 0
								? String(y)
								: cursor.toLocaleDateString(undefined, { month: 'short' }),
					// January carries the year and anchors the eye; the rest are minor.
					major: step >= 12 || cursor.getMonth() === 0
				});
				cursor.setMonth(cursor.getMonth() + step);
			}
		}
		// Keep only labels that clear their neighbour by a readable gap. Below the
		// width where even two fit, the axis drops to its endpoints.
		const minPercent = (58 / Math.max(trackWidth, 1)) * 100;
		const spaced: typeof out = [];
		for (const t of out) {
			if (t.at < 0 || t.at > 97) continue;
			const previous = spaced[spaced.length - 1];
			if (previous && t.at - previous.at < minPercent) continue;
			spaced.push(t);
		}
		return spaced;
	});

	/** Lanes arrive grouped by category and sorted by activity within it, so the
	 *  order stays stable when the range changes. */
	let groups = $derived.by(() => {
		const out: { category: string; lanes: Lane[]; ms: number }[] = [];
		for (const lane of lanes) {
			let group = out.find((g) => g.category === lane.category);
			if (!group) {
				group = { category: lane.category, lanes: [], ms: 0 };
				out.push(group);
			}
			group.lanes.push(lane);
			group.ms += lane.total_ms;
		}
		return out;
	});

	/** Two projects can share a leaf name in different trees. Showing both as the
	 *  same word makes the lanes unreadable, so a duplicate keeps its parent. */
	let duplicates = $derived.by(() => {
		const seen = new Map<string, number>();
		for (const l of lanes) seen.set(l.project, (seen.get(l.project) ?? 0) + 1);
		return seen;
	});

	function label(lane: Lane): string {
		if ((duplicates.get(lane.project) ?? 0) < 2) return lane.project;
		const parts = lane.project_path.split('/').filter(Boolean);
		return parts.slice(-2).join('/');
	}

	function barTitle(lane: Lane, b: Bar): string {
		const parts = [`${lane.project}  ${clock(b.started_ms)}–${clock(b.ended_ms)}`];
		if (b.sessions) parts.push(`${b.sessions} session${b.sessions === 1 ? '' : 's'}`);
		if (b.commits) parts.push(`${b.commits} commit${b.commits === 1 ? '' : 's'}`);
		if (b.file_changes) parts.push(`${b.file_changes} file change${b.file_changes === 1 ? '' : 's'}`);
		// The class is the one thing the shape says and no number does. It rides
		// the same string as the label, so the screen reader gets it too.
		parts.push(evidenceLabel.get(b.evidence) ?? 'from records only');
		return parts.join(' · ');
	}
</script>

<div class="lanes">
	<div class="axis">
		<div class="gutter"></div>
		<div class="track" bind:this={axisTrack}>
			{#each ticks as t (t.at)}
				<span class="tick" class:major={t.major} style="left: {t.at}%">{t.label}</span>
			{/each}
		</div>
	</div>

	{#if present.length > 1}
		<div class="key">
			<div class="gutter"></div>
			<ul>
				{#each present as e (e.id)}
					<li>
						<span class="mark sample" data-evidence={e.id} aria-hidden="true"></span>
						{e.label}
					</li>
				{/each}
			</ul>
		</div>
	{/if}

	<div class="body">
		{#each groups as group (group.category)}
			<div class="group">
				<div class="grouphead">
					<span class="swatch" data-category={group.category} aria-hidden="true"></span>
					<h3>{group.category}</h3>
					<span class="num">{compactDuration(group.ms)}</span>
				</div>

				{#each group.lanes as lane, i (lane.project_path)}
					<div class="lane" style="--stagger: {Math.min(i, 12) * 22}ms">
						<div class="gutter">
							<span class="project" title={lane.project_path}>{lane.project}</span>
							<span class="num total">{compactDuration(lane.total_ms)}</span>
						</div>
						<div class="track">
							{#each ticks as t (t.at)}
								<span class="grid" class:major={t.major} style="left: {t.at}%"></span>
							{/each}
							{#each lane.bars as b (b.block_id)}
								<button
									class="bar mark"
									class:on={selected === b.block_id}
									class:narrow={narrow(b)}
									data-evidence={b.evidence}
									style="left: {left(b.started_ms)}%; width: {width(b)}%"
									title={barTitle(lane, b)}
									aria-label={barTitle(lane, b)}
									onclick={() => onSelect(b.block_id)}
								>
									{#if b.commits > 0 && width(b) >= 1.4}
										<span class="commits" aria-hidden="true">
											{#each Array(Math.min(b.commits, 4)) as _, c (c)}
												<i></i>
											{/each}
										</span>
									{/if}
								</button>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		{/each}
	</div>
</div>

<style>
	.lanes {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}

	.axis {
		display: flex;
		flex: none;
		height: 30px;
		border-bottom: 1px solid var(--line);
		background: var(--surface);
	}

	.gutter {
		width: clamp(112px, 16vw, 172px);
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
		padding-right: 12px;
	}

	.track {
		position: relative;
		flex: 1;
		min-width: 0;
	}

	.tick {
		position: absolute;
		top: 8px;
		padding-left: 6px;
		font-family: var(--mono);
		font-size: var(--fs-min);
		font-weight: 560;
		color: var(--text-faint);
		white-space: nowrap;
	}
	.tick.major {
		color: var(--text-dim);
	}

	/* The key reads once, at the top, and never repeats. Four shapes with no
	   legend is a puzzle; a legend for classes the range does not hold is a
	   different one, so it lists only what is on screen. */
	.key {
		display: flex;
		flex: none;
		padding: 9px 12px 3px 16px;
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
	.sample {
		position: relative;
		width: 26px;
		height: 12px;
		flex: none;
		border-radius: 3px;
	}

	.body {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		padding-bottom: 24px;
	}

	.group {
		padding-top: 22px;
	}

	.grouphead {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 0 12px 6px 16px;
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

	.swatch {
		width: 9px;
		height: 9px;
		border-radius: 3px;
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

	.lane {
		display: flex;
		align-items: center;
		height: 40px;
		padding-left: 16px;
	}
	.lane:hover {
		background: rgba(255, 255, 255, 0.022);
	}

	.gutter .project {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 14px;
		font-weight: 540;
		color: var(--text-dim);
	}
	.lane:hover .project {
		color: var(--text);
	}
	.total {
		color: var(--text-faint);
	}

	.grid {
		position: absolute;
		top: -6px;
		bottom: -6px;
		width: 1px;
		background: var(--line);
	}
	.grid.major {
		background: var(--line-strong);
	}

	/* ---- Evidence ----------------------------------------------------------
	   The fill carries what backs the span, not which project it belongs to.
	   Category is stated by the group heading directly above every lane and by
	   the rail row that filtered to it; inside a group every bar is the same
	   hue, so the hue was saying nothing here.

	   One ink per class drives all four treatments, which is what lets selection
	   recolour a bar without flattening what it is: magenta takes the ink, the
	   treatment survives. */
	/* Two inks, not one, because they answer to different rules. `--ink` is every
	   mark that carries the class and must clear 3:1 against the ground on its
	   own. `--ink-bed` sits behind a hatch, where the strokes carry identity and
	   the bed is only ground. Conflating them forced the hatch bed as bright as
	   a connector and washed the hatch out. */
	.mark {
		--ink: rgba(255, 255, 255, 0.36); /* 3.28:1 on --ground */
		--ink-bed: rgba(255, 255, 255, 0.11);
	}

	/* A conversation brackets the whole span, so the whole span is drawn. */
	.mark[data-evidence='sessions'] {
		background: var(--ink);
	}

	/* Exact at the commits, inferred between them by the idle-gap rule. The
	   hatch claims no positions, which is the point: it says "not continuous"
	   without pretending to know where inside the span the work fell. */
	.mark[data-evidence='commits'] {
		background-color: var(--ink-bed);
		background-image: repeating-linear-gradient(45deg, var(--ink) 0 2px, transparent 2px 6px);
	}

	/* Two mtimes and nothing known between them. A solid bar would claim the
	   middle; the ends are marked because the ends are what the archive has.
	   Coverage is a floor here, and amber is already this system's word for
	   that — coarse timestamps, gaps — so this borrows a meaning rather than
	   inventing one. */
	.mark[data-evidence='saves'] {
		--ink: var(--amber); /* 9.79:1 */
		/* The connector is part of the control's shape, not its background:
		   without it, two end marks are two unrelated dots. 3.34:1. */
		--ink-link: color-mix(in srgb, var(--amber) 52%, transparent);
		background-image: linear-gradient(var(--ink-link), var(--ink-link));
		background-repeat: no-repeat;
		background-position: center;
		background-size: 100% 3px;
	}
	.mark[data-evidence='saves']::before,
	.mark[data-evidence='saves']::after {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		width: 3px;
		border-radius: 2px;
		background: var(--ink);
	}
	.mark[data-evidence='saves']::before {
		left: 0;
	}
	.mark[data-evidence='saves']::after {
		right: 0;
	}

	/* Records the timeline cannot itemise — harness state, prompt history. The
	   span is real, so it keeps a line; nothing in it is describable, so the
	   line is broken and it claims no ends. */
	.mark[data-evidence='bare'] {
		background-image: repeating-linear-gradient(90deg, var(--ink) 0 3px, transparent 3px 7px);
		background-repeat: no-repeat;
		background-position: center;
		background-size: 100% 3px;
	}

	.bar {
		position: absolute;
		top: 50%;
		height: 23px;
		min-width: 3px;
		transform: translateY(-50%);
		border-radius: 4px;
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 2px;
		padding: 0 3px;
		overflow: hidden;
		/* The one authored moment: bars grow from their own start along the axis
		   when a range loads, staggered down the lanes.
		   Revealed rather than scaled: scaleX squashed the end marks of a
		   saves-only bar into slivers and stretched the hatch pitch, so the two
		   treatments that carry the most meaning were the two it distorted. */
		animation: grow 620ms cubic-bezier(0.16, 1, 0.3, 1) both;
		animation-delay: var(--stagger);
	}

	/* Too narrow for a treatment to survive. Class colour holds; texture goes. */
	.bar.narrow {
		background-image: none;
		background-color: var(--ink);
	}
	.bar.narrow::before,
	.bar.narrow::after {
		content: none;
	}

	.bar:hover {
		--ink: rgba(255, 255, 255, 0.54);
		--ink-bed: rgba(255, 255, 255, 0.2);
	}
	.bar[data-evidence='saves']:hover {
		--ink: color-mix(in oklab, var(--amber) 82%, white);
		--ink-link: color-mix(in srgb, var(--amber) 68%, transparent);
	}
	/* Selection takes the ink, never the shape: a selected saves-only block is
	   still visibly a pair of end marks, not a solid claim. */
	.bar.on,
	.bar.on:hover {
		--ink: var(--accent); /* 5.20:1 */
		--ink-bed: var(--accent-soft);
		--ink-link: color-mix(in srgb, var(--accent) 72%, transparent); /* 3.14:1 */
		box-shadow: var(--lift-1);
	}
	.bar.on .commits i {
		background: var(--on-accent);
	}

	.commits {
		display: flex;
		gap: 2px;
	}
	.commits i {
		width: 3px;
		height: 12px;
		border-radius: 2px;
		background: rgba(255, 255, 255, 0.82);
	}

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

	@media (prefers-reduced-motion: reduce) {
		.bar {
			animation: none;
		}
	}
</style>
