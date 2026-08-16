<script lang="ts">
	import { untrack } from 'svelte';
	import PeriodPanel from '$lib/PeriodPanel.svelte';
	import Detail from '$lib/Detail.svelte';
	import Icon from '$lib/Icon.svelte';
	import Lanes from '$lib/Lanes.svelte';
	import { setCategories } from '$lib/palette.svelte';
	import Rail from '$lib/Rail.svelte';
	import Reader from '$lib/Reader.svelte';
	import Resizer from '$lib/Resizer.svelte';
	import Settings from '$lib/Settings.svelte';
	import Stream from '$lib/Stream.svelte';
	import { clock, collector } from '$lib/collector.svelte';
	import { markSelector, type Mark, type Scope } from '$lib/grain';
	import { readPref, writePref } from '$lib/prefs';
	import {
		archive,
		type BlockDetail,
		type CollectorStatus,
		type Lane,
		type SessionDetail,
		type Summary
	} from '$lib/archive';
	import {
		addDays,
		addMonths,
		compactDuration,
		duration,
		rangeLabel,
		retention,
		shortPath,
		startOfDay,
		startOfMonth,
		startOfWeek,
		tokens
	} from '$lib/format';

	/** How many blocks the Stream renders at once. Beyond this the view says what
	 *  it is not showing rather than trying to draw a year. */
	const STREAM_LIMIT = 300;

	type View = 'lanes' | 'stream';

	/** The landing pair is Lanes over the week: lanes at day scope is two bars on
	 *  an empty axis, and lanes over a month is where the shape of the work shows.
	 *  After that the last used view and range are remembered — switching view by
	 *  itself when the range changes would make the app unpredictable. */
	let view = $state<View>((readPref('view') as View) ?? 'lanes');
	/** Years or months at the widest grain. It lives here rather than inside the
	 *  timeline because the timeline is remounted whenever a filter changes, and
	 *  a view choice that resets when you narrow to a project is a view choice
	 *  the reader has to keep making. */
	let allShape = $state<'years' | 'months'>(
		(readPref('allShape') as 'years' | 'months') ?? 'years'
	);
	let scope = $state<Scope>((readPref('scope') as Scope) ?? 'week');
	/** How many periods back from the present the range sits. Zero is now.
	 *
	 *  Without this the range derived purely from `Date.now()`, so there was no way
	 *  to reach a specific past day at all — against a product whose first job is
	 *  reconstructing what happened on one. Ruling out the calendar grid removed the
	 *  grid, not the need to navigate. */
	let offset = $state(0);
	let category = $state<string | null>(null);
	let project = $state<string | null>(null);
	let query = $state('');
	let railOpen = $state(true);
	let detailOpen = $state(true);

	/** Below this the three panes cannot all be useful at once: at 1120px the outer
	 *  two leave the timeline about 500px, and at the window's own 880px minimum
	 *  they leave 264px, which is not a timeline.
	 *
	 *  This used to be a media query hiding `aside.detail` while `detailOpen` stayed
	 *  true — so selecting a block lit a bar and showed nothing, and ⇧⌘B did nothing
	 *  visible, for every width the window is allowed to be between 880 and 1120.
	 *  Rendering now follows state, and state follows the measurement, so the two
	 *  cannot disagree. The outer panes simply become mutually exclusive: detail is
	 *  the answer to a selection, the rail is how you ask the next question, and
	 *  both stay one keystroke away. */
	/* ---- pane widths ---------------------------------------------------------
	   The outer panes were fixed at 244 and 372, which are good defaults and
	   nobody else's business but the reader's: how much room a project list or a
	   commit message wants depends on the names in them.

	   No grid was needed for this, which the layout note used to assume. A width
	   on a `flex: none` item resizes exactly as well and keeps the property that
	   made flex right in the first place — a hidden pane reserves nothing, where a
	   grid track would sit there empty. */
	const RAIL = { min: 190, max: 420, base: 244 };
	const DETAIL = { min: 300, max: 620, base: 372 };
	/** What the middle pane may never fall below. The timeline is the window's
	 *  reason to exist; a splitter that can squeeze it to nothing is a splitter
	 *  that can break the app, and "the user chose it" is not a defence when the
	 *  only way back is dragging a 9px edge you can no longer see. */
	const CENTRE_MIN = 420;

	function storedWidth(key: string, base: number): number {
		const raw = Number(readPref(key));
		return Number.isFinite(raw) && raw > 0 ? raw : base;
	}
	/** What the reader asked for, which is not always what fits. Kept whole so
	 *  that narrowing the window and widening it again returns the pane to the
	 *  width they chose rather than to the one the narrow window allowed. */
	let railPref = $state(storedWidth('railWidth', RAIL.base));
	let detailPref = $state(storedWidth('detailWidth', DETAIL.base));
	let resizing = $state(false);
	let winWidth = $state(1280);

	$effect(() => {
		writePref('railWidth', String(railPref));
		writePref('detailWidth', String(detailPref));
	});

	const TIGHT = 1120;
	let tight = $state(false);
	$effect(() => {
		const mq = window.matchMedia(`(max-width: ${TIGHT}px)`);
		tight = mq.matches;
		const onChange = (e: MediaQueryListEvent) => (tight = e.matches);
		mq.addEventListener('change', onChange);
		return () => mq.removeEventListener('change', onChange);
	});

	// Only on the way in, and only when both are open. Reads of the pane state are
	// untracked so this settles once per crossing rather than fighting the toggles.
	$effect(() => {
		if (!tight) return;
		untrack(() => {
			if (!railOpen || !detailOpen) return;
			if (selectedBlock !== null) railOpen = false;
			else detailOpen = false;
		});
	});

	function toggleRail() {
		railOpen = !railOpen;
		if (tight && railOpen) detailOpen = false;
	}
	function toggleDetail() {
		detailOpen = !detailOpen;
		if (detailOpen) detailAsked = true;
		if (tight && detailOpen) railOpen = false;
	}

	/** Lanes is the view whose whole problem is horizontal room: with both side
	 *  panes open the timeline gets 492px of a 1280px window, and 123px at the
	 *  window's own 880px minimum. The detail pane is the answer to a selection,
	 *  so on arrival — Lanes, nothing selected — it has nothing to answer and
	 *  starts folded, handing the axis 372px back.
	 *
	 *  Asking for it outranks that for the rest of the session. The rule is about
	 *  what the window opens as, not a pane that keeps closing itself under
	 *  someone who wants it. */
	let detailAsked = $state(false);
	$effect(() => {
		if (view === 'lanes' && selectedBlock === null && !detailAsked) detailOpen = false;
	});

	let status = $state<CollectorStatus | null>(null);
	let summary = $state<Summary | null>(null);
	let lanes = $state<Lane[]>([]);
	let blocks = $state<BlockDetail[]>([]);
	let selectedBlock = $state<number | null>(null);
	/** The period a tile, a cell or a month panel stands for, and which kind it
	 *  is. Mutually exclusive with a block: the pane answers one selection, and a
	 *  grain draws only one kind of mark. */
	let selectedPeriod = $state<number | null>(null);
	let periodKind = $state<'day' | 'month'>('day');
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

	/** Which outer panes are actually on screen. The reader and settings replace
	 *  the centre rather than sitting beside it, so neither pane — nor its
	 *  splitter — exists while one of them is up. */
	let showRail = $derived(railOpen && !reader && !settingsOpen);
	let showDetail = $derived(detailOpen && !reader && !settingsOpen);

	/* The two limits resolve in order rather than in terms of each other, which
	   would be a cycle. The rail is sized first and reserves only the detail
	   pane's minimum; the detail pane then takes what is genuinely left. At the
	   worst case — 1121px with both panes dragged to their maxima — this lands the
	   rail at 401, the detail at 300 and the centre at exactly its floor. */
	let railLimit = $derived(
		Math.max(RAIL.min, Math.min(RAIL.max, winWidth - CENTRE_MIN - (showDetail ? DETAIL.min : 0)))
	);
	let railWidth = $derived(Math.max(RAIL.min, Math.min(railLimit, railPref)));
	let detailLimit = $derived(
		Math.max(DETAIL.min, Math.min(DETAIL.max, winWidth - CENTRE_MIN - (showRail ? railWidth : 0)))
	);
	let detailWidth = $derived(Math.max(DETAIL.min, Math.min(detailLimit, detailPref)));

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
	let range = $derived.by(() => {
		const now = Date.now();
		switch (scope) {
			case 'day': {
				const from = addDays(startOfDay(now), offset);
				return { from, to: addDays(from, 1) };
			}
			case 'week': {
				const from = addDays(startOfWeek(now), offset * 7);
				return { from, to: addDays(from, 7) };
			}
			case 'month': {
				const from = addMonths(startOfMonth(now), offset);
				return { from, to: addMonths(from, 1) };
			}
			default:
				// All time has no periods to step through; it is already everything.
				return {
					from: status?.earliest_ms ?? now - 365 * 86_400_000,
					to: startOfDay(now) + 86_400_000
				};
		}
	});
	let fromMs = $derived(range.from);
	let toMs = $derived(range.to);
	let atPresent = $derived(offset === 0);
	/** The one fact the product's guarantee rests on. It lives in the rail footer,
	 *  which folds itself below 1120px and is gone on every reader or settings
	 *  visit — so a closing window outranks a running scan for the range bar's
	 *  spare slot. */
	let health = $derived(retention(status?.last_scan_ms ?? null, clock.now));
	let steppable = $derived(scope !== 'all');

	function stepRange(by: number) {
		if (!steppable) return;
		// Never past the current period: there is no record of the future, and an
		// empty range that cannot contain anything is not a state worth reaching.
		offset = Math.min(0, offset + by);
		clearSelection();
	}

	/** A period chosen at a grain that draws periods rather than blocks. Clicking
	 *  the same one again clears it, exactly as a bar does. */
	function selectPeriod(at: number, kind: 'day' | 'month') {
		if (selectedPeriod === at) {
			selectedPeriod = null;
			detailOpen = false;
			return;
		}
		selectRequest += 1;
		selectedDetail = null;
		detailLoading = false;
		detailError = null;
		selectedBlock = null;
		selectedPeriod = at;
		periodKind = kind;
		detailOpen = true;
		if (tight) railOpen = false;
	}

	/** Walk down the grain ladder to the period holding `at`.
	 *
	 *  The timeline never owns its own range — it draws what the rail hands it —
	 *  so a click on a cell or a tile resolves to the same two pieces of state
	 *  the rail already sets. Offset stays "periods back from the present", which
	 *  is what makes this a navigation and not a second range model. */
	function drillTo(next: Scope, at: number) {
		const now = Date.now();
		if (next === 'day') {
			offset = Math.round((startOfDay(at) - startOfDay(now)) / 86_400_000);
		} else if (next === 'week') {
			offset = Math.round((startOfWeek(at) - startOfWeek(now)) / (7 * 86_400_000));
		} else {
			const there = new Date(at);
			const here = new Date(now);
			offset = (there.getFullYear() - here.getFullYear()) * 12 + (there.getMonth() - here.getMonth());
		}
		offset = Math.min(0, offset);
		scope = next;
		clearSelection();
	}

	$effect(() => {
		// ` · ` is this app's separator everywhere else it joins two facts.
		const onError = (e: ErrorEvent) => (crash = `${e.message} · ${e.filename}:${e.lineno}`);
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
		writePref('view', view);
		writePref('scope', scope);
		writePref('allShape', allShape);
	});

	/** Category colour is dealt from the configured roots, sorted, so it depends
	 *  on the set of names and nothing else — not the range on screen, which
	 *  holds only the categories active in it. Read once here and again whenever
	 *  a scan reports the config has changed underneath us. */
	$effect(() => {
		void archiveVersion;
		void archive
			.config()
			.then((view) => setCategories(view.config.roots.map((r) => r.category)))
			.catch(() => {
				// A palette is not worth failing the window over; categories stay
				// neutral until the next read succeeds.
			});
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

	// Lanes, range only. The rail derives its category and project options from
	// this, so narrowing it would remove the very rows that clear a filter.
	$effect(() => {
		const from = fromMs;
		const to = toMs;
		void archiveVersion;

		archive
			.lanes(from, to)
			.then((l) => {
				lanes = l;
				error = null;
			})
			.catch((e: Error) => {
				error = e.message ?? String(e);
				lanes = [];
			});
	});

	// Blocks, narrowed in SQL. The 300-cap has to apply to the *filtered* set or it
	// silently truncates a filter's results while the banner stays hidden: gated on
	// an unfiltered fetch, `summary.blocks > blocks.length` compared a narrowed
	// count against a whole-range 300 and read false exactly when it mattered.
	$effect(() => {
		const from = fromMs;
		const to = toMs;
		const wanted = view;
		const forProject = project;
		const forCategory = category;
		void archiveVersion;

		// Lanes draws no blocks, so this effect has nothing to fetch and must not
		// report on a fetch it did not make. It used to resolve an empty promise
		// and clear `error` on the way through — wiping a fault the *lanes* effect
		// had recorded, because that effect does not depend on the view and so
		// does not re-run to set it again. Switching to Stream and back therefore
		// cleared the archive's own error and the recovery button with it, and
		// only stepping the range brought either back.
		if (wanted !== 'stream') {
			blocks = [];
			loading = false;
			return;
		}

		loading = true;
		archive
			.timeline(from, to, STREAM_LIMIT, forProject ?? undefined, forCategory ?? undefined)
			.then((b) => {
				blocks = b;
				error = null;
			})
			.catch((e: Error) => {
				// The figures go with the failure. A digest still printing a confident
				// total above "the archive could not be read" is the most corrosive
				// state this window can render.
				error = e.message ?? String(e);
				summary = null;
				blocks = [];
			})
			.finally(() => (loading = false));
	});

	// The digest, on its own beat. It is a claim about what is on screen, so unlike
	// the two queries above it follows the rail's filters — and it is the only one
	// that has to refetch when they change, which is why it is a separate effect.
	$effect(() => {
		const from = fromMs;
		const to = toMs;
		const forProject = project;
		const forCategory = category;
		void archiveVersion;

		archive
			.summary(from, to, forProject ?? undefined, forCategory ?? undefined)
			.then((s) => (summary = s))
			.catch((e: Error) => {
				summary = null;
				error = e.message ?? String(e);
			});
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
		selectedPeriod = null;
		selectedDetail = null;
		detailLoading = false;
		detailError = null;
	}

	async function select(blockId: number) {
		// The bar is the toggle. Clicking the one already selected clears it and
		// closes the pane — a second click on the same thing means "done with
		// this", not "show it to me again". The pane closes outright rather than
		// leaving it to the arrival rule, which only folds a pane the reader has
		// never opened by hand.
		if (blockId === selectedBlock) {
			clearSelection();
			detailOpen = false;
			return;
		}

		selectedBlock = blockId;
		selectedPeriod = null;
		detailOpen = true;
		if (tight) railOpen = false;
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
			detailError = found ? null : 'That block is gone; a rebuild replaced it. Select another.';
		} catch (e) {
			if (ticket !== selectRequest) return;
			detailError = (e as Error).message;
		} finally {
			if (ticket === selectRequest) detailLoading = false;
		}
	}

	/** Where focus was before a surface took over, so closing returns it instead of
	 *  dropping the user at the top of the document. */
	let focusBeforeSurface: HTMLElement | null = null;
	function rememberFocus() {
		focusBeforeSurface = document.activeElement as HTMLElement | null;
	}
	function restoreFocus() {
		const target = focusBeforeSurface;
		focusBeforeSurface = null;
		// Deferred: the surface is still mounted this tick, and focusing an element
		// that is about to be replaced puts it straight back on the body.
		queueMicrotask(() => target?.focus?.({ preventScroll: true }));
	}

	async function openSession(id: string) {
		try {
			rememberFocus();
			reader = await archive.session(id);
		} catch (e) {
			error = (e as Error).message;
		}
	}

	/** Filtering narrows what is already loaded, across every field a person would
	 *  reach for: project, session title, commit subject, file path. */
	let filteredBlocks = $derived.by(() => {
		const q = query.trim().toLowerCase();
		// Project and category are applied in SQL now, so only the text query is
		// left to narrow here — re-applying them would be a second, drifting copy.
		const scoped = blocks;
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

	/** What a screen reader is told about the stage, in one place.
	 *
	 *  Nothing here announced anything before: not a load finishing, not a failure,
	 *  and not the view reloading under the reader when a background scan wrote to
	 *  the archive. Polite rather than assertive — none of it interrupts a task,
	 *  and a scan can land at any moment. */
	let announcement = $derived.by(() => {
		if (crash) return `The window hit an error: ${crash}`;
		if (error) return `The archive could not be read: ${error}`;
		if (statusError) return `Collector status is unavailable: ${statusError}`;
		if (loading) return 'Reading the archive';
		if (isEmpty)
			return activeFilters.length
				? `Nothing matches these filters: ${activeFilters.map((f) => f.label).join(', ')}`
				: 'Nothing recorded in this range';
		const n = view === 'lanes' ? filteredLanes.length : filteredBlocks.length;
		return view === 'lanes'
			? `${n} project${n === 1 ? '' : 's'} in this range`
			: `${n} block${n === 1 ? '' : 's'} in this range`;
	});

	/** Every narrowing currently applied, each with the control that lifts it.
	 *
	 *  The empty state used to branch on the query alone, so filtering to a project
	 *  and pressing 1 for Today produced "athar may not have been running" — the
	 *  archive blamed for a filter, with the cause off screen if the rail was
	 *  closed. */
	let activeFilters = $derived.by(() => {
		const out: { label: string; clear: () => void }[] = [];
		if (category) out.push({ label: `category ${category}`, clear: () => (category = null) });
		if (project)
			out.push({ label: shortPath(project, 2), clear: () => (project = null) });
		if (query) out.push({ label: `“${query}”`, clear: () => (query = '') });
		return out;
	});

	/** What a filter in this view can actually reach. Saying "no match" without
	 *  this was a claim the data does not support. */
	let searchReach = $derived.by(() => {
		if (!query) return null;
		if (view === 'lanes')
			return 'Lanes matches project and category names. Stream also searches sessions, commits and paths.';
		if (blocks.length >= STREAM_LIMIT)
			return `Searched the ${blocks.length} most recent blocks only.`;
		return null;
	});

	let isEmpty = $derived(
		!loading && !error && (view === 'lanes' ? filteredLanes.length === 0 : filteredBlocks.length === 0)
	);

	/** What the timeline drew, in the order the eye reads it.
	 *
	 *  Lanes publishes this rather than the page reassembling it. The page's own
	 *  version was a guess that held only while every rung drew blocks: after the
	 *  grain ladder, a month or all-time range draws days and months, and walking
	 *  it as blocks selected marks that were not on screen — the whole grid dimmed
	 *  with nothing lit, and the pane described a block nobody could see. It also
	 *  walked Lanes in time order while the week rung draws newest day first. */
	let lanesMarks = $state<Mark[]>([]);
	/** Empty whenever the timeline is not the thing on screen. Lanes keeps its
	 *  last published list when a fault or an empty range replaces it, and walking
	 *  that list would select a mark from the range before this one. */
	let lanesDrawn = $derived(!crash && !error && filteredLanes.length > 0);
	let walkable = $derived<Mark[]>(
		view === 'lanes'
			? lanesDrawn
				? lanesMarks
				: []
			: filteredBlocks.map((b) => ({ kind: 'block', id: b.id }))
	);

	function positionOf(mark: Mark): boolean {
		return mark.kind === 'block' ? mark.id === selectedBlock : mark.at === selectedPeriod;
	}

	function step(delta: number) {
		if (walkable.length === 0) return;
		const at = walkable.findIndex(positionOf);
		const next = at === -1 ? (delta > 0 ? 0 : walkable.length - 1) : at + delta;
		const target = walkable[Math.max(0, Math.min(next, walkable.length - 1))];
		if (target === undefined) return;
		// At either end the clamp lands back on what is already chosen, and both
		// selectors are toggles — so the last press of `j` used to clear the
		// selection instead of holding it. Staying put is the honest answer.
		if (at !== -1 && positionOf(target)) return;
		if (target.kind === 'block') void select(target.id);
		else selectPeriod(target.at, target.kind);
		reveal(target);
	}

	/** Bring a keyboard-chosen mark into view.
	 *
	 *  Walking with j/k moved the selection and the detail pane but never the
	 *  viewport, so past the fold the highlight went somewhere the user could not
	 *  see and the pane described something off screen. Deferred a frame because
	 *  the element may not have rendered yet when the range or view just changed.
	 *  The selector comes from the same module that types the mark, so the
	 *  attribute and the lookup cannot drift — they did once, when a rewrite of
	 *  the timeline dropped `data-block` and this quietly found nothing. */
	function reveal(mark: Mark) {
		requestAnimationFrame(() => {
			document
				.querySelector(markSelector(mark))
				?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
		});
	}

	/** Every binding, grouped, in one place. The sheet renders this list, so a
	 *  shortcut that exists but is not listed here is a bug in one direction and a
	 *  shortcut listed but not bound is a bug in the other — and nothing enforces
	 *  that, so the pair has to be kept honest by hand. Thirteen of these were
	 *  reachable and three were hinted, in tooltips, on icons. */
	const SHORTCUTS: { group: string; keys: [string, string][] }[] = [
		{
			group: 'Range',
			keys: [
				['1 2 3 4', 'Day, week, month, all time'],
				['←  →', 'Previous, next period']
			]
		},
		{
			group: 'View',
			keys: [
				['L', 'Lanes'],
				['S', 'Stream'],
				['⌘B', 'Scope rail'],
				['⇧⌘B', 'Detail pane'],
				// Contextual rather than global — the edge has to hold focus first —
				// but an accelerator nobody can find is not an accelerator.
				['⇥ then ←  →', 'Resize a pane edge']
			]
		},
		{
			group: 'Move',
			keys: [
				// "Mark" rather than "block": a month or an all-time range draws days
				// and months, and those are what these walk there.
				['J  ↓', 'Next mark'],
				['K  ↑', 'Previous mark'],
				['⏎', 'Open session'],
				['/', 'Filter']
			]
		},
		{
			group: 'Other',
			keys: [
				['⌘,', 'Settings'],
				['?', 'Shortcuts'],
				['esc', 'Close, then clear the filter']
			]
		}
	];
	let shortcutsOpen = $state(false);
	let sheetEl = $state<HTMLElement | null>(null);
	// Focus follows the sheet in and back out again. Without it a screen reader got
	// a scrim and no announcement, and Tab kept walking the timeline underneath.
	$effect(() => {
		if (shortcutsOpen) sheetEl?.focus({ preventScroll: true });
	});

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
			if (shortcutsOpen) {
				shortcutsOpen = false;
				restoreFocus();
			}
			else if (typing) focused?.blur();
			else if (settingsOpen) {
				settingsOpen = false;
				restoreFocus();
			} else if (reader) {
				reader = null;
				restoreFocus();
			}
			else if (query) query = '';
			return;
		}

		// Reachable from every surface, because not knowing the keys is the state it
		// exists to fix. Matched on the character, since `?` is a shifted key.
		if (event.key === '?' && !typing) {
			event.preventDefault();
			if (shortcutsOpen) {
				shortcutsOpen = false;
				restoreFocus();
			} else {
				rememberFocus();
				shortcutsOpen = true;
			}
			return;
		}
		if (shortcutsOpen) return;

		// Settings is reachable from wherever you are, as its standard binding is.
		if (event.metaKey && event.key === ',') {
			event.preventDefault();
			if (settingsOpen) {
				settingsOpen = false;
				restoreFocus();
			} else {
				rememberFocus();
				settingsOpen = true;
			}
			return;
		}
		// The panes it toggles are not rendered under the reader or settings, so
		// off that surface this would silently rearrange the view you return to.
		if (event.metaKey && event.key.toLowerCase() === 'b' && !covered) {
			event.preventDefault();
			if (event.shiftKey) toggleDetail();
			else toggleRail();
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
			offset = 0;
			clearSelection();
			return;
		}
		if (event.key === 'ArrowLeft') {
			event.preventDefault();
			stepRange(-1);
			return;
		}
		if (event.key === 'ArrowRight') {
			event.preventDefault();
			stepRange(1);
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

<svelte:window onkeydown={onKey} bind:innerWidth={winWidth} />

<main
	class="shell"
	class:rail-closed={!railOpen}
	class:resizing
>
	<h1 class="offscreen">athar — archive of your work</h1>
	<p class="offscreen" role="status" aria-live="polite">{announcement}</p>

	{#if showRail}
		<Rail
			width={railWidth}
			{scope}
			projects={railProjects}
			{category}
			{project}
			{categories}
			lastScanMs={status?.last_scan_ms ?? null}
			running={collector.busy}
			error={collector.error}
			{intervalMins}
			onScope={(s) => {
				scope = s;
				offset = 0;
				clearSelection();
			}}
			onCategory={(c) => (category = c)}
			onProject={(p) => (project = p)}
		/>
		<Resizer
			value={railWidth}
			min={RAIL.min}
			max={railLimit}
			base={RAIL.base}
			side="left"
			label="Scope rail width"
			onChange={(w) => (railPref = w)}
			onActive={(on) => (resizing = on)}
		/>
	{/if}

	{#if settingsOpen}
		<Settings
					onClose={() => {
						settingsOpen = false;
						restoreFocus();
					}}
					onSaved={() => {
						// A lane's category is derived from the config, so a saved root
						// changes what the archive answers and not merely what settings
						// displays. Status carries the scan interval, which the same save
						// may also have changed.
						archiveVersion += 1;
						void pollStatus();
					}}
				/>
	{:else if reader}
		<Reader
					detail={reader}
					onClose={() => {
						reader = null;
						restoreFocus();
					}}
				/>
	{:else}
		<div class="centre">
			<div class="toolbar" data-tauri-drag-region>
				<button
					class="ghost"
					onclick={toggleRail}
					title="Toggle scope rail (⌘B)"
					aria-label="Toggle scope rail"
				>
					<Icon name="panelLeft" size={18} />
				</button>

				<div class="views" role="group" aria-label="View">
					<button
							class:on={view === 'lanes'}
							aria-pressed={view === 'lanes'}
							onclick={() => (view = 'lanes')}
						>
						<Icon name="lanes" size={17} /> Lanes
					</button>
					<button
							class:on={view === 'stream'}
							aria-pressed={view === 'stream'}
							onclick={() => (view = 'stream')}
						>
						<Icon name="stream" size={17} /> Stream
					</button>
				</div>

				<label class="filter">
					<Icon name="search" size={16} />
					<input
						bind:this={filterField}
						bind:value={query}
						type="text"
						aria-label="Filter projects, sessions and commits"
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
					onclick={() => {
						rememberFocus();
						settingsOpen = true;
					}}
					title="Settings (⌘,)"
					aria-label="Settings"
				>
					<Icon name="settings" size={18} />
				</button>
				<!-- Last, so the two pane toggles bracket the toolbar the way the panes
				     bracket the window: left edge opens the left, right edge the right. -->
				<button
					class="ghost"
					onclick={toggleDetail}
					title="Toggle detail (⇧⌘B)"
					aria-label="Toggle detail"
				>
					<Icon name="panelRight" size={18} />
				</button>
			</div>

			<div class="context">
				<div class="rangebar">
					<button
					class="stepper back"
					onclick={() => stepRange(-1)}
					disabled={!steppable}
					title={steppable ? `Previous ${scope} (←)` : 'All time cannot be stepped'}
					aria-label="Previous {scope}"
					>
					<Icon name="chevron" size={16} />
					</button>
					<span class="range" aria-live="polite">{rangeLabel(scope, fromMs, toMs)}</span>
					<button
					class="stepper"
					onclick={() => stepRange(1)}
					disabled={!steppable || atPresent}
					title={steppable ? `Next ${scope} (→)` : 'All time cannot be stepped'}
					aria-label="Next {scope}"
					>
					<Icon name="chevron" size={16} />
					</button>
					{#if !railOpen && health.state === 'lapsed'}
						<span class="working bad" role="status">
							Not scanned in {health.daysSince} days. Some history is gone
						</span>
					{:else if !railOpen && health.state === 'alarm'}
						<span class="working warn" role="status">
							{health.daysLeft} day{health.daysLeft === 1 ? '' : 's'} of source history left
						</span>
					{:else if !railOpen && health.state === 'never'}
						<span class="working warn" role="status">Never scanned</span>
					{:else if collector.busy && !railOpen}
						<span class="working">
							<span class="pip" aria-hidden="true"></span>
							{collector.busy === 'rebuild' ? 'Rebuilding' : 'Scanning'}
						</span>
					{/if}
					{#each activeFilters as f (f.label)}
						<button class="chip small" onclick={f.clear} title="Clear this filter">
							{f.label}
							<Icon name="close" size={12} />
						</button>
					{/each}
					{#if !atPresent}
						<button
							class="now"
							onclick={() => {
								offset = 0;
								clearSelection();
							}}
						>
							Back to now
						</button>
					{/if}
				</div>

				{#if summary && summary.blocks > 0}
				<!-- Insight lives beside the range it describes, not on its own screen.
				     Two tiers, because the strip carried six peers and led with none:
				     time answers "where did it go", the census answers "how much of
				     what". Two time figures because they answer different questions:
				     elapsed is wall clock, across-projects is the sum and may exceed
				     the day — which is why the split hangs off the second one. -->
				<div class="digest">
					<!-- No glosses: the two labels carry the distinction themselves.
					     "Elapsed" is wall clock, "across projects" is a sum, and that is
					     also why the second can exceed the first. -->
					<div class="time">
						<span class="lead">
							<b class="big">{duration(summary.elapsed_ms)}</b> elapsed
						</span>
						<span>
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

					{#if query}
						<!-- The same rendering of the same value the filter chip above uses. A
						     raw path here could run the width of the pane, and printed one
						     narrowing two different ways in two places a line apart. -->
						<p class="scopes">
							Figures follow {project ? shortPath(project, 2) : (category ?? 'this range')},
							not “{query}”.
						</p>
					{/if}

					<div class="census">
						<span><b>{summary.sessions}</b> sessions</span>
						<span><b>{summary.commits}</b> commits</span>
						<span><b>{tokens(summary.input_tokens + summary.output_tokens)}</b> tokens</span>
						{#if summary.ai_share !== null}
							<span
								class="inferred"
								title="Share of changed files the assistant wrote. Inferred, from the same attribution the commits carry."
							>
								<Icon name="inferred" size={13} />
								<b>{Math.round(summary.ai_share)}%</b> AI-written
							</span>
						{/if}
					</div>
				</div>
			{/if}
			</div>

			<div class="stage">
				{#if crash}
					<div class="state">
						<h2>The window hit an error</h2>
						<p class="mono">{crash}</p>
						<p>The archive is intact. Reload to recover the view.</p>
						<button class="act" onclick={retry}>Reload</button>
					</div>
				{:else if error}
					<div class="state">
						<h2>The archive could not be read</h2>
						<p class="mono">{error}</p>
						<p>A scan rebuilds it.</p>
						<div class="acts">
							<button
								class="act strong"
								disabled={!!collector.busy}
								onclick={() => collector.run('scan')}
							>
								{collector.busy === 'scan' ? 'Scanning…' : 'Scan now'}
							</button>
							<button class="act" onclick={retry}>Try again</button>
						</div>
						{#if collector.error}<p class="mono">{collector.error}</p>{/if}
					</div>
				{:else if loading && !summary}
					<div class="state">
						<p>Reading the archive…</p>
						{#if statusError}<p class="mono">{statusError}</p>{/if}
					</div>
				{:else if isEmpty}
					<div class="state">
						{#if collector.busy}
							<!-- Gated on the collector, not on the record count. Gated on
							     `records === 0` this branch vanished the instant the first record
							     landed — the counter died exactly as it would first have moved,
							     and handed a running scan the words "athar may not have been
							     running". -->
							<h2>Reading your history</h2>
							<p>Scanning Claude Code sessions, git repositories and file timestamps.</p>
							<p class="counted">
								<b class="num">{status?.records ?? 0}</b> records so far
							</p>
						{:else if !status || status.records === 0}
							<!-- `!status` is the actual first run, and it was the one case this
							     branch could not match. With no archive on disk every command
							     fails, so `status` stays null — and the welcome was gated on it
							     being non-null, which left it reachable only in the odd case of
							     an archive file holding nothing. A brand new window landed on
							     "Nothing recorded in this range" instead: true, useless, and
							     offering no way forward. -->
							<h2>Nothing archived yet</h2>
							<p>
								athar reads Claude Code sessions, git history and file saves, and keeps
								them after those sources delete their own.
							</p>
							{#if !status?.roots?.length}
								<!-- Claude Code sits at a fixed path and needs no configuration, so a
								     scan is worth offering on its own. Git and file changes are not:
								     with no root there is nothing for them to read, and offering the
								     scan without saying so is how you get a scan that looks broken. -->
								<p class="reach">
									Git and file changes need a scanned root. Claude Code sessions are
									read without one.
								</p>
							{/if}
							<div class="acts">
								<button class="act strong" onclick={() => collector.run('scan')}>
									Read my history
								</button>
								<button
									class="act"
									onclick={() => {
										rememberFocus();
										settingsOpen = true;
									}}
								>
									Add a root…
								</button>
							</div>
						{:else if activeFilters.length}
							<h2>Nothing matches these filters</h2>
							<p>Narrowed to:</p>
							<p class="chips">
								{#each activeFilters as f (f.label)}
									<button class="chip" onclick={f.clear}>
										{f.label}
										<Icon name="close" size={13} />
									</button>
								{/each}
							</p>
							{#if searchReach}<p class="reach">{searchReach}</p>{/if}
						{:else}
							<h2>Nothing recorded in this range</h2>
							<!-- The cause worth naming, because it is the one the reader can act
							     on and the one the product's guarantee rests on. -->
							<p>athar scans only while its window is open. Try a wider range.</p>
						{/if}
					</div>
				{:else if view === 'lanes'}
					<!-- Remount on range and grouping, never on text. `query` is bound per
				     keystroke, so having it here rebuilt the subtree and replayed the
				     620ms staggered entrance on every character typed — while
				     `project`, which genuinely regroups the lanes, was missing. -->
				{#key `${scope}-${category}-${project}`}
						<Lanes
							lanes={filteredLanes}
							{fromMs}
							{toMs}
							{scope}
							selected={selectedBlock}
							onSelect={select}
							onDrill={drillTo}
							{selectedPeriod}
							onSelectPeriod={selectPeriod}
							{allShape}
							onShape={(next) => (allShape = next)}
							bind:marks={lanesMarks}
						/>
					{/key}
				{:else}
					{#if summary && blocks.length >= STREAM_LIMIT && summary.blocks > blocks.length}
						<p class="truncated">
							Showing the <b class="num">{blocks.length}</b> most recent of
							<b class="num">{summary.blocks}</b> blocks. Narrow the range to see the rest.
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

		{#if showDetail}
			<Resizer
				value={detailWidth}
				min={DETAIL.min}
				max={detailLimit}
				base={DETAIL.base}
				side="right"
				label="Detail pane width"
				onChange={(w) => (detailPref = w)}
				onActive={(on) => (resizing = on)}
			/>
			{#if selectedPeriod !== null}
				<aside class="periodpane" style="width: {detailWidth}px">
					<PeriodPanel
						lanes={filteredLanes}
						at={selectedPeriod}
						kind={periodKind}
						onOpen={(at, kind) => drillTo(kind, at)}
					/>
				</aside>
			{:else}
				<Detail
					width={detailWidth}
					block={selectedDetail}
					loading={detailLoading}
					error={detailError}
					onOpenSession={openSession}
				/>
			{/if}
		{/if}
	{/if}
	{#if shortcutsOpen}
		<!-- A sheet rather than a modal: it protects no task and interrupts none, so
		     it dismisses on any click and needs no confirmation. -->
		<!-- The scrim dismisses on click. It carried role="button" and tabindex="-1",
		     which is a handler no keyboard can reach wrapping the dialog it was meant
		     to dismiss; Escape is the keyboard route and always was. -->
		<div class="sheetwrap">
			<button
				class="scrim"
				aria-label="Close shortcuts"
				onclick={() => {
					shortcutsOpen = false;
					restoreFocus();
				}}
			></button>
			<div
				class="sheet"
				role="dialog"
				aria-modal="false"
				aria-label="Keyboard shortcuts"
				tabindex="-1"
				bind:this={sheetEl}
			>
				<h2>Keyboard</h2>
				<div class="groups">
					{#each SHORTCUTS as g (g.group)}
						<section>
							<h3>{g.group}</h3>
							<dl>
								{#each g.keys as [key, what] (key)}
									<dt><kbd>{key}</kbd></dt>
									<dd>{what}</dd>
								{/each}
							</dl>
						</section>
					{/each}
				</div>
				<p class="dismiss"><kbd>?</kbd> or <kbd>esc</kbd> to close.</p>
			</div>
		</div>
	{/if}
</main>

<style>
	.sheetwrap {
		position: fixed;
		inset: 0;
		display: grid;
		place-items: center;
		padding: 40px;
	}
	/* A real button rather than a div with a click handler: the dismiss affordance
	   is reachable by keyboard as well as pointer, and carries its own name. */
	.scrim {
		position: absolute;
		inset: 0;
		border: none;
		background: var(--scrim);
		cursor: default;
	}
	.sheet {
		position: relative;
		max-width: 620px;
		max-height: 100%;
		overflow-y: auto;
		padding: 22px 26px 18px;
		border: 1px solid var(--line-strong);
		border-radius: var(--radius);
		background: var(--surface);
		box-shadow: var(--lift-1);
	}
	.sheet h2 {
		margin: 0 0 16px;
		font-size: 19px;
		font-weight: 640;
		letter-spacing: -0.015em;
	}
	.groups {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
		gap: 18px 26px;
	}
	.sheet h3 {
		margin: 0 0 7px;
		font-size: var(--fs-meta);
		font-weight: 640;
		color: var(--text-faint);
	}
	.sheet dl {
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: baseline;
		gap: 6px 11px;
		margin: 0;
	}
	.sheet dd {
		margin: 0;
		font-size: var(--fs-meta);
		color: var(--text-dim);
	}
	kbd {
		display: inline-block;
		padding: 2px 7px;
		border: 1px solid var(--line-strong);
		border-radius: var(--radius-sm);
		background: var(--surface-raised);
		color: var(--text);
		font-family: var(--mono);
		font-size: var(--fs-min);
		font-weight: 560;
		white-space: nowrap;
	}
	.dismiss {
		margin: 18px 0 0;
		font-size: var(--fs-meta);
		color: var(--text-faint);
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

	.shell {
		display: flex;
		height: 100vh;
		background: var(--ground);
		overflow: hidden;
	}
	/* While an edge is being dragged the pointer is captured by the separator, but
	   a drag across text still selects it. The cursor is held here too so it does
	   not flicker back to a caret every time the pointer crosses a pane. */
	.shell.resizing {
		user-select: none;
		cursor: col-resize;
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
		padding-left: var(--traffic-inset-wide);
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
	/* Below the rule it overrides, not above it. `@media` adds no specificity, so
	   an identical selector earlier in the file loses on source order — this guard
	   was written and then silently did nothing. */
	@media (prefers-reduced-motion: reduce) {
		.views button {
			transition: none;
		}
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
	/* Drawn on the field so it frames the whole control — icon, input and clear
	   button — rather than a borderless box inside it. */
	.filter:has(input:focus-visible) {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.filter input {
		flex: 1;
		min-width: 0;
		border: none;
		background: none;
		/* The field draws the ring for the whole control; without this the input
		   drew a second one just inside it. */
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
		width: 24px;
		height: 24px;
		flex: none;
		border-radius: var(--radius-sm);
		color: var(--text-faint);
	}
	.clear:hover {
		color: var(--text);
	}

	/* Navigation and context in one line, above the figures and independent of
	   them: a range holding nothing still has to say which range it is, and still
	   has to let you leave. */
	.context {
		flex: none;
		border-bottom: 1px solid var(--line);
	}
	.rangebar {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 18px 0;
	}
	.stepper {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		border-radius: var(--radius-sm);
		color: var(--text-faint);
	}
	.stepper.back :global(svg) {
		transform: rotate(180deg);
	}
	.stepper:hover:not(:disabled) {
		background: var(--surface-hover);
		color: var(--text);
	}
	.stepper:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.rangebar:only-child {
		padding-bottom: 8px;
	}
	.range {
		font-size: 14px;
		font-weight: 560;
		color: var(--text-dim);
	}
	/* Shown only where the rail cannot: a scan is otherwise invisible whenever the
	   rail is folded, which below 1120px it folds by itself. */
	.working {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		margin-left: auto;
		text-align: right;
		font-size: var(--fs-meta);
		font-weight: 540;
		color: var(--text-dim);
	}
	.working.warn {
		color: var(--amber);
	}
	.working.bad {
		color: var(--del);
	}
	/* The same chip as the empty state's, at range-bar scale. Present whenever a
	   narrowing is, so the figures above are never an unexplained subset. */
	.chip.small {
		min-height: 22px;
		padding: 2px 8px;
		font-size: var(--fs-min);
	}
	.pip {
		width: 7px;
		height: 7px;
		border-radius: var(--radius-circle);
		background: var(--accent);
		animation: pip var(--motion-live) infinite alternate;
	}
	@keyframes pip {
		from {
			opacity: 1;
		}
		to {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.pip {
			animation: none;
		}
	}

	/* Only present when there is somewhere to come back from. */
	.now {
		margin-left: 4px;
		padding: 3px 9px;
		border-radius: var(--radius-pill);
		background: var(--accent-soft);
		box-shadow: inset 0 0 0 1px var(--accent-edge);
		color: var(--text);
		font-size: var(--fs-meta);
		font-weight: 540;
	}
	.now:hover {
		background: var(--surface-hover);
	}

	.digest {
		display: flex;
		flex-direction: column;
		flex: none;
		padding: 8px 18px 12px;
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

	/* Tier one: where the time went.

	   Baseline, not flex-start. The two figures are deliberately different sizes —
	   19px against 15px — so aligning their boxes at the top sat the smaller one's
	   text visibly high against the larger. `flex-start` was right only while each
	   figure carried a block gloss beneath it and the two read as columns; with
	   the glosses gone they are two figures on one line, and a line has a
	   baseline. */
	.time {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 6px 22px;
	}
	/* Brighter than the figure beside it, and the same size: the hierarchy between
	   these two metrics is carried by the numerals, so a half-pixel difference in
	   their labels reads as a mistake rather than as rank. */
	.time .lead {
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
		/* One of these metrics carries an icon and is an inline-flex of its own;
		   baseline keeps its numerals on the same line as the plain ones rather
		   than centring the pair against the row. */
		align-items: baseline;
		gap: 4px 18px;
		margin-top: 9px;
	}
	/* Dimmer than the time tier and the split above it. These are supporting
	   counts; left at full strength they read louder than the figure they
	   support, which is the flat strip this replaced. */
	.census b {
		color: var(--text-dim);
	}
	/* Every figure carries how it was established, and that includes what it
	   covers. These describe the range, not the filtered view under them. */
	.scopes {
		margin: 6px 0 0;
		color: var(--amber);
		white-space: normal;
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

	/* The same shell Detail draws, because it is the same pane — only its answer
	   differs. */
	.periodpane {
		display: flex;
		flex-direction: column;
		flex: none;
		min-height: 0;
		overflow: hidden;
		border-left: 1px solid var(--line);
		background: var(--surface);
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
	.acts {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 8px;
	}
	/* A live count, so a long first scan reads as progress rather than a stall.
	   Never a percentage: the total is not known until the walk finishes. */
	.counted {
		font-size: 15px;
		color: var(--text-dim);
	}
	.counted b {
		color: var(--text);
		font-weight: 620;
	}
	/* Each narrowing is its own control, so the way out is the thing that names
	   the cause. */
	.chips {
		display: flex;
		flex-wrap: wrap;
		justify-content: center;
		gap: 7px;
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 26px;
		padding: 4px 9px;
		border: 1px solid var(--line-strong);
		border-radius: var(--radius-pill);
		background: var(--surface-raised);
		color: var(--text-dim);
		font-size: var(--fs-meta);
		font-weight: 540;
	}
	.chip:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	/* What a filter in this view can actually reach. Without it, "no match" was a
	   claim about the whole archive that only ever searched part of it. */
	.reach {
		color: var(--amber);
	}

</style>
