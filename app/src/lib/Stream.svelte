<script lang="ts">
	import Icon from './Icon.svelte';
	import Moments from './Moments.svelte';
	import type { BlockDetail } from './archive';
	import { clock, day, dayKey, duration, fullDay, tokens } from './format';

	let {
		blocks,
		selected,
		onSelect,
		onOpenSession
	}: {
		blocks: BlockDetail[];
		selected: number | null;
		onSelect: (blockId: number, projectPath: string) => void;
		onOpenSession: (id: string) => void;
	} = $props();

	/** Day headings are dividers in one continuous column, not a grid of dates. */
	let days = $derived.by(() => {
		const out: { key: string; at: number; blocks: BlockDetail[] }[] = [];
		for (const block of blocks) {
			const key = dayKey(block.started_ms);
			let group = out.find((g) => g.key === key);
			if (!group) {
				group = { key, at: block.started_ms, blocks: [] };
				out.push(group);
			}
			group.blocks.push(block);
		}
		return out;
	});

	/** A resumed session can begin days before the block it continues into, so the
	 *  date shows whenever the clock alone would mislead. */
	function continuedFrom(startedMs: number | null, blockMs: number): string {
		if (startedMs === null) return 'earlier';
		const sameDay = new Date(startedMs).toDateString() === new Date(blockMs).toDateString();
		return sameDay ? clock(startedMs) : `${day(startedMs)} ${clock(startedMs)}`;
	}

	const tierLabel: Record<string, string> = {
		certain: 'Committed by the assistant',
		strong: 'From this session — files match',
		weak: 'Same session window only'
	};
</script>

