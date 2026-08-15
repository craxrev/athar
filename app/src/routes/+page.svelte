<script lang="ts">
	import Detail from '$lib/Detail.svelte';
	import Icon from '$lib/Icon.svelte';
	import Lanes from '$lib/Lanes.svelte';
	import Rail from '$lib/Rail.svelte';
	import Reader from '$lib/Reader.svelte';
	import Settings from '$lib/Settings.svelte';
	import Stream from '$lib/Stream.svelte';
	import { clock, collector } from '$lib/collector.svelte';
	import {
		archive,
		type BlockDetail,
		type CollectorStatus,
		type Lane,
		type SessionDetail,
		type Summary
	} from '$lib/archive';
	import { compactDuration, duration, startOfDay, startOfMonth, startOfWeek, tokens } from '$lib/format';

	/** How many blocks the Stream renders at once. Beyond this the view says what
	 *  it is not showing rather than trying to draw a year. */
	const STREAM_LIMIT = 300;

	type Scope = 'day' | 'week' | 'month' | 'all';
	type View = 'lanes' | 'stream';

	/** The landing pair is Lanes over the week: lanes at day scope is two bars on
	 *  an empty axis, and lanes over a month is where the shape of the work shows.
	 *  After that the last used view and range are remembered — switching view by
	 *  itself when the range changes would make the app unpredictable. */
	let view = $state<View>((localStorage.getItem('lore.view') as View) ?? 'lanes');
	let scope = $state<Scope>((localStorage.getItem('lore.scope') as Scope) ?? 'week');
	let category = $state<string | null>(null);
	let project = $state<string | null>(null);
	let query = $state('');
	let railOpen = $state(true);
	let detailOpen = $state(true);

	let status = $state<CollectorStatus | null>(null);
	let summary = $state<Summary | null>(null);
	let lanes = $state<Lane[]>([]);
	let blocks = $state<BlockDetail[]>([]);
	let selectedBlock = $state<number | null>(null);
	let reader = $state<SessionDetail | null>(null);
	/** Settings takes over the window, like the reader: it is somewhere you go,
	 *  not a mode the timeline sits inside. */
	let settingsOpen = $state(false);
	let error = $state<string | null>(null);
	let statusError = $state<string | null>(null);
	/** Anything that escaped a handler. Every failure so far in this app has been
	 *  silent — a stuck view with no cause on screen — which is worse than a crash. */
	let crash = $state<string | null>(null);
	let loading = $state(true);

	let filterField = $state<HTMLInputElement | null>(null);

	/// Bumped when the collector has written since the last check, which makes the
	/// data effect below refetch. Plain `let`, not state: reading it inside the
	/// polling effect would make that effect depend on itself.
	let lastSeenScan: number | null = null;
	let archiveVersion = $state(0);

	/** The range as two numbers rather than an object.
	 *
	 *  An object is compared by reference, so a fresh one on every status poll
	 *  re-triggered the data effect; on the widest range that restarted a query
	 *  before the previous had finished and the view never settled. Numbers are
	 *  compared by value, and the end is quantised to the end of today so a
	 *  recomputation does not shift it by milliseconds. */
	let fromMs = $derived.by(() => {
		const now = Date.now();
		switch (scope) {
			case 'day':
				return startOfDay(now);
			case 'week':
				return startOfWeek(now);
			case 'month':
				return startOfMonth(now);
			default:
				return status?.earliest_ms ?? now - 365 * 86_400_000;
		}
	});
	let toMs = $derived(
		scope === 'day' ? startOfDay(Date.now()) + 86_400_000 : startOfDay(Date.now()) + 86_400_000
	);

	$effect(() => {
		const onError = (e: ErrorEvent) => (crash = `${e.message} — ${e.filename}:${e.lineno}`);
		const onRejection = (e: PromiseRejectionEvent) =>
			(crash = `unhandled rejection: ${String(e.reason)}`);
		window.addEventListener('error', onError);
		window.addEventListener('unhandledrejection', onRejection);
		return () => {
			window.removeEventListener('error', onError);
			window.removeEventListener('unhandledrejection', onRejection);
		};
	});

	$effect(() => {
		localStorage.setItem('lore.view', view);
		localStorage.setItem('lore.scope', scope);
	});

	// The collector is a separate process writing on its own schedule, so the
	// window has to ask. Without this the status footer reports the moment the
	// window opened for as long as it stays open — and that footer is the only
	// place collector health is visible, since there is no tray.
	async function pollStatus() {
		try {
			const next = await archive.status();
			// Keyed on what was archived, not on scans: a scan that finds nothing
			// still finishes, and reloading every view for it would be work for an
			// unchanged answer.
			if (next.last_archived_ms !== lastSeenScan) {
				lastSeenScan = next.last_archived_ms;
				archiveVersion += 1;
			}
			status = next;
			// A scheduled scan is invisible otherwise: it starts outside the window
			// and nothing else here would ever know it was happening.
			collector.observed = next.running;
			statusError = null;
		} catch (e) {
			// Reported separately: a failing status poll must not mask, or be
			// masked by, a failure to read the timeline.
			statusError = (e as Error).message;
		}
	}

	/** Clears a fault and refetches, rather than only hiding the panel.
	 *
	 *  A view abandoned mid-load looks exactly like a complete one, so dismissing
	 *  the message without reloading would leave behind the silent wrong answer
	 *  the panel exists to prevent. */
	function retry() {
		crash = null;
		error = null;
		void pollStatus();
		archiveVersion += 1;
	}

	// Scanning belongs to the window now, so this effect is what keeps the archive
	// current. It depends on the interval as a *number*, not on `status`: the poll
	// replaces that object every 30 seconds, and depending on it restarted the
	// timer each time — which meant a fresh scan every 30 seconds instead of every
	// hour. Restarting only when the number changes is also what makes the setting
	// take effect immediately.
	let intervalMins = $derived(status?.scan_interval_mins ?? 60);
	// Armed only once the archive has answered. Before that `last_scan_ms` is
	// unknown, and an unknown last scan looks exactly like "never scanned" — so the
	// window scanned on every open, however recently one had run, which is the
	// behaviour the interval exists to prevent.
	//
	// All three dependencies are primitives on purpose. Re-anchoring when a scan
	// finishes is correct, since the next one is a full interval from that scan;
	// depending on `status` itself would restart the timer on every poll.
	let ready = $derived(status !== null);
	let lastScanMs = $derived(status?.last_scan_ms ?? null);
	$effect(() => (ready ? collector.watch(intervalMins, lastScanMs) : undefined));

	$effect(() => {
		// One clock for every "N ago" in the window, started with the shell.
		const stopClock = clock.start();
		void pollStatus();
		const timer = setInterval(pollStatus, 30_000);
		// A separate, much cheaper beat: a warm scan lasts a few seconds and would
		// fall between two full polls, so the one thing the footer shows live is
		// asked for on its own schedule.
		const runTimer = setInterval(async () => {
			try {
				const action = await archive.collectorRun();
				collector.observed = action;
				// A finished scan means new data; the last-scan time and counts come
				// from the full poll, so bring that forward rather than wait for it.
				if (!action && collector.wasObserved) void pollStatus();
				collector.wasObserved = !!action;
			} catch {
				// A failing liveness check is not worth reporting: the full poll
				// already surfaces an unreadable archive.
			}
		}, 2_000);
		// Coming back to the window is exactly when a stale reading is noticed.
		const onFocus = () => void pollStatus();
		window.addEventListener('focus', onFocus);
		return () => {
			stopClock();
			clearInterval(timer);
			clearInterval(runTimer);
			window.removeEventListener('focus', onFocus);
		};
	});

	// Reloads whenever the range, the filters or the view change.
	$effect(() => {
		const from = fromMs;
		const to = toMs;
		const wanted = view;
		// Refetch when the collector has archived something new.
		void archiveVersion;
		loading = true;

		// Both queries run unfiltered. Filtering happens below, on what is already
		// loaded, so the rail can keep offering every category and project in the
		// range — including the one that would clear the current filter.
		Promise.all([
			archive.summary(from, to),
			archive.lanes(from, to),
			// Capped: the widest range holds thousands of blocks, and rendering
			// them all is neither useful nor fast.
			wanted === 'stream'
				? archive.timeline(from, to, STREAM_LIMIT)
				: Promise.resolve<BlockDetail[]>([])
		])
			.then(([s, l, b]) => {
				summary = s;
				lanes = l;
				blocks = b;
				error = null;
			})
			.catch((e: Error) => (error = e.message ?? String(e)))
			.finally(() => (loading = false));
	});

	// The detail pane needs the selected block's contents whichever view chose it.
	let selectedDetail = $state<BlockDetail | null>(null);
	/** Set while a selection is being read, so the pane says so rather than going
	 *  on showing the block that was selected before it. */
	let detailLoading = $state(false);
	/** Scoped to the pane on purpose: one selection failing to read is not a
	 *  reason to replace the whole timeline with an error screen. */
	let detailError = $state<string | null>(null);
	/** Identifies the newest selection.
	 *
	 *  Three bars clicked in two seconds start three reads, and without this the
	 *  last one to *answer* wins rather than the last one asked for — the pane
	 *  settles on a block the pointer already moved off. A stale answer is
	 *  dropped rather than raced. */
	let selectRequest = 0;

	function clearSelection() {
		selectRequest += 1;
		selectedBlock = null;
		selectedDetail = null;
		detailLoading = false;
		detailError = null;
	}

	async function select(blockId: number) {
		selectedBlock = blockId;
		detailOpen = true;
		detailError = null;

		// Stream loads its blocks in full, so a selection made there is already
		// in hand and never flickers through a loading state.
		const known = blocks.find((b) => b.id === blockId);
		if (known) {
			selectRequest += 1;
			selectedDetail = known;
			detailLoading = false;
			return;
		}

		// Cleared *before* the await, not after it. A pane still describing the
		// previous block beside a freshly highlighted bar is a confident wrong
		// answer, which is the one thing this window must never render — and in
		// Lanes, where no block is ever held locally, it was every selection.
		const ticket = ++selectRequest;
		selectedDetail = null;
		detailLoading = true;
		try {
			const found = await archive.block(blockId);
			if (ticket !== selectRequest) return;
			selectedDetail = found;
			// A rebuild renumbers derived rows, so a selection can outlive the
			// block it names. Saying so beats an empty pane.
			detailError = found ? null : 'That block is no longer in the archive — a rebuild has replaced it. Select another.';
		} catch (e) {
			if (ticket !== selectRequest) return;
			detailError = (e as Error).message;
		} finally {
			if (ticket === selectRequest) detailLoading = false;
		}
	}

	async function openSession(id: string) {
		try {
			reader = await archive.session(id);
		} catch (e) {
			error = (e as Error).message;
		}
	}

	/** Filtering narrows what is already loaded, across every field a person would
	 *  reach for: project, session title, commit subject, file path. */
	let filteredBlocks = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const scoped = blocks.filter(
			(b) =>
				(!category || b.category === category) &&
				(!project || b.project_path === project)
		);
		if (!q) return scoped;
		return scoped.filter(
			(b) =>
				b.project.toLowerCase().includes(q) ||
				b.category.includes(q) ||
				b.sessions.some((s) => s.title.toLowerCase().includes(q)) ||
				b.commits.some((c) => c.subject.toLowerCase().includes(q)) ||
				b.file_changes.some((f) => f.path.toLowerCase().includes(q))
		);
	});

	let filteredLanes = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const scoped = lanes.filter(
			(l) =>
				(!category || l.category === category) &&
				(!project || l.project_path === project)
		);
		if (!q) return scoped;
		return scoped.filter((l) => l.project.toLowerCase().includes(q) || l.category.includes(q));
	});

	let categories = $derived.by(() => {
		const totals = new Map<string, number>();
		for (const lane of lanes) totals.set(lane.category, (totals.get(lane.category) ?? 0) + lane.total_ms);
		return [...totals.entries()]
			.map(([name, ms]) => ({ name, ms }))
			.sort((a, b) => b.ms - a.ms || a.name.localeCompare(b.name));
	});

	/** Projects active in this range, busiest first. Anything outside the range is
	 *  not offered, because selecting it could only ever show nothing. */
	let railProjects = $derived.by(() => {
		const seen = new Map<string, number>();
		for (const l of lanes) seen.set(l.project, (seen.get(l.project) ?? 0) + 1);
		return lanes
			.filter((l) => !category || l.category === category)
			.map((l) => ({
				path: l.project_path,
				// A duplicate leaf name keeps its parent, so two different projects
				// called `profile-next` are distinguishable.
				label:
					(seen.get(l.project) ?? 0) > 1
						? l.project_path.split('/').filter(Boolean).slice(-2).join('/')
						: l.project,
				category: l.category,
				ms: l.total_ms
			}))
			.sort((a, b) => b.ms - a.ms || a.label.localeCompare(b.label));
	});

	/** The counted time, split by what evidences it. Same four classes and the
	 *  same words the lane legend uses: a split the timeline calls something
	 *  else would be two vocabularies for one fact. Zero-length parts are
	 *  dropped rather than printed as `0m`. */
	const EVIDENCE_SPLIT = [
		{ id: 'sessions', label: 'from sessions' },
		{ id: 'commits', label: 'from commits' },
		{ id: 'saves', label: 'from saves' },
		// Phrased "from …" like the rest so it reads in both positions: as a part
		// ("38m from records only") and as the whole ("all from records only").
		{ id: 'bare', label: 'from records only' }
	] as const;

	let split = $derived.by(() => {
		const by = summary?.by_evidence;
		if (!by) return [];
		return EVIDENCE_SPLIT.map((e) => ({ ...e, ms: by[e.id] })).filter((p) => p.ms > 0);
	});

	let isEmpty = $derived(
		!loading && !error && (view === 'lanes' ? filteredLanes.length === 0 : filteredBlocks.length === 0)
	);

	/** Every block on screen in view order, so the keyboard can walk them. */
	let walkable = $derived.by(() =>
		view === 'lanes'
			? filteredLanes.flatMap((l) => l.bars.map((b) => b.block_id))
			: filteredBlocks.map((b) => b.id)
	);

	function step(delta: number) {
		if (walkable.length === 0) return;
		const at = selectedBlock === null ? -1 : walkable.indexOf(selectedBlock);
		const next = at === -1 ? (delta > 0 ? 0 : walkable.length - 1) : at + delta;
		const target = walkable[Math.max(0, Math.min(next, walkable.length - 1))];
		if (target !== undefined) void select(target);
	}

	/** Single-key shortcuts, no modifier: this is a tool for someone whose hands
	 *  are already on the keyboard, and reaching for the mouse to change range is
	 *  the friction the brief rules out.
	 *
	 *  Two things scope them, and both were missing.
	 *
	 *  A field owns its own keys, decided by what the focused element *is* rather
	 *  than by which one it is. The filter input unmounts while settings is open,
	 *  so an identity test went false at exactly the moment settings' own fields
	 *  held focus: typing `30` into the scan interval set the range to This month
	 *  and left a `0` behind, and the arrow keys that increment a number input
	 *  walked the timeline instead.
	 *
	 *  And the timeline's keys belong to the timeline. The reader and settings
	 *  replace it rather than covering it, so switching a view or walking a
	 *  selection under them acts on something nobody can see, and is noticed only
	 *  on the way back. */
	function onKey(event: KeyboardEvent) {
		const focused = document.activeElement as HTMLElement | null;
		const typing =
			focused instanceof HTMLInputElement ||
			focused instanceof HTMLTextAreaElement ||
			focused?.isContentEditable === true;
		const covered = settingsOpen || !!reader;

		if (event.key === 'Escape') {
			// A field takes the first Escape: leaving what you were typing should
			// not also leave the surface you were typing on.
			if (typing) focused?.blur();
			else if (settingsOpen) settingsOpen = false;
			else if (reader) reader = null;
			else if (query) query = '';
			return;
		}

		// Settings is reachable from wherever you are, as its standard binding is.
		if (event.metaKey && event.key === ',') {
			event.preventDefault();
			settingsOpen = !settingsOpen;
			return;
		}
		// The panes it toggles are not rendered under the reader or settings, so
		// off that surface this would silently rearrange the view you return to.
		if (event.metaKey && event.key.toLowerCase() === 'b' && !covered) {
			event.preventDefault();
			if (event.shiftKey) detailOpen = !detailOpen;
			else railOpen = !railOpen;
			return;
		}

		// Everything below is the timeline's, and only the timeline's. Bailing
		// before any preventDefault is the point: a swallowed key is worse than
		// an ignored one, because the character never reaches the field either.
		if (covered || typing || event.metaKey || event.ctrlKey || event.altKey) return;

		if (event.key === '/') {
			event.preventDefault();
			filterField?.focus();
			return;
		}

		const scopeKeys: Record<string, Scope> = { '1': 'day', '2': 'week', '3': 'month', '4': 'all' };
		if (scopeKeys[event.key]) {
			event.preventDefault();
			scope = scopeKeys[event.key];
			clearSelection();
			return;
		}
		switch (event.key.toLowerCase()) {
			case 'l':
				view = 'lanes';
				break;
			case 's':
				view = 'stream';
				break;
			case 'j':
			case 'arrowdown':
				event.preventDefault();
				step(1);
				break;
			case 'k':
			case 'arrowup':
				event.preventDefault();
				step(-1);
				break;
			case 'enter': {
				const first = selectedDetail?.sessions[0];
				if (first) void openSession(first.id);
				break;
			}
		}
	}
