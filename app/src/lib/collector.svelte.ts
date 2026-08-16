import { archive } from './archive';

/** What the window knows about the collector, held above any one screen.
 *
 *  A scan is a separate process: it keeps running when settings closes, and it
 *  can fail after nobody is watching. State owned by the settings component was
 *  destroyed along with it, so reopening the pane showed an idle button beside a
 *  scan still in flight — and pressing it started a second one. The pending
 *  reminders went the same way, which was worse: closing the pane silently threw
 *  away the only thing saying a change had not taken effect yet.
 *
 *  These still reset when the app restarts. What survives a restart is the
 *  archive itself, which is what the banner in settings reads. */
class Collector {
	/** 'scan' or 'rebuild' while one is running, so the button can refuse a second. */
	running = $state<string | null>(null);
	result = $state<string | null>(null);
	error = $state<string | null>(null);
	/** A change was saved that the stored data does not reflect yet. */
	needsScan = $state(false);
	needsRebuild = $state(false);

	/** What the archive says is running, which covers scheduled scans this window
	 *  never started. Refreshed by the status poll. */
	observed = $state<string | null>(null);
	/** Whether the last liveness check saw a run, so its ending can be noticed. */
	wasObserved = false;

	/** A collector is working, whoever started it. `running` reacts immediately to
	 *  this window's own button; `observed` lags the poll but sees everything. */
	get busy() {
		return this.running ?? this.observed;
	}

	/** Scans while the window is open, which is the only schedule athar has.
	 *
	 *  There is no installed agent and no OS scheduler: nothing to register, keep
	 *  current, or reimplement per platform. The cost is plain — a source deletes
	 *  its own history within about 30 days, so athar has to be opened inside that
	 *  window or the history is gone. Opening it is the act that archives.
	 *
	 *  Runs once on open, because a window opened after days away has the most to
	 *  catch up on. */
	/** @param lastScanMs When a collector last finished, or null if never. */
	watch(intervalMins: number, lastScanMs: number | null) {
		const every = Math.max(1, intervalMins) * 60_000;

		// A rebuild, or a scan already working, owns the archive; skipping is right
		// because the next tick comes around soon enough.
		const tick = () => {
			if (!this.running) void this.run('scan');
		};

		// The delay counts from the last scan, not from opening the window. Counting
		// from launch made the cadence depend on when the app happened to start:
		// quitting and reopening reset it, and reopening twice scanned twice. An
		// archive that was scanned ten minutes ago is ten minutes into its hour
		// however many times the window has been opened since.
		const elapsed = lastScanMs === null ? Infinity : Date.now() - lastScanMs;
		const due = Math.max(0, every - elapsed);

		// Deferred rather than called here even when due: `run` reads `running`, and
		// a read inside the effect that owns this subscribes the effect to it —
		// `running` clearing at the end of each scan then re-ran the effect, which
		// started another, forever. A timer runs outside that tracking.
		const first = setTimeout(tick, due);
		const timer = setInterval(tick, every);
		return () => {
			clearTimeout(first);
			clearInterval(timer);
		};
	}

	async run(action: 'scan' | 'rebuild') {
		if (this.running) return;
		this.running = action;
		this.result = null;
		this.error = null;
		try {
			this.result = await archive.runCollector(action);
			// A scan derives as its last step, so it settles both.
			this.needsRebuild = false;
			if (action === 'scan') this.needsScan = false;
		} catch (e) {
			this.error = (e as Error).message;
		} finally {
			this.running = null;
		}
	}
}

export const collector = new Collector();

/** A clock that advances, for text that reads "4m ago".
 *
 *  One timer for the window rather than one per caller, and a minute's
 *  resolution because that is the smallest unit any of that text shows. */
class Clock {
	now = $state(Date.now());
	#timer: ReturnType<typeof setInterval> | null = null;

	start() {
		if (this.#timer) return () => {};
		this.#timer = setInterval(() => (this.now = Date.now()), 30_000);
		return () => {
			if (this.#timer) clearInterval(this.#timer);
			this.#timer = null;
		};
	}
}

export const clock = new Clock();