<div class="stream">
	{#each days as d (d.key)}
		<div class="day">
			<h3>
				{fullDay(d.at)}
				<span class="num count">{d.blocks.length} block{d.blocks.length === 1 ? '' : 's'}</span>
			</h3>

			{#each d.blocks as block (block.id)}
				<article class:on={selected === block.id}>
					<button class="head" onclick={() => onSelect(block.id, block.project_path)}>
						<span class="num when">{clock(block.started_ms)}</span>
						<span class="project">{block.project}</span>
						<span class="swatch" data-category={block.category}>{block.category}</span>
						<span class="num span">{duration(block.ended_ms - block.started_ms)}</span>
					</button>

					{#each block.sessions as s (s.id)}
						<button
							class="item session"
							class:continued={s.continued}
							onclick={() => onOpenSession(s.id)}
						>
							<Icon name="session" size={17} />
							<span class="body">
								<span class="title">{s.title}</span>
								<span class="meta">
									{#if s.continued}
										continues from {continuedFrom(s.started_ms, block.started_ms)} — its
										figures count once, at the block where it began
									{:else}
										<span class="num">{s.prompts}</span> prompts ·
										<span class="num">{s.tool_calls}</span> tools ·
										<span class="num">{tokens(s.input_tokens + s.output_tokens)}</span> tokens
										{#if s.models.length}· {s.models.join(', ')}{/if}
									{/if}
								</span>
							</span>
							{#if !s.has_transcript}
								<span class="flag amber" title="Claude Code deleted this transcript; only the prompt history survives">
									<Icon name="warn" size={14} /> prompts only
								</span>
							{/if}
							<span class="chev"><Icon name="chevron" size={16} /></span>
						</button>
					{/each}

					{#each block.commits as c (c.sha)}
						<div class="item commit">
							<Icon name="commit" size={17} />
							<span class="body">
								<span class="title">{c.subject}</span>
								<span class="meta">
									<span class="num sha">{c.short}</span>
									<span class="num add">+{c.insertions}</span>
									<span class="num del">−{c.deletions}</span>
									{#if c.tier}
										<span class="tier" data-tier={c.tier} title={tierLabel[c.tier]}>
											<Icon name={c.tier === 'certain' ? 'witnessed' : 'inferred'} size={13} />
											{c.tier === 'certain' ? 'witnessed' : c.tier === 'strong' ? 'files match' : 'inferred'}
										</span>
									{:else}
										<span class="tier" data-tier="none">unattributed</span>
									{/if}
								</span>
							</span>
							{#if c.unreachable}
								<span class="flag amber" title="No ref reaches this commit. Git will collect it; lore has already kept it.">
									<Icon name="warn" size={14} /> only in lore
								</span>
							{/if}
						</div>
					{/each}

					{#if !block.sessions.length && !block.commits.length && !block.file_changes.length}
						<p class="bare">
							<span class="num">{block.records}</span> records archived here — harness
							state and prompt history, with nothing the timeline itemises.
						</p>
					{/if}

					{#if block.file_changes.length}
						<div class="item files">
							<Icon name="file" size={17} />
							<span class="body">
								<span class="title">
									{block.file_changes.length} file change{block.file_changes.length === 1 ? '' : 's'}
								</span>
								<Moments changes={block.file_changes} limit={3} />
							</span>
						</div>
					{/if}
				</article>
			{/each}
		</div>
	{/each}
</div>

<style>
	.stream {
		height: 100%;
		overflow-y: auto;
		padding: 4px 20px 40px;
	}

	.day {
		padding-top: 26px;
	}

	h3 {
		display: flex;
		align-items: baseline;
		gap: 10px;
		margin: 0 0 10px;
		font-size: 15px;
		font-weight: 640;
		color: var(--text);
	}
	h3 .count {
		color: var(--text-faint);
		font-weight: 500;
	}

	article {
		border: 1px solid var(--line);
		border-radius: var(--radius);
		background: var(--surface);
		margin-bottom: 10px;
		overflow: hidden;
	}
	article.on {
		border-color: var(--accent-edge);
		background: color-mix(in oklab, var(--accent) 6%, var(--surface));
		box-shadow: var(--lift-1);
	}

	.head {
		display: flex;
		align-items: center;
		gap: 11px;
		width: 100%;
		padding: 11px 14px;
		text-align: left;
	}
	.head:hover {
		background: var(--surface-hover);
	}

	.when {
		color: var(--text-dim);
		font-weight: 550;
	}

	.project {
		font-size: 15.5px;
		font-weight: 620;
		letter-spacing: -0.01em;
	}

	.swatch {
		font-size: var(--fs-meta);
		font-weight: 560;
		padding: 2px 7px;
		border-radius: 999px;
		color: var(--text-dim);
		background: rgba(255, 255, 255, 0.06);
	}
	.swatch[data-category='work'] {
		color: var(--cat-work-tint);
		background: var(--cat-work-fill);
	}
	.swatch[data-category='research'] {
		color: var(--cat-research-tint);
		background: var(--cat-research-fill);
	}
	.swatch[data-category='personal'] {
		color: var(--cat-personal-tint);
		background: var(--cat-personal-fill);
	}
	.swatch[data-category='freelance'] {
		color: var(--cat-freelance-tint);
		background: var(--cat-freelance-fill);
	}

	.span {
		margin-left: auto;
		color: var(--text-dim);
		font-weight: 550;
	}

	.item {
		display: flex;
		align-items: flex-start;
		gap: 10px;
		width: 100%;
		padding: 9px 14px;
		border-top: 1px solid var(--line);
		text-align: left;
		color: var(--text-dim);
	}
	.item :global(svg) {
		margin-top: 2px;
	}
	button.item:hover {
		background: var(--surface-hover);
	}
	.session :global(svg) {
		color: var(--accent);
	}
	/* A continuation is the same conversation, not another one. */
	.session.continued :global(svg) {
		color: var(--text-faint);
	}
	.session.continued .title {
		color: var(--text-dim);
		font-weight: 500;
	}

	.body {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.title {
		color: var(--text);
		font-size: 14.5px;
		font-weight: 540;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.meta {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 7px;
		font-size: 13px;
		color: var(--text-faint);
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

	.tier {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: var(--fs-meta);
		font-weight: 540;
	}
	/* Witnessed and inferred are different claims and must not look alike. */
	.tier[data-tier='certain'] {
		color: var(--add);
	}
	.tier[data-tier='strong'] {
		color: var(--text-dim);
	}
	.tier[data-tier='weak'],
	.tier[data-tier='none'] {
		color: var(--amber);
	}


	.flag {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		flex: none;
		font-size: var(--fs-meta);
		font-weight: 540;
		padding: 2px 8px;
		border-radius: 999px;
	}
	.flag.amber {
		color: var(--amber);
		background: var(--amber-soft);
	}

	.bare {
		margin: 0;
		padding: 9px 14px;
		border-top: 1px solid var(--line);
		font-size: 13px;
		line-height: 1.45;
		color: var(--text-faint);
	}

	.chev {
		flex: none;
		color: var(--text-faint);
		margin-top: 2px;
	}
	button.item:hover .chev {
		color: var(--text);
	}
</style>
