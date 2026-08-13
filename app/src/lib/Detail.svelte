<script lang="ts">
	import Icon from './Icon.svelte';
	import type { BlockDetail } from './archive';
	import { clock, duration, fullDay, shortPath, tokens } from './format';

	let {
		block,
		onOpenSession
	}: { block: BlockDetail | null; onOpenSession: (id: string) => void } = $props();
</script>

<aside class="detail">
	<div class="drag" data-tauri-drag-region></div>

	{#if !block}
		<div class="idle">
			<p>Select a block to see what happened in it.</p>
		</div>
	{:else}
		<div class="scroll">
			<header>
				<h2>{block.project}</h2>
				<p class="where" title={block.project_path}>{shortPath(block.project_path, 3)}</p>
				<p class="when">
					{fullDay(block.started_ms)}
					<span class="num">{clock(block.started_ms)}–{clock(block.ended_ms)}</span>
					<span class="num strong">{duration(block.ended_ms - block.started_ms)}</span>
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
									Transcript deleted at source — only the prompts survive
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
									Inferred from timing alone — likely committed by hand
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
				<section>
					<h3>File changes <span class="num floor">{block.file_changes.length} recorded</span></h3>
					<ul class="files">
						{#each block.file_changes as f (f.path + f.ts_ms)}
							<li>
								<span class="num time">{clock(f.ts_ms)}</span>
								<span class="path" title={f.path}>{shortPath(f.path, 2)}</span>
								<span class="state" data-state={f.state}>{f.state}</span>
							</li>
						{/each}
					</ul>
					<p class="caveat">
						A count of changes is a floor: saves between two scans leave only the most
						recent timestamp behind.
					</p>
				</section>
			{/if}

			{#if !block.sessions.length && !block.commits.length && !block.file_changes.length}
				<p class="idle">Activity was recorded here, but nothing survived in detail.</p>
			{/if}
		</div>
	{/if}
</aside>

<style>
	.detail {
		display: flex;
		flex-direction: column;
		min-height: 0;
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

	header {
		padding-bottom: 18px;
		border-bottom: 1px solid var(--line);
	}

	h2 {
		margin: 0;
		font-size: 21px;
		font-weight: 660;
		letter-spacing: -0.02em;
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

	.files {
		margin: 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.files li {
		display: flex;
		align-items: baseline;
		gap: 9px;
		padding: 4px 2px;
		font-size: 13px;
	}
	.time {
		color: var(--text-faint);
		flex: none;
	}
	.path {
		flex: 1;
		min-width: 0;
		font-family: var(--mono);
		font-size: var(--fs-min);
		color: var(--text-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.state {
		flex: none;
		font-size: var(--fs-min);
		font-weight: 540;
		color: var(--text-faint);
	}
	.state[data-state='dirty'] {
		color: var(--amber);
	}
	.state[data-state='untracked'] {
		color: var(--text-dim);
	}

	.caveat {
		margin: 10px 0 0;
		font-size: var(--fs-meta);
		line-height: 1.5;
		color: var(--text-faint);
	}
</style>
