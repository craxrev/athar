<script lang="ts">
	import Icon from './Icon.svelte';
	import { hueStyle } from './palette.svelte';
	import { compactDuration, relative, retention } from './format';
	import { clock } from './collector.svelte';

	let {
		width,
		scope,
		projects,
		category,
		project,
		categories,
		lastScanMs,
		intervalMins,
		running,
		error,
		onScope,
		onCategory,
		onProject
	}: {
		/** Owned by the page, which is the only place that knows what the centre
		 *  pane has left to give. */
		width: number;
		scope: 'day' | 'week' | 'month' | 'all';
		projects: { path: string; label: string; category: string; ms: number }[];
		category: string | null;
		project: string | null;
		categories: { name: string; ms: number }[];
		lastScanMs: number | null;
		intervalMins: number;
		/** A collector failed. It can fail with nobody watching, and this footer is
		 *  the only place in a trayless app where that is visible. */
		error: string | null;
		/** Set while a collector is working, whether this window started it or the
		 *  schedule did. The footer is the only place a background scan is visible. */
		running: string | null;
		onScope: (s: 'day' | 'week' | 'month' | 'all') => void;
		onCategory: (c: string | null) => void;
		onProject: (p: string | null) => void;
	} = $props();

	/** The unit, not the instance. These read as "Today" / "This week" only while
	 *  the range sits on the present; the range bar above the timeline names which
	 *  day or week is actually on screen, and these stay true at any offset. */
	const scopes = [
		{ id: 'day', label: 'By day' },
		{ id: 'week', label: 'By week' },
		{ id: 'month', label: 'By month' },
		{ id: 'all', label: 'All time' }
	] as const;

	/** How long a source keeps its own history before deleting it.
	 *
	 *  This is the single number the product depends on: athar has to run at least
	 *  once inside this window or that stretch is gone for good, from every source
	 *  at once. The footer used to report a timestamp in green — an archive five
	 *  days from permanent loss looked exactly like one scanned this morning. */
	let health = $derived(retention(lastScanMs, clock.now));
	let daysSince = $derived(health.daysSince);
	let daysLeft = $derived(health.daysLeft);
	let alarm = $derived(health.state === 'alarm');
	let lapsed = $derived(health.state === 'lapsed');

	/** What the footer is worth announcing, without the part of it that ticks. */
	let alert = $derived(
		running === 'rebuild'
			? 'Rebuilding the archive'
			: running
				? 'Scanning'
				: error
					? 'The last scan failed'
					: lastScanMs === null
						? 'Never scanned. Nothing kept yet'
						: lapsed
							? `Not scanned in ${daysSince} days`
							: alarm
								? `${daysLeft} day${daysLeft === 1 ? '' : 's'} of source history left`
								: ''
	);

	/** Every project in the range, busiest first, and deliberately uncapped.
	 *
	 *  This is the app's only enumeration of projects — no rung of the ladder
	 *  lists them as rows — so a cap here does not tidy a ranking, it removes a
	 *  filter. A project outside a top twelve would be unselectable from the one
	 *  surface that offers projects at all.
	 *
	 *  It is also not the kind of list a cap is for. Stream bounds at 300 because
	 *  a block card is a heavy thing to render a thousand of; a rail row is one
	 *  flex line, the list is already ordered by the thing you came to compare,
	 *  and it sits in its own scroller. Length costs a scroll here, not an answer. */
	let visible = $derived(category ? projects.filter((p) => p.category === category) : projects);

	/** No rung of the timeline lists projects as rows any more, so this list is
	 *  the only place they are enumerated — and the only place their sizes can be
	 *  compared. The durations were always here; the bar behind the label is what
	 *  turns a column of figures back into a ranking. Scaled against the busiest
	 *  in view, so the leader always fills and the rest read as shares of it. */
	let busiest = $derived(visible.reduce((most, p) => Math.max(most, p.ms), 0));
	let categoryPeak = $derived(categories.reduce((most, c) => Math.max(most, c.ms), 0));
	const share = (ms: number, peak: number) => (peak > 0 ? (ms / peak) * 100 : 0);
</script>

