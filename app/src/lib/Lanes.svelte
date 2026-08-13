<script lang="ts">
	import type { Bar, Lane } from './archive';
	import { clock, compactDuration } from './format';

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
		onSelect: (blockId: number, projectPath: string) => void;
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
		} else if (scope === 'week' || scope === 'month') {
			const step = scope === 'week' ? 1 : 7;
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
			const cursor = new Date(fromMs);
			cursor.setMonth(0, 1);
			while (cursor.getTime() <= toMs) {
				out.push({
					at: left(cursor.getTime()),
					label: String(cursor.getFullYear()),
					major: true
				});
				cursor.setMonth(cursor.getMonth() + 12);
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
									class="bar"
									class:on={selected === b.block_id}
									class:has-commits={b.commits > 0}
									data-category={lane.category}
									style="left: {left(b.started_ms)}%; width: {width(b)}%"
									title={barTitle(lane, b)}
									aria-label={barTitle(lane, b)}
									onclick={() => onSelect(b.block_id, lane.project_path)}
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
		min-height: 0;
		height: 100%;
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
		font-size: 11.5px;
		font-weight: 500;
		color: var(--text-faint);
		white-space: nowrap;
	}
	.tick.major {
		color: var(--text-dim);
	}

	.body {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		padding-bottom: 24px;
	}

	.group {
		padding-top: 18px;
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
		background: #4c8dff;
	}
	.swatch[data-category='research'] {
		background: var(--accent);
	}
	.swatch[data-category='personal'] {
		background: #56c98a;
	}
	.swatch[data-category='freelance'] {
		background: var(--amber);
	}

	.lane {
		display: flex;
		align-items: center;
		height: 32px;
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

	.bar {
		position: absolute;
		top: 50%;
		height: 17px;
		min-width: 3px;
		transform: translateY(-50%);
		border-radius: 4px;
		background: rgba(255, 255, 255, 0.17);
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 2px;
		padding: 0 3px;
		overflow: hidden;
		/* The one authored moment: bars grow from their own start along the axis
		   when a range loads, staggered down the lanes. */
		animation: grow 620ms cubic-bezier(0.16, 1, 0.3, 1) both;
		animation-delay: var(--stagger);
		transform-origin: left center;
	}
	.bar[data-category='work'] {
		background: rgba(76, 141, 255, 0.42);
	}
	.bar[data-category='research'] {
		background: rgba(233, 61, 151, 0.38);
	}
	.bar[data-category='personal'] {
		background: rgba(86, 201, 138, 0.38);
	}
	.bar[data-category='freelance'] {
		background: rgba(242, 169, 59, 0.38);
	}
	.bar:hover {
		background: rgba(255, 255, 255, 0.3);
	}
	.bar.on {
		background: var(--accent);
		box-shadow: 0 1px 6px rgba(0, 0, 0, 0.45);
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
		height: 9px;
		border-radius: 1px;
		background: rgba(255, 255, 255, 0.82);
	}

	@keyframes grow {
		from {
			transform: translateY(-50%) scaleX(0.02);
			opacity: 0.4;
		}
		to {
			transform: translateY(-50%) scaleX(1);
			opacity: 1;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.bar {
			animation: none;
		}
	}
</style>
