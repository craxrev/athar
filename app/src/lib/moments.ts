import type { FileChangeSummary } from './archive';

/** Files saved together are one moment. Listing each as its own event made a
 *  burst of six saves read as six separate things happening. */
export const CLUSTER_MS = 60_000;

/** An idle stretch shorter than this is rhythm, not information. */
export const GAP_MS = 5 * 60_000;

export interface Moment {
	at: number;
	files: FileChangeSummary[];
	gapFromPrevious: number;
}

export function clusterMoments(changes: FileChangeSummary[]): Moment[] {
	const sorted = [...changes].sort((a, b) => a.ts_ms - b.ts_ms);
	const out: Moment[] = [];
	for (const change of sorted) {
		const last = out[out.length - 1];
		if (last && change.ts_ms - last.at <= CLUSTER_MS) {
			last.files.push(change);
			continue;
		}
		out.push({
			at: change.ts_ms,
			files: [change],
			gapFromPrevious: last ? change.ts_ms - last.at : 0
		});
	}
	return out;
}
