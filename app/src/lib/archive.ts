import { invoke } from '@tauri-apps/api/core';

/** Mirrors `lore_core::api`. Coverage is uneven by design and these types say so. */

export type Tier = 'certain' | 'strong' | 'weak';

export interface CollectorStatus {
	/** When a collector last finished, and when it last wrote something. A scan
	 *  that finds nothing new advances the first only. */
	last_scan_ms: number | null;
	last_archived_ms: number | null;
	records: number;
	sessions: number;
	commits: number;
	file_changes: number;
	origins: number;
	earliest_ms: number | null;
	latest_ms: number | null;
	scan_interval_mins: number;
	/** 'scan' or 'rebuild' while a collector is working, whoever started it. */
	running: string | null;
	roots: string[];
	sessions_only_in_lore: number;
}

export interface ProjectInfo {
	path: string;
	name: string;
	category: string;
	last_activity_ms: number | null;
	blocks: number;
}

/** Counted time split by what evidences it. Splits `project_ms`, not
 *  `elapsed_ms`: elapsed merges overlapping blocks, so a per-class split of it
 *  would total more than the whole. These four add up exactly. */
export interface EvidenceMs {
	sessions: number;
	commits: number;
	saves: number;
	bare: number;
}

export interface Summary {
	elapsed_ms: number;
	project_ms: number;
	blocks: number;
	projects: number;
	sessions: number;
	commits: number;
	file_changes: number;
	input_tokens: number;
	output_tokens: number;
	ai_share: number | null;
	by_evidence: EvidenceMs;
}

export interface SessionSummary {
	id: string;
	title: string;
	started_ms: number | null;
	ended_ms: number | null;
	prompts: number;
	replies: number;
	tool_calls: number;
	input_tokens: number;
	output_tokens: number;
	models: string[];
	files_written: number;
	has_transcript: boolean;
	continued?: boolean;
	/** First record of this session inside the block being listed. */
	first_seen_ms?: number | null;
}

export interface CommitSummary {
	sha: string;
	short: string;
	ts_ms: number;
	subject: string;
	insertions: number;
	deletions: number;
	file_count: number;
	unreachable: boolean;
	tier: Tier | null;
	session_id: string | null;
	shared_files: number;
}

export interface CommitFile {
	path: string;
	name: string;
	added: number | null;
	deleted: number | null;
}

export interface FileChangeSummary {
	path: string;
	ts_ms: number;
	state: 'dirty' | 'untracked' | 'no-repo';
}

export interface BlockDetail {
	id: number;
	records: number;
	project_path: string;
	project: string;
	category: string;
	started_ms: number;
	ended_ms: number;
	sessions: SessionSummary[];
	commits: CommitSummary[];
	file_changes: FileChangeSummary[];
	evidence: Evidence;
}

/** What kind of record backs a block's span. Stamped by the archive, not derived
 *  here, so the timeline and the digest cannot classify a block differently. */
export type Evidence = 'sessions' | 'commits' | 'saves' | 'bare';

export interface Bar {
	block_id: number;
	started_ms: number;
	ended_ms: number;
	sessions: number;
	commits: number;
	file_changes: number;
	evidence: Evidence;
}

export interface Lane {
	project_path: string;
	project: string;
	category: string;
	total_ms: number;
	bars: Bar[];
}

export interface ToolCall {
	name: string;
	target: string;
	failed: boolean;
}

/** Markdown, parsed in Rust. The window renders these nodes as components rather
 *  than building markup, so archived text can never become markup. */
export type Span =
	| { s: 't'; text: string }
	| { s: 'c'; text: string }
	| { s: 'b'; spans: Span[] }
	| { s: 'i'; spans: Span[] }
	| { s: 'a'; href: string; spans: Span[] };

export type Block =
	| { b: 'p'; spans: Span[] }
	| { b: 'h'; level: number; spans: Span[] }
	| { b: 'code'; lang?: string; text: string }
	| { b: 'list'; ordered: boolean; start?: number; items: Block[][] }
	| { b: 'quote'; blocks: Block[] }
	| { b: 'table'; head: Span[][]; rows: Span[][][] }
	| { b: 'rule' };

export interface Turn {
	role: 'user' | 'assistant';
	ts_ms: number | null;
	blocks: Block[];
	truncated: boolean;
	tools: ToolCall[];
}

export interface TouchedFile {
	path: string;
	name: string;
	writes: number;
	reads: number;
}

export interface SessionDetail {
	session: SessionSummary;
	project_path: string;
	project: string;
	category: string;
	files: TouchedFile[];
	commits: CommitSummary[];
	turns: Turn[];
}

export interface Root {
	path: string;
	category: string;
}

export interface LoreConfig {
	scan_interval_mins: number;
	idle_gap_mins: number;
	file_lookback_days: number;
	roots: Root[];
	exclude: string[];
	identities: string[];
	sources: { claude: { enabled: boolean; path?: string | null } };
}

/** The config, with an identifier for the file it came from. Handing that back on
 *  save is what lets a stale copy be refused instead of overwriting. */
export interface ConfigView {
	config: LoreConfig;
	revision: string | null;
}

export interface Paths {
	config_path: string;
	db_path: string;
}

/** Commands surface their failure message so the UI can name the problem. */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	try {
		return await invoke<T>(command, args);
	} catch (error) {
		const message =
			typeof error === 'object' && error && 'message' in error
				? String((error as { message: unknown }).message)
				: String(error);
		throw new Error(message);
	}
}

export const archive = {
	status: () => call<CollectorStatus>('status'),
	paths: () => call<Paths>('paths'),
	collectorRun: () => call<string | null>('collector_run'),
	projects: () => call<ProjectInfo[]>('projects'),
	summary: (fromMs: number, toMs: number) => call<Summary>('summary', { fromMs, toMs }),
	timeline: (fromMs: number, toMs: number, limit?: number, project?: string, category?: string) =>
		call<BlockDetail[]>('timeline', {
			fromMs,
			toMs,
			project: project ?? null,
			category: category ?? null,
			limit: limit ?? null
		}),
	/** One block. Lanes carries bars rather than blocks, so a selection there has
	 *  nothing in hand; asking for the one row beats re-running the range. */
	block: (id: number) => call<BlockDetail | null>('block', { id }),
	lanes: (fromMs: number, toMs: number, category?: string) =>
		call<Lane[]>('lanes', { fromMs, toMs, category: category ?? null }),
	session: (id: string) => call<SessionDetail | null>('session', { id }),
	commitFiles: (sha: string) => call<CommitFile[]>('commit_files', { sha }),
	config: () => call<ConfigView>('read_config'),
	saveConfig: (config: LoreConfig, revision: string | null) =>
		call<ConfigView>('write_config', { config, revision }),
	runCollector: (action: 'scan' | 'rebuild') => call<string>('run_collector', { action })
};