</script>

<svelte:window onkeydown={onKey} />

<main
	class="shell"
	class:rail-closed={!railOpen}
	class:reading={!!reader}
>
	{#if !reader && !settingsOpen && railOpen}
		<Rail
			{scope}
			projects={railProjects}
			{category}
			{project}
			{categories}
			lastScanMs={status?.last_scan_ms ?? null}
			running={collector.busy}
			{intervalMins}
			onScope={(s) => {
				scope = s;
				clearSelection();
			}}
			onCategory={(c) => (category = c)}
			onProject={(p) => (project = p)}
		/>
	{/if}

	{#if settingsOpen}
		<Settings onClose={() => (settingsOpen = false)} />
	{:else if reader}
		<Reader detail={reader} onClose={() => (reader = null)} />
	{:else}
		<div class="centre">
			<div class="toolbar" data-tauri-drag-region>
				<button
					class="ghost"
					onclick={() => (railOpen = !railOpen)}
					title="Toggle scope rail (⌘B)"
					aria-label="Toggle scope rail"
				>
					<Icon name="panelLeft" size={18} />
				</button>

				<div class="views" role="group" aria-label="View">
					<button class:on={view === 'lanes'} onclick={() => (view = 'lanes')}>
						<Icon name="lanes" size={17} /> Lanes
					</button>
					<button class:on={view === 'stream'} onclick={() => (view = 'stream')}>
						<Icon name="stream" size={17} /> Stream
					</button>
				</div>

				<label class="filter">
					<Icon name="search" size={16} />
					<input
						bind:this={filterField}
						bind:value={query}
						type="text"
						placeholder="Filter projects, sessions, commits…"
						spellcheck="false"
					/>
					{#if query}
						<button class="clear" onclick={() => (query = '')} aria-label="Clear filter">
							<Icon name="close" size={14} />
						</button>
					{/if}
				</label>

				<button
					class="ghost"
					onclick={() => (detailOpen = !detailOpen)}
					title="Toggle detail (⇧⌘B)"
					aria-label="Toggle detail"
				>
					<Icon name="panelRight" size={18} />
				</button>
				<button
					class="ghost"
					onclick={() => (settingsOpen = true)}
					title="Settings (⌘,)"
					aria-label="Settings"
				>
					<Icon name="settings" size={18} />
				</button>
			</div>

			{#if summary && summary.blocks > 0}
				<!-- Insight lives beside the range it describes, not on its own screen.
				     Two tiers, because the strip carried six peers and led with none:
				     time answers "where did it go", the census answers "how much of
				     what". Two time figures because they answer different questions:
				     elapsed is wall clock, across-projects is the sum and may exceed
				     the day — which is why the split hangs off the second one. -->
				<div class="digest">
					<div class="time">
						<span class="lead" title="Wall clock: overlapping blocks counted once">
							<b class="big">{duration(summary.elapsed_ms)}</b> elapsed
						</span>
						<span title="Sum of every project's blocks; can exceed the range">
							<b class="mid">{compactDuration(summary.project_ms)}</b> across projects
						</span>
					</div>

					{#if split.length === 1}
						<p class="split"><span>all {split[0].label}</span></p>
					{:else if split.length > 1}
						<p class="split">
							{#each split as part (part.id)}
								<span><b>{compactDuration(part.ms)}</b> {part.label}</span>
							{/each}
						</p>
					{/if}

					<div class="census">
						<span><b>{summary.sessions}</b> sessions</span>
						<span><b>{summary.commits}</b> commits</span>
						<span><b>{tokens(summary.input_tokens + summary.output_tokens)}</b> tokens</span>
						{#if summary.ai_share !== null}
							<span
								class="inferred"
								title="Inferred: the share of files changed in this range that the assistant wrote, from the same attribution the commits carry"
							>
								<Icon name="inferred" size={13} />
								<b>{Math.round(summary.ai_share)}%</b> AI-written
							</span>
						{/if}
					</div>
				</div>
			{/if}

			<div class="stage">
				{#if crash}
					<div class="state">
						<h2>Something in the window failed</h2>
						<p class="mono">{crash}</p>
						<p>
							The archive is intact — this is the window, not the data. What was on screen may
							have been left half-loaded, so reloading is the way back.
						</p>
						<button class="act" onclick={retry}>Reload the view</button>
					</div>
				{:else if error}
					<div class="state">
						<h2>The archive could not be read</h2>
						<p class="mono">{error}</p>
						<p>Run <code>lore scan</code> in a terminal, then reopen this window.</p>
					</div>
				{:else if loading && !summary}
					<div class="state">
						<p>Reading the archive…</p>
						{#if statusError}<p class="mono">{statusError}</p>{/if}
					</div>
				{:else if isEmpty}
					<div class="state">
						<h2>Nothing recorded in this range</h2>
						{#if query}
							<p>No project, session or commit matches “{query}”.</p>
						{:else if status && status.records === 0}
							<p>The archive is empty. Run <code>lore scan</code> to read your history.</p>
						{:else}
							<p>
								lore may not have been running, or nothing happened. Widen the range to
								see what it does hold.
							</p>
						{/if}
					</div>
				{:else if view === 'lanes'}
					{#key `${scope}-${category}-${query}`}
						<Lanes
							lanes={filteredLanes}
							{fromMs}
							{toMs}
							{scope}
							selected={selectedBlock}
							onSelect={select}
						/>
					{/key}
				{:else}
					{#if summary && blocks.length >= STREAM_LIMIT && summary.blocks > blocks.length}
						<p class="truncated">
							Showing the most recent <b class="num">{blocks.length}</b> of
							<b class="num">{summary.blocks}</b> blocks in this range. Narrow the range to
							see the rest — the archive holds all of it.
						</p>
					{/if}
					<Stream
						blocks={filteredBlocks}
						selected={selectedBlock}
						onSelect={select}
						onOpenSession={openSession}
					/>
				{/if}
			</div>
		</div>

		{#if detailOpen}
			<Detail
				block={selectedDetail}
				loading={detailLoading}
				error={detailError}
				onOpenSession={openSession}
			/>
		{/if}
	{/if}
</main>

<style>
	.shell {
		display: flex;
		height: 100vh;
		background: var(--ground);
		overflow: hidden;
	}

	/* Below these widths a pane costs more than it gives: at 900px the three-pane
	   layout left the timeline about 280px, which is not a timeline. A pane taken
	   out here leaves no reserved space and nothing showing through. */
	@media (max-width: 1120px) {
		.shell:not(.reading) :global(aside.detail) {
			display: none;
		}
	}
	@media (max-width: 880px) {
		.shell:not(.reading) :global(nav.rail) {
			display: none;
		}
		.toolbar {
			padding-left: 82px;
		}
	}

	.centre {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
	}

	.toolbar {
		display: flex;
		align-items: center;
		gap: 10px;
		flex: none;
		height: calc(var(--titlebar) + 20px);
		padding: 14px 14px 0;
	}
	.shell.rail-closed .toolbar {
		/* Traffic lights sit here when the rail is closed. */
		padding-left: 82px;
	}

	.ghost {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border-radius: var(--radius-sm);
		color: var(--text-faint);
	}
	.ghost:hover {
		background: var(--surface-hover);
		color: var(--text);
	}

	.views {
		display: flex;
		gap: 2px;
		padding: 2px;
		border-radius: var(--radius);
		background: var(--surface);
	}
	.views button {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 5px 11px;
		border-radius: var(--radius-sm);
		color: var(--text-dim);
		font-size: 14px;
		font-weight: 560;
		transition: background var(--motion-state), color var(--motion-state);
	}
	.views button:hover {
		color: var(--text);
	}
	.views button.on {
		background: var(--accent-soft);
		color: var(--text);
		box-shadow: inset 0 0 0 1px var(--accent-edge);
	}
	.views button.on :global(svg) {
		color: var(--accent);
	}

	.filter {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		max-width: 420px;
		margin-left: auto;
		padding: 0 10px;
		height: 30px;
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface);
		color: var(--text-faint);
	}
	.filter:focus-within {
		border-color: var(--accent-edge);
		color: var(--text-dim);
	}
	.filter input {
		flex: 1;
		min-width: 0;
		border: none;
		background: none;
		outline: none;
		color: var(--text);
		font: inherit;
		font-size: 14px;
	}
	.filter input::placeholder {
		color: var(--text-faint);
	}
	.clear {
		display: grid;
		place-items: center;
		color: var(--text-faint);
	}
	.clear:hover {
		color: var(--text);
	}

	.digest {
		display: flex;
		flex-direction: column;
		flex: none;
		padding: 11px 18px 12px;
		border-bottom: 1px solid var(--line);
		font-size: 13.5px;
		font-weight: 500;
		color: var(--text-faint);
	}
	/* Every metric is one atomic unit, so a wrap lands between metrics and never
	   inside a phrase. */
	.digest span {
		white-space: nowrap;
	}
	.digest b {
		color: var(--text);
		font-family: var(--mono);
		font-variant-numeric: tabular-nums;
		font-weight: 620;
		font-size: 13.5px;
	}

	/* Tier one: where the time went. */
	.time {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 2px 18px;
	}
	.time .lead {
		font-size: 14px;
		color: var(--text-dim);
	}
	.digest b.big {
		font-size: 19px;
		font-weight: 640;
		letter-spacing: -0.01em;
	}
	.digest b.mid {
		font-size: 15px;
	}

	/* Bound to the figure above it by proximity: three of these sum to
	   across-projects exactly, which is the whole reason the split lands there
	   and not on elapsed. */
	.split {
		display: flex;
		flex-wrap: wrap;
		gap: 2px 14px;
		margin: 3px 0 0;
	}
	.split b {
		font-weight: 560;
	}

	/* Tier two: how much of what. More space above it than inside either group,
	   because it answers a different question. */
	.census {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 18px;
		margin-top: 9px;
	}
	/* Dimmer than the time tier and the split above it. These are supporting
	   counts; left at full strength they read louder than the figure they
	   support, which is the flat strip this replaced. */
	.census b {
		color: var(--text-dim);
	}
	/* The one figure here the archive infers rather than counts. The broken ring
	   is the same mark the commit tiers use, on the same axis — attribution —
	   so this borrows the existing grammar instead of starting another. */
	.inferred {
		display: inline-flex;
		align-items: center;
		gap: 5px;
	}
	.inferred :global(svg) {
		color: var(--amber);
	}

	.stage {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}

	/* A view that cannot show everything says so, rather than implying the archive
	   ends where the list does. */
	.truncated {
		flex: none;
		margin: 0;
		padding: 9px 20px;
		border-bottom: 1px solid var(--line);
		background: var(--amber-soft);
		color: var(--amber);
		font-size: 13px;
		line-height: 1.45;
	}
	.truncated b {
		font-family: var(--mono);
		font-weight: 620;
	}

	.state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
		height: 100%;
		padding: 40px;
		text-align: center;
		color: var(--text-dim);
	}
	.state h2 {
		margin: 0;
		font-size: 19px;
		font-weight: 620;
		color: var(--text);
	}
	.state p {
		margin: 0;
		max-width: 46ch;
		font-size: 14px;
		line-height: 1.55;
	}
	.state .mono {
		font-family: var(--mono);
		font-size: var(--fs-meta);
		color: var(--del);
	}
	.state :global(.act) {
		margin-top: 6px;
	}
	code {
		font-family: var(--mono);
		font-size: 0.92em;
		padding: 1px 5px;
		border-radius: 4px;
		background: var(--surface-raised);
		color: var(--text);
	}
</style>