<nav class="rail" style="width: {width}px">
	<div class="drag" data-tauri-drag-region></div>

	<div class="brand">
		<span class="mark" aria-hidden="true">
			<i style="width: 15px"></i><i style="width: 11px"></i><i style="width: 7px"></i>
		</span>
		<span class="name">athar</span>
	</div>

	<section>
		<h2>Range</h2>
		<ul>
			{#each scopes as s (s.id)}
				<li>
					<button
						class="row"
						class:on={scope === s.id}
						aria-current={scope === s.id ? 'true' : undefined}
						onclick={() => onScope(s.id)}
					>
						<Icon name="clock" size={17} />
						<span class="label">{s.label}</span>
					</button>
				</li>
			{/each}
		</ul>
	</section>

	{#if categories.length > 1}
		<section>
			<h2>Category</h2>
			<ul>
				<li>
					<button
							class="row"
							class:on={category === null}
							aria-current={category === null ? 'true' : undefined}
							onclick={() => onCategory(null)}
						>
						<span class="swatch all" aria-hidden="true"></span>
						<span class="label">Everything</span>
					</button>
				</li>
				{#each categories as c (c.name)}
					<li>
						<button
							class="row"
							class:on={category === c.name}
							aria-current={category === c.name ? 'true' : undefined}
							onclick={() => onCategory(category === c.name ? null : c.name)}
						>
							<span class="share" style="width: {share(c.ms, categoryPeak)}%" aria-hidden="true"
							></span>
							<span class="swatch" style={hueStyle(c.name)} aria-hidden="true"></span>
							<span class="label">{c.name}</span>
							<span class="num trail">{compactDuration(c.ms)}</span>
						</button>
					</li>
				{/each}
			</ul>
		</section>
	{/if}

	<section class="grow">
		<h2>Projects</h2>
		{#if visible.length === 0}
			<p class="empty">No projects in this range.</p>
		{:else}
			<ul>
				{#each visible as p (p.path)}
					<li>
						<button
							class="row"
							class:on={project === p.path}
							aria-current={project === p.path ? 'true' : undefined}
							onclick={() => onProject(project === p.path ? null : p.path)}
							title={p.path}
						>
							<span class="share" style="width: {share(p.ms, busiest)}%" aria-hidden="true"></span>
							<span class="swatch" style={hueStyle(p.category)} aria-hidden="true"></span>
							<span class="label">{p.label}</span>
							<span class="num trail">{compactDuration(p.ms)}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<!-- The announcement is carried here rather than by the footer itself.
	     `role="status"` on the footer meant the whole line was a live region, and
	     the line contains "scanned 4m ago" — a value the window's clock advances
	     every thirty seconds. A screen reader was told the collector's status,
	     unprompted, about once a minute. What is worth interrupting for is the
	     state, and a healthy archive is not news: the settled case announces
	     nothing at all, which is also the only case that ticks. -->
	<p class="offscreen" role="status">{alert}</p>

	<footer>
		<span
			class="dot"
			class:stale={lastScanMs === null || alarm}
			class:lapsed
			class:busy={!!running}
			aria-hidden="true"
		></span>
		<span class="state">
			{#if running === 'rebuild'}
				Rebuilding…
			{:else if running}
				Scanning…
			{:else if error}
				<span class="bad">The last scan failed</span>
				<span class="faint">· {error}</span>
			{:else if lastScanMs === null}
				Never scanned
				<span class="faint">· nothing kept yet</span>
			{:else if lapsed}
				<span class="bad">Not scanned in {daysSince} days</span>
				<span class="faint">· some history is gone</span>
			{:else if alarm}
				<span class="warn">{daysLeft} day{daysLeft === 1 ? '' : 's'} of source history left</span>
				<span class="faint">· scanned {relative(lastScanMs, clock.now)}</span>
			{:else}
				Scanned {relative(lastScanMs, clock.now)}
				<span class="faint">· every {intervalMins}m</span>
			{/if}
		</span>
	</footer>
</nav>

<style>
	.rail {
		/* No background of its own: the window's vibrancy material shows through
		   here, and only here. Dense content lives on solid ground. */
		display: flex;
		flex-direction: column;
		flex: none;
		min-height: 0;
		overflow: hidden;
		border-right: 1px solid var(--line);
		background: var(--wash);
	}

	.drag {
		height: var(--titlebar);
		flex: none;
	}

	.brand {
		display: flex;
		align-items: center;
		gap: 9px;
		padding: 4px 16px 18px;
	}

	.mark {
		display: flex;
		flex-direction: column;
		gap: 2.5px;
	}
	.mark i {
		height: 3px;
		border-radius: var(--radius-mark);
		background: var(--accent);
	}
	.mark i:nth-child(2) {
		background: color-mix(in oklab, var(--accent) 62%, var(--ground));
	}
	.mark i:nth-child(3) {
		background: color-mix(in oklab, var(--accent) 34%, var(--ground));
	}

	.name {
		font-size: 19px;
		font-weight: 640;
		letter-spacing: -0.015em;
	}

	section {
		padding: 0 8px 20px;
		min-height: 0;
	}
	section.grow {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}
	section.grow ul {
		overflow-y: auto;
		min-height: 0;
	}

	h2 {
		margin: 0 0 6px;
		padding: 0 8px;
		font-size: 13px;
		font-weight: 620;
		color: var(--text-faint);
	}

	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}

	.row {
		position: relative;
		display: flex;
		align-items: center;
		gap: 9px;
		width: 100%;
		padding: 7px 8px;
		border-radius: var(--radius-sm);
		color: var(--text-dim);
		font-size: 14.5px;
		font-weight: 520;
		text-align: left;
		overflow: hidden;
		transition: background var(--motion-state), color var(--motion-state);
	}

	/* The ranking, folded into the list that already carried the number. It sits
	   behind the label rather than beside it, so a row costs no more width than
	   it did and the figure stays the thing you read. */
	.share {
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		border-radius: var(--radius-sm);
		background: var(--fill-subtle);
	}
	.row.on .share {
		background: var(--accent-soft);
	}
	.row > :not(.share) {
		position: relative;
	}
	.row:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	/* Selection is a flat fill of the accent, at low alpha with a legible edge —
	   no glow anywhere. */
	.row.on {
		background: var(--accent-soft);
		color: var(--text);
		box-shadow: inset 0 0 0 1px var(--accent-edge);
	}
	.row.on :global(svg) {
		color: var(--accent);
	}
	/* Drawn inside, as Stream draws it: the project list is a scroller, and a
	   scroller clips an outset ring against its own edges — so the rows that
	   scroll lost the left and right sides of theirs. */
	.row:focus-visible {
		outline-offset: -2px;
	}

	.label {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.trail {
		color: var(--text-faint);
	}

	.swatch {
		width: 9px;
		height: 9px;
		flex: none;
		border-radius: var(--radius-swatch);
		background: var(--cat, var(--text-faint));
	}
	.swatch.all {
		background: var(--text-dim);
	}
	/* Categories come from the filesystem layout, so their set is open-ended;
	   these four are the ones this machine has. */

	.empty {
		margin: 2px 8px;
		font-size: 13.5px;
		color: var(--text-faint);
	}

	footer {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 11px 16px;
		border-top: 1px solid var(--line);
		font-size: 13px;
		color: var(--text-dim);
	}

	.dot {
		width: 7px;
		height: 7px;
		flex: none;
		border-radius: var(--radius-circle);
		background: var(--add);
	}
	.dot.stale {
		background: var(--amber);
	}
	.dot.lapsed {
		background: var(--del);
	}
	/* The only repeating motion in the app, and the only place it is warranted:
	   something is happening right now, outside this window, and will stop on its
	   own. Magenta because live state is magenta. */
	.dot.busy {
		background: var(--accent);
		animation: pulse var(--motion-live) infinite alternate;
	}
	@keyframes pulse {
		from {
			opacity: 1;
		}
		to {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.dot.busy {
			animation: none;
		}
		.row {
			transition: none;
		}
	}

	.faint {
		color: var(--text-faint);
	}
	/* The two states worth interrupting a glance for. Amber is this system's word
	   for uncertainty and a closing window; red is for a loss already taken. */
	.warn {
		color: var(--amber);
		font-weight: 560;
	}
	.bad {
		color: var(--del);
		font-weight: 560;
	}
	.state {
		min-width: 0;
		overflow-wrap: anywhere;
	}

	/* Present to assistive technology, absent to the eye. Not display:none, which
	   would take it out of the accessibility tree along with everything else. */
	.offscreen {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: -1px;
		padding: 0;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
		border: 0;
	}
</style>
