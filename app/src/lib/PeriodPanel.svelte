<script lang="ts">
	import Icon from './Icon.svelte';
	import type { Evidence, Lane } from './archive';
	import { compactDuration, fullDay } from './format';
	import { hueStyle } from './palette.svelte';

	/** What the detail pane answers at the grains where nothing is selectable.
	 *
	 *  A tile, a cell and a month panel stand for a period, not a block, so the
	 *  pane that answers a block selection had nothing to say at month or all
	 *  time — it asked the reader to select a block on a screen that draws none.
	 *  This answers the unit those rungs actually draw, whichever it is.
	 *
	 *  Everything here is counted from the lanes already on screen. The day is a
	 *  slice of what the timeline was handed, so a second query could only fetch
	 *  the same rows again and risk disagreeing with the marks beside it. */
	const EVIDENCE: { id: Evidence; label: string }[] = [
		{ id: 'sessions', label: 'from sessions' },
		{ id: 'commits', label: 'from commits' },
		{ id: 'saves', label: 'from saves' },
		{ id: 'bare', label: 'from records only' }
	];

	let {
		lanes,
		at,
		kind,
		onOpen
	}: {
		lanes: Lane[];
		/** Local midnight of the day, or of the first of the month. */
		at: number;
		kind: 'day' | 'month';
		onOpen: (at: number, kind: 'day' | 'month') => void;
	} = $props();

	/** The exclusive end of the period, stepped by calendar rather than by a
	 *  fixed span: a local day is twenty-three or twenty-five hours twice a year,
	 *  and a month is never a fixed length at all. */
	function endOf(from: number, of: 'day' | 'month'): number {
		const d = new Date(from);
		if (of === 'day') d.setDate(d.getDate() + 1);
		else d.setMonth(d.getMonth() + 1);
		d.setHours(0, 0, 0, 0);
		return d.getTime();
	}

	let label = $derived(
		kind === 'day'
			? fullDay(at)
			: new Date(at).toLocaleDateString(undefined, { month: 'long', year: 'numeric' })
	);

	/** Days that hold something, so a quiet month says how quiet rather than only
	 *  how long. Not shown for a single day, where it could only ever be one. */
	let activeDays = $derived.by(() => {
		if (kind === 'day') return 0;
		const end = endOf(at, kind);
		const seen = new Set<number>();
		for (const lane of lanes) {
			for (const bar of lane.bars) {
				let from = Math.max(bar.started_ms, at);
				const to = Math.min(bar.ended_ms, end);
				while (from < to) {
					const d = new Date(from);
					d.setHours(0, 0, 0, 0);
					seen.add(d.getTime());
					const next = new Date(d);
					next.setDate(next.getDate() + 1);
					from = next.getTime();
				}
			}
		}
		return seen.size;
	});

	/** A block crossing midnight is counted only for the part that fell on this
	 *  day, the same split the timeline draws. */
	let held = $derived.by(() => {
		const end = endOf(at, kind);
		const projects: { project: string; path: string; category: string; ms: number }[] = [];
		const classes = new Map<Evidence, number>();
		let ms = 0;
		let blocks = 0;
		let sessions = 0;
		let commits = 0;
		let files = 0;

		for (const lane of lanes) {
			let laneMs = 0;
			for (const bar of lane.bars) {
				const from = Math.max(bar.started_ms, at);
				const to = Math.min(bar.ended_ms, end);
				if (to <= from) continue;
				laneMs += to - from;
				ms += to - from;
				classes.set(bar.evidence, (classes.get(bar.evidence) ?? 0) + (to - from));
				blocks += 1;
				sessions += bar.sessions;
				commits += bar.commits;
				files += bar.file_changes;
			}
			if (laneMs > 0) {
				projects.push({
					project: lane.project,
					path: lane.project_path,
					category: lane.category,
					ms: laneMs
				});
			}
		}

		projects.sort((a, b) => b.ms - a.ms);
		const split = EVIDENCE.filter((e) => classes.has(e.id)).map((e) => ({
			label: e.label,
			ms: classes.get(e.id) ?? 0
		}));
		return { ms, blocks, sessions, commits, files, projects, split };
	});

	let peak = $derived(held.projects[0]?.ms ?? 1);
</script>

<!-- The window's titlebar is overlaid, so every pane reserves its own drag
     strip at the top. -->
<div class="drag" data-tauri-drag-region></div>

