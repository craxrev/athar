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
