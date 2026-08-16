<script lang="ts">
	import Icon from './Icon.svelte';
	import Moments from './Moments.svelte';
	import type { BlockDetail } from './archive';
	import { clock, duration, fullDay, shortPath, tokens } from './format';

	/** What the bar's shape asserted, said in words. The lane legend only renders
	 *  when a range holds more than one class, so the moment a user meets an
	 *  unfamiliar treatment cold is the moment with no key on screen — this pane
	 *  is the one surface with room to teach it. */
	const EVIDENCE_SENTENCE: Record<string, string> = {
		sessions: 'From sessions: a conversation covers the whole span.',
		commits: 'From commits: exact at each commit, inferred between them.',
		saves: 'From saves: timestamps at each end, nothing in between.',
		bare: 'From records only: the span is real; nothing in it is itemised.'
	};

	let {
		width,
		block,
		loading = false,
		error = null,
		onOpenSession
	}: {
		/** Owned by the page, which is the only place that knows what the centre
		 *  pane has left to give. */
		width: number;
		block: BlockDetail | null;
		/** A selection is being read. The pane must say so rather than keep
		 *  describing the block that was selected before it. */
		loading?: boolean;
		error?: string | null;
		onOpenSession: (id: string) => void;
	} = $props();
</script>

<aside class="detail" style="width: {width}px">
	<div class="drag" data-tauri-drag-region></div>

	{#if loading}
		<div class="idle">
			<p>Reading this block…</p>
		</div>
	{:else if error}
		<div class="idle">
			<p class="fault">{error}</p>
		</div>
	{:else if !block}
		<div class="idle">
			<p>Select a block to see what happened.</p>
		</div>
	{:else}
		<div class="scroll">
			<!-- Polite live region: walking the timeline with j/k changes this pane and
			     nothing else, so without it a keyboard user moved the selection five
			     times and was told nothing at all. Project, when, how long, and what
			     the span is evidenced by — the whole answer, once. -->
			<header aria-live="polite">
				<h2>{block.project}</h2>
				<p class="where" title={block.project_path}>{shortPath(block.project_path, 3)}</p>
				<p class="when">
					{fullDay(block.started_ms)}
					<span class="num">{clock(block.started_ms)}–{clock(block.ended_ms)}</span>
					<span class="num strong">{duration(block.ended_ms - block.started_ms)}</span>
				</p>
				<p class="evidence" data-evidence={block.evidence}>
					{EVIDENCE_SENTENCE[block.evidence] ?? EVIDENCE_SENTENCE.bare}
				</p>
			</header>

			{#if block.sessions.length}
				<section>
					<h3>Sessions</h3>
					{#each block.sessions as s (s.id)}
						<button class="card" onclick={() => onOpenSession(s.id)}>
							<span class="cardhead">
								<span class="title">{s.title}</span>
								<Icon name="chevron" size={16} />
							</span>
							<span class="figures">
								<span><b class="num">{s.prompts}</b> prompts</span>
								<span><b class="num">{s.replies}</b> replies</span>
								<span><b class="num">{s.tool_calls}</b> tools</span>
								<span><b class="num">{s.files_written}</b> files written</span>
								<span><b class="num">{tokens(s.input_tokens + s.output_tokens)}</b> tokens</span>
							</span>
							{#if !s.has_transcript}
								<span class="note amber">
									<Icon name="warn" size={14} />
									Transcript deleted at source. Prompts survive.
								</span>
							{/if}
						</button>
					{/each}
				</section>
			{/if}

			{#if block.commits.length}
				<section>
					<h3>Commits</h3>
					{#each block.commits as c (c.sha)}
						<div class="card static">
							<span class="title">{c.subject}</span>
							<span class="figures">
								<span class="num sha">{c.short}</span>
								<span class="num add">+{c.insertions}</span>
								<span class="num del">−{c.deletions}</span>
								<span><b class="num">{c.file_count}</b> files</span>
							</span>
							<span class="note" data-tier={c.tier ?? 'none'}>
								{#if c.tier === 'certain'}
									<Icon name="witnessed" size={14} />
									The transcript records the assistant running this commit
								{:else if c.tier === 'strong'}
									<Icon name="inferred" size={14} />
									Inferred: {c.shared_files} of {c.file_count} files were written in this session
								{:else if c.tier === 'weak'}
									<Icon name="inferred" size={14} />
									Inferred from timing alone; likely by hand
								{:else}
									<Icon name="warn" size={14} />
									No session to attribute this to
								{/if}
							</span>
							{#if c.unreachable}
								<span class="note amber">
									<Icon name="warn" size={14} />
									No ref reaches this commit. Git will collect it; lore kept it.
								</span>
							{/if}
						</div>
					{/each}
				</section>
			{/if}

			{#if block.file_changes.length}
				<!-- "at least" rides the figure rather than a sentence under it. The
				     count is a floor — three saves inside one scan interval leave one
				     timestamp — and that changes what the number means, so it cannot be a
				     footnote the eye skips. Two words on the number beat a caveat below
				     the list, and are shorter than the caveat was. -->
				<section>
					<h3>File changes <span class="num floor">at least {block.file_changes.length}</span></h3>
					<Moments changes={block.file_changes} limit={8} />
				</section>
			{/if}

			{#if !block.sessions.length && !block.commits.length && !block.file_changes.length}
				<!-- The same sentence Stream uses for the same block, so the two panes
				     do not describe one thing two ways. -->
				<p class="idle">
					<b class="num">{block.records}</b> records here, nothing itemised.
				</p>
			{/if}
		</div>
	{/if}
</aside>

<style>
	.detail {
		display: flex;
		flex-direction: column;
		flex: none;
		min-height: 0;
		overflow: hidden;
		border-left: 1px solid var(--line);
		background: var(--surface);
	}

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

	.idle {
		flex: 1;
		display: grid;
		place-items: center;
		padding: 24px;
		color: var(--text-faint);
		font-size: 14px;
		text-align: center;
	}
	/* A pane that could not read its block says why, in the same voice the
	   window's own fault panel uses. */
	.fault {
		margin: 0;
		max-width: 34ch;
		line-height: 1.55;
		color: var(--del);
	}

	header {
		padding-bottom: 18px;
		border-bottom: 1px solid var(--line);
	}

	h2 {
		margin: 0;
		font-size: 21px;
		font-weight: 660;
		letter-spacing: -0.02em;
		/* The pane is 372px and overflow:hidden; without this a long project name
		   was clipped with no way to recover it. */
		overflow-wrap: anywhere;
	}

	.where {
		margin: 3px 0 0;
		font-family: var(--mono);
		font-size: var(--fs-meta);
		color: var(--text-faint);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.when {
		display: flex;
		flex-wrap: wrap;
		gap: 9px;
		margin: 10px 0 0;
		font-size: 13.5px;
		color: var(--text-dim);
	}
	.when .strong {
		color: var(--text);
		font-weight: 600;
	}

	/* Amber only where the coverage is genuinely a floor; the other three classes
	   state their footing without raising an alarm about it. */
	.evidence {
		margin: 9px 0 0;
		font-size: var(--fs-meta);
		line-height: 1.5;
		color: var(--text-faint);
	}
	.evidence[data-evidence='saves'] {
		color: var(--amber);
	}

	section {
		padding-top: 20px;
	}

	h3 {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin: 0 0 9px;
		font-size: 13.5px;
		font-weight: 640;
		color: var(--text-faint);
	}
	.floor {
		font-weight: 500;
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: 7px;
		width: 100%;
		padding: 11px 12px;
		margin-bottom: 8px;
		border: 1px solid var(--line);
		border-radius: var(--radius-sm);
		background: var(--surface-raised);
		box-shadow: var(--lift-1);
		text-align: left;
	}
	.card:not(.static):hover {
		background: var(--surface-hover);
		border-color: var(--line-strong);
	}

	.cardhead {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--text-faint);
	}

	.title {
		flex: 1;
		min-width: 0;
		color: var(--text);
		font-size: 14.5px;
		font-weight: 560;
		line-height: 1.35;
		overflow-wrap: anywhere;
	}

	.figures {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 12px;
		font-size: 13px;
		color: var(--text-faint);
	}
	.figures b {
		color: var(--text-dim);
		font-weight: 600;
	}
	.sha {
		color: var(--text-dim);
	}
	.add {
		color: var(--add);
	}
	.del {
		color: var(--del);
	}

	.note {
		display: flex;
		align-items: flex-start;
		gap: 6px;
		font-size: var(--fs-meta);
		line-height: 1.45;
		color: var(--text-faint);
	}
	.note :global(svg) {
		margin-top: 2px;
	}
	/* Witnessed reads settled; inferred reads provisional. */
	.note[data-tier='certain'] {
		color: var(--add);
	}
	.note[data-tier='weak'],
	.note[data-tier='none'] {
		color: var(--amber);
	}
	.note.amber {
		color: var(--amber);
	}
</style>