<div class="scroll">
	<header>
		<h2>{label}</h2>
		<p class="lede">
			{#if held.blocks === 0}
				Nothing archived in this {kind}. lore may not have been running, or nothing
				happened.
			{:else}
				<b class="num">{compactDuration(held.ms)}</b> across
				<b class="num">{held.projects.length}</b>
				project{held.projects.length === 1 ? '' : 's'}, in
				<b class="num">{held.blocks}</b> block{held.blocks === 1 ? '' : 's'}{#if kind === 'month'},
					over <b class="num">{activeDays}</b> day{activeDays === 1 ? '' : 's'} that hold
					something{/if}.
			{/if}
		</p>
	</header>

	{#if held.blocks > 0}
		<!-- The same ranking the rail draws, at the scale of one day: the bar behind
		     each name is its share of the busiest project, so the shape of the day
		     reads before any figure does. -->
		<section>
			<h3>Where it went</h3>
			<ul class="projects">
				{#each held.projects as p (p.path)}
					<li>
						<span class="share" style="width: {(p.ms / peak) * 100}%" aria-hidden="true"></span>
						<span class="dot" style={hueStyle(p.category)} aria-hidden="true"></span>
						<span class="name" title={p.path}>{p.project}</span>
						<span class="num tail">{compactDuration(p.ms)}</span>
					</li>
				{/each}
			</ul>
		</section>

		<!-- The same split the digest prints, for one day. Every block carries
		     exactly one class, so these parts add up to the figure above them. -->
		<section>
			<h3>How it is known</h3>
			<ul class="split">
				{#each held.split as part (part.label)}
					<li><b class="num">{compactDuration(part.ms)}</b> {part.label}</li>
				{/each}
			</ul>
		</section>

		<section>
			<h3>What was recorded</h3>
			<ul class="census">
				<li><b class="num">{held.sessions}</b> session{held.sessions === 1 ? '' : 's'}</li>
				<li><b class="num">{held.commits}</b> commit{held.commits === 1 ? '' : 's'}</li>
				<li>
					<b class="num">{held.files}</b> file change{held.files === 1 ? '' : 's'}
					{#if held.files > 0}
						<span class="floor">at least — a save between two scans leaves one timestamp</span>
					{/if}
				</li>
			</ul>
		</section>
	{/if}

	<!-- Navigation is offered rather than forced. Clicking a tile or a month used
	     to jump the whole window there; now it answers here, and going is a
	     second, deliberate step. -->
	<button class="open" onclick={() => onOpen(at, kind)}>
		Open this {kind}
		<Icon name="chevron" size={15} />
	</button>
</div>

<style>
	.drag {
		height: var(--titlebar);
		flex: none;
	}

	.scroll {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
		padding: 6px 18px 32px;
	}

	header {
		padding: 10px 0 14px;
	}

	h2 {
		margin: 0;
		font-size: 21px;
		font-weight: 660;
		letter-spacing: -0.02em;
		color: var(--text);
	}

	.lede {
		margin: 7px 0 0;
		font-size: 13.5px;
		line-height: 1.5;
		color: var(--text-dim);
	}
	.lede b {
		color: var(--text);
		font-weight: 620;
	}

	section {
		padding-top: 18px;
	}

	h3 {
		margin: 0 0 8px;
		font-size: 13px;
		font-weight: 640;
		letter-spacing: 0.06em;
		text-transform: uppercase;
		color: var(--text-faint);
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.projects li {
		position: relative;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 7px;
		border-radius: var(--radius-sm);
		overflow: hidden;
		font-size: 14px;
		font-weight: 540;
		color: var(--text-dim);
	}
	/* Behind the label rather than beside it, so a row costs no more width than
	   the name it carries — the same device the rail's project list uses. */
	.share {
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		border-radius: var(--radius-sm);
		background: var(--fill-subtle);
	}
	.projects li > :not(.share) {
		position: relative;
	}
	.dot {
		width: 9px;
		height: 9px;
		flex: none;
		border-radius: var(--radius-swatch);
		background: var(--cat, var(--text-faint));
	}
	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.tail {
		flex: none;
		color: var(--text-faint);
	}

	.split li,
	.census li {
		padding: 3px 0;
		font-size: 13.5px;
		color: var(--text-dim);
	}
	.split b,
	.census b {
		color: var(--text);
		font-weight: 620;
	}

	/* The one figure on this pane the archive can only floor. */
	.floor {
		display: block;
		font-size: var(--fs-min);
		color: var(--text-faint);
	}

	.open {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		width: 100%;
		margin-top: 22px;
		padding: 9px 12px;
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface-raised);
		color: var(--text-dim);
		font-family: inherit;
		font-size: 14px;
		font-weight: 560;
		cursor: pointer;
		transition: background var(--motion-state), color var(--motion-state);
	}
	.open:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	@media (prefers-reduced-motion: reduce) {
		.open {
			transition: none;
		}
	}
</style>
