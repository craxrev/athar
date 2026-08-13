<script lang="ts">
	import Icon from './Icon.svelte';
	import { compactDuration, relative } from './format';
	import { clock } from './collector.svelte';

	let {
		scope,
		projects,
		category,
		project,
		categories,
		lastScanMs,
		intervalMins,
		running,
		onScope,
		onCategory,
		onProject
	}: {
		scope: 'day' | 'week' | 'month' | 'all';
		projects: { path: string; label: string; category: string; ms: number }[];
		category: string | null;
		project: string | null;
		categories: { name: string; ms: number }[];
		lastScanMs: number | null;
		intervalMins: number;
		/** Set while a collector is working, whether this window started it or the
		 *  schedule did. The footer is the only place a background scan is visible. */
		running: string | null;
		onScope: (s: 'day' | 'week' | 'month' | 'all') => void;
		onCategory: (c: string | null) => void;
		onProject: (p: string | null) => void;
	} = $props();

	const scopes = [
		{ id: 'day', label: 'Today' },
		{ id: 'week', label: 'This week' },
		{ id: 'month', label: 'This month' },
		{ id: 'all', label: 'All time' }
	] as const;

	// The rail answers "what am I looking at". Long project lists stay readable by
	// showing the ones with recent activity first, which the query already orders.
	let visible = $derived(
		category ? projects.filter((p) => p.category === category) : projects
	);
</script>

<nav class="rail">
	<div class="drag" data-tauri-drag-region></div>

	<div class="brand">
		<span class="mark" aria-hidden="true">
			<i style="width: 15px"></i><i style="width: 11px"></i><i style="width: 7px"></i>
		</span>
		<span class="name">lore</span>
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
					<button class="row" class:on={category === null} onclick={() => onCategory(null)}>
						<span class="swatch all" aria-hidden="true"></span>
						<span class="label">Everything</span>
					</button>
				</li>
				{#each categories as c (c.name)}
					<li>
						<button
							class="row"
							class:on={category === c.name}
							onclick={() => onCategory(category === c.name ? null : c.name)}
						>
							<span class="swatch" data-category={c.name} aria-hidden="true"></span>
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
			<p class="empty">No project activity in this range.</p>
		{:else}
			<ul>
				{#each visible as p (p.path)}
					<li>
						<button
							class="row"
							class:on={project === p.path}
							onclick={() => onProject(project === p.path ? null : p.path)}
							title={p.path}
						>
							<span class="swatch" data-category={p.category} aria-hidden="true"></span>
							<span class="label">{p.label}</span>
							<span class="num trail">{compactDuration(p.ms)}</span>
						</button>
					</li>
				{/each}
			</ul>
		{/if}
	</section>

	<footer>
		<span
			class="dot"
			class:stale={lastScanMs === null}
			class:busy={!!running}
			aria-hidden="true"
		></span>
		<span>
			{#if running === 'rebuild'}
				Rebuilding…
			{:else if running}
				Scanning…
			{:else if lastScanMs === null}
				Never scanned
			{:else}
				Scanned {relative(lastScanMs, clock.now)}
			{/if}
			<span class="faint">· every {intervalMins}m</span>
		</span>
	</footer>
</nav>

<style>
	.rail {
		/* No background of its own: the window's vibrancy material shows through
		   here, and only here. Dense content lives on solid ground. */
		display: flex;
		flex-direction: column;
		width: 244px;
		flex: none;
		min-height: 0;
		overflow: hidden;
		border-right: 1px solid var(--line);
		background: rgba(255, 255, 255, 0.022);
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
		border-radius: 2px;
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
		transition: background 120ms ease-out, color 120ms ease-out;
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
		border-radius: 3px;
		background: var(--text-faint);
	}
	.swatch.all {
		background: var(--text-dim);
	}
	/* Categories come from the filesystem layout, so their set is open-ended;
	   these four are the ones this machine has. */
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
		border-radius: 50%;
		background: var(--add);
	}
	.dot.stale {
		background: var(--amber);
	}
	/* The one moving thing in the window, and only while work is actually
	   happening — a scan runs for minutes and otherwise shows nothing at all. */
	.dot.busy {
		background: var(--accent);
		animation: pulse 1.6s ease-in-out infinite;
	}
	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.dot.busy {
			animation: none;
		}
	}

	.faint {
		color: var(--text-faint);
	}
</style>
